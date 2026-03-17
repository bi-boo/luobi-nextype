//
//  ConnectionManager.swift
//  NextypeApp
//
//  统一连接管理器 - 管理与中继服务器的 WebSocket 连接
//  替代原有的 WebSocketManager + RelayClient 双通道架构
//

import Combine
import Foundation
import UIKit

class ConnectionManager: NSObject, ObservableObject {

    // MARK: - 连接状态

    enum ConnectionState: Equatable {
        case disconnected
        case connecting
        case connected
    }

    @Published var connectionState: ConnectionState = .disconnected
    @Published var currentDevice: PairedMac?
    @Published var connectionError: String?
    @Published var lastSendError: String?
    @Published var showRePairAlert: Bool = false

    var isConnected: Bool { connectionState == .connected }

    // MARK: - 配置

    #if targetEnvironment(simulator)
    private let serverUrl = "ws://localhost:8443"
    #else
    private let serverUrl = "wss://nextypeapi.yuanfengai.cn:8443"
    #endif
    private let deviceId: String
    private let deviceName: String

    // MARK: - WebSocket

    private var webSocket: URLSessionWebSocketTask?
    private var session: URLSession?

    // MARK: - 心跳与重连

    private var heartbeatTimer: Timer?
    private var reconnectTimer: Timer?
    private var reconnectAttempts = 0
    private let maxReconnectAttempts = 10

    // MARK: - 回调

    var onMessageReceived: ((String, String) -> Void)?
    var onTrustListSync: (([RemoteMac]) -> Void)?
    var onDeviceUnpaired: ((String) -> Void)?
    var onRemoteCommand: ((String, [String: Any]) -> Void)?
    var onServerList: (([OnlineServerInfo]) -> Void)?

    // MARK: - 配对

    private var pairingCompletion: ((Result<RemoteMac, Error>) -> Void)?

    // MARK: - 单例

    static let shared = ConnectionManager()

    private override init() {
        self.deviceId = DeviceIDManager.shared.getDeviceId()
        self.deviceName = UIDevice.current.name
        super.init()
    }

    // MARK: - 连接管理

    /// 连接到中继服务器
    func connect() {
        let doConnect = {
            self.connectionError = nil

            guard self.webSocket == nil, self.connectionState != .connecting else {
                #if DEBUG
                print("⚠️ [连接] 已在连接中或已建立")
                #endif
                return
            }

            guard let url = URL(string: self.serverUrl) else {
                #if DEBUG
                print("❌ [连接] 无效的服务器地址")
                #endif
                return
            }

            self.connectionState = .connecting

            #if DEBUG
            print("🌐 [连接] 连接到中继服务器: \(self.serverUrl)")
            #endif

            let configuration = URLSessionConfiguration.default
            configuration.timeoutIntervalForRequest = 120
            configuration.timeoutIntervalForResource = 300
            self.session = URLSession(configuration: configuration, delegate: self, delegateQueue: OperationQueue())

            self.webSocket = self.session?.webSocketTask(with: url)
            self.webSocket?.resume()
            self.receiveMessage()
        }

        if Thread.isMainThread {
            doConnect()
        } else {
            DispatchQueue.main.async(execute: doConnect)
        }
    }

    /// 连接到指定设备
    func connectToDevice(_ device: PairedMac) {
        DispatchQueue.main.async {
            self.currentDevice = device
        }
        #if DEBUG
        print("🔍 [连接] 开始连接设备: \(device.deviceName) (ID: \(device.deviceId))")
        #endif

        if !isConnected {
            connect()
        }
    }

    /// 断开连接
    func disconnect() {
        let doDisconnect = {
            self.stopHeartbeat()
            self.stopReconnectTimer()

            self.webSocket?.cancel(with: .goingAway, reason: nil)
            self.webSocket = nil
            self.session = nil

            self.connectionState = .disconnected
            #if DEBUG
            print("👋 [连接] 已断开")
            #endif
        }

        if Thread.isMainThread {
            doDisconnect()
        } else {
            DispatchQueue.main.async(execute: doDisconnect)
        }
    }

    /// 切换设备（断开后重连）
    func switchToDevice(_ device: PairedMac) {
        DispatchQueue.main.async {
            self.currentDevice = device
        }
        if !isConnected {
            connect()
        }
        #if DEBUG
        print("🔄 [连接] 已切换到设备: \(device.getDisplayName())")
        #endif
    }

    // MARK: - 发送消息

    /// 发送剪贴板内容到 PC
    func sendClipboard(content: String, action: String, to deviceId: String) {
        let clipboardMsg: [String: Any] = [
            "type": "clipboard",
            "content": content,
            "action": action,
            "encrypted": true,
            "timestamp": Date().timeIntervalSince1970 * 1000,
        ]

        guard let jsonData = try? JSONSerialization.data(withJSONObject: clipboardMsg),
              let jsonString = String(data: jsonData, encoding: .utf8)
        else { return }

        relayToServer(serverId: deviceId, data: jsonString)
    }

    /// 中继转发消息到指定服务器
    func relayToServer(serverId: String, data: String) {
        let message: [String: Any] = [
            "type": "relay",
            "from": deviceId,
            "to": serverId,
            "data": data,
        ]
        send(message)
    }

    /// 验证配对码
    func verifyPairingCode(_ code: String, completion: @escaping (Result<RemoteMac, Error>) -> Void) {
        guard isConnected, webSocket != nil else {
            completion(.failure(NSError(domain: "ConnectionManager", code: -1,
                                       userInfo: [NSLocalizedDescriptionKey: "未连接到中继服务器，请检查网络"])))
            return
        }

        self.pairingCompletion = completion

        let message: [String: Any] = [
            "type": "verify_code",
            "code": code,
            "from": deviceId,
            "deviceName": deviceName,
        ]
        send(message)
        #if DEBUG
        print("🔢 [配对] 发送配对码验证: \(code)")
        #endif

        // 10秒超时
        DispatchQueue.main.asyncAfter(deadline: .now() + 10) { [weak self] in
            if self?.pairingCompletion != nil {
                self?.pairingCompletion?(.failure(NSError(domain: "ConnectionManager", code: -2,
                                                          userInfo: [NSLocalizedDescriptionKey: "配对请求超时，请重试"])))
                self?.pairingCompletion = nil
            }
        }
    }

    /// 发送解除配对请求
    func sendUnpairRequest(targetDeviceId: String) {
        let message: [String: Any] = [
            "type": "unpair_device",
            "targetDeviceId": targetDeviceId,
        ]
        send(message)
        #if DEBUG
        print("💔 [配对] 发送解除配对: \(targetDeviceId)")
        #endif
    }

    /// 请求同步信任列表
    func requestTrustSync() {
        let message: [String: Any] = ["type": "sync_trust_list"]
        send(message)
    }

    /// 发现在线设备（用于自动切换到最近活跃的电脑）
    func discoverOnlineDevices() {
        let message: [String: Any] = ["type": "discover"]
        send(message)
        #if DEBUG
        print("🔍 [发现] 发送 discover 请求")
        #endif
    }

    /// 发送屏幕参数
    func sendScreenInfo() {
        guard let device = currentDevice, isConnected else { return }

        let screen = UIScreen.main
        let bounds = screen.bounds
        let scale = screen.scale

        let screenInfo: [String: Any] = [
            "type": "device_info",
            "screenWidth": Int(bounds.width * scale),
            "screenHeight": Int(bounds.height * scale),
            "density": scale,
            "platform": "ios",
        ]

        guard let jsonData = try? JSONSerialization.data(withJSONObject: screenInfo),
              let jsonString = String(data: jsonData, encoding: .utf8)
        else { return }

        relayToServer(serverId: device.deviceId, data: jsonString)
        #if DEBUG
        print("📱 [屏幕] 已上报屏幕参数: \(Int(bounds.width * scale))x\(Int(bounds.height * scale))")
        #endif
    }

    // MARK: - 内部发送

    private func send(_ message: [String: Any]) {
        guard isConnected, let ws = webSocket else { return }

        guard let data = try? JSONSerialization.data(withJSONObject: message),
              let text = String(data: data, encoding: .utf8)
        else { return }

        let wsMessage = URLSessionWebSocketTask.Message.string(text)
        ws.send(wsMessage) { error in
            if let error = error {
                let msgType = message["type"] as? String ?? "unknown"
                #if DEBUG
                print("❌ [发送] \(msgType) 失败: \(error.localizedDescription)")
                #endif
            }
        }
    }

    private func register() {
        let message: [String: Any] = [
            "type": "register",
            "role": "client",
            "deviceId": deviceId,
            "deviceName": deviceName,
        ]
        send(message)
    }

    // MARK: - 消息接收

    private func receiveMessage() {
        webSocket?.receive { [weak self] result in
            guard let self = self else { return }

            switch result {
            case .success(let message):
                switch message {
                case .string(let text):
                    self.handleMessage(text)
                case .data(let data):
                    if let text = String(data: data, encoding: .utf8) {
                        self.handleMessage(text)
                    }
                @unknown default:
                    break
                }
                self.receiveMessage()

            case .failure(let error):
                #if DEBUG
                print("❌ [接收] 失败: \(error.localizedDescription)")
                #endif
                self.handleDisconnection(reason: "receiveMessage: \(error.localizedDescription)")
            }
        }
    }

    private func handleMessage(_ text: String) {
        guard let data = text.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let type = json["type"] as? String
        else { return }

        switch type {
        case "connected":
            #if DEBUG
            print("✅ [连接] 已连接到中继服务器")
            #endif
            // 必须在 connectionState = .connected 之后调用 register()，
            // 否则 send() 的 isConnected 检查在 OperationQueue 线程上会失败
            DispatchQueue.main.async {
                self.connectionState = .connected
                self.connectionError = nil
                self.reconnectAttempts = 0
                self.startHeartbeat()
                self.requestTrustSync()
                self.register()
                DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                    self.sendScreenInfo()
                }
            }

        case "registered":
            #if DEBUG
            print("✅ [连接] 注册成功")
            #endif

        case "relay":
            if let from = json["from"] as? String {
                // 兼容两种 data 格式：
                // - JSON 字符串（iOS/Android 发送方使用此格式）
                // - JSON 对象（Mac Rust 端 WsMessage::Relay 序列化后为对象）
                let cmdJson: [String: Any]?
                let rawDataString: String?

                if let dataStr = json["data"] as? String {
                    rawDataString = dataStr
                    cmdJson = (try? JSONSerialization.jsonObject(with: Data(dataStr.utf8))) as? [String: Any]
                } else if let dataObj = json["data"] as? [String: Any] {
                    rawDataString = nil
                    cmdJson = dataObj
                } else {
                    rawDataString = nil
                    cmdJson = nil
                }

                if let cmdJson = cmdJson, let cmdType = cmdJson["type"] as? String {
                    // 兼容 "command"（Mac Rust 端）和 "remote_command"（旧约定）两种类型名
                    if (cmdType == "remote_command" || cmdType == "command"),
                       let action = cmdJson["action"] as? String
                    {
                        DispatchQueue.main.async {
                            self.onRemoteCommand?(action, cmdJson)
                        }
                    } else if cmdType == "error", let errMsg = cmdJson["message"] as? String {
                        // Mac 端返回的错误（如解密失败）
                        #if DEBUG
                        print("❌ [中继] 来自 \(from) 的错误: \(errMsg)")
                        #endif
                        if errMsg == "decrypt_failed" {
                            DispatchQueue.main.async {
                                self.showRePairAlert = true
                            }
                        } else {
                            DispatchQueue.main.async {
                                self.lastSendError = "发送失败：\(errMsg)"
                            }
                        }
                    } else if let str = rawDataString {
                        DispatchQueue.main.async {
                            self.onMessageReceived?(from, str)
                        }
                    }
                } else if let str = rawDataString {
                    DispatchQueue.main.async {
                        self.onMessageReceived?(from, str)
                    }
                }
            }

        case "server_online":
            #if DEBUG
            print("🖥️ [状态] 服务器上线")
            #endif
            requestTrustSync()
            register()  // 通知 Mac 本机已在线，触发 Mac 端 client_online 更新状态

        case "server_offline":
            if let serverId = json["serverId"] as? String {
                #if DEBUG
                print("🖥️ [状态] 服务器离线: \(serverId)")
                #endif
                requestTrustSync()
            }

        case "heartbeat_ack":
            break

        case "error":
            if let errorMsg = json["message"] as? String {
                #if DEBUG
                print("❌ [服务器] 错误: \(errorMsg)")
                #endif
                DispatchQueue.main.async {
                    if self.pairingCompletion != nil {
                        self.pairingCompletion?(.failure(NSError(domain: "ConnectionManager", code: -1,
                                                            userInfo: [NSLocalizedDescriptionKey: errorMsg])))
                        self.pairingCompletion = nil
                    } else {
                        // 非配对场景的错误（如目标 Mac 离线）
                        let userMsg = errorMsg == "目标设备离线"
                            ? "电脑端未在线，请确认 Mac 应用已启动"
                            : "发送失败：\(errorMsg)"
                        self.lastSendError = userMsg
                    }
                }
            }

        case "pairing_success":
            if let server = json["server"] as? [String: Any],
               let sDeviceId = server["deviceId"] as? String,
               let sDeviceName = server["deviceName"] as? String
            {
                let sEncryptionKey = server["encryptionKey"] as? String
                let mac = RemoteMac(deviceId: sDeviceId, deviceName: sDeviceName, online: true,
                                    encryptionKey: sEncryptionKey)
                DispatchQueue.main.async {
                    self.pairingCompletion?(.success(mac))
                    self.pairingCompletion = nil
                }
            }

        case "pairing_error":
            if let message = json["message"] as? String {
                DispatchQueue.main.async {
                    self.pairingCompletion?(.failure(NSError(domain: "ConnectionManager", code: -1,
                                                             userInfo: [NSLocalizedDescriptionKey: message])))
                    self.pairingCompletion = nil
                }
            }

        case "trust_list":
            if let devices = json["devices"] as? [[String: Any]] {
                var macs: [RemoteMac] = []
                for device in devices {
                    if let id = device["id"] as? String,
                       let name = device["name"] as? String
                    {
                        macs.append(RemoteMac(deviceId: id, deviceName: name, online: true))
                    }
                }
                onTrustListSync?(macs)
            }

        case "device_unpaired":
            if let from = json["from"] as? String {
                #if DEBUG
                print("💔 [配对] 收到解除配对: \(from)")
                #endif
                onDeviceUnpaired?(from)
            }

        case "unpair_success":
            #if DEBUG
            print("✅ [配对] 解除成功")
            #endif

        case "ack":
            break

        case "server_list":
            if let servers = json["servers"] as? [[String: Any]] {
                let list: [OnlineServerInfo] = servers.compactMap { server in
                    guard let deviceId = server["deviceId"] as? String,
                          let deviceName = server["deviceName"] as? String
                    else { return nil }
                    let idleTime = server["idleTime"] as? Int ?? Int.max
                    return OnlineServerInfo(deviceId: deviceId, deviceName: deviceName, idleTime: idleTime)
                }
                #if DEBUG
                print("📋 [发现] 收到 server_list，共 \(list.count) 台设备")
                #endif
                DispatchQueue.main.async {
                    self.onServerList?(list)
                }
            }

        default:
            break
        }
    }

    // MARK: - 心跳

    private func startHeartbeat() {
        stopHeartbeat()
        heartbeatTimer = Timer.scheduledTimer(withTimeInterval: 20, repeats: true) { [weak self] _ in
            // 应用层心跳：让 relay-server 更新 lastHeartbeat，维持在线状态（与 Android 对齐）
            self?.send(["type": "heartbeat"])
            // 协议层 ping：检测 TCP 连接是否存活
            self?.webSocket?.sendPing { error in
                if let error = error {
                    #if DEBUG
                    print("❌ [心跳] ping 失败: \(error.localizedDescription)")
                    #endif
                    self?.handleDisconnection(reason: "ping failure")
                }
            }
        }
        // 立即发送第一次
        send(["type": "heartbeat"])
        webSocket?.sendPing { _ in }
    }

    private func stopHeartbeat() {
        heartbeatTimer?.invalidate()
        heartbeatTimer = nil
    }

    // MARK: - 重连

    private func handleDisconnection(reason: String = "unknown") {
        #if DEBUG
        print("🔌 [连接] 断开: \(reason)")
        #endif

        let capturedWs = webSocket
        DispatchQueue.main.async {
            // 忽略陈旧的断开通知：若当前 webSocket 已被新连接替换，则跳过避免覆盖新连接
            guard self.webSocket === capturedWs else {
                #if DEBUG
                print("ℹ️ [连接] 忽略陈旧断开通知（新连接已建立）")
                #endif
                return
            }
            self.connectionState = .disconnected
            capturedWs?.cancel(with: .abnormalClosure, reason: reason.data(using: .utf8))
            self.webSocket = nil
            self.stopHeartbeat()

            guard self.reconnectAttempts < self.maxReconnectAttempts else {
                #if DEBUG
                print("❌ [重连] 达到最大次数")
                #endif
                return
            }

            self.reconnectAttempts += 1
            let delay = min(pow(1.5, Double(self.reconnectAttempts)), 30.0)
            #if DEBUG
            print("🔄 [重连] \(String(format: "%.1f", delay))s 后重试 (\(self.reconnectAttempts)/\(self.maxReconnectAttempts))")
            #endif

            self.reconnectTimer = Timer.scheduledTimer(withTimeInterval: delay, repeats: false) { [weak self] _ in
                self?.connect()
            }
        }
    }

    private func stopReconnectTimer() {
        reconnectTimer?.invalidate()
        reconnectTimer = nil
        reconnectAttempts = 0
    }

    /// 检查连接状态，断开时自动重连
    func checkAndReconnect() {
        if connectionState == .disconnected {
            #if DEBUG
            print("🔄 [连接] 检测到断开，重连中...")
            #endif
            reconnectAttempts = 0
            connect()
        }
    }
}

// MARK: - URLSessionWebSocketDelegate

extension ConnectionManager: URLSessionWebSocketDelegate {
    nonisolated func urlSession(
        _ session: URLSession, webSocketTask: URLSessionWebSocketTask,
        didOpenWithProtocol protocol: String?
    ) {
        #if DEBUG
        print("✅ [WebSocket] 连接已建立")
        #endif
        // 连接建立后等待服务器发送 "connected" 消息来确认
    }

    nonisolated func urlSession(
        _ session: URLSession, webSocketTask: URLSessionWebSocketTask,
        didCloseWith closeCode: URLSessionWebSocketTask.CloseCode, reason: Data?
    ) {
        #if DEBUG
        print("🔌 [WebSocket] 连接已关闭: \(closeCode)")
        #endif
        self.handleDisconnection(reason: "didCloseWith: \(closeCode)")
    }
}

// MARK: - URLSessionTaskDelegate

extension ConnectionManager: URLSessionTaskDelegate {
    nonisolated func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        if let error = error {
            #if DEBUG
            print("❌ [WebSocket] 协议错误: \(error.localizedDescription)")
            #endif
            DispatchQueue.main.async {
                self.connectionError = error.localizedDescription
            }
            self.handleDisconnection(reason: "didCompleteWithError: \(error.localizedDescription)")
        }
    }
}

// MARK: - 在线设备信息（用于自动切换）

struct OnlineServerInfo {
    let deviceId: String
    let deviceName: String
    let idleTime: Int  // 秒，距上次活跃的时间
}

// MARK: - 远程 Mac 模型

struct RemoteMac: Identifiable {
    let id: String
    let deviceId: String
    let deviceName: String
    let online: Bool
    let encryptionKey: String?

    init(deviceId: String, deviceName: String, online: Bool, encryptionKey: String? = nil) {
        self.id = deviceId
        self.deviceId = deviceId
        self.deviceName = deviceName
        self.online = online
        self.encryptionKey = encryptionKey
    }
}
