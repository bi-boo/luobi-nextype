# 落笔 Nextype

跨平台加密文本传输工具 -- 手机打字，电脑上屏。

## 特性

- **端到端加密**：所有传输数据经 AES-256-CBC 加密，中继服务器无法解密
- **无需注册**：扫码配对即用，不收集任何个人信息
- **多平台支持**：macOS / Windows / Android / iOS
- **低延迟**：文字在设备间近实时同步
- **开源透明**：全部源代码公开，可自建中继服务器

## 下载

| 平台 | 状态 | 下载 |
|------|------|------|
| macOS | 稳定 | [GitHub Releases](https://github.com/bi-boo/luobi-nextype/releases) |
| Android | 稳定 | [GitHub Releases](https://github.com/bi-boo/luobi-nextype/releases) |
| iOS | 基本可用 | App Store（即将上架） |
| Windows | 开发中 | 即将发布 |

## 快速开始

1. 在电脑和手机上分别安装落笔 Nextype
2. 电脑端点击「连接手机」，显示配对二维码
3. 手机端扫码完成配对
4. 在手机上打字或语音输入，文字实时出现在电脑上

## 项目结构

| 目录 | 说明 | 技术栈 |
|------|------|--------|
| `NextypeMac/` | macOS 桌面端 | Tauri 2.x（Rust + HTML/CSS/JS） |
| `NextypeWindows/` | Windows 桌面端 | Tauri 2.x（Rust + HTML/CSS/JS） |
| `NextypeAndroid/` | Android 端 | Kotlin |
| `NextypeApp/` | iOS 端 | Swift |
| `relay-server/` | 中继服务器 | Node.js + WebSocket |
| `website/` | 官方网站 | HTML / CSS |
| `docs/` | 项目文档 | Markdown |

## 构建

各端的构建说明请参考对应目录下的 README：

- [macOS 端构建](./NextypeMac/README.md)
- [Windows 端构建](./NextypeWindows/README.md)
- [Android 端构建](./NextypeAndroid/README.md)
- [iOS 端构建](./NextypeApp/README.md)
- [中继服务器部署](./relay-server/README.md)

## 自建中继服务器

落笔 Nextype 默认连接官方中继服务器。如果你希望完全掌控数据链路，可以自建中继服务器。详见 [中继服务器自建指南](./relay-server/README.md)。

## 参与贡献

欢迎提交 Issue 和 Pull Request。详见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 许可证

本项目基于 [AGPL-3.0](./LICENSE) 许可证开源。

Copyright (c) 2024-2026 Zheng Bao

---

## English Summary

**Nextype** is a cross-platform encrypted text transfer tool. Type on your phone, and the text appears on your computer in real time.

- End-to-end AES-256-CBC encryption
- No registration required — pair via QR code
- Supports macOS, Windows, Android, and iOS
- Open source under AGPL-3.0 — self-host your own relay server

For build instructions and more details, see the subdirectory READMEs linked above.
