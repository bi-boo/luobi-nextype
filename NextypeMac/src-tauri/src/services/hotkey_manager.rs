// ============================================================
// 全局快捷键管理器
// ============================================================

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::services::relay_client::WsMessage;

#[cfg(target_os = "macos")]
use crate::services::native_hotkey::{is_fn_shortcut, NativeFnHotkey};

/// 防抖间隔（毫秒），与 Electron 版本一致
const DEBOUNCE_MS: u128 = 500;

/// 心跳间隔（毫秒）
const HEARTBEAT_INTERVAL_MS: u64 = 500;

/// 全局心跳停止标志
static HEARTBEAT_ACTIVE: AtomicBool = AtomicBool::new(false);

/// 快捷键管理器
pub struct HotkeyManager {
    /// 已注册的快捷键映射 (action -> accelerators)
    registered: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 上次触发时间（用于防抖）
    last_trigger_time: Arc<RwLock<HashMap<String, Instant>>>,
    /// 应用句柄
    app_handle: AppHandle,
    /// macOS 原生 Fn 快捷键监听器
    #[cfg(target_os = "macos")]
    native_fn: Arc<NativeFnHotkey>,
}

impl HotkeyManager {
    /// 创建新的快捷键管理器
    pub fn new(app_handle: AppHandle) -> Self {
        #[cfg(target_os = "macos")]
        let native_fn = Arc::new(NativeFnHotkey::new(app_handle.clone()));

        Self {
            registered: Arc::new(RwLock::new(HashMap::new())),
            last_trigger_time: Arc::new(RwLock::new(HashMap::new())),
            app_handle,
            #[cfg(target_os = "macos")]
            native_fn,
        }
    }

    /// 注册快捷键（支持多个）
    pub fn register(&self, action: String, accelerators: Vec<String>) -> Result<(), String> {
        self.unregister(&action)?;

        if accelerators.is_empty() {
            return Ok(());
        }

        for accelerator in &accelerators {
            if accelerator.is_empty() {
                continue;
            }

            // macOS: 含 Fn+ 的快捷键 或 longpress 快捷键 → 走原生 CGEventTap
            #[cfg(target_os = "macos")]
            {
                let is_longpress = action == "longpress";
                if is_fn_shortcut(accelerator) || is_longpress {
                    self.native_fn
                        .register(action.clone(), accelerator.clone(), is_longpress)?;

                    if self.native_fn.registered_count() == 1 {
                        self.native_fn.start_global_listener()?;
                    }

                    tracing::info!("✅ 快捷键注册成功 (CGEventTap): {} -> {}", action, accelerator);
                    continue;
                }
            }

            // 不含 Fn 的快捷键走原有 tauri-plugin-global-shortcut
            let shortcut: Shortcut = accelerator
                .parse()
                .map_err(|e| format!("无效的快捷键格式: {}", e))?;

            let action_clone = action.clone();
            let app_handle = self.app_handle.clone();
            let last_trigger = self.last_trigger_time.clone();

            self.app_handle
                .global_shortcut()
                .on_shortcut(shortcut.clone(), move |_app, _shortcut, _event| {
                    {
                        let mut times = last_trigger.write();
                        let now = Instant::now();
                        if let Some(last) = times.get(&action_clone) {
                            if now.duration_since(*last).as_millis() < DEBOUNCE_MS {
                                return;
                            }
                        }
                        times.insert(action_clone.clone(), now);
                    }

                    tracing::info!("🔔 快捷键触发: {}", action_clone);
                    let _ = app_handle.emit("hotkey_triggered", &action_clone);

                    let app_handle_inner = app_handle.clone();
                    let action_inner = action_clone.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = handle_hotkey_press(&app_handle_inner, &action_inner).await {
                            tracing::error!("❌ 处理快捷键动作失败: {}", e);
                        }
                    });
                })
                .map_err(|e| format!("注册快捷键失败: {}", e))?;

            tracing::info!("✅ 快捷键注册成功: {} -> {}", action, accelerator);
        }

        self.registered.write().insert(action, accelerators);
        Ok(())
    }

    /// 注销单个快捷键
    pub fn unregister(&self, action: &str) -> Result<(), String> {
        let mut registered = self.registered.write();

        if let Some(accelerators) = registered.get(action) {
            for accelerator in accelerators {
                if accelerator.is_empty() {
                    continue;
                }

                // macOS: 含 Fn 的快捷键或 longpress 从原生监听器移除
                #[cfg(target_os = "macos")]
                if is_fn_shortcut(accelerator) || action == "longpress" {
                    self.native_fn.unregister(action);
                    tracing::info!("🗑️ CGEventTap 快捷键已注销: {} ({})", action, accelerator);

                    if self.native_fn.registered_count() == 0 {
                        self.native_fn.stop_global_listener();
                    }
                    continue;
                }

                // 不含 Fn 的走原有逻辑
                if let Ok(shortcut) = accelerator.parse::<Shortcut>() {
                    self.app_handle
                        .global_shortcut()
                        .unregister(shortcut)
                        .map_err(|e| format!("注销快捷键失败: {}", e))?;

                    tracing::info!("🗑️ 快捷键已注销: {} ({})", action, accelerator);
                }
            }

            registered.remove(action);
        }

        Ok(())
    }

    /// 注销所有快捷键
    pub fn unregister_all(&self) -> Result<(), String> {
        let registered = self.registered.read().clone();

        for action in registered.keys() {
            self.unregister(action)?;
        }

        Ok(())
    }

    /// 批量注册快捷键
    pub fn register_all(&self, hotkeys: HashMap<String, Vec<String>>) -> Result<(), String> {
        self.unregister_all()?;

        for (action, accelerators) in hotkeys {
            if !accelerators.is_empty() {
                self.register(action, accelerators)?;
            }
        }

        Ok(())
    }

    /// 获取已注册的快捷键列表
    pub fn get_registered(&self) -> HashMap<String, Vec<String>> {
        self.registered.read().clone()
    }

    /// 开始原生按键录入（macOS）
    #[cfg(target_os = "macos")]
    pub fn start_recording(&self) -> Result<(), String> {
        self.native_fn.start_recording()
    }

    /// 停止原生按键录入（macOS）
    #[cfg(target_os = "macos")]
    pub fn stop_recording(&self) -> Result<(), String> {
        self.native_fn.stop_recording()
    }

    /// 通知辅助功能权限可能已变更（macOS，应用激活时调用）
    #[cfg(target_os = "macos")]
    pub fn notify_accessibility_change(&self) {
        self.native_fn.notify_accessibility_change();
    }
}

/// 处理快捷键按下后的动作分发（供 native_hotkey 模块调用）
pub async fn handle_hotkey_press_public(app: &AppHandle, action: &str) -> Result<(), String> {
    handle_hotkey_press(app, action).await
}

/// 处理快捷键释放后的动作分发（供 native_hotkey 模块调用，仅 longpress 使用）
pub async fn handle_hotkey_release_public(app: &AppHandle, action: &str) -> Result<(), String> {
    handle_hotkey_release(app, action).await
}

/// 处理快捷键按下后的动作分发
async fn handle_hotkey_press(app: &AppHandle, action: &str) -> Result<(), String> {
    use crate::commands::relay::SharedRelayClient;
    use crate::state::SharedAppState;

    let state = app.state::<SharedAppState>();
    let relay_client = app.state::<SharedRelayClient>();

    let online_clients = relay_client.read().get_online_clients();
    if online_clients.is_empty() {
        tracing::warn!("[快捷键操作] ⚠️ 指令发送失败：未发现任何已连接的设备");
        return Ok(());
    }

    let config = state.get_config();
    let from_device_id = config.device_id.clone();

    for target_id in online_clients {
        // iOS 不支持模拟点击，跳过
        if action == "tap" || action == "longpress" || action == "touch_down" {
            let platform = relay_client.read().get_device_platform(&target_id);
            if platform.as_deref() == Some("ios") {
                tracing::warn!("[快捷键操作] ⚠️ 设备 {} 为 iOS 平台，不支持模拟点击，已跳过", target_id);
                continue;
            }
        }

        let mut data = serde_json::json!({
            "type": "command",
            "action": action
        });

        // 如果是点击或长按指令，需要匹配坐标
        if action == "tap" || action == "longpress" {
            let coordinates = if action == "longpress" {
                &config.longpress_coordinates
            } else {
                &config.tap_coordinates
            };
            let session = relay_client.read().get_device_session(&target_id);
            if let Some(coords) = match_coordinates(coordinates, session.as_ref()) {
                // longpress 改发 touch_down（实时长按模式）
                let send_action = if action == "longpress" { "touch_down" } else { action };
                data = serde_json::json!({
                    "type": "command",
                    "action": send_action,
                    "x": coords.0,
                    "y": coords.1
                });
            } else {
                tracing::warn!(
                    "[快捷键操作] ⚠️ 设备 {} 的当前显示尺寸没有匹配的点击坐标配置",
                    target_id
                );
                continue;
            }
        }

        let msg = WsMessage::Relay {
            from: from_device_id.clone(),
            to: target_id.clone(),
            data,
        };

        if let Err(e) = relay_client.read().send(msg) {
            tracing::error!("向设备 {} 发送指令失败: {}", target_id, e);
        } else {
            tracing::info!("📤 指令 {} 已成功发送至设备 {}", action, target_id);
        }
    }

    // 如果是 longpress，启动心跳任务
    if action == "longpress" {
        HEARTBEAT_ACTIVE.store(true, Ordering::SeqCst);
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            tracing::info!("💓 心跳任务启动");
            while HEARTBEAT_ACTIVE.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(HEARTBEAT_INTERVAL_MS)).await;
                if !HEARTBEAT_ACTIVE.load(Ordering::SeqCst) {
                    break;
                }
                if let Err(e) = send_heartbeat(&app_clone).await {
                    tracing::error!("❌ 发送心跳失败: {}", e);
                }
            }
            tracing::info!("💓 心跳任务停止");
        });
    }

    Ok(())
}

/// 处理长按快捷键释放后的动作分发（发送 touch_up）
async fn handle_hotkey_release(app: &AppHandle, action: &str) -> Result<(), String> {
    if action != "longpress" {
        return Ok(());
    }

    // 停止心跳任务
    HEARTBEAT_ACTIVE.store(false, Ordering::SeqCst);

    use crate::commands::relay::SharedRelayClient;
    use crate::state::SharedAppState;

    let state = app.state::<SharedAppState>();
    let relay_client = app.state::<SharedRelayClient>();

    let online_clients = relay_client.read().get_online_clients();
    if online_clients.is_empty() {
        return Ok(());
    }

    let config = state.get_config();
    let from_device_id = config.device_id.clone();

    for target_id in online_clients {
        let session = relay_client.read().get_device_session(&target_id);
        if let Some(coords) = match_coordinates(&config.longpress_coordinates, session.as_ref()) {
            let data = serde_json::json!({
                "type": "command",
                "action": "touch_up",
                "x": coords.0,
                "y": coords.1
            });

            let msg = WsMessage::Relay {
                from: from_device_id.clone(),
                to: target_id.clone(),
                data,
            };

            if let Err(e) = relay_client.read().send(msg) {
                tracing::error!("向设备 {} 发送 touch_up 失败: {}", target_id, e);
            } else {
                tracing::info!("📤 touch_up 已成功发送至设备 {}", target_id);
            }
        }
    }

    // 如果开启了自动插入，延时后发送 insert
    if config.longpress_auto_insert {
        let app_clone = app.clone();
        let delay = config.longpress_auto_insert_delay;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            tracing::info!("🔄 longpress 自动插入 (延时 {}ms)", delay);
            if let Err(e) = send_insert_command(&app_clone).await {
                tracing::error!("❌ 自动插入失败: {}", e);
            }
        });
    }

    Ok(())
}

/// 发送长按心跳
async fn send_heartbeat(app: &AppHandle) -> Result<(), String> {
    use crate::commands::relay::SharedRelayClient;
    use crate::state::SharedAppState;

    let state = app.state::<SharedAppState>();
    let relay_client = app.state::<SharedRelayClient>();

    let online_clients = relay_client.read().get_online_clients();
    if online_clients.is_empty() {
        return Ok(());
    }

    let config = state.get_config();
    let from_device_id = config.device_id.clone();

    for target_id in online_clients {
        let session = relay_client.read().get_device_session(&target_id);
        if let Some(coords) = match_coordinates(&config.longpress_coordinates, session.as_ref()) {
            let data = serde_json::json!({
                "type": "command",
                "action": "touch_heartbeat",
                "x": coords.0,
                "y": coords.1
            });

            let msg = WsMessage::Relay {
                from: from_device_id.clone(),
                to: target_id.clone(),
                data,
            };

            if let Err(e) = relay_client.read().send(msg) {
                tracing::error!("向设备 {} 发送心跳失败: {}", target_id, e);
            }
        }
    }

    Ok(())
}

/// 发送插入指令
async fn send_insert_command(app: &AppHandle) -> Result<(), String> {
    use crate::commands::relay::SharedRelayClient;
    use crate::state::SharedAppState;

    let state = app.state::<SharedAppState>();
    let relay_client = app.state::<SharedRelayClient>();

    let online_clients = relay_client.read().get_online_clients();
    if online_clients.is_empty() {
        return Ok(());
    }

    let config = state.get_config();
    let from_device_id = config.device_id.clone();

    for target_id in online_clients {
        let data = serde_json::json!({
            "type": "command",
            "action": "insert"
        });

        let msg = WsMessage::Relay {
            from: from_device_id.clone(),
            to: target_id.clone(),
            data,
        };

        if let Err(e) = relay_client.read().send(msg) {
            tracing::error!("向设备 {} 发送 insert 失败: {}", target_id, e);
        } else {
            tracing::info!("📤 insert 已成功发送至设备 {}", target_id);
        }
    }

    Ok(())
}

/// 匹配点击坐标
fn match_coordinates(
    coordinates: &serde_json::Value,
    session: Option<&crate::services::relay_client::DeviceSession>,
) -> Option<(i32, i32)> {
    let config_list = if let Some(arr) = coordinates.as_array() {
        arr.clone()
    } else if let Some(obj) = coordinates.as_object() {
        let mut list = Vec::new();
        if let Some(f) = obj.get("folded") {
            list.push(f.clone());
        }
        if let Some(u) = obj.get("unfolded") {
            list.push(u.clone());
        }
        list
    } else {
        return None;
    };

    if config_list.is_empty() {
        return None;
    }

    // 如果没有设备会话信息（说明还没收到上报），使用第一个配置
    let (width, height) = match session {
        Some(s) => (s.screen_width as f64, s.screen_height as f64),
        None => {
            let first = &config_list[0];
            return Some((
                first.get("x")?.as_i64()? as i32,
                first.get("y")?.as_i64()? as i32,
            ));
        }
    };

    // 1. 精确匹配
    for config in &config_list {
        let (cw, ch) = match (
            config.get("width").and_then(|v| v.as_f64()),
            config.get("height").and_then(|v| v.as_f64()),
        ) {
            (Some(w), Some(h)) => (w, h),
            _ => continue, // 跳过缺少尺寸字段的旧格式条目
        };
        if (cw - width).abs() < 1.0 && (ch - height).abs() < 1.0 {
            return Some((
                config.get("x")?.as_i64()? as i32,
                config.get("y")?.as_i64()? as i32,
            ));
        }
    }

    // 2. 比例匹配
    let ratio = width / height;
    let mut best_match = None;
    let mut min_diff = f64::MAX;

    for config in &config_list {
        let (cw, ch) = match (
            config.get("width").and_then(|v| v.as_f64()),
            config.get("height").and_then(|v| v.as_f64()),
        ) {
            (Some(w), Some(h)) => (w, h),
            _ => continue, // 跳过缺少尺寸字段的旧格式条目
        };
        let cratio = cw / ch;
        let diff = (ratio - cratio).abs();

        if diff < 0.1 && diff < min_diff {
            min_diff = diff;
            best_match = Some(config);
        }
    }

    if let Some(config) = best_match {
        return Some((
            config.get("x")?.as_i64()? as i32,
            config.get("y")?.as_i64()? as i32,
        ));
    }

    // 3. 兜底匹配
    let first = &config_list[0];
    Some((
        first.get("x")?.as_i64()? as i32,
        first.get("y")?.as_i64()? as i32,
    ))
}
