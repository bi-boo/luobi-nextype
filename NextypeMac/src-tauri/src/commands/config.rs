// ============================================================
// 配置相关 Commands
// ============================================================

use tauri::{Emitter, State};
use tauri_plugin_store::StoreExt;

use crate::state::SharedAppState;
use crate::utils::config::{AppConfig, TrustedDevice};

const STORE_PATH: &str = "config.json";

/// 获取所有配置（内存优先：启动时已从 store 加载，直接读内存避免 stale 覆盖）
#[tauri::command]
pub async fn get_config(
    _app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<AppConfig, String> {
    Ok(state.get_config())
}

/// 保存配置
#[tauri::command]
pub async fn save_config(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    config: AppConfig,
) -> Result<(), String> {
    // 更新状态
    state.update_config(|c| *c = config.clone());

    // 保存到 store
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    store
        .set("config", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    // 广播配置更新
    let _ = app.emit("config_updated", &config);

    Ok(())
}

/// 获取单个配置项
#[tauri::command]
pub async fn get_config_value(
    state: State<'_, SharedAppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let config = state.get_config();
    let config_json = serde_json::to_value(&config).map_err(|e| e.to_string())?;

    config_json
        .get(&key)
        .cloned()
        .ok_or_else(|| format!("Config key '{}' not found", key))
}

/// 设置单个配置项
#[tauri::command]
pub async fn set_config_value(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    // 获取当前配置
    let mut config = state.get_config();

    // 将配置转换为 JSON 对象并更新
    let mut config_json = serde_json::to_value(&config).map_err(|e| e.to_string())?;
    if let Some(obj) = config_json.as_object_mut() {
        obj.insert(key, value);
    }

    // 解析回配置结构
    config = serde_json::from_value(config_json).map_err(|e| e.to_string())?;

    // 保存
    save_config(app, state, config).await
}

/// 获取设备 ID
#[tauri::command]
pub async fn get_device_id(state: State<'_, SharedAppState>) -> Result<String, String> {
    Ok(state.get_config().device_id)
}

/// 获取设备名称
#[tauri::command]
pub async fn get_device_name(state: State<'_, SharedAppState>) -> Result<String, String> {
    Ok(state.get_config().device_name)
}

/// 获取信任设备列表
#[tauri::command]
pub async fn get_trusted_devices(
    state: State<'_, SharedAppState>,
) -> Result<Vec<TrustedDevice>, String> {
    Ok(state.get_config().trusted_devices)
}

/// 添加信任设备
#[tauri::command]
pub async fn add_trusted_device(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    device: TrustedDevice,
) -> Result<(), String> {
    let mut config = state.get_config();
    config.add_trusted_device(device);
    save_config(app, state, config).await
}

/// 移除信任设备
#[tauri::command]
pub async fn remove_trusted_device(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    device_id: String,
) -> Result<bool, String> {
    let mut config = state.get_config();
    let removed = config.remove_trusted_device(&device_id);
    if removed {
        save_config(app, state, config).await?;
    }
    Ok(removed)
}

/// 检查设备是否受信任
#[tauri::command]
pub async fn is_device_trusted(
    state: State<'_, SharedAppState>,
    device_id: String,
) -> Result<bool, String> {
    Ok(state.get_config().is_device_trusted(&device_id))
}

/// 获取中继服务器地址
#[tauri::command]
pub async fn get_relay_server_url(state: State<'_, SharedAppState>) -> Result<String, String> {
    Ok(state.get_config().relay_server_url)
}

/// 重置配置为默认值
#[tauri::command]
pub async fn reset_config(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<(), String> {
    let config = AppConfig::default();
    save_config(app, state, config).await
}

/// 从 Electron 配置迁移数据
#[tauri::command]
pub async fn migrate_from_electron(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<bool, String> {
    // 尝试找到 Electron 配置文件
    let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?;
    let electron_config_path = config_dir.join("nextype").join("clipboard-sync-config.json");

    if !electron_config_path.exists() {
        return Ok(false);
    }

    // 读取 Electron 配置
    let content = std::fs::read_to_string(&electron_config_path)
        .map_err(|e| format!("读取 Electron 配置失败: {}", e))?;

    let electron_config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 Electron 配置失败: {}", e))?;

    // 转换为 AppConfig
    let mut config = state.get_config();

    // 迁移各项配置
    if let Some(v) = electron_config.get("enableBtn1").and_then(|v| v.as_bool()) {
        config.enable_btn1 = v;
    }
    if let Some(v) = electron_config.get("btn1Text").and_then(|v| v.as_str()) {
        config.btn1_text = v.to_string();
    }
    if let Some(v) = electron_config.get("btn1Suffix").and_then(|v| v.as_str()) {
        config.btn1_suffix = v.to_string();
    }
    if let Some(v) = electron_config.get("enableBtn2").and_then(|v| v.as_bool()) {
        config.enable_btn2 = v;
    }
    if let Some(v) = electron_config.get("btn2Text").and_then(|v| v.as_str()) {
        config.btn2_text = v.to_string();
    }
    if let Some(v) = electron_config.get("btn2Suffix").and_then(|v| v.as_str()) {
        config.btn2_suffix = v.to_string();
    }
    if let Some(v) = electron_config.get("showDockIcon").and_then(|v| v.as_bool()) {
        config.show_dock_icon = v;
    }
    if let Some(v) = electron_config.get("showMenuBarIcon").and_then(|v| v.as_bool()) {
        config.show_menu_bar_icon = v;
    }
    if let Some(v) = electron_config.get("autoLaunch").and_then(|v| v.as_bool()) {
        config.auto_launch = v;
    }
    if let Some(v) = electron_config
        .get("enableRemoteConnection")
        .and_then(|v| v.as_bool())
    {
        config.enable_remote_connection = v;
    }
    if let Some(v) = electron_config.get("relayServerUrl").and_then(|v| v.as_str()) {
        config.relay_server_url = v.to_string();
    }

    // 迁移 deviceId（关键：保持与手机端配对记录一致）
    if let Some(v) = electron_config.get("deviceId").and_then(|v| v.as_str()) {
        config.device_id = v.to_string();
        tracing::info!("✅ 已迁移 deviceId: {}", v);
    }

    // 迁移信任设备
    if let Some(devices) = electron_config.get("trustedDevices").and_then(|v| v.as_array()) {
        for device_val in devices {
            if let Ok(device) = serde_json::from_value::<TrustedDevice>(device_val.clone()) {
                config.add_trusted_device(device);
            }
        }
    }

    // 保存迁移后的配置
    save_config(app, state, config).await?;

    Ok(true)
}
