// ============================================================
// 快捷键相关 Commands
// ============================================================

use std::collections::HashMap;
use tauri::State;
use tauri_plugin_store::StoreExt;

use crate::services::hotkey_manager::HotkeyManager;
use crate::state::SharedAppState;
use crate::utils::config::AppConfig;

/// 快捷键管理器的共享类型
pub type SharedHotkeyManager = std::sync::Arc<parking_lot::RwLock<Option<HotkeyManager>>>;

const FN_SHORTCUT_SAFETY_HINT: &str =
    "为避免影响系统流畅度，Fn 快捷键仅支持“Fn + 普通按键”，不支持单独 Fn、Fn+Shift、Fn+Ctrl、Fn+Alt、Fn+Command，也不支持 Fn+Enter / Fn+Delete。";

fn is_fn_accelerator(accelerator: &str) -> bool {
    accelerator == "Fn" || accelerator.starts_with("Fn+")
}

fn is_modifier_token(token: &str) -> bool {
    matches!(
        token,
        "Fn"
            | "Ctrl"
            | "Control"
            | "Alt"
            | "Option"
            | "Shift"
            | "Command"
            | "CommandOrControl"
            | "Cmd"
            | "Meta"
    )
}

fn validate_macos_fn_shortcut(accelerator: &str) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = accelerator;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        if !is_fn_accelerator(accelerator) {
            return Ok(());
        }

        let parts: Vec<&str> = accelerator.split('+').filter(|part| !part.is_empty()).collect();
        let primary_keys: Vec<&str> = parts
            .iter()
            .copied()
            .filter(|token| !is_modifier_token(token))
            .collect();

        if primary_keys.is_empty() {
            return Err(FN_SHORTCUT_SAFETY_HINT.to_string());
        }

        let primary_key = primary_keys[0];
        if matches!(primary_key, "Enter" | "Return" | "Delete" | "Backspace" | "ForwardDelete") {
            return Err(FN_SHORTCUT_SAFETY_HINT.to_string());
        }

        Ok(())
    }
}

fn validate_hotkey_safety(accelerators: &[String]) -> Result<(), String> {
    for accelerator in accelerators {
        if accelerator.is_empty() {
            continue;
        }
        validate_macos_fn_shortcut(accelerator)?;
    }
    Ok(())
}

pub fn sanitize_hotkeys_for_macos(config: &mut AppConfig) -> Vec<String> {
    let mut removed = Vec::new();

    for (action, accelerators) in config.hotkeys.iter_mut() {
        accelerators.retain(|accelerator| {
            if validate_macos_fn_shortcut(accelerator).is_ok() {
                true
            } else {
                removed.push(format!("{} -> {}", action, accelerator));
                false
            }
        });
    }

    config.hotkeys.retain(|_, accelerators| !accelerators.is_empty());
    removed
}

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
    validate_hotkey_safety(&accelerators)?;
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
    for accelerators in hotkeys.values() {
        validate_hotkey_safety(accelerators)?;
    }

    let manager_guard = manager.read();
    if let Some(hotkey_manager) = manager_guard.as_ref() {
        hotkey_manager.register_all(hotkeys)?;
    }
    Ok(())
}

/// 获取已配置的快捷键（从持久化配置读取，而非运行时注册状态）
/// 运行时注册状态依赖设备在线才会填充，但 UI 需要始终显示用户配置的值
#[tauri::command]
pub async fn get_registered_hotkeys(
    state: State<'_, SharedAppState>,
) -> Result<HashMap<String, Vec<String>>, String> {
    let config = state.get_config();
    Ok(config.hotkeys)
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
