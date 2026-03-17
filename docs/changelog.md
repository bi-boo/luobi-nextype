# 修改记录（已归档，不再更新）

> 本文档记录了 2026-02-15 至 2026-02-17 期间的修改历史，已停止维护。后续变更请参考各端的 PRD 和架构文档中的「当前状态」章节。

# [2026-02-17 03:00]
- **用户需求/反馈**: 上一轮修复后问题依然存在——手机和 PC 连接后长期无操作，PC 端仍会显示手机掉线。手机进入设置页或退到后台再回前台后又能恢复连接。
- **技术逻辑变更**:
    - 发现根本原因：PC 端的 `client_last_seen` 仅在设备首次注册（ClientOnline）时更新一次，之后永远不更新。Android 每 30 秒发送的心跳仅到达服务器，PC 端完全不知道手机还活着，120 秒后必然判定离线
    - 中继服务器新增逻辑：收到 Android 端心跳后，向所有 PC 端广播 `client_heartbeat` 消息（含 clientId）
    - PC 端（Tauri）新增 `ClientHeartbeat` 消息类型处理：收到后更新对应设备的 `client_last_seen` 时间戳，防止超时误判
- **涉及文件清单**:
    - `relay-server/server.js`
    - `NextypeTauri/nextype-tauri/src-tauri/src/services/relay_client.rs`
    - `docs/changelog.md`
    - `docs/architecture.md`
- **变更原因**:
    - 此前心跳信息流是单向的（Android → 服务器），PC 端完全无法感知手机的心跳活动
    - 修复后形成完整的心跳链路：Android → 服务器 → PC 端，PC 端每 30 秒刷新一次 `client_last_seen`，彻底解决误判离线问题

# [2026-02-17 02:30]
- **用户需求/反馈**: PC 端和安卓端的连接过一会就会显示断开，用户体验不佳。经排查发现：
    1. Android 端心跳间隔 60 秒，但中继服务器超时断开阈值仅 90 秒（30秒*3），容错时间太短
    2. 服务器超时断开使用 `ws.terminate()` 暴力关闭，导致 Android 端需要等待 TCP 超时才能检测到断开，重连不及时
    3. 轻微的网络抖动或延迟就可能导致心跳超时被误判断开
- **技术逻辑变更**:
    - Android 端心跳间隔从 60 秒优化为 30 秒，与服务器的 `HEARTBEAT_INTERVAL` 保持一致
    - OkHttp TCP ping 间隔同步调整为 30 秒
    - 中继服务器超时阈值从 90 秒（30秒*3）调整为 120 秒（30秒*4），增加 33% 的容错时间
    - 服务器超时断开从 `ws.terminate()` 改为 `ws.close(1000, 'Heartbeat timeout')`，优雅关闭触发客户端立即重连
- **涉及文件清单**:
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `relay-server/server.js`
    - `docs/changelog.md`
    - `docs/architecture.md`
- **变更原因**:
    - 30 秒心跳相比 60 秒每天仅增加约 0.5-1% 的耗电（20-40 mAh），对用户影响可忽略
    - 优雅关闭能让 Android 端立即感知断连并触发重连机制，而 terminate() 需要等待 TCP 超时（30-120秒）
    - 120 秒超时阈值给予心跳 4 倍的容错时间，即使网络延迟 30-40 秒也不会误判断开
    - 预期收益：连接稳定性大幅提升，断线后 2-4 秒内自动重连，用户基本无感知

# [2026-02-16 22:00]
- **用户需求/反馈**: 当前使用三条WebSocket连接（数据通道、同步通道、控制通道），导致资源占用高、心跳频繁、耗电量大。用户咨询是否应该改用HTTP协议。
- **技术逻辑变更**:
    - 合并三条WebSocket连接为一条统一连接（dataWebSocket）
    - 移除 `MainActivity` 中的 `relayClient` 实例和 `controlWebSocket`
    - 在 `dataWebSocket` 的消息处理中增加对 `server_list`、`server_online`、`server_offline` 等消息的支持
    - 新增 `discoverOnlineDevices` 函数，直接通过 dataWebSocket 发送 discover 请求
    - 简化 `NextypeApplication`，移除独立的 controlWebSocket，保留全局解除配对处理逻辑
    - 优化心跳频率：应用层心跳从20秒调整为60秒，TCP ping保持60秒
    - 保留 `RelayClient` 类用于 `PairingActivity` 的配对流程
- **涉及文件清单**:
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `NextypeAndroid/app/src/main/java/com/nextype/android/NextypeApplication.kt`
    - `docs/architecture.md`
    - `docs/prd.md`
    - `docs/changelog.md`
- **变更原因**:
    - 虽然消息频率不高（非实时同步），但需要PC→手机的远程控制主动推送能力，WebSocket仍是最佳选择
    - 通过合并连接和优化心跳，在保持功能不变的前提下大幅降低资源占用
    - 预期收益：资源占用减少66%，心跳消息减少83%，耗电量降低约50%，连接更稳定，启动速度快2倍

# [2026-02-16 16:20]
- **用户需求/反馈**: 开启屏幕常亮+定时变暗后，屏幕变暗时任何操作都无法唤醒：
    1. 在软键盘上按键（拼音阶段）不会唤醒，只有输入文字进入输入框时才唤醒
    2. 在折叠屏的悬浮窗口中操作也不会唤醒
    用户希望屏幕上任何触摸操作都能立即唤醒屏幕。
- **技术逻辑变更**:
    - 修改 `accessibility_service_config.xml`，添加 `typeTouchInteractionStart` 事件监听，使 AccessibilityService 能捕获系统级全局触摸事件。
    - 在 `NextypeAccessibilityService` 的 `onAccessibilityEvent` 中监听 `TYPE_TOUCH_INTERACTION_START`，检测到屏幕任意位置的触摸（包括键盘区域、悬浮窗口）时通过主线程 Handler 触发回调。
    - 在 `MainActivity` 中注册全局触摸回调 `onScreenTouched`，触摸时唤醒屏幕或重置倒计时；在 `onDestroy` 时清理回调避免内存泄漏。
    - 在 `MainActivity` 中重写 `onWindowFocusChanged`，当窗口失去焦点（用户切到悬浮窗）时唤醒屏幕并暂停倒计时，重新获得焦点时重启倒计时。
    - 新建 `WakeUpEditText` 自定义控件（通过 Handler 切到主线程执行回调），作为补充方案处理部分输入法的选词确认。
- **涉及文件清单**:
    - `NextypeAndroid/app/src/main/res/xml/accessibility_service_config.xml`
    - `NextypeAndroid/app/src/main/java/com/nextype/android/NextypeAccessibilityService.kt`
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `NextypeAndroid/app/src/main/java/com/nextype/android/WakeUpEditText.kt`（新建）
    - `NextypeAndroid/app/src/main/res/layout/activity_main.xml`
    - `docs/changelog.md`
- **变更原因**:
    - InputConnection 包装方案无法检测中文输入法的拼音按键阶段（输入法内部处理，不调用 InputConnection）。
    - dispatchTouchEvent 只能检测应用自己的窗口，无法检测键盘窗口和悬浮窗口。
    - AccessibilityService 的 TYPE_TOUCH_INTERACTION_START 事件是系统级全局触摸检测，能覆盖所有场景。
- **重要提示**: 因修改了 AccessibilityService 配置，需在手机设置中重新开启一次"落笔 Nextype"的辅助功能服务。

# [2026-02-15 02:00]
- **用户需求/反馈**: 点击“开机自启动”开关导致应用崩溃或退出。
- **技术逻辑变更**: 
    - 修复初始化竞态：在 `preferences.html` 中增加了配置加载状态检查，防止在初始握手未完成时触发保存操作，避免了将未定义或错误的开关值（如 `showMenuBarIcon: false`）写入系统设置。
    - UI 稳健性：将 HTML 中图标属性的默认值设置为 `checked`，确保即使在极端加载延迟下，保存逻辑也不会误隐藏应用入口。
    - 逻辑防护：在 `saveConfig` 函数中增加了 `currentConfig` 的空值判断卫语句。
- **涉及文件清单**: 
    - `NextypeTauri/nextype-tauri/src/preferences.html`
    - `docs/changelog.md`
- **变更原因**: 解决因前端状态同步时机不当导致的“误隐藏图标”现象（用户感知的崩溃）。

# [2026-02-15 01:20]
- **用户需求/反馈**: 解决 Tauri 版本中“连接手机”没反应、配对码不显示以及标签页切换失效等顽固 Bug。
- **技术逻辑变更**: 
    - 废除盲目延迟推送机制，建立前端主动握手（Handshake）的初始化流程。
    - 统一 Rust 端与 JS 端的事件名称映射（如 `relay:pairing_completed` 对齐 `pairing-success`）。
    - 完善配对码同步机制：在 `generate_pairing_code` 指令中自动触发中继服务器注册流程。
    - 增加 URL Hash 支持，解决冷启动时窗口切换事件丢失的问题。
- **涉及文件清单**: 
    - `src/tauri-bridge.js`
    - `src/preferences.html`
    - `src-tauri/src/commands/devices.rs`
    - `src-tauri/src/services/tray.rs`
    - `docs/changelog.md`
- **变更原因**: 之前的修复多为局部补丁，未解决初始化竞态和协议命名不一致的架构根源。此次通过全链路审计并重构通信层彻底解决。


# [2026-02-15 00:50]
- **用户需求/反馈**: 菜单栏点击“连接手机”没反应，偏好设置中无法生成配对码。第一轮修复后问题依然存在。
- **技术逻辑变更**: 
    - `preferences.html`: 重构初始化流，移除严格的环境检测，改为智能降级；增加 `ipcRenderer` 的动态获取逻辑。
    - `tauri-bridge.js`: 增强健壮性，防止在缺失 Tauri API 时抛出严重异常，确保前端 UI 逻辑不中断。
- **涉及文件清单**: 
    - `NextypeTauri/nextype-tauri/src/preferences.html`
    - `NextypeTauri/nextype-tauri/src/tauri-bridge.js`
    - `docs/prd.md`
    - `docs/changelog.md`
- **变更原因**: 解决因前端初始化逻辑在 API 未就绪时提前退出导致的事件绑定失效。

# [2026-02-15 00:25]
- **用户需求/反馈**: 补全 Tauri 版本的后端核心逻辑，使其在功能上与 Electron 版本完全对齐（剪贴板后缀、自动清空、快捷键指令转发）。
- **技术逻辑变更**: 
    - `services/clipboard.rs`: 增加配置感知，支持 `btn1_suffix` / `btn2_suffix` 和 `clear_after_paste`（200ms 延迟清空）。
    - `services/hotkey_manager.rs`: 实现 `match_coordinates` 坐标匹配算法（比例容错），并在快捷键触发时直接通过 `RelayClient` 向在线设备分发指令（Send/Insert/Clear/Tap）。
    - `services/relay_client.rs`: 增加 `DeviceSession` 存储结构，自动解析并存储 `device_info` 指令；并同步在线设备状态至 `AppState` 以便 UI 消费。
    - `services/tray.rs`: 修正托盘图标 Drop 问题，将 `TrayIcon` 句柄通过 `AppState` 持久化，并优化了 macOS 下的菜单刷新机制。
    - `lib.rs`: 建立后端全局事件监听器，实现全自动化文字同步处理（无需前端 bridge 干预核心逻辑）。
- **涉及文件清单**: 
    - `NextypeTauri/nextype-tauri/src-tauri/src/services/clipboard.rs`
    - `NextypeTauri/nextype-tauri/src-tauri/src/services/hotkey_manager.rs`
    - `NextypeTauri/nextype-tauri/src-tauri/src/services/relay_client.rs`
    - `NextypeTauri/nextype-tauri/src-tauri/src/lib.rs`
    - `docs/changelog.md`
- **变更原因**: 确保 Tauri 版本在核心同步体验和远程控制功能上与成熟的 Electron 版本保持 100% 对齐，提升后台运行稳定性。

# [2026-02-13 12:55]
- **用户需求/反馈**: 取消屏幕从变暗状态恢复时的渐变延时，希望亮度恢复能直接、顺滑（瞬间亮屏）。
- **技术逻辑变更**: 
    - `MainActivity.kt`：重构 `wakeUpScreen()` 函数，移除原有的 `ValueAnimator` 渐变逻辑。改为直接将 `window.attributes.screenBrightness` 设置为 `-1.0f`（恢复系统默认），实现亮度瞬时恢复。保留了进入变暗状态时的 300ms 平滑渐变。
- **涉及文件清单**: 
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `docs/prd.md`
    - `docs/architecture.md`
    - `docs/changelog.md`
- **变更原因**: 提升响应感。用户认为变暗过程可以缓慢渐变以减少分心，但恢复亮屏应该“干脆利落”，无动画延时使用起来更顺滑。

# [2026-02-13 02:05]
- **用户需求/反馈**: 打包 PC 端新版本，要求按照签名规则进行手动签名和清除隔离属性。
- **技术逻辑变更**: 
    - 环境修复：因 `electron-builder` 自动签名检索 Keychain 失败，切换为手动签名策略。
    - 构建流程：使用 `csc_link=null` 进行纯构建，生成 `.app` 后手动执行 `codesign`（指定 `Developer ID Application` 证书）和 `xattr -cr`。
    - 重新打包：使用 `prepackaged` 参数重新生成带签名的 DMG，并对 DMG 本身进行手动签名。
- **涉及文件清单**: 
    - `Nextype.dmg` (根目录)
    - `docs/changelog.md`
- **变更原因**: 确保发布的 PC 端安装包在 Mac 系统下具有合法签名，避免“已损坏”或“无法打开”的系统拦截。

# [2026-02-13 00:53]
- **用户需求/反馈**: 荣耀折叠屏展开态横屏时，左侧因挖孔屏让出约 105 像素空白区域，应用没有撑满全屏。用户希望像视频应用一样忽略挖孔区域，强制撑满。
- **技术逻辑变更**: 
  - `MainActivity.kt`：`onCreate` 中新增 `LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES` 设置，让应用内容延伸到短边方向的挖孔屏区域。`SHORT_EDGES` 模式比之前的 `ALWAYS` 更精确，仅在横屏时让内容覆盖挖孔区域。
- **涉及文件清单**: 
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `docs/changelog.md`
- **变更原因**: Android 默认的 cutout 模式在横屏时会避开挖孔区域，导致左侧出现空白。设置 `SHORT_EDGES` 模式后，应用忽略挖孔区域铺满全屏，与视频应用行为一致。

# [2026-02-13 00:29]
- **用户需求/反馈**: PC 端按模拟点击快捷键但手机辅助控制权限已关闭时，手机端没有任何提示，用户只能通过日志才能发现问题
- **技术逻辑变更**: 
  - `MainActivity.kt`：新增 `showAccessibilityHintToast()` 和 `hideAccessibilityHintToast()` 方法，复用 `toast_swipe_hint.xml` 气泡样式。在 `performSimulatedTap()` 的权限缺失分支中调用，显示"⚠️ 辅助控制权限未开启，点击前往设置"的可点击气泡。5 秒自动消失，10 秒内防重复弹出。
- **涉及文件清单**: 
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `docs/prd.md`
    - `docs/architecture.md`
    - `docs/changelog.md`
- **变更原因**: 提升用户体验，将辅助控制权限丢失的提示从日志级别提升为可视化气泡提示，减少用户排查问题的成本

# [2026-02-13 00:14]
- **用户需求/反馈**: 手机端切换到另一台电脑后，从后台回到前台时没有自动切换到当前活跃的 PC
- **技术逻辑变更**: 
  - `MainActivity.kt`：将 `onResume` 热启动中的固定延迟 1.5 秒改为使用 `relayClient.connect()` 的连接成功回调触发自动切换。新增 `hasAutoSwitchedThisResume` 防重复标志位，配合 5 秒超时兜底机制，确保自动切换只执行一次且不遗漏。数据通道和监控不再等待同步通道，立即启动。
- **涉及文件清单**: 
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `docs/changelog.md`
    - `docs/prd.md`
- **变更原因**: 后台全断方案下，同步通道重连需要完成 TCP→SSL→WebSocket→注册 全流程（2-3秒），固定等1.5秒后调用 `discoverOnlineDevices` 时通道通常还没连上，导致返回空列表、自动切换不执行。

# [2026-02-12 23:57]
- **用户需求/反馈**: PC 端日志显示重复的"辅助通道就绪"和"tap 指令发送"消息，且明明只连了一台手机却显示"向3台设备发送"。
- **技术逻辑变更**: 
  - `relay-client.js`：`client_online` 处理中，辅助通道 ID（含 `_sync`/`_ctrl` 后缀）不再加入 `onlineClients` Set，只记录调试日志。
- **涉及文件清单**: 
    - `electron-app/src/server/relay-client.js`
    - `docs/changelog.md`
- **变更原因**: 手机连接中继时注册 3 个通道 ID（主通道 + `_sync` + `_ctrl`），全部被计入在线设备导致计数错误和指令重复发送。

# [2026-02-12 23:48]
- **用户需求/反馈**: 系统提示"后台频繁刷新，耗电过快"，因为应用在后台维持了 3 条 WebSocket 连接和多个心跳定时器，约每 15 秒产生一次网络活动。
- **技术逻辑变更**: 
  - `MainActivity.kt`：`onStop()` 中增加全断逻辑，断开数据 WebSocket、同步通道（RelayClient）、控制通道（NextypeApplication），停止心跳。`onResume()` 简化为统一重连模式：热启动时按顺序重建同步通道→控制通道→数据通道。
  - `NextypeApplication.kt`：`onFailure` 和 `onClosed` 中去除 5 秒自动重连循环，改为等待前台时由 MainActivity 触发重连。
- **涉及文件清单**: 
    - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
    - `NextypeAndroid/app/src/main/java/com/nextype/android/NextypeApplication.kt`
    - `docs/prd.md`
    - `docs/architecture.md`
    - `docs/changelog.md`
- **变更原因**: 落笔是"开着用、用完放"的工具，后台无业务价值。后台全断可实现零耗电，1-2 秒的前台重连延迟完全可接受。

# [2026-02-12 21:59]
- **用户需求/反馈**: 日志窗口在同一秒收到大量相似消息，内容冗余，且 ping 消息刷屏。
- **技术逻辑变更**: 
  - `RelayClient.js`: 移除 `handleMessage` 顶层的“收到通知”通用日志；移除 `relay` 类型的初步动作日志；在解析细节内心跳包时过滤 `ping` 消息记录。
  - `RelayClient.js`: 为 `default` 消息类型增加通知日志补丁，确保排查未知命令的能力。
- **涉及文件清单**: 
    - `electron-app/src/server/relay-client.js`
    - `docs/prd.md`
    - `docs/changelog.md`
- **变更原因**: 提升日志窗口业务信息的密度和可读性，消除高频心跳包带来的视觉干扰。

# [2026-02-12 21:55]
- **用户需求/反馈**: 彻底移除本地 HTTP 服务器及 Web H5 输入页面功能。
- **技术逻辑变更**: 
  - `electron-app/main.js`: 剥离 `HTTPServer` 模块，重构启动流，不再监听 9020 端口。
  - `src/server/http.js`: 物理删除该模块源码。
  - `package.json`: 卸载 `express` 及其关联依赖。
- **涉及文件清单**: 
  - `electron-app/main.js`
  - `electron-app/src/server/http.js`
  - `electron-app/package.json`
- **变更原因**: 跨端连接已全面转向更稳定的专用 App 架构，局域网 H5 页面已无实际使用场景，移除它可以精简系统资源并减少启动噪音。

# [2026-02-12 21:40]
- **用户需求/反馈**: Android 端配对信息丢失，且重新配对后瞬间失效。
- **技术逻辑变更**: 
  - `relay-server/server.js`: 在 `sync_trust_list`、`verify_code` 和 `unpair_device` 逻辑中增加 `deviceId` 后缀剥离处理（`.split('_')[0]`）。
- **涉及文件清单**: 
  - `relay-server/server.js`
- **变更原因**: 解决因业务通道 ID 后缀（`_sync`）导致中继服务器无法在数据库中正确查询/存入配对关系的问题。确保 ID 后缀仅用于连接隔离，不影响业务配对逻辑。

# [2026-02-12 21:03]
- **用户需求/反馈**: 修复 Android 端连接波动大且 PC 端日志刷屏的问题。
- **技术逻辑变更**: 
  - `relay-server/server.js`: 增加 WebSocket 实例校验逻辑。
  - `RelayClient.kt`: 同步通道 ID 增加 `_sync` 后缀，实现 ID 隔离。
  - `electron-app`: **深度精简日志**。静默内部通道相关记录，将非业务核心日志全部降级为 `debug`。
  - `MainActivity.kt`: **降低监控频率**。将 5s 次的连接检查放宽至 60s，减少因冗余检查引起的连接波动。
- **涉及文件清单**: 
  - `relay-server/server.js`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/RelayClient.kt`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `electron-app/src/server/relay-client.js`
  - `electron-app/main.js`
- **变更原因**: 解决由于多重连接冲突带来的系统噪音，并通过降低日志与检查频率提升整体连接的“安静感”与稳定性。

# [2026-02-12 21:00]
- **用户需求/反馈**: 实现多端连接自动切换策略，解决用户在多台 PC 间切换使用时手动点击连接的繁琐。
- **技术逻辑变更**: 
  - `electron-app/src/server/relay-client.js`: 引入 `powerMonitor`，心跳包包含系统闲置时间。
  - `relay-server/server.js`: 存储并下发在线设备的 `idleTime`。
  - `RelayClient.kt`: 扩展 `OnlineDeviceInfo` 模型，解析服务器下发的闲置数据。
  - `MainActivity.kt`: 实现 `autoSwitchToActiveDevice()` 决策方法，集成到 `onResume`；判定算法：闲置 < 120s 且闲置最小者胜。
- **涉及文件清单**: 
  - `electron-app/src/server/relay-client.js`
  - `relay-server/server.js`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/RelayClient.kt`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 提升多设备使用环境下的操作流畅度，实现“随人而动”的无感连接。同时修复了服务器初始 idleTime 设为 0 导致的误切换 Bug。

# [2026-02-12 19:42]
- **用户需求/反馈**: 实现智能常亮与自动变暗功能——手机屏幕保持常亮不锁屏，闲置一段时间后自动变暗，触摸或远程指令唤醒
- **技术逻辑变更**: 
  - `SettingsActivity.kt`：新增 `keepScreenOn`/`autoDimEnabled`/`autoDimTimeout` 三个 SharedPreferences 键，新增静态读取方法 `getKeepScreenOn()`/`getAutoDimEnabled()`/`getAutoDimTimeout()`，新增 UI 初始化逻辑和联动控制（主开关关闭时子项灰显），新增 `showDimTimeoutPicker()` 弹窗选择器
  - `activity_settings.xml`：在剪贴板设置卡片后新增"屏幕常亮"设置卡片组（SwitchCompat 主开关 + SwitchCompat 子开关 + 等待时间选择行）
  - `MainActivity.kt`：新增 `isDimmed`/`dimHandler`/`dimRunnable`/`brightnessAnimator` 成员变量；新增 `loadScreenSettings()`/`applyKeepScreenOn()`/`resetDimTimer()`/`stopDimTimer()`/`dimScreen()`/`wakeUpScreen()` 方法；重写 `dispatchTouchEvent()` 防误触唤醒；`handleRemoteCommand()` 入口增加唤醒逻辑；优化 `TextWatcher` 文本变化监听和 `volcanoASR.isRecording` 监听，实现打字或语音输入时的内容感知唤醒，以及录音期间暂停倒计时逻辑；集成了生命周期管理。
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/java/com/nextype/android/SettingsActivity.kt`
  - `NextypeAndroid/app/src/main/res/layout/activity_settings.xml`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 用户长时间在电脑前使用手机作为输入端，屏幕需要保持常亮但闲置时应降低亮度以保护屏幕和节省电量

# [2026-02-11 21:50]
- **用户需求/反馈**: Android 折叠屏展开态（2247x2106）显示有黑边（Letterboxing），无法全屏。
- **技术逻辑变更**: 
  - Android `themes.xml`：主题中新增 `android:windowLayoutInDisplayCutoutMode=always`，允许应用布局延伸到摄像头切口区域（**真正根因**）。
  - Android `AndroidManifest.xml`：Application 标签新增 `android:resizeableActivity="true"`、`<supports-screens>` 声明、`android.max_aspect` meta-data、MainActivity 的 `android:maxAspectRatio="99.0"`。
  - Android `MainActivity.kt`：新增 `checkScreenOrientation` 方法，`onCreate` 和 `onConfigurationChanged` 中调用；展开态方向从 `UNSPECIFIED` 改为 `USER`。
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/res/values/themes.xml`
  - `NextypeAndroid/app/src/main/AndroidManifest.xml`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
- **变更原因**: ADB 诊断发现 `letterboxReason=DISPLAY_CUTOUT`，荣耀折叠屏展开态内屏的摄像头挖孔导致系统默认将应用窗口缩小 105px 以避开切口区域。设置 `windowLayoutInDisplayCutoutMode=always` 后，应用窗口从 `Rect(105,0)` 恢复为 `Rect(0,0)` 铺满全屏。

# [2026-02-11 21:20]
- **用户需求/反馈**: 手机折叠屏切换状态时，展开和折叠有两种坐标，但横屏或分屏时由于尺寸变化会导致坐标失效。同时希望折叠态（极窄屏幕）能锁定竖屏，不要旋转。解决 Java 环境识别不稳问题。
- **技术逻辑变更**: 
  - Android `MainActivity.kt`：在 `onConfigurationChanged` 中实现基于宽高比的动态旋转控制逻辑（ratio < 0.65 锁定 PORTRAIT，否则解锁）；强化编译脚本探测。
  - Android `build-and-install.sh`：固化 Java 探测逻辑，支持 Homebrew OpenJDK 17 和 JBR 等常见路径。
  - PC `preferences.html`：坐标配置从静态 2 行重构为动态数组列表；支持 `device-screen-info` 自动捕获新尺寸并通知用户。
  - PC `hotkey-manager.js`：`_matchCoordinates` 改为遍历配置数组，实现精确尺寸匹配优先 + 比例偏差模糊匹配容错。
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `NextypeAndroid/build-and-install.sh`
  - `electron-app/assets/preferences.html`
  - `electron-app/src/automation/hotkey-manager.js`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 提升折叠屏在不同形态（横横屏/竖屏/分屏）下的兼容性，防止误触，并解决开发者环境配置稳定性问题。

# [2026-02-11 20:40]
- **用户需求/反馈**: 手机屏幕尺寸需要手动输入太麻烦，希望手机连接时自动上报；折叠屏变形态后坐标要自动更新
- **技术逻辑变更**: 
  - Android `MainActivity.kt`：注册成功后发送 `device_info` 消息上报屏幕宽高；新增 `onConfigurationChanged` 监听折叠屏变化发送 `screen_changed`；简化 `handleRemoteTap` 优先读取 PC 端预匹配的直接坐标
  - Android `AndroidManifest.xml`：MainActivity 添加 `configChanges` 属性支持折叠屏
  - PC `relay-client.js`：新增 `deviceSessions` Map 存储设备屏幕参数；relay 消息中解析 `device_info`/`screen_changed`；设备下线时清理会话
  - PC `hotkey-manager.js`：tap 指令改为按设备逐个查 deviceSession 匹配坐标后发送；新增 `_matchCoordinates` 方法
  - PC `main.js`：两处回调设置添加 `onDeviceScreenChanged`，通知 preferences 窗口
  - PC `preferences.html`：添加 `device-screen-info` IPC 监听，自动回填屏幕尺寸
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/AndroidManifest.xml`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `electron-app/src/server/relay-client.js`
  - `electron-app/src/automation/hotkey-manager.js`
  - `electron-app/main.js`
  - `electron-app/assets/preferences.html`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 消除手动输入屏幕分辨率的繁琐流程，实现设备屏幕参数自动感知和坐标精准匹配

# [2026-02-11 11:50]
- **用户需求/反馈**: 1) PC 端启动弹窗通知没必要，直接不弹了；2) Android 辅助功能会自动失效，希望在设置页加一个快捷跳转入口，带状态检测
- **技术逻辑变更**: 
  - PC 端 `main.js`：移除正常启动时的 `Notification`，仅保留重试恢复时的通知
  - Android `activity_settings.xml`：剪贴板设置和使用说明之间新增辅助功能入口卡片
  - Android `SettingsActivity.kt`：添加 `isAccessibilityServiceEnabled()` 通过系统 API 检测服务状态，`onResume` 时自动刷新状态文字（已开启/未开启）
- **涉及文件清单**: 
  - `electron-app/main.js`
  - `NextypeAndroid/app/src/main/res/layout/activity_settings.xml`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/SettingsActivity.kt`
- **变更原因**: 提升用户体验，减少不必要的系统通知，方便用户快速管理辅助功能权限

# [2026-02-11 10:20]
- **用户需求/反馈**: PC 端启动时还会弹出"服务器已启动 手机访问 http://192.168.x.x:9018"的局域网通知
- **技术逻辑变更**: 
  - `startServers()` 启动通知文案从局域网 IP 改为"落笔 Nextype 已启动 公网中继已连接"
  - 移除 `enableRemoteConnection` 配置判断残留（配对码注册不再需要此条件）
- **涉及文件清单**: 
  - `electron-app/main.js`
- **变更原因**: 局域网通道已移除，启动通知文案是历史残留

# [2026-02-11 09:57]
- **用户需求/反馈**: 确认纯公网中继模式下，PC 和手机能否准确感知对方在线状态；发现手机断线后 PC 端不会显示离线
- **技术逻辑变更**: 
  - 中继服务器 `ws.on('close')`：当 `role === 'client'` 断开时，新增 `broadcastToServers({ type: 'client_offline' })` 广播
  - 中继服务器心跳超时清理：`terminate()` 之前先广播离线通知（区分 server/client 角色）
  - PC 端 `relay-client.js`：新增 `clientLastSeen` Map 和 `clientTimeoutChecker` 定时器，120秒无活动自动标记离线；断连时清空 `onlineClients` 和 `clientLastSeen`
  - Android `RelayClient.kt`：新增应用层心跳（每30秒发送 `{type:"heartbeat"}`），确保服务器 `lastHeartbeat` 被更新
- **涉及文件清单**: 
  - `relay-server/server.js`
  - `electron-app/src/server/relay-client.js`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/RelayClient.kt`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 中继服务器原本只在 PC（server 角色）断开时广播离线，手机（client 角色）断开无通知，导致 PC 端 onlineClients 只增不减，绿点永不消失

# [2026-02-11 09:30]
- **用户需求/反馈**: PC 端模拟点击时，手机屏幕快速点击两次导致操作被抵消
- **技术逻辑变更**: 
  - Android 端 `handleRemoteCommand()` 入口添加 500ms 防抖逻辑，防止同一条命令被重复执行
  - `NextypeApplication` 和 `MainActivity` 的控制通道注册 deviceId 改为 `${deviceId}_ctrl`，避免在中继服务器 Map 中覆盖数据通道的注册
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `NextypeAndroid/app/src/main/java/com/nextype/android/NextypeApplication.kt`
  - `electron-app/src/server/relay-client.js`
- **变更原因**: 控制通道和数据通道使用同一 deviceId 注册导致中继服务器 Map 互相覆盖，可能导致消息路由异常；PC 端需过滤 `_ctrl` 后缀避免向控制通道发送无效指令

# [2026-02-11 03:00]
- **用户需求/反馈**: 清理 Android 端所有局域网连接逻辑，与 PC 端保持一致，全端只走公网中继
- **技术逻辑变更**: 
  - 删除 `connectToLocal()` 函数（~100行局域网 WebSocket 连接逻辑）
  - 删除 `smartConnect()`、`findLanDevice()`、`findLanDeviceFromPaired()` 函数
  - 简化 `checkAndSwitch()` 和 `reconnect()` 移除局域网分支
  - 简化 `sendContent()`/`sendContentWithText()`/`sendErrorToPC()` 移除 `currentHost == "relay"` 判断，始终包装 relay 消息
  - 移除 `discoveryService`（DeviceDiscoveryService）和 `currentHost` 成员变量
  - 移除生命周期中的 `discoveryService.startDiscovery()/stopDiscovery()` 调用
  - 设备状态文案从"局域网连接/公网已连接"统一为"已连接"
  - 在线状态查询移除局域网补充在线检查逻辑
  - `connectToRelay()` 入口增加旧连接关闭逻辑防止双重 WebSocket
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 与 PC 端保持一致，全端统一只使用公网中继通信，简化架构避免残留局域网代码引起的潜在问题

# [2026-02-11 02:14]
- **用户需求/反馈**: 模拟点击快捷键触发了两次
- **技术逻辑变更**: 
  - 将防抖从仅 `tap` 操作（200ms）改为**所有快捷键操作统一防抖**（500ms）
  - 防抖放在 `handleHotkeyPress` 入口处，任何操作在 500ms 内不会重复触发
- **涉及文件清单**: 
  - `electron-app/src/automation/hotkey-manager.js`
- **变更原因**: macOS `globalShortcut` 在某些键盘/组合键下可能触发两次回调，统一防抖可彻底避免

# [2026-02-11 01:55]
- **用户需求/反馈**: 移除局域网后，PC 端 UI 上所有"公网""局域网"的区分展示都不再需要，只需显示"已连接/未连接"
- **技术逻辑变更**: 
  - `main.js`: 移除 `enableRemoteConnection` 残留逻辑和重复 stats 初始化；托盘菜单设备列表去掉"公网"标签和 `connectionTypes`/`displayIp` 字段；日志前缀从 `[公网]` 统一为 `[消息]`
  - `local.html`: 连接提示从"公网已连接/局域网已连接"统一为"已连接 · 请输入内容"
  - `logs.html`: 移除局域网(青色)/公网(紫色)的颜色区分，统一为消息(蓝色)标签
  - `onboarding.html`: 隐私说明从"所有数据都在您的局域网内传输"改为"所有数据均经过端到端加密传输"
- **涉及文件清单**: 
  - `electron-app/main.js`
  - `electron-app/assets/local.html`
  - `electron-app/assets/logs.html`
  - `electron-app/assets/onboarding.html`
- **变更原因**: 移除局域网通道后，连接方式只有公网中继一种，不再需要区分展示

# [2026-02-11 01:20]
- **用户需求/反馈**: 简化网络架构，移除局域网 WebSocket 通道，只保留公网中继
- **技术逻辑变更**: 
  - 从 `main.js` 中移除 WebSocketServer、BonjourService、PairingService 的全部引用和初始化逻辑
  - 重写 `hotkey-manager.js`（258→160行），移除 wsServer 依赖和双通道判断，所有指令只走公网中继
  - 简化消息处理器为直接处理剪贴板更新，移除了"转发到本地WebSocket"的中间环节
  - 移除 `enableRemoteConnection` 配置开关，公网中继改为必须项
  - 简化托盘菜单设备列表，只显示公网中继设备
  - 网络监控移除 Bonjour 重新发布逻辑
- **涉及文件清单**: 
  - `electron-app/main.js`
  - `electron-app/src/automation/hotkey-manager.js`
- **变更原因**: 双通道（局域网+公网）架构是大量稳定性问题的根源，维护成本高于延迟收益

# [2026-02-10 21:30]
- **用户需求/反馈**: 远程遥控功能使用几分钟后失灵，尤其是手机锁屏或退到后台之后，PC 端按快捷键无响应
- **技术逻辑变更**: 
  - 为 `connectToRelay()` 和 `connectToLocal()` 的 `onFailure`/`onClosed` 添加自动重连调用
  - 新增 `scheduleDataReconnect()` 方法，使用指数退避策略（2s→30s，最多 10 次）
  - 新增 `onResume()` 生命周期方法，Activity 回到前台时检查并恢复 WebSocket 连接
  - 新增 `onStop()` 生命周期方法，记录 Activity 进入后台的状态
  - 新增 `dataReconnectAttempts`/`maxDataReconnectAttempts`/`isInBackground` 成员变量
- **涉及文件清单**: 
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt`
  - `docs/prd.md`
  - `docs/architecture.md`
- **变更原因**: 数据 WebSocket 断开后没有自动重连机制，且 Activity 缺少生命周期管理，导致锁屏/退后台后连接永久丢失

# [2026-02-09 23:05]
- **用户需求/反馈**: 实现 PC 端快捷键远程控制手机端功能，包括触发发送、插入、清空按钮，以及模拟屏幕点击操作
- **技术逻辑变更**: 
  - PC 端新增 HotkeyManager 模块管理全局快捷键
  - 通过公网中继发送 command 类型消息到手机端
  - Android 端新增 relay command 消息处理逻辑
  - 新增 AccessibilityService 实现模拟点击
  - 坐标配置在 PC 端，Android 端根据屏幕尺寸自动匹配
- **涉及文件清单**: 
  - `electron-app/src/automation/hotkey-manager.js` (新增)
  - `electron-app/main.js` (修改：添加 HotkeyManager 初始化和 IPC)
  - `electron-app/assets/preferences.html` (修改：新增快捷键配置标签页)
  - `electron-app/assets/style.css` (修改：新增快捷键输入框样式)
  - `NextypeAndroid/app/src/main/java/com/nextype/android/MainActivity.kt` (修改：添加 command 消息处理)
  - `NextypeAndroid/app/src/main/java/com/nextype/android/NextypeAccessibilityService.kt` (新增)
  - `NextypeAndroid/app/src/main/res/xml/accessibility_service_config.xml` (新增)
  - `NextypeAndroid/app/src/main/res/values/strings.xml` (修改：添加服务字符串)
  - `NextypeAndroid/app/src/main/AndroidManifest.xml` (修改：注册 AccessibilityService)
- **变更原因**: 用户希望在电脑上通过快捷键快速触发手机端的输入操作，提高跨设备输入效率
