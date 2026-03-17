use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::Level;
use tracing_subscriber::Layer;
use once_cell::sync::Lazy;
use chrono::Local;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub struct TauriLogger {
    app_handle: Arc<RwLock<Option<AppHandle>>>,
    log_cache: Arc<RwLock<Vec<LogEntry>>>,
    log_file_path: PathBuf,
    max_cache_size: usize,
}

// 全局静态实例，用于获取缓存数据
static LOG_CACHE: Lazy<Arc<RwLock<Vec<LogEntry>>>> = Lazy::new(|| Arc::new(RwLock::new(Vec::with_capacity(1000))));
static APP_HANDLE: Lazy<Arc<RwLock<Option<AppHandle>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

impl TauriLogger {
    pub fn new(app_name: &str) -> Self {
        // 解析日志目录 (对齐 Electron 的路径习惯)
        // macOS: ~/Library/Logs/<app_name>/clipboard-sync.log
        let log_dir = if cfg!(target_os = "macos") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Library/Logs").join(app_name)
        } else {
            // Windows/Linux 退而求其次使用应用本地目录或 temp
            std::env::current_dir().unwrap_or_default().join("logs")
        };

        if !log_dir.exists() {
            let _ = fs::create_dir_all(&log_dir);
        }

        let log_file_path = log_dir.join("clipboard-sync.log");

        Self {
            app_handle: APP_HANDLE.clone(),
            log_cache: LOG_CACHE.clone(),
            log_file_path,
            max_cache_size: 1000,
        }
    }

    pub fn setup_app_handle(app_handle: AppHandle) {
        let mut handle = APP_HANDLE.write();
        *handle = Some(app_handle);
    }

    pub fn get_logs() -> Vec<LogEntry> {
        LOG_CACHE.read().clone()
    }

    pub fn clear_logs() {
        let mut cache = LOG_CACHE.write();
        cache.clear();

        // 同时尝试清空文件
        if let Ok(_handle) = APP_HANDLE.read().as_ref().ok_or("No AppHandle") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let log_file = PathBuf::from(home).join("Library/Logs/nextype-tauri/clipboard-sync.log");
            if log_file.exists() {
                let _ = fs::write(log_file, "");
            }
        }
    }

    fn write_to_file(&self, level: &str, message: &str) {
        // 日志轮转：超过 10MB 时归档旧文件（与 Electron 对齐）
        if let Ok(metadata) = fs::metadata(&self.log_file_path) {
            if metadata.len() > 10 * 1024 * 1024 {
                let old_path = self.log_file_path.with_extension("log.old");
                let _ = fs::rename(&self.log_file_path, &old_path);
            }
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let log_line = format!("[{}] [{}] {}\n", timestamp, level, message);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    }
}

impl<S> Layer<S> for TauriLogger
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        
        // 我们只处理 INFO 级别及以上的日志
        if *metadata.level() > Level::INFO {
            return;
        }

        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let entry = LogEntry {
            timestamp,
            level: level.clone(),
            message: message.clone(),
        };

        // 1. 写入内存缓存
        {
            let mut cache = self.log_cache.write();
            cache.push(entry.clone());
            if cache.len() > self.max_cache_size {
                cache.remove(0);
            }
        }

        // 2. 写入文件
        self.write_to_file(&level, &message);

        // 3. 实时推送 (如果 AppHandle 已就绪)
        if let Some(app) = self.app_handle.read().as_ref() {
            let _ = app.emit("new_log_entry", entry);
        }
    }
}

#[derive(Default)]
struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // 移除调试格式带来的额外引号 (如果是字符串)
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
