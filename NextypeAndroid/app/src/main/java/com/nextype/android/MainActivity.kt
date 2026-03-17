package com.nextype.android

import android.animation.ValueAnimator
import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.pm.ActivityInfo
import android.content.res.Configuration
import android.os.Bundle
import android.view.WindowManager
import android.util.Log
import android.view.KeyEvent
import android.view.MotionEvent
import android.util.TypedValue
import android.widget.Button
import android.widget.EditText
import android.widget.ImageButton
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.google.android.material.button.MaterialButton
import okhttp3.*
import org.json.JSONObject
import java.security.MessageDigest
import java.io.ByteArrayOutputStream
import javax.crypto.Cipher
import javax.crypto.spec.SecretKeySpec


class MainActivity : AppCompatActivity() {
    
    private lateinit var inputEditText: WakeUpEditText
    private lateinit var swipePreviewText: TextView  // 上滑预览文本
    // 设备状态标签组件
    private lateinit var deviceStatusCard: com.google.android.material.card.MaterialCardView
    private lateinit var deviceStatusIcon: android.widget.ImageView
    private lateinit var deviceStatusName: TextView
    private lateinit var deviceStatusArrow: android.widget.ImageView
    private lateinit var deviceStatusDot: android.view.View  // 连接状态绿点
    private lateinit var clearButton: Button
    private lateinit var syncButton: Button
    private lateinit var sendButton: Button
    private lateinit var buttonsContainer: android.widget.LinearLayout
    private lateinit var settingsButton: ImageButton
    // WebSocket 连接（公网中继）- 统一使用一条连接
    private var dataWebSocket: WebSocket? = null
    @Volatile private var isWebSocketConnected = false

    // 配置 OkHttpClient 启用心跳检测，每30秒发送ping
    private val client = OkHttpClient.Builder()
        .pingInterval(30, java.util.concurrent.TimeUnit.SECONDS)
        .build()
    private var deviceId: String? = null

    // 在线设备查询回调
    private var onlineDevicesCallback: ((List<OnlineDeviceInfo>) -> Unit)? = null

    // 服务器上下线回调 (deviceId, deviceName, isOnline)
    private var serverStatusCallback: ((String, String, Boolean) -> Unit)? = null

    private var deviceName: String? = null
    private lateinit var deviceManager: PairedDeviceManager
    
    // 连接管理
    private var connectionCheckTimer: java.util.Timer? = null
    private var isSmartSwitching = false  // 防止并发切换
    private var hasAutoSwitchedThisResume = false  // 防止同一次 onResume 中重复执行自动切换
    private var deviceSelectorAdapter: DeviceSelectorAdapter? = null  // 设备选择器适配器引用
    
    // 数据 WebSocket 断线重连
    private var dataReconnectAttempts = 0
    private val maxDataReconnectAttempts = 10
    private var isInBackground = false  // Activity 是否在后台
    
    // 屏幕尺寸跟踪（用于检测折叠屏状态变化）
    private var lastScreenWidth = 0
    private var lastScreenHeight = 0
    
    // 远程命令防抖
    private var lastRemoteCommandTime = 0L
    private val remoteCommandDebounceMs = 500L
    
    // 主动心跳机制
    private var heartbeatHandler: android.os.Handler? = null
    private var heartbeatRunnable: Runnable? = null

    // 重连延迟任务（可取消）
    private var reconnectHandler: android.os.Handler? = null
    private var reconnectRunnable: Runnable? = null
    
    // 剪贴板设置
    private var pasteCopiesToClipboard = true
    private var pasteEnterCopiesToClipboard = true
    
    // 屏幕常亮 & 自动变暗状态机
    private var isKeepScreenOn = false
    private var isAutoDimEnabled = true
    private var autoDimTimeoutMs = 30_000L
    private var isDimmed = false               // 当前是否处于变暗状态
    private var dimHandler: android.os.Handler? = null
    private var dimRunnable: Runnable? = null
    private var brightnessAnimator: ValueAnimator? = null
    
    // 上次发送的内容（用于上滑重发功能）
    private var lastSentContent: String? = null
    // 上次清空的内容（用于上滑恢复功能）
    private var lastClearedContent: String? = null
    private var resendPopup: android.widget.PopupWindow? = null
    
    // 上滑操作提示弹窗
    private var swipeHintPopup: android.widget.PopupWindow? = null
    private var swipeHintHandler: android.os.Handler? = null
    
    // 辅助控制权限提示弹窗
    private var accessibilityHintPopup: android.widget.PopupWindow? = null
    private var accessibilityHintHandler: android.os.Handler? = null
    private var lastAccessibilityHintTime: Long = 0
    
    companion object {
        private const val PREFS_SWIPE_HINT = "SwipeHintPrefs"
    }
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 返回键：退到后台而非销毁 Activity（保持 WebSocket 连接）
        onBackPressedDispatcher.addCallback(this, object : androidx.activity.OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                moveTaskToBack(true)
            }
        })

        // 💡 忽略挖孔屏区域，让内容撑满全屏（解决折叠屏横屏时左侧让出空间的问题）
        // 标准 Android API：设置 cutout 模式
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) {
            val lp = window.attributes
            lp.layoutInDisplayCutoutMode = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
            } else {
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            }
            window.attributes = lp
        }
        // 华为/荣耀私有 API：通过元反射设置 FLAG_NOTCH_SUPPORT (hwFlags=0x10000)
        setHuaweiNotchSupport()
        
        // Edge-to-edge：让应用内容延伸到系统栏区域（包括挖孔区域）
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        // 窗口背景色匹配应用背景，避免挖孔区域显示黑色（输入法不覆盖挖孔时左下角会露出窗口背景）
        window.decorView.setBackgroundColor(androidx.core.content.ContextCompat.getColor(this, R.color.background))
        window.navigationBarColor = androidx.core.content.ContextCompat.getColor(this, R.color.background)
        
        setContentView(R.layout.activity_main)
        
        // 手动处理 insets：顶部+底部加 padding，左右不加（让横屏时内容覆盖挖孔区域）
        // 同时处理 IME（键盘）高度，替代默认的 adjustResize 行为
        val rootView = findViewById<android.view.View>(android.R.id.content)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(rootView) { view, windowInsets ->
            val systemBarInsets = windowInsets.getInsets(
                androidx.core.view.WindowInsetsCompat.Type.systemBars() or 
                androidx.core.view.WindowInsetsCompat.Type.displayCutout()
            )
            val imeInsets = windowInsets.getInsets(
                androidx.core.view.WindowInsetsCompat.Type.ime()
            )
            // 顶部：状态栏高度；底部：取导航栏和键盘高度中的较大值
            val bottomPadding = maxOf(systemBarInsets.bottom, imeInsets.bottom)
            view.setPadding(0, systemBarInsets.top, 0, bottomPadding)
            windowInsets
        }
        
        // 初始化屏幕尺寸跟踪
        val dm = resources.displayMetrics
        lastScreenWidth = dm.widthPixels
        lastScreenHeight = dm.heightPixels
        
        // 💡 初始化屏幕旋转策略
        checkScreenOrientation(lastScreenWidth, lastScreenHeight)
        
        // 初始化视图
        inputEditText = findViewById(R.id.inputEditText)
        swipePreviewText = findViewById(R.id.swipePreviewText)  // 上滑预览文本
        // 初始化设备状态标签组件
        deviceStatusCard = findViewById(R.id.deviceStatusCard)
        deviceStatusIcon = findViewById(R.id.deviceStatusIcon)
        deviceStatusName = findViewById(R.id.deviceStatusName)
        deviceStatusArrow = findViewById(R.id.deviceStatusArrow)
        deviceStatusDot = findViewById(R.id.deviceStatusDot)
        clearButton = findViewById(R.id.clearButton)
        syncButton = findViewById(R.id.syncButton)
        sendButton = findViewById(R.id.sendButton)
        buttonsContainer = findViewById(R.id.buttonsContainer)
        settingsButton = findViewById(R.id.settingsButton)
        
        // 设置键盘活动回调：软键盘的任何输入操作都触发唤醒/重置倒计时
        inputEditText.onKeyboardActivity = {
            if (isDimmed) {
                Log.d("MainActivity", "🔆 键盘活动触发唤醒")
                wakeUpScreen()
            } else if (isKeepScreenOn && isAutoDimEnabled) {
                resetDimTimer()
            }
        }

        // 设置全局触摸回调：通过 AccessibilityService 检测屏幕任意位置的触摸
        // 能检测到键盘区域、悬浮窗口等 dispatchTouchEvent 无法覆盖的区域
        NextypeAccessibilityService.onScreenTouched = {
            if (isDimmed) {
                Log.d("MainActivity", "🔆 全局触摸触发唤醒（AccessibilityService）")
                wakeUpScreen()
            } else if (isKeepScreenOn && isAutoDimEnabled) {
                resetDimTimer()
            }
        }

        // 监听输入内容变化，如果有内容则隐藏引导文字
        inputEditText.addTextChangedListener(object : android.text.TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: android.text.Editable?) {
                // 💡 内容变化时唤醒屏幕或重置倒计时
                if (isDimmed) {
                    Log.d("MainActivity", "🔆 内容变化触发唤醒")
                    wakeUpScreen()
                } else {
                    resetDimTimer()
                }
            }
        })
        
        // 初始化配对管理器
        deviceManager = PairedDeviceManager(this)
        
        // 初始化 RelayClient
        Log.d("MainActivity", "🚀 [DEBUG] 准备初始化 RelayClient...")
        val lastDevice = deviceManager.getLastConnectedDevice()
        deviceId = lastDevice?.deviceId
        deviceName = lastDevice?.deviceName ?: "未知设备"
        
        // 读取剪贴板设置
        loadClipboardSettings()
        
        // 初始化屏幕常亮 & 自动变暗
        loadScreenSettings()
        applyKeepScreenOn()
        resetDimTimer()
        
        Log.d("MainActivity", "配对设备: $deviceName ($deviceId)")
        Log.d("MainActivity", "上次连接设备ID: ${deviceManager.lastConnectedDeviceId}")
        Log.d("MainActivity", "剪贴板设置: 同步=$pasteCopiesToClipboard, 发送=$pasteEnterCopiesToClipboard")
        
        // 设置按钮点击事件
        settingsButton.setOnClickListener {
            val intent = Intent(this, SettingsActivity::class.java)
            startActivity(intent)
        }
        
        // 设备状态标签点击事件 - 点击打开设备切换菜单
        deviceStatusCard.setOnClickListener { view ->
            val devices = deviceManager.getPairedDevices()
            if (devices.isEmpty()) {
                // 没有配对设备，跳转到欢迎页面
                val intent = Intent(this, EmptyStateActivity::class.java)
                startActivity(intent)
            } else {
                showDeviceSwitchMenu(view)
            }
        }
        
        // 更新设备状态标签显示
        updateDeviceStatusDisplay()
        
        // 设置清空按钮（带上滑恢复功能）
        setupRestoreTouchListener(clearButton)
        
        // 设置插入按钮（带上滑重发功能）
        setupResendTouchListener(syncButton, "插入", pressEnter = false)
        
        // 设置发送按钮（带上滑重发功能）
        setupResendTouchListener(sendButton, "发送", pressEnter = true)
        
        // 根据惯用手设置调整按钮顺序
        updateButtonOrder()
        
        // 启动数据传输通道
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            checkOnlineAndAutoSwitch {
                if (deviceId != null) {
                    deviceId?.let { connectToRelay(it) }
                }
                startConnectionMonitoring()
            }
        }, 1000)
        
        // 进入页面时默认显示光标并弹出键盘
        inputEditText.requestFocus()
        // 延迟弹出键盘，等待布局完成
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            imm.showSoftInput(inputEditText, android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT)
        }, 300)
    }
    
    /**
     * 自动切换到当前活跃的设备 (Follow the Light 策略)
     */
    private fun autoSwitchToActiveDevice() {
        // 防止同一次 onResume 中重复执行
        if (hasAutoSwitchedThisResume) {
            Log.d("MainActivity", "🤖 自动切换跳过: 本次 onResume 已执行过")
            return
        }
        hasAutoSwitchedThisResume = true
        
        if (!isWebSocketConnected || dataWebSocket == null) {
            Log.w("MainActivity", "🤖 自动切换跳过: WebSocket 未连接")
            return
        }

        discoverOnlineDevices { onlineDevices ->
            Log.d("MainActivity", "📋 收到在线服务器列表: ${onlineDevices.size} 个")
            onlineDevices.forEach { Log.d("MainActivity", "  - 设备: ${it.deviceName}, ID: ${it.deviceId}, idle: ${it.idleTime}s") }
            Log.d("MainActivity", "🤖 自动切换: 收到在线设备响应, 数量=${onlineDevices.size}")
            if (onlineDevices.isEmpty()) {
                Log.d("MainActivity", "🤖 自动切换: 无在线设备")
                return@discoverOnlineDevices
            }

            // 获取所有配对设备 ID 以进行过滤（确保只切换到自己配对过的设备）
            val pairedDeviceIds = deviceManager.getPairedDevices().map { it.deviceId }.toSet()
            
            // 过滤逻辑：
            // 1. 必须是配对过的设备
            // 2. 闲置时间 < 120秒
            val activeDevices = onlineDevices.filter { 
                val isPaired = pairedDeviceIds.contains(it.deviceId)
                val isRecent = it.idleTime < 120
                Log.d("MainActivity", "  - 检查设备: ${it.deviceName}, 配对=$isPaired, 闲置=${it.idleTime}s, 符合=$isRecent")
                isPaired && isRecent
            }

            if (activeDevices.isEmpty()) {
                Log.d("MainActivity", "🤖 自动切换: 未发现符合活跃条件的设备 (阈值 120s)")
                return@discoverOnlineDevices
            }

            // 寻找闲置时间最短的设备 (最新活跃)
            val mostActive = activeDevices.minByOrNull { it.idleTime } ?: return@discoverOnlineDevices

            Log.d("MainActivity", "🤖 自动切换决策: 全局最活跃设备为 ${mostActive.deviceName} (闲置 ${mostActive.idleTime}s)")

            // 如果当前已经连接的就是最活跃的，且连接正常，则不处理
            if (mostActive.deviceId == deviceId) {
                if (isWebSocketConnected) {
                    Log.d("MainActivity", "🤖 自动切换跳过: 当前已连接设备就是最活跃的，无需操作")
                } else {
                    Log.i("MainActivity", "🤖 自动切换触发: 当前选择虽然是最活跃，但连接已断开，执行重连...")
                    runOnUiThread { switchToDevice(deviceManager.getPairedDevices().find { it.deviceId == mostActive.deviceId } ?: return@runOnUiThread) }
                }
                return@discoverOnlineDevices
            }

            Log.d("MainActivity", "🤖 自动切换触发: 发现活跃设备 ${mostActive.deviceName} (闲置 ${mostActive.idleTime}s)")
            
            runOnUiThread {
                // 执行切换
                val targetPairedDevice = deviceManager.getPairedDevices().find { it.deviceId == mostActive.deviceId }
                if (targetPairedDevice != null) {
                    val displayName = targetPairedDevice.getDisplayName()
                    switchToDevice(targetPairedDevice)
                    // 弹出轻量提示（非阻塞）
                    android.widget.Toast.makeText(this, "已自动连接活跃设备: $displayName", android.widget.Toast.LENGTH_SHORT).show()
                }
            }
        }
    }
    

    private fun cancelPendingReconnect() {
        reconnectRunnable?.let { reconnectHandler?.removeCallbacks(it) }
        reconnectRunnable = null
    }

    private fun connectToRelay(targetDeviceId: String) {
        cancelPendingReconnect()
        // 先关闭旧连接，防止双重 WebSocket
        dataWebSocket?.close(1000, "Reconnecting")
        dataWebSocket = null
        
        val relayUrl = BuildConfig.RELAY_URL
        val request = Request.Builder().url(relayUrl).build()
        
        dataWebSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.d("MainActivity", "WebSocket 已连接到中继服务器")
                isWebSocketConnected = true  // 💡 标记连接已建立
                dataReconnectAttempts = 0  // 重置重连计数器
                
                // 刷新设备选择器列表（连接成功）
                runOnUiThread {
                    deviceSelectorAdapter?.updateConnectionStatus()
                }
                
                // 保存上次连接的设备ID
                if (deviceId != null) {
                    deviceManager.lastConnectedDeviceId = deviceId
                }
                
                // 注册设备
                val myDeviceId = DeviceIDManager.getInstance().getDeviceId(this@MainActivity)
                val registerMsg = JSONObject().apply {
                    put("type", "register")
                    put("role", "client")
                    put("deviceId", myDeviceId)
                    put("deviceName", android.os.Build.MODEL)
                }
                webSocket.send(registerMsg.toString())
            }
            
            override fun onMessage(webSocket: WebSocket, text: String) {
                Log.d("MainActivity", "收到消息: $text")
                try {
                    val json = JSONObject(text)
                    when (json.getString("type")) {
                        "connected" -> {
                            runOnUiThread {
                                updateDeviceStatusDisplay()
                                updateInputHint("已连接 · 请输入内容")
                            }
                        }
                        "registered" -> {
                            Log.d("MainActivity", "已注册到中继服务器")

                            // 启动心跳
                            startHeartbeat()

                            // 💡 主动发送 Ping 给 PC，触发 PC 端的上线感知
                            if (deviceId != null) {
                                try {
                                    val myDeviceId = DeviceIDManager.getInstance().getDeviceId(this@MainActivity)
                                    val pingMsg = JSONObject().apply {
                                        put("type", "ping")
                                        put("timestamp", System.currentTimeMillis())
                                    }

                                    val relayMessage = JSONObject().apply {
                                        put("type", "relay")
                                        put("from", myDeviceId)
                                        put("to", deviceId)
                                        put("data", pingMsg.toString())
                                    }
                                    dataWebSocket?.send(relayMessage.toString())
                                    Log.d("MainActivity", "📤 已发送上线通知给 PC")

                                    // 📱 上报屏幕参数给 PC（握手信息）
                                    sendScreenInfoToPC()
                                } catch (e: Exception) {
                                    Log.e("MainActivity", "发送上线通知失败", e)
                                }
                            }

                            // 触发自动切换（如果还未执行）
                            if (!hasAutoSwitchedThisResume) {
                                Log.d("MainActivity", "📡 注册成功，触发自动切换检查")
                                autoSwitchToActiveDevice()
                            }
                        }
                        "ack" -> {
                            Log.d("MainActivity", "✅ PC端确认收到")
                        }
                        "device_unpaired" -> {
                            // 💡 收到解除配对通知 - 无论消息来源，只要收到就处理
                            Log.d("MainActivity", "💔 收到解除配对通知")
                            handleUnpairNotification(webSocket)
                        }
                        "relay" -> {
                            // 💡 收到 PC 端中继消息
                            try {
                                val dataStr = json.getString("data")
                                val dataJson = JSONObject(dataStr)
                                val innerType = dataJson.optString("type", "")

                                if (innerType == "command") {
                                    val action = dataJson.getString("action")
                                    Log.d("MainActivity", "🎮 收到 PC 端远程指令: $action")

                                    runOnUiThread {
                                        handleRemoteCommand(action, dataJson)
                                    }
                                } else if (innerType == "error" && dataJson.optString("message") == "decrypt_failed") {
                                    Log.w("MainActivity", "🔐 收到解密失败通知，提示用户重新配对")
                                    runOnUiThread {
                                        val dialog = androidx.appcompat.app.AlertDialog.Builder(this@MainActivity)
                                            .setTitle("加密密钥已失效")
                                            .setMessage("与电脑的加密连接已中断（通常因重装 App 导致）。\n\n请重新配对以恢复正常使用。")
                                            .setPositiveButton("立即重新配对") { _, _ ->
                                                val intent = Intent(this@MainActivity, PairingActivity::class.java)
                                                startActivity(intent)
                                            }
                                            .setNegativeButton("稍后处理", null)
                                            .create()
                                        dialog.show()
                                        dialog.getButton(androidx.appcompat.app.AlertDialog.BUTTON_POSITIVE)
                                            .setTextColor(androidx.core.content.ContextCompat.getColor(this@MainActivity, R.color.accent))
                                        dialog.getButton(androidx.appcompat.app.AlertDialog.BUTTON_NEGATIVE)
                                            .setTextColor(androidx.core.content.ContextCompat.getColor(this@MainActivity, R.color.text_secondary))
                                    }
                                }
                            } catch (e: Exception) {
                                Log.e("MainActivity", "解析 relay 数据失败", e)
                            }
                        }
                        "server_list" -> {
                            // 收到在线服务器列表
                            val serversArray = json.optJSONArray("servers")
                            val onlineDevices = mutableListOf<OnlineDeviceInfo>()
                            if (serversArray != null) {
                                for (i in 0 until serversArray.length()) {
                                    val serverObj = serversArray.optJSONObject(i)
                                    if (serverObj != null) {
                                        val devId = serverObj.optString("deviceId")
                                        val devName = serverObj.optString("deviceName", "Unknown")
                                        val idleTime = serverObj.optLong("idleTime", 999999L)
                                        if (devId.isNotEmpty()) {
                                            onlineDevices.add(OnlineDeviceInfo(devId, devName, idleTime))
                                        }
                                    }
                                }
                            }
                            Log.d("MainActivity", "📋 收到在线服务器列表: ${onlineDevices.size} 个")
                            onlineDevicesCallback?.invoke(onlineDevices)
                            onlineDevicesCallback = null
                        }
                        "server_online" -> {
                            // PC 设备上线
                            val serverId = json.optString("serverId")
                            val serverName = json.optString("serverName", "Unknown")
                            Log.d("MainActivity", "🟢 PC 上线: $serverName ($serverId)")

                            // 用最新名称更新本地存储
                            if (serverId.isNotEmpty() && serverName.isNotEmpty()) {
                                deviceManager.updateDeviceName(serverId, serverName)
                            }

                            serverStatusCallback?.invoke(serverId, serverName, true)
                            runOnUiThread { updateDeviceStatusDisplay() }
                        }
                        "server_offline" -> {
                            // PC 设备下线
                            val serverId = json.optString("serverId")
                            Log.d("MainActivity", "🔴 PC 下线: $serverId")
                            serverStatusCallback?.invoke(serverId, "", false)
                        }
                    }
                } catch (e: Exception) {
                    Log.e("MainActivity", "解析消息失败", e)
                }
            }
            
            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e("MainActivity", "WebSocket 连接失败", t)
                isWebSocketConnected = false
                stopHeartbeat()  // 停止心跳
                runOnUiThread {
                    updateDeviceStatusDisplay()
                }
                // 自动重连（非后台状态且未超过最大重试次数）
                scheduleDataReconnect()
            }
            
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.d("MainActivity", "WebSocket 连接已关闭: $reason")
                isWebSocketConnected = false
                runOnUiThread {
                    updateDeviceStatusDisplay()
                }
                // 非正常关闭时自动重连
                if (code != 1000) {
                    scheduleDataReconnect()
                }
            }
        })
    }
    
    
    /**
     * 调度数据 WebSocket 重连
     * 使用指数退避策略，后台时不重连
     */
    private fun scheduleDataReconnect() {
        if (dataReconnectAttempts >= maxDataReconnectAttempts) {
            Log.w("MainActivity", "🔄 数据通道重连已达最大次数 ($maxDataReconnectAttempts)，停止重连")
            return
        }

        cancelPendingReconnect()

        dataReconnectAttempts++
        // 指数退避：2s, 4s, 8s, 16s... 最大 30s
        val delay = minOf(2000L * (1L shl (dataReconnectAttempts - 1)), 30000L)

        Log.d("MainActivity", "🔄 数据通道将在 ${delay}ms 后重连 (第 $dataReconnectAttempts 次)")

        if (reconnectHandler == null) {
            reconnectHandler = android.os.Handler(android.os.Looper.getMainLooper())
        }
        reconnectRunnable = Runnable {
            reconnectRunnable = null
            if (!isWebSocketConnected && !isSmartSwitching && deviceId != null) {
                Log.d("MainActivity", "🔄 开始数据通道重连...")
                deviceId?.let { connectToRelay(it) }
            }
        }
        reconnectHandler?.postDelayed(reconnectRunnable!!, delay)
    }
    
    /**
     * 启动主动心跳机制
     * 每60秒发送一次心跳消息，检测连接是否真的有效
     * 优化：心跳间隔从20秒调整为60秒，减少耗电和流量
     */
    private fun startHeartbeat() {
        stopHeartbeat()  // 先停止旧的心跳
        
        heartbeatHandler = android.os.Handler(android.os.Looper.getMainLooper())
        heartbeatRunnable = object : Runnable {
            override fun run() {
                if (isWebSocketConnected && dataWebSocket != null) {
                    try {
                        val heartbeat = org.json.JSONObject().apply {
                            put("type", "heartbeat")
                            put("timestamp", System.currentTimeMillis())
                        }
                        dataWebSocket?.send(heartbeat.toString())
                        Log.d("MainActivity", "💓 发送心跳")
                        // 继续下一次心跳
                        heartbeatHandler?.postDelayed(this, 30000) // 每30秒
                    } catch (e: Exception) {
                        Log.e("MainActivity", "💓 ❌ 心跳发送失败", e)
                        // 心跳失败说明连接异常，触发重连
                        isWebSocketConnected = false
                        scheduleDataReconnect()
                    }
                } else {
                    Log.d("MainActivity", "💓 连接已断开，停止心跳")
                }
            }
        }
        // 首次延迟30秒后发送
        heartbeatHandler?.postDelayed(heartbeatRunnable!!, 30000)
        Log.d("MainActivity", "💓 心跳已启动（30秒间隔）")
    }
    
    /**
     * 停止主动心跳
     */
    private fun stopHeartbeat() {
        heartbeatRunnable?.let { runnable ->
            heartbeatHandler?.removeCallbacks(runnable)
        }
        heartbeatHandler = null
        heartbeatRunnable = null
        Log.d("MainActivity", "💓 心跳已停止")
    }

    /**
     * 查询在线的 PC 设备列表（通过统一的dataWebSocket）
     */
    private fun discoverOnlineDevices(callback: (List<OnlineDeviceInfo>) -> Unit) {
        if (!isWebSocketConnected || dataWebSocket == null) {
            Log.w("MainActivity", "⚠️ 无法查询在线设备: WebSocket未连接")
            callback(emptyList())
            return
        }

        onlineDevicesCallback = callback
        val message = JSONObject().apply {
            put("type", "discover")
        }
        Log.d("MainActivity", "🔍 发送 DISCOVER 请求...")
        dataWebSocket?.send(message.toString())

        // 5秒请求超时
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            if (onlineDevicesCallback != null) {
                Log.w("MainActivity", "⚠️ 查询在线设备超时")
                onlineDevicesCallback?.invoke(emptyList())
                onlineDevicesCallback = null
            }
        }, 5000)
    }


    /**
     * 设置带上滑重发功能的触摸监听器
     * 按住按钮上滑可重发上次内容
     */
    @SuppressLint("ClickableViewAccessibility")
    private fun setupResendTouchListener(button: Button, actionName: String, pressEnter: Boolean) {
        var isSlideUp = false
        var popupShowing = false
        var buttonLocation = IntArray(2)
        
        button.setOnTouchListener { view, event ->
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    isSlideUp = false
                    popupShowing = false
                    view.isPressed = true
                    // 获取按钮在屏幕上的位置
                    view.getLocationOnScreen(buttonLocation)
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    // Update pressed state based on if finger is inside
                    if (!isSlideUp && !popupShowing) {
                        view.isPressed = isPointInsideView(event.rawX, event.rawY, view)
                    } else {
                        view.isPressed = false
                    }

                    // 按钮上边缘的 Y 坐标
                    val buttonTopY = buttonLocation[1]
                    // 当前触摸点的 Y 坐标
                    val currentY = event.rawY
                    
                    // 检查是否有上次内容可重发
                    // 注意：只有当手指确实滑出按钮范围一段距离后才开始检测上滑
                    // 避免普通点击时的微小移动误触
                    if (lastSentContent != null && inputEditText.text?.isEmpty() == true) {
                        val popupText = "上滑$actionName"
                        
                        // 逻辑：手指位置在按钮上边缘之上 -> 显示并选中
                        // 手指位置在按钮上边缘之下 -> 如果已显示，则变为未选中（不立即隐藏，给用户悔棋机会，除非松手或滑太远）
                        
                        if (currentY < buttonTopY) {
                            // 在水平线之上
                            if (!popupShowing) {
                                // 刚越过红线，显示弹窗（选中状态）
                                showResendPopup(view, popupText, R.drawable.ic_sync, isSelected = true)
                                popupShowing = true
                                performHapticFeedback()
                                // 显示预览内容
                                showSwipePreview(lastSentContent!!)
                            } else {
                                // 已经在显示，保持/更新为选中状态
                                showResendPopup(view, popupText, R.drawable.ic_sync, isSelected = true)
                                // 确保预览继续显示
                                showSwipePreview(lastSentContent!!)
                            }
                            isSlideUp = true
                        } else {
                            // 在水平线之下
                            if (popupShowing) {
                                // 如果弹窗已显示，则变为未选中状态（灰色），提示用户松手将取消
                                showResendPopup(view, popupText, R.drawable.ic_sync, isSelected = false)
                                // 隐藏预览内容
                                hideSwipePreview()
                                isSlideUp = false
                            }
                        }
                    }
                    true
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    view.isPressed = false
                    hideResendPopup()
                    hideSwipePreview()  // 隐藏预览
                    
                    if (isSlideUp && lastSentContent != null) {
                        // 上滑松开且选中状态，重发上次内容
                        Log.d("MainActivity", "🔄 上滑重发上次内容")
                        sendContentWithText(lastSentContent!!, pressEnter)
                    } else if (!popupShowing && event.action == MotionEvent.ACTION_UP) {
                        // 没有显示弹窗，且手指在按钮范围内松开 -> 视为点击
                        if (isPointInsideView(event.rawX, event.rawY, view)) {
                            performHapticFeedback()
                            sendContent(pressEnter)
                            // 显示上滑操作提示（全局只显示一次）
                            showSwipeHintToast(view)
                        }
                    }
                    
                    isSlideUp = false
                    popupShowing = false
                    if (event.action == MotionEvent.ACTION_UP && isPointInsideView(event.rawX, event.rawY, view)) {
                        view.performClick()
                    }
                    true
                }
                else -> false
            }
        }
    }

    /**
     * 设置带上滑恢复功能的触摸监听器（用于清空按钮）
     * 按住按钮上滑可恢复上次清空的内容
     */
    @SuppressLint("ClickableViewAccessibility")
    private fun setupRestoreTouchListener(button: Button) {
        var isSlideUp = false
        var popupShowing = false
        var buttonLocation = IntArray(2)
        
        button.setOnTouchListener { view, event ->
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    isSlideUp = false
                    popupShowing = false
                    view.isPressed = true
                    // 获取按钮在屏幕上的位置
                    view.getLocationOnScreen(buttonLocation)
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    // Update pressed state based on if finger is inside
                    if (!isSlideUp && !popupShowing) {
                        view.isPressed = isPointInsideView(event.rawX, event.rawY, view)
                    } else {
                        view.isPressed = false
                    }

                    // 按钮上边缘的 Y 坐标
                    val buttonTopY = buttonLocation[1]
                    // 当前触摸点的 Y 坐标
                    val currentY = event.rawY
                    
                    // 检查是否有上次内容可恢复
                    // 逻辑：当前输入框为空，且有上次清空的内容
                    if (lastClearedContent != null && inputEditText.text?.isEmpty() == true) {
                        val popupText = "上滑恢复"
                        
                        if (currentY < buttonTopY) {
                            // 在水平线之上
                            if (!popupShowing) {
                                // 刚越过红线，显示弹窗（选中状态）
                                showResendPopup(view, popupText, R.drawable.ic_undo, isSelected = true)
                                popupShowing = true
                                performHapticFeedback()
                                // 显示预览内容
                                showSwipePreview(lastClearedContent!!)
                            } else {
                                // 已经在显示，保持/更新为选中状态
                                showResendPopup(view, popupText, R.drawable.ic_undo, isSelected = true)
                                // 确保预览继续显示
                                showSwipePreview(lastClearedContent!!)
                            }
                            isSlideUp = true
                        } else {
                            // 在水平线之下
                            if (popupShowing) {
                                // 如果弹窗已显示，则变为未选中状态（灰色），提示用户松手将取消
                                showResendPopup(view, popupText, R.drawable.ic_undo, isSelected = false)
                                // 隐藏预览内容
                                hideSwipePreview()
                                isSlideUp = false
                            }
                        }
                    }
                    true
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    view.isPressed = false
                    hideResendPopup()
                    hideSwipePreview()  // 隐藏预览
                    
                    if (isSlideUp && lastClearedContent != null) {
                        // 上滑松开且选中状态，恢复上次内容
                        Log.d("MainActivity", "🔄 上滑恢复上次内容")
                        inputEditText.setText(lastClearedContent)
                        inputEditText.setSelection(lastClearedContent!!.length)
                        android.widget.Toast.makeText(this, "已恢复内容", android.widget.Toast.LENGTH_SHORT).show()
                    } else if (!popupShowing && event.action == MotionEvent.ACTION_UP) {
                        // 没有显示弹窗，且手指在按钮范围内松开 -> 视为点击，执行清空操作
                        if (isPointInsideView(event.rawX, event.rawY, view)) {
                            performHapticFeedback()
                            // 保存内容用于恢复
                            val currentText = inputEditText.text.toString()
                            if (currentText.isNotEmpty()) {
                                lastClearedContent = currentText
                                inputEditText.text?.clear()
                                Log.d("MainActivity", "🗑️ 已清空内容，并保存用于恢复")
                            }
                            // 清空按钮不显示上滑提示，让用户自己发现
                        }
                    }
                    
                    isSlideUp = false
                    popupShowing = false
                    if (event.action == MotionEvent.ACTION_UP && isPointInsideView(event.rawX, event.rawY, view)) {
                        view.performClick()
                    }
                    true
                }
                else -> false
            }
        }
    }

    /**
     * 判断触摸点是否在视图范围内
     */
    private fun isPointInsideView(rawX: Float, rawY: Float, view: android.view.View): Boolean {
        val location = IntArray(2)
        view.getLocationOnScreen(location)
        val viewX = location[0]
        val viewY = location[1]
        
        return rawX >= viewX && rawX <= (viewX + view.width) &&
               rawY >= viewY && rawY <= (viewY + view.height)
    }
    
    /**
     * 显示重发提示弹窗（样式与底部按钮高度一致，圆角矩形）
     */
    /**
     * 显示重发/恢复提示弹窗（样式与底部按钮高度一致，圆角矩形）
     */
    private fun showResendPopup(anchorView: android.view.View, text: String, iconResId: Int, isSelected: Boolean = true) {
        if (resendPopup?.isShowing == true) {
            // 更新选中状态
            updateResendPopupState(isSelected)
            return
        }
        
        // 获取底部按钮的实际尺寸
        val displayMetrics = resources.displayMetrics
        val buttonWidth = anchorView.width
        val buttonHeight = anchorView.height
        
        // 使用 XML 布局 inflate 按钮，确保与底部按钮样式完全一致
        val button = layoutInflater.inflate(R.layout.popup_swipe_button, null) as com.google.android.material.button.MaterialButton
        
        // 设置文案和图标
        button.text = text
        button.icon = androidx.core.content.ContextCompat.getDrawable(this, iconResId)?.mutate()
        
        // 根据选中状态设置颜色
        if (isSelected) {
            // 选中态：蓝色背景，白色文字
            button.backgroundTintList = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.accent))
            button.setTextColor(android.graphics.Color.WHITE)
            button.iconTint = android.content.res.ColorStateList.valueOf(android.graphics.Color.WHITE)
            button.strokeColor = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.accent))
        } else {
            // 未选中态：白色背景，黑色文字
            button.backgroundTintList = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.card_background))
            button.setTextColor(ContextCompat.getColor(this, R.color.text_primary))
            button.iconTint = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.text_primary))
            button.strokeColor = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.border))
        }

        // 禁用点击效果
        button.isClickable = false
        button.isFocusable = false
        button.stateListAnimator = null
        
        resendPopup = android.widget.PopupWindow(
            button,
            buttonWidth,
            buttonHeight,
            false
        ).apply {
            animationStyle = 0
            inputMethodMode = android.widget.PopupWindow.INPUT_METHOD_NEEDED
        }
        
        // 计算位置（在按钮正上方完全对齐）
        val location = IntArray(2)
        anchorView.getLocationOnScreen(location)
        val x = location[0]
        val y = location[1] - buttonHeight - (8 * displayMetrics.density).toInt()
        
        resendPopup?.showAtLocation(anchorView, android.view.Gravity.NO_GRAVITY, x, y)
    }
    
    /**
     * 更新重发弹窗的选中状态
     */
    private fun updateResendPopupState(isSelected: Boolean) {
        val button = resendPopup?.contentView as? com.google.android.material.button.MaterialButton ?: return
        
        if (isSelected) {
            // 选中态：蓝色背景，白色文字
            button.backgroundTintList = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.accent))
            button.setTextColor(android.graphics.Color.WHITE)
            button.iconTint = android.content.res.ColorStateList.valueOf(android.graphics.Color.WHITE)
            button.strokeColor = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.accent))
        } else {
            // 未选中态：卡片白色背景，主色文字
            button.backgroundTintList = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.card_background))
            button.setTextColor(ContextCompat.getColor(this, R.color.text_primary))
            button.iconTint = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.text_primary))
            button.strokeColor = android.content.res.ColorStateList.valueOf(ContextCompat.getColor(this, R.color.border))
        }
    }
    

    /**
     * 隐藏重发提示弹窗
     */
    private fun hideResendPopup() {
        resendPopup?.dismiss()
        resendPopup = null
    }
    
    /**
     * 显示上滑预览文本
     * 在输入框中以灰色形式展示将要处理的内容
     */
    private fun showSwipePreview(content: String) {
        runOnUiThread {
            swipePreviewText.text = content
            swipePreviewText.visibility = android.view.View.VISIBLE
            // 同步字号设置
            swipePreviewText.textSize = inputEditText.textSize / resources.displayMetrics.scaledDensity
        }
    }
    
    /**
     * 隐藏上滑预览文本
     */
    private fun hideSwipePreview() {
        runOnUiThread {
            swipePreviewText.visibility = android.view.View.GONE
            swipePreviewText.text = ""
        }
    }
    
    /**
     * 执行震动反馈
     */
    private fun performHapticFeedback() {
        val vibrator = getSystemService(Context.VIBRATOR_SERVICE) as android.os.Vibrator
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) {
            vibrator.vibrate(android.os.VibrationEffect.createPredefined(android.os.VibrationEffect.EFFECT_CLICK))
        } else if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
            vibrator.vibrate(android.os.VibrationEffect.createOneShot(20, 255))
        } else {
            @Suppress("DEPRECATION")
            vibrator.vibrate(20)
        }
    }
    
    /**
     * 加密并通过中继发送内容（sendContent / sendContentWithText 的公共核心）
     */
    private fun sendEncryptedMessage(content: String, pressEnter: Boolean) {
        val myDeviceId = DeviceIDManager.getInstance().getDeviceId(this)
        val currentDevice = deviceId?.let { id -> deviceManager.getPairedDevices().find { it.deviceId == id } }
        val encryptionKey = currentDevice?.encryptionKey?.takeIf { it.isNotEmpty() } ?: myDeviceId

        val encryptedContent = encryptContent(content, encryptionKey)
        val action = if (pressEnter) "paste-enter" else "paste"
        val message = JSONObject().apply {
            put("type", "clipboard")
            put("content", encryptedContent)
            put("action", action)
            put("encrypted", true)
        }

        if (deviceId != null) {
            val relayMessage = JSONObject().apply {
                put("type", "relay")
                put("from", myDeviceId)
                put("to", deviceId)
                put("data", message.toString())
            }
            dataWebSocket?.send(relayMessage.toString())
            Log.d("MainActivity", "📤 通过中继服务器发送: $action")
        }
    }

    /**
     * 使用指定内容发送（用于重发上次内容）
     */
    private fun sendContentWithText(content: String, pressEnter: Boolean) {
        try {
            sendEncryptedMessage(content, pressEnter)
            Log.d("MainActivity", "✅ [重发] 消息已发送")
            android.widget.Toast.makeText(this, "已重新发送上次内容", android.widget.Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Log.e("MainActivity", "❌ [重发] 发送失败", e)
        }
    }

    private fun sendContent(pressEnter: Boolean) {
        val content = inputEditText.text.toString()
        if (content.isEmpty()) {
            Log.d("MainActivity", "输入内容为空,不发送")
            return
        }

        try {
            Log.d("MainActivity", "🔐 [加密] 目标设备ID: $deviceId，内容长度: ${content.length} 字节")
            sendEncryptedMessage(content, pressEnter)

            // 💡 保存上次发送的内容（用于上滑重发）
            lastSentContent = content

            // 清空输入框
            inputEditText.text?.clear()
            Log.d("MainActivity", "✅ 消息已发送")

            // 🔴 复制到剪贴板(根据设置)
            val shouldCopy = if (pressEnter) pasteEnterCopiesToClipboard else pasteCopiesToClipboard
            if (shouldCopy) {
                val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                val clip = android.content.ClipData.newPlainText("Nextype", content)
                clipboard.setPrimaryClip(clip)
                Log.d("MainActivity", "📋 已复制到剪贴板")
            }

        } catch (e: Exception) {
            Log.e("MainActivity", "❌ 发送失败", e)
        }
    }
    
    /**
     * 处理 PC 端远程指令
     * 支持: send(发送), insert(插入), clear(清空), tap(模拟点击)
     */
    private fun handleRemoteCommand(action: String, commandJson: JSONObject) {
        // 防抖：防止同一条命令被重复执行
        val now = System.currentTimeMillis()
        if (now - lastRemoteCommandTime < remoteCommandDebounceMs) {
            Log.d("MainActivity", "🎮 ⏳ 防抖中，忽略重复命令: $action")
            return
        }
        lastRemoteCommandTime = now
        
        // 收到远程指令时，先唤醒屏幕
        if (isDimmed) {
            Log.d("MainActivity", "🎮 💡 远程指令触发唤醒屏幕")
            wakeUpScreen()
        } else {
            // 非变暗状态也重置倒计时
            resetDimTimer()
        }
        
        Log.d("MainActivity", "🎮 处理远程指令: $action")
        
        when (action) {
            "send" -> {
                // 触发发送按钮（回车）
                Log.d("MainActivity", "🎮 执行: 发送(回车)")
                sendContent(true)
            }
            "insert" -> {
                // 触发插入按钮（无回车）
                Log.d("MainActivity", "🎮 执行: 插入")
                sendContent(false)
            }
            "clear" -> {
                // 触发清空按钮
                Log.d("MainActivity", "🎮 执行: 清空")
                val currentText = inputEditText.text.toString()
                if (currentText.isNotEmpty()) {
                    lastClearedContent = currentText
                    inputEditText.text?.clear()
                    Log.d("MainActivity", "🎮 已清空内容，保存用于恢复")
                }
            }
            "tap" -> {
                // 模拟点击指定坐标
                Log.d("MainActivity", "🎮 执行: 模拟点击")
                handleRemoteTap(commandJson)
            }
            "touch_down" -> {
                // 长按按下（手指按住不放）
                Log.d("MainActivity", "🎮 执行: 长按按下")
                handleRemoteTouchDown(commandJson)
            }
            "touch_up" -> {
                // 长按释放（抬起手指）
                Log.d("MainActivity", "🎮 执行: 长按释放")
                handleRemoteTouchUp()
            }
            "touch_heartbeat" -> {
                // 长按心跳（保持长按状态）
                NextypeAccessibilityService.instance?.onHeartbeat()
            }
            else -> {
                Log.w("MainActivity", "🎮 未知指令: $action")
            }
        }
    }
    
    /**
     * 处理远程模拟点击指令
     * PC 端已根据设备屏幕参数匹配好坐标，直接使用 x、y
     */
    private fun handleRemoteTap(commandJson: JSONObject) {
        try {
            // 优先使用 PC 端已匹配好的直接坐标
            if (commandJson.has("x") && commandJson.has("y")) {
                val targetX = commandJson.getInt("x")
                val targetY = commandJson.getInt("y")
                Log.d("MainActivity", "🎮 收到 tap 指令: ($targetX, $targetY)")
                performSimulatedTap(targetX, targetY)
                return
            }
            
            // 向后兼容：旧格式带 coordinates 对象
            val coordinatesJson = commandJson.optJSONObject("coordinates")
            if (coordinatesJson == null) {
                Log.e("MainActivity", "🎮 ❌ 缺少坐标配置")
                return
            }
            
            val displayMetrics = resources.displayMetrics
            val screenWidth = displayMetrics.widthPixels
            val screenHeight = displayMetrics.heightPixels
            Log.d("MainActivity", "🎮 (兼容模式) 当前屏幕尺寸: ${screenWidth}x${screenHeight}")
            
            val foldedConfig = coordinatesJson.optJSONObject("folded")
            val unfoldedConfig = coordinatesJson.optJSONObject("unfolded")
            
            var targetX = 0
            var targetY = 0
            var matched = false
            
            if (foldedConfig != null) {
                val fw = foldedConfig.optInt("width", 0)
                val fh = foldedConfig.optInt("height", 0)
                if (screenWidth == fw && screenHeight == fh) {
                    targetX = foldedConfig.optInt("x", 0)
                    targetY = foldedConfig.optInt("y", 0)
                    matched = true
                }
            }
            
            if (!matched && unfoldedConfig != null) {
                val uw = unfoldedConfig.optInt("width", 0)
                val uh = unfoldedConfig.optInt("height", 0)
                if (screenWidth == uw && screenHeight == uh) {
                    targetX = unfoldedConfig.optInt("x", 0)
                    targetY = unfoldedConfig.optInt("y", 0)
                    matched = true
                }
            }
            
            if (!matched) {
                if (foldedConfig != null) {
                    targetX = foldedConfig.optInt("x", screenWidth / 2)
                    targetY = foldedConfig.optInt("y", screenHeight - 200)
                } else if (unfoldedConfig != null) {
                    targetX = unfoldedConfig.optInt("x", screenWidth / 2)
                    targetY = unfoldedConfig.optInt("y", screenHeight - 200)
                } else {
                    Log.e("MainActivity", "🎮 ❌ 无有效坐标配置")
                    return
                }
            }
            
            performSimulatedTap(targetX, targetY)
            
        } catch (e: Exception) {
            Log.e("MainActivity", "🎮 ❌ 处理模拟点击失败", e)
        }
    }

    /**
     * 处理远程长按按下指令（手指按住不放）
     */
    private fun handleRemoteTouchDown(commandJson: JSONObject) {
        try {
            if (commandJson.has("x") && commandJson.has("y")) {
                val targetX = commandJson.getInt("x")
                val targetY = commandJson.getInt("y")
                Log.d("MainActivity", "🎮 收到 touch_down 指令: ($targetX, $targetY)")

                val service = NextypeAccessibilityService.instance
                if (service != null) {
                    service.performTouchDown(targetX.toFloat(), targetY.toFloat())
                    Log.d("MainActivity", "🎮 ✅ 长按按下已发送到 AccessibilityService")
                } else {
                    Log.w("MainActivity", "🎮 ⚠️ AccessibilityService 未启用，无法执行长按")
                    sendErrorToPC("辅助功能服务未启用", "长按失败：请在系统设置中开启「落笔 Nextype」的辅助功能权限")
                    runOnUiThread {
                        showAccessibilityHintToast()
                    }
                }
            } else {
                Log.e("MainActivity", "🎮 ❌ touch_down 缺少坐标")
            }
        } catch (e: Exception) {
            Log.e("MainActivity", "🎮 ❌ 处理长按按下失败", e)
        }
    }

    /**
     * 处理远程长按释放指令（抬起手指）
     */
    private fun handleRemoteTouchUp() {
        try {
            val service = NextypeAccessibilityService.instance
            if (service != null) {
                service.performTouchUp()
                Log.d("MainActivity", "🎮 ✅ 长按释放已发送到 AccessibilityService")
            } else {
                Log.w("MainActivity", "🎮 ⚠️ AccessibilityService 未启用")
            }
        } catch (e: Exception) {
            Log.e("MainActivity", "🎮 ❌ 处理长按释放失败", e)
        }
    }

    /**
     * 执行模拟点击
     * 需要通过 AccessibilityService 实现
     */
    private fun performSimulatedTap(x: Int, y: Int) {
        Log.d("MainActivity", "🎮 执行模拟点击: ($x, $y)")
        
        // 检查 AccessibilityService 是否可用
        val service = NextypeAccessibilityService.instance
        if (service != null) {
            service.performTap(x.toFloat(), y.toFloat())
            Log.d("MainActivity", "🎮 ✅ 模拟点击已发送到 AccessibilityService")
        } else {
            Log.w("MainActivity", "🎮 ⚠️ AccessibilityService 未启用，无法执行模拟点击")
            
            // 💡 向 PC 端发送错误消息
            sendErrorToPC("辅助功能服务未启用", "模拟点击失败：请在系统设置中开启「落笔 Nextype」的辅助功能权限")
            
            // 💡 在移动端显示气泡提示
            runOnUiThread {
                showAccessibilityHintToast()
            }
        }
    }
    
    /**
     * 向 PC 端发送错误消息
     */
    private fun sendErrorToPC(errorTitle: String, errorMessage: String) {
        try {
            val errorJson = JSONObject().apply {
                put("type", "error")
                put("errorType", "permission_denied")
                put("errorTitle", errorTitle)
                put("errorMessage", errorMessage)
            }
            
            // 通过中继服务器发送
            if (dataWebSocket != null && isWebSocketConnected && deviceId != null) {
                val myDeviceId = DeviceIDManager.getInstance().getDeviceId(this)
                val relayMessage = JSONObject().apply {
                    put("type", "relay")
                    put("from", myDeviceId)
                    put("to", deviceId)
                    put("data", errorJson.toString())
                }
                dataWebSocket?.send(relayMessage.toString())
                Log.d("MainActivity", "🎮 ✉️ 错误消息已发送到 PC: $errorTitle")
            } else {
                Log.w("MainActivity", "🎮 ⚠️ 无法发送错误消息：WebSocket 未连接")
            }
        } catch (e: Exception) {
            Log.e("MainActivity", "🎮 ❌ 发送错误消息失败", e)
        }
    }

    /**
     * 向 PC 端发送当前屏幕参数
     * 用于连接握手和折叠屏状态变更上报
     */
    private fun sendScreenInfoToPC(eventType: String = "device_info") {
        try {
            if (dataWebSocket == null || !isWebSocketConnected || deviceId == null) {
                Log.w("MainActivity", "📱 无法发送屏幕信息: WebSocket 未连接")
                return
            }
            
            val displayMetrics = resources.displayMetrics
            val screenWidth = displayMetrics.widthPixels
            val screenHeight = displayMetrics.heightPixels
            
            val myDeviceId = DeviceIDManager.getInstance().getDeviceId(this)
            val screenInfo = JSONObject().apply {
                put("type", eventType)
                put("screenWidth", screenWidth)
                put("screenHeight", screenHeight)
                put("platform", "android")
            }
            val relayMessage = JSONObject().apply {
                put("type", "relay")
                put("from", myDeviceId)
                put("to", deviceId)
                put("data", screenInfo.toString())
            }
            dataWebSocket?.send(relayMessage.toString())
            Log.d("MainActivity", "📱 已上报屏幕参数: ${screenWidth}x${screenHeight} ($eventType)")
        } catch (e: Exception) {
            Log.e("MainActivity", "📱 发送屏幕参数失败", e)
        }
    }
    
    /**
     * 监听屏幕配置变化（折叠屏展开/折叠）
     * 当屏幕尺寸发生变化时，自动上报新的屏幕参数给 PC
     */
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        
        val newWidth = resources.displayMetrics.widthPixels
        val newHeight = resources.displayMetrics.heightPixels
        
        if (newWidth != lastScreenWidth || newHeight != lastScreenHeight) {
            Log.d("MainActivity", "📱 屏幕尺寸变化: ${lastScreenWidth}x${lastScreenHeight} -> ${newWidth}x${newHeight}")
            lastScreenWidth = newWidth
            lastScreenHeight = newHeight
            
            // 💡 动态控制旋转锁
            checkScreenOrientation(newWidth, newHeight)
            
            // 发送屏幕变更通知给 PC
            sendScreenInfoToPC("screen_changed")
        }
    }
    
    /**
     * 根据屏幕比例动态控制旋转锁定
     * 极窄屏幕锁定竖屏，宽屏允许自由旋转
     */
    private fun checkScreenOrientation(width: Int, height: Int) {
        val ratio = width.toFloat() / height.toFloat()
        
        if (ratio < 0.65f) {
            // 极窄屏幕（折叠态竖屏）：锁定为竖屏，禁止横屏带来的糟糕体验
            if (requestedOrientation != ActivityInfo.SCREEN_ORIENTATION_PORTRAIT) {
                requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
                Log.d("MainActivity", "📱 锁定旋转: 竖屏模式 (比例: $ratio)")
            }
        } else if (ratio > 1.54f && ratio < 1.65f) {
            // 某些设备折叠态横屏比例可能在这里，保持锁定或根据需要调整
            // 这里暂不处理，交给下方的 USER
        } else {
            // 较宽屏幕（展开态）：允许随传感器自动旋转（USER模式兼容性比UNSPECIFIED更好）
            if (requestedOrientation != ActivityInfo.SCREEN_ORIENTATION_USER) {
                requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_USER
                Log.d("MainActivity", "📖 解锁旋转: 自动模式 (比例: $ratio)")
            }
        }
    }
    
    /**
     * 华为/荣耀设备私有 API：设置 FLAG_NOTCH_SUPPORT
     * 通过反射将 0x00010000 标志添加到 WindowManager.LayoutParams.hwFlags
     * 告知荣耀系统该应用支持挖孔屏，允许内容延伸到挖孔区域
     */
    private fun setHuaweiNotchSupport() {
        val FLAG_NOTCH_SUPPORT = 0x00010000
        try {
            val layoutParams = window.attributes
            
            // 方法1：元反射绕过 Android 9+ 隐藏 API 限制
            // 通过反射获取 Class.getDeclaredField 方法本身，可以绕过 blacklist 检查
            val getDeclaredFieldMethod = Class::class.java.getDeclaredMethod(
                "getDeclaredField", String::class.java
            )
            val hwFlagsField = getDeclaredFieldMethod.invoke(
                layoutParams.javaClass, "hwFlags"
            ) as java.lang.reflect.Field
            hwFlagsField.isAccessible = true
            val currentFlags = hwFlagsField.getInt(layoutParams)
            hwFlagsField.setInt(layoutParams, currentFlags or FLAG_NOTCH_SUPPORT)
            window.attributes = layoutParams
            return
        } catch (e: Exception) {
            // 方法1 失败，尝试方法2
        }
        
        try {
            // 方法2：使用 getDeclaredFields() 遍历所有字段（不受 blacklist 限制的旧版方式）
            val layoutParams = window.attributes
            var targetClass: Class<*>? = layoutParams.javaClass
            while (targetClass != null) {
                try {
                    val fields = targetClass.declaredFields
                    for (field in fields) {
                        if (field.name == "hwFlags") {
                            field.isAccessible = true
                            val currentFlags = field.getInt(layoutParams)
                            field.setInt(layoutParams, currentFlags or FLAG_NOTCH_SUPPORT)
                            window.attributes = layoutParams
                            return
                        }
                    }
                } catch (e: Exception) { /* skip this class level */ }
                targetClass = targetClass.superclass
            }
        } catch (e: Exception) { /* ignore */ }
        
        try {
            // 方法3：使用 Unsafe 直接写入内存（最后手段）
            val unsafeClass = Class.forName("sun.misc.Unsafe")
            val unsafeField = unsafeClass.getDeclaredField("theUnsafe")
            unsafeField.isAccessible = true
            val unsafe = unsafeField.get(null)
            
            val layoutParams = window.attributes
            val lpClass = layoutParams.javaClass
            
            // 获取 getDeclaredField 以绕过限制
            val forNameMethod = Class::class.java.getDeclaredMethod("forName", String::class.java)
            val lpFullClass = forNameMethod.invoke(null, lpClass.name) as Class<*>
            
            // 尝试获取字段偏移量
            val objectFieldOffset = unsafeClass.getDeclaredMethod("objectFieldOffset", java.lang.reflect.Field::class.java)
        } catch (e: Exception) {
            // hwFlags 设置失败，静默处理
        }
    }

    /**
     * 使用CryptoJS兼容格式加密内容
     * 参照iOS的EncryptionManager实现
     */
    private fun encryptContent(content: String, key: String): String {
        // 生成8字节随机salt
        val salt = ByteArray(8)
        java.security.SecureRandom().nextBytes(salt)

        // 使用EVP_BytesToKey算法派生key和IV
        val (derivedKey, derivedIV) = deriveKeyAndIV(key, salt)

        // AES/CBC/PKCS5Padding 加密
        val cipher = Cipher.getInstance("AES/CBC/PKCS5Padding")
        val secretKey = SecretKeySpec(derivedKey, "AES")
        val ivSpec = javax.crypto.spec.IvParameterSpec(derivedIV)
        cipher.init(Cipher.ENCRYPT_MODE, secretKey, ivSpec)
        val encryptedBytes = cipher.doFinal(content.toByteArray())

        // 构建CryptoJS格式: "Salted__" + salt + 密文
        val salted = "Salted__".toByteArray()
        val combined = salted + salt + encryptedBytes

        // Base64编码
        return android.util.Base64.encodeToString(combined, android.util.Base64.NO_WRAP)
    }
    
    /**
     * EVP_BytesToKey算法 - 与OpenSSL/CryptoJS兼容
     * 从密码和salt派生256位key和128位IV
     */
    private fun deriveKeyAndIV(password: String, salt: ByteArray): Pair<ByteArray, ByteArray> {
        val passwordBytes = password.toByteArray()
        val keySize = 32  // 256位
        val ivSize = 16   // 128位
        val totalSize = keySize + ivSize
        
        val md = MessageDigest.getInstance("MD5")
        val derivedData = ByteArrayOutputStream()
        var block = ByteArray(0)
        
        // EVP_BytesToKey算法
        while (derivedData.size() < totalSize) {
            md.reset()
            md.update(block)
            md.update(passwordBytes)
            md.update(salt)
            block = md.digest()
            derivedData.write(block)
        }
        
        val derived = derivedData.toByteArray()
        val key = derived.copyOfRange(0, keySize)
        val iv = derived.copyOfRange(keySize, keySize + ivSize)
        
        return Pair(key, iv)
    }
    
    // 辅助函数:ByteArray转16进制字符串
    private fun ByteArray.toHexString(): String {
        return joinToString("") { "%02x".format(it) }
    }
    
    /**
     * 显示设备切换下拉菜单 - Material Design 风格底部弹窗
     */
    private fun showDeviceSwitchMenu(anchor: android.view.View) {
        val devices = deviceManager.getPairedDevices()
        
        if (devices.isEmpty()) {
            // 没有配对设备时直接跳转到设置页面
            val intent = Intent(this, SettingsActivity::class.java)
            startActivity(intent)
            return
        }
        
        // 创建 BottomSheetDialog
        val bottomSheetDialog = com.google.android.material.bottomsheet.BottomSheetDialog(this)
        val dialogView = layoutInflater.inflate(R.layout.dialog_device_selector, null)
        bottomSheetDialog.setContentView(dialogView)
        
        // 设置圆角背景
        val bottomSheet = bottomSheetDialog.findViewById<android.view.View>(com.google.android.material.R.id.design_bottom_sheet)
        bottomSheet?.background = ContextCompat.getDrawable(this, R.drawable.bg_bottom_sheet)
        
        // 获取设备列表容器
        val recyclerView = dialogView.findViewById<androidx.recyclerview.widget.RecyclerView>(R.id.deviceRecyclerView)
        recyclerView.layoutManager = androidx.recyclerview.widget.LinearLayoutManager(this)
        
        // 创建设备列表适配器
        val adapter = DeviceSelectorAdapter(
            devices = devices,
            currentDeviceId = deviceId,
            onDeviceClick = { selectedDevice ->
                // 切换设备但不关闭弹窗，刷新选中状态
                if (selectedDevice.deviceId != deviceId) {
                    switchToDevice(selectedDevice)
                    // 刷新列表选中状态（显示"连接中..."）
                    deviceSelectorAdapter?.updateCurrentDevice(selectedDevice.deviceId, connecting = true)
                }
            },
            onEditClick = { device ->
                bottomSheetDialog.hide()
                showEditDeviceDialog(device) {
                    // 编辑弹层关闭后，重新显示设备选择弹层
                    bottomSheetDialog.show()
                    // 刷新列表以显示最新的设备信息
                    val updatedDevices = deviceManager.getPairedDevices()
                    deviceSelectorAdapter?.updateDevices(updatedDevices)
                }
            },
            onUnpairClick = { device ->
                bottomSheetDialog.dismiss()
                showUnpairConfirmDialog(device)
            }
        )
        recyclerView.adapter = adapter
        deviceSelectorAdapter = adapter  // 保存引用供连接状态变化时刷新
        
        // 弹窗关闭时清除引用
        bottomSheetDialog.setOnDismissListener {
            deviceSelectorAdapter = null
        }
        
        // 添加设备按钮 - 直接跳转到配对页面
        dialogView.findViewById<com.google.android.material.button.MaterialButton>(R.id.addDeviceButton).setOnClickListener {
            bottomSheetDialog.dismiss()
            val intent = Intent(this, PairingActivity::class.java)
            startActivity(intent)
        }
        
        bottomSheetDialog.show()
        
        // 查询所有设备的在线状态
        val deviceIds = devices.map { it.deviceId }
        checkDevicesOnlineStatus(deviceIds)
    }
    
    /**
     * 查询指定设备列表的在线状态
     */
    private fun checkDevicesOnlineStatus(deviceIds: List<String>) {
        if (deviceIds.isEmpty()) return
        
        Thread {
            try {
                val relayUrl = BuildConfig.RELAY_URL
                val request = okhttp3.Request.Builder().url(relayUrl).build()
                val latch = java.util.concurrent.CountDownLatch(1)
                
                client.newWebSocket(request, object : okhttp3.WebSocketListener() {
                    override fun onOpen(webSocket: okhttp3.WebSocket, response: okhttp3.Response) {
                        Log.d("MainActivity", "📡 在线状态查询: WebSocket 已连接")
                    }
                    
                    override fun onMessage(webSocket: okhttp3.WebSocket, text: String) {
                        try {
                            val json = JSONObject(text)
                            val msgType = json.optString("type", "")
                            Log.d("MainActivity", "📥 在线状态查询收到消息: $msgType")
                            
                            when (msgType) {
                                "connected" -> {
                                    // 收到欢迎消息后发送查询请求
                                    val checkMsg = JSONObject().apply {
                                        put("type", "check_online_status")
                                        put("deviceIds", org.json.JSONArray(deviceIds))
                                    }
                                    webSocket.send(checkMsg.toString())
                                    Log.d("MainActivity", "📤 发送设备在线状态查询: ${deviceIds.size}个设备")
                                }
                                "online_status_result" -> {
                                    val devicesArray = json.getJSONArray("devices")
                                    val statusMap = mutableMapOf<String, Boolean>()
                                    
                                    for (i in 0 until devicesArray.length()) {
                                        val device = devicesArray.getJSONObject(i)
                                        statusMap[device.getString("deviceId")] = device.getBoolean("online")
                                    }
                                    
                                    
                                    // 💡 如果当前设备已连接，确保它被标记为在线
                                    if (isWebSocketConnected && deviceId != null && statusMap.containsKey(deviceId)) {
                                        deviceId?.let { statusMap[it] = true }
                                        Log.d("MainActivity", "📡 当前设备已连接，标记为在线: $deviceId")
                                    }
                                    
                                    Log.d("MainActivity", "📥 设备在线状态（最终）: ${statusMap.filter { it.value }.size}/${statusMap.size} 在线, adapter=${deviceSelectorAdapter != null}")
                                    
                                    runOnUiThread {
                                        deviceSelectorAdapter?.updateOnlineStatus(statusMap)
                                    }
                                    
                                    webSocket.close(1000, "Done")
                                    latch.countDown()
                                }
                            }
                        } catch (e: Exception) {
                            Log.e("MainActivity", "❌ 解析在线状态结果失败", e)
                        }
                    }
                    
                    override fun onFailure(webSocket: okhttp3.WebSocket, t: Throwable, response: okhttp3.Response?) {
                        Log.e("MainActivity", "❌ 查询在线状态失败", t)
                        latch.countDown()
                    }
                })
                
                // 等待最多 5 秒
                latch.await(5, java.util.concurrent.TimeUnit.SECONDS)
            } catch (e: Exception) {
                Log.e("MainActivity", "❌ 查询在线状态异常", e)
            }
        }.start()
    }
    
    /**
     * 设备选择器适配器 - 支持编辑和取消配对
     */
    private inner class DeviceSelectorAdapter(
        private var devices: List<PairedDevice>,
        private var currentDeviceId: String?,
        private val onDeviceClick: (PairedDevice) -> Unit,
        private val onEditClick: (PairedDevice) -> Unit,
        private val onUnpairClick: (PairedDevice) -> Unit
    ) : androidx.recyclerview.widget.RecyclerView.Adapter<DeviceSelectorAdapter.ViewHolder>() {
        
        // 是否正在连接中（刚切换设备，WebSocket 还未建立）
        private var isConnecting = false
        
        // 设备在线状态（从中继服务器查询）
        private val deviceOnlineStatus = mutableMapOf<String, Boolean>()
        
        // 更新设备列表（编辑设备后刷新）
        fun updateDevices(newDevices: List<PairedDevice>) {
            devices = newDevices
            notifyDataSetChanged()
        }
        
        // 更新当前选中设备并刷新列表
        fun updateCurrentDevice(newDeviceId: String?, connecting: Boolean = true) {
            currentDeviceId = newDeviceId
            isConnecting = connecting
            notifyDataSetChanged()
        }
        
        // 更新连接状态（WebSocket 连接成功/失败后调用）
        fun updateConnectionStatus() {
            isConnecting = false
            notifyDataSetChanged()
        }
        
        // 更新设备在线状态（从中继服务器查询结果）
        fun updateOnlineStatus(statusMap: Map<String, Boolean>) {
            deviceOnlineStatus.clear()
            deviceOnlineStatus.putAll(statusMap)
            notifyDataSetChanged()
        }
        
        inner class ViewHolder(itemView: android.view.View) : androidx.recyclerview.widget.RecyclerView.ViewHolder(itemView) {
            val deviceIcon: android.widget.ImageView = itemView.findViewById(R.id.deviceIcon)
            val deviceName: android.widget.TextView = itemView.findViewById(R.id.deviceName)
            val deviceStatus: android.widget.TextView = itemView.findViewById(R.id.deviceStatus)
            val checkIcon: android.widget.ImageView = itemView.findViewById(R.id.checkIcon)
            val editButton: android.widget.ImageButton = itemView.findViewById(R.id.editButton)
            val moreButton: android.widget.ImageButton = itemView.findViewById(R.id.moreButton)
            val cardView: com.google.android.material.card.MaterialCardView = itemView as com.google.android.material.card.MaterialCardView
        }
        
        override fun onCreateViewHolder(parent: android.view.ViewGroup, viewType: Int): ViewHolder {
            val view = layoutInflater.inflate(R.layout.item_device, parent, false)
            return ViewHolder(view)
        }
        
        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            val device = devices[position]
            val isCurrentDevice = device.deviceId == currentDeviceId
            val isOnline = deviceOnlineStatus[device.deviceId] ?: false
            
            // 显示自定义名称（如果有）
            holder.deviceName.text = device.getDisplayName()
            // 设置状态文案
            holder.deviceStatus.text = when {
                isCurrentDevice && isConnecting -> "连接中..."
                isCurrentDevice && isWebSocketConnected -> "已连接"
                isCurrentDevice && !isWebSocketConnected -> "设备不在线"
                isOnline -> "在线 · 点击切换"
                else -> "离线"
            }
            
            // 设置文字颜色
            if (isCurrentDevice && isWebSocketConnected) {
                // 已连接：绿色
                holder.deviceStatus.setTextColor(ContextCompat.getColor(this@MainActivity, R.color.success))
            } else {
                // 其他状态：灰色
                holder.deviceStatus.setTextColor(ContextCompat.getColor(this@MainActivity, R.color.text_secondary))
            }
            // 显示自定义图标
            holder.deviceIcon.setImageResource(getIconResourceForType(device.customIcon))
            
            // 已选中状态：蓝色边框高亮（不显示对勾图标）
            if (isCurrentDevice) {
                holder.checkIcon.visibility = android.view.View.GONE
                holder.cardView.strokeColor = ContextCompat.getColor(this@MainActivity, R.color.primary)
                holder.cardView.strokeWidth = (2 * resources.displayMetrics.density).toInt()
                holder.deviceIcon.setColorFilter(ContextCompat.getColor(this@MainActivity, R.color.primary))
            } else {
                holder.checkIcon.visibility = android.view.View.GONE
                holder.cardView.strokeColor = ContextCompat.getColor(this@MainActivity, R.color.border)
                holder.cardView.strokeWidth = (1 * resources.displayMetrics.density).toInt()
                holder.deviceIcon.setColorFilter(ContextCompat.getColor(this@MainActivity, R.color.icon_default))
            }
            
            // 点击卡片切换设备
            holder.itemView.setOnClickListener {
                onDeviceClick(device)
            }
            
            // 编辑按钮
            holder.editButton.setOnClickListener {
                onEditClick(device)
            }
            
            // 更多菜单按钮 - 显示 PopupMenu
            holder.moreButton.setOnClickListener { view ->
                val popup = android.widget.PopupMenu(this@MainActivity, view)
                popup.menu.add(0, 1, 0, "取消配对")
                popup.setOnMenuItemClickListener { menuItem ->
                    when (menuItem.itemId) {
                        1 -> {
                            onUnpairClick(device)
                            true
                        }
                        else -> false
                    }
                }
                popup.show()
            }
        }
        
        override fun getItemCount() = devices.size
    }
    
    /**
     * 切换到指定设备
     */
    private fun switchToDevice(device: PairedDevice) {
        Log.d("MainActivity", "🔄 切换到设备: ${device.getDisplayName()}")
        
        // 更新当前设备
        deviceId = device.deviceId
        deviceName = device.getDisplayName()
        deviceManager.lastConnectedDeviceId = device.deviceId
        
        // 断开当前连接
        dataWebSocket?.close(1000, "Switching device")
        dataWebSocket = null
        isWebSocketConnected = false
        
        // 更新设备状态标签显示
        updateDeviceStatusDisplay()
        
        // 重新连接
        if (deviceId != null) {
            deviceId?.let { connectToRelay(it) }
        }
    }
    
    /**
     * 显示编辑设备弹窗
     * @param onDismiss 弹窗关闭时的回调，用于返回设备选择弹层
     */
    private fun showEditDeviceDialog(device: PairedDevice, onDismiss: (() -> Unit)? = null) {
        val bottomSheetDialog = com.google.android.material.bottomsheet.BottomSheetDialog(this)
        val dialogView = layoutInflater.inflate(R.layout.dialog_edit_device, null)
        bottomSheetDialog.setContentView(dialogView)
        
        // 容器设为透明，圆角由内容 View 的 bg_bottom_sheet 负责
        val bottomSheet = bottomSheetDialog.findViewById<android.view.View>(com.google.android.material.R.id.design_bottom_sheet)
        bottomSheet?.setBackgroundColor(android.graphics.Color.TRANSPARENT)

        // 获取输入框
        val editDeviceName = dialogView.findViewById<android.widget.EditText>(R.id.editDeviceName)
        editDeviceName.setText(device.customName ?: "")
        editDeviceName.hint = device.deviceName
        
        // 图标选择器
        val iconContainers = mapOf(
            "laptop" to dialogView.findViewById<android.view.View>(R.id.iconLaptop),
            "desktop" to dialogView.findViewById<android.view.View>(R.id.iconDesktop),
            "monitor" to dialogView.findViewById<android.view.View>(R.id.iconMonitor)
        )
        
        var selectedIcon = device.customIcon
        
        // 更新图标选中状态
        fun updateIconSelection() {
            iconContainers.forEach { (iconType, container) ->
                container?.isSelected = (iconType == selectedIcon)
                val imageView = (container as? android.widget.FrameLayout)?.getChildAt(0) as? android.widget.ImageView
                imageView?.setColorFilter(
                    ContextCompat.getColor(this@MainActivity, if (iconType == selectedIcon) R.color.accent else R.color.icon_default),
                    android.graphics.PorterDuff.Mode.SRC_IN
                )
            }
        }
        
        updateIconSelection()
        
        iconContainers.forEach { (iconType, container) ->
            container?.setOnClickListener {
                selectedIcon = iconType
                updateIconSelection()
            }
        }
        
        // 关闭按钮
        dialogView.findViewById<android.widget.ImageButton>(R.id.closeButton).setOnClickListener {
            bottomSheetDialog.dismiss()
            onDismiss?.invoke()
        }
        
        // 取消按钮
        dialogView.findViewById<android.widget.TextView>(R.id.cancelButton).setOnClickListener {
            bottomSheetDialog.dismiss()
            onDismiss?.invoke()
        }
        
        // 保存按钮
        dialogView.findViewById<android.widget.TextView>(R.id.saveButton).setOnClickListener {
            val customName = editDeviceName.text?.toString()?.trim()?.takeIf { it.isNotEmpty() }

            // 更新设备
            deviceManager.updateDevice(device.deviceId, customName, selectedIcon)

            // 同步备注名到服务器
            syncDeviceAliasToServer(device.deviceId, customName)

            // 如果是当前设备，更新显示
            if (device.deviceId == deviceId) {
                deviceName = customName ?: device.deviceName
                updateDeviceStatusDisplay()
            }

            bottomSheetDialog.dismiss()
            onDismiss?.invoke()
            Log.d("MainActivity", "✏️ 设备已更新: ${device.deviceId}")
        }
        
        bottomSheetDialog.show()
    }
    
    /**
     * 显示取消配对确认对话框
     */
    private fun showUnpairConfirmDialog(device: PairedDevice) {
        val dialog = androidx.appcompat.app.AlertDialog.Builder(this)
            .setTitle("取消配对")
            .setMessage("确定要取消与「${device.getDisplayName()}」的配对吗？")
            .setPositiveButton("取消配对") { _, _ ->
                unpairDevice(device)
            }
            .setNegativeButton("返回", null)
            .create()
        
        dialog.show()
        
        // 设置按钮颜色，解决主题 colorPrimary 太浅导致看不清的问题
        dialog.getButton(androidx.appcompat.app.AlertDialog.BUTTON_POSITIVE)?.setTextColor(
            ContextCompat.getColor(this, R.color.danger) // 红色，表示危险操作
        )
        dialog.getButton(androidx.appcompat.app.AlertDialog.BUTTON_NEGATIVE)?.setTextColor(
            ContextCompat.getColor(this, R.color.accent)
        )
    }
    
    /**
     * 执行取消配对操作
     */
    private fun unpairDevice(device: PairedDevice) {
        // 1. 通知中继服务器
        sendUnpairNotification(device.deviceId)
        
        // 2. 删除本地设备记录
        deviceManager.removeDevice(device.deviceId)
        
        // 3. 如果是当前设备，停止连接并切换
        if (device.deviceId == deviceId) {
            dataWebSocket?.close(1000, "Device unpaired")
            dataWebSocket = null
            isWebSocketConnected = false

            // 检查是否还有其他配对设备
            val remainingDevices = deviceManager.getPairedDevices()
            if (remainingDevices.isEmpty()) {
                // 跳转到空白页面
                val intent = Intent(this, EmptyStateActivity::class.java)
                intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                startActivity(intent)
                finish()
            } else {
                // 切换到第一个设备
                switchToDevice(remainingDevices.first())
            }
        }
        
        Log.d("MainActivity", "💔 设备已解除配对: ${device.deviceId}")
    }
    
    /**
     * 发送解除配对通知给中继服务器
     */
    private fun sendUnpairNotification(targetDeviceId: String) {
        Thread {
            try {
                val relayClient = RelayClient()
                var connected = false
                
                relayClient.connect(this) {
                    connected = true
                    relayClient.sendUnpairRequest(targetDeviceId)
                    Log.d("MainActivity", "💔 已发送解除配对通知: $targetDeviceId")
                    
                    android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                        relayClient.disconnect()
                    }, 1000)
                }
                
                var waitTime = 0
                while (!connected && waitTime < 5000) {
                    Thread.sleep(100)
                    waitTime += 100
                }
            } catch (e: Exception) {
                Log.e("MainActivity", "❌ 发送解除配对通知失败", e)
            }
        }.start()
    }

    /**
     * 同步设备备注名到服务器
     */
    private fun syncDeviceAliasToServer(targetDeviceId: String, alias: String?) {
        Thread {
            try {
                val relayClient = RelayClient()
                var connected = false

                relayClient.connect(this) {
                    connected = true
                    relayClient.setDeviceAlias(targetDeviceId, alias)
                    Log.d("MainActivity", "✏️ 已同步备注名到服务器: $targetDeviceId -> $alias")

                    android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                        relayClient.disconnect()
                    }, 1000)
                }

                var waitTime = 0
                while (!connected && waitTime < 5000) {
                    Thread.sleep(100)
                    waitTime += 100
                }
            } catch (e: Exception) {
                Log.e("MainActivity", "❌ 同步备注名失败", e)
            }
        }.start()
    }

    /**
     * 更新设备状态标签显示
     * 显示当前设备的图标、名称、连接状态（绿点），以及多设备时的下拉箭头
     */
    private fun updateDeviceStatusDisplay() {
        runOnUiThread {
            val currentDevice = deviceManager.getLastConnectedDevice()
            val devices = deviceManager.getPairedDevices()
            
            // 设置设备名称（无配对设备时显示"配对电脑"）
            deviceStatusName.text = when {
                devices.isEmpty() -> "配对电脑"
                currentDevice != null -> currentDevice.getDisplayName()
                else -> "未连接"
            }
            
            // 设置设备图标
            val iconRes = getIconResourceForType(currentDevice?.customIcon ?: "laptop")
            deviceStatusIcon.setImageResource(iconRes)
            
            // 根据配对状态设置不同样式
            if (devices.isEmpty()) {
                // 无配对设备：蓝色填充背景 + 白色图标和文字（与欢迎页面配对按钮风格一致）
                deviceStatusCard.setCardBackgroundColor(ContextCompat.getColor(this@MainActivity, R.color.primary))
                deviceStatusIcon.setColorFilter(android.graphics.Color.WHITE, android.graphics.PorterDuff.Mode.SRC_IN)
                deviceStatusName.setTextColor(android.graphics.Color.WHITE)
                deviceStatusDot.visibility = android.view.View.GONE
                deviceStatusArrow.visibility = android.view.View.GONE
            } else {
                // 有配对设备：统一灰色系 + 绿点状态指示
                val iconColor = if (isWebSocketConnected) ContextCompat.getColor(this@MainActivity, R.color.icon_default) else ContextCompat.getColor(this@MainActivity, R.color.text_secondary)
                deviceStatusIcon.setColorFilter(iconColor, android.graphics.PorterDuff.Mode.SRC_IN)
                deviceStatusCard.setCardBackgroundColor(ContextCompat.getColor(this@MainActivity, R.color.divider))
                deviceStatusName.setTextColor(ContextCompat.getColor(this@MainActivity, R.color.text_primary))
                deviceStatusDot.visibility = if (isWebSocketConnected) android.view.View.VISIBLE else android.view.View.GONE
                deviceStatusArrow.visibility = if (devices.size > 1) android.view.View.VISIBLE else android.view.View.GONE
            }
        }
    }
    
    /**
     * 根据图标类型获取对应的资源ID
     */
    private fun getIconResourceForType(iconType: String): Int {
        return when (iconType) {
            "laptop" -> R.drawable.ic_device_laptop
            "desktop" -> R.drawable.ic_device_desktop
            "imac" -> R.drawable.ic_device_imac
            "macmini" -> R.drawable.ic_device_macmini
            "monitor" -> R.drawable.ic_device_monitor
            "server" -> R.drawable.ic_device_server
            else -> R.drawable.ic_device_laptop
        }
    }
    
    private fun updateInputHint(hint: String) {
        // 不再设置hint，已使用居中的渐变色引导文字代替
    }
    
    /**
     * 处理解除配对通知
     */
    private fun handleUnpairNotification(webSocket: WebSocket?) {
        runOnUiThread {
            Log.d("MainActivity", "💔 开始处理解除配对...")
            
            // 清除配对信息
            getSharedPreferences("NextypeDevices", Context.MODE_PRIVATE).edit().clear().apply()
            
            // 清除 PairedDeviceManager 中的数据
            val dm = PairedDeviceManager(this@MainActivity)
            deviceId?.let { dm.removeDevice(it) }
            
            deviceId = null
            deviceName = null
            
            // 断开连接
            dataWebSocket?.close(1000, "Unpaired")
            dataWebSocket = null
            isWebSocketConnected = false

            // 停止自动重连任务
            connectionCheckTimer?.cancel()
            isSmartSwitching = false

            // 委托给 Application 统一处理
            NextypeApplication.instance.handleUnpairNotification()
        }
    }
    
    override fun onResume() {
        super.onResume()
        
        val isWarmStart = isInBackground
        isInBackground = false
        dataReconnectAttempts = 0
        hasAutoSwitchedThisResume = false  // 重置防重复标志
        
        Log.d("MainActivity", "▶️ onResume: isWarmStart=$isWarmStart")
        
        // 💡 后台全断方案：热启动时统一重建所有连接
        if (isWarmStart && deviceId != null) {
            Log.i("MainActivity", "▶️ 热启动：重建所有连接")
            // 立即重连数据通道和启动监控
            if (!isWebSocketConnected && deviceId != null) {
                deviceId?.let { connectToRelay(it) }
            }
            startConnectionMonitoring()
            // 4. 超时兜底：如果同步通道 5 秒内没连上，也尝试一次自动切换
            android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                if (!hasAutoSwitchedThisResume) {
                    Log.w("MainActivity", "▶️ 同步通道 5 秒内未就绪，兜底执行自动切换")
                    autoSwitchToActiveDevice()
                }
            }, 5000)
        }
        
        // 刷新设备状态标签（用户可能在设置中修改了设备名称/图标）
        updateDeviceStatusDisplay()
        
        // 刷新剪贴板同步设置
        loadClipboardSettings()
        
        // 更新按钮顺序（根据惯用手设置）
        updateButtonOrder()
        
        // 应用输入框字号设置
        applyInputFontSize()
        
        // 重新加载屏幕常亮/变暗设置（用户可能在设置中修改了）
        loadScreenSettings()
        applyKeepScreenOn()
        // 恢复正常亮度并重启倒计时
        if (isDimmed) {
            wakeUpScreen()
        } else {
            resetDimTimer()
        }
        
        // 检查配对信息是否有变更（例如在设置中取消了配对）
        val pairedDevices = deviceManager.getPairedDevices()
        val currentDeviceStillPaired = deviceId != null && pairedDevices.any { it.deviceId == deviceId }
        
        if (deviceId == null) {
            // 首次加载，使用上次连接的设备
            val lastDevice = deviceManager.getLastConnectedDevice()
            if (lastDevice != null) {
                Log.d("MainActivity", "♻️ 首次加载，使用上次连接的设备: ${lastDevice.deviceId}")
                deviceId = lastDevice.deviceId
                deviceName = lastDevice.deviceName ?: "未知设备"
                deviceId?.let { connectToRelay(it) }
                startConnectionMonitoring()
            }
        } else if (!currentDeviceStillPaired) {
            // 当前设备已被解除配对
            Log.d("MainActivity", "💔 当前设备已被解除配对: $deviceId")
            dataWebSocket?.close(1000, "Device unpaired")
            dataWebSocket = null
            
            if (pairedDevices.isEmpty()) {
                Log.d("MainActivity", "💔 没有配对设备，跳转到空白页面")
                stopConnectionMonitoring()
                val intent = Intent(this, EmptyStateActivity::class.java)
                intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                startActivity(intent)
                finish()
                return
            } else {
                val newDevice = pairedDevices.first()
                Log.d("MainActivity", "🔄 切换到其他设备: ${newDevice.deviceId}")
                deviceId = newDevice.deviceId
                deviceName = newDevice.getDisplayName()
                deviceManager.lastConnectedDeviceId = newDevice.deviceId
                deviceId?.let { connectToRelay(it) }
                startConnectionMonitoring()
            }
        }
        
        // 从后台恢复时自动弹出键盘
        // 延迟 500ms：避开系统关闭键盘 → OnGlobalLayoutListener → clearFocus 的时序干扰
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            inputEditText.requestFocus()
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            imm.showSoftInput(inputEditText, android.view.inputmethod.InputMethodManager.SHOW_FORCED)
        }, 500)
    }

    override fun onStop() {
        super.onStop()
        isInBackground = true
        Log.d("MainActivity", "💤 应用进入后台，断开所有连接")
        // 停止监控定时器
        stopConnectionMonitoring()
        // 停止变暗倒计时
        stopDimTimer()
        // 💡 后台全断：断开所有 WebSocket 连接，实现零后台耗电
        stopHeartbeat()
        dataWebSocket?.close(1000, "进入后台")
        dataWebSocket = null
        isWebSocketConnected = false
        Log.d("MainActivity", "💤 WebSocket 连接已断开")
    }

    override fun onDestroy() {
        super.onDestroy()
        cancelPendingReconnect()
        connectionCheckTimer?.cancel()
        dataWebSocket?.close(1000, "Activity destroyed")
        // 清理变暗相关资源
        stopDimTimer()
        brightnessAnimator?.cancel()
        dimHandler = null
        // 清理全局触摸回调，避免内存泄漏
        NextypeAccessibilityService.onScreenTouched = null
    }
    
    // MARK: - 智能连接和自动切换
    
    /**
 * 检查在线设备并自动切换
 * 启动时调用，查询中继服务器获取在线 PC 列表
 * 💡 修改：不再自动切换设备，尊重用户的上次选择
 * @param onComplete 检查完成后的回调
 */
private fun checkOnlineAndAutoSwitch(onComplete: () -> Unit) {
    val pairedDevices = deviceManager.getPairedDevices()
    
    // 如果没有配对设备，直接跳过
    if (pairedDevices.isEmpty()) {
        Log.d("MainActivity", "🔄 没有配对设备，跳过在线检查")
        onComplete()
        return
    }
    
    // 如果只有一个配对设备，直接使用
    if (pairedDevices.size == 1) {
        Log.d("MainActivity", "🔄 只有一个配对设备，直接使用")
        onComplete()
        return
    }
    
    // 💡 有多个配对设备时，使用上次选择的设备，不自动切换
    Log.d("MainActivity", "🔄 有多个配对设备，使用上次选择的设备: $deviceId")
    onComplete()
}
    
    
    /**
     * 启动定时连接检查
     */
    private fun startConnectionMonitoring() {
        connectionCheckTimer?.cancel()
        connectionCheckTimer = java.util.Timer()
        connectionCheckTimer?.scheduleAtFixedRate(object : java.util.TimerTask() {
            override fun run() {
                checkAndSwitch()
            }
        }, 30000, 60000) // 💡 优化：放缓检查频率（1分钟一次），优先信任 WebSocket 自身的重连机制
        Log.d("MainActivity", "✅ 定时监控已启动（每60秒安全兜底）")
    }

    /**
     * 停止连接监控
     */
    private fun stopConnectionMonitoring() {
        connectionCheckTimer?.cancel()
        connectionCheckTimer = null
        Log.d("MainActivity", "🛑 定时监控已停止")
    }
    
    /**
     * 检查并自动重连
     */
    private fun checkAndSwitch() {
        if (isSmartSwitching) {
            Log.d("MainActivity", "⏭️ 正在切换中，跳过本次检查")
            return
        }
        
        // 如果连接已断开，主动重连
        if (!isWebSocketConnected && deviceId != null) {
            Log.d("MainActivity", "🔄 检测到连接断开，尝试重新连接")
            reconnect()
        }
    }
    
    /**
     * 重新连接
     */
    private fun reconnect() {
        isSmartSwitching = true
        
        runOnUiThread {
            // 断开当前连接
            dataWebSocket?.close(1000, "Reconnecting")
            dataWebSocket = null
            
            // 重新连接到中继服务器
            if (deviceId != null) {
                deviceId?.let { connectToRelay(it) }
            }
            
            // 延迟重置切换标志
            android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                isSmartSwitching = false
            }, 2000)
        }
    }
    
    /**
     * 根据惯用手设置更新按钮顺序
     * 左手模式：发送 - 插入 - 清空（发送在左侧，方便左手大拇指点击）
     * 右手模式：清空 - 插入 - 发送（发送在右侧，方便右手大拇指点击）
     */
    private fun updateButtonOrder() {
        val isRightHand = SettingsActivity.isRightHandMode(this)
        
        // 先移除所有按钮
        buttonsContainer.removeAllViews()
        
        // 根据惯用手重新添加按钮
        if (isRightHand) {
            // 右手模式：清空 - 插入 - 发送（默认顺序）
            buttonsContainer.addView(clearButton)
            buttonsContainer.addView(syncButton)
            buttonsContainer.addView(sendButton)
        } else {
            // 左手模式：发送 - 插入 - 清空
            buttonsContainer.addView(sendButton)
            buttonsContainer.addView(syncButton)
            buttonsContainer.addView(clearButton)
        }
        
        // 更新按钮的 margin（第一个和第二个按钮需要右边距，最后一个不需要）
        updateButtonMargins(isRightHand)
        
        Log.d("MainActivity", "👋 按钮顺序已更新: ${if (isRightHand) "右手模式" else "左手模式"}")
    }
    
    /**
     * 更新按钮的 margin，确保间距正确
     */
    private fun updateButtonMargins(isRightHand: Boolean) {
        val density = resources.displayMetrics.density
        val marginEnd = (8 * density).toInt()
        
        // 获取当前顺序的按钮
        val button1 = buttonsContainer.getChildAt(0) as? com.google.android.material.button.MaterialButton
        val button2 = buttonsContainer.getChildAt(1) as? com.google.android.material.button.MaterialButton
        val button3 = buttonsContainer.getChildAt(2) as? com.google.android.material.button.MaterialButton
        
        // 第一个和第二个按钮有右边距，第三个没有
        button1?.apply {
            val params = layoutParams as android.widget.LinearLayout.LayoutParams
            params.marginEnd = marginEnd
            layoutParams = params
        }
        button2?.apply {
            val params = layoutParams as android.widget.LinearLayout.LayoutParams
            params.marginEnd = marginEnd
            layoutParams = params
        }
        button3?.apply {
            val params = layoutParams as android.widget.LinearLayout.LayoutParams
            params.marginEnd = 0
            layoutParams = params
        }
    }
    
    /**
     * 应用输入框字号设置
     */
    private fun applyInputFontSize() {
        val fontSize = SettingsActivity.getInputFontSize(this)
        inputEditText.setTextSize(TypedValue.COMPLEX_UNIT_SP, fontSize)
        Log.d("MainActivity", "✏️ 输入框字号已设置为: ${fontSize}sp")
    }
    /**
     * 加载剪贴板同步设置
     */
    private fun loadClipboardSettings() {
        pasteCopiesToClipboard = SettingsActivity.getPasteCopiesToClipboard(this)
        pasteEnterCopiesToClipboard = SettingsActivity.getPasteEnterCopiesToClipboard(this)
        Log.d("MainActivity", "📋 剪贴板设置已更新: 同步=$pasteCopiesToClipboard, 发送=$pasteEnterCopiesToClipboard")
    }
    
    
    /**
     * 检查是否已显示过该按钮的上滑提示
     */
    private fun hasShownSwipeHint(): Boolean {
        val prefs = getSharedPreferences(PREFS_SWIPE_HINT, Context.MODE_PRIVATE)
        return prefs.getBoolean("hintShown_global", false)
    }
    
    /**
     * 标记已显示过上滑提示（全局只显示一次）
     */
    private fun markSwipeHintShown() {
        val prefs = getSharedPreferences(PREFS_SWIPE_HINT, Context.MODE_PRIVATE)
        prefs.edit().putBoolean("hintShown_global", true).apply()
    }
    
    // MARK: - 屏幕常亮 & 自动变暗
    
    /**
     * 从 SharedPreferences 加载屏幕常亮/变暗相关设置
     */
    private fun loadScreenSettings() {
        isKeepScreenOn = SettingsActivity.getKeepScreenOn(this)
        isAutoDimEnabled = SettingsActivity.getAutoDimEnabled(this)
        autoDimTimeoutMs = SettingsActivity.getAutoDimTimeout(this).toLong()
        Log.d("MainActivity", "🔆 屏幕设置: 常亮=$isKeepScreenOn, 自动变暗=$isAutoDimEnabled, 超时=${autoDimTimeoutMs}ms")
    }
    
    /**
     * 根据开关状态添加或移除 FLAG_KEEP_SCREEN_ON
     */
    private fun applyKeepScreenOn() {
        if (isKeepScreenOn) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            Log.d("MainActivity", "🔆 已启用屏幕常亮")
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            // 关闭常亮时，恢复正常亮度并停止倒计时
            if (isDimmed) {
                val lp = window.attributes
                lp.screenBrightness = -1.0f
                window.attributes = lp
                isDimmed = false
            }
            stopDimTimer()
            Log.d("MainActivity", "🔆 已关闭屏幕常亮")
        }
    }
    
    /**
     * 重置变暗倒计时：取消旧任务并重新开始
     */
    private fun resetDimTimer() {
        stopDimTimer()
        if (!isKeepScreenOn || !isAutoDimEnabled) return

        dimHandler = android.os.Handler(android.os.Looper.getMainLooper())
        dimRunnable = Runnable {
            Log.d("MainActivity", "🌙 倒计时结束，执行变暗")
            dimScreen()
        }
        dimHandler?.postDelayed(dimRunnable!!, autoDimTimeoutMs)
        Log.d("MainActivity", "🔆 变暗倒计时已重置: ${autoDimTimeoutMs}ms")
    }
    
    /**
     * 停止变暗倒计时
     */
    private fun stopDimTimer() {
        dimRunnable?.let { dimHandler?.removeCallbacks(it) }
        dimRunnable = null
    }
    
    /**
     * 执行屏幕变暗动画（300ms 平滑过渡到最低亮度）
     */
    private fun dimScreen() {
        if (isDimmed) return
        isDimmed = true

        brightnessAnimator?.cancel()

        val lp = window.attributes
        // 获取起始亮度：如果是系统默认(-1)，则获取系统实际亮度值，避免闪亮
        val startBrightness = if (lp.screenBrightness < 0) {
            try {
                android.provider.Settings.System.getInt(
                    contentResolver,
                    android.provider.Settings.System.SCREEN_BRIGHTNESS,
                    128
                ) / 255f
            } catch (e: Exception) {
                0.5f // 获取失败时使用中等亮度作为起点
            }
        } else {
            lp.screenBrightness
        }

        brightnessAnimator = ValueAnimator.ofFloat(startBrightness, 0.01f).apply {
            duration = 300
            addUpdateListener { animation ->
                val value = animation.animatedValue as Float
                val params = window.attributes
                params.screenBrightness = value
                window.attributes = params
            }
            start()
        }

        Log.d("MainActivity", "🌙 屏幕开始变暗，起始亮度: $startBrightness")
    }
    
    /**
     * 唤醒屏幕：恢复正常亮度并重置倒计时
     * 公开方法，可被远程指令调用
     */
    fun wakeUpScreen() {
        if (!isDimmed) {
            // 未处于变暗状态时只重置倒计时
            resetDimTimer()
            return
        }
        
        isDimmed = false
        brightnessAnimator?.cancel()
        
        // 直接恢复正常亮度（-1.0f 表示系统默认亮度）
        val lp = window.attributes
        lp.screenBrightness = -1.0f
        window.attributes = lp
        
        // 重置倒计时
        resetDimTimer()
        
        Log.d("MainActivity", "🔆 屏幕已唤醒")
    }
    
    /**
     * 拦截触摸事件：变暗状态下第一次触摸只唤醒屏幕，不触发按钮（防误触）
     * 非变暗状态下正常分发并重置倒计时
     */
    override fun dispatchTouchEvent(ev: MotionEvent?): Boolean {
        if (ev?.action == MotionEvent.ACTION_DOWN) {
            if (isDimmed) {
                // 变暗状态：拦截触摸，唤醒屏幕，消费事件不下发
                Log.d("MainActivity", "🔆 触摸唤醒（防误触，事件已拦截）")
                wakeUpScreen()
                return true
            } else if (isKeepScreenOn && isAutoDimEnabled) {
                // 正常状态：用户操作，重置倒计时
                resetDimTimer()
            }
        }
        return super.dispatchTouchEvent(ev)
    }
    
    /**
     * 拦截按键事件：捕获来自软键盘的按键（退格、回车等）以触发唤醒
     */
    override fun dispatchKeyEvent(event: KeyEvent?): Boolean {
        if (event?.action == KeyEvent.ACTION_DOWN) {
            if (isDimmed) {
                Log.d("MainActivity", "🔆 按键事件触发唤醒")
                wakeUpScreen()
            } else if (isKeepScreenOn && isAutoDimEnabled) {
                resetDimTimer()
            }
        }
        return super.dispatchKeyEvent(event)
    }

    /**
     * 窗口焦点变化回调：处理折叠屏悬浮窗口等场景
     * 当用户切换到悬浮的其他应用窗口时，Activity 会失去焦点
     * 此时唤醒屏幕并暂停倒计时；重新获得焦点时重置倒计时
     */
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!isKeepScreenOn || !isAutoDimEnabled) return

        if (!hasFocus) {
            // 失去焦点：用户正在操作其他窗口，唤醒屏幕并暂停倒计时
            if (isDimmed) {
                Log.d("MainActivity", "🔆 失去焦点触发唤醒（悬浮窗口操作）")
                wakeUpScreen()
            }
            // 暂停倒计时，避免用户操作悬浮窗时屏幕又变暗
            stopDimTimer()
            Log.d("MainActivity", "🔆 窗口失去焦点，暂停变暗倒计时")
        } else {
            // 重新获得焦点：用户回到本应用，重新开始倒计时
            if (isDimmed) {
                Log.d("MainActivity", "🔆 获得焦点触发唤醒")
                wakeUpScreen()
            } else {
                resetDimTimer()
            }
            Log.d("MainActivity", "🔆 窗口获得焦点，重启变暗倒计时")
        }
    }

    /**
     * 显示上滑操作提示（全局只显示一次）
     * @param anchorView 锚点视图（按钮）
     */
    private fun showSwipeHintToast(anchorView: android.view.View) {
        // 如果已显示过，则不再显示
        if (hasShownSwipeHint()) {
            return
        }
        
        // 标记为已显示
        markSwipeHintShown()
        
        // 隐藏当前可能存在的提示
        hideSwipeHintToast()
        
        // 创建提示视图
        val hintView = layoutInflater.inflate(R.layout.toast_swipe_hint, null)
        val hintText = hintView.findViewById<TextView>(R.id.hintText)
        val closeButton = hintView.findViewById<android.widget.ImageButton>(R.id.closeButton)
        
        // 设置通用提示文案
        hintText.text = "按住按钮向上滑动，可快速重复操作"
        
        // 获取按钮区域的位置信息（清空按钮左边距到发送按钮右边距）
        val clearButtonLocation = IntArray(2)
        val sendButtonLocation = IntArray(2)
        clearButton.getLocationOnScreen(clearButtonLocation)
        sendButton.getLocationOnScreen(sendButtonLocation)
        
        val hintX = clearButtonLocation[0]  // 左边距对齐清空按钮左边距
        val hintWidth = (sendButtonLocation[0] + sendButton.width) - clearButtonLocation[0]  // 宽度到发送按钮右边距
        
        // 创建 PopupWindow（无动画，固定宽度，无阴影）
        swipeHintPopup = android.widget.PopupWindow(
            hintView,
            hintWidth,
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT,
            false
        ).apply {
            animationStyle = 0  // 无动画
        }
        
        // 关闭按钮点击事件
        closeButton.setOnClickListener {
            hideSwipeHintToast()
        }
        
        // 计算 Y 位置（在按钮上方）
        hintView.measure(
            android.view.View.MeasureSpec.makeMeasureSpec(hintWidth, android.view.View.MeasureSpec.EXACTLY),
            android.view.View.MeasureSpec.makeMeasureSpec(0, android.view.View.MeasureSpec.UNSPECIFIED)
        )
        val hintHeight = hintView.measuredHeight
        val y = clearButtonLocation[1] - hintHeight - (8 * resources.displayMetrics.density).toInt()
        
        swipeHintPopup?.showAtLocation(anchorView, android.view.Gravity.NO_GRAVITY, hintX, y)
        
        Log.d("MainActivity", "💡 显示上滑提示（全局单次）")
    }
    
    /**
     * 隐藏上滑操作提示
     */
    private fun hideSwipeHintToast() {
        swipeHintHandler?.removeCallbacksAndMessages(null)
        swipeHintHandler = null
        swipeHintPopup?.dismiss()
        swipeHintPopup = null
    }
    
    /**
     * 显示辅助控制权限未开启的气泡提示
     * 复用上滑提示的气泡样式，点击可跳转到系统辅助功能设置页
     * 10秒内不重复弹出，显示5秒后自动消失
     */
    private fun showAccessibilityHintToast() {
        // 防抖：10秒内不重复弹出
        val now = System.currentTimeMillis()
        if (now - lastAccessibilityHintTime < 10_000) {
            Log.d("MainActivity", "🔒 辅助控制提示防抖中，跳过")
            return
        }
        lastAccessibilityHintTime = now
        
        // 隐藏当前可能存在的提示
        hideAccessibilityHintToast()
        
        // 复用 toast_swipe_hint 布局
        val hintView = layoutInflater.inflate(R.layout.toast_swipe_hint, null)
        val hintText = hintView.findViewById<TextView>(R.id.hintText)
        val closeButton = hintView.findViewById<android.widget.ImageButton>(R.id.closeButton)
        
        // 设置权限提示文案
        hintText.text = "⚠️ 辅助控制权限未开启，点击前往设置"
        
        // 整个气泡可点击，跳转到辅助功能设置页
        hintView.setOnClickListener {
            try {
                val intent = android.content.Intent(android.provider.Settings.ACTION_ACCESSIBILITY_SETTINGS)
                startActivity(intent)
                Log.d("MainActivity", "🔧 跳转到辅助功能设置页")
            } catch (e: Exception) {
                Log.e("MainActivity", "❌ 跳转辅助功能设置失败", e)
            }
            hideAccessibilityHintToast()
        }
        
        // 关闭按钮
        closeButton.setOnClickListener {
            hideAccessibilityHintToast()
        }
        
        // 获取按钮区域的位置信息（与上滑提示相同位置）
        val clearButtonLocation = IntArray(2)
        val sendButtonLocation = IntArray(2)
        clearButton.getLocationOnScreen(clearButtonLocation)
        sendButton.getLocationOnScreen(sendButtonLocation)
        
        val hintX = clearButtonLocation[0]
        val hintWidth = (sendButtonLocation[0] + sendButton.width) - clearButtonLocation[0]
        
        // 创建 PopupWindow
        accessibilityHintPopup = android.widget.PopupWindow(
            hintView,
            hintWidth,
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT,
            false
        ).apply {
            animationStyle = 0
        }
        
        // 计算 Y 位置（在按钮上方）
        hintView.measure(
            android.view.View.MeasureSpec.makeMeasureSpec(hintWidth, android.view.View.MeasureSpec.EXACTLY),
            android.view.View.MeasureSpec.makeMeasureSpec(0, android.view.View.MeasureSpec.UNSPECIFIED)
        )
        val hintHeight = hintView.measuredHeight
        val y = clearButtonLocation[1] - hintHeight - (8 * resources.displayMetrics.density).toInt()
        
        accessibilityHintPopup?.showAtLocation(clearButton, android.view.Gravity.NO_GRAVITY, hintX, y)
        
        Log.d("MainActivity", "⚠️ 显示辅助控制权限提示气泡")
        
        // 5秒后自动消失
        accessibilityHintHandler = android.os.Handler(mainLooper)
        accessibilityHintHandler?.postDelayed({
            hideAccessibilityHintToast()
        }, 5000)
    }
    
    /**
     * 隐藏辅助控制权限提示
     */
    private fun hideAccessibilityHintToast() {
        accessibilityHintHandler?.removeCallbacksAndMessages(null)
        accessibilityHintHandler = null
        accessibilityHintPopup?.dismiss()
        accessibilityHintPopup = null
    }
}
