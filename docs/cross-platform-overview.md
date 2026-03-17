# 落笔 Nextype — 跨端对比总览

## 各端功能对比矩阵

### 核心功能

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 文本输入与插入（paste） | ✅ 接收并粘贴 | ✅ 输入并发送 | ✅ 输入并发送 | 待开发 |
| 文本发送（paste-enter） | ✅ 粘贴+回车 | ✅ 输入并发送 | ✅ 输入并发送 | 待开发 |
| 清空输入框 | — | ✅ | ✅ | 待开发 |
| AES 加密通信 | ✅ 解密 | ✅ 加密 | ✅ 加密+解密 | 待开发 |
| ACK 回执 | ✅ 发送回执 | ✅ 接收回执 | ✅ 接收回执 | 待开发 |

### 配对与连接

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 4 位配对码生成 | ✅ 7 种易记模式 | — | — | 待开发 |
| 配对码输入验证 | — | ✅ 中继验证 | ✅ 中继验证 | 待开发 |
| 二维码配对 | ✅ 生成二维码 | ❌ | ❌ | 待开发 |
| 公网中继连接 | ✅ | ✅ | ✅ | 待开发 |
| 局域网直连 | ❌ 已移除 | ❌ 已移除 | ❌ 已移除 | 待开发 |
| 多设备管理 | ✅ 信任列表 | ✅ 底部弹窗 | ✅ 顶部菜单+设置页 | 待开发 |
| 设备信任列表同步 | ✅ | ✅ | ✅ | 待开发 |
| 设备恢复（重装后） | ✅ 信任列表同步 | ✅ 基于 ANDROID_ID | ✅ 基于 UUID | 待开发 |
| 自动重连 | ✅ 5s 间隔，最多 10 次 | ✅ 指数退避 2s→30s | ✅ 指数退避 1.5^n 秒，最大 30s | 待开发 |
| 网络变化监测 | ✅ 5s 检测 IP 变化 | ❌ | ❌ | 待开发 |

### 远程控制

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 快捷键发送指令 | ✅ send/insert/clear | — | — | 待开发 |
| 模拟点击（tap） | ✅ 发送坐标指令 | ✅ AccessibilityService | ❌ 系统限制 | 待开发 |
| 模拟长按（longpress） | ✅ 心跳保持 | ✅ 可续传长按 | ❌ 系统限制 | 待开发 |
| 响应远程控制指令 | — | ✅ send/insert/clear/tap | ✅ send/insert/clear | 待开发 |
| 屏幕参数上报 | ✅ 接收并匹配坐标 | ✅ device_info/screen_changed | ✅ device_info | 待开发 |
| 快捷键录入（含 Fn） | ✅ NSEvent 原生录入 | — | — | 待开发 |

### 语音输入

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 火山引擎 ASR | — | ✅ 功能隐藏但代码完整 | ❌ 已移除 | 待开发 |
| 按住说话交互 | — | ✅ | ❌ 已移除 | 待开发 |
| 打字机效果 | — | ✅ 30ms/字 | ❌ 已移除 | 待开发 |
| 声波动画 | — | ✅ 7 条声波 | ❌ 已移除 | 待开发 |
| 上移取消 | — | ❌ | ❌ 已移除 | 待开发 |

### 用户体验

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 上滑重发/恢复手势 | — | ✅ | ✅ | 待开发 |
| 惯用手设置 | — | ✅ 左/右 | ✅ 左/右 | 待开发 |
| 输入字号设置 | — | ✅ 5 档离散 | ✅ 连续 16-28pt | 待开发 |
| 剪贴板同步设置 | — | ✅ 插入/发送分别控制 | ✅ 插入/发送分别控制 | 待开发 |
| 屏幕常亮 | — | ✅ FLAG_KEEP_SCREEN_ON | ✅ isIdleTimerDisabled | 待开发 |
| 闲置自动变暗 | — | ✅ 亮度状态机+动画 | ✅ ScreenDimManager 状态机 | 待开发 |
| 自动切换设备（Follow the Light） | — | ✅ 基于闲置时间 | ✅ 基于闲置时间 | 待开发 |
| 后台零耗电 | — | ✅ 后台断连+前台重连 | ✅ 后台断连+前台重连 | 待开发 |
| 暗黑模式 | ✅ CSS 变量自动切换 | ✅ Material Design 3 | ✅ 语义色 | 待开发 |
| 使用说明/关于页 | ✅ 关于和反馈标签页 | ✅ AboutActivity | ✅ AboutView + UsageGuideView | 待开发 |
| 空状态引导页 | ✅ 4 步引导流程 | ✅ EmptyStateActivity | ✅ EmptyStateView | 待开发 |
| 使用统计 | ✅ 同步次数/字数 | ❌ | ❌ | 待开发 |

### 系统集成

| 功能 | Mac (Tauri) | Android | iOS | Windows |
|------|:-----------:|:-------:|:---:|:-------:|
| 托盘/菜单栏常驻 | ✅ Template 图标 | — | — | 待开发 |
| 开机启动 | ✅ LaunchAgent | — | — | 待开发 |
| Dock 图标控制 | ✅ ActivationPolicy | — | — | 待开发 |
| 单实例保护 | ✅ | — | — | 待开发 |
| 辅助功能权限 | ✅ CGEventTap 需要 | ✅ AccessibilityService | ❌ 系统无对应 API | 待开发 |
| 折叠屏适配 | — | ✅ 自适应旋转+挖孔屏 | — | 待开发 |
| Electron 数据迁移 | ✅ | — | — | 待开发 |

---

## 共享组件说明

### 中继协议（所有端共用）

所有端通过同一套 WebSocket 消息协议与中继服务器通信，协议基于 JSON over WebSocket：

- **服务器地址**：`wss://nextypeapi.yuanfengai.cn:8443`
- **角色区分**：PC 端注册为 `role: "server"`，手机端注册为 `role: "client"`
- **核心消息类型**：
  - 注册与身份：`register` / `registered`
  - 配对流程：`register_code` / `verify_code` / `pairing_success` / `pairing_completed`
  - 消息转发：`relay`（双向，`data` 字段承载业务数据）
  - 心跳保活：`heartbeat` / `heartbeat_ack` / `client_heartbeat`
  - 设备状态：`client_online` / `client_offline` / `server_online` / `server_offline`
  - 信任管理：`sync_trust_list` / `trust_list` / `unpair_device` / `device_unpaired`
- **配对码有效期**：60 秒，存储在服务器内存中
- **心跳超时**：120 秒无心跳判定离线

详见 [relay-server.md](relay-server.md)。

### 加密方案（AES-256-CBC，CryptoJS 兼容）

所有端使用统一的加密格式，确保跨端互通：

- **算法**：AES-256-CBC + PKCS7 填充
- **密钥派生**：EVP_BytesToKey（MD5 迭代哈希）
  1. `MD5(password + salt)` → 16 字节
  2. `MD5(上轮结果 + password + salt)` → 16 字节
  3. `MD5(上轮结果 + password + salt)` → 16 字节
  4. 前 32 字节 → Key，接下来 16 字节 → IV
- **密码来源**：发送方的 `deviceId`（手机端加密，PC 端用手机的 deviceId 解密）
- **输出格式**：`Base64("Salted__" + salt(8字节) + 密文)`，与 `CryptoJS.AES.encrypt()` 输出一致

各端实现差异：

| 端 | 实现方式 | 说明 |
|----|----------|------|
| Mac (Rust) | `aes` + `cbc` + `md-5` crate | 仅解密 |
| Android (Kotlin) | `javax.crypto.Cipher` (AES/CBC/PKCS5Padding) | 仅加密 |
| iOS (Swift) | `CommonCrypto` (CCCrypt) | 加密+解密均实现（解密未被调用） |

### 配对流程（4 位配对码，60 秒有效期）

1. PC 端生成 4 位配对码（Mac 端支持 7 种易记模式），通过 `register_code` 注册到中继服务器
2. 手机端输入配对码，通过 `verify_code` 发送验证请求
3. 中继服务器验证成功后：写入持久化数据库，向手机端返回 `pairing_success`，向 PC 端发送 `pairing_completed`
4. 双方将对方加入信任设备列表

iOS 端额外支持 UDP 广播 + HTTP 直连两种配对方式与中继竞速，取最先成功的结果。Android 端已移除这两种方式，仅使用中继配对。

---

## 平台差异说明

### 各平台能力限制

| 平台 | 限制 | 影响 |
|------|------|------|
| macOS | 需要辅助功能权限才能使用 CGEventTap 全局快捷键和 AppleScript 模拟按键 | 首次使用需引导用户授权 |
| macOS | ActivationPolicy 切换时系统自动隐藏所有窗口 | 需要记录并恢复窗口可见状态 |
| iOS | 无 AccessibilityService，无法模拟点击 | PC 端 tap/longpress 指令无法响应 |
| iOS | 无全局快捷键，无后台监听能力 | 无法实现系统级快捷键 |
| iOS | 后台执行受限，WebSocket 进入后台后被系统断开 | 需要 BGTaskScheduler 延长后台时间（当前未实现） |
| iOS | 屏幕亮度控制是全局设置（`UIScreen.main.brightness`） | 不像 Android 可以只控制当前窗口亮度 |
| Android | AccessibilityService 需用户手动在设置中开启 | 需要引导提示和权限检测 |
| Android | 软键盘是独立窗口，触摸事件不经过 Activity | 需要 WakeUpEditText 拦截输入事件 |

### 各端技术栈差异

| 维度 | Mac (Tauri) | Android | iOS |
|------|-------------|---------|-----|
| 语言 | Rust + HTML/CSS/JS | Kotlin | Swift |
| UI 框架 | 原生 HTML（无前端框架） | Android View (XML) | SwiftUI |
| 网络库 | tokio-tungstenite | OkHttp 4 | URLSession WebSocketTask |
| 异步模型 | Tokio async/await | Kotlin Coroutines | Combine + GCD |
| 加密库 | aes + cbc + md-5 crate | javax.crypto | CommonCrypto |
| 数据持久化 | tauri-plugin-store (JSON) | SharedPreferences | Keychain |
| 设备 ID 生成 | SHA256(hostname-username-platform) 前 16 位 hex | MD5(ANDROID_ID + salt) 前 16 位 hex | UUID（随机生成，Keychain 持久化） |
| 心跳策略 | 10s 应用层心跳 | 30s 应用层 + 30s TCP ping | 20s WebSocket ping |
| 重连策略 | 5s 固定间隔，最多 10 次 | 指数退避 2s→30s，最多 10 次 | 5s 固定间隔，最多 10 次 |
| 连接架构 | 单条 WebSocket | 统一单条 WebSocket | 统一 ConnectionManager（单连接） |

---

## iOS 端代码质量总结

### 整体评分：8.5 / 10 — 重构完成，功能完整（2026-03-11）

iOS 端经过完整重构，已解决全部已知安全问题，架构大幅改善。

### 重构成果

1. **安全修复（全部完成）**：
   - 语音功能及 API 密钥（VolcanoASRManager）已完全移除
   - 62 处 `print` 日志全部包裹 `#if DEBUG` 条件编译
   - `NSAllowsArbitraryLoads` 已移除，ATS 恢复默认严格模式
   - 密钥存储从 UserDefaults 迁移至 Keychain

2. **架构优化（全部完成）**：
   - 合并 `WebSocketManager` + `RelayClient` 为统一的 `ConnectionManager`
   - `MainInputView` 从 1028 行精简至 624 行（通用 actionButton 组件消除重复手势代码）
   - 代码从 18 文件/4808 行精简至 14 文件/2997 行

3. **功能补齐（全部完成）**：屏幕常亮、闲置自动变暗、远程控制响应、自动切换设备（Follow the Light）、后台生命周期管理、屏幕参数上报、暗黑模式、使用说明页

### 待完善项

- 无单元测试（EncryptionManager 等核心模块建议添加）
- Dark/Tinted App Icon 变体图片缺失（需设计资源）
- 需正式版 Xcode 构建后才能提交 App Store

---

## Windows 端和 iOS 端开发规划建议

### 技术选型建议

| 维度 | 建议 | 理由 |
|------|------|------|
| Windows PC 端 | Tauri 2.x（Rust + Web 前端） | 与 Mac 端共享 90% 以上的 Rust 后端代码和全部前端代码 |
| iOS 端重构 | 保持 Swift 原生 | SwiftUI 已有基础，重构优于重写 |
| 移动端 UI | 各端原生实现 | Android View / SwiftUI 各有生态优势，强行统一反而增加复杂度 |
| 跨端共享 | 中继协议 + 加密方案 + 配对流程 | 协议层已标准化，各端独立实现即可 |

### Windows 端开发规划

**可从 Mac 端复用的部分**：

- **Rust 后端（约 90% 可复用）**：
  - `relay_client.rs` — 中继客户端（完全复用）
  - `device_manager.rs` — 配对码生成/验证（完全复用）
  - `state.rs` / `utils/config.rs` — 状态管理和配置（完全复用）
  - `commands/` — 大部分 Tauri Command（完全复用）
  - `services/stats.rs` — 统计服务（完全复用）
- **前端（约 95% 可复用）**：
  - `preferences.html` / `onboarding.html` / `logs.html` — 所有页面
  - `style.css` / `tauri-bridge.js` — 样式和桥接层

**需要重新开发的部分**：

- `services/clipboard.rs` — 粘贴操作需从 AppleScript 改为 Windows SendInput API
- `services/native_hotkey.rs` — 全局快捷键需从 CGEventTap 改为 Windows Hook API
- `services/tray.rs` — 托盘菜单和窗口定位需适配 Windows 系统托盘
- 辅助功能权限检查 — 从 AXIsProcessTrusted 改为 Windows UI Automation
- 系统闲置时间获取 — 从 `ioreg` 改为 `GetLastInputInfo` API

### iOS 端补全规划

**可从 Android 端对齐的功能**（按优先级排序）：

1. **P0 安全修复**：API 密钥迁移、删除调试日志（1-2 天）
2. **连接架构对齐**：合并双通道为统一连接，对齐 Android 的单 WebSocket 架构（2-3 天）
3. **远程控制响应**：实现 send/insert/clear 指令响应（tap 因系统限制无法实现）（1 天）
4. **屏幕常亮 + 自动变暗**：通过 `UIApplication.shared.isIdleTimerDisabled` + `UIScreen.main.brightness`（1-2 天）
5. **后台生命周期管理**：`scenePhase` 监听，后台断连+前台重连（1 天）
6. **暗黑模式**：移除强制浅色，使用语义色（1 天）
7. **自动切换设备**：移植 Follow the Light 策略（1 天）

---

## 各端废弃代码汇总

### Mac 端 (Tauri)

| 类型 | 位置 | 说明 |
|------|------|------|
| 未使用依赖 | `Cargo.toml` — `aes-gcm` | 计划用 AES-GCM 但最终用了 AES-CBC，从未引用 |
| 未使用依赖 | `Cargo.toml` — `thiserror` | 错误处理用 String，未使用 derive 宏 |
| 残留架构 | `src/tauri-bridge.js` | Electron→Tauri 适配层，长期应重构为直接使用 Tauri API |
| 残留字段 | `utils/config.rs` — `port`、`min_poll_interval`、`max_poll_interval`、`auto_update` | Electron 版本遗留配置字段 |
| 残留文件 | `src/main.js` | 空占位文件，未被引用 |
| 残留文件 | `src/styles.css` | 未被任何 HTML 引用的旧版样式 |
| 残留代码 | `preferences.html` — Electron shell/ipcRenderer 分支 | 被 tauri-bridge.js 覆盖，永远不会执行 |

### Android 端

| 类型 | 位置 | 说明 |
|------|------|------|
| 残留文件 | `DeviceDiscoveryService.kt`（264 行） | 局域网 mDNS+UDP 发现服务，已被公网中继取代 |
| 残留文件 | `DeviceListActivity.kt`（121 行） | 局域网设备列表页，未被启动 |
| 残留文件 | `UDPPairingClient.kt`（85 行） | UDP 广播配对，已被 RelayClient 取代 |
| 残留文件 | `HTTPPairingClient.kt`（81 行） | HTTP 配对，已被 RelayClient 取代 |
| 残留布局 | `activity_device_list.xml` | 对应已废弃的 DeviceListActivity |
| 未使用依赖 | `build.gradle.kts` — `constraintlayout` | 所有布局用 LinearLayout/FrameLayout |
| 已注释代码 | `SettingsActivity.kt` 多处 | 语音输入开关、火山引擎配置、设备管理 UI（已移至首页） |
| 废弃方法 | `MainActivity.kt` — `connectControlChannel()` | 标注已废弃，方法体为空 |
| 残留封装 | `MainActivity.kt` — `connectToServer()` | 直接调用 connectToRelay，多余一层封装 |
| 残留权限 | AndroidManifest — `ACCESS_WIFI_STATE`、`CHANGE_WIFI_MULTICAST_STATE`、`CHANGE_NETWORK_STATE` | 局域网发现残留 |

### iOS 端

重构后废弃代码已全部清理。当前无已知冗余代码。

| 类型 | 位置 | 处理结果 |
|------|------|----------|
| 废弃文件 | `VolcanoASRManager.swift`（语音识别） | ✅ 已删除 |
| 废弃文件 | `WebSocketManager.swift` + `RelayClient.swift` | ✅ 已合并为 ConnectionManager |
| 废弃文件 | `SpeechRecognitionManager.swift` | ✅ 已删除 |
| 废弃文件 | `DeviceDiscoveryService.swift`（Bonjour） | ✅ 已删除 |
| 废弃文件 | `ServerDevice.swift` / `HTTPPairingClient.swift` / `UDPPairingClient.swift` | ✅ 已删除 |
| 冗余注释 | `PairingCodeView.swift` — `// ConnectionStatusView struct removed` | ✅ 已删除 |
| 冗余资源 | `AccentColor.colorset/AppIcon.png`（320KB 误放文件） | ✅ 已删除 |
| 调试代码 | 全部文件 — 62 处 `print` | ✅ 已包裹 `#if DEBUG` |

### 中继服务器

| 类型 | 位置 | 说明 |
|------|------|------|
| 未使用字段 | `server.js` — `platform` 字段 | 客户端未传递，始终为 "unknown" |
| 待完成功能 | `server.js` — relay 消息配对关系校验 | 注释提到应检查但未实现 |
| 安全隐患 | `deploy.sh` / `manage.js` | 硬编码服务器 IP 和密码 |
