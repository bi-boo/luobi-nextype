# 落笔 Nextype — macOS 端

基于 Tauri 2.x 的 macOS 桌面客户端。

## 技术栈

- 后端: Rust (Tauri 2.x)
- 前端: 原生 HTML / CSS / JavaScript
- 系统要求: macOS 11.0+

## 前置条件

- [Rust](https://rustup.rs/) (最新稳定版)
- [Node.js](https://nodejs.org/) 18+
- Xcode Command Line Tools: `xcode-select --install`

## 开发

```bash
cd NextypeMac
npm install
npm run tauri dev
```

## 构建

```bash
# 无签名构建（开发测试用）
npm run tauri build

# 带签名构建（需要 Apple Developer 证书）
# 设置环境变量后运行 build.sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_TEAM_ID="YOUR_TEAM_ID"
bash build.sh
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 项目结构

```
NextypeMac/
├── src/                  # 前端代码 (HTML/CSS/JS)
├── src-tauri/
│   ├── src/
│   │   ├── commands/     # Tauri 命令处理
│   │   ├── services/     # 核心服务 (中继、剪贴板、快捷键)
│   │   └── utils/        # 工具模块
│   ├── Cargo.toml
│   └── tauri.conf.json
├── build.sh              # 签名构建脚本
└── package.json
```

## 许可证

[AGPL-3.0](../LICENSE)
