package com.nextype.android

import android.content.Context
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.widget.EditText
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class PairingActivity : AppCompatActivity() {
    
    private lateinit var code1: EditText
    private lateinit var code2: EditText
    private lateinit var code3: EditText
    private lateinit var code4: EditText
    private lateinit var pairingProgress: LinearLayout
    private lateinit var errorMessage: TextView
    private var relayClient: RelayClient? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_pairing)
        
        // 返回按钮
        findViewById<ImageButton>(R.id.backButton).setOnClickListener {
            finish()
        }
        
        // 初始化视图
        code1 = findViewById(R.id.code1)
        code2 = findViewById(R.id.code2)
        code3 = findViewById(R.id.code3)
        code4 = findViewById(R.id.code4)
        pairingProgress = findViewById(R.id.pairingProgress)
        errorMessage = findViewById(R.id.errorMessage)
        
        // 设置光标颜色为蓝色
        setCursorColor(code1)
        setCursorColor(code2)
        setCursorColor(code3)
        setCursorColor(code4)
        
        // 禁用自动填充（包括厂商的短信验证码助手）
        disableAutofill(code1)
        disableAutofill(code2)
        disableAutofill(code3)
        disableAutofill(code4)
        
        // 设置自动跳转
        setupAutoFocus()
        
        // 第一个输入框自动获取焦点
        code1.requestFocus()
    }
    
    /**
     * 设置输入框光标颜色为蓝色
     */
    private fun setCursorColor(editText: EditText) {
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) {
            // Android 10+ 使用 textCursorDrawable
            editText.textCursorDrawable = getDrawable(R.drawable.cursor_blue)
        } else {
            // 旧版本通过反射设置
            try {
                val cursorDrawableRes = EditText::class.java.getDeclaredField("mCursorDrawableRes")
                cursorDrawableRes.isAccessible = true
                cursorDrawableRes.setInt(editText, R.drawable.cursor_blue)
            } catch (e: Exception) {
                // 忽略反射失败
            }
        }
    }
    
    /**
     * 禁用输入框的自动填充功能
     * 这可以防止系统和厂商的短信验证码助手弹窗
     */
    private fun disableAutofill(editText: EditText) {
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
            editText.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
            editText.setAutofillHints(null)
        }
        // 通过设置私有 IME 选项来告诉输入法不要提供验证码建议
        editText.privateImeOptions = "nm,com.google.android.inputmethod.latin.noMicrophoneKey,disableSticker,disableGifSearch,oneTimeCode=false"
        // 设置内容类型，明确告知这不是验证码
        editText.imeOptions = android.view.inputmethod.EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
    }
    
    private fun setupAutoFocus() {
        val codeFields = listOf(code1, code2, code3, code4)
        
        // 为每个输入框设置监听器
        codeFields.forEachIndexed { index, editText ->
            // 文本变化监听
            editText.addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
                override fun afterTextChanged(s: Editable?) {
                    // 有内容时隐藏光标，无内容时显示光标
                    editText.isCursorVisible = s.isNullOrEmpty()
                    
                    // 文本变化时隐藏错误提示
                    if (errorMessage.visibility == View.VISIBLE) {
                        errorMessage.visibility = View.GONE
                    }
                    
                    if (s?.length == 1) {
                        // 输入了一个字符，跳到下一个输入框
                        if (index < 3) {
                            codeFields[index + 1].requestFocus()
                        }
                        
                        // 检查是否所有格子都已填入，触发配对
                        if (codeFields.all { it.text?.length == 1 }) {
                            startPairing()
                        }
                    }
                }
            })
            
            // 删除键监听 - 处理空格子的删除回退
            editText.setOnKeyListener { _, keyCode, event ->
                if (keyCode == android.view.KeyEvent.KEYCODE_DEL && 
                    event.action == android.view.KeyEvent.ACTION_DOWN) {
                    
                    // 隐藏错误提示
                    if (errorMessage.visibility == View.VISIBLE) {
                        errorMessage.visibility = View.GONE
                    }
                    
                    if (editText.text.isNullOrEmpty() && index > 0) {
                        // 当前格子为空，删除键回退到前一个格子
                        val prevField = codeFields[index - 1]
                        prevField.requestFocus()
                        // 删除前一个格子的内容
                        prevField.text?.clear()
                        return@setOnKeyListener true
                    } else if (!editText.text.isNullOrEmpty()) {
                        // 当前格子有内容，清空当前格子并回退
                        editText.text?.clear()
                        if (index > 0) {
                            codeFields[index - 1].requestFocus()
                        }
                        return@setOnKeyListener true
                    }
                }
                false
            }
            
            // 点击时恢复光标显示（如果为空）
            editText.setOnFocusChangeListener { _, hasFocus ->
                if (hasFocus) {
                    editText.isCursorVisible = editText.text.isNullOrEmpty()
                    // 如果有内容，光标放到末尾
                    editText.setSelection(editText.text?.length ?: 0)
                }
            }
            
            // 点击输入框时，如果有内容则禁用选择
            editText.setOnClickListener {
                if (!editText.text.isNullOrEmpty()) {
                    editText.isCursorVisible = false
                    editText.setSelection(editText.text?.length ?: 0)
                }
            }
        }
    }
    
    private fun startPairing() {
        val pairingCode = "${code1.text}${code2.text}${code3.text}${code4.text}"
        
        android.util.Log.d("Pairing", "=== 开始配对流程(公网) ===")
        android.util.Log.d("Pairing", "配对码: $pairingCode")
        
        // 显示配对中
        pairingProgress.visibility = View.VISIBLE
        errorMessage.visibility = View.GONE
        
        // 使用协程执行配对
        lifecycleScope.launch {
            try {
                // 使用 RelayClient 进行公网配对(与 iOS 一致)
                android.util.Log.d("Pairing", "🌐 开始公网中继配对...")
                
                val client = RelayClient()
                relayClient = client

                // 等待连接建立(最多5秒)
                var connected = false
                client.connect(this@PairingActivity) {
                    connected = true
                }

                // 等待连接
                var waitTime = 0
                while (!connected && waitTime < 5000) {
                    kotlinx.coroutines.delay(100)
                    waitTime += 100
                }

                if (!connected) {
                    android.util.Log.e("Pairing", "❌ 连接中继服务器超时")
                    showError("网络连接失败，请检查网络")
                    return@launch
                }

                android.util.Log.d("Pairing", "✅ 已连接到中继服务器,开始验证配对码...")

                // 验证配对码
                val result = client.verifyPairingCode(pairingCode, this@PairingActivity)

                if (result.isSuccess) {
                    val response = result.getOrNull()!!
                    android.util.Log.d("Pairing", "✅ 公网配对成功: ${response.deviceName}")

                    // 断开中继连接
                    client.disconnect()

                    handlePairingSuccess(response)
                } else {
                    val error = result.exceptionOrNull()?.message ?: "未知错误"
                    android.util.Log.e("Pairing", "❌ 配对失败: $error")

                    // 断开中继连接
                    client.disconnect()

                    showError("配对码无效，请确认电脑端已生成配对码")
                }
                
            } catch (e: Exception) {
                android.util.Log.e("Pairing", "❌ 配对异常", e)
                showError("配对失败，请重试")
            }
        }
    }
    
    private fun handlePairingSuccess(response: PairingResponse) {
        // 保存设备信息
        savePairedDevice(response.deviceId, response.deviceName, response.ip, response.port, response.encryptionKey)
        
        // 隐藏进度
        pairingProgress.visibility = View.GONE
        
        // 显示成功提示并返回
        android.widget.Toast.makeText(this, "配对成功", android.widget.Toast.LENGTH_SHORT).show()
        
        // 延迟返回,让用户看到提示
        android.os.Handler(mainLooper).postDelayed({
            finish()
        }, 500)
    }
    
    private fun showError(message: String) {
        pairingProgress.visibility = View.GONE
        errorMessage.text = message
        errorMessage.visibility = View.VISIBLE
    }
    
    override fun onDestroy() {
        super.onDestroy()
        relayClient?.disconnect()
    }

    private fun savePairedDevice(deviceId: String, deviceName: String, host: String, port: Int, encryptionKey: String = "") {
        val manager = PairedDeviceManager(this)
        val device = PairedDevice(
            deviceId = deviceId,
            deviceName = deviceName,
            host = host,
            port = port,
            pairedAt = System.currentTimeMillis(),
            encryptionKey = encryptionKey
        )
        manager.addDevice(device)
        // 将新配对的设备设置为上次连接的设备
        manager.lastConnectedDeviceId = deviceId
    }
}
