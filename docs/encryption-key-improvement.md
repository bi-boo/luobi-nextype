# 加密密钥安全改进方案

## 当前现状

- 手机端发送消息时，使用手机的 `deviceId` 作为 AES-256-CBC 加密密钥
- `deviceId` 在 WebSocket 通信中以明文传输（relay 消息的 `from`/`to` 字段）
- 中继服务器或任何能监听 WebSocket 的人理论上可以解密所有传输内容
- 加密实现使用 CryptoJS 兼容格式（EVP_BytesToKey 密钥派生 + AES-256-CBC）

## 风险评估

- **实际风险等级**：低
- 攻击者需要反编译 App 才能知道密钥来源是 deviceId
- 中继服务器由自己控制，非公共服务
- 产品场景为输入法工具，传输内容敏感度有限
- 当前阶段可接受，用户量增长或涉及敏感数据时需升级

## 改进方案：配对时协商共享密钥

### 流程

1. 电脑端配对时生成随机 256 位密钥
2. 配对码验证成功后，电脑通过 `pairing_success` 响应将密钥发给手机
3. 手机将密钥保存到 `PairedMac.encryptionKey` 字段
4. 后续所有通信使用该共享密钥加密

### 需要改动的地方

- **Tauri 端**：配对流程中生成随机密钥，通过中继服务器发送给手机
- **iOS 端**：`PairingCodeView.startPairing()` 中从 `pairing_success` 响应提取密钥，存入 `PairedMac.encryptionKey`
- **iOS 端**：`MainInputView.sendMessage()` 中改用 `device.encryptionKey` 而非 `myDeviceId`
- **Android 端**：同步修改
- **中继服务器**：无需改动（只是透传）

### 注意事项

- 配对那一瞬间密钥经过中继服务器，如果需要更高安全性可引入 Diffie-Hellman 密钥交换
- 已配对的旧设备需要兼容处理（检测到 encryptionKey == deviceId 时提示重新配对或保持旧逻辑）
- 电脑端和手机端需要同时发版

## 优先级

后续版本实现，非紧急。
