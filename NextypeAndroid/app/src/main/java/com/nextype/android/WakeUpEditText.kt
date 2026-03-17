package com.nextype.android

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.view.KeyEvent
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputConnectionWrapper

/**
 * 自定义 EditText，通过包装 InputConnection 拦截所有来自软键盘的操作，
 * 在检测到任何键盘交互时触发屏幕唤醒回调。
 *
 * 解决的问题：软键盘是独立窗口，触摸事件不经过 Activity.dispatchTouchEvent()，
 * 导致键盘上的操作无法触发屏幕唤醒。
 *
 * 注意：InputConnection 的方法由输入法进程通过 Binder 线程调用，
 * 必须使用 Handler 切回主线程执行唤醒操作。
 */
class WakeUpEditText @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = android.R.attr.editTextStyle
) : androidx.appcompat.widget.AppCompatEditText(context, attrs, defStyleAttr) {

    /** 键盘活动回调（会在主线程执行） */
    var onKeyboardActivity: (() -> Unit)? = null

    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        val baseConnection = super.onCreateInputConnection(outAttrs) ?: return null
        return WakeUpInputConnection(baseConnection, true)
    }

    /**
     * 包装 InputConnection，在所有输入操作时触发唤醒回调
     */
    private inner class WakeUpInputConnection(
        target: InputConnection,
        mutable: Boolean
    ) : InputConnectionWrapper(target, mutable) {

        private fun notifyActivity() {
            // InputConnection 方法在 Binder 线程调用，必须切回主线程
            mainHandler.post {
                onKeyboardActivity?.invoke()
            }
        }

        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            notifyActivity()
            return super.commitText(text, newCursorPosition)
        }

        override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
            notifyActivity()
            return super.setComposingText(text, newCursorPosition)
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            notifyActivity()
            return super.deleteSurroundingText(beforeLength, afterLength)
        }

        override fun sendKeyEvent(event: KeyEvent?): Boolean {
            notifyActivity()
            return super.sendKeyEvent(event)
        }

        override fun finishComposingText(): Boolean {
            notifyActivity()
            return super.finishComposingText()
        }

        override fun setComposingRegion(start: Int, end: Int): Boolean {
            notifyActivity()
            return super.setComposingRegion(start, end)
        }

        override fun performEditorAction(editorAction: Int): Boolean {
            notifyActivity()
            return super.performEditorAction(editorAction)
        }

        override fun commitCompletion(text: android.view.inputmethod.CompletionInfo?): Boolean {
            notifyActivity()
            return super.commitCompletion(text)
        }

        override fun commitCorrection(correctionInfo: android.view.inputmethod.CorrectionInfo?): Boolean {
            notifyActivity()
            return super.commitCorrection(correctionInfo)
        }
    }
}
