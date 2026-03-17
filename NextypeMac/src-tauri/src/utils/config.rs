// ============================================================
// 配置管理器
// 功能：使用 tauri-plugin-store 持久化配置
// ============================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 自定义反序列化函数：兼容旧格式(字符串)和新格式(数组)
fn deserialize_hotkeys<'de, D>(deserializer: D) -> Result<HashMap<String, Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Object(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                match v {
                    // 新格式：数组
                    serde_json::Value::Array(arr) => {
                        let vec: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        result.insert(k, vec);
                    }
                    // 旧格式：字符串，自动迁移到第一组
                    serde_json::Value::String(s) => {
                        result.insert(k, vec![s]);
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        _ => Ok(HashMap::new()),
    }
}

/// 信任的设备信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustedDevice {
    pub id: String,
    /// 设备名称（与 Electron 版本的 name 字段对齐）
    pub name: String,
    #[serde(default)]
    pub last_ip: String,
    #[serde(default)]
    pub last_seen: i64,
    /// 设备类型（与 Electron 对齐）
    #[serde(default)]
    pub device_type: String,
    /// 配对时间（与 Electron 对齐）
    #[serde(default)]
    pub paired_at: String,
    /// 配对时协商的共享加密密钥（hex 编码的 256 位随机密钥）
    #[serde(default)]
    pub encryption_key: Option<String>,
}

/// 应用配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    // 按钮配置
    #[serde(default = "default_true")]
    pub enable_btn1: bool,
    #[serde(default = "default_btn1_text")]
    pub btn1_text: String,
    #[serde(default)]
    pub btn1_suffix: String,
    #[serde(default = "default_true")]
    pub enable_btn2: bool,
    #[serde(default = "default_btn2_text")]
    pub btn2_text: String,
    #[serde(default)]
    pub btn2_suffix: String,

    // 显示选项
    #[serde(default = "default_true")]
    pub show_dock_icon: bool,
    #[serde(default = "default_true")]
    pub show_menu_bar_icon: bool,
    #[serde(default)]
    pub auto_launch: bool,

    // 远程连接配置
    #[serde(default = "default_true")]
    pub enable_remote_connection: bool,
    #[serde(default = "default_relay_server_url")]
    pub relay_server_url: String,

    // 设备信息
    #[serde(default = "generate_device_id")]
    pub device_id: String,
    #[serde(default = "default_device_name")]
    pub device_name: String,

    // 安全配置
    #[serde(default)]
    pub trusted_devices: Vec<TrustedDevice>,

    // 快捷键配置（支持每个 action 最多 2 组快捷键）
    #[serde(default, deserialize_with = "deserialize_hotkeys")]
    pub hotkeys: HashMap<String, Vec<String>>,

    // 统计功能配置
    #[serde(default = "default_true")]
    pub enable_stats: bool,

    // 清空剪贴板配置
    #[serde(default)]
    pub clear_after_paste: bool,

    // 点击坐标配置 (JSON 数组)
    #[serde(default)]
    pub tap_coordinates: serde_json::Value,

    // 长按坐标配置 (JSON 数组，与 tap_coordinates 同结构)
    #[serde(default)]
    pub longpress_coordinates: serde_json::Value,

    // 模拟长按完成后自动插入
    #[serde(default)]
    pub longpress_auto_insert: bool,
    #[serde(default = "default_auto_insert_delay")]
    pub longpress_auto_insert_delay: u64,

    // 版本信息
    #[serde(default = "default_version")]
    pub version: String,

    // 历史数据迁移标记：将 UTF-8 字节数修正为 Unicode 字符数（一次性）
    #[serde(default)]
    pub bytes_to_chars_migrated: bool,
}

// 默认值函数
fn default_true() -> bool {
    true
}

fn default_btn1_text() -> String {
    "同步".to_string()
}

fn default_btn2_text() -> String {
    "发送".to_string()
}

fn default_relay_server_url() -> String {
    "wss://nextypeapi.yuanfengai.cn:8443".to_string()
}

fn default_auto_insert_delay() -> u64 {
    300
}

fn default_device_name() -> String {
    // 优先使用 macOS "电脑名称"（系统设置 > 通用 > 共享）
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("scutil")
            .args(["--get", "ComputerName"])
            .output()
        {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }
    // 回退：使用 hostname
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Mac".to_string())
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// 生成稳定的设备 ID（与 Electron 版本一致）
/// 使用 hostname + username + platform 生成，确保每次启动 ID 不变
fn generate_device_id() -> String {
    use sha2::{Digest, Sha256};

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let platform = std::env::consts::OS; // "macos", "windows", "linux"

    let input = format!("{}-{}-{}", hostname, username, platform);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();

    hex::encode(&result[..8]) // 取前 8 字节 = 16 位十六进制字符串，与 Electron 一致
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enable_btn1: true,
            btn1_text: "同步".to_string(),
            btn1_suffix: String::new(),
            enable_btn2: true,
            btn2_text: "发送".to_string(),
            btn2_suffix: String::new(),
            show_dock_icon: true,
            show_menu_bar_icon: true,
            auto_launch: false,
            enable_remote_connection: true,
            relay_server_url: "wss://nextypeapi.yuanfengai.cn:8443".to_string(),
            device_id: generate_device_id(),
            device_name: default_device_name(),
            trusted_devices: Vec::new(),
            hotkeys: HashMap::new(),
            enable_stats: true,
            clear_after_paste: false,
            tap_coordinates: serde_json::Value::Array(Vec::new()),
            longpress_coordinates: serde_json::Value::Array(Vec::new()),
            longpress_auto_insert: false,
            longpress_auto_insert_delay: 300,
            version: "1.0.0".to_string(),
            bytes_to_chars_migrated: false,
        }
    }
}

impl AppConfig {
    /// 添加信任设备
    pub fn add_trusted_device(&mut self, device: TrustedDevice) {
        // 拒绝无效设备
        if device.name.is_empty() || device.name == "Unknown Device" {
            tracing::warn!("拒绝添加无效设备: {}", device.id);
            return;
        }
        // 如果设备已存在，更新它（不用空值覆盖已有的非空字段）
        if let Some(existing) = self.trusted_devices.iter_mut().find(|d| d.id == device.id) {
            if !device.name.is_empty() {
                existing.name = device.name;
            }
            if !device.last_ip.is_empty() {
                existing.last_ip = device.last_ip;
            }
            if device.last_seen > 0 {
                existing.last_seen = device.last_seen;
            }
            if !device.device_type.is_empty() {
                existing.device_type = device.device_type;
            }
            if !device.paired_at.is_empty() {
                existing.paired_at = device.paired_at;
            }
            if device.encryption_key.is_some() {
                existing.encryption_key = device.encryption_key;
            }
        } else {
            self.trusted_devices.push(device);
        }
    }

    /// 移除信任设备
    pub fn remove_trusted_device(&mut self, device_id: &str) -> bool {
        let len_before = self.trusted_devices.len();
        self.trusted_devices.retain(|d| d.id != device_id);
        self.trusted_devices.len() < len_before
    }

    /// 检查设备是否受信任
    pub fn is_device_trusted(&self, device_id: &str) -> bool {
        self.trusted_devices.iter().any(|d| d.id == device_id)
    }

    /// 获取信任设备
    pub fn get_trusted_device(&self, device_id: &str) -> Option<&TrustedDevice> {
        self.trusted_devices.iter().find(|d| d.id == device_id)
    }
}
