# 落笔 Nextype — Windows 端

基于 Tauri 2.x 的 Windows 桌面客户端。

## 技术栈

- 后端: Rust (Tauri 2.x)
- 前端: 原生 HTML / CSS / JavaScript
- 通信: WebSocket (tokio-tungstenite)
- 加密: AES-256-CBC (CryptoJS 兼容)
- 系统要求: Windows 10+

## 与 Mac 端的差异

1. **剪贴板操作**: 使用 Windows SendInput API 替代 AppleScript
2. **快捷键**: 使用 tauri-plugin-global-shortcut，不支持 Fn 键
3. **系统托盘**: 使用 Windows 系统托盘 API
4. **开机启动**: 使用 Windows 注册表
5. **无 Dock 图标**: Windows 没有 Dock 概念

## 前置条件

- [Rust](https://rustup.rs/) (最新稳定版)
- [Node.js](https://nodejs.org/) 18+
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 2019+（含 C++ 工作负载）
- WebView2 Runtime（Windows 11 内置；Windows 10 需手动安装）

## 开发

```bash
cd NextypeWindows
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 项目结构

```
NextypeWindows/
├── src/                  # 前端代码 (HTML/CSS/JS)
├── src-tauri/
│   ├── src/
│   │   ├── commands/     # Tauri 命令处理
│   │   ├── services/     # 核心服务 (中继、剪贴板、快捷键)
│   │   └── utils/        # 工具模块
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## 注意事项

- Windows 不需要辅助功能权限
- 快捷键使用 Ctrl 替代 Cmd
- 系统托盘图标需要 .ico 格式

## 许可证

[AGPL-3.0](../LICENSE)
