package com.nextype.android

import android.util.Log
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.*
import org.json.JSONObject
import kotlin.coroutines.resume

data class TrustDeviceInfo(val id: String, val name: String, val customName: String? = null)

data class OnlineDeviceInfo(
    val deviceId: String,
    val deviceName: String,
    val idleTime: Long
)

class RelayClient(private val serverUrl: String = BuildConfig.RELAY_URL) {
    
    private var webSocket: WebSocket? = null
    
    // 配置 OkHttpClient 启用心跳检测，每60秒发送ping (调松以省电)
    private val client = OkHttpClient.Builder()
        .pingInterval(60, java.util.concurrent.TimeUnit.SECONDS)
        .build()
    private var isConnected = false
    private var pairingCallback: ((Result<PairingResponse>) -> Unit)? = null
    private var trustListCallback: ((List<TrustDeviceInfo>) -> Unit)? = null  // 信任列表回调
    private var connectionCallback: (() -> Unit)? = null  // 添加连接回调
    private var onlineDevicesCallback: ((List<OnlineDeviceInfo>) -> Unit)? = null  // 在线设备回调
    
    // 服务器上线/下线回调 (deviceId, deviceName, isOnline)
    var serverStatusCallback: ((String, String, Boolean) -> Unit)? = null
    
    private var context: android.content.Context? = null
    
    // 应用层心跳，确保服务器的 lastHeartbeat 被更新
    private var heartbeatHandler: android.os.Handler? = null
    private var heartbeatRunnable: Runnable? = null
    
    fun connect(context: android.content.Context, onConnected: () -> Unit = {}) {
        this.context = context
        this.connectionCallback = onConnected  // 保存回调
        
        val request = Request.Builder()
            .url(serverUrl)
            .build()
        
        Log.d("RelayClient", "🌐 连接到中继服务器: $serverUrl")
        
        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.d("RelayClient", "✅ WebSocket 已连接")
                isConnected = true
                startHeartbeat()
                // 注意:不在这里调用回调,等待服务器发送 "connected" 消息
            }
            
            override fun onMessage(webSocket: WebSocket, text: String) {
                Log.d("RelayClient", "📥 收到消息: $text")
                handleMessage(text)
            }
            
            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e("RelayClient", "❌ WebSocket 连接失败", t)
                isConnected = false
                stopHeartbeat()
                pairingCallback?.invoke(Result.failure(Exception("连接失败: ${t.message}")))
                pairingCallback = null
            }
            
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.d("RelayClient", "👋 WebSocket 已关闭: $reason")
                isConnected = false
                stopHeartbeat()
            }
        })
    }
    
    private fun handleMessage(text: String) {
        try {
            val json = JSONObject(text)
            val type = json.getString("type")
            Log.d("RelayClient", "📨 收到其消息: $type")
            
            when (type) {
                "connected" -> {
                    Log.d("RelayClient", "✅ 已连接到中继服务器")
                    isConnected = true
                    // 先注册设备，注册成功后再调用回调
                    context?.let { register(it) }
                }
                "registered" -> {
                    Log.d("RelayClient", "✅ 注册成功")
                    // 自动同步信任列表
                    syncTrustListInternal()
                    // 注册成功后调用连接回调
                    connectionCallback?.invoke()
                    connectionCallback = null
                }
                "pairing_success" -> {
                    val server = json.getJSONObject("server")
                    val deviceId = server.getString("deviceId")
                    val deviceName = server.getString("deviceName")
                    val encryptionKey = server.optString("encryptionKey", "")

                    Log.d("RelayClient", "✅ 配对成功: $deviceName")

                    val response = PairingResponse(
                        deviceId = deviceId,
                        deviceName = deviceName,
                        port = 8080,
                        encryptionKey = encryptionKey,
                        ip = "relay"
                    )

                    pairingCallback?.invoke(Result.success(response))
                    pairingCallback = null
                }
                "pairing_error" -> {
                    val message = json.getString("message")
                    Log.e("RelayClient", "❌ 配对失败: $message")
                    pairingCallback?.invoke(Result.failure(Exception(message)))
                    pairingCallback = null
                }
                "error" -> {
                    val message = json.optString("message", "未知错误")
                    Log.e("RelayClient", "❌ 服务器错误: $message")
                    pairingCallback?.invoke(Result.failure(Exception(message)))
                    pairingCallback = null
                }
                "trust_list" -> {
                    // 收到信任列表
                    val devicesArray = json.optJSONArray("devices")
                    val trustDevices = mutableListOf<TrustDeviceInfo>()
                    if (devicesArray != null) {
                        for (i in 0 until devicesArray.length()) {
                            val deviceObj = devicesArray.optJSONObject(i)
                            if (deviceObj != null) {
                                val id = deviceObj.optString("id", "")
                                val name = deviceObj.optString("name", "")
                                val customName = if (deviceObj.has("customName") && !deviceObj.isNull("customName"))
                                    deviceObj.optString("customName") else null
                                if (id.isNotEmpty()) {
                                    trustDevices.add(TrustDeviceInfo(id, name, customName))
                                }
                            }
                        }
                    }
                    Log.d("RelayClient", "📋 收到信任列表: ${trustDevices.size} 个设备")
                    trustListCallback?.invoke(trustDevices)
                    trustListCallback = null
                }
                "server_list" -> {
                    // 收到在线服务器列表
                    val serversArray = json.optJSONArray("servers")
                    val onlineDevices = mutableListOf<OnlineDeviceInfo>()
                    if (serversArray != null) {
                        for (i in 0 until serversArray.length()) {
                            val serverObj = serversArray.optJSONObject(i)
                            if (serverObj != null) {
                                val deviceId = serverObj.optString("deviceId")
                                val deviceName = serverObj.optString("deviceName", "Unknown")
                                val idleTime = serverObj.optLong("idleTime", 999999L)
                                if (deviceId.isNotEmpty()) {
                                    onlineDevices.add(OnlineDeviceInfo(deviceId, deviceName, idleTime))
                                }
                            }
                        }
                    }
                    Log.d("RelayClient", "📋 收到在线服务器列表: ${onlineDevices.size} 个")
                    onlineDevicesCallback?.invoke(onlineDevices)
                    onlineDevicesCallback = null
                }
                "server_online" -> {
                    // PC 设备上线
                    val serverId = json.optString("serverId")
                    val serverName = json.optString("serverName", "Unknown")
                    Log.d("RelayClient", "🟢 PC 上线: $serverName ($serverId)")
                    serverStatusCallback?.invoke(serverId, serverName, true)
                }
                "server_offline" -> {
                    // PC 设备下线
                    val serverId = json.optString("serverId")
                    Log.d("RelayClient", "🔴 PC 下线: $serverId")
                    serverStatusCallback?.invoke(serverId, "", false)
                }
            }
        } catch (e: Exception) {
            Log.e("RelayClient", "❌ 解析消息失败", e)
        }
    }
    
    private fun register(context: android.content.Context) {
        val deviceId = DeviceIDManager.getInstance().getDeviceId(context)
        val deviceName = android.os.Build.MODEL
        
        val message = JSONObject().apply {
            put("type", "register")
            put("role", "client")
            put("deviceId", "${deviceId}_sync") // 💡 增加后缀，避免与主数据通道冲突
            put("deviceName", deviceName)
        }
        
        Log.d("RelayClient", "📝 发送注册请求 (设备ID: $deviceId)")
        webSocket?.send(message.toString())
    }
    
    suspend fun verifyPairingCode(code: String, context: android.content.Context): Result<PairingResponse> {
        return suspendCancellableCoroutine { continuation ->
            if (!isConnected) {
                continuation.resume(Result.failure(Exception("未连接到中继服务器")))
                return@suspendCancellableCoroutine
            }
            
            pairingCallback = { result ->
                continuation.resume(result)
            }
            
            val deviceId = DeviceIDManager.getInstance().getDeviceId(context)
            val deviceName = android.os.Build.MODEL
            
            val message = JSONObject().apply {
                put("type", "verify_code")
                put("code", code)
                put("from", deviceId)
                put("deviceName", deviceName)
            }
            
            Log.d("RelayClient", "🔢 发送配对码验证请求: $code (设备ID: $deviceId)")
            webSocket?.send(message.toString())
            
            // 10秒超时
            android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                if (pairingCallback != null) {
                    pairingCallback?.invoke(Result.failure(Exception("配对请求超时,请重试")))
                    pairingCallback = null
                }
            }, 10000)
        }
    }
    
    /**
     * 发送解除配对请求到中继服务器
     * @param targetDeviceId 要解除配对的目标设备ID（PC端）
     */
    fun sendUnpairRequest(targetDeviceId: String) {
        if (!isConnected) {
            Log.w("RelayClient", "⚠️ 无法发送解除配对请求: 未连接到中继服务器")
            return
        }
        
        val message = JSONObject().apply {
            put("type", "unpair_device")
            put("targetDeviceId", targetDeviceId)
        }
        
        Log.d("RelayClient", "💔 发送解除配对请求: $targetDeviceId")
        webSocket?.send(message.toString())
    }

    /**
     * 设置设备备注名（同步到服务器）
     * @param targetDeviceId 目标设备ID
     * @param alias 备注名，传 null 或空字符串则清除备注
     */
    fun setDeviceAlias(targetDeviceId: String, alias: String?) {
        if (!isConnected) {
            Log.w("RelayClient", "⚠️ 无法设置备注名: 未连接到中继服务器")
            return
        }

        val message = JSONObject().apply {
            put("type", "set_device_alias")
            put("targetDeviceId", targetDeviceId)
            put("alias", alias ?: JSONObject.NULL)
        }

        Log.d("RelayClient", "✏️ 发送设备备注: $targetDeviceId -> $alias")
        webSocket?.send(message.toString())
    }

    /**
     * 请求同步信任列表
     * @param callback 回调函数，返回服务器上当前有效的配对设备列表
     */
    fun syncTrustList(callback: (List<TrustDeviceInfo>) -> Unit) {
        if (!isConnected) {
            Log.w("RelayClient", "⚠️ 无法同步信任列表: 未连接到中继服务器")
            callback(emptyList())
            return
        }
        
        trustListCallback = callback
        
        val message = JSONObject().apply {
            put("type", "sync_trust_list")
        }
        
        Log.d("RelayClient", "📋 请求同步信任列表")
        webSocket?.send(message.toString())
    }
    
    /**
     * 内部同步方法：启动时自动调用，以服务器配对列表为准覆盖本地
     */
    private fun syncTrustListInternal() {
        if (!isConnected) {
            Log.w("RelayClient", "⚠️ 无法同步信任列表: 未连接到中继服务器")
            return
        }
        
        val message = JSONObject().apply {
            put("type", "sync_trust_list")
        }
        
        Log.d("RelayClient", "🔄 启动时自动同步信任列表")
        webSocket?.send(message.toString())
        
        // 设置回调处理服务器返回的信任列表 - 以服务器为准覆盖本地
        trustListCallback = { serverDevices ->
            context?.let { ctx ->
                val deviceManager = PairedDeviceManager(ctx)
                val localDevices = deviceManager.getPairedDevices()
                val serverDeviceIdSet = serverDevices.map { it.id }.toSet()

                // 1. 删除服务器上不存在的本地设备
                var removedCount = 0
                localDevices.forEach { localDevice ->
                    if (!serverDeviceIdSet.contains(localDevice.deviceId)) {
                        deviceManager.removeDevice(localDevice.deviceId)
                        Log.d("RelayClient", "➖ 移除本地过期配对: ${localDevice.getDisplayName()}")
                        removedCount++
                    }
                }

                // 2. 用服务器返回的名称更新本地设备信息
                serverDevices.forEach { trustDevice ->
                    val localDevice = localDevices.firstOrNull { it.deviceId == trustDevice.id }
                    if (localDevice != null) {
                        var needUpdate = false
                        var updatedDevice = localDevice

                        // 更新 deviceName（原始名称）
                        if (trustDevice.name.isNotEmpty() && trustDevice.name != localDevice.deviceName) {
                            updatedDevice = updatedDevice.copy(deviceName = trustDevice.name)
                            needUpdate = true
                        }

                        // 用服务器备注名恢复本地备注（如果本地没有而服务器有）
                        if (trustDevice.customName != null && localDevice.customName == null) {
                            updatedDevice = updatedDevice.copy(customName = trustDevice.customName)
                            needUpdate = true
                        }

                        if (needUpdate) {
                            deviceManager.addDevice(updatedDevice)
                            Log.d("RelayClient", "🔄 更新设备信息: ${trustDevice.id} -> name=${trustDevice.name}, customName=${trustDevice.customName}")
                        }
                    }
                }

                if (removedCount > 0) {
                    Log.d("RelayClient", "✅ 同步完成: 移除 $removedCount 个过期配对")
                } else {
                    Log.d("RelayClient", "✅ 本地与服务器配对列表一致")
                }
            }
        }
    }
    
    /**
     * 查询在线的 PC 设备列表
     * 增加等待机制：如果当前未连接，会尝试等待一小段时间看是否能连上
     * @param callback 回调函数，返回在线设备列表
     */
    fun discoverOnlineDevices(callback: (List<OnlineDeviceInfo>) -> Unit) {
        val maxWaitMs = 3000L
        val checkIntervalMs = 500L
        var waitedMs = 0L

        fun performDiscover() {
            if (!isConnected) {
                Log.w("RelayClient", "⚠️ 无法查询在线设备: 尝试等待 $maxWaitMs ms 后仍未连接")
                callback(emptyList())
                return
            }

            onlineDevicesCallback = callback
            val message = JSONObject().apply {
                put("type", "discover")
            }
            Log.d("RelayClient", "🔍 [DEBUG] 发送 DISCOVER 请求...")
            webSocket?.send(message.toString())

            // 5秒请求超时
            android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                if (onlineDevicesCallback != null) {
                    Log.w("RelayClient", "⚠️ 查询在线设备超时")
                    onlineDevicesCallback?.invoke(emptyList())
                    onlineDevicesCallback = null
                }
            }, 5000)
        }

        // 递归检查连接逻辑
        fun checkAndProceed() {
            if (isConnected) {
                if (waitedMs > 0) Log.i("RelayClient", "✅ 等待 $waitedMs ms 后连接已恢复，继续执行 DISCOVER")
                performDiscover()
            } else if (waitedMs < maxWaitMs) {
                waitedMs += checkIntervalMs
                android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                    checkAndProceed()
                }, checkIntervalMs)
            } else {
                performDiscover() // 最终尝试
            }
        }

        checkAndProceed()
    }
    
    fun disconnect() {
        stopHeartbeat()
        webSocket?.close(1000, "正常关闭")
        webSocket = null
        isConnected = false
        Log.d("RelayClient", "👋 已断开中继服务器连接")
    }
    
    /**
     * 启动应用层心跳
     * 每30秒发送一次 {type:"heartbeat"} 消息，确保服务器的 lastHeartbeat 被更新
     */
    private fun startHeartbeat() {
        stopHeartbeat()
        heartbeatHandler = android.os.Handler(android.os.Looper.getMainLooper())
        heartbeatRunnable = object : Runnable {
            override fun run() {
                if (isConnected) {
                    try {
                        webSocket?.send("""{"type":"heartbeat"}""")
                        Log.d("RelayClient", "💓 发送心跳")
                    } catch (e: Exception) {
                        Log.e("RelayClient", "💓 心跳发送失败", e)
                    }
                    heartbeatHandler?.postDelayed(this, 30000)
                }
            }
        }
        heartbeatHandler?.postDelayed(heartbeatRunnable!!, 30000)
    }
    
    /**
     * 停止心跳
     */
    private fun stopHeartbeat() {
        heartbeatRunnable?.let { heartbeatHandler?.removeCallbacks(it) }
        heartbeatHandler = null
        heartbeatRunnable = null
    }
}
