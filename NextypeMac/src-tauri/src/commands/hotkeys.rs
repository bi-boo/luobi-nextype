// ============================================================
// 快捷键相关 Commands
// ============================================================

use std::collections::HashMap;
use tauri::State;
use tauri_plugin_store::StoreExt;

use crate::services::hotkey_manager::HotkeyManager;
use crate::state::SharedAppState;

/// 快捷键管理器的共享类型
pub type SharedHotkeyManager = std::sync::Arc<parking_lot::RwLock<Option<HotkeyManager>>>;

/// 注册单个快捷键（向后兼容，转换为数组格式）
#[tauri::command]
pub async fn register_hotkey(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    manager: State<'_, SharedHotkeyManager>,
    action: String,
    accelerator: String,
) -> Result<(), String> {
    let accelerators = if accelerator.is_empty() {
        vec![]
    } else {
        vec![accelerator]
    };
    register_hotkey_group(app, state, manager, action, accelerators).await
}

/// 注册快捷键组（支持多个快捷键）
#[tauri::command]
pub async fn register_hotkey_group(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    manager: State<'_, SharedHotkeyManager>,
    action: String,
    accelerators: Vec<String>,
) -> Result<(), String> {
    check_hotkey_conflicts(&action, &accelerators, &state)?;

    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.register(action.clone(), accelerators.clone())?;
    }

    state.update_config(|c| {
        if accelerators.is_empty() {
            c.hotkeys.remove(&action);
        } else {
            c.hotkeys.insert(action.clone(), accelerators.clone());
        }
    });

    let config = state.get_config();
    let store = app
        .store("config.json")
        .map_err(|e| format!("无法打开配置存储: {}", e))?;
    store.set("config", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

/// 检查快捷键冲突
fn check_hotkey_conflicts(
    current_action: &str,
    accelerators: &[String],
    state: &State<'_, SharedAppState>,
) -> Result<(), String> {
    let config = state.get_config();

    // 检查同一功能的两组快捷键是否相同
    if accelerators.len() == 2 && !accelerators[0].is_empty() && accelerators[0] == accelerators[1] {
        return Err("同一功能的两组快捷键不能相同".to_string());
    }

    // 检查不同功能间的冲突
    for (action, existing_accelerators) in &config.hotkeys {
        if action == current_action {
            continue;
        }

        for acc in accelerators {
            if acc.is_empty() {
                continue;
            }
            if existing_accelerators.contains(acc) {
                return Err(format!("快捷键 {} 已被功能「{}」使用", acc, action));
            }
        }
    }

    Ok(())
}

/// 注销单个快捷键
#[tauri::command]
pub async fn unregister_hotkey(
    manager: State<'_, SharedHotkeyManager>,
    action: String,
) -> Result<(), String> {
    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.unregister(&action)?;
    }
    Ok(())
}

/// 批量注册快捷键
#[tauri::command]
pub async fn register_all_hotkeys(
    manager: State<'_, SharedHotkeyManager>,
    hotkeys: HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.register_all(hotkeys)?;
    }
    Ok(())
}

/// 获取已注册的快捷键
#[tauri::command]
pub async fn get_registered_hotkeys(
    manager: State<'_, SharedHotkeyManager>,
) -> Result<HashMap<String, Vec<String>>, String> {
    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        Ok(hotkey_manager.get_registered())
    } else {
        Ok(HashMap::new())
    }
}

/// 保存点击坐标配置
#[tauri::command]
pub async fn save_tap_coordinates(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    coordinates: serde_json::Value,
) -> Result<(), String> {
    // 更新配置中的坐标
    state.update_config(|c| {
        c.tap_coordinates = coordinates.clone();
    });

    // 保存到 store
    let config = state.get_config();
    let store = app
        .store("config.json")
        .map_err(|e| format!("无法打开配置存储: {}", e))?;

    store.set("config", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    tracing::info!("📍 点击坐标配置已保存");
    Ok(())
}

/// 获取点击坐标配置
#[tauri::command]
pub async fn get_tap_coordinates(
    state: State<'_, SharedAppState>,
) -> Result<serde_json::Value, String> {
    let config = state.get_config();
    Ok(config.tap_coordinates)
}

/// 保存长按坐标配置
#[tauri::command]
pub async fn save_longpress_coordinates(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    coordinates: serde_json::Value,
) -> Result<(), String> {
    state.update_config(|c| {
        c.longpress_coordinates = coordinates.clone();
    });

    let config = state.get_config();
    let store = app
        .store("config.json")
        .map_err(|e| format!("无法打开配置存储: {}", e))?;

    store.set("config", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    tracing::info!("📍 长按坐标配置已保存");
    Ok(())
}

/// 获取长按坐标配置
#[tauri::command]
pub async fn get_longpress_coordinates(
    state: State<'_, SharedAppState>,
) -> Result<serde_json::Value, String> {
    let config = state.get_config();
    Ok(config.longpress_coordinates)
}

/// 开始原生按键录入（macOS，支持 Fn 键捕获）
#[tauri::command]
pub async fn start_hotkey_recording(
    manager: State<'_, SharedHotkeyManager>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let manager_guard = manager.read();
        if let Some(hotkey_manager) = manager_guard.as_ref() {
            hotkey_manager.start_recording()?;
        }
    }
    Ok(())
}

/// 停止原生按键录入（macOS）
#[tauri::command]
pub async fn stop_hotkey_recording(
    manager: State<'_, SharedHotkeyManager>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let manager_guard = manager.read();
        if let Some(hotkey_manager) = manager_guard.as_ref() {
            hotkey_manager.stop_recording()?;
        }
    }
    Ok(())
}
