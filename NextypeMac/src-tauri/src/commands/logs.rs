// ============================================================
// 日 sys 日志相关的 Commands
// ============================================================

use crate::utils::logger::{LogEntry, TauriLogger};

/// 获取内存中缓存的日志
#[tauri::command]
pub async fn get_logs() -> Result<Vec<LogEntry>, String> {
    Ok(TauriLogger::get_logs())
}

/// 清空日志（内存和文件）
#[tauri::command]
pub async fn clear_logs() -> Result<(), String> {
    TauriLogger::clear_logs();
    Ok(())
}
