// ============================================================
// 中继客户端 - WebSocket 连接管理
// ============================================================

use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::state::SharedAppState;
use crate::utils::config::TrustedDevice;

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // 注册
    Register {
        role: String,
        #[serde(rename = "deviceId")]
        device_id: String,
        #[serde(rename = "deviceName")]
        device_name: String,
    },
    // 心跳
    Heartbeat {
        #[serde(rename = "idleTime")]
        idle_time: u64,
    },
    // 转发消息
    Relay {
        from: String,
        to: String,
        data: serde_json::Value,
    },
    // 注册配对码
    RegisterCode {
        code: String,
        #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
        encryption_key: Option<String>,
    },
    // 解除配对
    UnpairDevice {
        #[serde(rename = "targetDeviceId")]
        target_device_id: String,
    },
    // 请求同步信任列表
    SyncTrustList,
}

/// 服务器响应消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Connected,
    Registered {
        #[serde(rename = "deviceId")]
        device_id: String,
    },
    CodeRegistered { code: String },
    CodeConflict { code: String },
    Relay {
        from: String,
        data: serde_json::Value,
    },
    ClientOnline {
        #[serde(rename = "clientId")]
        client_id: String,
        #[serde(rename = "deviceName")]
        device_name: Option<String>,
    },
    ClientOffline {
        #[serde(rename = "clientId")]
        client_id: String,
    },
    ClientHeartbeat {
        #[serde(rename = "clientId")]
        client_id: String,
    },
    HeartbeatAck,
    Error { message: String },
    TrustList { devices: Vec<TrustListDevice> },
    DeviceUnpaired { from: String },
    PairingCompleted { client: PairedClient },
    UnpairSuccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustListDevice {
    pub id: String,
    pub name: String,
    pub role: String,
    #[serde(rename = "pairedAt", default)]
    pub paired_at: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedClient {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "deviceName")]
    pub device_name: String,
}

/// 中继客户端状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayClientState {
    Disconnected,
    Connecting,
    Connected,
}

/// 中继客户端命令
#[derive(Debug)]
pub enum RelayCommand {
    Connect(String),
    Disconnect,
    Send(WsMessage),
    RegisterPairingCode(String),
    UnpairDevice(String),
}

/// 设备会话信息（如屏幕尺寸等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    #[serde(rename = "screenWidth")]
    pub screen_width: u32,
    #[serde(rename = "screenHeight")]
    pub screen_height: u32,
    #[serde(rename = "lastSeen")]
    pub last_seen: i64,
    #[serde(default)]
    pub platform: Option<String>,
}

/// 中继客户端内部状态
pub struct RelayClientInner {
    pub state: RelayClientState,
    pub server_url: Option<String>,
    pub online_clients: HashSet<String>,
    pub client_last_seen: HashMap<String, i64>,
    pub device_sessions: HashMap<String, DeviceSession>,
    pub reconnect_attempts: u32,
    /// 待注册的配对码（用于在重连后自动注册）
    pub pending_pairing_code: Option<String>,
    /// 待确认的加密密钥（配对完成前暂存）
    pub pending_encryption_key: Option<String>,
}

impl Default for RelayClientInner {
    fn default() -> Self {
        Self {
            state: RelayClientState::Disconnected,
            server_url: None,
            online_clients: HashSet::new(),
            client_last_seen: HashMap::new(),
            device_sessions: HashMap::new(),
            reconnect_attempts: 0,
            pending_pairing_code: None,
            pending_encryption_key: None,
        }
    }
}

/// 中继客户端
pub struct RelayClient {
    inner: Arc<RwLock<RelayClientInner>>,
    command_tx: Option<mpsc::Sender<RelayCommand>>,
}

impl RelayClient {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RelayClientInner::default())),
            command_tx: None,
        }
    }

    /// 启动中继客户端
    pub fn start(&mut self, app: AppHandle) {
        let (tx, rx) = mpsc::channel::<RelayCommand>(32);
        self.command_tx = Some(tx);

        let inner = self.inner.clone();

        // 启动后台任务 - 使用 tauri::async_runtime::spawn 而不是 tokio::spawn
        tauri::async_runtime::spawn(async move {
            relay_client_task(app, inner, rx).await;
        });

        tracing::info!("🌐 中继客户端已启动");
    }

    /// 连接到服务器
    pub fn connect(&self, server_url: String) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.try_send(RelayCommand::Connect(server_url))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&self) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.try_send(RelayCommand::Disconnect)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 发送消息
    pub fn send(&self, message: WsMessage) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.try_send(RelayCommand::Send(message))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 注册配对码
    pub fn register_pairing_code(&self, code: String) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.try_send(RelayCommand::RegisterPairingCode(code))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 解除配对
    pub fn unpair_device(&self, device_id: String) -> Result<(), String> {
        if let Some(tx) = &self.command_tx {
            tx.try_send(RelayCommand::UnpairDevice(device_id))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 获取在线客户端列表
    pub fn get_online_clients(&self) -> Vec<String> {
        self.inner.read().online_clients.iter().cloned().collect()
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.inner.read().state == RelayClientState::Connected
    }

    /// 获取设备会话信息
    pub fn get_device_session(&self, device_id: &str) -> Option<DeviceSession> {
        self.inner.read().device_sessions.get(device_id).cloned()
    }

    /// 获取设备平台信息
    pub fn get_device_platform(&self, device_id: &str) -> Option<String> {
        self.inner.read().device_sessions.get(device_id).and_then(|s| s.platform.clone())
    }

    /// 重置重连计数器（网络变化时调用，与 Electron 的 resetReconnectAttempts 对齐）
    pub fn reset_reconnect_attempts(&self) {
        self.inner.write().reconnect_attempts = 0;
    }
}

/// 中继客户端后台任务
async fn relay_client_task(
    app: AppHandle,
    inner: Arc<RwLock<RelayClientInner>>,
    mut command_rx: mpsc::Receiver<RelayCommand>,
) {
    let mut ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>> = None;
    let mut heartbeat_interval = interval(Duration::from_secs(10));
    let mut timeout_check_interval = interval(Duration::from_secs(60)); // 每60秒检查一次超时
    let max_reconnect_attempts = 10;
    let reconnect_delay = Duration::from_secs(5);

    loop {
        tokio::select! {
            // 处理命令
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    RelayCommand::Connect(url) => {
                        inner.write().server_url = Some(url.clone());
                        inner.write().state = RelayClientState::Connecting;

                        match connect_to_server(&url).await {
                            Ok(stream) => {
                                ws_stream = Some(stream);
                                inner.write().state = RelayClientState::Connected;
                                inner.write().reconnect_attempts = 0;
                                tracing::info!("✅ 中继服务器连接成功");

                                // 注册设备
                                let state = app.state::<SharedAppState>();
                                let config = state.get_config();
                                let register_msg = WsMessage::Register {
                                    role: "server".to_string(),
                                    device_id: config.device_id.clone(),
                                    device_name: config.device_name.clone(),
                                };
                                if let Some(stream) = &mut ws_stream {
                                    let _ = send_message(stream, &register_msg).await;
                                    tracing::info!("📝 已发送注册请求");

                                    // 自动恢复配对码注册（带加密密钥）
                                    let pending_code = inner.read().pending_pairing_code.clone();
                                    let pending_enc_key = inner.read().pending_encryption_key.clone();
                                    if let Some(code) = pending_code {
                                        let msg = WsMessage::RegisterCode {
                                            code: code.clone(),
                                            encryption_key: pending_enc_key,
                                        };
                                        let _ = send_message(stream, &msg).await;
                                        tracing::info!("🔢 已自动恢复配对码注册: {}", code);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("❌ 连接中继服务器失败: {}", e);
                                inner.write().state = RelayClientState::Disconnected;
                            }
                        }
                    }
                    RelayCommand::Disconnect => {
                        if let Some(mut stream) = ws_stream.take() {
                            let _ = stream.close(None).await;
                        }
                        inner.write().state = RelayClientState::Disconnected;
                        let was_empty = inner.read().online_clients.is_empty();
                        inner.write().online_clients.clear();
                        inner.write().client_last_seen.clear();
                        tracing::info!("👋 已断开中继服务器连接");
                        sync_hotkeys_with_online_status(&app, was_empty, true);
                    }
                    RelayCommand::Send(msg) => {
                        if let Some(stream) = &mut ws_stream {
                            if let Err(e) = send_message(stream, &msg).await {
                                tracing::error!("❌ 发送消息失败: {}", e);
                            }
                        }
                    }
                    RelayCommand::RegisterPairingCode(code) => {
                        // 每次生成新配对码时生成新的 256 位随机共享密钥
                        let encryption_key = crate::services::device_manager::DeviceManager::generate_encryption_key();
                        inner.write().pending_pairing_code = Some(code.clone());
                        inner.write().pending_encryption_key = Some(encryption_key.clone());
                        if let Some(stream) = &mut ws_stream {
                            let msg = WsMessage::RegisterCode {
                                code: code.clone(),
                                encryption_key: Some(encryption_key),
                            };
                            if let Err(e) = send_message(stream, &msg).await {
                                tracing::error!("❌ 注册配对码失败: {}", e);
                            } else {
                                tracing::info!("🔢 已向中继服务器注册配对码: {}", code);
                            }
                        }
                    }
                    RelayCommand::UnpairDevice(device_id) => {
                        if let Some(stream) = &mut ws_stream {
                            let msg = WsMessage::UnpairDevice {
                                target_device_id: device_id.clone(),
                            };
                            if let Err(e) = send_message(stream, &msg).await {
                                tracing::error!("❌ 发送解除配对请求失败: {}", e);
                            } else {
                                tracing::info!("💔 发送解除配对请求: {}", device_id);
                            }
                        }
                    }
                }
            }

            // 处理接收消息
            _ = async {
                if let Some(stream) = &mut ws_stream {
                    if let Some(msg) = stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                handle_server_message(&app, &inner, &text).await;
                            }
                            Ok(Message::Close(_)) => {
                                tracing::warn!("⚠️ 中继服务器连接关闭");
                                inner.write().state = RelayClientState::Disconnected;
                                let was_empty = inner.read().online_clients.is_empty();
                                inner.write().online_clients.clear();
                                sync_hotkeys_with_online_status(&app, was_empty, true);
                            }
                            Err(e) => {
                                tracing::error!("❌ WebSocket 错误: {}", e);
                                inner.write().state = RelayClientState::Disconnected;
                            }
                            _ => {}
                        }
                    }
                } else {
                    // 没有连接时等待
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } => {}

            // 心跳（P2-9: 获取实际系统闲置时间）
            _ = heartbeat_interval.tick() => {
                if inner.read().state == RelayClientState::Connected {
                    if let Some(stream) = &mut ws_stream {
                        let idle_time = get_system_idle_time();
                        let msg = WsMessage::Heartbeat { idle_time };
                        let _ = send_message(stream, &msg).await;
                    }
                }
            }

            // 客户端超时检查（与 Electron 的 startClientTimeoutChecker 对齐）
            // 每60秒扫描一次，120秒无活动标记为离线
            _ = timeout_check_interval.tick() => {
                if inner.read().state == RelayClientState::Connected {
                    let now = chrono::Utc::now().timestamp();
                    let timeout_threshold = 120; // 120秒超时

                    let was_empty = inner.read().online_clients.is_empty();
                    let timed_out_clients: Vec<String> = {
                        let guard = inner.read();
                        guard.client_last_seen.iter()
                            .filter(|(_, last_seen)| now - **last_seen > timeout_threshold)
                            .map(|(id, _)| id.clone())
                            .collect()
                    };

                    for client_id in timed_out_clients {
                        tracing::info!("⏰ 客户端 {} 超时离线 (120秒无活动)", client_id);
                        inner.write().online_clients.remove(&client_id);
                        inner.write().client_last_seen.remove(&client_id);
                        inner.write().device_sessions.remove(&client_id);

                        let state = app.state::<crate::state::SharedAppState>();
                        state.remove_online_device(&client_id);
                        let _ = app.emit("relay:client_offline", &client_id);

                        if let Err(e) = crate::services::tray::update_tray_menu(&app) {
                            tracing::error!("更新托盘菜单失败: {}", e);
                        }
                    }
                    // 如果没有设备在线了，切换为未连接图标
                    if inner.read().online_clients.is_empty() {
                        let _ = crate::services::tray::update_tray_icon(
                            &app,
                            crate::services::tray::TrayIconType::Disconnected,
                        );
                    }
                    sync_hotkeys_with_online_status(&app, was_empty, inner.read().online_clients.is_empty());
                }
            }
        }

        // 检查是否需要重连
        let should_reconnect = {
            let guard = inner.read();
            guard.state == RelayClientState::Disconnected
                && guard.server_url.is_some()
                && guard.reconnect_attempts < max_reconnect_attempts
        };

        if should_reconnect {
            let url = inner.read().server_url.clone();
            if let Some(url) = url {
                inner.write().reconnect_attempts += 1;
                let attempts = inner.read().reconnect_attempts;
                tracing::info!(
                    "🔄 {}秒后尝试重连 ({}/{})",
                    reconnect_delay.as_secs(),
                    attempts,
                    max_reconnect_attempts
                );

                tokio::time::sleep(reconnect_delay).await;

                inner.write().state = RelayClientState::Connecting;
                match connect_to_server(&url).await {
                    Ok(stream) => {
                        ws_stream = Some(stream);
                        inner.write().state = RelayClientState::Connected;
                        inner.write().reconnect_attempts = 0;
                        tracing::info!("✅ 重连成功");

                        // 重新注册
                        let state = app.state::<SharedAppState>();
                        let config = state.get_config();
                        let register_msg = WsMessage::Register {
                            role: "server".to_string(),
                            device_id: config.device_id.clone(),
                            device_name: config.device_name.clone(),
                        };
                        if let Some(stream) = &mut ws_stream {
                            let _ = send_message(stream, &register_msg).await;
                            
                            // 自动恢复配对码注册（带加密密钥）
                            let pending_code = inner.read().pending_pairing_code.clone();
                            let pending_enc_key = inner.read().pending_encryption_key.clone();
                            if let Some(code) = pending_code {
                                let msg = WsMessage::RegisterCode {
                                    code: code.clone(),
                                    encryption_key: pending_enc_key,
                                };
                                let _ = send_message(stream, &msg).await;
                                tracing::info!("🔢 重连后已自动恢复配对码注册: {}", code);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ 重连失败: {}", e);
                        inner.write().state = RelayClientState::Disconnected;
                    }
                }
            }
        }
    }
}

/// 获取系统闲置时间（秒）- Windows 实现
fn get_system_idle_time() -> u64 {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::GetTickCount64;
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

        unsafe {
            let mut last_input = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut last_input).as_bool() {
                // GetTickCount64 不溢出；dwTime 是 u32，转 u32 再 wrapping_sub 处理 49 天回绕
                let tick_count = GetTickCount64() as u32;
                let idle_ms = tick_count.wrapping_sub(last_input.dwTime);
                return (idle_ms / 1000) as u64;
            }
        }
    }
    0
}

/// 连接到服务器
async fn connect_to_server(
    url: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn std::error::Error + Send + Sync>> {
    let connect_timeout = Duration::from_secs(10);
    let (ws_stream, _) = timeout(connect_timeout, connect_async(url)).await??;
    Ok(ws_stream)
}

/// 发送消息
async fn send_message(
    stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    message: &WsMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(message)?;
    stream.send(Message::Text(json)).await?;
    Ok(())
}

/// 处理服务器消息
async fn handle_server_message(
    app: &AppHandle,
    inner: &Arc<RwLock<RelayClientInner>>,
    text: &str,
) {
    let message: ServerMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("⚠️ 解析服务器消息失败: {} - {}", e, text);
            return;
        }
    };

    match message {
        ServerMessage::Connected => {
            tracing::debug!("✅ 已连接到中继服务器");
        }
        ServerMessage::Registered { device_id } => {
            tracing::info!("✅ 本机已后台登录: {}", device_id);
            let _ = app.emit("relay:registered", &device_id);

            // 注册成功后，自动请求同步信任列表（与 Electron 的 requestTrustSync 对齐）
            let relay = app.state::<crate::commands::SharedRelayClient>();
            let _ = relay.read().send(WsMessage::SyncTrustList);
            tracing::info!("📋 已自动请求同步信任列表");
        }
        ServerMessage::CodeRegistered { code } => {
            tracing::info!("✅ 配对码注册成功: {}", code);
            let _ = app.emit("relay:code_registered", serde_json::json!({
                "code": code,
                "expiresIn": 60
            }));
        }
        ServerMessage::CodeConflict { code } => {
            tracing::warn!("⚠️ 配对码冲突: {}，需要重新生成", code);
            let _ = app.emit("relay:code_conflict", code);
        }
        ServerMessage::Relay { from, data } => {
            tracing::info!("[中继消息] 📥 收到来自 {} 的消息", from);

            // 关键修复：Android 发的 data 可能是 JSON 字符串而非对象
            // 中继服务器原样转发，所以需要先解析
            let parsed_data = if let Some(s) = data.as_str() {
                match serde_json::from_str::<serde_json::Value>(s) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("[中继消息] ⚠️ 解析 data 字符串失败: {} - raw: {}", e, s);
                        data.clone()
                    }
                }
            } else {
                data.clone()
            };

            let msg_type = parsed_data.get("type").and_then(|v| v.as_str());
            tracing::info!("[中继消息] 消息类型: {:?}", msg_type);

            // 忽略 ping 消息
            if msg_type == Some("ping") {
                return;
            }

            // 处理错误消息
            if msg_type == Some("error") {
                let error_title = parsed_data.get("errorTitle").and_then(|v| v.as_str()).unwrap_or("未知错误");
                let error_message = parsed_data.get("errorMessage").and_then(|v| v.as_str()).unwrap_or("");
                tracing::error!("[中继消息] 来自 {} 的错误: {} - {}", from, error_title, error_message);
                return;
            }

            // 处理设备信息/屏幕变化
            if msg_type == Some("device_info") || msg_type == Some("screen_changed") {
                let width = parsed_data.get("screenWidth").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let height = parsed_data.get("screenHeight").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let platform = parsed_data.get("platform").and_then(|v| v.as_str()).map(|s| s.to_lowercase());

                if width > 0 && height > 0 {
                    let mut inner_write = inner.write();
                    inner_write.device_sessions.insert(from.clone(), DeviceSession {
                        screen_width: width,
                        screen_height: height,
                        last_seen: chrono::Utc::now().timestamp(),
                        platform: platform.clone(),
                    });
                    tracing::info!("[中继通知] 📱 设备 {} 屏幕尺寸更新: {}x{}", from, width, height);
                    // 字段名与 Electron 前端期望对齐: clientId, width, height
                    let _ = app.emit("device_screen_info", serde_json::json!({
                        "clientId": from,
                        "width": width,
                        "height": height,
                        "platform": platform.as_deref().unwrap_or("")
                    }));
                }
                return;
            }

            // 处理剪贴板同步消息（Android 发 "clipboard"，也兼容 "content"）
            if msg_type == Some("clipboard") || msg_type == Some("content") {
                let mut content = parsed_data.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let action = parsed_data.get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("copy")
                    .to_string();
                let encrypted = parsed_data.get("encrypted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                tracing::info!(
                    "[中继消息] 📋 收到剪贴板内容: {} 字符, action={}, encrypted={}",
                    content.len(), action, encrypted
                );

                // 解密：优先使用配对时协商的共享密钥，兼容旧版（回退到 from deviceId）
                if encrypted {
                    let decryption_key = {
                        let state = app.state::<SharedAppState>();
                        let config = state.get_config();
                        config.trusted_devices
                            .iter()
                            .find(|d| d.id == from)
                            .and_then(|d| d.encryption_key.clone())
                            .unwrap_or_else(|| from.clone())
                    };
                    match decrypt_cryptojs_aes(&content, &decryption_key) {
                        Ok(decrypted) => {
                            tracing::info!("[中继消息] 🔓 解密成功: {} 字符", decrypted.len());
                            content = decrypted;
                        }
                        Err(e) => {
                            tracing::error!("[中继消息] ❌ 解密失败: {}", e);
                            return;
                        }
                    }
                }

                // 直接处理剪贴板内容（不通过事件系统，避免 Tauri 2 事件路由问题）
                let app_clone = app.clone();
                let from_device = from.clone();
                tauri::async_runtime::spawn(async move {
                    tracing::info!("[后端中心] 正在处理剪贴板内容 (action: {})", action);
                    let config = app_clone
                        .state::<crate::state::SharedAppState>()
                        .get_config();
                    if let Err(e) = crate::services::clipboard::handle_clipboard_content(
                        &app_clone,
                        content,
                        &action,
                        &config.btn1_suffix,
                        &config.btn2_suffix,
                        config.clear_after_paste,
                    )
                    .await
                    {
                        tracing::error!("[后端中心] 处理剪贴板内容失败: {}", e);
                    } else {
                        // 发送确认回执（与 Electron 的 ACK 机制对齐）
                        let state = app_clone.state::<crate::state::SharedAppState>();
                        let device_id = state.get_config().device_id;
                        let ack_msg = WsMessage::Relay {
                            from: device_id,
                            to: from_device,
                            data: serde_json::json!({
                                "type": "ack",
                                "timestamp": chrono::Utc::now().timestamp_millis()
                            }),
                        };
                        let relay = app_clone.state::<crate::commands::SharedRelayClient>();
                        let send_result = relay.read().send(ack_msg);
                        drop(relay);
                        if let Err(e) = send_result {
                            tracing::warn!("[后端中心] 发送 ACK 回执失败: {}", e);
                        } else {
                            tracing::debug!("[后端中心] 📤 已向手机端发送接收确认回执");
                        }
                    }
                });
                return;
            }

            // 其他未识别的 relay 消息，转发到前端
            tracing::debug!("[中继消息] 未识别的消息类型: {:?}, 转发到前端", msg_type);
            let _ = app.emit(
                "relay:message",
                serde_json::json!({
                    "from": from,
                    "data": parsed_data
                }),
            );
        }
        ServerMessage::ClientOnline {
            client_id,
            device_name,
        } => {
            // 只将主通道加入在线列表
            if !client_id.contains('_') {
                let was_empty = inner.read().online_clients.is_empty();
                inner.write().online_clients.insert(client_id.clone());
                inner
                    .write()
                    .client_last_seen
                    .insert(client_id.clone(), chrono::Utc::now().timestamp());

                let display_name = device_name.as_deref().unwrap_or(&client_id);
                tracing::info!("📱 {} 已连接", display_name);

                // 同步到全局状态
                let state = app.state::<crate::state::SharedAppState>();
                state.add_online_device(client_id.clone());

                // 自动更新信任列表（与 Electron 对齐）
                // 如果设备已在信任列表中则更新 last_seen，否则自动添加
                state.update_config(|config| {
                    config.add_trusted_device(TrustedDevice {
                        id: client_id.clone(),
                        name: device_name.clone().unwrap_or_default(),
                        last_ip: String::new(),
                        last_seen: chrono::Utc::now().timestamp(),
                        device_type: String::new(),
                        paired_at: String::new(),
                        encryption_key: None,
                    });
                });

                // 持久化到 store
                persist_config_to_store(app);

                let _ = app.emit(
                    "relay:client_online",
                    serde_json::json!({
                        "clientId": client_id,
                        "deviceName": device_name
                    }),
                );

                // 更新托盘菜单和图标（与 Electron 对齐）
                if let Err(e) = crate::services::tray::update_tray_menu(app) {
                    tracing::error!("更新托盘菜单失败: {}", e);
                }
                // 有设备在线时切换为已连接图标
                let _ = crate::services::tray::update_tray_icon(
                    app,
                    crate::services::tray::TrayIconType::Connected,
                );
                sync_hotkeys_with_online_status(app, was_empty, inner.read().online_clients.is_empty());
            }
        }
        ServerMessage::ClientOffline { client_id } => {
            let was_empty = inner.read().online_clients.is_empty();
            inner.write().online_clients.remove(&client_id);
            inner.write().client_last_seen.remove(&client_id);
            inner.write().device_sessions.remove(&client_id);

            if !client_id.contains('_') {
                tracing::info!("📱 {} 已断开", client_id);

                // 同步从全局状态移除
                let state = app.state::<crate::state::SharedAppState>();
                state.remove_online_device(&client_id);

                let _ = app.emit("relay:client_offline", &client_id);

                // 更新托盘菜单
                if let Err(e) = crate::services::tray::update_tray_menu(app) {
                    tracing::error!("更新托盘菜单失败: {}", e);
                }
                // 如果没有设备在线了，切换为未连接图标
                if inner.read().online_clients.is_empty() {
                    let _ = crate::services::tray::update_tray_icon(
                        app,
                        crate::services::tray::TrayIconType::Disconnected,
                    );
                }
                sync_hotkeys_with_online_status(app, was_empty, inner.read().online_clients.is_empty());
            }
        }
        ServerMessage::ClientHeartbeat { client_id } => {
            // 客户端心跳通知：更新 client_last_seen，防止误判超时离线
            if inner.read().online_clients.contains(&client_id) {
                inner
                    .write()
                    .client_last_seen
                    .insert(client_id, chrono::Utc::now().timestamp());
            }
        }
        ServerMessage::HeartbeatAck => {
            // 心跳响应，不需要处理
        }
        ServerMessage::Error { message } => {
            tracing::error!("❌ 服务器错误: {}", message);
            let _ = app.emit("relay:error", message);
        }
        ServerMessage::TrustList { devices } => {
            tracing::info!("📋 收到信任列表同步: {}个设备", devices.len());

            // 合并到本地
            let state = app.state::<SharedAppState>();
            for device in &devices {
                // pairedAt 可能是 i64 或 ISO 字符串，统一处理
                let paired_at_str = device.paired_at.as_ref().map(|v| {
                    match v {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => String::new(),
                    }
                }).unwrap_or_default();
                let last_seen = device.paired_at.as_ref().and_then(|v| v.as_i64()).unwrap_or(0);

                state.update_config(|config| {
                    config.add_trusted_device(TrustedDevice {
                        id: device.id.clone(),
                        name: device.name.clone(),
                        last_ip: String::new(),
                        last_seen,
                        device_type: device.role.clone(),
                        paired_at: paired_at_str.clone(),
                        encryption_key: None,
                    });
                });
            }

            // 持久化到 store
            persist_config_to_store(app);

            let _ = app.emit("relay:trust_list", &devices);
        }
        ServerMessage::DeviceUnpaired { from } => {
            tracing::info!("💔 收到解除配对通知: 来自 {}", from);
            let _ = app.emit("relay:device_unpaired", from);
        }
        ServerMessage::PairingCompleted { client } => {
            tracing::info!(
                "🤝 配对完成: {} ({})",
                client.device_name,
                client.device_id
            );

            // 取出并清除暂存的加密密钥
            let encryption_key = inner.write().pending_encryption_key.take();
            inner.write().pending_pairing_code = None;

            // 添加到信任列表（携带共享加密密钥）
            let state = app.state::<SharedAppState>();
            let now = chrono::Utc::now().timestamp();
            state.update_config(|config| {
                config.add_trusted_device(TrustedDevice {
                    id: client.device_id.clone(),
                    name: client.device_name.clone(),
                    last_ip: String::new(),
                    last_seen: now,
                    device_type: "client".to_string(),
                    paired_at: now.to_string(),
                    encryption_key: encryption_key.clone(),
                });
            });

            // 持久化到 store（解决内存更新但前端 get_config 读不到的问题）
            persist_config_to_store(app);

            let _ = app.emit("relay:pairing_completed", serde_json::json!({
                "deviceName": client.device_name,
                "deviceId": client.device_id
            }));

            // 发送系统通知（与 Electron 对齐）
            use tauri_plugin_notification::NotificationExt;
            let _ = app.notification()
                .builder()
                .title("配对成功")
                .body(format!("设备 {} 已成功配对", client.device_name))
                .show();
        }
        ServerMessage::UnpairSuccess => {
            tracing::info!("✅ 解除配对成功");
            let _ = app.emit("relay:unpair_success", ());
        }
    }
}

/// 根据在线设备数量变化同步快捷键注册状态
/// was_empty: 变更前 online_clients 是否为空；is_empty_now: 变更后是否为空
fn sync_hotkeys_with_online_status(app: &AppHandle, was_empty: bool, is_empty_now: bool) {
    if was_empty && !is_empty_now {
        // 0 → 1+：首台设备上线，注册快捷键
        let state = app.state::<SharedAppState>();
        let hotkeys = state.get_config().hotkeys.clone();
        if !hotkeys.is_empty() {
            let manager_guard = app.state::<crate::commands::SharedHotkeyManager>();
            let guard = manager_guard.read();
            if let Some(hk_manager) = guard.as_ref() {
                match hk_manager.register_all(hotkeys.clone()) {
                    Ok(()) => tracing::info!("⌨️ 设备上线，已注册 {} 个快捷键", hotkeys.len()),
                    Err(e) => tracing::error!("❌ 注册快捷键失败: {}", e),
                }
            }
        }
    } else if !was_empty && is_empty_now {
        // 1+ → 0：最后一台设备离线，注销快捷键
        let manager_guard = app.state::<crate::commands::SharedHotkeyManager>();
        let guard = manager_guard.read();
        if let Some(hk_manager) = guard.as_ref() {
            match hk_manager.unregister_all() {
                Ok(()) => tracing::info!("⌨️ 所有设备离线，已注销快捷键"),
                Err(e) => tracing::error!("❌ 注销快捷键失败: {}", e),
            }
        }
    }
}

/// 将内存中的配置持久化到 tauri-plugin-store（解决 state.update_config 只改内存的问题）
fn persist_config_to_store(app: &AppHandle) {
    use tauri_plugin_store::StoreExt;
    let state = app.state::<SharedAppState>();
    let config = state.get_config();
    match app.store("config.json") {
        Ok(store) => {
            if let Ok(value) = serde_json::to_value(&config) {
                store.set("config", value);
                let _ = store.save();
                tracing::debug!("[持久化] ✅ 配置已写入 store");
            }
        }
        Err(e) => {
            tracing::error!("[持久化] ❌ 打开 store 失败: {}", e);
        }
    }
    // 同时通知前端配置已更新
    let _ = app.emit("config_updated", &config);
}

/// CryptoJS 兼容的 AES/CBC 解密
/// CryptoJS.AES.encrypt 默认使用 OpenSSL 格式: "Salted__" + 8字节salt + 密文
/// 密钥派生: EVP_BytesToKey(MD5, password, salt, 1, 32, 16) -> key(32B) + iv(16B)
fn decrypt_cryptojs_aes(encrypted_base64: &str, password: &str) -> Result<String, String> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use base64::Engine;

    // Base64 解码
    let data = base64::engine::general_purpose::STANDARD
        .decode(encrypted_base64)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;

    // 检查 "Salted__" 前缀 (OpenSSL 格式)
    if data.len() < 16 || &data[..8] != b"Salted__" {
        return Err("无效的 CryptoJS 加密数据（缺少 Salted__ 前缀）".to_string());
    }

    let salt = &data[8..16];
    let ciphertext = &data[16..];

    // EVP_BytesToKey: 使用 MD5 迭代派生 key(32B) + iv(16B)
    let password_bytes = password.as_bytes();
    let mut derived = Vec::with_capacity(48);

    let mut prev_hash: Vec<u8> = Vec::new();
    while derived.len() < 48 {
        use md5::{Md5, Digest};
        let mut hasher = Md5::new();
        if !prev_hash.is_empty() {
            hasher.update(&prev_hash);
        }
        hasher.update(password_bytes);
        hasher.update(salt);
        prev_hash = hasher.finalize().to_vec();
        derived.extend_from_slice(&prev_hash);
    }

    let key = &derived[..32];
    let iv = &derived[32..48];

    // AES-256-CBC 解密
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
    let decryptor = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| format!("创建解密器失败: {}", e))?;

    let mut buf = ciphertext.to_vec();
    let decrypted = decryptor
        .decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buf)
        .map_err(|e| format!("解密失败（密码可能不匹配）: {}", e))?;

    String::from_utf8(decrypted.to_vec())
        .map_err(|e| format!("解密结果非 UTF-8: {}", e))
}
