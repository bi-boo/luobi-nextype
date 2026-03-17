package com.nextype.android

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * 配对响应数据（中继服务器返回）
 */
data class PairingResponse(
    val deviceId: String,
    val deviceName: String,
    val port: Int,
    val encryptionKey: String,
    val ip: String
)

/**
 * 已配对设备的数据模型
 * 
 * @param deviceId 设备唯一标识符
 * @param deviceName 设备原始名称（从PC端获取）
 * @param host 设备主机地址
 * @param port 设备端口
 * @param pairedAt 配对时间戳
 * @param customName 用户自定义别名（可选，为空时使用 deviceName）
 * @param customIcon 用户自定义图标标识（默认 "laptop"）
 */
data class PairedDevice(
    val deviceId: String,
    val deviceName: String,
    val host: String,
    val port: Int,
    val pairedAt: Long,
    val encryptionKey: String = "",
    val customName: String? = null,
    val customIcon: String = "laptop"
) {
    /**
     * 获取显示名称：优先使用自定义名称，否则使用原始名称
     */
    fun getDisplayName(): String = customName ?: deviceName
    
    /**
     * 创建一个更新了自定义属性的副本
     */
    fun withCustomization(newName: String?, newIcon: String): PairedDevice {
        return copy(customName = newName, customIcon = newIcon)
    }
    
    fun toJson(): JSONObject {
        return JSONObject().apply {
            put("deviceId", deviceId)
            put("deviceName", deviceName)
            put("host", host)
            put("port", port)
            put("pairedAt", pairedAt)
            put("encryptionKey", encryptionKey)
            put("customName", customName ?: JSONObject.NULL)
            put("customIcon", customIcon)
        }
    }
    
    companion object {
        // 可用图标列表
        val AVAILABLE_ICONS = listOf(
            "laptop" to "笔记本",
            "desktop" to "台式机",
            "imac" to "一体机",
            "macmini" to "迷你主机",
            "monitor" to "显示器",
            "server" to "服务器"
        )
        
        fun fromJson(json: JSONObject): PairedDevice {
            return PairedDevice(
                deviceId = json.getString("deviceId"),
                deviceName = json.getString("deviceName"),
                host = json.getString("host"),
                port = json.getInt("port"),
                pairedAt = json.getLong("pairedAt"),
                encryptionKey = json.optString("encryptionKey", ""),
                customName = if (json.has("customName") && !json.isNull("customName"))
                    json.getString("customName") else null,
                customIcon = json.optString("customIcon", "laptop")
            )
        }
    }
}

/**
 * 配对设备管理器 - 支持多设备配对
 */
class PairedDeviceManager(private val context: Context) {
    
    init {
        // 迁移旧明文数据到加密存储（仅首次执行）
        SecurePrefsHelper.migrateFromPlaintext(context, "NextypeDevices")
    }

    private val prefs by lazy { SecurePrefsHelper.getPrefs(context, "NextypeDevices") }
    private val devicesKey = "pairedDevicesList"
    private val lastConnectedKey = "lastConnectedDeviceId"
    
    /**
     * 获取所有已配对的设备
     */
    fun getPairedDevices(): List<PairedDevice> {
        val jsonString = prefs.getString(devicesKey, null)
        
        // 如果新格式为空，尝试从旧格式迁移
        if (jsonString == null) {
            return migrateFromOldFormat()
        }
        
        return try {
            val jsonArray = JSONArray(jsonString)
            (0 until jsonArray.length()).map { i ->
                PairedDevice.fromJson(jsonArray.getJSONObject(i))
            }
        } catch (e: Exception) {
            android.util.Log.e("PairedDeviceManager", "解析设备列表失败", e)
            // 尝试从旧格式迁移
            migrateFromOldFormat()
        }
    }
    
    /**
     * 添加或更新配对设备
     */
    fun addDevice(device: PairedDevice) {
        val devices = getPairedDevices().toMutableList()
        
        // 检查是否已存在，已存在则更新
        val existingIndex = devices.indexOfFirst { it.deviceId == device.deviceId }
        if (existingIndex >= 0) {
            devices[existingIndex] = device
            android.util.Log.d("PairedDeviceManager", "🔄 更新设备: ${device.deviceName}")
        } else {
            devices.add(device)
            android.util.Log.d("PairedDeviceManager", "➕ 添加新设备: ${device.deviceName}")
        }
        
        saveDevices(devices)
    }
    
    /**
     * 移除配对设备
     */
    fun removeDevice(deviceId: String) {
        val devices = getPairedDevices().toMutableList()
        devices.removeAll { it.deviceId == deviceId }
        saveDevices(devices)
        
        // 如果移除的是上次连接的设备，清除记录
        if (lastConnectedDeviceId == deviceId) {
            lastConnectedDeviceId = null
        }
        
        android.util.Log.d("PairedDeviceManager", "➖ 移除设备: $deviceId")
    }
    
    /**
     * 更新设备的自定义属性（名称和图标）
     */
    fun updateDevice(deviceId: String, customName: String?, customIcon: String) {
        val devices = getPairedDevices().toMutableList()
        val index = devices.indexOfFirst { it.deviceId == deviceId }

        if (index >= 0) {
            devices[index] = devices[index].withCustomization(customName, customIcon)
            saveDevices(devices)
            android.util.Log.d("PairedDeviceManager", "✏️ 更新设备: $deviceId -> 名称=$customName, 图标=$customIcon")
        }
    }

    /**
     * 更新设备的原始名称（不影响用户备注名）
     */
    fun updateDeviceName(deviceId: String, newName: String) {
        val devices = getPairedDevices().toMutableList()
        val index = devices.indexOfFirst { it.deviceId == deviceId }

        if (index >= 0 && devices[index].deviceName != newName) {
            devices[index] = devices[index].copy(deviceName = newName)
            saveDevices(devices)
            android.util.Log.d("PairedDeviceManager", "🔄 更新设备名称: $deviceId -> $newName")
        }
    }

    /**
     * 上次连接的设备ID
     */
    var lastConnectedDeviceId: String?
        get() = prefs.getString(lastConnectedKey, null)
        set(value) {
            prefs.edit().putString(lastConnectedKey, value).apply()
            android.util.Log.d("PairedDeviceManager", "💾 保存上次连接设备: $value")
        }
    
    /**
     * 获取上次连接的设备，如果不存在则返回第一个配对设备
     */
    fun getLastConnectedDevice(): PairedDevice? {
        val devices = getPairedDevices()
        val lastId = lastConnectedDeviceId
        
        return if (lastId != null) {
            devices.firstOrNull { it.deviceId == lastId } ?: devices.firstOrNull()
        } else {
            devices.firstOrNull()
        }
    }
    
    /**
     * 检查是否有配对设备
     */
    fun hasPairedDevices(): Boolean {
        return getPairedDevices().isNotEmpty()
    }
    
    private fun saveDevices(devices: List<PairedDevice>) {
        val jsonArray = JSONArray()
        devices.forEach { device ->
            jsonArray.put(device.toJson())
        }
        prefs.edit().putString(devicesKey, jsonArray.toString()).apply()
        android.util.Log.d("PairedDeviceManager", "💾 已保存 ${devices.size} 个配对设备")
    }
    
    /**
     * 从旧格式（单设备）迁移到新格式（多设备列表）
     */
    private fun migrateFromOldFormat(): List<PairedDevice> {
        val oldDeviceId = prefs.getString("pairedDeviceId", null)
        val oldDeviceName = prefs.getString("pairedDeviceName", null)
        val oldHost = prefs.getString("pairedDeviceHost", null)
        val oldPort = prefs.getInt("pairedDevicePort", 0)
        val oldPairedAt = prefs.getLong("pairedAt", System.currentTimeMillis())
        
        if (oldDeviceId != null && oldDeviceName != null && oldHost != null) {
            android.util.Log.d("PairedDeviceManager", "📦 从旧格式迁移设备: $oldDeviceName")
            val device = PairedDevice(
                deviceId = oldDeviceId,
                deviceName = oldDeviceName,
                host = oldHost,
                port = oldPort,
                pairedAt = oldPairedAt
            )
            
            // 保存为新格式
            saveDevices(listOf(device))
            
            // 设置为上次连接的设备
            lastConnectedDeviceId = oldDeviceId
            
            return listOf(device)
        }
        
        return emptyList()
    }
}
