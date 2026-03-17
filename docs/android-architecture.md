# 落笔 Nextype — Android 端技术架构文档

## 技术栈

| 类别 | 技术 | 版本 |
|------|------|------|
| 语言 | Kotlin | JVM Target 1.8 |
| Android SDK | compileSdk / targetSdk | 34 (Android 14) |
| 最低支持 | minSdk | 24 (Android 7.0) |
| 构建工具 | Gradle (Kotlin DSL) | — |
| UI 框架 | Android View (XML 布局) | — |
| 网络通信 | OkHttp 3 | 4.12.0 |
| 异步框架 | Kotlin Coroutines | 1.7.3 |
| 生命周期 | AndroidX Lifecycle | 2.6.1 |
| UI 组件库 | Material Design 3 | 1.11.0 |
| 核心库 | AndroidX Core KTX | 1.12.0 |
| 兼容库 | AndroidX AppCompat | 1.6.1 |

---

## 项目结构与类职责

```
NextypeAndroid/app/src/main/java/com/nextype/android/
├── NextypeApplication.kt          # Application 子类，全局单例，处理解除配对通知
├── MainActivity.kt                # 主界面，文本输入/发送、WebSocket 数据通道、远程控制、屏幕管理
├── EmptyStateActivity.kt          # 欢迎页，首次启动引导、配对恢复
├── PairingActivity.kt             # 配对页，4位配对码输入与公网中继配对
├── SettingsActivity.kt            # 设置页，所有用户偏好配置
├── AboutActivity.kt               # 使用说明页，产品介绍与帮助
├── DeviceListActivity.kt          # [残留] 局域网设备列表页（已废弃，未使用）
├── RelayClient.kt                 # 公网中继客户端，配对/信任列表/设备发现等控制消息
├── DeviceDiscoveryService.kt      # [残留] 局域网设备发现服务（mDNS + UDP，已废弃）
├── DeviceIDManager.kt             # 设备ID管理器，基于 ANDROID_ID 生成稳定唯一标识
├── PairedDevice.kt                # 配对设备数据模型 + PairedDeviceManager 多设备管理器
├── NextypeAccessibilityService.kt # 辅助功能服务，模拟点击/长按、全局触摸检测
├── WakeUpEditText.kt              # 自定义 EditText，拦截软键盘输入触发屏幕唤醒
├── HTTPPairingClient.kt           # [残留] HTTP 配对客户端（已废弃，未使用）
└── UDPPairingClient.kt            # [残留] UDP 广播配对客户端（已废弃，未使用）
```

### 各文件职责详述

| 文件 | 行数 | 职责 |
|------|------|------|
| `MainActivity.kt` | ~3200 | 应用核心，承载文本输入 UI、数据 WebSocket 连接管理、消息加密发送、远程指令处理、设备切换、屏幕常亮/变暗状态机、折叠屏适配 |
| `RelayClient.kt` | ~461 | 独立的中继服务器 WebSocket 客户端，用于配对验证、信任列表同步、设备发现、解除配对、备注名同步等控制类操作 |
| `NextypeAccessibilityService.kt` | ~309 | Android AccessibilityService 实现，提供模拟点击（GestureDescription）、可续传长按（willContinue）、全局触摸事件检测 |
| `PairedDevice.kt` | ~243 | 数据模型 `PairedDevice` 和管理器 `PairedDeviceManager`，负责多设备的 CRUD、JSON 序列化、旧格式迁移 |
| `SettingsActivity.kt` | ~678 | 设置页面，管理所有用户偏好（惯用手、字号、剪贴板、屏幕常亮、辅助功能状态检测） |
| `DeviceDiscoveryService.kt` | ~264 | 局域网设备发现（mDNS + UDP 广播 + USB 端口扫描），当前已被公网中继取代 |
| `DeviceIDManager.kt` | ~100 | 单例模式，基于 `ANDROID_ID` + MD5 生成 16 位十六进制设备标识，支持缓存和持久化 |
| `WakeUpEditText.kt` | ~98 | 通过包装 `InputConnection` 拦截所有软键盘操作（commitText、deleteSurroundingText 等），解决键盘独立窗口无法触发 Activity 触摸事件的问题 |

---

## 通信架构

### 整体设计

应用采用「公网中继」架构，所有手机与 PC 之间的通信都通过中继服务器（`wss://nextypeapi.yuanfengai.cn:8443`）转发。

```
┌──────────┐     WebSocket      ┌──────────────┐     WebSocket      ┌──────────┐
│  Android  │ ◄──────────────► │  中继服务器    │ ◄──────────────► │   PC 端   │
│  (Client) │                   │ (Relay Server)│                   │ (Server)  │
└──────────┘                   └──────────────┘                   └──────────┘
```

### WebSocket 连接分类

应用中存在两种 WebSocket 连接，职责不同：

#### 1. 数据通道（MainActivity.dataWebSocket）

- 位置：`MainActivity.kt` 中的 `dataWebSocket` 字段
- 用途：文本传输、远程控制指令、心跳、设备上下线通知
- 生命周期：跟随 Activity 前后台状态（onStop 断开，onResume 重连）
- 注册身份：`deviceId`（不带后缀），角色 `client`
- 心跳：每 30 秒发送 `{"type":"heartbeat"}`
- 重连策略：指数退避（2s → 4s → 8s → ... → 30s），最多 10 次

#### 2. 控制通道（RelayClient）

- 位置：`RelayClient.kt` 独立类
- 用途：配对验证、信任列表同步、设备发现、解除配对通知、备注名同步
- 生命周期：按需创建，操作完成后断开（短连接模式）
- 注册身份：`deviceId_sync`（带 `_sync` 后缀，避免与数据通道冲突）
- 心跳：每 30 秒发送 `{"type":"heartbeat"}`（应用层心跳）
- OkHttp ping：每 60 秒（协议层心跳）

### 连接管理策略

```
onResume（前台恢复）
  ├── 重建 dataWebSocket → connectToRelay(deviceId)
  ├── 启动连接监控（每 60 秒兜底检查）
  ├── 触发自动切换（Follow the Light）
  └── 5 秒兜底：如果同步通道未就绪，强制执行一次自动切换

onStop（进入后台）
  ├── 断开 dataWebSocket
  ├── 停止心跳
  ├── 停止连接监控
  └── 停止变暗倒计时
```

---

## 消息协议

### 中继服务器消息（JSON over WebSocket）

#### 发送的消息

| type | 用途 | 发送方 | 关键字段 |
|------|------|--------|----------|
| `register` | 注册设备 | 两个通道 | `role`, `deviceId`, `deviceName` |
| `heartbeat` | 心跳保活 | 两个通道 | `timestamp`（数据通道）/ 无（控制通道） |
| `verify_code` | 验证配对码 | RelayClient | `code`, `from`, `deviceName` |
| `relay` | 中继转发 | 数据通道 | `from`, `to`, `data`（内嵌 JSON 字符串） |
| `sync_trust_list` | 请求信任列表 | RelayClient | — |
| `unpair_device` | 解除配对 | RelayClient | `targetDeviceId` |
| `set_device_alias` | 设置备注名 | RelayClient | `targetDeviceId`, `alias` |
| `discover` | 查询在线设备 | 两个通道 | — |
| `check_online_status` | 批量查询在线状态 | 临时连接 | `deviceIds`（JSON 数组） |
| `ping` | 上线通知 | 数据通道 | `timestamp` |

#### 接收的消息

| type | 用途 | 关键字段 |
|------|------|----------|
| `connected` | 服务器欢迎消息 | — |
| `registered` | 注册成功确认 | — |
| `pairing_success` | 配对成功 | `server.deviceId`, `server.deviceName` |
| `pairing_error` | 配对失败 | `message` |
| `trust_list` | 信任列表响应 | `devices[]`（id, name, customName） |
| `server_list` | 在线服务器列表 | `servers[]`（deviceId, deviceName, idleTime） |
| `server_online` | PC 设备上线 | `serverId`, `serverName` |
| `server_offline` | PC 设备下线 | `serverId` |
| `device_unpaired` | 解除配对通知 | — |
| `relay` | 中继转发（来自 PC） | `data`（内嵌 JSON） |
| `online_status_result` | 在线状态查询结果 | `devices[]`（deviceId, online） |
| `ack` | PC 端确认收到 | — |
| `error` | 服务器错误 | `message` |

#### relay.data 内嵌消息

##### 发送给 PC 的内嵌消息

| type | 用途 | 关键字段 |
|------|------|----------|
| `clipboard` | 发送加密文本 | `content`（Base64 密文）, `action`（paste/paste-enter）, `encrypted` |
| `ping` | 上线通知 | `timestamp` |
| `device_info` | 屏幕参数上报 | `screenWidth`, `screenHeight` |
| `screen_changed` | 屏幕尺寸变更 | `screenWidth`, `screenHeight` |
| `error` | 错误通知 | `errorType`, `errorTitle`, `errorMessage` |

##### 从 PC 接收的内嵌消息

| type | action | 用途 |
|------|--------|------|
| `command` | `send` | 触发发送（粘贴+回车） |
| `command` | `insert` | 触发插入（仅粘贴） |
| `command` | `clear` | 触发清空 |
| `command` | `tap` | 模拟点击（x, y 坐标） |
| `command` | `touch_down` | 长按按下（x, y 坐标） |
| `command` | `touch_up` | 长按释放 |
| `command` | `touch_heartbeat` | 长按心跳 |

---

## 核心类详解

### MainActivity（主界面控制器）

**文件**：`MainActivity.kt`（约 3200 行）
**职责**：应用的核心 Activity，承载文本输入 UI、数据通道管理、远程控制、屏幕管理等所有主要功能。

#### 关键属性

```kotlin
// WebSocket 数据通道
private var dataWebSocket: WebSocket? = null
private var isWebSocketConnected = false
private val client = OkHttpClient.Builder()
    .pingInterval(30, TimeUnit.SECONDS).build()

// 当前连接的目标设备
private var deviceId: String? = null
private var deviceName: String? = null

// 屏幕常亮 & 自动变暗状态机
private var isKeepScreenOn = false
private var isAutoDimEnabled = true
private var autoDimTimeoutMs = 30_000L
private var isDimmed = false

// 断线重连
private var dataReconnectAttempts = 0
private val maxDataReconnectAttempts = 10

// 上滑重发/恢复
private var lastSentContent: String? = null
private var lastClearedContent: String? = null
```

#### 关键方法

| 方法签名 | 职责 |
|----------|------|
| `onCreate(savedInstanceState: Bundle?)` | 初始化 UI、Edge-to-edge 适配、挖孔屏适配、折叠屏旋转策略、WebSocket 连接启动 |
| `connectToRelay(targetDeviceId: String)` | 建立数据 WebSocket 连接，注册设备，处理所有消息类型 |
| `sendContent(pressEnter: Boolean)` | 加密输入内容并通过 relay 消息发送到 PC |
| `encryptContent(content: String, key: String): String` | AES-256-CBC 加密，CryptoJS 兼容格式 |
| `handleRemoteCommand(action: String, commandJson: JSONObject)` | 分发 PC 端远程指令（send/insert/clear/tap/touch_down/touch_up） |
| `autoSwitchToActiveDevice()` | Follow the Light 策略，自动切换到最近活跃的 PC |
| `switchToDevice(device: PairedDevice)` | 切换目标设备，断开旧连接并建立新连接 |
| `dimScreen()` | 执行屏幕变暗动画（300ms 过渡到 0.01f 亮度） |
| `wakeUpScreen()` | 恢复正常亮度并重置变暗倒计时 |
| `dispatchTouchEvent(ev: MotionEvent?): Boolean` | 拦截触摸事件，变暗状态下防误触 |
| `onConfigurationChanged(newConfig: Configuration)` | 检测折叠屏状态变化，上报屏幕参数 |
| `checkScreenOrientation(width: Int, height: Int)` | 根据屏幕宽高比动态锁定/解锁旋转 |
| `setHuaweiNotchSupport()` | 华为/荣耀挖孔屏适配（元反射设置 hwFlags） |
| `showDeviceSwitchMenu(anchor: View)` | 显示设备选择 BottomSheetDialog |
| `scheduleDataReconnect()` | 指数退避重连调度 |
| `startHeartbeat()` / `stopHeartbeat()` | 管理 30 秒间隔的应用层心跳 |
| `sendScreenInfoToPC(eventType: String)` | 通过 relay 消息上报屏幕尺寸 |

#### 生命周期管理

```
onCreate → 初始化 UI + 延迟 1 秒启动连接
onResume → 重建连接 + 启动监控 + 自动切换 + 弹出键盘
onStop   → 断开所有连接 + 停止心跳 + 停止监控（零后台耗电）
onDestroy → 清理回调 + 释放资源
```

#### 内部类

- `DeviceSelectorAdapter`：设备选择弹窗的 RecyclerView 适配器，管理设备列表、在线状态、连接状态的 UI 更新

---

### RelayClient（中继控制客户端）

**文件**：`RelayClient.kt`（461 行）
**设计模式**：回调模式（callback-based），短连接使用

#### 关键属性

```kotlin
private val serverUrl = "wss://nextypeapi.yuanfengai.cn:8443"
private var webSocket: WebSocket? = null
private var isConnected = false

// 各类操作的回调
private var pairingCallback: ((Result<PairingResponse>) -> Unit)? = null
private var trustListCallback: ((List<TrustDeviceInfo>) -> Unit)? = null
private var onlineDevicesCallback: ((List<OnlineDeviceInfo>) -> Unit)? = null
var serverStatusCallback: ((String, String, Boolean) -> Unit)? = null
```

#### 关键方法

| 方法签名 | 职责 |
|----------|------|
| `connect(context: Context, onConnected: () -> Unit)` | 建立 WebSocket 连接，连接成功后自动注册并同步信任列表 |
| `suspend verifyPairingCode(code: String, context: Context): Result<PairingResponse>` | 挂起函数，发送配对码验证请求，10 秒超时 |
| `syncTrustList(callback: (List<TrustDeviceInfo>) -> Unit)` | 请求服务器返回信任设备列表 |
| `syncTrustListInternal()` | 注册成功后自动调用，以服务器为准覆盖本地配对列表 |
| `discoverOnlineDevices(callback: (List<OnlineDeviceInfo>) -> Unit)` | 查询在线 PC 列表，带 3 秒等待连接机制 |
| `sendUnpairRequest(targetDeviceId: String)` | 发送解除配对请求 |
| `setDeviceAlias(targetDeviceId: String, alias: String?)` | 同步设备备注名到服务器 |
| `disconnect()` | 正常关闭 WebSocket 连接 |

#### 注册身份

RelayClient 注册时使用 `deviceId_sync` 后缀，避免与 MainActivity 的数据通道（使用原始 `deviceId`）在服务器端冲突。

---

### NextypeAccessibilityService（辅助功能服务）

**文件**：`NextypeAccessibilityService.kt`（309 行）
**设计模式**：单例引用（`companion object` 中的 `instance`）+ 全局回调

#### 核心能力

1. **模拟点击**：通过 `GestureDescription` API 在指定屏幕坐标执行 50ms 点击手势
2. **可续传长按**：使用 `StrokeDescription(willContinue=true)` 实现无限时长长按
3. **全局触摸检测**：监听 `TYPE_TOUCH_INTERACTION_START` 事件，检测屏幕任意位置触摸

#### 关键方法

| 方法签名 | 职责 |
|----------|------|
| `performTap(x: Float, y: Float)` | 在指定坐标执行模拟点击（API 24+） |
| `performTouchDown(x: Float, y: Float)` | 开始长按（API 26+，willContinue=true） |
| `performTouchUp()` | 结束长按（continueStroke + willContinue=false） |
| `onHeartbeat()` | 刷新长按心跳计时器，防止超时自动释放 |
| `performLegacyLongPress(x: Float, y: Float)` | 降级方案：固定 800ms 长按（API 24-25） |

#### 长按心跳机制

```
PC 端发送 touch_down → performTouchDown() → 启动心跳检测（每 200ms）
PC 端持续发送 touch_heartbeat → onHeartbeat() → 刷新 lastHeartbeatTime
PC 端发送 touch_up → performTouchUp() → 结束手势
超时（1 秒无心跳）→ 自动调用 performTouchUp() 释放
```

---

### DeviceIDManager（设备ID管理器）

**文件**：`DeviceIDManager.kt`（100 行）
**设计模式**：双重检查锁定单例（DCL Singleton）

#### 生成算法

```
输入: ANDROID_ID + "nextype_salt_v1"
算法: MD5 哈希
输出: 取前 16 个十六进制字符
```

#### 特性

- 卸载重装后 ID 不变（基于 ANDROID_ID）
- 恢复出厂设置后 ID 改变（ANDROID_ID 重置）
- 三级缓存：内存缓存 → SharedPreferences → 实时计算
- 向后兼容：优先读取已保存的旧 ID

---

### PairedDevice & PairedDeviceManager（配对设备管理）

**文件**：`PairedDevice.kt`（243 行）

#### PairedDevice 数据模型

```kotlin
data class PairedDevice(
    val deviceId: String,       // 设备唯一标识
    val deviceName: String,     // 原始名称（PC 端提供）
    val host: String,           // 主机地址（当前固定为 "relay"）
    val port: Int,              // 端口（当前固定为 8080）
    val pairedAt: Long,         // 配对时间戳
    val customName: String?,    // 用户自定义别名
    val customIcon: String      // 图标标识（"laptop" / "desktop"）
)
```

#### PairedDeviceManager 关键方法

| 方法签名 | 职责 |
|----------|------|
| `getPairedDevices(): List<PairedDevice>` | 获取所有配对设备（支持旧格式自动迁移） |
| `addDevice(device: PairedDevice)` | 添加或更新设备（按 deviceId 去重） |
| `removeDevice(deviceId: String)` | 移除设备并清理 lastConnectedDeviceId |
| `updateDevice(deviceId, customName, customIcon)` | 更新自定义属性 |
| `updateDeviceName(deviceId, newName)` | 更新原始名称（不影响备注名） |
| `getLastConnectedDevice(): PairedDevice?` | 获取上次连接的设备，不存在则返回第一个 |
| `migrateFromOldFormat(): List<PairedDevice>` | 从单设备旧格式迁移到多设备列表 |

#### 存储格式

使用 SharedPreferences（`NextypeDevices`），以 JSON 数组字符串存储设备列表。

---

### WakeUpEditText（唤醒输入框）

**文件**：`WakeUpEditText.kt`（98 行）
**设计模式**：装饰器模式（InputConnectionWrapper）

#### 解决的问题

Android 软键盘是独立窗口，触摸事件不经过 `Activity.dispatchTouchEvent()`，导致键盘上的操作无法触发屏幕唤醒。

#### 实现原理

```
WakeUpEditText
  └── onCreateInputConnection() → 返回 WakeUpInputConnection
        └── WakeUpInputConnection extends InputConnectionWrapper
              ├── commitText()        → notifyActivity() + super
              ├── setComposingText()  → notifyActivity() + super
              ├── deleteSurroundingText() → notifyActivity() + super
              ├── sendKeyEvent()      → notifyActivity() + super
              └── ...（所有输入操作均拦截）
```

注意：`InputConnection` 的方法由输入法进程通过 Binder 线程调用，`notifyActivity()` 内部使用 `Handler(Looper.getMainLooper())` 切回主线程。

---

## 数据存储

应用使用 SharedPreferences 进行本地数据持久化，不使用数据库。

### SharedPreferences 文件清单

#### 1. `NextypeDevices`（配对设备存储）

| Key | 类型 | 说明 |
|-----|------|------|
| `pairedDevicesList` | String (JSON Array) | 所有配对设备列表，每个元素包含 deviceId、deviceName、host、port、pairedAt、customName、customIcon |
| `lastConnectedDeviceId` | String | 上次连接的设备 ID |
| `pairedDeviceId` | String | [旧格式] 单设备 ID，用于迁移 |
| `pairedDeviceName` | String | [旧格式] 单设备名称，用于迁移 |
| `pairedDeviceHost` | String | [旧格式] 单设备主机地址，用于迁移 |
| `pairedDevicePort` | Int | [旧格式] 单设备端口，用于迁移 |
| `pairedAt` | Long | [旧格式] 配对时间戳，用于迁移 |

#### 2. `NextypeDeviceID`（设备标识存储）

| Key | 类型 | 说明 |
|-----|------|------|
| `nextype_device_id` | String | 16 位十六进制设备唯一标识 |
| `legacy_id_migrated` | Boolean | 旧版 ID 迁移标记（预留，当前未使用） |

#### 3. `NextypeSettings`（用户设置存储）

| Key | 类型 | 默认值 | 说明 |
|-----|------|--------|------|
| `pasteCopiesToClipboard` | Boolean | true | 插入时同步到剪贴板 |
| `pasteEnterCopiesToClipboard` | Boolean | true | 发送时同步到剪贴板 |
| `showVoiceInputButton` | Boolean | false | 显示语音输入按钮（当前隐藏） |
| `handMode` | String | "right" | 惯用手（"left" / "right"） |
| `inputFontSize` | Int | 1 | 字号档位（0-4，对应 16/18/20/24/28sp） |
| `keepScreenOn` | Boolean | true | 屏幕常亮开关 |
| `autoDimEnabled` | Boolean | true | 闲置自动变暗开关 |
| `autoDimTimeout` | Int | 60000 | 变暗等待时间（毫秒） |

#### 4. `SwipeHintPrefs`（上滑提示状态）

| Key | 类型 | 说明 |
|-----|------|------|
| `hintShown_global` | Boolean | 是否已显示过上滑操作提示（全局单次） |

---

## 权限清单

### AndroidManifest.xml 声明的权限

| 权限 | 用途 |
|------|------|
| `android.permission.INTERNET` | WebSocket 连接（中继服务器） |
| `android.permission.ACCESS_NETWORK_STATE` | 检测网络连接状态 |
| `android.permission.ACCESS_WIFI_STATE` | 获取 WiFi 连接信息（局域网发现，当前残留） |
| `android.permission.CHANGE_WIFI_MULTICAST_STATE` | mDNS 多播支持（局域网发现，当前残留） |
| `android.permission.CHANGE_NETWORK_STATE` | 网络状态变更（局域网发现，当前残留） |
| `android.permission.VIBRATE` | 触觉反馈（按钮点击、语音录音开始/结束） |
| `android.permission.RECORD_AUDIO` | 麦克风录音（语音识别，运行时动态申请） |

### 特殊权限

| 权限 | 用途 |
|------|------|
| `BIND_ACCESSIBILITY_SERVICE` | 辅助功能服务绑定权限（系统级，需用户在设置中手动开启） |

### 应用级配置

| 配置项 | 值 | 说明 |
|--------|-----|------|
| `usesCleartextTraffic` | true | 允许明文 HTTP（局域网配对残留，当前实际只用 WSS） |
| `resizeableActivity` | true | 支持分屏/折叠屏多窗口 |
| `android.max_aspect` | 99.0 | 支持所有屏幕宽高比，解除折叠屏 Letterboxing |
| `android.notch_support` | true | 华为/荣耀挖孔屏支持声明 |

---

## 生命周期管理

### Activity 生命周期与连接管理

```
┌─────────────────────────────────────────────────────────────┐
│                    EmptyStateActivity                         │
│  onCreate → 尝试从服务器恢复配对 → 有配对设备则跳转 MainActivity │
│  onResume → 检查是否已有配对设备 → 有则跳转 MainActivity        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      MainActivity                            │
│                                                              │
│  onCreate:                                                   │
│    ├── 初始化 UI（Edge-to-edge、挖孔屏、折叠屏适配）            │
│    ├── 初始化 PairedDeviceManager                              │
│    ├── 读取设置（剪贴板、屏幕常亮、字号）                       │
│    ├── 延迟 1 秒：checkOnlineAndAutoSwitch → connectToRelay   │
│    └── 延迟 300ms：弹出软键盘                                  │
│                                                              │
│  onResume（热启动）:                                          │
│    ├── 重建 dataWebSocket 连接                                │
│    ├── 启动连接监控（每 60 秒兜底检查）                         │
│    ├── 触发 Follow the Light 自动切换                          │
│    ├── 刷新设备状态、设置、字号                                 │
│    ├── 恢复屏幕亮度 + 重置变暗倒计时                            │
│    └── 延迟 500ms：弹出软键盘                                  │
│                                                              │
│  onStop（进入后台）:                                          │
│    ├── 断开 dataWebSocket                                     │
│    ├── 停止心跳                                               │
│    ├── 停止连接监控                                            │
│    └── 停止变暗倒计时                                          │
│                                                              │
│  onDestroy:                                                  │
│    ├── 取消连接监控定时器                                      │
│    ├── 关闭 dataWebSocket                                     │
│    ├── 取消亮度动画                                            │
│    └── 清理 AccessibilityService 全局回调                      │
└─────────────────────────────────────────────────────────────┘
```

### 后台零耗电策略

应用采用「后台全断」策略：

1. `onStop` 时断开所有 WebSocket 连接，停止所有定时器
2. `onResume` 时重建所有连接，恢复所有状态
3. 不使用前台 Service，不保持后台连接
4. 断线重连仅在前台状态下执行

### 断线重连策略

```
重连间隔 = min(2000ms * 2^(attempt-1), 30000ms)

第 1 次: 2 秒
第 2 次: 4 秒
第 3 次: 8 秒
第 4 次: 16 秒
第 5 次: 30 秒（达到上限）
...
第 10 次: 30 秒（达到最大重试次数，停止重连）
```

触发条件：
- WebSocket `onFailure` 回调
- WebSocket `onClosed` 且关闭码非 1000（非正常关闭）
- 心跳发送失败

---

## 加密方案

### AES-256-CBC 加密（CryptoJS 兼容格式）

应用使用与 CryptoJS 完全兼容的加密格式，确保 Android 端加密的内容可以在 PC 端（JavaScript）正确解密。

#### 加密流程

```
1. 生成 8 字节随机 Salt
2. 使用 EVP_BytesToKey 算法从密码和 Salt 派生 Key(32字节) 和 IV(16字节)
3. 使用 AES/CBC/PKCS5Padding 加密明文
4. 组装 CryptoJS 格式: "Salted__" + Salt(8字节) + 密文
5. Base64 编码输出
```

#### EVP_BytesToKey 算法

```kotlin
private fun deriveKeyAndIV(password: String, salt: ByteArray): Pair<ByteArray, ByteArray> {
    val passwordBytes = password.toByteArray()
    val keySize = 32   // AES-256
    val ivSize = 16    // CBC IV
    val md = MessageDigest.getInstance("MD5")
    val derivedData = ByteArrayOutputStream()
    var block = ByteArray(0)

    while (derivedData.size() < keySize + ivSize) {
        md.reset()
        md.update(block)          // 上一轮的 MD5 结果
        md.update(passwordBytes)  // 密码
        md.update(salt)           // Salt
        block = md.digest()
        derivedData.write(block)
    }

    val derived = derivedData.toByteArray()
    return Pair(
        derived.copyOfRange(0, keySize),           // Key
        derived.copyOfRange(keySize, keySize + ivSize)  // IV
    )
}
```

#### 密钥来源

加密密钥为 Android 端的 `deviceId`（16 位十六进制字符串），PC 端在配对时已获取该 ID，双方使用相同密钥。

#### 输出格式

```
Base64( "Salted__" + salt[8] + AES_CBC_encrypt(plaintext) )
```

此格式与 `CryptoJS.AES.encrypt(content, key).toString()` 的输出完全一致。

---

## 依赖项清单

### 运行时依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `androidx.core:core-ktx` | 1.12.0 | Kotlin 扩展函数，简化 Android API 调用 |
| `androidx.appcompat:appcompat` | 1.6.1 | 向后兼容的 Activity、Toolbar 等组件 |
| `com.google.android.material:material` | 1.11.0 | Material Design 3 组件（MaterialButton、BottomSheetDialog、MaterialCardView、Slider、SwitchCompat） |
| `androidx.constraintlayout:constraintlayout` | 2.1.4 | **[未使用]** 约束布局，项目中所有布局均使用 LinearLayout/FrameLayout |
| `androidx.lifecycle:lifecycle-runtime-ktx` | 2.6.1 | `lifecycleScope` 协程作用域，用于 Activity 内的协程管理 |
| `org.jetbrains.kotlinx:kotlinx-coroutines-android` | 1.7.3 | Kotlin 协程 Android 调度器，用于异步操作 |
| `com.squareup.okhttp3:okhttp` | 4.12.0 | WebSocket 客户端（中继服务器通信） |

### 测试依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `junit:junit` | 4.13.2 | 单元测试框架 |
| `androidx.test.ext:junit` | 1.1.5 | AndroidX 测试扩展 |
| `androidx.test.espresso:espresso-core` | 3.5.1 | UI 自动化测试 |

### Gradle 插件

| 插件 | 用途 |
|------|------|
| `com.android.application` | Android 应用构建插件 |
| `org.jetbrains.kotlin.android` | Kotlin Android 编译插件 |

---

## 废弃与未使用代码

### 已注释代码

- **[已注释]** `SettingsActivity.kt:49-52` — 设备管理 UI 组件声明（`pairedDevicesLabel`、`devicesListContainer`、`addDeviceButton`），已移至首页弹层
- **[已注释]** `SettingsActivity.kt:168-171` — 设备管理 UI 初始化
- **[已注释]** `SettingsActivity.kt:269-273` — 添加新设备按钮点击事件
- **[已注释]** `SettingsActivity.kt:395-460` — `loadPairedDevices` 方法（设备列表渲染），已移至首页弹层
- **[已注释]** `SettingsActivity.kt:596` — `loadPairedDevices()` 调用
- **[已注释]** `SettingsActivity.kt:598` — `syncServerPairingStatus()` 调用

### 未调用代码

- **[未调用]** `SettingsActivity.kt:462-471` — `getIconResourceForType()` 方法，仅在已注释的 `loadPairedDevices` 中使用
  - 建议：删除，或保留等待设备管理 UI 回归设置页时使用
- **[未调用]** `SettingsActivity.kt:476-548` — `showEditDeviceDialog()` 方法，仅在已注释的 `loadPairedDevices` 中使用
  - 建议：删除，MainActivity 中已有同名方法承担此功能
- **[未调用]** `SettingsActivity.kt:554-587` — `sendUnpairNotification()` 方法，仅在已注释的 `loadPairedDevices` 中使用
  - 建议：删除，MainActivity 中已有同名方法承担此功能
- **[未调用]** `SettingsActivity.kt:605-646` — `syncServerPairingStatus()` 方法，调用已被注释
  - 建议：删除，信任列表同步已由 RelayClient.syncTrustListInternal() 在启动时自动完成
- **[未调用]** `PairedDevice.kt:53-59` — `AVAILABLE_ICONS` 列表定义了 6 种图标（laptop/desktop/imac/macmini/monitor/server），但编辑弹窗实际只使用 laptop 和 desktop
  - 建议：保留，后续可能扩展图标选择

### 废弃标注

- **[废弃标注]** `MainActivity.kt:721-723` — `connectControlChannel()` 方法，注释标注"已废弃，完全由 NextypeApplication 托管"，方法体为空日志输出
  - 建议：删除

### 残留代码

- **[残留代码]** `MainActivity.kt:631-640` — `connectToServer()` 方法，内部直接调用 `connectToRelay()`，是早期局域网连接的残留封装
  - 建议：删除，将调用方直接改为 `connectToRelay()`
- **[残留代码]** `DeviceDiscoveryService.kt` — 整个文件（264 行），mDNS + UDP 局域网发现服务，当前配对流程已完全使用公网中继
  - 建议：删除整个文件
- **[残留代码]** `DeviceListActivity.kt` — 整个文件（121 行），局域网设备列表页面，未在任何地方被启动
  - 建议：删除整个文件及对应布局 `activity_device_list.xml`
- **[残留代码]** `UDPPairingClient.kt` — 整个文件（85 行），UDP 广播配对客户端，已被 RelayClient 取代
  - 建议：删除整个文件
- **[残留代码]** `HTTPPairingClient.kt` — 整个文件（81 行），HTTP 配对客户端，已被 RelayClient 取代
  - 建议：删除整个文件

### 未使用依赖

- **[未使用依赖]** `build.gradle.kts:39` — `androidx.constraintlayout:constraintlayout:2.1.4`，项目中所有布局均使用 LinearLayout/FrameLayout，未使用 ConstraintLayout
  - 建议：移除依赖，减小 APK 体积

### 残留权限

- **[残留权限]** `AndroidManifest.xml:7` — `ACCESS_WIFI_STATE`，局域网发现残留
- **[残留权限]** `AndroidManifest.xml:8` — `CHANGE_WIFI_MULTICAST_STATE`，mDNS 多播残留
- **[残留权限]** `AndroidManifest.xml:9` — `CHANGE_NETWORK_STATE`，局域网发现残留
  - 建议：如果确认不再使用局域网发现功能，可移除这三个权限
