<p align="center">
  <img src="website/assets/logo.png" alt="落笔 Nextype" width="80">
</p>

<h1 align="center">落笔 Nextype</h1>

<p align="center">
  <strong>手机输入，电脑输出</strong><br>
  不做语音输入法，只做内容搬运工
</p>

<p align="center">
  <a href="https://github.com/bi-boo/luobi-nextype/releases">下载</a> · <a href="https://yuanfengai.cn">官网</a> · <a href="./CONTRIBUTING.md">参与贡献</a>
</p>

---

## 解决什么问题？

电脑上的语音输入法有个尴尬的问题：**你得对着屏幕说话**。

在开放工位、会议室、咖啡馆，旁边坐着人，对着电脑说话既社死又录进去一堆杂音。手机语音输入法（豆包、讯飞、iOS 原生）用了很久，识别准确率已经很高了——但这些输入法**没有电脑版**。

**落笔解决的就是这个断层**：手机上说，电脑上出。你已经用顺手的语音输入法一个字都不用换，识别效果零损耗。

## 适合谁用？

> **"开放工位旁边坐着人，根本没办法对着屏幕说话。后来试了落笔，手机放到嘴边轻声说完，内容直接出现在光标那里，旁边同事完全不知道我在干嘛。"**
> — 王磊，互联网从业者

> **"豆包输入法用了很久，专业词、人名都摸透了，基本不用纠正。但豆包没有电脑版。落笔把这个问题绕过去了——手机上说，内容直接到电脑，识别效果一点没损耗。"**
> — 林静，产品经理

> **"家里一台电脑，公司一台电脑，不可能每个地方都买麦克风。手机随时在身上，落笔直接把手机变成麦克风，配上 Cursor 简直是 Vibe Coding 标配。"**
> — Ryan M.，独立开发者

## 核心特性

- **端到端加密** — AES-256-CBC 加密传输，中继服务器无法解密你的内容
- **无需注册** — 输入配对码即用，不收集任何个人信息
- **多平台** — macOS / Windows / Android / iOS
- **低延迟** — 文字在设备间近实时同步
- **开源透明** — 全部源代码公开，可自建中继服务器

## 下载

| 平台 | 状态 | 下载 |
|------|------|------|
| macOS | 稳定 | [GitHub Releases](https://github.com/bi-boo/luobi-nextype/releases) |
| Android | 稳定 | [GitHub Releases](https://github.com/bi-boo/luobi-nextype/releases) |
| Windows | 可用，未充分测试 | [GitHub Releases](https://github.com/bi-boo/luobi-nextype/releases) |
| iOS | 源码可用 | 可自行克隆项目用 Xcode 构建安装，App Store 版本筹备中 |

## 快速开始

**1. 安装** — 在电脑和手机上分别安装落笔 Nextype

**2. 配对** — 电脑端点击「连接手机」显示配对码，手机端输入配对码完成连接

**3. 使用** — 在手机上打字或语音输入，点击发送，文字实时出现在电脑光标处

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
- No registration required — pair with a code
- Supports macOS, Windows, Android, and iOS
- Open source under AGPL-3.0 — self-host your own relay server

For build instructions and more details, see the subdirectory READMEs linked above.
