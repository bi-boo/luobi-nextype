// ============================================================
// 落笔 Nextype - Tauri 主入口
// ============================================================

mod commands;
mod services;
mod state;
mod utils;

use std::sync::Arc;

use parking_lot::RwLock;
use services::device_manager::DeviceManager;
use services::hotkey_manager::HotkeyManager;
use services::relay_client::RelayClient;
use state::create_shared_state;
use tauri::Manager;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志 (控制台 + 文件 + 内存缓存 + 前端推送)
    let tauri_logger = utils::logger::TauriLogger::new("nextype-tauri");
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tauri_logger)
        .init();

    // 创建共享状态
    let app_state = create_shared_state();

    // 创建中继客户端
    let relay_client: commands::SharedRelayClient = Arc::new(RwLock::new(RelayClient::new()));

    // 创建设备管理器
    let device_manager: commands::SharedDeviceManager = Arc::new(DeviceManager::new());

    // 创建快捷键管理器（初始为 None，在 setup 中初始化）
    let hotkey_manager: commands::SharedHotkeyManager = Arc::new(RwLock::new(None));

    tauri::Builder::default()
        // 注册插件
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!("检测到重复启动，正在聚焦现有实例...");
            let _ = app.get_webview_window("preferences")
                .or_else(|| app.get_webview_window("main"))
                .map(|w| {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                });
        }))
        .plugin(tauri_plugin_dialog::init())
        // 注册状态
        .manage(app_state)
        .manage(relay_client.clone())
        .manage(device_manager)
        .manage(hotkey_manager.clone())
        // 设置启动回调
        .setup(move |app| {
            // 初始化日志关联
            utils::logger::TauriLogger::setup_app_handle(app.handle().clone());

            // 从 store 加载持久化配置到内存（必须在其他模块使用配置之前完成）
            {
                use tauri_plugin_store::StoreExt;
                let mut config_loaded_from_store = false;
                match app.store("config.json") {
                    Ok(store) => {
                        if let Some(config_value) = store.get("config") {
                            match serde_json::from_value::<utils::config::AppConfig>(config_value.clone()) {
                                Ok(config) => {
                                    let state = app.state::<state::SharedAppState>();
                                    state.update_config(|c| *c = config);
                                    tracing::info!("📦 已从 store 加载持久化配置");
                                    config_loaded_from_store = true;
                                }
                                Err(e) => {
                                    tracing::warn!("⚠️ 解析持久化配置失败，使用默认值: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ 打开配置 store 失败: {}", e);
                    }
                }

                // 首次启动时自动从 Electron 配置迁移（store 中没有配置时）
                if !config_loaded_from_store {
                    if let Some(config_dir) = dirs::config_dir() {
                        let electron_config_path = config_dir.join("nextype").join("clipboard-sync-config.json");
                        if electron_config_path.exists() {
                            tracing::info!("🔄 检测到 Electron 配置，开始自动迁移...");
                            match std::fs::read_to_string(&electron_config_path) {
                                Ok(content) => {
                                    if let Ok(electron_config) = serde_json::from_str::<serde_json::Value>(&content) {
                                        let state = app.state::<state::SharedAppState>();
                                        let mut config = state.get_config();

                                        // 迁移 deviceId（最关键）
                                        if let Some(v) = electron_config.get("deviceId").and_then(|v| v.as_str()) {
                                            config.device_id = v.to_string();
                                            tracing::info!("✅ 已迁移 deviceId: {}", v);
                                        }
                                        // 迁移信任设备
                                        if let Some(devices) = electron_config.get("trustedDevices").and_then(|v| v.as_array()) {
                                            for device_val in devices {
                                                if let Ok(device) = serde_json::from_value::<utils::config::TrustedDevice>(device_val.clone()) {
                                                    config.add_trusted_device(device);
                                                }
                                            }
                                        }
                                        // 迁移 UI 配置
                                        if let Some(v) = electron_config.get("enableBtn1").and_then(|v| v.as_bool()) { config.enable_btn1 = v; }
                                        if let Some(v) = electron_config.get("btn1Text").and_then(|v| v.as_str()) { config.btn1_text = v.to_string(); }
                                        if let Some(v) = electron_config.get("enableBtn2").and_then(|v| v.as_bool()) { config.enable_btn2 = v; }
                                        if let Some(v) = electron_config.get("btn2Text").and_then(|v| v.as_str()) { config.btn2_text = v.to_string(); }
                                        if let Some(v) = electron_config.get("showDockIcon").and_then(|v| v.as_bool()) { config.show_dock_icon = v; }
                                        if let Some(v) = electron_config.get("showMenuBarIcon").and_then(|v| v.as_bool()) { config.show_menu_bar_icon = v; }

                                        state.update_config(|c| *c = config.clone());

                                        // 持久化到 store
                                        if let Ok(store) = app.store("config.json") {
                                            if let Ok(value) = serde_json::to_value(&config) {
                                                store.set("config", value);
                                                let _ = store.save();
                                            }
                                        }
                                        tracing::info!("✅ Electron 配置迁移完成");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("⚠️ 读取 Electron 配置失败: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // 迁移旧 device_name：将 hostname 格式（如 xxx.local）升级为 ComputerName
            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_store::StoreExt;
                let state_ref = app.state::<state::SharedAppState>();
                let current_name = state_ref.get_config().device_name.clone();
                let current_hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_default();

                let needs_migration = current_name.ends_with(".local")
                    || (!current_hostname.is_empty() && current_name == current_hostname);

                if needs_migration {
                    if let Ok(output) = std::process::Command::new("scutil")
                        .args(["--get", "ComputerName"])
                        .output()
                    {
                        let new_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !new_name.is_empty() {
                            state_ref.update_config(|c| c.device_name = new_name.clone());
                            if let Ok(store) = app.store("config.json") {
                                let updated_config = state_ref.get_config();
                                if let Ok(value) = serde_json::to_value(&updated_config) {
                                    store.set("config", value);
                                    let _ = store.save();
                                }
                            }
                            tracing::info!("🔄 已将设备名称从 '{}' 迁移为 '{}'", current_name, new_name);
                        }
                    }
                }
            }

            // 初始化托盘
            services::tray::setup_tray(app.handle())?;

            // 根据持久化配置应用 Dock 图标和菜单栏图标的可见性
            {
                let state_ref = app.state::<state::SharedAppState>();
                let config = state_ref.get_config();

                // 应用 Dock 图标设置（默认 true，仅在用户关闭时才切换为 Accessory）
                #[cfg(target_os = "macos")]
                if !config.show_dock_icon {
                    use tauri::ActivationPolicy;
                    if let Err(e) = app.handle().set_activation_policy(ActivationPolicy::Accessory) {
                        tracing::warn!("⚠️ 启动时设置 Dock 图标隐藏失败: {}", e);
                    } else {
                        tracing::info!("🚀 启动时已隐藏 Dock 图标（根据用户配置）");
                    }
                }

                // 应用菜单栏图标设置
                if !config.show_menu_bar_icon {
                    if let Some(tray) = app.handle().tray_by_id("main-tray") {
                        let _ = tray.set_visible(false);
                        tracing::info!("🚀 启动时已隐藏菜单栏图标（根据用户配置）");
                    }
                }
            }

            // 初始化快捷键管理器
            {
                let mut manager = hotkey_manager.write();
                *manager = Some(HotkeyManager::new(app.handle().clone()));
                tracing::info!("⌨️ 快捷键管理器已初始化");
            }

            // 启动中继客户端
            {
                let mut client = relay_client.write();
                client.start(app.handle().clone());
            }

            // 注：剪贴板同步消息已由 relay_client.rs 直接处理，
            // 不再需要通过 app.listen("relay:message") 中转

            // 自动连接到中继服务器
            let relay_client_clone = relay_client.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<state::SharedAppState>();
                let config = state.get_config();

                if config.enable_remote_connection {
                    tracing::info!("🌐 正在根据持久化配置启动中继连接: {}", config.relay_server_url);
                    let _ = relay_client_clone.read().connect(config.relay_server_url);
                }
            });

            // 启动网络变化监测（与 Electron 的 startNetworkMonitoring 对齐）
            {
                let relay_for_network = relay_client.clone();
                let app_for_network = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let mut last_ip = local_ip_address::local_ip()
                        .map(|ip| ip.to_string())
                        .unwrap_or_default();
                    tracing::info!("🌐 网络监测已启动，当前 IP: {}", if last_ip.is_empty() { "无" } else { &last_ip });

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let current_ip = local_ip_address::local_ip()
                            .map(|ip| ip.to_string())
                            .unwrap_or_default();

                        if !current_ip.is_empty() && !last_ip.is_empty() && current_ip != last_ip {
                            // IP 变化 - 网络切换
                            tracing::info!("🔄 检测到网络变化: {} -> {}", last_ip, current_ip);
                            let state = app_for_network.state::<state::SharedAppState>();
                            let config = state.get_config();
                            if config.enable_remote_connection {
                                relay_for_network.read().reset_reconnect_attempts();
                                let _ = relay_for_network.read().connect(config.relay_server_url);
                            }
                            last_ip = current_ip;
                        } else if last_ip.is_empty() && !current_ip.is_empty() {
                            // 网络恢复
                            tracing::info!("🌐 检测到网络恢复，IP: {}", current_ip);
                            let state = app_for_network.state::<state::SharedAppState>();
                            let config = state.get_config();
                            if config.enable_remote_connection {
                                relay_for_network.read().reset_reconnect_attempts();
                                let _ = relay_for_network.read().connect(config.relay_server_url);
                            }
                            last_ip = current_ip;
                        } else if !last_ip.is_empty() && current_ip.is_empty() {
                            // 网络断开
                            tracing::warn!("⚠️ 检测到网络断开");
                            last_ip = String::new();
                        }
                    }
                });
            }

            // 启动引导：没有配对设备时自动打开偏好设置到"配对手机"页
            {
                let state_ref = app.state::<state::SharedAppState>();
                let config = state_ref.get_config();
                if config.trusted_devices.is_empty() {
                    tracing::info!("未检测到配对设备，自动打开配对手机页面");
                    let _ = services::tray::create_preferences_window(app.handle(), Some("devices"));
                }
            }

            tracing::info!("🚀 落笔 Nextype 启动成功");
            Ok(())
        })
        // 注册命令
        .invoke_handler(tauri::generate_handler![
            // 配置命令
            commands::get_config,
            commands::save_config,
            commands::get_config_value,
            commands::set_config_value,
            commands::get_logs,
            commands::clear_logs,
            commands::get_device_id,
            commands::get_device_name,
            commands::get_trusted_devices,
            commands::add_trusted_device,
            commands::remove_trusted_device,
            commands::is_device_trusted,
            commands::get_relay_server_url,
            commands::reset_config,
            commands::migrate_from_electron,
            // 窗口命令
            commands::open_preferences_window,
            commands::open_logs_window,
            commands::open_onboarding_window,
            commands::close_window,
            commands::focus_window,
            commands::hide_window,
            commands::show_window,
            // 中继命令
            commands::relay_connect,
            commands::relay_disconnect,
            commands::relay_is_connected,
            commands::relay_get_online_clients,
            commands::relay_register_pairing_code,
            commands::relay_unpair_device,
            commands::relay_send_to_device,
            // 剪贴板和键盘命令
            commands::has_accessibility_permission,
            commands::request_accessibility_permission,
            commands::open_accessibility_settings,
            commands::paste,
            commands::paste_and_enter,
            commands::write_clipboard,
            commands::read_clipboard,
            commands::handle_clipboard_content,
            // 设备管理命令
            commands::generate_pairing_code,
            commands::get_current_pairing_code,
            commands::verify_pairing_code,
            commands::clear_pairing_code,
            commands::generate_encryption_key,
            commands::get_local_ip,
            // 系统设置命令
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::set_dock_icon_visible,
            commands::set_menu_bar_icon_visible,
            commands::get_platform,
            commands::write_file,
            // 统计数据命令
            commands::get_stats,
            commands::record_paste,
            commands::reset_stats,
            commands::set_stats_enabled,
            commands::get_daily_history,
            // 快捷键命令
            commands::register_hotkey,
            commands::register_hotkey_group,
            commands::unregister_hotkey,
            commands::register_all_hotkeys,
            commands::get_registered_hotkeys,
            commands::save_tap_coordinates,
            commands::get_tap_coordinates,
            commands::save_longpress_coordinates,
            commands::get_longpress_coordinates,
            commands::start_hotkey_recording,
            commands::stop_hotkey_recording,
            // 应用信息命令
            commands::get_app_version,
            commands::get_app_name,
            commands::get_build_info,
        ])
        // 运行应用
        .build(tauri::generate_context!())
        .expect("构建应用时发生错误")
        .run(|app_handle, event| {
            match event {
                // 当点击窗口关闭按钮时，拦截退出请求
                // 这样应用会一直在后台运行（托盘），直到用户从托盘点击"退出"
                tauri::RunEvent::ExitRequested { api, .. } => {
                    let state = app_handle.state::<state::SharedAppState>();
                    if !state.should_quit.load(std::sync::atomic::Ordering::SeqCst) {
                        api.prevent_exit();
                    }
                }
                // 应用激活时（用户从系统设置切回来），通知等待权限的线程重试
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Resumed => {
                    let manager_guard = app_handle.state::<commands::SharedHotkeyManager>();
                    let guard = manager_guard.read();
                    if let Some(hk_manager) = guard.as_ref() {
                        hk_manager.notify_accessibility_change();
                    }
                }
                // 点击 Dock 图标时打开偏好设置窗口
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                    if !has_visible_windows {
                        if let Err(e) = services::tray::create_preferences_window(app_handle, None) {
                            tracing::error!("点击 Dock 图标打开偏好设置失败: {}", e);
                        }
                    }
                }
                _ => {}
            }
        });
}
