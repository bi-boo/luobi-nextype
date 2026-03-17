# 落笔 Nextype — iOS 端技术架构文档

> 更新时间：2026-03-11 | 代码总量：14 个 Swift 文件，2997 行

---

## 技术栈

| 项目 | 详情 |
|------|------|
| 语言 | Swift 5.0 |
| UI 框架 | SwiftUI（纯 SwiftUI，UIKit 仅用于设备信息和键盘管理） |
| 最低部署版本 | iOS 17.0 |
| Bundle ID | `com.nextype.app` |
| 版本号 | 1.0.0 (Build 1) |
| 界面模式 | 支持暗黑模式（全部使用语义色） |
| 网络安全 | ATS 默认严格模式（WSS 加密传输） |

### 关键框架依赖

| 框架 | 用途 |
|------|------|
| Foundation / UIKit | 基础能力、设备信息、亮度控制 |
| SwiftUI | 全部 UI 层 |
| Combine | 响应式数据绑定 |
| CommonCrypto | AES-256-CBC 加密 |
| Security | Keychain 读写 |

### 第三方服务

| 服务 | 用途 |
|------|------|
| 自建中继服务器 `nextypeapi.yuanfengai.cn:8443` | 公网 WebSocket 中继 |

**零第三方 SDK 依赖**。纯原生实现，仅使用系统框架。

---

## 现有代码结构

```
NextypeApp/NextypeApp/
├── NextypeAppApp.swift          (17行)   应用入口
├── ContentView.swift            (76行)   根视图路由 + 生命周期管理
├── MainInputView.swift         (624行)   主输入界面（重构后）
├── EmptyStateView.swift        (185行)   空状态引导页
├── PairingCodeView.swift       (229行)   配对码输入界面
├── DeviceEditView.swift        (110行)   设备编辑页面
├── SettingsView.swift          (197行)   设置页面（含屏幕常亮/变暗设置）
├── AboutView.swift              (~50行)  关于/使用说明页
├── UsageGuideView.swift         (~80行)  使用场景引导页
├── ConnectionManager.swift     (627行)   统一连接管理器（替代 WebSocketManager + RelayClient）
├── ScreenDimManager.swift      (167行)   屏幕常亮 + 闲置自动变暗
├── EncryptionManager.swift     (214行)   AES 加密/解密
├── PairedMac.swift             (304行)   配对设备模型 + 管理器
└── DeviceIDManager.swift        (94行)   设备 ID 管理（Keychain）
```

总计：14 个文件，约 2997 行代码

### 文件职责详解

| 文件 | 职责 |
|------|------|
| `NextypeAppApp.swift` | `@main` 入口，创建 `WindowGroup` 并加载 `ContentView` |
| `ContentView.swift` | 根路由：无配对设备时显示 `EmptyStateView`，有设备时显示 `MainInputView`；scenePhase 生命周期管理（前台重连/后台断连/屏幕常亮） |
| `MainInputView.swift` | 主输入界面：文本输入、发送/插入/清空按钮（含通用上滑重发手势）、设备切换菜单、连接状态监控、远程控制响应、自动切换设备 |
| `EmptyStateView.swift` | 首次使用引导：配对入口、下载链接复制、从服务器自动恢复配对信息 |
| `PairingCodeView.swift` | 4 位配对码输入 UI + 中继配对逻辑（仅中继，移除 UDP/HTTP 竞速） |
| `DeviceEditView.swift` | 修改设备自定义名称和图标（SF Symbol 图标可选） |
| `SettingsView.swift` | 剪贴板同步开关、惯用手切换、输入字号调节、屏幕常亮/变暗设置 |
| `AboutView.swift` | 关于页面 + 使用说明，包含 UsageGuideView 入口 |
| `UsageGuideView.swift` | 功能使用场景引导 |
| `ConnectionManager.swift` | 统一连接管理器：WebSocket 连接、设备注册、心跳保活（20s 双层心跳）、指数退避重连（1.5^n，最大 30s）、消息收发、配对验证、信任列表同步、在线设备发现 |
| `ScreenDimManager.swift` | 屏幕常亮（isIdleTimerDisabled）+ 闲置自动变暗状态机（30s 倒计时 + 平滑亮度动画 + 窗口触摸唤醒） |
| `EncryptionManager.swift` | AES-256-CBC 加密/解密，兼容 CryptoJS 的 Salted__ 格式，使用 EVP_BytesToKey 密钥派生 |
| `PairedMac.swift` | `PairedMac` 数据模型 + `PairedMacManager` 管理器（CRUD、信任列表同步、Keychain 持久化） |
| `DeviceIDManager.swift` | 单例，生成并缓存 UUID 作为设备唯一标识，存储于 Keychain |

---

## 通信架构

### 统一单连接设计

iOS 端采用统一的 `ConnectionManager` 单连接架构，移除了原有的局域网 WebSocket 双通道。

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│   iPhone    │◄──WSS──►│  中继服务器       │◄──WSS──►│    Mac      │
│  (Client)   │         │ nextypeapi.      │         │  (Server)   │
│             │         │ yuanfengai.cn    │         │             │
└─────────────┘         └──────────────────┘         └─────────────┘
```

- **模拟器专属**：`#if targetEnvironment(simulator)` 编译条件，模拟器自动连接 `ws://localhost:8443`，防止测试设备出现在线上设备列表

### 连接流程

1. 应用启动 → `ContentView.onAppear` → `ConnectionManager.shared.connect()`
2. 连接建立 → 服务器发送 `connected` 消息
3. 收到 `connected` → 注册（`register`）+ 同步信任列表（`sync_trust_list`）+ 延迟 1 秒上报屏幕参数
4. 收到 `registered` → 更新连接状态 = `.connected`
5. 多台配对设备时自动发送 `discover` → 收到 `server_list` → 自动切换到最近活跃设备

### 重连策略

- **指数退避**：延迟 = min(1.5^n 秒, 30 秒)，最大重试 10 次
- **前台恢复**：`scenePhase` 监听，进入前台后调用 `checkAndReconnect()`
- **5 秒兜底**：前台后 5 秒若未触发自动切换，再次发送 `discover`

### 配对流程

```
用户输入 4 位配对码
        │
        ▼
ConnectionManager.verifyPairingCode(_:completion:)
        │
        ▼
中继服务器验证 → pairing_success / pairing_error
        │
        ▼（成功）
保存 PairedMac 到 Keychain
```

---

## 消息协议

### 应用层消息（JSON over WebSocket）

| 消息类型 | 方向 | 字段 | 说明 |
|----------|------|------|------|
| `register` | Client→Server | `role`, `deviceId`, `deviceName` | 注册设备身份 |
| `connected` | Server→Client | — | 连接确认 |
| `registered` | Server→Client | — | 注册成功 |
| `discover` | Client→Server | — | 请求在线设备列表（自动切换用） |
| `server_list` | Server→Client | `servers[]` | 在线服务器列表（含 idleTime） |
| `verify_code` | Client→Server | `code`, `from`, `deviceName` | 配对码验证 |
| `pairing_success` | Server→Client | `server{deviceId, deviceName, encryptionKey}` | 配对成功 |
| `pairing_error` | Server→Client | `message` | 配对失败 |
| `relay` | 双向 | `from`, `to`, `data` | 消息中继转发 |
| `clipboard` | Client→Mac | `content`, `action`, `encrypted`, `timestamp` | 剪贴板内容（AES 加密） |
| `device_info` | Client→Mac | `screenWidth`, `screenHeight`, `density`, `platform` | 屏幕参数上报 |
| `command` / `remote_command` | Mac→Client | `action`, ... | 远程控制指令 |
| `ack` | Mac→Client | — | 消息确认 |
| `sync_trust_list` | Client→Server | — | 请求信任列表同步 |
| `trust_list` | Server→Client | `devices[{id, name}]` | 信任列表数据 |
| `unpair_device` | Client→Server | `targetDeviceId` | 解除配对 |
| `device_unpaired` | Server→Client | `from` | 解除配对通知 |
| `heartbeat` | Client→Server | — | 应用层心跳 |
| `heartbeat_ack` | Server→Client | — | 心跳响应 |
| `server_online` | Server→Client | — | 服务器上线通知 |
| `server_offline` | Server→Client | `serverId` | 服务器离线通知 |
| `error` | Server→Client | `message` | 错误信息 |

### relay.data 远程控制指令（Mac → iOS）

| action | 行为 |
|--------|------|
| `send` | 发送输入框内容并附带回车 |
| `insert` | 插入输入框内容到 Mac 光标位置 |
| `clear` | 清空输入框内容 |

---

## 核心类详解

### 1. ConnectionManager（单例）

**设计模式**：单例 + ObservableObject + 回调闭包

**关键属性**：
- `connectionState: ConnectionState` — 三态枚举：`.disconnected` / `.connecting` / `.connected`
- `currentDevice: PairedMac?` — 当前连接的目标设备
- `webSocket: URLSessionWebSocketTask?` — WSS 连接
- `heartbeatTimer` — 20 秒双层心跳（应用层 `heartbeat` + 协议层 `sendPing`）
- `reconnectAttempts` / `maxReconnectAttempts(10)` — 指数退避重连

**回调接口**：
- `onMessageReceived: ((String, String) -> Void)?` — 收到中继消息
- `onTrustListSync: (([RemoteMac]) -> Void)?` — 信任列表同步
- `onDeviceUnpaired: ((String) -> Void)?` — 解除配对通知
- `onRemoteCommand: ((String, [String: Any]) -> Void)?` — 远程控制指令
- `onServerList: (([OnlineServerInfo]) -> Void)?` — 在线设备列表（自动切换用）

**关键方法**：
- `connect()` / `disconnect()` — 连接管理
- `connectToDevice(_:)` — 连接到指定设备
- `switchToDevice(_:)` — 切换目标设备
- `verifyPairingCode(_:completion:)` — 配对码验证（10 秒超时）
- `relayToServer(serverId:data:)` — 消息转发
- `sendClipboard(content:action:to:)` — 发送加密剪贴板内容
- `sendScreenInfo()` — 上报屏幕参数
- `discoverOnlineDevices()` — 请求在线设备列表
- `checkAndReconnect()` — 检查并按需重连

### 2. PairedMacManager（ObservableObject）

**职责**：管理已配对设备的 CRUD 和持久化

**数据同步**：
- 监听 `ConnectionManager.onTrustListSync` → 与服务器信任列表双向同步
- **空列表保护**：服务器返回空列表且本地有设备时跳过，防止服务器异常误删数据
- 监听 `ConnectionManager.onDeviceUnpaired` → 远程解除配对
- 本地解除配对时通过 `ConnectionManager.sendUnpairRequest()` 通知服务器

**持久化**：全部使用 **Keychain**（kSecAttrAccessibleAfterFirstUnlock）
- 配对设备列表（含 encryptionKey）→ `nextype_paired_macs`
- 上次连接的设备 ID → `nextype_last_connected_device_id`
- **自动迁移**：首次启动时若 UserDefaults 有旧数据，自动迁移至 Keychain 并删除旧值

### 3. EncryptionManager（单例）

**加密方案**：AES-256-CBC，兼容 CryptoJS 格式

**加密流程**：
1. 生成 8 字节随机 Salt（`SecRandomCopyBytes`）
2. EVP_BytesToKey 算法从密码（配对设备 encryptionKey 或 deviceId）+ Salt 派生 32B Key + 16B IV
3. AES-256-CBC + PKCS7 填充加密
4. 输出格式：`Base64("Salted__" + salt + ciphertext)`

**密钥来源**：优先使用配对时交换的 `encryptionKey`，降级时使用本机 `deviceId`

### 4. DeviceIDManager（单例）

**职责**：生成并持久化设备唯一标识 UUID

**存储**：Keychain（`kSecAttrAccessibleAfterFirstUnlock`）
- 首次启动自动从 UserDefaults 迁移旧值到 Keychain

### 5. ScreenDimManager（单例）

**状态机**：
```
活跃 ──30s无操作──► 变暗（0.01 亮度，300ms 平滑动画）
变暗 ──触摸/交互──► 活跃（恢复原亮度，重置 30s 倒计时）
```

**窗口级触摸检测**：在 `UIWindow` 上添加 `AnyTouchRecognizer`（`state = .failed`，不消费事件），确保变暗后任意触摸均可唤醒。

---

## 数据存储

### Keychain Key 清单

| Account | 类型 | 说明 |
|---------|------|------|
| `nextype_device_id` | String | 设备唯一标识（UUID） |
| `nextype_paired_macs` | Data (JSON) | 已配对设备列表（含 encryptionKey） |
| `nextype_last_connected_device_id` | String | 上次连接的设备 ID |

### @AppStorage Key 清单

| Key | 类型 | 默认值 | 说明 |
|-----|------|--------|------|
| `pasteCopiesToClipboard` | Bool | true | 插入时同步剪贴板 |
| `pasteEnterCopiesToClipboard` | Bool | true | 发送时同步剪贴板 |
| `handMode` | String | "right" | 惯用手（left/right） |
| `inputFontSizeIndex` | Int | 1 | 输入框字号档位（0-4，对应 16/18/20/24/28pt） |
| `hasShownSwipeHint` | Bool | false | 首次上滑提示标记 |
| `keepScreenOn` | Bool | true | 屏幕常亮开关 |
| `autoDimEnabled` | Bool | true | 自动变暗开关 |
| `autoDimTimeout` | Int | 30 | 自动变暗超时（秒） |
| `skipPairing` | Bool | false | 跳过配对进入主界面 |

---

## 加密方案

### AES-256-CBC（兼容 CryptoJS）

```
配对时交换的 encryptionKey（或降级为 iPhone deviceId）
        │
        ▼
EVP_BytesToKey(password, salt)
  ├── MD5(password + salt) → block1
  ├── MD5(block1 + password + salt) → block2
  └── MD5(block2 + password + salt) → block3
        │
        ▼
  Key = block1 + block2 (32 bytes)
  IV  = block3 (16 bytes)
        │
        ▼
AES-256-CBC + PKCS7 Padding
        │
        ▼
输出 = Base64("Salted__" + salt(8B) + ciphertext)
```

**实现细节**：
- 使用 `CommonCrypto` 的 `CCCrypt` 函数
- Salt 由 `SecRandomCopyBytes` 生成（密码学安全随机数）
- MD5 用于密钥派生（`CC_MD5`），已标注 `@available(iOS, deprecated:)` 警告

---

## 架构决策记录

### 1. 移除双通道，统一为 ConnectionManager

**决策**：删除 `WebSocketManager.swift` 和 `RelayClient.swift`，合并为 `ConnectionManager.swift` 单连接架构。

**理由**：与 Android 端对齐（统一单条 WebSocket），减少连接资源占用和状态同步复杂度，消除职责重叠。

### 2. 密钥存储迁移至 Keychain

**决策**：设备 ID、配对设备列表（含 encryptionKey）、上次连接设备 ID 全部迁移至 Keychain。

**理由**：Keychain 在卸载重装后可保留数据（`kSecAttrAccessibleAfterFirstUnlock`），提升用户体验；同时满足 App Store 安全要求，加密密钥不应存储于明文的 UserDefaults。

### 3. 模拟器隔离

**决策**：通过 `#if targetEnvironment(simulator)` 让模拟器连接本地 relay server（`ws://localhost:8443`）。

**理由**：防止 Xcode 模拟器运行时，以模拟器设备名（如"iPhone 16e"）注册到线上 relay server，导致陌生设备出现在用户的配对列表中。

### 4. 自动切换设备（Follow the Light）

**决策**：连接成功或回到前台时发送 `discover` 请求，基于服务器返回的 `idleTime` 自动切换到最近活跃的电脑。

**约束**：每次前台仅切换一次（`hasAutoSwitchedThisResume` 标志），防止重复触发；候选设备必须在配对列表中且 `idleTime < 120s`。

---

## 依赖项清单

### 系统框架

| 框架 | 引用文件 | 是否必需 |
|------|----------|----------|
| SwiftUI | 所有视图文件 | 是 |
| Foundation | 所有文件 | 是 |
| UIKit | ConnectionManager, ContentView, ScreenDimManager, DeviceIDManager | 是（设备信息、亮度控制） |
| Combine | ConnectionManager, PairedMac | 是 |
| CommonCrypto | EncryptionManager | 是（AES 加密） |
| Security | DeviceIDManager, PairedMac | 是（Keychain） |

### 第三方服务依赖

| 服务 | 端点 | 协议 | 用途 |
|------|------|------|------|
| 自建中继服务器 | `wss://nextypeapi.yuanfengai.cn:8443` | WebSocket (WSS) | 设备注册、配对、消息中继 |
