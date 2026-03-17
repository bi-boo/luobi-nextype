// ============================================================
// 应用信息 Commands
// ============================================================

/// 获取应用版本
#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// 获取应用名称
#[tauri::command]
pub async fn get_app_name() -> Result<String, String> {
    Ok(env!("CARGO_PKG_NAME").to_string())
}

/// 获取构建信息
#[tauri::command]
pub async fn get_build_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "authors": env!("CARGO_PKG_AUTHORS"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
    }))
}
