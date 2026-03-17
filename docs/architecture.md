# 落笔 Nextype — 系统级架构总览

## 系统整体架构

落笔 Nextype 采用「手机 ↔ 中继服务器 ↔ PC」的星型通信拓扑，所有端通过公网中继服务器进行消息转发。

```
┌─────────────────┐                                    ┌─────────────────┐
│   Android 端     │                                    │   Mac 端 (Tauri) │
│   role: client   │──── WSS ────┐          ┌── WSS ───│   role: server   │
└─────────────────┘              │          │          └─────────────────┘
                                 ▼          ▼
                          ┌──────────────────────┐
                          │    中继服务器          │
                          │    Node.js + ws       │
                          │    WSS :8443          │
                          │                      │
                          │  connectedDevices    │
                          │  pairingCodes        │
                          │  trust_relationships │
                          └──────────────────────┘
                                 ▲          ▲
┌─────────────────┐              │          │          ┌─────────────────┐
│   iOS 端         │──── WSS ────┘          └── WSS ───│  Windows 端      │
│   role: client   │                                    │  role: server    │
└─────────────────┘                                    │  （待开发）       │
                                                       └─────────────────┘
```

**通信特点**：
- 所有端与中继服务器之间使用 WebSocket (WSS) 长连接
- PC 端注册为 `server` 角色，手机端注册为 `client` 角色
- 消息通过 `relay` 类型在设备间转发，中继服务器不解析业务数据
- 中继服务器不保留任何传输内容，仅维护连接状态和配对关系

---

## 加密方案概述

### AES-256-CBC（CryptoJS 兼容格式）

所有端使用统一的加密方案，确保跨端互通：

```
密钥字符串（发送方 deviceId）
        │
        ▼
EVP_BytesToKey(password, salt)
  ├── MD5(password + salt) → block1 (16B)
  ├── MD5(block1 + password + salt) → block2 (16B)
  └── MD5(block2 + password + salt) → block3 (16B)
        │
        ▼
  Key = block1 + block2 (32 bytes)
  IV  = block3 (16 bytes)
        │
        ▼
AES-256-CBC + PKCS7 Padding
        │
        ▼
输出 = Base64("Salted__" + salt(8B) + ciphertext)
```

- **密码来源**：发送方的 `deviceId`，PC 端在配对时已获取
- **Salt**：每次加密随机生成 8 字节
- **输出格式**：与 `CryptoJS.AES.encrypt()` 完全兼容
- **流向**：手机端加密 → 中继透传 → PC 端解密

---

## 通信协议概述

### WebSocket 消息分类

所有消息基于 JSON over WebSocket，按功能分为以下几类：

**连接与身份**：
- `register` — 设备注册（携带 deviceId、role、deviceName）
- `registered` — 注册成功确认
- `connected` — 连接建立欢迎消息

**配对流程**：
- `register_code` — PC 端注册 4 位配对码（60 秒有效）
- `verify_code` — 手机端验证配对码
- `pairing_success` / `pairing_completed` — 配对成功通知双方
- `code_conflict` — 配对码冲突

**消息转发**：
- `relay` — 核心转发消息，`data` 字段承载业务数据（剪贴板内容、远程控制指令、屏幕参数等）

**心跳保活**：
- `heartbeat` / `heartbeat_ack` — 客户端心跳与服务器确认
- `client_heartbeat` — 服务器转发手机心跳给 PC 端

**设备状态广播**：
- `client_online` / `client_offline` — 手机上下线通知
- `server_online` / `server_offline` — PC 上下线通知

**信任管理**：
- `sync_trust_list` / `trust_list` — 信任列表同步
- `unpair_device` / `device_unpaired` — 解除配对

完整协议详见 [relay-server.md](relay-server.md)。

### relay.data 业务消息

| type | 方向 | 说明 |
|------|------|------|
| `clipboard` | 手机→PC | 加密文本内容，action 为 paste 或 paste-enter |
| `ack` | PC→手机 | 内容处理成功回执 |
| `command` | PC→手机 | 远程控制指令（send/insert/clear/tap/touch_down/touch_up/touch_heartbeat） |
| `device_info` | 手机→PC | 连接建立后上报屏幕尺寸 |
| `screen_changed` | 手机→PC | 屏幕尺寸变化通知（折叠屏） |
| `ping` | 手机→PC | 上线通知 |
| `error` | 双向 | 错误信息 |

---

## 各端技术栈一览

| 维度 | Mac (Tauri) | Android | iOS | 中继服务器 |
|------|-------------|---------|-----|-----------|
| 语言 | Rust + HTML/CSS/JS | Kotlin | Swift | Node.js |
| 框架 | Tauri 2.x | Android View (XML) | SwiftUI | ws 8.x |
| 网络库 | tokio-tungstenite | OkHttp 4 | URLSession | ws (npm) |
| 异步模型 | Tokio async/await | Kotlin Coroutines | Combine + GCD | Node.js 事件循环 |
| 加密 | aes + cbc + md-5 | javax.crypto | CommonCrypto | — (透传) |
| 持久化 | tauri-plugin-store (JSON) | SharedPreferences | Keychain | JSON 文件 |
| 最低版本 | macOS 10.13+ | Android 7.0 (API 24) | iOS 17.0 | Node.js 18+ |
| 代码规模 | ~632 行 Rust + ~3000 行前端 | ~6000 行 Kotlin | ~2997 行 Swift | ~400 行 JS |

---

## 各端架构文档索引

| 端 | 架构文档 | 核心内容 |
|----|----------|----------|
| Mac 桌面端 | [mac-architecture.md](mac-architecture.md) | Tauri Command 接口清单、服务层设计（relay_client/hotkey_manager/native_hotkey/clipboard/tray）、状态管理、日志架构、macOS 原生 API |
| Android 移动端 | [android-architecture.md](android-architecture.md) | 双通道→统一连接架构、MainActivity 核心控制器、AccessibilityService 模拟点击、火山引擎 ASR 二进制协议、生命周期管理 |
| iOS 移动端 | [ios-architecture.md](ios-architecture.md) | 双通道通信架构、核心类详解、代码质量逐文件评估（含评分）、问题清单与修复建议、重写 vs 修复建议 |
| 中继服务器 | [relay-server.md](relay-server.md) | 完整消息协议表、配对流程时序图、心跳与超时机制、数据持久化结构、部署配置、安全机制 |

---

## 关键架构决策记录

### 1. 公网中继优先，移除局域网直连

**决策**：Mac 端和 Android 端已完全移除局域网直连代码，统一使用公网中继通信。iOS 端保留局域网代码但优先使用中继。

**理由**：局域网直连在跨网络场景下不可用，且双通道架构增加了连接管理复杂度。公网中继通过 WSS 加密传输，配合端到端 AES 加密，安全性有保障。

### 2. CryptoJS 兼容的 AES-256-CBC 加密

**决策**：使用 EVP_BytesToKey + AES-256-CBC + OpenSSL 格式（Salted__ 前缀），而非更现代的 AES-GCM。

**理由**：项目最初的 Electron 版本使用 CryptoJS 库加密，为保持向后兼容和跨端互通，所有端统一实现 CryptoJS 兼容格式。密码为发送方 deviceId，双方在配对时已交换。

### 3. Android 端统一单条 WebSocket 连接

**决策**：将原来的三条 WebSocket 连接（数据通道、控制通道、同步通道）合并为一条统一连接。

**理由**：减少 66% 的连接资源占用，降低约 50% 耗电量，消除多连接间的状态同步问题。所有消息类型通过同一条连接处理，心跳统一为 30 秒。

### 4. 后台零耗电策略（Android）

**决策**：应用进入后台时断开所有 WebSocket 连接，停止所有定时器；回到前台时统一重建。

**理由**：不使用前台 Service，不保持后台连接，确保后台零网络活动。代价是回到前台需要 1-2 秒重建连接，但用户体验可接受。

### 5. 心跳与超时策略

**决策**：
- Android 端：30 秒双层心跳（应用层 + TCP ping）
- Mac 端：10 秒应用层心跳（附带系统闲置时间）
- iOS 端：20 秒 WebSocket ping
- 服务器超时：120 秒（4 倍心跳间隔），优雅关闭（`ws.close()` 而非 `ws.terminate()`）

**理由**：Mac 端 10 秒心跳是为了实时上报闲置时间，支持 Follow the Light 自动切换策略。服务器优雅关闭可触发客户端立即重连（2-4 秒），而非等待 TCP 超时（30-120 秒）。

### 6. Tauri 替代 Electron（Mac 端）

**决策**：Mac 桌面端从 Electron 迁移到 Tauri 2.x。

**理由**：大幅降低资源占用（内存从 ~200MB 降至 ~30MB），Rust 后端提供更好的性能和安全性。前端代码通过 tauri-bridge.js 兼容层保持了与 Electron 版本的兼容，降低迁移成本。

### 7. 配对码设计（4 位数字，7 种易记模式）

**决策**：使用 4 位数字配对码，优先生成 AABB、ABAB、连续递增/递减、回文等易记模式，冲突时切换纯随机。

**理由**：4 位数字在 60 秒有效期内碰撞概率极低，同时易于口头传达。7 种易记模式覆盖约 400+ 种组合，提升用户体验。

### 8. 设备 ID 生成策略

**决策**：各端采用不同的 ID 生成策略，但都保证稳定性。

| 端 | 算法 | 稳定性 |
|----|------|--------|
| Mac | SHA256(hostname-username-platform) 前 16 位 hex | 重装系统后可能变化 |
| Android | MD5(ANDROID_ID + salt) 前 16 位 hex | 卸载重装不变，恢复出厂重置后变化 |
| iOS | UUID 随机生成，Keychain 持久化 | 卸载重装后保留（Keychain 不随卸载清除） |

Android 的稳定 ID 支持卸载重装后自动恢复配对关系。iOS 的 UUID 方案在重装后需要重新配对，但可通过服务器信任列表部分恢复。
