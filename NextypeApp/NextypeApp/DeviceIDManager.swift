import Foundation
import UIKit
import Security

class DeviceIDManager {
    static let shared = DeviceIDManager()

    private let keychainService = "com.nextype.app"
    private let keychainAccount = "nextype_device_id"
    private let userDefaultsKey  = "nextype_device_id"   // 仅用于迁移旧数据
    private var cachedDeviceId: String?

    private init() {}

    // MARK: - 公开接口

    /// 获取设备 ID，优先读 Keychain；首次启动时自动从 UserDefaults 迁移旧值
    func getDeviceId() -> String {
        if let cached = cachedDeviceId { return cached }

        // 1. 尝试读 Keychain
        if let existing = keychainRead() {
            cachedDeviceId = existing
            return existing
        }

        // 2. 迁移：若 UserDefaults 有旧值，写入 Keychain 后删除旧值
        if let legacy = UserDefaults.standard.string(forKey: userDefaultsKey) {
            keychainWrite(legacy)
            UserDefaults.standard.removeObject(forKey: userDefaultsKey)
            #if DEBUG
            print("🔑 设备ID已从 UserDefaults 迁移至 Keychain: \(legacy)")
            #endif
            cachedDeviceId = legacy
            return legacy
        }

        // 3. 全新设备，生成 UUID 并写入 Keychain
        let newId = UUID().uuidString
        keychainWrite(newId)
        #if DEBUG
        print("🆕 生成新的设备ID: \(newId)")
        #endif
        cachedDeviceId = newId
        return newId
    }

    /// 重置设备ID（仅用于调试）
    func resetDeviceId() {
        keychainDelete()
        UserDefaults.standard.removeObject(forKey: userDefaultsKey)
        cachedDeviceId = nil
    }

    // MARK: - Keychain 私有方法

    private func keychainQuery() -> [CFString: Any] {
        return [
            kSecClass:       kSecClassGenericPassword,
            kSecAttrService: keychainService,
            kSecAttrAccount: keychainAccount
        ]
    }

    private func keychainRead() -> String? {
        var query = keychainQuery()
        query[kSecReturnData]      = true
        query[kSecMatchLimit]      = kSecMatchLimitOne

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess,
              let data = result as? Data,
              let value = String(data: data, encoding: .utf8)
        else { return nil }
        return value
    }

    private func keychainWrite(_ value: String) {
        guard let data = value.data(using: .utf8) else { return }
        var query = keychainQuery()
        query[kSecAttrAccessible] = kSecAttrAccessibleAfterFirstUnlock

        if keychainRead() != nil {
            // 已存在，更新
            let update: [CFString: Any] = [kSecValueData: data]
            SecItemUpdate(query as CFDictionary, update as CFDictionary)
        } else {
            // 不存在，新增
            query[kSecValueData] = data
            SecItemAdd(query as CFDictionary, nil)
        }
    }

    private func keychainDelete() {
        SecItemDelete(keychainQuery() as CFDictionary)
    }
}
