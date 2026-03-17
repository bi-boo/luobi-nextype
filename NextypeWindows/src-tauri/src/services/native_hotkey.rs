// ============================================================
// Windows 原生快捷键支持
// ============================================================
//
// Windows 平台使用 tauri-plugin-global-shortcut 处理全局快捷键
// 不需要像 macOS 那样实现复杂的 Fn 键支持

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct NativeHotkeyManager {
    app: AppHandle,
}

impl NativeHotkeyManager {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// 开始快捷键录入（Windows 使用前端实现）
    pub fn start_recording(&self) -> Result<(), String> {
        tracing::info!("Windows 平台使用前端 JavaScript 录入快捷键");
        Ok(())
    }

    /// 停止快捷键录入
    pub fn stop_recording(&self) -> Result<(), String> {
        Ok(())
    }
}

/// 创建原生快捷键管理器
pub fn create_native_hotkey_manager(app: AppHandle) -> Arc<RwLock<Option<NativeHotkeyManager>>> {
    let manager = NativeHotkeyManager::new(app);
    Arc::new(RwLock::new(Some(manager)))
}
