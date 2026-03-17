# 落笔 Nextype — Windows 端技术架构文档

## 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 框架 | Tauri | 2.x |
| 后端语言 | Rust | 2021 Edition |
| 前端 | 原生 HTML/CSS/JS | - |
| 异步运行时 | Tokio | 1.x |
| WebSocket | tokio-tungstenite | 0.24 |
| Windows API | windows-rs | 0.58 |

---

## 项目结构

```
NextypeTauri/nextype-windows/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── state.rs
│       ├── commands/          # Tauri Commands
│       ├── services/          # 业务逻辑层
│       └── utils/             # 工具模块
└── src/                       # 前端资源
    ├── index.html
    ├── preferences.html
    ├── onboarding.html
    ├── logs.html
    └── style.css
```

---

## 核心模块

### 1. 剪贴板服务 (clipboard.rs)

**Windows 实现**：
- 使用 `windows::Win32::UI::Input::KeyboardAndMouse::SendInput` 模拟按键
- Ctrl+V: `SendInput([VK_CONTROL, VK_V])`
- Enter: `SendInput([VK_RETURN])`
- 不需要辅助功能权限

### 2. 快捷键管理 (hotkey_manager.rs)

**Windows 实现**：
- 使用 `tauri-plugin-global-shortcut` 注册全局快捷键
- 不支持 Fn 键（Windows 系统限制）
- 支持 Ctrl/Alt/Shift/Win 修饰键

### 3. 原生快捷键 (native_hotkey.rs)

**Windows 实现**：
- 简化实现，仅保留接口兼容
- 快捷键录入由前端 JavaScript 处理

### 4. 中继客户端 (relay_client.rs)

**跨平台实现**：
- 与 Mac 端完全相同
- WebSocket 通信
- AES-256-CBC 加密
- 自动重连

### 5. 托盘管理 (tray.rs)

**Windows 适配**：
- 使用 Tauri 的跨平台托盘 API
- 图标格式：.ico
- 菜单项与 Mac 端一致

---

## Windows 特有实现

### SendInput API

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, VK_CONTROL, VK_V, VK_RETURN,
};

fn send_key_combination(vk_codes: &[u16]) -> Result<(), String> {
    // 按下所有键
    for &vk in vk_codes {
        // 创建 INPUT 结构
    }
    // 释放所有键（逆序）
    SendInput(&inputs, size_of::<INPUT>())
}
```

### 开机启动

使用 `tauri-plugin-autostart`，在 Windows 上通过注册表实现：
- 注册表路径：`HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`

---

## 依赖项清单

### Windows 特有依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `windows` | 0.58 | Windows API 绑定 |
| - Win32_UI_Input_KeyboardAndMouse | - | SendInput 键盘模拟 |
| - Win32_Foundation | - | 基础类型 |

### 跨平台依赖

与 Mac 端相同，包括：
- Tauri 核心和插件
- tokio + tokio-tungstenite
- 加密库（aes, cbc, md-5, sha2）
- 序列化（serde, serde_json）

---

## 与 Mac 端的代码复用率

- **Rust 后端**: ~90% 复用
  - 完全复用：relay_client, device_manager, stats, utils
  - 部分修改：clipboard, hotkey_manager, native_hotkey
  - 移除：macOS 特定的 CGEventTap、AppleScript 代码

- **前端**: ~95% 复用
  - 完全复用：所有 HTML/CSS/JS 文件
  - 仅需调整：快捷键显示（Cmd → Ctrl）

---

## 构建配置

### tauri.conf.json

```json
{
  "identifier": "com.nextype.app.windows",
  "bundle": {
    "targets": ["msi", "nsis"],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256"
    }
  }
}
```

### Cargo.toml

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
] }
```

---

## 已知限制

1. **不支持 Fn 键**：Windows 系统限制，无法像 macOS 那样捕获 Fn 键
2. **无 Dock 图标控制**：Windows 没有 Dock 概念
3. **托盘图标格式**：必须使用 .ico 格式，不支持 Template 图标

---

## 测试清单

- [ ] 配对功能
- [ ] 剪贴板同步（粘贴、粘贴+回车）
- [ ] 快捷键远程控制
- [ ] 托盘菜单
- [ ] 偏好设置窗口
- [ ] 日志系统
- [ ] 开机启动
- [ ] 单实例保护
- [ ] 自动重连
