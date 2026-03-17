package com.nextype.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.button.MaterialButton

/**
 * 产品说明页面
 * 展示产品介绍、使用步骤、功能说明、电脑端设置和隐私保护
 */
class AboutActivity : AppCompatActivity() {
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_about)
        
        // 返回按钮
        val backButton = findViewById<ImageButton>(R.id.backButton)
        backButton.setOnClickListener {
            finish()
        }
        
        // 复制域名
        val copyUrlLayout = findViewById<LinearLayout>(R.id.copyUrlLayout)
        copyUrlLayout.setOnClickListener {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = ClipData.newPlainText("网址", "yuanfengai.cn")
            clipboard.setPrimaryClip(clip)
            Toast.makeText(this, "已复制", Toast.LENGTH_SHORT).show()
        }
        
        // 配对电脑按钮
        val pairButton = findViewById<MaterialButton>(R.id.pairButton)
        pairButton.setOnClickListener {
            val intent = Intent(this, PairingActivity::class.java)
            startActivity(intent)
        }
    }
}
