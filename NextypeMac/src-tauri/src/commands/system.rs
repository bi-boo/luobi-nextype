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

/// 设置 Dock 图标可见性 (macOS)
#[tauri::command]
pub async fn set_dock_icon_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;

        // 切换 ActivationPolicy 前，记录当前所有可见窗口
        // 因为切换到 Accessory 模式时 macOS 会自动隐藏所有窗口
        let visible_windows: Vec<tauri::WebviewWindow> = app
            .webview_windows()
            .values()
            .filter(|w| w.is_visible().unwrap_or(false))
            .cloned()
            .collect();

        if visible {
            app.set_activation_policy(ActivationPolicy::Regular)
                .map_err(|e| format!("设置 Dock 图标可见性失败: {}", e))?;
            tracing::info!("✅ Dock 图标已显示");
        } else {
            app.set_activation_policy(ActivationPolicy::Accessory)
                .map_err(|e| format!("设置 Dock 图标可见性失败: {}", e))?;
            tracing::info!("❌ Dock 图标已隐藏");
        }

        // 恢复之前可见的窗口
        for window in visible_windows {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

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
/// 限制写入路径：仅允许桌面、文档、下载目录
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    use std::path::PathBuf;

    let home_dir = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
    let canonical_home = home_dir
        .canonicalize()
        .map_err(|e| format!("主目录解析失败: {}", e))?;

    let target = PathBuf::from(&path);
    let parent = target
        .parent()
        .ok_or_else(|| "无效路径：缺少父目录".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("路径解析失败: {}", e))?;

    if !canonical_parent.starts_with(&canonical_home) {
        return Err("不允许写入该路径，仅支持用户主目录下的路径".to_string());
    }

    std::fs::write(&path, &content)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    tracing::info!("📄 文件已写入: {}", path);
    Ok(())
}
