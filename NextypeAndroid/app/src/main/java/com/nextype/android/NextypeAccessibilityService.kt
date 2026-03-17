package com.nextype.android

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.accessibility.AccessibilityEvent

/**
 * 辅助功能服务 - 用于执行模拟点击和长按
 * 需要用户在系统设置中手动开启
 */
class NextypeAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "NextypeAccessibility"
        private const val HEARTBEAT_TIMEOUT_MS = 1000L // 1秒内没收到心跳则自动释放

        // 单例引用，供 MainActivity 调用
        var instance: NextypeAccessibilityService? = null
            private set

        /**
         * 全局触摸回调：屏幕上任何位置（包括键盘、悬浮窗口）的触摸都会触发
         * 由 MainActivity 设置，用于屏幕变暗后的唤醒检测
         */
        var onScreenTouched: (() -> Unit)? = null
    }

    // 当前长按的 StrokeDescription（用于 willContinue 续传）
    private var activeLongPressStroke: GestureDescription.StrokeDescription? = null
    private var longPressX: Float = 0f
    private var longPressY: Float = 0f
    private val handler = Handler(Looper.getMainLooper())
    private var lastHeartbeatTime: Long = 0L
    private val heartbeatCheckRunnable = object : Runnable {
        override fun run() {
            if (activeLongPressStroke != null) {
                val elapsed = System.currentTimeMillis() - lastHeartbeatTime
                if (elapsed > HEARTBEAT_TIMEOUT_MS) {
                    Log.w(TAG, "⏰ 心跳超时 (${elapsed}ms)，自动释放长按")
                    performTouchUp()
                } else {
                    // 继续检测
                    handler.postDelayed(this, 200L)
                }
            }
        }
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        Log.d(TAG, "✅ AccessibilityService 已连接")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event?.eventType == AccessibilityEvent.TYPE_TOUCH_INTERACTION_START) {
            // 屏幕上任意位置触摸（包括键盘、悬浮窗口），通知 MainActivity 唤醒
            handler.post {
                onScreenTouched?.invoke()
            }
        }
    }

    override fun onInterrupt() {
        Log.d(TAG, "⚠️ AccessibilityService 被中断")
    }

    override fun onDestroy() {
        super.onDestroy()
        handler.removeCallbacks(heartbeatCheckRunnable)
        instance = null
        Log.d(TAG, "🛑 AccessibilityService 已销毁")
    }

    /**
     * 执行模拟点击
     * @param x 点击的 X 坐标（屏幕像素）
     * @param y 点击的 Y 坐标（屏幕像素）
     */
    fun performTap(x: Float, y: Float) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.N) {
            Log.e(TAG, "❌ 模拟手势需要 Android 7.0 (API 24) 及以上版本")
            return
        }

        Log.d(TAG, "🎯 执行模拟点击: ($x, $y)")

        try {
            // 创建点击路径
            val path = Path().apply {
                moveTo(x, y)
            }

            // 创建手势描述（点击持续 50ms）
            val gestureBuilder = GestureDescription.Builder()
            val strokeDescription = GestureDescription.StrokeDescription(
                path,
                0,      // 开始时间
                50      // 持续时间 (ms)
            )
            gestureBuilder.addStroke(strokeDescription)

            // 执行手势
            val result = dispatchGesture(
                gestureBuilder.build(),
                object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) {
                        Log.d(TAG, "✅ 模拟点击成功: ($x, $y)")
                    }

                    override fun onCancelled(gestureDescription: GestureDescription?) {
                        Log.w(TAG, "⚠️ 模拟点击被取消: ($x, $y)")
                    }
                },
                null    // Handler
            )

            if (!result) {
                Log.e(TAG, "❌ 模拟点击请求失败: ($x, $y)")
            }

        } catch (e: Exception) {
            Log.e(TAG, "❌ 模拟点击异常: ${e.message}", e)
        }
    }

    /**
     * 执行长按按下（手指按住不放）
     * @param x 按下的 X 坐标（屏幕像素）
     * @param y 按下的 Y 坐标（屏幕像素）
     */
    fun performTouchDown(x: Float, y: Float) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            Log.e(TAG, "❌ 持续长按需要 Android 8.0 (API 26) 及以上版本，降级为普通长按")
            // 降级：执行固定时长的长按
            performLegacyLongPress(x, y)
            return
        }

        // 如果已有长按进行中，先释放
        if (activeLongPressStroke != null) {
            Log.w(TAG, "⚠️ 已有长按进行中，先释放")
            performTouchUp()
        }

        Log.d(TAG, "👇 执行长按按下: ($x, $y)")
        longPressX = x
        longPressY = y

        try {
            val path = Path().apply {
                moveTo(x, y)
            }

            // 使用 willContinue=true 创建可续传的手势
            val strokeDescription = GestureDescription.StrokeDescription(
                path,
                0,      // 开始时间
                1000,   // 初始持续时间 1秒（会被续传延长）
                true    // willContinue = true，表示手势会继续
            )

            val gestureBuilder = GestureDescription.Builder()
            gestureBuilder.addStroke(strokeDescription)

            val result = dispatchGesture(
                gestureBuilder.build(),
                object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) {
                        Log.d(TAG, "✅ 长按按下成功: ($x, $y)")
                    }

                    override fun onCancelled(gestureDescription: GestureDescription?) {
                        Log.w(TAG, "⚠️ 长按按下被取消: ($x, $y)")
                        activeLongPressStroke = null
                        handler.removeCallbacks(heartbeatCheckRunnable)
                    }
                },
                null
            )

            if (result) {
                activeLongPressStroke = strokeDescription
                // 启动心跳检测
                lastHeartbeatTime = System.currentTimeMillis()
                handler.removeCallbacks(heartbeatCheckRunnable)
                handler.postDelayed(heartbeatCheckRunnable, 500L)
            } else {
                Log.e(TAG, "❌ 长按按下请求失败: ($x, $y)")
            }

        } catch (e: Exception) {
            Log.e(TAG, "❌ 长按按下异常: ${e.message}", e)
        }
    }

    /**
     * 执行长按释放（抬起手指）
     */
    fun performTouchUp() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            Log.d(TAG, "👆 Android 8.0 以下无需手动释放")
            return
        }

        val currentStroke = activeLongPressStroke
        if (currentStroke == null) {
            Log.d(TAG, "👆 没有进行中的长按，忽略释放")
            return
        }

        Log.d(TAG, "👆 执行长按释放: ($longPressX, $longPressY)")
        handler.removeCallbacks(heartbeatCheckRunnable)

        try {
            val path = Path().apply {
                moveTo(longPressX, longPressY)
            }

            // 使用 willContinue=false 结束手势
            val endStroke = currentStroke.continueStroke(
                path,
                0,
                50,     // 结束阶段持续 50ms
                false   // willContinue = false，表示手势结束
            )

            val gestureBuilder = GestureDescription.Builder()
            gestureBuilder.addStroke(endStroke)

            val result = dispatchGesture(
                gestureBuilder.build(),
                object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) {
                        Log.d(TAG, "✅ 长按释放成功: ($longPressX, $longPressY)")
                    }

                    override fun onCancelled(gestureDescription: GestureDescription?) {
                        Log.w(TAG, "⚠️ 长按释放被取消: ($longPressX, $longPressY)")
                    }
                },
                null
            )

            if (!result) {
                Log.e(TAG, "❌ 长按释放请求失败")
            }

        } catch (e: Exception) {
            Log.e(TAG, "❌ 长按释放异常: ${e.message}", e)
        } finally {
            activeLongPressStroke = null
        }
    }

    /**
     * 处理心跳消息，刷新超时计时器
     */
    fun onHeartbeat() {
        if (activeLongPressStroke != null) {
            lastHeartbeatTime = System.currentTimeMillis()
            Log.d(TAG, "💓 收到心跳")
        }
    }

    /**
     * 降级方案：执行固定时长的长按（Android 8.0 以下）
     */
    private fun performLegacyLongPress(x: Float, y: Float) {
        Log.d(TAG, "🎯 执行固定长按 (降级): ($x, $y)")

        try {
            val path = Path().apply {
                moveTo(x, y)
            }

            // 固定 800ms 长按
            val gestureBuilder = GestureDescription.Builder()
            val strokeDescription = GestureDescription.StrokeDescription(
                path,
                0,
                800
            )
            gestureBuilder.addStroke(strokeDescription)

            dispatchGesture(
                gestureBuilder.build(),
                object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) {
                        Log.d(TAG, "✅ 固定长按成功: ($x, $y)")
                    }

                    override fun onCancelled(gestureDescription: GestureDescription?) {
                        Log.w(TAG, "⚠️ 固定长按被取消: ($x, $y)")
                    }
                },
                null
            )

        } catch (e: Exception) {
            Log.e(TAG, "❌ 固定长按异常: ${e.message}", e)
        }
    }
}
