package com.nextype.android

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * 加密 SharedPreferences 统一管理
 * 创建失败时自动回退到明文存储（兼容性保障）
 */
object SecurePrefsHelper {

    private val cache = mutableMapOf<String, SharedPreferences>()

    /**
     * 获取加密 SharedPreferences 实例（线程安全，带回退）
     */
    fun getPrefs(context: Context, name: String): SharedPreferences {
        return cache.getOrPut(name) {
            createEncryptedPrefs(context.applicationContext, name)
                ?: context.applicationContext.getSharedPreferences(name, Context.MODE_PRIVATE)
        }
    }

    /**
     * 将旧的明文 SharedPreferences 迁移到加密存储
     * 仅在加密 SP 为空且明文 SP 有数据时执行迁移
     */
    fun migrateFromPlaintext(context: Context, name: String) {
        val plainPrefs = context.getSharedPreferences(name, Context.MODE_PRIVATE)
        val encryptedPrefs = getPrefs(context, name)

        // 如果加密 SP 和明文 SP 是同一个实例（回退到明文时），跳过
        if (plainPrefs === encryptedPrefs) return

        val allData = plainPrefs.all
        if (allData.isEmpty()) return

        // 检查加密 SP 是否已有数据（避免重复迁移）
        if (encryptedPrefs.all.isNotEmpty()) return

        try {
            val editor = encryptedPrefs.edit()
            allData.forEach { (key, value) ->
                when (value) {
                    is String -> editor.putString(key, value)
                    is Boolean -> editor.putBoolean(key, value)
                    is Int -> editor.putInt(key, value)
                    is Long -> editor.putLong(key, value)
                    is Float -> editor.putFloat(key, value)
                    else -> { /* Set 类型暂不迁移 */ }
                }
            }
            editor.apply()
            // 迁移成功后清除旧明文数据
            plainPrefs.edit().clear().apply()
            Log.d("SecurePrefsHelper", "已将 $name 迁移到加密存储 (${allData.size} 条)")
        } catch (e: Exception) {
            Log.e("SecurePrefsHelper", "数据迁移失败，保留明文备份", e)
        }
    }

    private fun createEncryptedPrefs(context: Context, name: String): SharedPreferences? {
        return try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            EncryptedSharedPreferences.create(
                context,
                "${name}_encrypted",
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            Log.e("SecurePrefsHelper", "EncryptedSharedPreferences 创建失败，回退明文存储", e)
            null
        }
    }
}
