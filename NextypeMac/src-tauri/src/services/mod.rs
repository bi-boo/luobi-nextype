// ============================================================
// 服务模块
// ============================================================

pub mod clipboard;
pub mod device_manager;
pub mod hotkey_manager;
#[cfg(target_os = "macos")]
pub mod native_hotkey;
pub mod relay_client;
pub mod stats;
pub mod tray;
