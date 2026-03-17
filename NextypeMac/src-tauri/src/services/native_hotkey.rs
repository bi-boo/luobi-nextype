// ============================================================
// macOS 原生快捷键支持（Fn 键录入与全局监听）
// ============================================================
//
// 浏览器 KeyboardEvent 和 tauri-plugin-global-shortcut 均不支持 Fn 键，
// 因此通过 macOS 原生 API 实现两个能力：
// 1. 录入：NSEvent.addLocalMonitorForEvents 捕获含 Fn 的按键（keyDown + flagsChanged）
// 2. 全局监听：CGEventTap 监听含 Fn 的全局快捷键（支持纯修饰键组合）
//
// 支持的快捷键类型：
// - Fn 单独（纯修饰键组合）
// - Fn + 其他修饰键（如 Fn+Ctrl）
// - Fn + 主键（如 Fn+1）
// - Fn + 修饰键 + 主键（如 Fn+Ctrl+1）

#![allow(non_upper_case_globals)]

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType,
};

// ============================================================
// macOS 修饰键的原始位掩码值
// ============================================================

const FN_KEY_FLAG: u64 = 0x800000; // kCGEventFlagMaskSecondaryFn
const CTRL_FLAG: u64 = 0x40000; // kCGEventFlagMaskControl
const ALT_FLAG: u64 = 0x80000; // kCGEventFlagMaskAlternate
const SHIFT_FLAG: u64 = 0x20000; // kCGEventFlagMaskShift
const CMD_FLAG: u64 = 0x100000; // kCGEventFlagMaskCommand

// ============================================================
// macOS 修饰键 keycode（用于区分 keyDown 和 flagsChanged）
// ============================================================

/// 判断 keycode 是否为修饰键（keycode 54-63）
fn is_modifier_keycode(keycode: u16) -> bool {
    // 54=RightCmd, 55=LeftCmd, 56=LeftShift, 57=CapsLock,
    // 58=LeftOption, 59=LeftCtrl, 60=RightShift, 61=RightOption,
    // 62=RightCtrl, 63=Fn
    (54..=63).contains(&keycode)
}

// ============================================================
// CGKeyCode → 键名 映射
// ============================================================

fn keycode_to_name(keycode: u16) -> Option<&'static str> {
    match keycode {
        29 => Some("0"),
        18 => Some("1"),
        19 => Some("2"),
        20 => Some("3"),
        21 => Some("4"),
        23 => Some("5"),
        22 => Some("6"),
        26 => Some("7"),
        28 => Some("8"),
        25 => Some("9"),
        0 => Some("A"),
        11 => Some("B"),
        8 => Some("C"),
        2 => Some("D"),
        14 => Some("E"),
        3 => Some("F"),
        5 => Some("G"),
        4 => Some("H"),
        34 => Some("I"),
        38 => Some("J"),
        40 => Some("K"),
        37 => Some("L"),
        46 => Some("M"),
        45 => Some("N"),
        31 => Some("O"),
        35 => Some("P"),
        12 => Some("Q"),
        15 => Some("R"),
        1 => Some("S"),
        17 => Some("T"),
        32 => Some("U"),
        9 => Some("V"),
        13 => Some("W"),
        7 => Some("X"),
        16 => Some("Y"),
        6 => Some("Z"),
        122 => Some("F1"),
        120 => Some("F2"),
        99 => Some("F3"),
        118 => Some("F4"),
        96 => Some("F5"),
        97 => Some("F6"),
        98 => Some("F7"),
        100 => Some("F8"),
        101 => Some("F9"),
        109 => Some("F10"),
        103 => Some("F11"),
        111 => Some("F12"),
        49 => Some("Space"),
        36 => Some("Enter"),
        76 => Some("Enter"), // Fn+Return 产生的 keycode（macOS 将 Return 转换为 numpad Enter）
        48 => Some("Tab"),
        53 => Some("Escape"),
        51 => Some("Backspace"),
        117 => Some("Delete"),
        123 => Some("Left"),
        124 => Some("Right"),
        125 => Some("Down"),
        126 => Some("Up"),
        27 => Some("-"),
        24 => Some("="),
        33 => Some("["),
        30 => Some("]"),
        42 => Some("\\"),
        41 => Some(";"),
        39 => Some("'"),
        43 => Some(","),
        47 => Some("."),
        44 => Some("/"),
        50 => Some("`"),
        115 => Some("Home"),
        119 => Some("End"),
        116 => Some("PageUp"),
        121 => Some("PageDown"),
        _ => None,
    }
}

fn name_to_keycode(name: &str) -> Option<u16> {
    match name {
        "0" => Some(29),
        "1" => Some(18),
        "2" => Some(19),
        "3" => Some(20),
        "4" => Some(21),
        "5" => Some(23),
        "6" => Some(22),
        "7" => Some(26),
        "8" => Some(28),
        "9" => Some(25),
        "A" => Some(0),
        "B" => Some(11),
        "C" => Some(8),
        "D" => Some(2),
        "E" => Some(14),
        "F" => Some(3),
        "G" => Some(5),
        "H" => Some(4),
        "I" => Some(34),
        "J" => Some(38),
        "K" => Some(40),
        "L" => Some(37),
        "M" => Some(46),
        "N" => Some(45),
        "O" => Some(31),
        "P" => Some(35),
        "Q" => Some(12),
        "R" => Some(15),
        "S" => Some(1),
        "T" => Some(17),
        "U" => Some(32),
        "V" => Some(9),
        "W" => Some(13),
        "X" => Some(7),
        "Y" => Some(16),
        "Z" => Some(6),
        "F1" => Some(122),
        "F2" => Some(120),
        "F3" => Some(99),
        "F4" => Some(118),
        "F5" => Some(96),
        "F6" => Some(97),
        "F7" => Some(98),
        "F8" => Some(100),
        "F9" => Some(101),
        "F10" => Some(109),
        "F11" => Some(103),
        "F12" => Some(111),
        "Space" => Some(49),
        "Enter" => Some(36),
        "Tab" => Some(48),
        "Escape" => Some(53),
        "Backspace" => Some(51),
        "Delete" => Some(117),
        "Left" => Some(123),
        "Right" => Some(124),
        "Down" => Some(125),
        "Up" => Some(126),
        "-" => Some(27),
        "=" => Some(24),
        "[" => Some(33),
        "]" => Some(30),
        "\\" => Some(42),
        ";" => Some(41),
        "'" => Some(39),
        "," => Some(43),
        "." => Some(47),
        "/" => Some(44),
        "`" => Some(50),
        "Home" => Some(115),
        "End" => Some(119),
        "PageUp" => Some(116),
        "PageDown" => Some(121),
        _ => None,
    }
}

// ============================================================
// 修饰键状态
// ============================================================

#[derive(Debug, Clone, Default, serde::Serialize)]
struct ModifierState {
    r#fn: bool,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

impl ModifierState {
    /// 从原始标志位读取修饰键状态
    fn from_raw_flags(raw: u64) -> Self {
        Self {
            r#fn: (raw & FN_KEY_FLAG) != 0,
            ctrl: (raw & CTRL_FLAG) != 0,
            alt: (raw & ALT_FLAG) != 0,
            shift: (raw & SHIFT_FLAG) != 0,
            meta: (raw & CMD_FLAG) != 0,
        }
    }

    /// 是否有任何修饰键被按下
    fn has_any(&self) -> bool {
        self.r#fn || self.ctrl || self.alt || self.shift || self.meta
    }

    /// 将另一个状态合并进来（取并集，用于 peak 追踪）
    fn merge(&mut self, other: &Self) {
        self.r#fn |= other.r#fn;
        self.ctrl |= other.ctrl;
        self.alt |= other.alt;
        self.shift |= other.shift;
        self.meta |= other.meta;
    }

    /// 与另一个状态是否完全匹配
    fn matches(&self, other: &Self) -> bool {
        self.r#fn == other.r#fn
            && self.ctrl == other.ctrl
            && self.alt == other.alt
            && self.shift == other.shift
            && self.meta == other.meta
    }
}

// ============================================================
// accelerator 构建/解析
// ============================================================

/// 构建 accelerator 字符串，key_name 为 None 表示纯修饰键组合
fn build_accelerator(modifiers: &ModifierState, key_name: Option<&str>) -> String {
    let mut parts = Vec::new();
    if modifiers.r#fn {
        parts.push("Fn");
    }
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.meta {
        parts.push("CommandOrControl");
    }
    if let Some(key) = key_name {
        parts.push(key);
    }
    parts.join("+")
}

/// 构建显示字符串（macOS 符号）
fn build_display(modifiers: &ModifierState, key_name: Option<&str>) -> String {
    let mut parts = Vec::new();
    if modifiers.r#fn {
        parts.push("Fn".to_string());
    }
    if modifiers.ctrl {
        parts.push("⌃".to_string());
    }
    if modifiers.alt {
        parts.push("⌥".to_string());
    }
    if modifiers.shift {
        parts.push("⇧".to_string());
    }
    if modifiers.meta {
        parts.push("⌘".to_string());
    }
    if let Some(key) = key_name {
        let display_key = match key {
            "Space" => "␣",
            "Backspace" => "⌫",
            "Delete" => "⌦",
            "Enter" => "↵",
            "Tab" => "⇥",
            "Up" => "↑",
            "Down" => "↓",
            "Left" => "←",
            "Right" => "→",
            other => other,
        };
        parts.push(display_key.to_string());
    }
    parts.join(" ")
}

/// 从 accelerator 字符串解析出修饰键和可选主键
fn parse_accelerator(accelerator: &str) -> Option<(ModifierState, Option<u16>)> {
    let parts: Vec<&str> = accelerator.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = ModifierState::default();
    let mut keycode: Option<u16> = None;

    for &part in &parts {
        match part {
            "Fn" => modifiers.r#fn = true,
            "Ctrl" => modifiers.ctrl = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            "CommandOrControl" => modifiers.meta = true,
            key_name => {
                if let Some(kc) = name_to_keycode(key_name) {
                    keycode = Some(kc);
                } else {
                    return None; // 无效键名
                }
            }
        }
    }

    // 至少需要一个修饰键
    if !modifiers.has_any() {
        return None;
    }

    Some((modifiers, keycode))
}

/// 检查两个 keycode 是否等价
/// macOS 下 Fn+Return(36) 会产生 numpad Enter(76)，需要视为同一个键
fn keycodes_equivalent(a: u16, b: u16) -> bool {
    if a == b {
        return true;
    }
    // Return(36) 和 numpad Enter(76) 互相等价
    (a == 36 && b == 76) || (a == 76 && b == 36)
}

/// 检查 accelerator 是否包含 Fn 键
pub fn is_fn_shortcut(accelerator: &str) -> bool {
    accelerator == "Fn" || accelerator.starts_with("Fn+")
}

// ============================================================
// 事件 payload
// ============================================================

#[derive(Clone, serde::Serialize)]
struct HotkeyRecordedPayload {
    accelerator: String,
    display: String,
}

#[derive(Clone, serde::Serialize)]
struct ModifierDisplayPayload {
    display: String,
}

// ============================================================
// 录入内部状态
// ============================================================

#[derive(Default)]
struct RecordingInternalState {
    /// 录入期间见到的修饰键并集（取 peak，用于纯修饰键组合）
    peak_modifiers: ModifierState,
    /// 录入期间是否有非修饰键被按下
    had_key_down: bool,
}

// ============================================================
// 全局监听内部状态
// ============================================================

#[derive(Default)]
struct GlobalMonitorInternalState {
    /// 上一次的修饰键状态
    previous_modifiers: ModifierState,
    /// 自 Fn 按下以来是否有主键被按下
    had_key_down_since_fn: bool,
    /// 当前按住的长按动作名（用于 KeyUp 时发送 touch_up）
    active_longpress: Option<String>,
    /// 上次 keyDown 触发的时间戳（用于过滤 Fn+Return 产生的重复 keyDown）
    last_keydown_trigger: Option<std::time::Instant>,
}

// ============================================================
// 注册项
// ============================================================

#[derive(Clone)]
struct FnShortcutEntry {
    modifiers: ModifierState,
    keycode: Option<u16>, // None = 纯修饰键组合
    action: String,
    is_longpress: bool, // 是否为长按快捷键（需要 KeyUp 支持）
}

// ============================================================
// NativeFnHotkey - 主结构体
// ============================================================

pub struct NativeFnHotkey {
    registered: Arc<RwLock<Vec<FnShortcutEntry>>>,
    recording_monitor: Arc<RwLock<Option<*mut std::ffi::c_void>>>,
    recording_state: Arc<RwLock<RecordingInternalState>>,
    tap_runloop: Arc<RwLock<Option<CFRunLoop>>>,
    /// 标记是否有等待权限的监听器线程（用于应用激活时触发重试）
    pending_listener: Arc<std::sync::atomic::AtomicBool>,
    /// 用于唤醒等待权限的线程
    pending_notify: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    app_handle: AppHandle,
}

// AXIsProcessTrusted: 轻量检查辅助功能权限（不弹窗）
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// 检查当前进程是否已获得辅助功能权限
fn is_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

unsafe impl Send for NativeFnHotkey {}
unsafe impl Sync for NativeFnHotkey {}

impl NativeFnHotkey {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            registered: Arc::new(RwLock::new(Vec::new())),
            recording_monitor: Arc::new(RwLock::new(None)),
            recording_state: Arc::new(RwLock::new(RecordingInternalState::default())),
            tap_runloop: Arc::new(RwLock::new(None)),
            pending_listener: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_notify: Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new())),
            app_handle,
        }
    }

    // ========================================
    // 录入功能（NSEvent local monitor）
    // ========================================

    /// 开始录入：安装 NSEvent 本地键盘事件监听器
    /// 同时监听 keyDown (1<<10) 和 flagsChanged (1<<12)
    pub fn start_recording(&self) -> Result<(), String> {
        self.stop_recording()?;

        // 重置录入状态
        *self.recording_state.write() = RecordingInternalState::default();

        let app_handle = self.app_handle.clone();
        let monitor_ref = self.recording_monitor.clone();
        let recording_state = self.recording_state.clone();

        unsafe {
            use objc2::rc::Retained;
            use objc2_app_kit::NSEvent;
            use objc2_foundation::NSObject;

            // NSEventMask: keyDown (1<<10) | flagsChanged (1<<12)
            let mask: u64 = (1 << 10) | (1 << 12);

            let handler = block2::RcBlock::new(move |event: *mut NSEvent| -> *mut NSEvent {
                if event.is_null() {
                    return event;
                }

                let event_ref: &NSEvent = &*event;
                let keycode = event_ref.keyCode();
                let raw_flags = event_ref.modifierFlags().bits() as u64;
                let current_mods = ModifierState::from_raw_flags(raw_flags);

                // ---- flagsChanged 事件（修饰键按下/释放） ----
                if is_modifier_keycode(keycode) {
                    let mut state = recording_state.write();

                    if current_mods.has_any() {
                        // 有修饰键按下：合并到 peak，发送实时显示
                        state.peak_modifiers.merge(&current_mods);

                        let display_str = build_display(&current_mods, None);
                        let _ = app_handle.emit(
                            "hotkey-recording-modifiers",
                            ModifierDisplayPayload {
                                display: display_str,
                            },
                        );
                    } else {
                        // 所有修饰键释放：如果 peak 包含 Fn 且没按过主键，完成录入
                        if state.peak_modifiers.r#fn && !state.had_key_down {
                            let accelerator =
                                build_accelerator(&state.peak_modifiers, None);
                            let display_str =
                                build_display(&state.peak_modifiers, None);

                            tracing::info!(
                                "🎹 原生录入（纯修饰键）: accelerator={}, display={}",
                                accelerator,
                                display_str
                            );

                            let _ = app_handle.emit(
                                "hotkey-recorded",
                                HotkeyRecordedPayload {
                                    accelerator,
                                    display: display_str,
                                },
                            );
                        }
                        // 重置 peak（无论是否触发）
                        state.peak_modifiers = ModifierState::default();
                        state.had_key_down = false;
                    }

                    // 放行 flagsChanged 事件，不吞掉（避免破坏系统修饰键追踪）
                    return event;
                }

                // ---- keyDown 事件（非修饰键） ----
                {
                    let mut state = recording_state.write();
                    state.had_key_down = true;
                }

                // Escape 没有修饰键时：放行给 JS 处理
                if keycode == 53 && !current_mods.has_any() {
                    return event;
                }

                // 至少要有一个修饰键（包括 Fn）
                if !current_mods.has_any() {
                    // 没有修饰键的普通按键：吞掉，防止在输入框中打字
                    return std::ptr::null_mut();
                }

                // 有修饰键 + 主键：完成录入
                let key_name = match keycode_to_name(keycode) {
                    Some(name) => name,
                    None => return std::ptr::null_mut(), // 未知键，吞掉
                };

                let accelerator = build_accelerator(&current_mods, Some(key_name));
                let display_str = build_display(&current_mods, Some(key_name));

                tracing::info!(
                    "🎹 原生录入（修饰键+主键）: keycode={}, key={}, accelerator={}, display={}",
                    keycode,
                    key_name,
                    accelerator,
                    display_str
                );

                let _ = app_handle.emit(
                    "hotkey-recorded",
                    HotkeyRecordedPayload {
                        accelerator,
                        display: display_str,
                    },
                );

                // 吞掉事件，防止触发其他快捷键或输入字符
                std::ptr::null_mut()
            });

            let monitor: Option<Retained<NSObject>> = objc2::msg_send![
                objc2::class!(NSEvent),
                addLocalMonitorForEventsMatchingMask: mask,
                handler: &*handler
            ];

            if let Some(monitor_obj) = monitor {
                let raw_ptr = Retained::into_raw(monitor_obj) as *mut std::ffi::c_void;
                *monitor_ref.write() = Some(raw_ptr);
                tracing::info!("🎹 原生按键录入监听器已启动（keyDown + flagsChanged）");
            } else {
                return Err("安装 NSEvent 本地监听器失败".to_string());
            }
        }

        Ok(())
    }

    /// 停止录入
    pub fn stop_recording(&self) -> Result<(), String> {
        let mut monitor_ref = self.recording_monitor.write();
        if let Some(monitor_ptr) = monitor_ref.take() {
            unsafe {
                use objc2_foundation::NSObject;
                let monitor_obj =
                    objc2::rc::Retained::from_raw(monitor_ptr as *mut NSObject);
                if let Some(obj) = monitor_obj {
                    let _: () =
                        objc2::msg_send![objc2::class!(NSEvent), removeMonitor: &*obj];
                    tracing::info!("🎹 原生按键录入监听器已停止");
                }
            }
        }
        Ok(())
    }

    // ========================================
    // 全局监听功能（CGEventTap）
    // ========================================

    /// 注册一个快捷键到全局监听列表
    /// is_longpress=true 时允许非 Fn 快捷键也注册到 CGEventTap（需要 KeyUp 支持）
    pub fn register(&self, action: String, accelerator: String, is_longpress: bool) -> Result<(), String> {
        let (modifiers, keycode) = parse_accelerator(&accelerator)
            .ok_or_else(|| format!("无法解析快捷键: {}", accelerator))?;

        if !modifiers.r#fn && !is_longpress {
            return Err("此方法仅用于含 Fn 的快捷键或 longpress 快捷键".to_string());
        }

        // 不在此处 unregister —— 调用方 HotkeyManager::register() 已在循环前统一清理，
        // 此处若清理会导致同一 action 的多个快捷键只保留最后一个
        let entry = FnShortcutEntry {
            modifiers,
            keycode,
            action,
            is_longpress,
        };

        self.registered.write().push(entry);
        tracing::info!("✅ 快捷键已注册到 CGEventTap: {} (longpress={})", accelerator, is_longpress);
        Ok(())
    }

    pub fn unregister(&self, action: &str) {
        self.registered.write().retain(|e| e.action != action);
    }

    /// 启动 CGEventTap 全局监听线程
    /// 使用 Default 模式以便消费匹配的事件（阻止原始按键动作）
    /// 如果缺少辅助功能权限，会自动等待权限授予后重试（指数退避，最长 5 分钟）
    pub fn start_global_listener(&self) -> Result<(), String> {
        self.stop_global_listener();

        let registered = self.registered.clone();
        let app_handle = self.app_handle.clone();
        let runloop_ref = self.tap_runloop.clone();
        let pending_listener = self.pending_listener.clone();
        let pending_notify = self.pending_notify.clone();

        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            use std::time::{Duration, Instant};

            let max_wait = Duration::from_secs(5 * 60); // 最长等待 5 分钟
            let start_time = Instant::now();
            let mut interval = Duration::from_secs(2); // 初始 2 秒
            let max_interval = Duration::from_secs(30); // 最大 30 秒

            // 如果没有辅助功能权限，先等待权限授予
            if !is_accessibility_trusted() {
                tracing::warn!(
                    "⚠️ 缺少辅助功能权限，CGEventTap 将在权限授予后自动启动"
                );
                pending_listener.store(true, Ordering::SeqCst);

                loop {
                    if start_time.elapsed() >= max_wait {
                        tracing::warn!(
                            "⏰ 等待辅助功能权限超时（5 分钟），停止自动重试。请授权后重启应用"
                        );
                        pending_listener.store(false, Ordering::SeqCst);
                        return;
                    }

                    // 等待：支持被 notify_accessibility_change 唤醒
                    {
                        let (lock, cvar) = &*pending_notify;
                        let mut notified = lock.lock().unwrap_or_else(|e| e.into_inner());
                        if !*notified {
                            let _ = cvar.wait_timeout(notified, interval).unwrap_or_else(|e| e.into_inner());
                        } else {
                            *notified = false;
                        }
                    }

                    if is_accessibility_trusted() {
                        tracing::info!("✅ 辅助功能权限已授予，正在启动 CGEventTap...");
                        break;
                    }

                    // 指数退避
                    interval = (interval * 2).min(max_interval);
                }

                pending_listener.store(false, Ordering::SeqCst);
            }

            // 权限已就绪，创建 CGEventTap
            let monitor_state = RwLock::new(GlobalMonitorInternalState::default());

            let tap = CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default, // Default 模式，可以消费事件
                vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
                move |_proxy, _event_type, event| {
                    let flags = event.get_flags();
                    let raw_flags = flags.bits();
                    let current_mods = ModifierState {
                        r#fn: (raw_flags & FN_KEY_FLAG) != 0,
                        ctrl: flags.contains(CGEventFlags::CGEventFlagControl),
                        alt: flags.contains(CGEventFlags::CGEventFlagAlternate),
                        shift: flags.contains(CGEventFlags::CGEventFlagShift),
                        meta: flags.contains(CGEventFlags::CGEventFlagCommand),
                    };

                    let keycode_raw = event.get_integer_value_field(
                        core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                    ) as u16;
                    let is_flags_changed = is_modifier_keycode(keycode_raw);

                    // 用 _event_type 的原始值区分 KeyDown(10) / KeyUp(1) / FlagsChanged(12)
                    let event_type_raw = _event_type as u32;
                    let is_key_up = event_type_raw == 1; // kCGEventKeyUp

                    // ---- KeyUp：处理 longpress 释放 ----
                    if is_key_up {
                        let mut state = monitor_state.write();
                        if let Some(ref active_action) = state.active_longpress.clone() {
                            tracing::info!("🔔 长按快捷键释放 (KeyUp): {}", active_action);
                            state.active_longpress = None;

                            let app_inner = app_handle.clone();
                            let action_inner = active_action.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) =
                                    super::hotkey_manager::handle_hotkey_release_public(
                                        &app_inner,
                                        &action_inner,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "❌ 处理长按释放失败: {}",
                                        e
                                    );
                                }
                            });
                            return None;
                        }
                        return Some(event.clone());
                    }

                    if is_flags_changed {
                        // ---- flagsChanged：处理纯修饰键快捷键 ----
                        let mut state = monitor_state.write();

                        // 如果有 active_longpress 且修饰键不再匹配，释放长按
                        if let Some(ref active_action) = state.active_longpress.clone() {
                            // 检查是否有纯修饰键 longpress 条目不再匹配
                            let entries = registered.read();
                            let still_matches = entries.iter().any(|entry| {
                                entry.is_longpress
                                    && entry.keycode.is_none()
                                    && entry.action == *active_action
                                    && entry.modifiers.matches(&current_mods)
                            });
                            if !still_matches {
                                tracing::info!("🔔 长按快捷键释放 (FlagsChanged): {}", active_action);
                                let app_inner = app_handle.clone();
                                let action_inner = active_action.clone();
                                state.active_longpress = None;
                                drop(entries);
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) =
                                        super::hotkey_manager::handle_hotkey_release_public(
                                            &app_inner,
                                            &action_inner,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "❌ 处理长按释放失败: {}",
                                            e
                                        );
                                    }
                                });
                                state.previous_modifiers = current_mods;
                                return None;
                            }
                        }

                        // Fn 刚按下时重置 key_down 追踪
                        if current_mods.r#fn && !state.previous_modifiers.r#fn {
                            state.had_key_down_since_fn = false;
                        }

                        // ============================================================
                        // 修复：当修饰键按下后匹配纯修饰键快捷键时，剥离修饰键标志
                        // 防止 Fn+Shift 等组合键的副作用被触发
                        // 注意：只有当没有主键被按下时才剥离（避免影响 Fn+Shift+A 等组合）
                        // ============================================================
                        {
                            let entries = registered.read();
                            // 检查当前修饰键组合是否完全匹配一个纯修饰键快捷键（非 longpress）
                            // 且之前的修饰键状态不匹配该快捷键（表示这是按下新修饰键的瞬间）
                            // 且没有主键被按下（避免影响正常的 Fn+Shift+A 等组合）
                            let matched_shortcut = entries.iter().find(|entry| {
                                entry.keycode.is_none()                                // 纯修饰键
                                    && !entry.is_longpress                            // 非 longpress
                                    && entry.modifiers.matches(&current_mods)         // 精确匹配当前状态
                                    && !entry.modifiers.matches(&state.previous_modifiers)  // 之前不匹配（按下新修饰键）
                                    && !state.had_key_down_since_fn                  // 没有主键被按下
                            });

                            if let Some(entry) = matched_shortcut {
                                tracing::debug!(
                                    "🛡️ 屏蔽 flagsChanged 事件，防止 {} 副作用",
                                    entry.action
                                );
                                // macOS 对 flagsChanged 事件的 None 消费可能不生效，
                                // 因此采用双重策略：
                                // 1. 剥离所有修饰键标志（只保留 Fn）
                                // 2. 修改 keycode 为无害值
                                let raw_flags = event.get_flags().bits();
                                let fn_only = raw_flags & FN_KEY_FLAG;
                                use core_graphics::event::CGEventFlags;
                                event.set_flags(CGEventFlags::from_bits_truncate(fn_only));
                                event.set_integer_value_field(
                                    core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                                    0xFFFF,
                                );
                                state.previous_modifiers = current_mods;
                                return Some(event.clone());
                            }
                        }

                        // 检查纯修饰键快捷键
                        if state.previous_modifiers.r#fn && !state.had_key_down_since_fn {
                            let entries = registered.read();
                            for entry in entries.iter() {
                                if entry.keycode.is_some() {
                                    continue;
                                }
                                if entry.modifiers.matches(&state.previous_modifiers)
                                    && !entry.modifiers.matches(&current_mods)
                                {
                                    // longpress 纯修饰键：不在释放时触发（已在按下时处理）
                                    if entry.is_longpress {
                                        continue;
                                    }

                                    tracing::info!(
                                        "🔔 Fn 纯修饰键快捷键触发: {}",
                                        entry.action
                                    );
                                    let _ = app_handle
                                        .emit("hotkey_triggered", &entry.action);

                                    let app_inner = app_handle.clone();
                                    let action_inner = entry.action.clone();
                                    tauri::async_runtime::spawn(async move {
                                        if let Err(e) =
                                            super::hotkey_manager::handle_hotkey_press_public(
                                                &app_inner,
                                                &action_inner,
                                            )
                                            .await
                                        {
                                            tracing::error!(
                                                "❌ 处理 Fn 快捷键动作失败: {}",
                                                e
                                            );
                                        }
                                    });
                                    break;
                                }
                            }
                        }

                        // 检查纯修饰键 longpress 按下（修饰键刚匹配上）
                        // 同时剥离修饰键标志，防止副作用
                        {
                            let entries = registered.read();
                            for entry in entries.iter() {
                                if !entry.is_longpress || entry.keycode.is_some() {
                                    continue;
                                }
                                if entry.modifiers.matches(&current_mods)
                                    && !entry.modifiers.matches(&state.previous_modifiers)
                                    && state.active_longpress.is_none()
                                {
                                    tracing::info!(
                                        "🔔 长按快捷键按下 (FlagsChanged): {}",
                                        entry.action
                                    );
                                    state.active_longpress = Some(entry.action.clone());

                                    let _ = app_handle
                                        .emit("hotkey_triggered", &entry.action);

                                    let app_inner = app_handle.clone();
                                    let action_inner = entry.action.clone();
                                    tauri::async_runtime::spawn(async move {
                                        if let Err(e) =
                                            super::hotkey_manager::handle_hotkey_press_public(
                                                &app_inner,
                                                &action_inner,
                                            )
                                            .await
                                        {
                                            tracing::error!(
                                                "❌ 处理长按按下失败: {}",
                                                e
                                            );
                                        }
                                    });
                                    // 屏蔽修饰键副作用：剥离标志 + 修改 keycode
                                    let raw_flags = event.get_flags().bits();
                                    let fn_only = raw_flags & FN_KEY_FLAG;
                                    use core_graphics::event::CGEventFlags;
                                    event.set_flags(CGEventFlags::from_bits_truncate(fn_only));
                                    event.set_integer_value_field(
                                        core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                                        0xFFFF,
                                    );
                                    state.previous_modifiers = current_mods;
                                    return Some(event.clone());
                                }
                            }
                        }

                        state.previous_modifiers = current_mods;
                        // 放行 flagsChanged 事件
                        return Some(event.clone());
                    }

                    // ---- keyDown：处理 修饰键+主键 快捷键 ----
                    let fn_held_from_flags;
                    {
                        let mut state = monitor_state.write();
                        state.had_key_down_since_fn = true;
                        // 从 flagsChanged 追踪的状态获取 Fn 是否按住
                        // macOS 在 Fn+Return 时可能从 keyDown 事件的 flags 中剥离 Fn 标志，
                        // 但 flagsChanged 事件已经记录了 Fn 按下
                        fn_held_from_flags = state.previous_modifiers.r#fn;

                        // 如果已有 active_longpress，过滤 OS 按键重复
                        if state.active_longpress.is_some() {
                            return None;
                        }

                        // 过滤 Fn+Return 产生的重复 keyDown（keycode 36 和 76 间隔极短）
                        if let Some(last) = state.last_keydown_trigger {
                            if last.elapsed().as_millis() < 50 {
                                return None;
                            }
                        }
                    }

                    let entries = registered.read();
                    for entry in entries.iter() {
                        if let Some(entry_keycode) = entry.keycode {
                            // 判断 Fn 是否实际按住：
                            // 1. 当前事件 flags 包含 Fn
                            // 2. flagsChanged 追踪到 Fn 按住（macOS 可能从 keyDown flags 中剥离 Fn）
                            // 3. keycode 76 (numpad Enter) 只能由 Fn+Return 产生
                            let fn_implied = current_mods.r#fn
                                || fn_held_from_flags
                                || (keycode_raw == 76 && keycodes_equivalent(entry_keycode, keycode_raw));

                            // 非 Fn 的 longpress 条目不要求 Fn 键
                            let fn_ok = if entry.modifiers.r#fn {
                                fn_implied
                            } else {
                                !current_mods.r#fn || entry.is_longpress
                            };

                            if keycodes_equivalent(entry_keycode, keycode_raw)
                                && fn_ok
                                && entry.modifiers.ctrl == current_mods.ctrl
                                && entry.modifiers.alt == current_mods.alt
                                && entry.modifiers.shift == current_mods.shift
                                && entry.modifiers.meta == current_mods.meta
                            {
                                tracing::info!(
                                    "🔔 快捷键触发: {} (longpress={}, keycode={})",
                                    entry.action,
                                    entry.is_longpress,
                                    keycode_raw
                                );
                                let _ = app_handle
                                    .emit("hotkey_triggered", &entry.action);

                                if entry.is_longpress {
                                    let mut state = monitor_state.write();
                                    state.active_longpress = Some(entry.action.clone());
                                    state.last_keydown_trigger = Some(std::time::Instant::now());
                                } else {
                                    let mut state = monitor_state.write();
                                    state.last_keydown_trigger = Some(std::time::Instant::now());
                                }

                                let app_inner = app_handle.clone();
                                let action_inner = entry.action.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) =
                                        super::hotkey_manager::handle_hotkey_press_public(
                                            &app_inner,
                                            &action_inner,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "❌ 处理快捷键动作失败: {}",
                                            e
                                        );
                                    }
                                });

                                // 消费事件���修改 keycode 为无害值并返回 None
                                // 双重保险：即使 None 未能消费事件，修改后的 keycode 也不会产生回车
                                event.set_integer_value_field(
                                    core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                                    0xFFFF,
                                );
                                return None;
                            }
                        }
                    }

                    // 未匹配：放行事件
                    Some(event.clone())
                },
            );

            match tap {
                Ok(tap) => {
                    unsafe {
                        let loop_source =
                            match tap.mach_port.create_runloop_source(0) {
                                Ok(src) => src,
                                Err(_) => {
                                    tracing::error!("❌ 创建 RunLoop source 失败");
                                    return;
                                }
                            };
                        let current_loop = CFRunLoop::get_current();
                        current_loop.add_source(&loop_source, kCFRunLoopCommonModes);
                        tap.enable();

                        *runloop_ref.write() = Some(current_loop.clone());

                        tracing::info!(
                            "🌐 CGEventTap 全局快捷键监听已启动（Default 模式，keyDown + keyUp + flagsChanged）"
                        );

                        CFRunLoop::run_current();
                    }
                }
                Err(()) => {
                    tracing::error!(
                        "❌ 创建 CGEventTap 失败（权限已授予但创建仍失败）"
                    );
                }
            }
        });

        Ok(())
    }

    pub fn stop_global_listener(&self) {
        let mut runloop_ref = self.tap_runloop.write();
        if let Some(runloop) = runloop_ref.take() {
            runloop.stop();
            tracing::info!("🌐 CGEventTap 全局 Fn 快捷键监听已停止");
        }
    }

    pub fn registered_count(&self) -> usize {
        self.registered.read().len()
    }

    /// 通知辅助功能权限可能已变更（应用激活时调用）
    /// 如果有等待权限的监听器线程，立即唤醒它重试
    pub fn notify_accessibility_change(&self) {
        if self.pending_listener.load(std::sync::atomic::Ordering::SeqCst) {
            let (lock, cvar) = &*self.pending_notify;
            let mut notified = lock.lock().unwrap_or_else(|e| e.into_inner());
            *notified = true;
            cvar.notify_one();
            tracing::info!("🔔 已通知等待中的 CGEventTap 线程重新检查权限");
        }
    }
}

impl Drop for NativeFnHotkey {
    fn drop(&mut self) {
        let _ = self.stop_recording();
        self.stop_global_listener();
    }
}
