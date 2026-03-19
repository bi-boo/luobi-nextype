// ============================================================
// 剪贴板和键盘自动化服务
// ============================================================

use std::process::Command;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// 检查是否有 Accessibility 权限
pub fn has_accessibility_permission() -> bool {
    // 使用 JXA 直接调用 AXIsProcessTrusted()，无副作用且准确
    let result = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg("ObjC.import('ApplicationServices'); $.AXIsProcessTrusted()")
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            output.status.success() && stdout.trim() == "true"
        }
        Err(_) => false,
    }
}

/// 请求 Accessibility 权限（触发系统提示）
pub fn request_accessibility_permission() -> bool {
    // 尝试触发系统权限弹窗
    let result = Command::new("osascript")
        .arg("-e")
        .arg(
            r#"
            tell application "System Events"
                keystroke ""
            end tell
        "#,
        )
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 打开系统辅助功能设置
pub fn open_accessibility_settings() -> Result<(), String> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map_err(|e| e.to_string())?;

    tracing::info!("📖 已打开系统设置");
    Ok(())
}

/// 执行粘贴操作 (Cmd+V)
pub async fn paste() -> Result<bool, String> {
    if !has_accessibility_permission() {
        tracing::warn!(
            "[本地处理] ⚠️ 缺少辅助功能权限，无法执行自动粘贴，请在系统设置中授予权限"
        );
        return Ok(false);
    }

    // 等待 100ms 确保剪贴板写入完成（与 Electron 对齐）
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let script = r#"
        tell application "System Events"
            keystroke "v" using command down
        end tell
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;

    if result.status.success() {
        tracing::info!("[本地处理] ⌨️ 正在执行自动粘贴 (模拟按下 Cmd+V)");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        tracing::error!("[本地] ❌ 粘贴失败: {}", stderr);
        Ok(false)
    }
}

/// 执行粘贴+回车操作
pub async fn paste_and_enter() -> Result<bool, String> {
    // 先粘贴
    let paste_success = paste().await?;
    if !paste_success {
        return Ok(false);
    }

    // 延迟 300ms 后按回车
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let script = r#"
        tell application "System Events"
            keystroke return
        end tell
    "#;

    let result = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;

    if result.status.success() {
        tracing::info!("[本地处理] ⌨️ 正在模拟按下回车键 (Enter)");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        tracing::error!("[本地] ❌ 回车失败: {}", stderr);
        Ok(false)
    }
}

/// 写入剪贴板
pub fn write_clipboard(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;

    tracing::debug!("[剪贴板] 已写入文本 ({} 字符)", text.len());
    Ok(())
}

/// 读取剪贴板
pub fn read_clipboard(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| e.to_string())
}

/// 处理接收到的剪贴板内容
pub async fn handle_clipboard_content(
    app: &AppHandle,
    content: &str,
    action: &str,
) -> Result<(), String> {
    use crate::state::SharedAppState;
    use tauri::Manager;

    let state = app.state::<SharedAppState>();
    let config = state.get_config();

    // 处理后缀
    let mut final_content = content.to_string();
    match action {
        "paste" => {
            if !config.btn1_suffix.is_empty() {
                final_content.push_str(&config.btn1_suffix);
            }
        }
        "paste-enter" | "paste_enter" => {
            if !config.btn2_suffix.is_empty() {
                final_content.push_str(&config.btn2_suffix);
            }
        }
        _ => {}
    }

    // 写入剪贴板
    write_clipboard(app, &final_content)?;

    let mut execute_success = false;
    match action {
        "copy" => {
            // 仅复制，不执行其他操作
            tracing::info!("[本地处理] 📋 文本已复制到剪贴板");
            execute_success = true;
        }
        "paste" => {
            // 复制并粘贴
            if paste().await? {
                execute_success = true;
            }
        }
        "paste-enter" | "paste_enter" => {
            // 复制、粘贴并回车
            if paste_and_enter().await? {
                execute_success = true;
            }
        }
        _ => {
            tracing::warn!("[本地处理] ⚠️ 未知操作: {}", action);
        }
    }

    // 粘贴失败且缺少辅助功能权限 → 打开偏好设置引导用户授权
    if !execute_success && action != "copy" && !has_accessibility_permission() {
        tracing::info!("粘贴因缺少辅助功能权限失败，打开偏好设置引导授权");
        let _ = crate::services::tray::create_preferences_window(app, Some("devices"));
    }

    // 记录统计数据（与 Electron 的 stats.recordPaste 对齐）
    if execute_success && (action == "paste" || action == "paste-enter" || action == "paste_enter" || action == "copy") {
        let char_count = content.chars().count();
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            use tauri_plugin_store::StoreExt;
            match app_clone.store("stats.json") {
                Ok(store) => {
                    let mut stats: crate::services::stats::Statistics = store
                        .get("stats")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    stats.check_and_reset_daily();
                    stats.record_paste(char_count);

                    if let Ok(value) = serde_json::to_value(&stats) {
                        store.set("stats", value);
                        let _ = store.save();
                    }

                    let _ = app_clone.emit("stats_updated", &stats);
                    tracing::info!("[本地处理] 📈 使用量统计更新: +{} 字符", char_count);
                }
                Err(e) => {
                    tracing::error!("[统计] ❌ 打开统计存储失败: {}", e);
                }
            }
        });
    }

    // 粘贴后自动清空
    if execute_success && config.clear_after_paste && (action == "paste" || action == "paste-enter" || action == "paste_enter") {
        let app_clone = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if let Err(e) = write_clipboard(&app_clone, "") {
                tracing::error!("[本地处理] ❌ 自动清空剪贴板失败: {}", e);
            } else {
                tracing::info!("[本地处理] 🗑️ 为保护隐私，剪贴板内容已自动清空");
            }
        });
    }

    Ok(())
}
