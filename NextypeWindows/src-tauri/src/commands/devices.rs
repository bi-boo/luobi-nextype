// ============================================================
// 设备管理相关 Commands
// ============================================================

use serde::Serialize;
use tauri::State;

use crate::commands::relay::SharedRelayClient;
use crate::services::device_manager::{generate_qr_code_data_url, DeviceManager};
use crate::state::SharedAppState;

/// 共享的 DeviceManager 状态
pub type SharedDeviceManager = std::sync::Arc<DeviceManager>;

/// 配对码响应
#[derive(Debug, Serialize)]
pub struct PairingCodeResponse {
    pub code: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: u64,
    #[serde(rename = "qrCodeUrl")]
    pub qr_code_url: Option<String>,
}

/// 生成配对码
#[tauri::command]
pub fn generate_pairing_code(
    state: State<'_, SharedAppState>,
    device_manager: State<'_, SharedDeviceManager>,
    relay_client: State<'_, SharedRelayClient>,
    force_random: Option<bool>,
) -> Result<PairingCodeResponse, String> {
    let code = device_manager.generate_pairing_code(force_random.unwrap_or(false));

    // 自动向中继服务器注册配对码 (这是关键补丁)
    let _ = relay_client.read().register_pairing_code(code.clone());

    // 获取设备信息用于生成二维码
    let config = state.get_config();
    let relay_server_url = &config.relay_server_url;
    let device_id = &config.device_id;

    // 生成二维码内容（包含配对信息）
    let qr_content = serde_json::json!({
        "type": "nextype_pairing",
        "code": code,
        "serverId": device_id,
        "relayServer": relay_server_url
    })
    .to_string();

    // 生成二维码
    let qr_code_url = generate_qr_code_data_url(&qr_content).ok();

    Ok(PairingCodeResponse {
        code,
        expires_in: 60,
        qr_code_url,
    })
}

/// 获取当前配对码
#[tauri::command]
pub fn get_current_pairing_code(
    device_manager: State<'_, SharedDeviceManager>,
) -> Result<Option<PairingCodeResponse>, String> {
    match device_manager.get_current_pairing_code() {
        Some((code, expires_in)) => Ok(Some(PairingCodeResponse {
            code,
            expires_in,
            qr_code_url: None,
        })),
        None => Ok(None),
    }
}

/// 验证配对码
#[tauri::command]
pub fn verify_pairing_code(
    device_manager: State<'_, SharedDeviceManager>,
    code: String,
) -> Result<bool, String> {
    Ok(device_manager.verify_pairing_code(&code))
}

/// 清除当前配对码
#[tauri::command]
pub fn clear_pairing_code(device_manager: State<'_, SharedDeviceManager>) -> Result<(), String> {
    device_manager.clear_pairing_code();
    Ok(())
}

/// 生成加密密钥
#[tauri::command]
pub fn generate_encryption_key() -> Result<String, String> {
    Ok(DeviceManager::generate_encryption_key())
}

/// 获取本机 IP 地址
#[tauri::command]
pub fn get_local_ip() -> Result<String, String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
}
