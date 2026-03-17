# 落笔 Nextype — Android 端产品需求文档

## 产品定位与目标

落笔 Nextype Android 端是一款手机端文本输入中转工具。核心场景是：用户在手机上通过语音或键盘输入文字，一键同步到电脑端光标所在位置，让手机替代电脑键盘。特别适合 Vibe Coding、与 AI 对话等需要快速大量输入的场景。

产品特点：
- 无需登录，不收集个人信息
- 通过公网中继服务器（WebSocket）实现跨网络连接
- 内容传输使用 AES 加密，服务器不保留任何传输内容
- 支持多设备配对与自动切换
- 后台零耗电（进入后台即断开所有连接）

---

## 核心功能清单

### 1. 文本输入与发送

- **输入框**：全屏多行文本输入区域，支持自定义字号（5档：16/18/20/24/28sp）
- **发送（paste-enter）**：将输入内容加密后通过 WebSocket 中继发送到 PC 端，PC 端粘贴并按回车
- **插入（paste）**：将输入内容加密后发送到 PC 端，PC 端仅粘贴不按回车
- **清空**：清空输入框内容，保存已清空内容用于恢复
- **发送后自动清空**：发送/插入成功后自动清空输入框
- **剪贴板同步**：发送/插入时可选同步复制到手机剪贴板（分别可配置）
- **AES 加密**：使用 CryptoJS 兼容格式（EVP_BytesToKey + AES/CBC/PKCS5Padding），密钥为 Android 端 deviceId

### 2. 上滑重发 / 上滑恢复

- **上滑重发**：在发送/插入按钮上按住并上滑，可重新发送上次已发送的内容（输入框为空时可用）
- **上滑恢复**：在清空按钮上按住并上滑，可恢复上次清空的内容（输入框为空时可用）
- **交互细节**：
  - 手指越过按钮上边缘时显示蓝色选中态弹窗 + 触觉反馈
  - 手指回到按钮下方时弹窗变为灰色未选中态
  - 松手时根据选中状态决定是否执行操作
  - 上滑过程中在输入框区域显示灰色预览文本
- **上滑操作提示**：首次使用发送/插入按钮时，显示一次性气泡提示"按住按钮向上滑动，可快速重复操作"

### 3. 设备配对（4位配对码 + 公网中继）

- **配对流程**：用户在 PC 端生成 4 位数字配对码 → 手机端输入配对码 → 通过中继服务器验证 → 配对成功
- **配对码输入**：4 个独立输入框，每输入 1 位自动跳转下一格，支持删除键回退，禁用自动填充/验证码助手
- **配对结果**：成功后保存设备信息（deviceId、deviceName、host、port、pairedAt），Toast 提示并返回
- **错误处理**：连接超时、配对码无效等场景均有明确错误提示

### 4. 多设备管理

- **设备列表**：底部弹窗（BottomSheetDialog）展示所有已配对设备，显示设备名称、在线状态、连接状态
- **设备切换**：点击设备卡片切换连接目标，断开旧连接并建立新连接
- **设备编辑**：底部弹窗编辑设备别名（最多16字符）和图标（笔记本/台式机两种）
- **取消配对**：通过更多菜单触发，弹出确认对话框，确认后通知中继服务器并删除本地记录
- **在线状态查询**：打开设备选择弹窗时，通过独立 WebSocket 连接查询所有配对设备的在线状态
- **设备状态标签**：主界面顶部显示当前设备图标、名称、连接绿点，多设备时显示下拉箭头
- **备注名同步**：编辑设备别名后通过 `set_device_alias` 消息同步到中继服务器

### 5. 自动切换策略（Follow the Light）

- **触发时机**：每次 `onResume`（前台恢复）时，通过 `discover` 消息查询在线 PC 列表
- **切换条件**：
  - 设备必须是已配对的
  - 设备闲置时间 < 120 秒
  - 选择闲置时间最短的设备（最近活跃）
- **防重复**：同一次 `onResume` 只执行一次自动切换
- **兜底机制**：注册成功后 5 秒内如果同步通道未就绪，也会触发一次自动切换

### 6. 远程控制响应

- **接收 PC 端命令**：通过 WebSocket relay 消息接收 PC 端发来的 `command` 类型指令
- **支持的指令**：
  - `send`：触发发送按钮（粘贴+回车）
  - `insert`：触发插入按钮（仅粘贴）
  - `clear`：触发清空按钮
  - `tap`：模拟点击指定坐标（需 AccessibilityService）
  - `touch_down`：长按按下（手指按住不放）
  - `touch_up`：长按释放（抬起手指）
  - `touch_heartbeat`：长按心跳（保持长按状态）
- **防抖**：500ms 内不重复执行同一命令
- **远程指令唤醒**：收到远程指令时自动唤醒变暗的屏幕

### 7. 模拟点击（AccessibilityService）

- **单击**：通过 `GestureDescription` 在指定坐标执行 50ms 点击手势
- **长按**：使用 `willContinue=true` 的 `StrokeDescription` 实现可续传长按
  - 按下后启动心跳检测（每 200ms 检查一次）
  - 1 秒内未收到心跳自动释放
  - 收到 `touch_up` 时通过 `continueStroke(willContinue=false)` 结束手势
- **降级方案**：Android 8.0 以下使用固定 800ms 长按
- **全局触摸检测**：通过 `TYPE_TOUCH_INTERACTION_START` 事件检测屏幕任意位置触摸，用于屏幕唤醒

### 8. 屏幕常亮与自动变暗

- **屏幕常亮**：通过 `FLAG_KEEP_SCREEN_ON` 保持屏幕不灭（默认开启）
- **自动变暗**：闲置指定时间后将屏幕亮度降至最低（0.01f），300ms 平滑动画过渡
- **变暗等待时间**：可选 30秒 / 1分钟 / 5分钟 / 10分钟（默认 1 分钟）
- **唤醒触发源**：
  - Activity 触摸事件（`dispatchTouchEvent`）
  - 按键事件（`dispatchKeyEvent`）
  - 软键盘输入（`WakeUpEditText` 的 `InputConnection` 包装）
  - AccessibilityService 全局触摸检测
  - 输入框内容变化（TextWatcher）
  - 远程控制指令
  - 窗口焦点变化（`onWindowFocusChanged`）
  - 语音录音开始
- **防误触**：变暗状态下第一次触摸只唤醒屏幕，不触发按钮点击
- **录音保护**：录音期间暂停变暗倒计时，录音结束后恢复

### 9. 屏幕参数上报（折叠屏自适应）

- **握手上报**：连接建立后发送 `device_info` 消息，包含 screenWidth 和 screenHeight
- **变更上报**：`onConfigurationChanged` 检测到屏幕尺寸变化时发送 `screen_changed` 消息
- **旋转锁定策略**：
  - 屏幕宽高比 < 0.65（极窄屏幕/折叠态竖屏）：锁定竖屏
  - 其他情况：允许随传感器自动旋转（USER 模式）
- **挖孔屏适配**：
  - 标准 API：`LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS`
  - 华为/荣耀私有 API：通过元反射设置 `hwFlags` 的 `FLAG_NOTCH_SUPPORT`

### 10. 设置页

- **惯用手**：左手/右手模式，控制发送按钮在左侧还是右侧
- **输入框字号**：5 档滑动条（小/标准/中/大/特大），实时预览
- **剪贴板同步**：插入时同步到剪贴板 / 发送时同步到剪贴板（分别开关）
- **屏幕常亮**：主开关 + 自动变暗子开关 + 变暗等待时间选择
- **辅助功能**：显示当前开启状态，点击跳转系统辅助功能设置页
- **使用说明**：跳转到 AboutActivity

### 11. 设备恢复（卸载重装后恢复配对）

- **触发条件**：EmptyStateActivity 启动时，本地无配对设备
- **恢复流程**：连接中继服务器 → 查询信任列表（`sync_trust_list`）→ 如果服务器有配对记录则恢复到本地 → 自动跳转主界面
- **依赖条件**：设备 ID 基于 ANDROID_ID 生成，卸载重装后保持不变

### 12. 后台零耗电

- **进入后台（onStop）**：断开所有 WebSocket 连接、停止心跳、停止连接监控、停止变暗倒计时
- **恢复前台（onResume）**：重建 WebSocket 连接、启动连接监控、重置变暗倒计时、触发自动切换
- **断线重连**：指数退避策略（2s → 4s → 8s → ... → 30s），最多重试 10 次

### 13. 辅助功能权限提示

- **触发条件**：PC 端发来模拟点击/长按指令但 AccessibilityService 未启用
- **提示方式**：底部气泡提示"辅助控制权限未开启，点击前往设置"
- **防抖**：10 秒内不重复弹出
- **自动消失**：5 秒后自动隐藏
- **点击跳转**：点击气泡跳转到系统辅助功能设置页
- **PC 端通知**：同时向 PC 端发送 `error` 类型消息，告知权限未开启

### 14. 欢迎页（EmptyStateActivity）

- **展示内容**：App Logo、欢迎语、配对电脑按钮、获取电脑端应用（域名复制）、稍后配对链接
- **自动跳转**：`onResume` 时检查是否已有配对设备，有则自动跳转主界面
- **设备恢复**：启动时自动尝试从服务器恢复配对信息

### 15. 使用说明页（AboutActivity）

- **展示内容**：使用场景介绍、配对步骤说明（3步）、插入/发送功能说明、隐私保护声明
- **交互**：域名复制、配对电脑按钮

### 16. 键盘切换

- **功能**：语音按钮左侧的键盘图标，点击可显示/隐藏软键盘
- **状态同步**：根据键盘实际显示状态切换图标（键盘/收起键盘）
- **Placeholder 联动**：键盘显示时隐藏引导文字，键盘隐藏且输入框为空时显示引导文字

### 17. 渐变色引导文字

- **位置**：输入框居中显示"点击空白处开始输入"
- **样式**：使用 `LinearGradient` 实现从橙色（#FFCBA4）到紫色（#E8A4FF）的渐变效果

---

## 用户流程

### 首次使用流程

1. 启动 App → 进入 EmptyStateActivity（欢迎页）
2. 后台自动尝试从服务器恢复配对信息（基于稳定的设备 ID）
3. 如果恢复成功 → 自动跳转 MainActivity
4. 如果无法恢复 → 用户点击"配对电脑" → 进入 PairingActivity
5. 输入 PC 端显示的 4 位配对码 → 通过中继服务器验证
6. 配对成功 → 返回 EmptyStateActivity → onResume 检测到配对设备 → 跳转 MainActivity
7. 进入主界面，自动弹出键盘，开始输入

### 日常使用流程

1. 启动 App → EmptyStateActivity 检测到已有配对设备 → 自动跳转 MainActivity
2. 自动连接中继服务器 → 注册设备 → 发送上线通知给 PC → 上报屏幕参数
3. 多设备场景：自动切换到最近活跃的 PC（Follow the Light）
4. 用户输入文字 → 点击"发送"或"插入" → 内容加密后通过中继发送到 PC
5. 输入框自动清空，内容保存用于上滑重发
6. 进入后台 → 断开所有连接（零耗电）
7. 回到前台 → 重建连接，恢复工作状态

### 远程控制流程

1. PC 端发送 `command` 类型 relay 消息
2. Android 端收到后根据 action 执行对应操作
3. 模拟点击/长按需要 AccessibilityService 权限
4. 权限未开启时显示气泡提示并通知 PC 端

---

## 各功能详细交互说明

### 发送/插入按钮交互

- 普通点击：执行发送/插入操作 + 触觉反馈
- 按住上滑（输入框为空且有上次内容时）：
  - 越过按钮上边缘 → 显示蓝色弹窗 + 预览文本 + 触觉反馈
  - 回到按钮下方 → 弹窗变灰 + 隐藏预览
  - 松手时选中态 → 执行重发
  - 松手时未选中态 → 取消操作

### 清空按钮交互

- 普通点击：清空输入框 + 保存内容用于恢复 + 触觉反馈
- 按住上滑（输入框为空且有上次清空内容时）：
  - 交互逻辑同发送按钮，但执行的是恢复操作
  - 弹窗显示"上滑恢复"+ 撤销图标

### 设备状态标签交互

- 无配对设备：蓝色填充背景 + 白色图标文字，点击跳转欢迎页
- 有配对设备：灰色背景 + 绿点状态指示，点击打开设备选择弹窗
- 多设备时显示下拉箭头

---

## 废弃与未使用代码

- **[已注释]** `SettingsActivity.kt:49-52` — 设备管理相关 UI 组件（`pairedDevicesLabel`、`devicesListContainer`、`addDeviceButton`），已移至首页弹层
- **[已注释]** `SettingsActivity.kt:168-171` — 设备管理 UI 初始化代码
- **[已注释]** `SettingsActivity.kt:269-273` — 添加新设备按钮点击事件
- **[已注释]** `SettingsActivity.kt:395-460` — `loadPairedDevices` 方法（设备列表渲染逻辑），已移至首页弹层
- **[已注释]** `SettingsActivity.kt:596` — `loadPairedDevices` 调用
- **[已注释]** `SettingsActivity.kt:598` — `syncServerPairingStatus` 调用
- **[未调用]** `SettingsActivity.kt:462-471` — `getIconResourceForType` 方法，仅在已注释的 `loadPairedDevices` 中使用
- **[未调用]** `SettingsActivity.kt:476-548` — `showEditDeviceDialog` 方法，仅在已注释的 `loadPairedDevices` 中使用
- **[未调用]** `SettingsActivity.kt:554-587` — `sendUnpairNotification` 方法，仅在已注释的 `loadPairedDevices` 中使用
- **[未调用]** `SettingsActivity.kt:605-646` — `syncServerPairingStatus` 方法，调用已被注释
- **[废弃标注]** `MainActivity.kt:721-723` — `connectControlChannel` 方法，注释标注"已废弃，完全由 NextypeApplication 托管"，方法体为空
- **[残留代码]** `MainActivity.kt:631-640` — `connectToServer` 方法，内部直接调用 `connectToRelay`，是早期局域网连接的残留封装
- **[未调用]** `PairedDevice.kt:53-59` — `AVAILABLE_ICONS` 列表定义了 6 种图标（laptop/desktop/imac/macmini/monitor/server），但编辑弹窗实际只使用了 laptop 和 desktop 两种
- **[残留代码]** `DeviceDiscoveryService.kt` — 整个类（mDNS + UDP 局域网发现服务），当前配对流程已完全使用公网中继，此类仅被 `DeviceListActivity` 使用
- **[残留代码]** `DeviceListActivity.kt` — 整个类（局域网设备列表页面），当前流程不再使用局域网发现，此 Activity 未在任何地方被启动（AndroidManifest 中有声明但无 intent-filter）
- **[残留代码]** `UDPPairingClient.kt` — 整个类（UDP 广播配对客户端），当前配对已完全使用公网中继（RelayClient）
- **[残留代码]** `HTTPPairingClient.kt` — 整个类（HTTP 配对客户端），当前配对已完全使用公网中继（RelayClient）
- **[残留代码]** `activity_device_list.xml` — 局域网设备列表页面布局，对应已废弃的 `DeviceListActivity`
- **[未使用依赖]** `build.gradle.kts:39` — `constraintlayout:2.1.4`，项目中所有布局均使用 LinearLayout/FrameLayout，未使用 ConstraintLayout
