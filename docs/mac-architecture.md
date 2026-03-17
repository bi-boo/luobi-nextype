# 落笔 Nextype — Mac 端技术架构文档

## 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 框架 | Tauri | 2.x |
| 后端语言 | Rust | 2021 Edition |
| 前端 | 原生 HTML/CSS/JS（无框架） | - |
| 异步运行时 | Tokio | 1.x (full features) |
| WebSocket | tokio-tungstenite | 0.24 (native-tls) |
| 序列化 | serde + serde_json | 1.x |
| 日志 | tracing + tracing-subscriber | 0.1 / 0.3 |
| macOS 原生 | core-graphics + core-foundation + objc2 | - |

---

## 项目结构与模块划分

```
NextypeTauri/nextype-tauri/
├── src-tauri/
│   ├── Cargo.toml                    # Rust 依赖配置
│   ├── tauri.conf.json               # Tauri 应用配置（窗口、插件、打包）
│   ├── icons/                        # 应用图标和托盘图标
│   └── src/
│       ├── main.rs                   # 程序入口，调用 lib::run()
│       ├── lib.rs                    # 应用初始化：插件注册、状态创建、setup 回调、命令注册
│       ├── state.rs                  # 全局状态管理（AppState）
│       ├── commands/                 # Tauri Command 层（前端可调用的接口）
│       │   ├── mod.rs               # 模块声明和重导出
│       │   ├── app.rs               # 应用信息命令（版本、名称、构建信息）
│       │   ├── clipboard.rs         # 剪贴板和键盘命令（粘贴、写入、权限检查）
│       │   ├── config.rs            # 配置管理命令（CRUD、迁移、信任设备）
│       │   ├── devices.rs           # 设备管理命令（配对码、加密密钥、IP）
│       │   ├── hotkeys.rs           # 快捷键命令（注册/注销、坐标配置、录入控制）
│       │   ├── logs.rs              # 日志命令（获取、清空）
│       │   ├── relay.rs             # 中继命令（连接/断开、在线列表、配对码注册）
│       │   ├── stats.rs             # 统计命令（获取、记录、重置、启用/禁用）
│       │   ├── system.rs            # 系统命令（开机启动、Dock/菜单栏图标、平台信息、文件写入）
│       │   └── windows.rs           # 窗口命令（打开/关闭/聚焦/隐藏/显示）
│       ├── services/                # 服务层（核心业务逻辑）
│       │   ├── mod.rs               # 模块声明
│       │   ├── clipboard.rs         # 剪贴板服务（AppleScript 键盘模拟、权限检查、内容处理）
│       │   ├── device_manager.rs    # 设备管理服务（配对码生成/验证、二维码生成）
│       │   ├── hotkey_manager.rs    # 快捷键管理服务（注册/注销、动作分发、坐标匹配）
│       │   ├── native_hotkey.rs     # macOS 原生快捷键（CGEventTap 全局监听、NSEvent 录入）
│       │   ├── relay_client.rs      # 中继客户端（WebSocket 连接、消息处理、重连、加密）
│       │   ├── stats.rs             # 统计数据服务（记录、日期重置）
│       │   └── tray.rs              # 托盘管理（菜单构建、图标切换、窗口创建）
│       └── utils/                   # 工具模块
│           ├── mod.rs               # 模块声明
│           ├── config.rs            # 配置数据结构（AppConfig、TrustedDevice）
│           └── logger.rs            # 日志系统（四层架构：控制台+文件+内存+前端推送）
└── src/                             # 前端资源
    ├── index.html                   # 入口页（立即重定向到 preferences.html）
    ├── preferences.html             # 偏好设置页（5 个标签页，核心 UI）
    ├── onboarding.html              # 引导页（4 步引导流程）
    ├── logs.html                    # 日志查看页
    ├── style.css                    # 全局样式（深色/浅色模式 CSS 变量）
    └── onboarding.html              # 引导页（4 步引导流程）
```

---

## 前后端通信机制

### Tauri Commands（前端 → 后端）

前端通过 `window.__TAURI__.core.invoke('command_name', { params })` 调用后端 Rust 函数。所有命令在 `lib.rs` 的 `invoke_handler` 中注册。

### Tauri Events（后端 → 前端）

后端通过 `app.emit("event_name", payload)` 向所有窗口广播事件，前端通过 `window.__TAURI__.event.listen("event_name", callback)` 监听。

### 前端直接调用 Tauri API

前端通过 `window.__TAURI__.core.invoke('command_name', { params })` 调用后端 Rust 函数，通过 `window.__TAURI__.event.listen('event_name', callback)` 监听后端事件。无中间适配层。

### 关键事件流

```
手机端发送文字
    ↓
中继服务器转发 (WebSocket)
    ↓
relay_client.rs: handle_server_message() 收到 Relay 消息
    ↓
解析 data.type == "clipboard" / "content"
    ↓
如果 encrypted=true → decrypt_cryptojs_aes() 解密
    ↓
clipboard::handle_clipboard_content() 处理
    ↓
write_clipboard() 写入剪贴板
    ↓
根据 action: paste() / paste_and_enter() / 仅复制
    ↓
记录统计数据 → emit("stats_updated")
    ↓
发送 ACK 回执给手机端
```

---

## Tauri Command 接口清单

### 配置命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `get_config` | 无 | `AppConfig` | 获取完整配置（优先从 store 加载） |
| `save_config` | `config: AppConfig` | `()` | 保存配置到内存和 store，广播 `config_updated` |
| `get_config_value` | `key: String` | `serde_json::Value` | 获取单个配置项 |
| `set_config_value` | `key: String, value: Value` | `()` | 设置单个配置项 |
| `get_device_id` | 无 | `String` | 获取本机设备 ID |
| `get_device_name` | 无 | `String` | 获取本机设备名称 |
| `get_trusted_devices` | 无 | `Vec<TrustedDevice>` | 获取信任设备列表 |
| `add_trusted_device` | `device: TrustedDevice` | `()` | 添加信任设备 |
| `remove_trusted_device` | `device_id: String` | `bool` | 移除信任设备 |
| `is_device_trusted` | `device_id: String` | `bool` | 检查设备是否受信任 |
| `get_relay_server_url` | 无 | `String` | 获取中继服务器地址 |
| `reset_config` | 无 | `()` | 重置配置为默认值 |
| `migrate_from_electron` | 无 | `bool` | 从 Electron 配置迁移数据 |

### 窗口命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `open_preferences_window` | `initial_tab: Option<String>` | `()` | 打开偏好设置窗口（可指定初始标签页） |
| `open_logs_window` | 无 | `()` | 打开日志窗口 |
| `open_onboarding_window` | 无 | `()` | 打开引导窗口 |
| `close_window` | `label: String` | `()` | 关闭指定窗口 |
| `focus_window` | `label: String` | `()` | 聚焦指定窗口 |
| `hide_window` | `label: String` | `()` | 隐藏指定窗口 |
| `show_window` | `label: String` | `()` | 显示指定窗口 |

### 中继命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `relay_connect` | 无（从配置读取 URL） | `()` | 连接中继服务器 |
| `relay_disconnect` | 无 | `()` | 断开中继连接 |
| `relay_is_connected` | 无 | `bool` | 获取连接状态 |
| `relay_get_online_clients` | 无 | `Vec<String>` | 获取在线客户端列表 |
| `relay_register_pairing_code` | `code: String` | `()` | 向中继注册配对码 |
| `relay_unpair_device` | `device_id: String` | `()` | 解除设备配对 |
| `relay_send_to_device` | `target_device_id: String, data: Value` | `()` | 发送消息到指定设备 |

### 剪贴板和键盘命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `has_accessibility_permission` | 无 | `bool` | 检查辅助功能权限 |
| `request_accessibility_permission` | 无 | `bool` | 请求辅助功能权限 |
| `open_accessibility_settings` | 无 | `()` | 打开系统辅助功能设置 |
| `paste` | 无 | `bool` | 执行粘贴（Cmd+V） |
| `paste_and_enter` | 无 | `bool` | 执行粘贴+回车 |
| `write_clipboard` | `text: String` | `()` | 写入剪贴板 |
| `read_clipboard` | 无 | `String` | 读取剪贴板 |
| `handle_clipboard_content` | `content: String, action: String` | `()` | 处理剪贴板内容（复制/粘贴/粘贴回车） |

### 设备管理命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `generate_pairing_code` | `force_random: Option<bool>` | `PairingCodeResponse` | 生成配对码（含二维码 URL） |
| `get_current_pairing_code` | 无 | `Option<PairingCodeResponse>` | 获取当前配对码 |
| `verify_pairing_code` | `code: String` | `bool` | 验证配对码 |
| `clear_pairing_code` | 无 | `()` | 清除当前配对码 |
| `generate_encryption_key` | 无 | `String` | 生成 32 字节随机加密密钥 |
| `get_local_ip` | 无 | `String` | 获取本机 IP 地址 |

### 系统设置命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `get_autostart_enabled` | 无 | `bool` | 获取开机启动状态 |
| `set_autostart_enabled` | `enabled: bool` | `()` | 设置开机启动 |
| `set_dock_icon_visible` | `visible: bool` | `()` | 设置 Dock 图标可见性 |
| `set_menu_bar_icon_visible` | `visible: bool` | `()` | 设置菜单栏图标可见性 |
| `get_platform` | 无 | `String` | 获取平台信息（"macos"） |
| `write_file` | `path: String, content: String` | `()` | 写入文件（供日志导出使用） |

### 统计数据命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `get_stats` | 无 | `Statistics` | 获取统计数据 |
| `record_paste` | `char_count: usize` | `()` | 记录一次粘贴操作 |
| `reset_stats` | 无 | `()` | 重置统计数据 |
| `set_stats_enabled` | `enabled: bool` | `()` | 设置是否启用统计 |

### 快捷键命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `register_hotkey` | `action: String, accelerator: String` | `()` | 注册单个快捷键（同时持久化） |
| `unregister_hotkey` | `action: String` | `()` | 注销单个快捷键 |
| `register_all_hotkeys` | `hotkeys: HashMap<String, String>` | `()` | 批量注册快捷键 |
| `get_registered_hotkeys` | 无 | `HashMap<String, String>` | 获取已注册的快捷键 |
| `save_tap_coordinates` | `coordinates: Value` | `()` | 保存点击坐标配置 |
| `get_tap_coordinates` | 无 | `Value` | 获取点击坐标配置 |
| `save_longpress_coordinates` | `coordinates: Value` | `()` | 保存长按坐标配置 |
| `get_longpress_coordinates` | 无 | `Value` | 获取长按坐标配置 |
| `start_hotkey_recording` | 无 | `()` | 开始原生按键录入 |
| `stop_hotkey_recording` | 无 | `()` | 停止原生按键录入 |

### 应用信息命令

| 命令名 | 参数 | 返回值 | 功能 |
|--------|------|--------|------|
| `get_app_version` | 无 | `String` | 获取应用版本 |
| `get_app_name` | 无 | `String` | 获取应用名称 |
| `get_build_info` | 无 | `Value` | 获取构建信息（版本、名称、作者、描述） |

---

## 服务层设计

### relay_client（中继客户端）

**文件**：`services/relay_client.rs`

核心架构采用 Actor 模式：
- `RelayClient` 是外部接口，持有 `mpsc::Sender<RelayCommand>` 和 `Arc<RwLock<RelayClientInner>>` 共享状态
- `relay_client_task()` 是后台异步任务，通过 `tokio::select!` 同时处理：
  1. 命令通道（Connect/Disconnect/Send/RegisterPairingCode/UnpairDevice）
  2. WebSocket 消息接收
  3. 心跳定时器（10 秒间隔）
  4. 客户端超时检查（60 秒间隔，120 秒超时）
- 断线自动重连：5 秒间隔，最多 10 次，重连后自动重新注册设备和恢复配对码

**消息协议**：
- 发送消息类型（`WsMessage`）：Register、Heartbeat、Relay、RegisterCode、UnpairDevice、SyncTrustList
- 接收消息类型（`ServerMessage`）：Connected、Registered、CodeRegistered、CodeConflict、Relay、ClientOnline、ClientOffline、ClientHeartbeat、HeartbeatAck、Error、TrustList、DeviceUnpaired、PairingCompleted、UnpairSuccess

**加密**：`decrypt_cryptojs_aes()` 实现 CryptoJS 兼容的 AES-256-CBC 解密（OpenSSL 格式），使用 EVP_BytesToKey(MD5) 从密码+salt 派生 key(32B)+iv(16B)。

### hotkey_manager（快捷键管理器）

**文件**：`services/hotkey_manager.rs`

双轨快捷键系统：
1. **tauri-plugin-global-shortcut**：处理不含 Fn 键的常规快捷键
2. **CGEventTap（native_hotkey）**：处理含 Fn 键的快捷键和 longpress 快捷键

动作分发流程：
- `handle_hotkey_press()` → 获取在线设备 → 构建指令 JSON → 通过 relay_client 发送
- tap/longpress 动作需要调用 `match_coordinates()` 匹配坐标
- longpress 按下时启动心跳任务（500ms 间隔），释放时停止并可选自动插入

坐标匹配策略（`match_coordinates()`）：三级容错——精确匹配 → 比例匹配（差值<0.1）→ 兜底使用第一个配置。

### native_hotkey（macOS 原生快捷键）

**文件**：`services/native_hotkey.rs`（仅 macOS 编译）

两个独立功能：

**录入功能**（NSEvent local monitor）：
- 通过 `NSEvent.addLocalMonitorForEventsMatchingMask` 监听 keyDown(1<<10) + flagsChanged(1<<12)
- 使用 `RecordingInternalState` 追踪修饰键峰值（peak_modifiers）和是否有主键按下
- 纯修饰键组合：所有修饰键释放时，如果 peak 包含 Fn 且没按过主键，完成录入
- 修饰键+主键：keyDown 时立即完成录入
- 通过 `hotkey-recorded` 和 `hotkey-recording-modifiers` 事件通知前端

**全局监听功能**（CGEventTap）：
- 使用 Default 模式（可消费事件），监听 KeyDown + KeyUp + FlagsChanged
- 辅助功能权限等待：如果缺少权限，自动等待（指数退避，2s→30s，最长 5 分钟），应用激活时通过 Condvar 唤醒重试
- 纯修饰键快捷键：在 flagsChanged 中检测修饰键组合匹配，释放时触发
- 修饰键+主键快捷键：在 keyDown 中匹配，支持 Fn+Return→numpad Enter(76) 等价处理
- longpress 支持：keyDown 时记录 `active_longpress`，keyUp 或 flagsChanged 不匹配时触发释放
- 副作用屏蔽：匹配到快捷键时，剥离修饰键标志并修改 keycode 为 0xFFFF，防止原始按键动作

### device_manager（设备管理器）

**文件**：`services/device_manager.rs`

- 配对码生成：7 种易记忆模式（约 400+ 种组合），冲突时切换纯随机
- 配对码存储：`RwLock<Option<PairingCodeInfo>>`，包含创建时间和 60 秒有效期
- 二维码生成：`qrcode` crate 生成 QR 矩阵 → `image` crate 渲染为 PNG → Base64 编码为 Data URL
- 加密密钥生成：32 字节随机数，hex 编码

### clipboard（剪贴板服务）

**文件**：`services/clipboard.rs`

- 权限检查：通过 AppleScript `tell application "System Events" return true` 测试
- 权限请求：通过 AppleScript `keystroke ""` 触发系统权限弹窗
- 粘贴操作：AppleScript `keystroke "v" using command down`，延迟 100ms 确保剪贴板写入完成
- 内容处理：根据 action 追加后缀 → 写入剪贴板 → 执行粘贴 → 记录统计 → 可选自动清空

### tray（托盘管理）

**文件**：`services/tray.rs`

- 托盘图标：嵌入二进制资源（`include_bytes!`），使用 Template 图标自动适配深色/浅色
- 菜单构建：动态生成，包含在线设备列表（从 AppState 读取）
- 窗口创建：偏好设置（700x600，不可调整大小）、日志（900x700，可调整）、引导（400x680，不可调整）
- 多显示器定位：通过 CGEvent API 获取鼠标物理坐标，遍历显示器找到鼠标所在屏幕，计算居中位置
- Accessory 模式前置：`set_always_on_top(true)` → `set_focus()` → `set_always_on_top(false)`

### stats（统计服务）

**文件**：`services/stats.rs`

- 数据结构：`total_chars`、`total_pastes`、`today_chars`、`last_date`
- 日期重置：每次访问时检查 `last_date` 是否为今天，不是则重置 `today_chars`
- 持久化：通过 `tauri-plugin-store` 存储在 `stats.json`

---

## 状态管理（AppState）

**文件**：`state.rs`

```rust
pub struct AppState {
    pub config: RwLock<AppConfig>,           // 应用配置（内存副本）
    pub online_devices: RwLock<Vec<String>>, // 在线设备 ID 列表
    pub tray_icon: RwLock<Option<TrayIcon>>, // 托盘图标句柄（防止被 drop）
    pub should_quit: AtomicBool,             // 是否应该退出（区分窗口关闭和主动退出）
}
```

- 类型别名：`SharedAppState = Arc<AppState>`
- 通过 `tauri::manage()` 注入到 Tauri 状态系统
- 配置的持久化通过 `tauri-plugin-store`（`config.json`）实现，内存中的 `config` 是运行时副本

其他共享状态（通过 `tauri::manage()` 注入）：
- `SharedRelayClient = Arc<RwLock<RelayClient>>`
- `SharedDeviceManager = Arc<DeviceManager>`
- `SharedHotkeyManager = Arc<RwLock<Option<HotkeyManager>>>`

---

## 配置数据结构（AppConfig）

**文件**：`utils/config.rs`

```rust
pub struct AppConfig {
    // 端口配置（Electron 残留）
    pub port: u16,                          // 默认 9001

    // 按钮配置
    pub enable_btn1: bool,                  // 默认 true
    pub btn1_text: String,                  // 默认 "同步"
    pub btn1_suffix: String,                // 默认 ""
    pub enable_btn2: bool,                  // 默认 true
    pub btn2_text: String,                  // 默认 "发送"
    pub btn2_suffix: String,                // 默认 ""

    // 性能配置（Electron 残留）
    pub min_poll_interval: u64,             // 默认 100
    pub max_poll_interval: u64,             // 默认 2000

    // 显示选项
    pub show_dock_icon: bool,               // 默认 true
    pub show_menu_bar_icon: bool,           // 默认 true
    pub auto_launch: bool,                  // 默认 false
    pub auto_update: bool,                  // 默认 true（Electron 残留）

    // 远程连接
    pub enable_remote_connection: bool,     // 默认 true
    pub relay_server_url: String,           // 默认 "wss://nextypeapi.yuanfengai.cn:8443"

    // 设备信息
    pub device_id: String,                  // SHA256(hostname-username-platform) 前 16 位 hex
    pub device_name: String,                // 系统 hostname

    // 安全
    pub trusted_devices: Vec<TrustedDevice>,

    // 快捷键
    pub hotkeys: HashMap<String, String>,   // action -> accelerator

    // 统计
    pub enable_stats: bool,                 // 默认 true

    // 剪贴板
    pub clear_after_paste: bool,            // 默认 false

    // 坐标配置
    pub tap_coordinates: Value,             // JSON 数组
    pub longpress_coordinates: Value,       // JSON 数组
    pub longpress_auto_insert: bool,        // 默认 false
    pub longpress_auto_insert_delay: u64,   // 默认 300ms

    // 版本
    pub version: String,                    // 默认 "2.0.0"

    // 迁移标记
    pub electron_data_migrated: bool,       // 默认 false
}
```

序列化使用 `#[serde(rename_all = "camelCase")]`，与前端 JSON 字段名对齐。

设备 ID 生成算法：`SHA256(hostname + "-" + username + "-" + platform)` 取前 8 字节 hex 编码（16 字符），确保每次启动 ID 稳定不变。

---

## 加密方案

### CryptoJS 兼容 AES-256-CBC 解密

**文件**：`services/relay_client.rs` → `decrypt_cryptojs_aes()`

手机端使用 CryptoJS.AES.encrypt 加密，默认 OpenSSL 格式：

```
Base64 解码后的数据结构：
[8 字节 "Salted__"] [8 字节 salt] [N 字节密文]
```

密钥派生（EVP_BytesToKey）：
1. 使用 MD5 迭代哈希
2. 第一轮：`MD5(password + salt)` → 16 字节
3. 第二轮：`MD5(上一轮结果 + password + salt)` → 16 字节
4. 第三轮：`MD5(上一轮结果 + password + salt)` → 16 字节
5. 取前 32 字节作为 key，接下来 16 字节作为 iv

解密：AES-256-CBC + PKCS7 padding，密码为发送方的 deviceId。

---

## 日志架构

**文件**：`utils/logger.rs`

四层日志架构，通过 `tracing_subscriber` Layer 实现：

```
tracing::info!("消息")
    ↓
tracing_subscriber::registry()
    ├── EnvFilter (INFO 级别及以上)
    ├── fmt::layer() ──────────→ 控制台输出
    └── TauriLogger
        ├── write_to_file() ──→ 文件写入 (~/Library/Logs/nextype-tauri/clipboard-sync.log)
        ├── log_cache ────────→ 内存缓存 (最近 1000 条, Vec<LogEntry>)
        └── app.emit() ──────→ 前端推送 (new_log_entry 事件)
```

- 全局静态实例：`LOG_CACHE` 和 `APP_HANDLE` 通过 `once_cell::Lazy` 初始化
- 日志轮转：文件超过 10MB 时重命名为 `.log.old`
- `LogEntry` 结构：`{ timestamp, level, message }`

---

## macOS 特有实现

### AppleScript 键盘操作

**文件**：`services/clipboard.rs`

- 粘贴：`osascript -e 'tell application "System Events" keystroke "v" using command down'`
- 回车：`osascript -e 'tell application "System Events" keystroke return'`
- 权限检查：`osascript -e 'tell application "System Events" return true'`
- 权限请求：`osascript -e 'tell application "System Events" keystroke ""'`

### CGEventTap 全局快捷键监听

**文件**：`services/native_hotkey.rs`

- 使用 `core-graphics` crate 的 `CGEventTap::new()` 创建事件拦截器
- 监听事件类型：KeyDown、KeyUp、FlagsChanged
- 运行在独立线程的 CFRunLoop 中
- 通过修改事件的 flags 和 keycode（设为 0xFFFF）来消费/屏蔽匹配的按键

### NSEvent 本地键盘监听（录入）

**文件**：`services/native_hotkey.rs`

- 使用 `objc2` crate 调用 `NSEvent.addLocalMonitorForEventsMatchingMask`
- 通过 `block2::RcBlock` 创建 Objective-C block 回调
- 返回 `null_mut()` 吞掉事件，返回原始 event 放行

### ActivationPolicy 切换

**文件**：`commands/system.rs`

- `Regular`：显示 Dock 图标，应用出现在 Cmd+Tab 切换器中
- `Accessory`：隐藏 Dock 图标，应用不出现在 Cmd+Tab 中
- 切换前记录所有可见窗口，切换后恢复（macOS 切换到 Accessory 时会自动隐藏所有窗口）

### 系统闲置时间获取

**文件**：`services/relay_client.rs` → `get_system_idle_time()`

- 通过 `ioreg -c IOHIDSystem -d 4` 命令读取 `HIDIdleTime`（纳秒）
- 转换为秒后作为心跳消息的 `idle_time` 字段发送

### 鼠标位置获取

**文件**：`services/tray.rs` → `get_cursor_position()`

- 通过 `CGEventCreate(null)` + `CGEventGetLocation()` 获取全局物理坐标
- 用于多显示器场景下将新窗口定位到鼠标所在屏幕

### 辅助功能权限检查

**文件**：`services/native_hotkey.rs`

- 通过 FFI 调用 `AXIsProcessTrusted()` 轻量检查（不弹窗）
- 用于 CGEventTap 启动前的权限判断

---

## 依赖项清单

### Tauri 核心与插件

| 依赖 | 版本 | 用途 |
|------|------|------|
| `tauri` | 2 | 应用框架核心（含 tray-icon feature） |
| `tauri-plugin-opener` | 2 | 打开外部链接 |
| `tauri-plugin-store` | 2 | 配置和统计数据持久化（JSON 文件） |
| `tauri-plugin-clipboard-manager` | 2 | 系统剪贴板读写 |
| `tauri-plugin-notification` | 2 | 系统通知（配对成功提示） |
| `tauri-plugin-global-shortcut` | 2 | 全局快捷键注册（非 Fn 键） |
| `tauri-plugin-autostart` | 2 | 开机启动（macOS LaunchAgent） |
| `tauri-plugin-shell` | 2 | Shell 命令执行 |
| `tauri-plugin-dialog` | 2 | 文件保存对话框（日志导出） |
| `tauri-plugin-single-instance` | 2.0.0-rc.3 | 单实例保护 |

### 异步与网络

| 依赖 | 版本 | 用途 |
|------|------|------|
| `tokio` | 1 (full) | 异步运行时 |
| `tokio-tungstenite` | 0.24 (native-tls) | WebSocket 客户端 |
| `futures-util` | 0.3 | Stream/Sink 扩展（WebSocket 读写） |
| `local-ip-address` | 0.6 | 获取本机 IP（网络变化监测） |

### 序列化

| 依赖 | 版本 | 用途 |
|------|------|------|
| `serde` | 1 (derive) | 序列化/反序列化框架 |
| `serde_json` | 1 | JSON 处理 |

### 加密

| 依赖 | 版本 | 用途 |
|------|------|------|
| `aes` | 0.8 | AES 加密核心（用于 AES-256-CBC 解密） |
| `cbc` | 0.1 (alloc) | CBC 模式解密 |
| `md-5` | 0.10 | MD5 哈希（EVP_BytesToKey 密钥派生） |
| `sha2` | 0.10 | SHA-256 哈希（设备 ID 生成） |
| `rand` | 0.8 | 随机数生成（配对码、加密密钥） |
| `base64` | 0.22 | Base64 编解码 |
| `hex` | 0.4 | 十六进制编解码 |

### 系统信息

| 依赖 | 版本 | 用途 |
|------|------|------|
| `hostname` | 0.4 | 获取系统主机名 |
| `dirs` | 6 | 获取系统目录路径（配置目录） |

### 二维码

| 依赖 | 版本 | 用途 |
|------|------|------|
| `qrcode` | 0.14 | 二维码生成 |
| `image` | 0.25 | 图像处理（QR 渲染为 PNG、托盘图标加载） |

### 日志

| 依赖 | 版本 | 用途 |
|------|------|------|
| `chrono` | 0.4 | 时间格式化（日志时间戳、日期重置） |
| `tracing` | 0.1 | 日志框架 |
| `tracing-subscriber` | 0.3 (env-filter) | 日志订阅器（控制台+自定义 Layer） |

### 工具

| 依赖 | 版本 | 用途 |
|------|------|------|
| `once_cell` | 1 | 全局静态实例（日志缓存、AppHandle） |
| `parking_lot` | 0.12 | 高性能读写锁（替代 std::sync::RwLock） |

### macOS 原生 API（条件编译）

| 依赖 | 版本 | 用途 |
|------|------|------|
| `core-graphics` | 0.24 | CGEventTap 全局事件监听、鼠标位置获取 |
| `core-foundation` | 0.10 | CFRunLoop 事件循环 |
| `objc2` | 0.6 | Objective-C 运行时绑定 |
| `objc2-foundation` | 0.3 | NSObject 等基础类型 |
| `objc2-app-kit` | 0.3 | NSEvent 键盘事件监听 |
| `block2` | 0.6 | Objective-C block 创建 |

---

## 废弃与未使用代码（架构视角补充）

- **[残留字段]** `utils/config.rs` — `port`、`min_poll_interval`、`max_poll_interval`、`auto_update`
  - 架构说明：这些字段是 Electron 版本局域网直连架构的遗留。Tauri 版本完全基于 WebSocket 中继通信，不再需要端口配置和轮询间隔。`auto_update` 在 Tauri 版本中也未实现
  - 建议：保留 `#[serde(default)]` 确保旧配置文件反序列化兼容，但可在代码注释中标注为 deprecated
