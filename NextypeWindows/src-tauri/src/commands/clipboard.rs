// ============================================================
// 剪贴板和键盘相关 Commands
// ============================================================

use tauri::{AppHandle, Manager};

use crate::services::clipboard;
use crate::state::SharedAppState;

/// 检查是否有辅助功能权限
#[tauri::command]
pub fn has_accessibility_permission() -> bool {
    clipboard::has_accessibility_permission()
}

/// 请求辅助功能权限
#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    clipboard::request_accessibility_permission()
}

/// 打开系统辅助功能设置
#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    clipboard::open_accessibility_settings()
}

/// 执行粘贴操作
#[tauri::command]
pub async fn paste() -> Result<bool, String> {
    clipboard::paste().await
}

/// 执行粘贴+回车操作
#[tauri::command]
pub async fn paste_and_enter() -> Result<bool, String> {
    clipboard::paste_and_enter().await
}

/// 写入剪贴板
#[tauri::command]
pub fn write_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    clipboard::write_clipboard(&app, &text)
}

/// 读取剪贴板
#[tauri::command]
pub fn read_clipboard(app: AppHandle) -> Result<String, String> {
    clipboard::read_clipboard(&app)
}

/// 处理剪贴板内容（复制/粘贴/粘贴回车）
#[tauri::command]
pub async fn handle_clipboard_content(
    app: AppHandle,
    content: String,
    action: String,
) -> Result<(), String> {
    let config = app.state::<SharedAppState>().get_config();
    clipboard::handle_clipboard_content(
        &app,
        content,
        &action,
        &config.btn1_suffix,
        &config.btn2_suffix,
        config.clear_after_paste,
    )
    .await
}
