//
//  PairedMac.swift
//  NextypeApp
//
//  配对的Mac设备数据模型
//

import Combine
import Foundation
import Security

struct PairedMac: Codable, Identifiable {
    let id: String  // 等于deviceId
    let deviceId: String  // Mac的唯一设备ID
    var deviceName: String  // Mac的显示名称
    var lastKnownIP: String?  // 最后已知的IP地址
    var lastKnownPort: Int  // 最后已知的端口
    var encryptionKey: String  // 加密密钥
    let pairedAt: Date  // 配对时间
    var lastConnected: Date?  // 最后连接时间
    var supportRelay: Bool  // 是否支持中继连接
    var customName: String?  // 用户自定义名称
    var customIcon: String?  // 用户自定义图标（SF Symbol 名称）

    init(
        deviceId: String, deviceName: String, ip: String?, port: Int, encryptionKey: String,
        supportRelay: Bool = false
    ) {
        self.id = deviceId
        self.deviceId = deviceId
        self.deviceName = deviceName
        self.lastKnownIP = ip
        self.lastKnownPort = port
        self.encryptionKey = encryptionKey
        self.pairedAt = Date()
        self.lastConnected = nil
        self.supportRelay = supportRelay
    }

    /// 更新最后连接时间和IP
    mutating func updateConnection(ip: String, port: Int) {
        self.lastKnownIP = ip
        self.lastKnownPort = port
        self.lastConnected = Date()
    }

    /// 获取显示名称
    func getDisplayName() -> String {
        return customName ?? deviceName
    }
}

/// 配对的Mac设备管理器
class PairedMacManager: ObservableObject {
    @Published var pairedMacs: [PairedMac] = []

    private let keychainService = "com.nextype.app"
    private let pairedMacsAccount = "nextype_paired_macs"
    private let lastConnectedAccount = "nextype_last_connected_device_id"

    // 仅用于迁移旧数据
    private let legacyPairedMacsKey = "pairedMacs"
    private let legacyLastConnectedKey = "lastConnectedDeviceId"

    /// 上次连接的设备ID（读写 Keychain，卸载重装后保留）
    var lastConnectedDeviceId: String? {
        get {
            guard let data = keychainReadData(account: lastConnectedAccount) else { return nil }
            return String(data: data, encoding: .utf8)
        }
        set {
            if let value = newValue, let data = value.data(using: .utf8) {
                keychainWriteData(data, account: lastConnectedAccount)
            } else {
                keychainDeleteData(account: lastConnectedAccount)
            }
            #if DEBUG
            print("💾 保存上次连接设备ID: \(newValue ?? "nil")")
            #endif
        }
    }

    /// 获取上次连接的Mac，如果不存在则返回第一个配对设备
    func getLastConnectedMac() -> PairedMac? {
        if let lastId = lastConnectedDeviceId {
            return pairedMacs.first { $0.deviceId == lastId } ?? pairedMacs.first
        }
        return pairedMacs.first
    }

    init() {
        loadPairedMacs()
        setupRelaySync()
    }

    private func setupRelaySync() {
        // 监听信任列表同步
        ConnectionManager.shared.onTrustListSync = { [weak self] remoteMacs in
            DispatchQueue.main.async {
                self?.syncWithRemoteList(remoteMacs)
            }
        }

        // 监听解除配对通知
        ConnectionManager.shared.onDeviceUnpaired = { [weak self] deviceId in
            DispatchQueue.main.async {
                self?.removePairedMac(deviceId: deviceId, notifyServer: false)
            }
        }
    }

    private func syncWithRemoteList(_ remoteMacs: [RemoteMac]) {
        #if DEBUG
        print("🔄 开始同步信任列表...")
        #endif

        // 空列表保护：如果服务器返回空列表但本地有设备，跳过同步
        // 防止服务器异常时误删所有本地配对
        if remoteMacs.isEmpty && !pairedMacs.isEmpty {
            #if DEBUG
            print("⚠️ 服务器返回空信任列表，跳过同步以保护本地数据")
            #endif
            return
        }

        var changed = false

        let remoteIds = Set(remoteMacs.map { $0.deviceId })
        let localIds = Set(pairedMacs.map { $0.deviceId })

        #if DEBUG
        print("📊 本地设备: \(localIds)")
        #endif
        #if DEBUG
        print("☁️ 远程设备: \(remoteIds)")
        #endif

        // 找出需要移除的
        let toRemove = localIds.subtracting(remoteIds)
        if !toRemove.isEmpty {
            #if DEBUG
            print("➖ 发现需要移除的设备: \(toRemove)")
            #endif
            pairedMacs.removeAll { toRemove.contains($0.deviceId) }
            changed = true
            #if DEBUG
            print("✅ 同步移除 \(toRemove.count) 个设备完成")
            #endif
        } else {
            #if DEBUG
            print("✅ 没有需要移除的设备")
            #endif
        }

        // 找出需要添加的（自动恢复）
        let toAdd = remoteIds.subtracting(localIds)
        if !toAdd.isEmpty {
            #if DEBUG
            print("➕ 发现服务器有但本地没有的设备（自动恢复）: \(toAdd)")
            #endif
            for deviceId in toAdd {
                if let remote = remoteMacs.first(where: { $0.deviceId == deviceId }) {
                    // 创建恢复的设备记录
                    // 注意：从 trust_list 同步时无法获取共享密钥，encryptionKey 暂为空串
                    // 此设备下次使用前需重新配对才能正常加密
                    let recoveredMac = PairedMac(
                        deviceId: remote.deviceId,
                        deviceName: remote.deviceName,
                        ip: "relay",
                        port: 8080,
                        encryptionKey: "",
                        supportRelay: true
                    )
                    pairedMacs.append(recoveredMac)
                }
            }
            changed = true
            #if DEBUG
            print("✅ 自动恢复了 \(toAdd.count) 个设备")
            #endif
        }

        if changed {
            savePairedMacs()
        }
    }

    /// 加载已配对的Mac列表（优先读 Keychain，首次启动时自动从 UserDefaults 迁移）
    func loadPairedMacs() {
        // 1. 尝试读 Keychain
        if let data = keychainReadData(account: pairedMacsAccount),
           let macs = try? JSONDecoder().decode([PairedMac].self, from: data) {
            self.pairedMacs = macs
            #if DEBUG
            print("📱 从 Keychain 加载了 \(macs.count) 个已配对的Mac")
            #endif
            return
        }

        // 2. 迁移：若 UserDefaults 有旧数据，写入 Keychain 后删除旧值
        if let data = UserDefaults.standard.data(forKey: legacyPairedMacsKey),
           let macs = try? JSONDecoder().decode([PairedMac].self, from: data) {
            keychainWriteData(data, account: pairedMacsAccount)
            UserDefaults.standard.removeObject(forKey: legacyPairedMacsKey)
            self.pairedMacs = macs
            #if DEBUG
            print("🔑 配对列表已从 UserDefaults 迁移至 Keychain，共 \(macs.count) 个")
            #endif

            // 同步迁移 lastConnectedDeviceId
            if let lastId = UserDefaults.standard.string(forKey: legacyLastConnectedKey) {
                if let idData = lastId.data(using: .utf8) {
                    keychainWriteData(idData, account: lastConnectedAccount)
                }
                UserDefaults.standard.removeObject(forKey: legacyLastConnectedKey)
                #if DEBUG
                print("🔑 lastConnectedDeviceId 已从 UserDefaults 迁移至 Keychain")
                #endif
            }
        }
    }

    /// 保存已配对的Mac列表到 Keychain
    func savePairedMacs() {
        if let data = try? JSONEncoder().encode(pairedMacs) {
            keychainWriteData(data, account: pairedMacsAccount)
            #if DEBUG
            print("💾 已保存 \(pairedMacs.count) 个配对的Mac 到 Keychain")
            #endif
        }
    }

    // MARK: - Keychain 私有方法

    private func keychainReadData(account: String) -> Data? {
        let query: [CFString: Any] = [
            kSecClass:       kSecClassGenericPassword,
            kSecAttrService: keychainService,
            kSecAttrAccount: account,
            kSecReturnData:  true,
            kSecMatchLimit:  kSecMatchLimitOne
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return nil }
        return data
    }

    private func keychainWriteData(_ data: Data, account: String) {
        let baseQuery: [CFString: Any] = [
            kSecClass:          kSecClassGenericPassword,
            kSecAttrService:    keychainService,
            kSecAttrAccount:    account,
            kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlock
        ]
        if keychainReadData(account: account) != nil {
            SecItemUpdate(baseQuery as CFDictionary, [kSecValueData: data] as CFDictionary)
        } else {
            var newQuery = baseQuery
            newQuery[kSecValueData] = data
            SecItemAdd(newQuery as CFDictionary, nil)
        }
    }

    private func keychainDeleteData(account: String) {
        let query: [CFString: Any] = [
            kSecClass:       kSecClassGenericPassword,
            kSecAttrService: keychainService,
            kSecAttrAccount: account
        ]
        SecItemDelete(query as CFDictionary)
    }

    /// 添加新配对的Mac
    func addPairedMac(_ mac: PairedMac) {
        // 检查是否已存在
        if let index = pairedMacs.firstIndex(where: { $0.deviceId == mac.deviceId }) {
            // 重新配对时保留用户设置的自定义名称和图标
            var updatedMac = mac
            updatedMac.customName = pairedMacs[index].customName
            updatedMac.customIcon = pairedMacs[index].customIcon
            pairedMacs[index] = updatedMac
            #if DEBUG
            print("🔄 更新已配对的Mac: \(mac.deviceName)")
            #endif
        } else {
            // 添加新的
            pairedMacs.append(mac)
            #if DEBUG
            print("➕ 添加新配对的Mac: \(mac.deviceName)")
            #endif
        }
        savePairedMacs()
    }

    /// 移除配对的Mac
    func removePairedMac(deviceId: String, notifyServer: Bool = true) {
        pairedMacs.removeAll { $0.deviceId == deviceId }
        savePairedMacs()
        #if DEBUG
        print("➖ 移除配对的Mac: \(deviceId)")
        #endif

        if notifyServer {
            ConnectionManager.shared.sendUnpairRequest(targetDeviceId: deviceId)
        }
    }

    /// 更新Mac的连接信息
    func updateMacConnection(deviceId: String, ip: String, port: Int) {
        if let index = pairedMacs.firstIndex(where: { $0.deviceId == deviceId }) {
            pairedMacs[index].updateConnection(ip: ip, port: port)
            savePairedMacs()
        }
    }

    /// 更新设备的名称和图标
    func updateMacCustomInfo(deviceId: String, name: String?, icon: String?) {
        if let index = pairedMacs.firstIndex(where: { $0.deviceId == deviceId }) {
            pairedMacs[index].customName = name
            pairedMacs[index].customIcon = icon
            savePairedMacs()
            objectWillChange.send()  // 确保 UI 刷新
            #if DEBUG
            print("📝 更新设备 \(deviceId) 信息: \(name ?? "nil"), \(icon ?? "nil")")
            #endif
        }
    }

    /// 检查设备是否已配对
    func isPaired(deviceId: String) -> Bool {
        return pairedMacs.contains { $0.deviceId == deviceId }
    }

    /// 获取配对的Mac
    func getPairedMac(deviceId: String) -> PairedMac? {
        return pairedMacs.first { $0.deviceId == deviceId }
    }
}
