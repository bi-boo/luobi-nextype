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

/// 注册单个快捷键（同时持久化到配置）
#[tauri::command]
pub async fn register_hotkey(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    manager: State<'_, SharedHotkeyManager>,
    action: String,
    accelerator: String,
) -> Result<(), String> {
    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.register(action.clone(), accelerator.clone())?;
    }

    // 持久化到配置 store（与 Electron 对齐）
    state.update_config(|c| {
        if accelerator.is_empty() {
            c.hotkeys.remove(&action);
        } else {
            c.hotkeys.insert(action.clone(), accelerator.clone());
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

/// 注册快捷键组（支持多个候选键 — Windows 使用首个有效值）
#[tauri::command]
pub async fn register_hotkey_group(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    manager: State<'_, SharedHotkeyManager>,
    action: String,
    accelerators: Vec<String>,
) -> Result<(), String> {
    check_hotkey_conflicts(&action, &accelerators, &state)?;

    // Windows 每个 action 仅支持单个加速键，取首个有效值
    let accelerator = accelerators
        .into_iter()
        .find(|a| !a.is_empty())
        .unwrap_or_default();

    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.register(action.clone(), accelerator.clone())?;
    }

    state.update_config(|c| {
        if accelerator.is_empty() {
            c.hotkeys.remove(&action);
        } else {
            c.hotkeys.insert(action.clone(), accelerator.clone());
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

    for (action, existing_acc) in &config.hotkeys {
        if action == current_action {
            continue;
        }
        for acc in accelerators {
            if acc.is_empty() {
                continue;
            }
            if acc == existing_acc {
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
    hotkeys: HashMap<String, String>,
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
) -> Result<HashMap<String, String>, String> {
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
    
    {
        let manager_guard = manager.read();
        if let Some(hotkey_manager) = manager_guard.as_ref() {
            hotkey_manager.stop_recording()?;
        }
    }
    Ok(())
}
