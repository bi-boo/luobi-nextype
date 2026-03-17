package com.nextype.android

import android.content.Context
import android.provider.Settings
import android.util.Log
import java.security.MessageDigest

/**
 * 设备ID管理器
 * 
 * 使用 ANDROID_ID 生成稳定的设备标识，确保：
 * 1. 卸载重装后设备ID保持不变
 * 2. 覆盖安装后设备ID保持不变
 * 3. 恢复出厂设置后设备ID会改变（这是合理的）
 * 
 * 设备ID = MD5(ANDROID_ID + "nextype")[:16]
 * 使用16位十六进制字符串，既简短又有足够的唯一性
 */
class DeviceIDManager private constructor() {
    
    companion object {
        private const val PREFS_NAME = "NextypeDeviceID"
        private const val KEY_DEVICE_ID = "nextype_device_id"
        private const val KEY_LEGACY_MIGRATED = "legacy_id_migrated"
        
        @Volatile
        private var instance: DeviceIDManager? = null
        
        fun getInstance(): DeviceIDManager {
            return instance ?: synchronized(this) {
                instance ?: DeviceIDManager().also { instance = it }
            }
        }
    }
    
    private var cachedDeviceId: String? = null
    
    /**
     * 获取唯一的设备ID
     * 
     * 优先级：
     * 1. 内存缓存
     * 2. SharedPreferences 中已保存的 ID（向后兼容）
     * 3. 基于 ANDROID_ID 生成新的稳定 ID
     */
    fun getDeviceId(context: Context): String {
        // 从缓存返回
        cachedDeviceId?.let { return it }
        
        val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        
        // 检查是否有已保存的 ID（向后兼容旧版本）
        val storedId = prefs.getString(KEY_DEVICE_ID, null)
        if (storedId != null) {
            cachedDeviceId = storedId
            Log.d("DeviceIDManager", "📱 从存储读取设备ID: $storedId")
            return storedId
        }
        
        // 使用 ANDROID_ID 生成稳定的设备 ID
        val androidId = Settings.Secure.getString(context.contentResolver, Settings.Secure.ANDROID_ID)
        val stableId = generateStableId(androidId)
        
        // 保存到本地（作为缓存加速，同时保持向后兼容）
        prefs.edit().putString(KEY_DEVICE_ID, stableId).apply()
        cachedDeviceId = stableId
        
        Log.d("DeviceIDManager", "🆕 生成稳定设备ID: $stableId (基于 ANDROID_ID)")
        return stableId
    }
    
    /**
     * 基于 ANDROID_ID 生成稳定的设备 ID
     * 使用 MD5 哈希确保唯一性和固定长度
     */
    private fun generateStableId(androidId: String?): String {
        val input = (androidId ?: "unknown") + "nextype_salt_v1"
        
        return try {
            val md = MessageDigest.getInstance("MD5")
            val digest = md.digest(input.toByteArray())
            // 取前16个字符作为设备ID
            digest.joinToString("") { "%02x".format(it) }.take(16)
        } catch (e: Exception) {
            Log.e("DeviceIDManager", "MD5 计算失败，使用 ANDROID_ID", e)
            androidId?.take(16) ?: "fallback_id_0000"
        }
    }
    
    /**
     * 重置设备ID（仅用于调试）
     * 注意：这会导致需要重新配对
     */
    fun resetDeviceId(context: Context) {
        val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        prefs.edit().remove(KEY_DEVICE_ID).apply()
        cachedDeviceId = null
        Log.d("DeviceIDManager", "🔄 已重置设备ID")
    }
}

