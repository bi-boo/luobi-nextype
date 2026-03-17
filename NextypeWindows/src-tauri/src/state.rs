// ============================================================
// 全局状态管理
// ============================================================

use parking_lot::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::utils::config::AppConfig;

/// 应用全局状态
pub struct AppState {
    /// 配置（通过 tauri-plugin-store 持久化）
    pub config: RwLock<AppConfig>,

    /// 在线设备列表
    pub online_devices: RwLock<Vec<String>>,

    /// 托盘图标句柄（必须保持持有，否则图标会消失）
    pub tray_icon: RwLock<Option<tauri::tray::TrayIcon>>,

    /// 是否应该退出（用于区分窗口关闭和主动退出）
    pub should_quit: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: RwLock::new(AppConfig::default()),
            online_devices: RwLock::new(Vec::new()),
            tray_icon: RwLock::new(None),
            should_quit: AtomicBool::new(false),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取配置的只读引用
    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// 更新配置
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write();
        f(&mut config);
    }

    /// 添加在线设备
    pub fn add_online_device(&self, device_id: String) {
        let mut devices = self.online_devices.write();
        if !devices.contains(&device_id) {
            devices.push(device_id);
        }
    }

    /// 移除在线设备
    pub fn remove_online_device(&self, device_id: &str) {
        let mut devices = self.online_devices.write();
        devices.retain(|d| d != device_id);
    }

    /// 获取在线设备列表
    pub fn get_online_devices(&self) -> Vec<String> {
        self.online_devices.read().clone()
    }
}

/// 用于在 Tauri 中管理状态的包装类型
pub type SharedAppState = Arc<AppState>;

/// 创建共享状态
pub fn create_shared_state() -> SharedAppState {
    Arc::new(AppState::new())
}
