// ============================================================
// 中继相关 Commands
// ============================================================

use tauri::State;

use crate::services::relay_client::{RelayClient, WsMessage};
use crate::state::SharedAppState;

/// 共享的 RelayClient 状态
pub type SharedRelayClient = std::sync::Arc<parking_lot::RwLock<RelayClient>>;

/// 连接到中继服务器
#[tauri::command]
pub fn relay_connect(
    state: State<'_, SharedAppState>,
    relay_client: State<'_, SharedRelayClient>,
) -> Result<(), String> {
    let server_url = state.get_config().relay_server_url;
    relay_client.read().connect(server_url)
}

/// 断开中继服务器连接
#[tauri::command]
pub fn relay_disconnect(relay_client: State<'_, SharedRelayClient>) -> Result<(), String> {
    relay_client.read().disconnect()
}

/// 获取中继连接状态
#[tauri::command]
pub fn relay_is_connected(relay_client: State<'_, SharedRelayClient>) -> Result<bool, String> {
    Ok(relay_client.read().is_connected())
}

/// 获取在线客户端列表
#[tauri::command]
pub fn relay_get_online_clients(
    relay_client: State<'_, SharedRelayClient>,
) -> Result<Vec<String>, String> {
    Ok(relay_client.read().get_online_clients())
}

/// 注册配对码
#[tauri::command]
pub fn relay_register_pairing_code(
    relay_client: State<'_, SharedRelayClient>,
    code: String,
) -> Result<(), String> {
    relay_client.read().register_pairing_code(code)
}

/// 解除设备配对
#[tauri::command]
pub fn relay_unpair_device(
    relay_client: State<'_, SharedRelayClient>,
    device_id: String,
) -> Result<(), String> {
    relay_client.read().unpair_device(device_id)
}

/// 发送消息到指定设备
#[tauri::command]
pub fn relay_send_to_device(
    state: State<'_, SharedAppState>,
    relay_client: State<'_, SharedRelayClient>,
    target_device_id: String,
    data: serde_json::Value,
) -> Result<(), String> {
    let device_id = state.get_config().device_id;
    let msg = WsMessage::Relay {
        from: device_id,
        to: target_device_id,
        data,
    };
    relay_client.read().send(msg)
}
