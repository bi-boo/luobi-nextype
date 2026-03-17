# iOS 端审查报告 (NextypeApp / Swift)

**审查日期**: 2026-03-11
**代码规模**: 14 个 Swift 文件, 2997 行
**Bundle ID**: com.nextype.app
**部署目标**: iOS 17.0

---

## 总览评级

| 维度 | 评级 | 核心发现 |
|------|------|---------|
| 功能完整性 | ⚠️ 需改进 | 语音输入已移除但 PRD 未同步；使用说明已实现但 PRD 未更新 |
| 代码质量 | ✅ 就绪 | 重构效果显著，从 18 文件/4808 行精简至 14 文件/2997 行；旧架构问题全部解决 |
| 构建与签名 | ⚠️ 需改进 | Xcode beta 版本构建、iPhone 横屏配置矛盾、Bundle ID 与文档不一致 |
| 分发就绪度 | ⚠️ 需改进 | 需正式版 Xcode 构建；Dark/Tinted App Icon 无图片；加密出口合规声明需复核 |
| 安全性 | ✅ 就绪 | 三个 P0 安全问题全部解决；密钥迁移至 Keychain；ATS 恢复默认严格模式 |
| 已知问题与风险 | ⚠️ 需改进 | 架构文档和 PRD 严重过时；62 处 print 日志需清理 |
| 依赖管理 | ✅ 就绪 | 零第三方依赖，框架引用干净 |
| 测试覆盖 | ❌ 阻塞 | 无任何单元测试或 UI 测试 |

---

## 1. 功能完整性 -- ⚠️ 需改进

### PRD 对照

| # | PRD 功能 | 状态 | 说明 |
|---|---------|------|------|
| 1 | 文本输入与插入（paste） | ✅ | MainInputView.syncContent() |
| 2 | 文本发送（paste-enter） | ✅ | MainInputView.sendContent() |
| 3 | 清空输入框 | ✅ | 含上滑恢复手势 |
| 4 | 设备配对（配对码） | ✅ | 统一中继配对 |
| 5 | 多设备管理 | ✅ | 添加、删除、切换、编辑名称和图标 |
| 6 | 语音输入（火山引擎 ASR） | ❌ 已移除 | VolcanoASRManager.swift 已不存在 |
| 7 | 公网中继连接 | ✅ | ConnectionManager 统一管理 |
| 8 | AES 加密通信 | ✅ | EncryptionManager.swift |
| 9 | 剪贴板同步设置 | ✅ | SettingsView 中可分别控制 |
| 10 | 惯用手设置 | ✅ | |
| 11 | 输入字号设置 | ✅ | 5 档可调 |
| 12 | 上滑重发/恢复手势 | ✅ | |
| 13 | 设备信任列表同步 | ✅ | 从中继服务器同步 |
| 14 | 空状态引导页 | ✅ | 含下载链接复制和自动恢复 |
| 15 | 屏幕常亮 + 闲置自动变暗 | ✅ | ScreenDimManager 状态机 |
| 16 | PC 快捷键远程控制响应 | ✅ | send/insert/clear 指令 |
| 17 | 后台零耗电策略 | ✅ | scenePhase 监听 |
| 18 | 屏幕参数自动上报 | ✅ | device_info 消息 |
| 19 | 暗黑模式支持 | ✅ | 全部使用语义色 |
| 20 | 使用说明/关于页 | ✅ | AboutView + UsageGuideView（PRD 标记为待实现，已完成） |

### 关键发现

1. **语音输入功能已完全移除**: PRD 标注为"已实现"，但代码中已无任何语音相关引用。PRD 需更新。
2. **使用说明页已新增**: PRD 标注为"待实现"，实际已完成。
3. **自动切换到最近活跃设备**: discover -> server_list -> 自动切换逻辑，PRD 未提及的新功能。

---

## 2. 代码质量 -- ✅ 就绪

### 优点

- 重构后架构清晰：视图层 / 管理层 / 模型层分离
- ConnectionManager 统一了连接管理（626 行，逻辑清晰）
- MainInputView 从 1028 行缩减至 624 行
- 错误处理完善：加密 throws 错误链、指数退避重连（1.5^n 秒，最大 30 秒）
- 回调闭包统一使用 `[weak self]`，无循环引用
- Swift 6 MainActor 隔离正确处理

### 问题

| 问题 | 严重程度 |
|------|----------|
| 62 处 print 日志，生产环境噪声 | 中 |
| AccentColor.colorset 中多余的 AppIcon.png（320KB） | 低 |
| ConnectionManager.handleMessage 约 170 行 switch | 低 |
| PairingCodeView 第 209 行残留注释 | 低 |

---

## 3. 构建与签名 -- ⚠️ 需改进

| 配置项 | 当前值 | 评估 |
|--------|--------|------|
| Bundle ID | com.nextype.app | ⚠️ 与 PRD 记录的 cn.yuanfengai.NextypeApp 不一致 |
| 部署目标 | iOS 17.0 | ✅ |
| 版本号 | 1.0.0 (Build 1) | ✅ |
| 签名方式 | Automatic | 需自行在 Xcode 中配置开发者团队 |
| Xcode 版本 | 26.1.1（beta） | ⚠️ App Store 不接受 beta Xcode 构建 |
| iPhone 横屏 | Info.plist 仅竖屏，但 project.pbxproj 含横屏 | ⚠️ 配置矛盾 |

---

## 4. 分发就绪度 -- ⚠️ 需改进

| 检查项 | 状态 |
|--------|------|
| Info.plist 完整性 | ✅ CFBundleDisplayName、ITSAppUsesNonExemptEncryption 等齐全 |
| AppIcon (1024x1024) | ✅ 存在 |
| Dark/Tinted Icon 变体 | ⚠️ Contents.json 声明了但无图片文件 |
| Privacy Manifest | ✅ NSPrivacyTracking=false, DeviceID for AppFunctionality |
| 加密出口合规 | ⚠️ 声明 ITSAppUsesNonExemptEncryption=false 但实际使用了 AES-256-CBC |
| Xcode 正式版构建 | ❌ 当前为 beta 版 |

---

## 5. 安全性 -- ✅ 就绪

### 已解决的 P0 问题

| 原 P0 问题 | 当前状态 |
|-----------|---------|
| 火山引擎 API 密钥硬编码 | ✅ 语音功能完全移除 |
| 加密调试日志输出明文和密钥 | ✅ EncryptionManager 中无 print |
| NSAllowsArbitraryLoads = true | ✅ ATS 恢复默认严格模式 |

### 密钥存储: ✅ 全部使用 Keychain

- 设备 ID: kSecAttrAccessibleAfterFirstUnlock
- 配对设备列表（含 encryptionKey）: Keychain
- lastConnectedDeviceId: Keychain

### 加密实现: ✅

- AES-256-CBC + PKCS7 + CryptoJS 兼容
- Salt 使用 SecRandomCopyBytes
- 中继 WSS + ATS 默认严格模式

---

## 6. 已知问题与风险 -- ⚠️ 需改进

- ✅ 代码中无 TODO/FIXME/HACK
- ⚠️ 架构文档 ios-architecture.md 仍描述 18 文件/4808 行旧架构
- ⚠️ PRD 多处过时（语音、Bundle ID、代码行数）
- ✅ 无兼容性问题（部署目标 iOS 17.0 与使用的 API 匹配）

---

## 7. 依赖管理 -- ✅ 就绪

零第三方依赖。纯原生实现，仅使用系统框架：SwiftUI、Foundation、UIKit、Combine、CommonCrypto、Security。

---

## 8. 测试覆盖 -- ❌ 阻塞

无任何单元测试或 UI 测试。project.pbxproj 中只有一个 target。

---

## 优先行动项

### P0（提交前必须修复）

1. 使用正式版 Xcode 重新打开项目并构建
2. 修复 iPhone 横屏配置矛盾（project.pbxproj 中移除 LandscapeLeft/Right）
3. 复核 ITSAppUsesNonExemptEncryption 声明（使用了 AES 加密但声明为 false）

### P1（强烈建议）

4. 更新 PRD 和架构文档
5. 添加 EncryptionManager 单元测试
6. 将 62 处 print 改为 `#if DEBUG` 或 os.Logger

### P2（改善项）

7. 为 Dark/Tinted App Icon 提供图片资源
8. 清理 AccentColor.colorset 中多余的 AppIcon.png
