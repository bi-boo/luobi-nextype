# Android 端审查报告 (NextypeAndroid / Kotlin)

**审查日期**: 2026-03-11
**代码规模**: 15 个 Kotlin 源文件, 主文件 MainActivity.kt 约 2848 行
**Application ID**: com.nextype.app
**SDK**: compileSdk 34, minSdk 24, targetSdk 34

---

## 总览评级

| 维度 | 评级 | 核心发现 |
|------|------|----------|
| 功能完整性 | ⚠️ 需改进 | PRD 与代码有多处不一致（语音功能已删除、残留代码已清理），文档需同步 |
| 代码质量 | ⚠️ 需改进 | MainActivity 过于庞大（2848 行），存在线程安全隐患和重复代码 |
| 构建与签名 | ✅ 就绪 | 签名外部化，R8 混淆启用，SDK 版本合理 |
| 分发就绪度 | ⚠️ 需改进 | 缺少自适应图标和圆形图标，versionCode 需管理，需配置 AAB 构建 |
| 安全性 | ⚠️ 需改进 | 加密实现正确、网络安全配置完善，但 keystore 密码存在泄露风险 |
| 已知问题与风险 | ✅ 就绪 | 无 TODO/FIXME，残留代码和权限已全部清理 |
| 依赖管理 | ✅ 就绪 | 依赖精简，版本稳定 |
| 测试覆盖 | ❌ 阻塞 | 零测试 |
| 权限最小化 | ✅ 就绪 | 仅 3 个必要权限 + 1 个 Service 级别权限 |

---

## 1. 功能完整性 -- ⚠️ 需改进

### PRD 对照

| PRD 功能 | 状态 | 说明 |
|----------|------|------|
| 文本输入与发送 | ✅ | 输入框、发送(paste-enter)、插入(paste)、清空、AES 加密 |
| 上滑重发/恢复 | ✅ | 蓝色选中态、触觉反馈、预览文本、一次性气泡提示 |
| 设备配对（4位配对码） | ✅ | 4格输入框、自动跳转、禁用自动填充/验证码助手 |
| 多设备管理 | ✅ | BottomSheet、切换、编辑别名/图标、取消配对、在线状态 |
| 自动切换（Follow the Light） | ✅ | onResume 触发、闲置<120s、最短闲置优先 |
| 语音输入（火山引擎 ASR） | ⚠️ 代码已删除 | PRD 标注"代码完整保留"但实际已移除 |
| 远程控制响应 | ✅ | send/insert/clear/tap/touch 全部实现，500ms 防抖 |
| 模拟点击（AccessibilityService） | ✅ | 单击 50ms、长按、心跳检测、API 24 降级 |
| 屏幕常亮与自动变暗 | ✅ | 4档等待时间、7种唤醒触发源 |
| 屏幕参数上报 | ✅ | device_info 握手、screen_changed 变更、华为适配 |
| 设置页 | ✅ | 惯用手、字号5档、剪贴板同步、屏幕常亮子设置 |
| 设备恢复 | ✅ | EmptyStateActivity 自动恢复、基于 ANDROID_ID |
| 后台零耗电 | ✅ | onStop 全断、onResume 重建 |
| 辅助功能权限提示 | ✅ | 底部气泡、10秒防抖、5秒自动消失 |
| 欢迎页 / 使用说明页 | ✅ | Logo、配对按钮、域名复制 |

### PRD 与代码不一致项

1. PRD 标注"语音输入代码完整保留"，但 VolcanoASRManager.kt 和 VolcanoConfig.kt 已被删除
2. PRD 未记录新增类：CrashHandler、SecurePrefsHelper、ScenarioActivity、PrivacyPolicyActivity
3. 架构文档仍列出已删除文件

---

## 2. 代码质量 -- ⚠️ 需改进

### 优点

- 命名规范统一（Kotlin 风格）
- 日志体系完善（带 emoji 标记分级，Release 通过 ProGuard 移除）
- 生命周期管理清晰（零后台耗电策略）
- 错误处理覆盖完整
- 加密存储使用 EncryptedSharedPreferences + 回退机制

### 问题

| 问题 | 严重程度 | 说明 |
|------|----------|------|
| MainActivity 体量过大 | 中 | 2848 行单文件，违反单一职责 |
| 线程安全隐患 | 中 | isWebSocketConnected 和 dataWebSocket 在 OkHttp 回调线程和主线程间并发读写无同步 |
| Handler 泄漏风险 | 低 | scheduleDataReconnect 中的匿名 Handler 未被追踪 |
| 重复代码 | 低 | sendContent/sendContentWithText 逻辑高度重复 |
| build-and-install.sh 包名错误 | 低 | 使用 com.nextype.android 启动但 applicationId 是 com.nextype.app |

---

## 3. 构建与签名 -- ✅ 就绪

| 检查项 | 状态 |
|--------|------|
| AGP 8.2.0, Kotlin 1.9.20 | ✅ |
| compileSdk / targetSdk 34 | ✅ |
| minSdk 24 (覆盖 98%+ 设备) | ✅ |
| 签名配置（keystore.properties 外部化） | ✅ |
| R8 混淆 + 资源压缩 | ✅ |
| Release 日志移除（ProGuard assumenosideeffects） | ✅ |

注意: ProGuard 规则 `-keep class com.nextype.android.** { *; }` 保留了整个包，混淆几乎无效。

---

## 4. 分发就绪度 -- ⚠️ 需改进

| 检查项 | 状态 | 说明 |
|--------|------|------|
| App Icons | ⚠️ | 仅 ic_launcher.png，无圆形图标，无自适应图标 |
| 版本号 | ⚠️ | versionCode=1, versionName="1.0.0" 初始值，需递增管理 |
| AAB 构建 | ⚠️ | 未配置 bundleRelease（Google Play 要求 AAB） |
| AndroidManifest | ✅ | exported 属性正确 |
| 隐私政策 | ✅ | 已内置 PrivacyPolicyActivity |
| 深色模式 | ✅ | values-night/colors.xml |

---

## 5. 安全性 -- ⚠️ 需改进

| 检查项 | 状态 | 说明 |
|--------|------|------|
| AES-256-CBC 加密 | ✅ | CryptoJS 兼容，随机 salt |
| EncryptedSharedPreferences | ✅ | AES256_GCM + 回退机制 |
| 网络安全配置 | ✅ | cleartextTrafficPermitted=false |
| allowBackup=false | ✅ | 防 adb backup 提取 |
| Keystore 密码 | ❌ | keystore.properties 含明文密码，虽已 gitignore 但存在风险 |
| 中继 URL 硬编码 | ⚠️ | wss://nextypeapi.yuanfengai.cn:8443 分散在两处 |
| 加密密钥强度 | ⚠️ | 原始密钥空间仅 64 位（16 位十六进制 deviceId） |

---

## 6. 已知问题与风险 -- ✅ 就绪

- ✅ 无 TODO/FIXME/HACK
- ✅ 残留代码和权限已全部清理
- ⚠️ CrashHandler.getAppVersion() 使用 pInfo.longVersionCode（API 28+ only），minSdk 24 会导致 NoSuchMethodError
- ⚠️ 华为挖孔屏适配使用三层反射（含 sun.misc.Unsafe），高版本 Android 可能失效（已有 try-catch 降级）

---

## 7. 依赖管理 -- ✅ 就绪

| 依赖 | 版本 | 评估 |
|------|------|------|
| androidx.core:core-ktx | 1.12.0 | 稳定 |
| com.google.android.material | 1.11.0 | 稳定 |
| com.squareup.okhttp3 | 4.12.0 | 最新稳定版 |
| androidx.security:security-crypto | 1.1.0-alpha06 | alpha 但社区广泛使用，有降级保护 |
| kotlinx-coroutines-android | 1.7.3 | 稳定 |

未使用依赖（constraintlayout）已清理。

---

## 8. 测试覆盖 -- ❌ 阻塞

- test/ 和 androidTest/ 目录不存在
- build.gradle 声明了测试依赖但无测试代码

---

## 9. 权限最小化 -- ✅ 就绪

| 权限 | 必要性 |
|------|--------|
| INTERNET | ✅ WebSocket 通信 |
| ACCESS_NETWORK_STATE | ✅ 网络检测 |
| VIBRATE | ✅ 触觉反馈 |
| BIND_ACCESSIBILITY_SERVICE | ✅ 模拟点击（需用户手动开启） |

已清理: ACCESS_WIFI_STATE、CHANGE_WIFI_MULTICAST_STATE、CHANGE_NETWORK_STATE、RECORD_AUDIO

---

## 优先行动项

### P0（高优先级）

1. **Keystore 密码管理**: 密码改为从环境变量读取
2. **CrashHandler 兼容性修复**: 使用 PackageInfoCompat.getLongVersionCode()
3. **补充加密模块单元测试**: 验证 AES 与 CryptoJS 互操作性

### P1（中优先级）

4. 自适应图标 + 圆形图标
5. versionCode 递增管理机制
6. 中继 URL 常量化（提取到 BuildConfig 字段）

### P2（低优先级）

7. PRD/架构文档同步更新
8. MainActivity 拆分（WebSocket Manager、加密 Helper 独立）
