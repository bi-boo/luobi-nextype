// ============================================================
// 剪贴板和键盘自动化服务 (Windows)
// ============================================================

use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_RETURN, VK_V,
};

/// 检查是否有 Accessibility 权限 (Windows 不需要特殊权限)
pub fn has_accessibility_permission() -> bool {
    // Windows 不需要特殊的辅助功能权限
    true
}

/// 请求 Accessibility 权限 (Windows 不需要)
pub fn request_accessibility_permission() -> bool {
    true
}

/// 打开系统辅助功能设置 (Windows 占位)
pub fn open_accessibility_settings() -> Result<(), String> {
    // Windows 不需要特殊权限，这里不做任何操作
    Ok(())
}

#[cfg(target_os = "windows")]
fn send_key_combination(vk_codes: &[u16]) -> Result<(), String> {
    unsafe {
        let mut inputs = Vec::new();

        // 按下所有键
        for &vk in vk_codes {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }

        // 释放所有键（逆序）
        for &vk in vk_codes.iter().rev() {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }

        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);

        if result == inputs.len() as u32 {
            Ok(())
        } else {
            Err("Failed to send input".to_string())
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn send_key_combination(_vk_codes: &[u16]) -> Result<(), String> {
    Err("Not implemented for this platform".to_string())
}

/// 执行粘贴操作 (Ctrl+V)
pub async fn paste() -> Result<bool, String> {
    // 等待 100ms 确保剪贴板写入完成
    tokio::time::sleep(Duration::from_millis(100)).await;

    #[cfg(target_os = "windows")]
    {
        send_key_combination(&[VK_CONTROL.0, VK_V.0])?;
        tracing::info!("[本地处理] ⌨️ 正在执行自动粘贴 (模拟按下 Ctrl+V)");
        Ok(true)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Not implemented for this platform".to_string())
    }
}

/// 执行粘贴+回车操作
pub async fn paste_and_enter() -> Result<bool, String> {
    // 先粘贴
    let paste_success = paste().await?;
    if !paste_success {
        return Ok(false);
    }

    // 等待 300ms 后按回车（与 Mac 端对齐）
    tokio::time::sleep(Duration::from_millis(300)).await;

    #[cfg(target_os = "windows")]
    {
        send_key_combination(&[VK_RETURN.0])?;
        tracing::info!("[本地处理] ⏎ 正在执行回车");
        Ok(true)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Not implemented for this platform".to_string())
    }
}

/// 写入剪贴板
pub fn write_clipboard(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    tracing::info!("[本地处理] 📋 已写入剪贴板: {} 字符", text.len());
    Ok(())
}

/// 读取剪贴板
pub fn read_clipboard(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| e.to_string())
}

/// 处理剪贴板内容（核心业务逻辑）
pub async fn handle_clipboard_content(
    app: &AppHandle,
    content: String,
    action: &str,
    btn1_suffix: &str,
    btn2_suffix: &str,
    clear_after_paste: bool,
) -> Result<(), String> {
    // 根据 action 追加后缀
    let final_content = match action {
        "paste" => format!("{}{}", content, btn1_suffix),
        "paste-enter" => format!("{}{}", content, btn2_suffix),
        _ => content.clone(),
    };

    // 写入剪贴板
    write_clipboard(app, &final_content)?;

    // 执行对应操作
    match action {
        "paste" => {
            paste().await?;
        }
        "paste-enter" => {
            paste_and_enter().await?;
        }
        _ => {
            tracing::info!("[本地处理] 📋 仅复制到剪贴板，不执行粘贴");
        }
    }

    // 可选：粘贴后自动清空剪贴板
    if clear_after_paste && action != "copy" {
        tokio::time::sleep(Duration::from_millis(200)).await;
        write_clipboard(app, "")?;
        tracing::info!("[本地处理] 🧹 已自动清空剪贴板");
    }

    Ok(())
}
