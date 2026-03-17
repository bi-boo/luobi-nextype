package com.nextype.android

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.text.TextUtils
import android.util.TypedValue
import android.widget.ImageButton
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.RadioGroup
import android.widget.RadioButton
import com.google.android.material.slider.Slider

class SettingsActivity : AppCompatActivity() {
    
    private lateinit var switchPasteClipboard: SwitchCompat
    private lateinit var switchPasteEnterClipboard: SwitchCompat
    // 屏幕常亮 & 自动变暗
    private lateinit var switchKeepScreenOn: SwitchCompat
    private lateinit var switchAutoDim: SwitchCompat
    private lateinit var autoDimTimeoutRow: LinearLayout
    private lateinit var autoDimTimeoutValue: TextView
    private lateinit var autoDimSettingsContainer: LinearLayout
    
    // 惯用手设置
    private lateinit var handModeRadioGroup: RadioGroup
    private lateinit var radioLeftHand: RadioButton
    private lateinit var radioRightHand: RadioButton
    
    // 设备管理相关（已移至首页弹层）
    // private lateinit var pairedDevicesLabel: TextView
    // private lateinit var devicesListContainer: LinearLayout
    // private lateinit var addDeviceButton: LinearLayout

    
    // 字号设置相关
    private lateinit var fontSizeSlider: Slider
    private lateinit var fontSizePreview: TextView
    private lateinit var fontLabels: List<TextView>
    
    companion object {
        private const val PREFS_NAME = "NextypeSettings"
        private const val KEY_PASTE_COPIES_TO_CLIPBOARD = "pasteCopiesToClipboard"
        private const val KEY_PASTE_ENTER_COPIES_TO_CLIPBOARD = "pasteEnterCopiesToClipboard"
        private const val KEY_HAND_MODE = "handMode"  // "left" 或 "right"
        private const val KEY_INPUT_FONT_SIZE = "inputFontSize"  // 字号档位 0-4
        private const val KEY_KEEP_SCREEN_ON = "keepScreenOn"
        private const val KEY_AUTO_DIM_ENABLED = "autoDimEnabled"
        private const val KEY_AUTO_DIM_TIMEOUT = "autoDimTimeout"  // 毫秒
        
        // ====== 默认值常量（修改默认值只需改这里） ======
        private const val DEFAULT_PASTE_COPIES_TO_CLIPBOARD = false
        private const val DEFAULT_PASTE_ENTER_COPIES_TO_CLIPBOARD = false
        private const val DEFAULT_HAND_MODE = "right"
        private const val DEFAULT_INPUT_FONT_SIZE = 1  // 档位1（标准 18sp）
        private const val DEFAULT_KEEP_SCREEN_ON = true
        private const val DEFAULT_AUTO_DIM_ENABLED = true
        private const val DEFAULT_AUTO_DIM_TIMEOUT = 60_000  // 1 分钟
        
        // 变暗等待时间选项（毫秒 -> 显示文字）
        val DIM_TIMEOUT_OPTIONS = linkedMapOf(
            30_000 to "30 秒",
            60_000 to "1 分钟",
            300_000 to "5 分钟",
            600_000 to "10 分钟"
        )
        
        // 字号档位对应的 sp 值 (16, 18, 20, 24, 28)
        val FONT_SIZE_VALUES = floatArrayOf(16f, 18f, 20f, 24f, 28f)
        
        // 提供静态方法供 MainActivity 读取设置
        fun getPasteCopiesToClipboard(context: Context): Boolean {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getBoolean(KEY_PASTE_COPIES_TO_CLIPBOARD, DEFAULT_PASTE_COPIES_TO_CLIPBOARD)
        }
        
        fun getPasteEnterCopiesToClipboard(context: Context): Boolean {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getBoolean(KEY_PASTE_ENTER_COPIES_TO_CLIPBOARD, DEFAULT_PASTE_ENTER_COPIES_TO_CLIPBOARD)
        }
        
        // 获取惯用手设置：true = 右手（默认），false = 左手
        fun isRightHandMode(context: Context): Boolean {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getString(KEY_HAND_MODE, DEFAULT_HAND_MODE) == "right"
        }
        
        // 获取输入框字号（sp）
        fun getInputFontSize(context: Context): Float {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            val sizeIndex = prefs.getInt(KEY_INPUT_FONT_SIZE, DEFAULT_INPUT_FONT_SIZE)
            return FONT_SIZE_VALUES.getOrElse(sizeIndex) { 18f }
        }
        
        // 获取屏幕常亮开关状态
        fun getKeepScreenOn(context: Context): Boolean {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getBoolean(KEY_KEEP_SCREEN_ON, DEFAULT_KEEP_SCREEN_ON)
        }
        
        // 获取闲置自动变暗开关状态
        fun getAutoDimEnabled(context: Context): Boolean {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getBoolean(KEY_AUTO_DIM_ENABLED, DEFAULT_AUTO_DIM_ENABLED)
        }
        
        // 获取变暗等待时间（毫秒）
        fun getAutoDimTimeout(context: Context): Int {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return prefs.getInt(KEY_AUTO_DIM_TIMEOUT, DEFAULT_AUTO_DIM_TIMEOUT)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        // 返回按钮
        findViewById<ImageButton>(R.id.backButton).setOnClickListener {
            finish()
        }
        

        // 初始化开关
        switchPasteClipboard = findViewById(R.id.switchPasteClipboard)
        switchPasteEnterClipboard = findViewById(R.id.switchPasteEnterClipboard)
        // 初始化惯用手选择
        handModeRadioGroup = findViewById(R.id.handModeRadioGroup)
        radioLeftHand = findViewById(R.id.radioLeftHand)
        radioRightHand = findViewById(R.id.radioRightHand)
        
        
        // 初始化设备管理UI（已移至首页弹层）
        // pairedDevicesLabel = findViewById(R.id.pairedDevicesLabel)
        // devicesListContainer = findViewById(R.id.devicesListContainer)
        // addDeviceButton = findViewById(R.id.addDeviceButton)
        
        // 初始化字号设置UI
        fontSizeSlider = findViewById(R.id.fontSizeSlider)
        fontSizePreview = findViewById(R.id.fontSizePreview)
        fontLabels = listOf(
            findViewById(R.id.fontLabel0),
            findViewById(R.id.fontLabel1),
            findViewById(R.id.fontLabel2),
            findViewById(R.id.fontLabel3),
            findViewById(R.id.fontLabel4)
        )
        
        // 字号滑动条监听
        fontSizeSlider.addOnChangeListener { _, value, fromUser ->
            if (fromUser) {
                val sizeIndex = value.toInt()
                // 保存设置
                val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                prefs.edit().putInt(KEY_INPUT_FONT_SIZE, sizeIndex).apply()
                // 更新预览和标签
                updateFontSizeUI(sizeIndex)
            }
        }
        
        // 初始化屏幕常亮 & 自动变暗 UI
        switchKeepScreenOn = findViewById(R.id.switchKeepScreenOn)
        switchAutoDim = findViewById(R.id.switchAutoDim)
        autoDimTimeoutRow = findViewById(R.id.autoDimTimeoutRow)
        autoDimTimeoutValue = findViewById(R.id.autoDimTimeoutValue)
        autoDimSettingsContainer = findViewById(R.id.autoDimSettingsContainer)
        
        // 常亮主开关监听
        switchKeepScreenOn.setOnCheckedChangeListener { _, isChecked ->
            saveSetting(KEY_KEEP_SCREEN_ON, isChecked)
            // 联动子控件的可用状态
            updateAutoDimSettingsEnabled(isChecked)
        }
        
        // 自动变暗子开关监听
        switchAutoDim.setOnCheckedChangeListener { _, isChecked ->
            saveSetting(KEY_AUTO_DIM_ENABLED, isChecked)
            // 联动等待时间行的可用状态
            autoDimTimeoutRow.alpha = if (isChecked) 1.0f else 0.4f
            autoDimTimeoutRow.isClickable = isChecked
        }
        
        // 变暗等待时间选择器
        autoDimTimeoutRow.setOnClickListener {
            showDimTimeoutPicker()
        }

        // 加载保存的设置
        loadSettings()

        // 设置监听器
        switchPasteClipboard.setOnCheckedChangeListener { _, isChecked ->
            saveSetting(KEY_PASTE_COPIES_TO_CLIPBOARD, isChecked)
        }

        switchPasteEnterClipboard.setOnCheckedChangeListener { _, isChecked ->
            saveSetting(KEY_PASTE_ENTER_COPIES_TO_CLIPBOARD, isChecked)
        }


        // 惯用手选择监听
        handModeRadioGroup.setOnCheckedChangeListener { _, checkedId ->
            val handMode = if (checkedId == R.id.radioLeftHand) "left" else "right"
            val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            prefs.edit().putString(KEY_HAND_MODE, handMode).apply()
        }
        

        // 使用说明入口
        findViewById<LinearLayout>(R.id.aboutButton).setOnClickListener {
            val intent = Intent(this, AboutActivity::class.java)
            startActivity(intent)
        }
        
        // 隐私政策入口
        findViewById<LinearLayout>(R.id.privacyPolicyButton).setOnClickListener {
            val intent = Intent(this, PrivacyPolicyActivity::class.java)
            startActivity(intent)
        }

        // 辅助功能入口
        findViewById<LinearLayout>(R.id.accessibilityButton).setOnClickListener {
            val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
            startActivity(intent)
        }
        
        // 初始化辅助功能状态
        updateAccessibilityStatus()
    }

    private fun loadSettings() {
        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        switchPasteClipboard.isChecked = prefs.getBoolean(KEY_PASTE_COPIES_TO_CLIPBOARD, DEFAULT_PASTE_COPIES_TO_CLIPBOARD)
        switchPasteEnterClipboard.isChecked = prefs.getBoolean(KEY_PASTE_ENTER_COPIES_TO_CLIPBOARD, DEFAULT_PASTE_ENTER_COPIES_TO_CLIPBOARD)
        // 加载惯用手设置，默认右手
        val handMode = prefs.getString(KEY_HAND_MODE, DEFAULT_HAND_MODE)
        if (handMode == "left") {
            radioLeftHand.isChecked = true
        } else {
            radioRightHand.isChecked = true
        }
        
        // 加载字号设置，默认档位1（标准 18sp）
        val fontSizeIndex = prefs.getInt(KEY_INPUT_FONT_SIZE, DEFAULT_INPUT_FONT_SIZE)
        fontSizeSlider.value = fontSizeIndex.toFloat()
        updateFontSizeUI(fontSizeIndex)
        
        // 加载屏幕常亮 & 自动变暗设置
        val keepScreenOn = prefs.getBoolean(KEY_KEEP_SCREEN_ON, DEFAULT_KEEP_SCREEN_ON)
        switchKeepScreenOn.isChecked = keepScreenOn
        
        val autoDimEnabled = prefs.getBoolean(KEY_AUTO_DIM_ENABLED, DEFAULT_AUTO_DIM_ENABLED)
        switchAutoDim.isChecked = autoDimEnabled
        
        val autoDimTimeout = prefs.getInt(KEY_AUTO_DIM_TIMEOUT, DEFAULT_AUTO_DIM_TIMEOUT)
        autoDimTimeoutValue.text = DIM_TIMEOUT_OPTIONS[autoDimTimeout] ?: DIM_TIMEOUT_OPTIONS[DEFAULT_AUTO_DIM_TIMEOUT]
        
        // 联动子控件状态
        updateAutoDimSettingsEnabled(keepScreenOn)
    }

    private fun saveSetting(key: String, value: Boolean) {
        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        prefs.edit().putBoolean(key, value).apply()
    }
    
    /**
     * 更新字号设置 UI（预览文字和档位标签）
     */
    private fun updateFontSizeUI(sizeIndex: Int) {
        // 更新预览文字字号
        val fontSize = FONT_SIZE_VALUES.getOrElse(sizeIndex) { 18f }
        fontSizePreview.setTextSize(TypedValue.COMPLEX_UNIT_SP, fontSize)
        
        // 更新档位标签颜色（使用 MD3 颜色资源）
        val activeColor = androidx.core.content.ContextCompat.getColor(this, R.color.md_theme_primary)
        val inactiveColor = androidx.core.content.ContextCompat.getColor(this, R.color.md_theme_outline)
        
        fontLabels.forEachIndexed { index, label ->
            label.setTextColor(if (index == sizeIndex) activeColor else inactiveColor)
        }
    }
    
    /**
     * 更新自动变暗子控件的可用状态
     * 主开关关闭时，子开关和等待时间选择器灰显不可用
     */
    private fun updateAutoDimSettingsEnabled(keepScreenOn: Boolean) {
        autoDimSettingsContainer.alpha = if (keepScreenOn) 1.0f else 0.4f
        switchAutoDim.isEnabled = keepScreenOn
        val autoDimOn = keepScreenOn && switchAutoDim.isChecked
        autoDimTimeoutRow.isClickable = autoDimOn
        autoDimTimeoutRow.alpha = if (autoDimOn) 1.0f else 0.4f
    }
    
    /**
     * 显示变暗等待时间选择弹窗
     */
    private fun showDimTimeoutPicker() {
        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val currentTimeout = prefs.getInt(KEY_AUTO_DIM_TIMEOUT, DEFAULT_AUTO_DIM_TIMEOUT)
        
        val entries = DIM_TIMEOUT_OPTIONS.values.toTypedArray()
        val values = DIM_TIMEOUT_OPTIONS.keys.toList()
        val currentIndex = values.indexOf(currentTimeout).coerceAtLeast(0)
        
        AlertDialog.Builder(this)
            .setTitle("变暗等待时间")
            .setSingleChoiceItems(entries, currentIndex) { dialog, which ->
                val selectedTimeout = values[which]
                prefs.edit().putInt(KEY_AUTO_DIM_TIMEOUT, selectedTimeout).apply()
                autoDimTimeoutValue.text = entries[which]
                dialog.dismiss()
            }
            .setNegativeButton("取消", null)
            .show()
    }



    override fun onResume() {
        super.onResume()
        // 重新加载剪贴板设置（可能在其他地方被修改）
        loadSettings()
        // 刷新辅助功能状态（从系统设置页返回时更新）
        updateAccessibilityStatus()
    }

    /**
     * 检测辅助功能是否已开启，并更新 UI 状态
     */
    private fun updateAccessibilityStatus() {
        val statusView = findViewById<TextView>(R.id.accessibilityStatus)
        val isEnabled = isAccessibilityServiceEnabled()
        if (isEnabled) {
            statusView.text = "已开启"
            statusView.setTextColor(androidx.core.content.ContextCompat.getColor(this, R.color.md_theme_primary))
        } else {
            statusView.text = "未开启"
            statusView.setTextColor(androidx.core.content.ContextCompat.getColor(this, R.color.danger))
        }
    }
    
    /**
     * 通过系统 API 检测 AccessibilityService 是否已启用
     */
    private fun isAccessibilityServiceEnabled(): Boolean {
        val serviceName = "${packageName}/${NextypeAccessibilityService::class.java.canonicalName}"
        val enabledServices = Settings.Secure.getString(
            contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return TextUtils.SimpleStringSplitter(':').let { splitter ->
            splitter.setString(enabledServices)
            splitter.any { it.equals(serviceName, ignoreCase = true) }
        }
    }

}
