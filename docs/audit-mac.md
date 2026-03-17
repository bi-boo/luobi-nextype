# Mac 端审查报告 (NextypeMac / Tauri 2.x + Rust)

**审查日期**: 2026-03-11
**技术栈**: Tauri 2.x (Rust 后端 + 原生 HTML/CSS/JS 前端)
**签名身份**: 通过环境变量 `APPLE_SIGNING_IDENTITY` 配置
**最低系统版本**: macOS 11.0

---

## 总览评级

| 维度 | 评级 | 核心发现 |
|------|------|---------|
| 功能完整性 | ⚠️ 需改进 | PRD 14 项功能均已实现，但缺少自动更新，版本号三处不一致 |
| 代码质量 | ✅ 就绪 | 架构清晰、关注点分离好、错误处理规范 |
| 构建与签名 | ⚠️ 需改进 | Developer ID 签名已配置，但 notarization 流程缺失 |
| 分发就绪度 | ❌ 阻塞 | 未公证的 DMG 会被 Gatekeeper 拦截，无法正常分发 |
| 安全性 | ⚠️ 需改进 | CSP 已禁用、加密密钥明文存储、write_file 命令无路径限制 |
| 已知问题与风险 | ⚠️ 需改进 | unwrap() 可能 panic、版本号不一致、RC 依赖 |
| 依赖管理 | ⚠️ 需改进 | 锁文件齐全，但存在 RC 版依赖和版本号散乱 |
| 测试覆盖 | ❌ 阻塞 | 零测试 |

---

## 1. 功能完整性 -- ⚠️ 需改进

### PRD 对照（14 项核心功能）

| PRD 功能 | 实现文件 | 状态 |
|---------|---------|------|
| 设备配对 (QR/信任链) | relay_client.rs, device.rs | ✅ |
| 剪贴板同步 (粘贴/回车/复制) | clipboard.rs | ✅ |
| 快捷键远程控制 | hotkey_manager.rs | ✅ |
| 快捷键录制 | native_hotkey.rs (NSEvent) | ✅ |
| 屏幕坐标配置 | screen.rs | ✅ |
| 托盘菜单 | tray.rs | ✅ |
| 偏好设置窗口 (5 Tab) | preferences.html | ✅ |
| 引导流程 (4 步) | onboarding.html | ✅ |
| 日志系统 (4 层) | log_manager.rs, logs.html | ✅ |
| 使用统计 | stats.rs | ✅ |
| 系统集成 (辅助功能/开机启动/Dock) | system.rs | ✅ |
| Electron 数据迁移 | lib.rs setup | ✅ |
| 中继/网络管理 | relay_client.rs (WSS) | ✅ |
| 深色/亮色模式 | style.css | ✅ |
| 自动更新 | -- | ❌ 未实现 |

### 关键问题

**1.1 缺少自动更新机制**
未集成 tauri-plugin-updater，也没有替代的更新检查。用户只能手动下载新版 DMG。

**1.2 版本号三处不一致**

| 文件 | 版本号 |
|-----|--------|
| package.json | 0.1.0 |
| tauri.conf.json | 1.0.0 |
| config.rs 默认值 | 2.0.0 |

---

## 2. 代码质量 -- ✅ 就绪

### 架构优秀

- **Actor 模式**的 relay client（mpsc channel + tokio::select!），解决 WebSocket 并发控制
- **双轨快捷键系统**: tauri-plugin-global-shortcut 处理普通键 + CGEventTap 处理 Fn 键
- **4 层日志架构**: console + 文件 + 内存缓存 1000 条 + 前端推送
- **服务层分离清晰**: relay_client / hotkey_manager / native_hotkey / clipboard / tray / stats

### 问题

- native_hotkey.rs 中 5 处 unwrap() 在 CGEventTap 回调线程中，panic 会导致全局事件监听崩溃
- 代码中无 TODO/FIXME/HACK（积极信号）

---

## 3. 构建与签名 -- ⚠️ 需改进

### 已配置

- Developer ID 签名: 通过环境变量 `APPLE_SIGNING_IDENTITY` 配置
- Entitlements: JIT、网络、Apple Events 权限
- DMG + App 双目标
- build.sh 自动检测证书

### 问题

**3.1 未配置 Notarization（公证）-- 核心阻塞**

build.sh 中没有 xcrun notarytool submit。macOS 10.15+ 要求所有网页分发的 App 必须公证，否则 Gatekeeper 直接阻止打开。

**3.2 Hardened Runtime 权限审查**

entitlements.plist 声明了 allow-jit 和 allow-unsigned-executable-memory，会降低 Hardened Runtime 安全效果（WebView/JavaScriptCore 通常需要）。

**3.3 未启用 App Sandbox**

非 App Store 分发不强制，但应用需要 CGEventTap 和 AppleScript，启用完整沙盒有兼容性挑战。

---

## 4. 分发就绪度 -- ❌ 阻塞

| 要求 | 状态 | 影响 |
|-----|------|------|
| Developer ID 签名 | ✅ 已配置 | -- |
| Notarization（公证） | ❌ 未配置 | Gatekeeper 拦截，用户无法打开 |
| Staple | ❌ 未配置 | 离线用户无法验证 |
| 自动更新 | ❌ 未实现 | 需手动下载每个新版本 |

**修复方案**（在 build.sh 中添加）:
```bash
xcrun notarytool submit *.dmg \
  --apple-id "$APPLE_ID" \
  --password "$APP_SPECIFIC_PASSWORD" \
  --team-id "$APPLE_TEAM_ID" --wait
xcrun stapler staple *.dmg
```

---

## 5. 安全性 -- ⚠️ 需改进

### 做得好的方面

- E2E 加密: CryptoJS 兼容 AES-256-CBC
- WSS 加密连接
- 设备信任机制: QR 码配对 + 密钥交换
- 辅助功能权限正确检查

### 需要改进

| 问题 | 风险等级 | 说明 |
|------|----------|------|
| CSP 完全禁用 | 高 | tauri.conf.json 中 csp: null，WebView 无内容安全策略 |
| 加密密钥明文存储 | 中 | encryption_key 在 JSON 文件中明文存储，未使用 macOS Keychain |
| write_file 命令无路径限制 | 中 | 接受任意路径和内容，配合 CSP 缺失可被利用 |
| Capabilities 过于宽松 | 低 | 所有 Tauri 插件权限对所有窗口开放 |

---

## 6. 已知问题与风险 -- ⚠️ 需改进

- ⚠️ native_hotkey.rs 5 处 unwrap()，CGEventTap 线程中 panic 导致快捷键失效
- ⚠️ 网络监控使用 5 秒轮询（应使用 SCNetworkReachability）
- ⚠️ AppleScript 键盘模拟在安全设置严格的环境可能被阻止
- ⚠️ Electron 迁移代码应有时间阈值来清除
- ⚠️ tauri-plugin-single-instance 使用 RC 版本（2.0.0-rc.3）

---

## 7. 依赖管理 -- ⚠️ 需改进

- Cargo.lock 和 package-lock.json 均存在
- macOS 特定依赖正确隔离在 cfg(target_os = "macos") 下
- 约 30 个 Rust 依赖，数量合理

问题:
- tauri-plugin-single-instance 2.0.0-rc.3（RC 版本）
- 版本号三处不一致
- 多数依赖使用大版本范围（如 "2"），cargo update 可能引入不兼容变更

---

## 8. 测试覆盖 -- ❌ 阻塞

项目中不存在任何 #[test]、#[cfg(test)]、测试文件或测试目录。零测试。

高风险未测试模块:
- relay_client.rs（1100 行，最复杂模块）
- native_hotkey.rs（1100 行，系统级 API 交互）
- decrypt_cryptojs_aes()（加密核心函数）
- config.rs 自定义反序列化

---

## 优先行动项

### P0（分发前必须完成）

1. **配置 Notarization 流程**: 不修复则无法通过官网分发
2. **设置 CSP 策略**: 至少配置 `default-src 'self'; script-src 'self' 'unsafe-inline'`
3. **统一版本号管理**: 以 tauri.conf.json 为准，其他同步

### P1（强烈建议）

4. 加密密钥迁移到 Keychain（security-framework crate）
5. write_file 添加路径白名单
6. 为核心函数添加单元测试（decrypt_cryptojs_aes、AppConfig 反序列化）

### P2（改善项）

7. 集成 tauri-plugin-updater（自动更新）
8. 替换 unwrap() 为安全写法（parking_lot::Mutex 或 unwrap_or_else）
9. 升级 single-instance 到稳定版
