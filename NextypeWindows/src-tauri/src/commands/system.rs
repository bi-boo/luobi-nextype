// ============================================================
// 系统权限与设置 Commands
// ============================================================

use tauri::Manager;

// 注意: has_accessibility_permission 和 open_accessibility_settings
// 已在 clipboard.rs 中实现，这里不再重复

/// 获取开机启动状态
#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;

    let autostart_manager = app.autolaunch();
    autostart_manager
        .is_enabled()
        .map_err(|e| format!("获取开机启动状态失败: {}", e))
}

/// 设置开机启动
#[tauri::command]
pub async fn set_autostart_enabled(
    app: tauri::AppHandle,
    enabled: bool,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let autostart_manager = app.autolaunch();

    if enabled {
        autostart_manager
            .enable()
            .map_err(|e| format!("启用开机启动失败: {}", e))?;
        tracing::info!("✅ 开机启动已启用");
    } else {
        autostart_manager
            .disable()
            .map_err(|e| format!("禁用开机启动失败: {}", e))?;
        tracing::info!("❌ 开机启动已禁用");
    }

    Ok(())
}

/// 设置 Dock 图标可见性 (Windows 不支持，保留接口兼容)
#[tauri::command]
pub async fn set_dock_icon_visible(_app: tauri::AppHandle, _visible: bool) -> Result<(), String> {
    // Windows 没有 Dock，这个功能不适用
    tracing::info!("Windows 平台不支持 Dock 图标设置");
    Ok(())
}

/// 设置菜单栏图标可见性
#[tauri::command]
pub async fn set_menu_bar_icon_visible(
    app: tauri::AppHandle,
    visible: bool,
) -> Result<(), String> {
    // 获取托盘图标
    if let Some(tray) = app.tray_by_id("main-tray") {
        if visible {
            tray.set_visible(true)
                .map_err(|e| format!("显示菜单栏图标失败: {}", e))?;
            tracing::info!("✅ 菜单栏图标已显示");
        } else {
            tray.set_visible(false)
                .map_err(|e| format!("隐藏菜单栏图标失败: {}", e))?;
            tracing::info!("❌ 菜单栏图标已隐藏");
        }
    }

    Ok(())
}

/// 获取平台信息
#[tauri::command]
pub async fn get_platform() -> Result<String, String> {
    Ok(std::env::consts::OS.to_string())
}

/// 写入文件（供前端日志导出等功能使用）
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    tracing::info!("📄 文件已写入: {}", path);
    Ok(())
}
