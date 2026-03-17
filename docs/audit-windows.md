# Windows 端审查报告 (NextypeWindows / Tauri 2.x + Rust)

**审查日期**: 2026-03-11
**技术栈**: Tauri 2.x (Rust 后端 + 原生 HTML/CSS/JS 前端)
**Identifier**: com.nextype.app（与 Mac 端相同，有冲突风险）
**状态**: 架构完成，90% 复用 Mac 端代码，未经 Windows 真机测试

---

## 总览评级

| 维度 | 评级 | 核心发现 |
|------|------|-----------|
| 功能完整性 | ⚠️ 需改进 | 核心功能代码已到位，但引导流程/偏好设置有 macOS 残留 UI |
| 代码质量 | ⚠️ 需改进 | 整体结构清晰，但日志模块有 macOS 硬编码路径 Bug |
| 构建配置 | ⚠️ 需改进 | MSI/NSIS 双目标已配置，但版本号不一致、无 capabilities |
| 分发就绪度 | ❌ 阻塞 | 无代码签名证书，SmartScreen 会拦截 |
| 安全性 | ⚠️ 需改进 | E2E 加密已实现，但 CSP 为 null |
| 已知问题与风险 | ⚠️ 需改进 | 多处 macOS 残留代码、未经 Windows 真机验证 |
| 依赖管理 | ✅ 就绪 | Cargo 依赖完整，无 npm 依赖 |
| Windows 适配度 | ⚠️ 需改进 | 核心 API 适配完成，前端有多处 macOS 概念残留 |

---

## 1. 功能完整性 -- ⚠️ 需改进

### PRD 对照

| PRD 功能 | 状态 | 说明 |
|----------|------|------|
| 配对码生成（4位、7种模式） | ✅ | device_manager.rs |
| 配对码 60s 有效期 | ✅ | |
| 二维码配对 | ✅ | base64 PNG |
| 配对完成通知 | ✅ | tauri_plugin_notification |
| 写入剪贴板 | ✅ | tauri_plugin_clipboard_manager |
| 自动粘贴 (Ctrl+V) | ✅ | SendInput API |
| 粘贴+回车 | ✅ | 粘贴后 300ms Enter |
| AES-256-CBC 加密 | ✅ | CryptoJS 兼容 |
| 快捷键远程控制（5种） | ✅ | send/insert/clear/tap/longpress |
| 屏幕坐标配置 | ✅ | 三级容错匹配 |
| 系统托盘 + 菜单 | ✅ | TrayIconBuilder |
| 偏好设置窗口 | ✅ | 5个标签页 |
| 引导流程 4 步 | ⚠️ | 权限页仍显示 macOS 内容 |
| 日志系统 | ✅ | 四层架构 |
| 开机启动 / 单实例 | ✅ | tauri_plugin_autostart / single_instance |
| WebSocket 中继 | ✅ | tokio-tungstenite + WSS |
| 自动更新 | ❌ | 无 updater 配置 |

### 关键问题: macOS 残留 UI

1. **引导页权限说明** (onboarding.html): 仍展示"辅助功能权限"和"控制 System Events"，这是 macOS 概念
2. **Dock 图标开关** (preferences.html): Windows 没有 Dock
3. **辅助功能权限设置** (preferences.html): Windows 下永远返回 true，无意义
4. **WiFi 局域网措辞** (onboarding.html): "确保手机和电脑在同一 Wi-Fi 下"，实际使用中继服务器
5. **figma-pc-preferences.html 遗留文件**: 设计稿预览，不应包含在产品中

---

## 2. 代码质量 -- ⚠️ 需改进

### 优点

- 模块划分清晰: commands/services/utils 三层分离
- 错误处理一致: Result<T, String> 统一错误格式
- 命名规范: Rust snake_case + 前端 camelCase + serde rename

### Bug: 日志模块 macOS 路径硬编码

**logger.rs 第 72-75 行**: clear_logs() 硬编码 `~/Library/Logs/nextype-windows/clipboard-sync.log`，Windows 上完全无法工作。

**logger.rs 第 35-41 行**: Windows 分支日志目录回退到 current_dir().join("logs")，不是 Windows 标准做法（应使用 %LOCALAPPDATA%）。

### 版本号不一致

| 来源 | 版本号 |
|------|--------|
| Cargo.toml | 2.0.0 |
| tauri.conf.json | 1.0.0 |
| config.rs default_version() | 2.0.0 |

---

## 3. 构建配置 -- ⚠️ 需改进

### tauri.conf.json

| 配置项 | 当前值 | 评估 |
|--------|--------|------|
| identifier | com.nextype.app | ⚠️ 与 Mac 端相同 |
| bundle.targets | ["msi", "nsis"] | ✅ 双格式 |
| certificateThumbprint | null | ❌ 未签名 |
| timestampUrl | "" | ❌ 空字符串 |
| security.csp | null | ❌ CSP 关闭 |
| frontendDist | "../src" | ✅ 静态 HTML |

### 缺失配置

- **无 capabilities 目录**: Tauri 2.x 需要定义权限范围，可能导致运行时调用被拒绝
- **未指定目标架构**: 默认编译当前平台，分发 x64+arm64 需交叉编译
- **托盘图标使用 iconTemplate.png**: macOS Template 格式，Windows 上可能显示不清晰

---

## 4. 分发就绪度 -- ❌ 阻塞

| 项目 | 状态 | 说明 |
|------|------|------|
| 代码签名证书 | ❌ 缺失 | 无 Authenticode 证书，SmartScreen 会拦截 |
| 版本号统一 | ❌ | Cargo.toml 与 tauri.conf.json 不一致 |
| 安装包配置 | ✅ | MSI + NSIS 双格式 |
| 自动更新 | ❌ | 无 tauri-plugin-updater |
| timestampUrl | ❌ | 空字符串 |
| identifier | ⚠️ | 与 Mac 端相同，有冲突风险 |
| Windows 真机编译 | ❌ | 所有代码未经 Windows 编译测试 |

**EV 代码签名证书**: 约 300-500 美元/年，购买后 SmartScreen 仍需积累信誉（OV 证书首次会被警告，EV 证书可立即通过）。

---

## 5. 安全性 -- ⚠️ 需改进

### E2E 加密: ✅

- CryptoJS 兼容 AES-256-CBC
- 配对时生成 256 位随机密钥
- 旧版设备无密钥时回退到 deviceId

### 密钥存储: ⚠️

- 使用 tauri-plugin-store 明文 JSON 存储（%APPDATA%/com.nextype.app/config.json）
- 未使用 Windows Credential Manager

### CSP: ❌

- csp: null 完全禁用内容安全策略

### unsafe 代码审计（4 处，均为标准 Windows API）

1. clipboard.rs: SendInput 模拟按键
2. tray.rs: GetCursorPos 获取鼠标位置
3. relay_client.rs: GetLastInputInfo + GetTickCount64 获取空闲时间
4. config.rs: GetComputerNameW 获取计算机名

用法正确，风险可控。

---

## 6. 已知问题与风险 -- ⚠️ 需改进

### macOS 残留（最突出问题类别）

| 位置 | 残留内容 |
|------|---------|
| onboarding.html 第 209-226 行 | 辅助功能权限 + System Events（macOS 独有） |
| preferences.html 第 126-131 行 | Dock 图标开关 |
| preferences.html 第 153-160 行 | 辅助功能权限设置 |
| onboarding.html 第 255 行 | "同一 Wi-Fi"措辞（实际用中继） |
| logger.rs clear_logs() | macOS 路径硬编码 |
| figma-pc-preferences.html | Figma 设计稿遗留 |

### 未经 Windows 真机验证

IMPLEMENTATION_SUMMARY.md 明确记录"由于当前在 macOS 环境下开发，无法进行实际的编译和测试"。所有测试清单均标记为未完成。

### 网络监测

5 秒轮询 local_ip_address::local_ip()，多网络适配器（如 VPN）可能误判。

---

## 7. 依赖管理 -- ✅ 就绪

关键依赖:
- tauri 2, tokio 1 (full), tokio-tungstenite 0.24 (native-tls)
- aes 0.8 + cbc 0.1 (RustCrypto)
- windows 0.58 (条件编译)
- tauri-plugin-single-instance 2.0.0-rc.3 (RC 版本)

无 npm 依赖（纯静态前端）。

---

## 8. Windows 适配度 -- ⚠️ 需改进

### 已完成

| 功能 | Mac 实现 | Windows 实现 |
|------|----------|-------------|
| 粘贴模拟 | AppleScript Cmd+V | SendInput Ctrl+V |
| 开机启动 | LaunchAgent | 注册表（autostart 插件） |
| 空闲时间 | -- | GetLastInputInfo + GetTickCount64 |
| 鼠标位置 | -- | GetCursorPos |
| 计算机名 | hostname | GetComputerNameW + 回退 |
| 窗口子系统 | -- | windows_subsystem = "windows" |

### 未完成

1. 日志路径使用 current_dir (应使用 %LOCALAPPDATA%)
2. 托盘图标 icon_as_template(true) 是 macOS 概念
3. 前端 macOS 残留 UI
4. identifier 与 Mac 端相同
5. Windows 通知 AppUserModelId 未配置

---

## 优先行动项

### P0（阻塞发布）

1. **修复 logger.rs macOS 路径硬编码**: clear_logs() 使用 macOS 路径，Windows 必然失败
2. **统一版本号**: tauri.conf.json (1.0.0) 与 Cargo.toml (2.0.0) 不一致
3. **Windows 真机编译验证**: 当前所有代码未经 Windows 编译测试

### P1（影响用户体验）

4. 适配引导页: 跳过或重写权限页，去除 macOS 说明
5. 隐藏 Dock 图标设置 + 辅助功能权限设置
6. 修正"同一 Wi-Fi"措辞
7. 删除 figma-pc-preferences.html

### P2（发布前建议完成）

8. 配置代码签名证书（避免 SmartScreen 拦截）
9. 添加 Tauri capabilities 配置
10. 配置 CSP
11. Windows identifier 改为 com.nextype.app.windows
12. 自动更新机制
13. 升级 single-instance 到稳定版
