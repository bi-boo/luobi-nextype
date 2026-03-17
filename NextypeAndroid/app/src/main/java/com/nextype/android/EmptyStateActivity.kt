package com.nextype.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.button.MaterialButton

class EmptyStateActivity : AppCompatActivity() {

    private var isRecoveryChecked = false
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_empty_state)
        
        // 配对电脑按钮
        val addDeviceButton = findViewById<MaterialButton>(R.id.addDeviceButton)
        addDeviceButton.setOnClickListener {
            val intent = Intent(this, PairingActivity::class.java)
            startActivity(intent)
        }
        
        // 复制域名
        val copyUrlLayout = findViewById<LinearLayout>(R.id.copyUrlLayout)
        copyUrlLayout.setOnClickListener {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = ClipData.newPlainText("网址", "yuanfengai.cn")
            clipboard.setPrimaryClip(clip)
            Toast.makeText(this, "已复制", Toast.LENGTH_SHORT).show()
        }
        
        // 使用场景 - 跳转到 ScenarioActivity
        val scenarioButton = findViewById<android.widget.TextView>(R.id.scenarioButton)
        scenarioButton.setOnClickListener {
            val intent = Intent(this, ScenarioActivity::class.java)
            startActivity(intent)
        }

        // 稍后配对 - 跳过配对直接进入主界面
        val skipButton = findViewById<android.widget.TextView>(R.id.skipButton)
        skipButton.setOnClickListener {
            val intent = Intent(this, MainActivity::class.java)
            startActivity(intent)
            finish()
        }
        
        tryRecoverPairingFromServer()
    }
    
    override fun onResume() {
        super.onResume()
        // 检查是否已有配对设备,如果有则跳转到主界面
        if (hasPairedDevice()) {
            val intent = Intent(this, MainActivity::class.java)
            startActivity(intent)
            finish()
        }
    }
    
    private fun hasPairedDevice(): Boolean {
        return PairedDeviceManager(this).hasPairedDevices()
    }
    
    /**
     * 尝试从服务器恢复配对信息
     * 用于设备ID稳定后，卸载重装App时自动恢复配对关系
     */
    private fun tryRecoverPairingFromServer() {
        if (isRecoveryChecked) return
        isRecoveryChecked = true
        
        // 如果本地已有配对设备，无需恢复
        if (hasPairedDevice()) {
            Log.d("EmptyState", "本地已有配对设备，无需恢复")
            return
        }
        
        Log.d("EmptyState", "🔄 尝试从服务器恢复配对信息...")
        
        val relayClient = RelayClient()
        relayClient.connect(this) {
            // 连接成功后，查询信任列表
            relayClient.syncTrustList { trustedDevices ->
                runOnUiThread {
                    if (trustedDevices.isNotEmpty()) {
                        Log.d("EmptyState", "✅ 发现服务器配对记录: ${trustedDevices.size} 个设备")

                        // 恢复配对设备到本地
                        val deviceManager = PairedDeviceManager(this)
                        trustedDevices.forEach { trustDevice ->
                            val device = PairedDevice(
                                deviceId = trustDevice.id,
                                deviceName = trustDevice.name.ifEmpty { "PC 设备" },
                                host = "relay",
                                port = 8080,
                                pairedAt = System.currentTimeMillis(),
                                customName = trustDevice.customName
                            )
                            deviceManager.addDevice(device)
                            Log.d("EmptyState", "♻️ 恢复设备: ${trustDevice.id} (${trustDevice.name})")
                        }

                        // 设置上次连接设备
                        deviceManager.lastConnectedDeviceId = trustedDevices.first().id

                        // 跳转到主界面
                        Toast.makeText(
                            this,
                            "已恢复 ${trustedDevices.size} 个配对设备",
                            Toast.LENGTH_SHORT
                        ).show()

                        val intent = Intent(this, MainActivity::class.java)
                        startActivity(intent)
                        finish()
                    } else {
                        Log.d("EmptyState", "服务器无配对记录")
                    }

                    relayClient.disconnect()
                }
            }
        }
    }

}
