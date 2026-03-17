// ============================================================
// 窗口管理 Commands
// ============================================================

use tauri::{AppHandle, Manager};

use crate::services::tray;

/// 打开偏好设置窗口
#[tauri::command]
pub async fn open_preferences_window(
    app: AppHandle,
    initial_tab: Option<String>,
) -> Result<(), String> {
    tray::create_preferences_window(&app, initial_tab.as_deref()).map_err(|e| e.to_string())
}

/// 打开日志窗口
#[tauri::command]
pub async fn open_logs_window(app: AppHandle) -> Result<(), String> {
    tray::create_logs_window(&app).map_err(|e| e.to_string())
}

/// 打开引导窗口
#[tauri::command]
pub async fn open_onboarding_window(app: AppHandle) -> Result<(), String> {
    tray::create_onboarding_window(&app).map_err(|e| e.to_string())
}

/// 关闭指定窗口
#[tauri::command]
pub async fn close_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

/// 聚焦指定窗口
#[tauri::command]
pub async fn focus_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.set_focus().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

/// 隐藏指定窗口
#[tauri::command]
pub async fn hide_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.hide().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

/// 显示指定窗口
#[tauri::command]
pub async fn show_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}
