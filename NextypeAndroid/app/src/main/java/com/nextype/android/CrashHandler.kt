package com.nextype.android

import android.content.Context
import android.os.Build
import android.util.Log
import androidx.core.content.pm.PackageInfoCompat
import java.io.File
import java.io.PrintWriter
import java.io.StringWriter
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * 全局崩溃收集器
 * 崩溃信息写入 filesDir/crash_logs/，下次启动时可查阅
 */
class CrashHandler(private val context: Context) : Thread.UncaughtExceptionHandler {

    private val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()
    private val crashLogDir = File(context.filesDir, "crash_logs")
    private val dateFormat = SimpleDateFormat("yyyy-MM-dd_HH-mm-ss", Locale.getDefault())

    init {
        crashLogDir.mkdirs()
    }

    companion object {
        private const val TAG = "CrashHandler"

        fun install(context: Context): CrashHandler {
            val handler = CrashHandler(context.applicationContext)
            Thread.setDefaultUncaughtExceptionHandler(handler)
            Log.d(TAG, "崩溃收集器已安装")
            return handler
        }

        fun getPendingCrashLogs(context: Context): List<File> {
            val dir = File(context.filesDir, "crash_logs")
            return dir.listFiles()?.filter { it.isFile && it.name.endsWith(".txt") } ?: emptyList()
        }
    }

    override fun uncaughtException(thread: Thread, throwable: Throwable) {
        try {
            saveCrashLog(thread, throwable)
        } catch (e: Exception) {
            Log.e(TAG, "保存崩溃日志失败", e)
        } finally {
            defaultHandler?.uncaughtException(thread, throwable)
        }
    }

    private fun saveCrashLog(thread: Thread, throwable: Throwable) {
        val timestamp = dateFormat.format(Date())
        val fileName = "crash_$timestamp.txt"
        val logFile = File(crashLogDir, fileName)

        val sw = StringWriter()
        throwable.printStackTrace(PrintWriter(sw))

        val content = buildString {
            appendLine("=== 崩溃报告 ===")
            appendLine("时间: $timestamp")
            appendLine("应用版本: ${getAppVersion()}")
            appendLine("Android 版本: ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})")
            appendLine("设备型号: ${Build.MANUFACTURER} ${Build.MODEL}")
            appendLine("线程: ${thread.name}")
            appendLine()
            appendLine("=== 异常堆栈 ===")
            appendLine(sw.toString())
        }

        logFile.writeText(content)
        Log.e(TAG, "崩溃日志已写入: $fileName")
    }

    private fun getAppVersion(): String {
        return try {
            val pInfo = context.packageManager.getPackageInfo(context.packageName, 0)
            "${pInfo.versionName} (${PackageInfoCompat.getLongVersionCode(pInfo)})"
        } catch (e: Exception) {
            "unknown"
        }
    }
}
