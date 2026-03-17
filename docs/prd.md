# 落笔 Nextype — 项目总览

## 项目简介

落笔 Nextype 是一款「手机输入，电脑出字」的跨设备文字同步工具。用户在手机上通过键盘或语音输入文字，内容通过公网中继服务器实时同步到电脑端，支持自动粘贴到光标位置（插入）或粘贴并回车（发送）。本质上是将手机变成电脑的无线键盘。

核心特点：
- **无需登录**，不收集个人信息
- **端到端加密**（AES-256-CBC），中继服务器不保留任何传输内容
- **多设备配对**，支持自动切换到最近活跃的电脑
- **PC 端快捷键远程控制**，可触发手机端发送/插入/清空/模拟点击

## 各端产品文档索引

| 端 | PRD 文档 | 架构文档 | 状态 |
|----|----------|----------|------|
| Mac 桌面端 (Tauri) | [mac-prd.md](mac-prd.md) | [mac-architecture.md](mac-architecture.md) | ✅ 主力开发中 |
| Android 移动端 | [android-prd.md](android-prd.md) | [android-architecture.md](android-architecture.md) | ✅ 主力开发中 |
| iOS 移动端 | [ios-prd.md](ios-prd.md) | [ios-architecture.md](ios-architecture.md) | ✅ 重构完成 |
| Windows 桌面端 | — | — | 🔲 待开发 |

## 其他文档

| 文档 | 说明 |
|------|------|
| [relay-server.md](relay-server.md) | 中继服务器协议与架构文档，包含完整消息协议表、配对流程、心跳机制、数据持久化结构、部署配置 |
| [cross-platform-overview.md](cross-platform-overview.md) | 跨端对比总览，包含功能对比矩阵、共享组件说明、平台差异、iOS 代码质量评估、Windows/iOS 开发规划、废弃代码汇总 |
| [architecture.md](architecture.md) | 系统级架构总览，包含通信拓扑、加密方案、通信协议、技术栈一览、关键架构决策 |

## 项目结构

```
落笔 Nextype/
├── NextypeTauri/        # Mac 桌面端（Tauri 2.x，当前维护版本）
├── NextypeAndroid/      # Android 移动端
├── NextypeApp/          # iOS 移动端
├── relay-server/        # 中继服务器（Node.js）
├── Nextype 官网/        # 官方网站
├── electron-app/        # [已废弃] 旧版 Electron 桌面端
└── docs/                # 项目文档
```

## 当前开发状态总结

### Mac 端 (Tauri)
从 Electron 版本迁移完成，功能完整。支持配对码生成、剪贴板同步、快捷键远程控制（含 Fn 键和模拟长按）、托盘菜单、引导流程、日志系统、使用统计、深色/浅色模式。前端仍保留 Electron 兼容层（tauri-bridge.js），长期应重构为直接使用 Tauri API。

### Android 端
功能最完整的移动端。已完成 WebSocket 三合一架构优化（统一单条连接），支持文本输入发送、语音输入（功能隐藏）、多设备管理与自动切换（Follow the Light）、远程控制响应（含 AccessibilityService 模拟点击/长按）、屏幕常亮与自动变暗、折叠屏适配、后台零耗电。

### iOS 端
重构完成（2026-03-11 审查通过）。代码从 18 文件/4808 行精简至 14 文件/2997 行。主要完成：移除语音功能、合并双通道为 ConnectionManager 统一架构、迁移密钥至 Keychain、补齐屏幕常亮/自动变暗/远程控制/自动切换设备/暗黑模式/使用说明等功能、ATS 恢复严格模式。

### 中继服务器
Node.js + ws 库实现，部署在 `nextypeapi.yuanfengai.cn:8443`（WSS）。负责设备注册、配对管理、消息转发、心跳保活。数据持久化使用 JSON 文件。当前安全局限：消息转发未强制校验配对关系、部署脚本硬编码服务器密码。

### Windows 端
待开发。计划使用 Tauri 2.x，可从 Mac 端复用约 90% 的 Rust 后端代码和 95% 的前端代码，主要需重新开发剪贴板操作（SendInput API）、全局快捷键（Windows Hook）、托盘菜单等平台相关模块。

## Windows 端开发进度

**状态**: ✅ 基础架构完成，待测试

**完成项**:
- 项目结构创建
- Rust 后端代码移植（90% 复用 Mac 端）
- Windows 剪贴板实现（SendInput API）
- 快捷键管理器适配
- 配置文件调整
- 文档编写（PRD + 架构文档）

**待完成**:
- Windows 环境下的编译测试
- 功能验证（配对、剪贴板、快捷键）
- 打包测试（MSI/NSIS）
- 图标优化

**技术要点**:
- 使用 Windows SendInput API 替代 AppleScript
- 不支持 Fn 键（Windows 系统限制）
- 使用 .ico 格式托盘图标
- 通过注册表实现开机启动
