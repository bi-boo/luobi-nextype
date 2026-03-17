// ============================================================
// 托盘管理
// ============================================================

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Wry,
};

use crate::state::SharedAppState;

/// 托盘图标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconType {
    Disconnected,
    Connected,
}

// 嵌入图标资源
static ICON_DISCONNECTED: &[u8] = include_bytes!("../../icons/iconTemplate.png");
static ICON_CONNECTED: &[u8] = include_bytes!("../../icons/iconConnectedTemplate.png");

/// 初始化托盘图标
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 显式清理旧托盘（如果已存在），解决图标重复问题
    {
        let state = app.state::<SharedAppState>();
        let mut tray_lock = state.tray_icon.write();
        if let Some(_old_tray) = tray_lock.take() {
            tracing::info!("♻️ 检测到旧托盘句柄，正在清理以防止图标重复");
            // 注意：旧实例会在 drop 时自动移除
        }
    }

    // 创建菜单
    let menu = build_tray_menu(app)?;

    // 加载图标
    let icon = load_tray_icon(TrayIconType::Disconnected)?;

    // 创建托盘图标
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true) // 启用左键菜单
        .tooltip("落笔 Nextype")
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .build(app)?;

    // 将托盘图标句柄存入全局状态以保持其生命周期
    let state = app.state::<SharedAppState>();
    *state.tray_icon.write() = Some(tray);

    tracing::info!("📍 菜单栏图标已创建并持久化");
    Ok(())
}

/// 构建托盘菜单
fn build_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, Box<dyn std::error::Error>> {
    let state = app.state::<SharedAppState>();
    let online_devices = state.get_online_devices();

    // 创建菜单项
    let connect_phone = MenuItem::with_id(app, "connect_phone", "配对手机", true, None::<&str>)?;
    let preferences = MenuItem::with_id(app, "preferences", "偏好设置", true, None::<&str>)?;

    let separator1 = PredefinedMenuItem::separator(app)?;

    // 在线设备标题
    let online_devices_label =
        MenuItem::with_id(app, "online_devices_label", "在线设备", false, None::<&str>)?;

    // 在线设备列表
    let device_items: Vec<MenuItem<Wry>> = if online_devices.is_empty() {
        vec![MenuItem::with_id(
            app,
            "no_devices",
            "    (暂无设备)",
            false,
            None::<&str>,
        )?]
    } else {
        let config = state.get_config();
        online_devices
            .iter()
            .filter_map(|device_id| {
                config.get_trusted_device(device_id).map(|device| {
                    let name = if device.name.is_empty() {
                        "Unknown Device"
                    } else {
                        &device.name
                    };
                    let char_count = name.chars().count();
                    let display_name = if char_count > 10 {
                        let truncated: String = name.chars().take(8).collect();
                        format!("    {}...", truncated)
                    } else {
                        format!("    {}", name)
                    };
                    MenuItem::with_id(
                        app,
                        format!("device_{}", device_id),
                        display_name,
                        false,
                        None::<&str>,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
    };

    let separator2 = PredefinedMenuItem::separator(app)?;

    let logs = MenuItem::with_id(app, "logs", "日志", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "重新启动", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 落笔 Nextype", true, None::<&str>)?;

    // 构建菜单
    let menu = Menu::with_items(
        app,
        &[
            &connect_phone,
            &preferences,
            &separator1,
            &online_devices_label,
        ],
    )?;

    // 添加设备项
    for item in &device_items {
        menu.append(item)?;
    }

    // 添加剩余项
    menu.append(&separator2)?;
    menu.append(&logs)?;
    menu.append(&restart)?;
    menu.append(&quit)?;

    Ok(menu)
}

/// 处理菜单事件
fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "connect_phone" => {
            if let Err(e) = create_preferences_window(app, Some("devices")) {
                tracing::error!("打开偏好设置窗口失败: {}", e);
            }
        }
        "preferences" => {
            if let Err(e) = create_preferences_window(app, Some("settings")) {
                tracing::error!("打开偏好设置窗口失败: {}", e);
            }
        }
        "logs" => {
            if let Err(e) = create_logs_window(app) {
                tracing::error!("打开日志窗口失败: {}", e);
            }
        }
        "restart" => {
            app.restart();
        }
        "quit" => {
            let state = app.state::<SharedAppState>();
            state.should_quit.store(true, std::sync::atomic::Ordering::SeqCst);
            app.exit(0);
        }
        _ => {}
    }
}

/// 加载托盘图标
fn load_tray_icon(icon_type: TrayIconType) -> Result<Image<'static>, Box<dyn std::error::Error>> {
    let icon_bytes: &[u8] = match icon_type {
        TrayIconType::Disconnected => ICON_DISCONNECTED,
        TrayIconType::Connected => ICON_CONNECTED,
    };

    let img = image::load_from_memory(icon_bytes)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(Image::new_owned(rgba.into_raw(), width, height))
}

/// 更新托盘图标
pub fn update_tray_icon(
    app: &AppHandle,
    icon_type: TrayIconType,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = load_tray_icon(icon_type)?;
        tray.set_icon(Some(icon))?;
    }
    Ok(())
}

/// 更新托盘菜单
pub fn update_tray_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let menu = build_tray_menu(app)?;
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

// ============================================================
// 窗口管理
// ============================================================

/// 将窗口强制带到最前端（兼容 Accessory 模式）
fn bring_window_to_front(window: &tauri::WebviewWindow) {
    // 先用 set_always_on_top 强制置顶，再取消置顶
    // 这是 macOS Accessory 模式下最可靠的前置方式
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    let _ = window.set_always_on_top(false);
}

/// 获取鼠标所在屏幕的窗口居中位置 (逻辑坐标)
fn get_window_position_on_current_screen(
    app: &AppHandle,
    width: f64,
    height: f64,
) -> (f64, f64) {
    // 获取鼠标位置（物理坐标），然后找到鼠标所在的显示器
    let cursor_pos = get_cursor_position();

    if let Ok(monitors) = app.available_monitors() {
        for monitor in &monitors {
            let scale = monitor.scale_factor();
            let mpos = monitor.position();
            let msize = monitor.size();

            // 显示器物理区域
            let mx = mpos.x as f64;
            let my = mpos.y as f64;
            let mw = msize.width as f64;
            let mh = msize.height as f64;

            if cursor_pos.0 >= mx && cursor_pos.0 < mx + mw
                && cursor_pos.1 >= my && cursor_pos.1 < my + mh
            {
                // 鼠标在这个显示器上，计算居中位置（逻辑坐标）
                let logical_w = mw / scale;
                let logical_h = mh / scale;
                let logical_x = mx / scale;
                let logical_y = my / scale;

                let x = logical_x + (logical_w - width) / 2.0;
                let y = logical_y + (logical_h - height) / 2.0;
                return (x, y);
            }
        }

        // 鼠标不在任何已知显示器上，回退到主显示器
        if let Some(monitor) = monitors.first() {
            let scale = monitor.scale_factor();
            let mpos = monitor.position();
            let msize = monitor.size();
            let logical_w = msize.width as f64 / scale;
            let logical_h = msize.height as f64 / scale;
            let logical_x = mpos.x as f64 / scale;
            let logical_y = mpos.y as f64 / scale;
            let x = logical_x + (logical_w - width) / 2.0;
            let y = logical_y + (logical_h - height) / 2.0;
            return (x, y);
        }
    }

    // 默认位置
    (100.0, 100.0)
}

/// 获取鼠标光标的物理坐标位置
fn get_cursor_position() -> (f64, f64) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point).is_ok() {
                return (point.x as f64, point.y as f64);
            }
        }
    }
    (0.0, 0.0)
}

/// 创建偏好设置窗口
pub fn create_preferences_window(
    app: &AppHandle,
    initial_tab: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::WebviewWindowBuilder;

    let window_label = "preferences";

    // 如果窗口已存在，聚焦并切换标签页
    if let Some(window) = app.get_webview_window(window_label) {
        let _ = window.show();
        let _ = window.unminimize();
        bring_window_to_front(&window);
        if let Some(tab) = initial_tab {
            window.emit("switch-tab", tab)?;
        }
        return Ok(());
    }

    let width = 800.0;
    let height = 700.0;
    let (x, y) = get_window_position_on_current_screen(app, width, height);

    let url = if let Some(tab) = initial_tab {
        format!("preferences.html#{}", tab)
    } else {
        "preferences.html".to_string()
    };

    let _window = WebviewWindowBuilder::new(app, window_label, tauri::WebviewUrl::App(url.into()))
        .title("落笔 Nextype")
        .inner_size(width, height)
        .position(x, y)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .build()?;

    // Accessory 模式下新建窗口也需要强制前置
    if let Some(window) = app.get_webview_window(window_label) {
        bring_window_to_front(&window);
    }

    Ok(())
}

/// 创建日志窗口
pub fn create_logs_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::WebviewWindowBuilder;

    let window_label = "logs";

    // 如果窗口已存在，聚焦
    if let Some(window) = app.get_webview_window(window_label) {
        let _ = window.show();
        let _ = window.unminimize();
        bring_window_to_front(&window);
        return Ok(());
    }

    let width = 900.0;
    let height = 700.0;
    let (x, y) = get_window_position_on_current_screen(app, width, height);

    let _window = WebviewWindowBuilder::new(app, window_label, tauri::WebviewUrl::App("logs.html".into()))
        .title("系统日志")
        .inner_size(width, height)
        .position(x, y)
        .resizable(true)
        .minimizable(true)
        .maximizable(true)
        .build()?;

    // Accessory 模式下新建窗口也需要强制前置
    if let Some(window) = app.get_webview_window(window_label) {
        bring_window_to_front(&window);
    }

    tracing::info!("日志窗口已打开");
    Ok(())
}

/// 创建引导窗口
pub fn create_onboarding_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::WebviewWindowBuilder;

    let window_label = "onboarding";

    // 如果窗口已存在，聚焦
    if let Some(window) = app.get_webview_window(window_label) {
        let _ = window.show();
        let _ = window.unminimize();
        window.set_focus()?;
        return Ok(());
    }

    let width = 400.0;
    let height = 680.0;
    let (x, y) = get_window_position_on_current_screen(app, width, height);

    let _window = WebviewWindowBuilder::new(app, window_label, tauri::WebviewUrl::App("onboarding.html".into()))
        .title("欢迎使用落笔 Nextype")
        .inner_size(width, height)
        .position(x, y)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .build()?;

    Ok(())
}
