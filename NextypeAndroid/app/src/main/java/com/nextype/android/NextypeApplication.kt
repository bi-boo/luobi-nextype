package com.nextype.android

import android.app.Application
import android.content.Intent
import android.util.Log

/**
 * 应用级 Application 类
 * 优化后：移除了独立的控制通道，所有消息统一通过 MainActivity 的 dataWebSocket 处理
 */
class NextypeApplication : Application() {

    companion object {
        private const val TAG = "NextypeApp"

        // 单例访问
        lateinit var instance: NextypeApplication
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        // 安装全局崩溃收集器
        CrashHandler.install(this)
        Log.d(TAG, "🚀 NextypeApplication 启动")
    }

    /**
     * 处理解除配对通知（从 MainActivity 调用）
     * 清除本地配对信息并跳转到空白页面
     */
    fun handleUnpairNotification() {
        Log.d(TAG, "💔 开始处理解除配对...")

        // 在主线程执行
        android.os.Handler(android.os.Looper.getMainLooper()).post {
            // 1. 清除配对信息
            getSharedPreferences("NextypeDevices", MODE_PRIVATE).edit().clear().apply()

            // 2. 清除 PairedDeviceManager 中的数据
            val deviceManager = PairedDeviceManager(this)
            deviceManager.getPairedDevices().forEach { device ->
                deviceManager.removeDevice(device.deviceId)
            }

            // 3. 跳转到空白页面（使用 FLAG_ACTIVITY_NEW_TASK 因为从 Application 启动）
            val intent = Intent(this, EmptyStateActivity::class.java)
            intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            startActivity(intent)

            Log.d(TAG, "💔 解除配对处理完成，已跳转到空白页面")
        }
    }
}
