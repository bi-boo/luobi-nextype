// ============================================================
// 统计数据服务
// ============================================================

use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 每日记录
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyRecord {
    pub chars: usize,
    pub pastes: usize,
}

/// 统计数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    /// 累计同步字符数
    pub total_chars: usize,
    /// 累计同步次数
    pub total_pastes: usize,
    /// 今日同步字符数
    pub today_chars: usize,
    /// 最后更新日期 (YYYY-MM-DD)
    pub last_date: String,
    /// 每日历史记录，key 为 "YYYY-MM-DD"
    #[serde(default)]
    pub daily_history: HashMap<String, DailyRecord>,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            total_chars: 0,
            total_pastes: 0,
            today_chars: 0,
            last_date: Self::current_date(),
            daily_history: HashMap::new(),
        }
    }
}

impl Statistics {
    /// 获取当前日期字符串 (YYYY-MM-DD)
    fn current_date() -> String {
        let now = Local::now();
        format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
    }

    /// 检查并重置今日统计（如果日期变更），并清理 365 天前的旧记录
    pub fn check_and_reset_daily(&mut self) {
        let today = Self::current_date();
        if today != self.last_date {
            tracing::info!(
                "📅 日期已从 {} 变更为 {}，正在重置今日使用量统计",
                self.last_date,
                today
            );
            self.today_chars = 0;
            self.last_date = today.clone();

            // 清理 365 天前的旧记录
            if let Ok(today_date) = NaiveDate::parse_from_str(&today, "%Y-%m-%d") {
                let cutoff = today_date - Duration::days(365);
                self.daily_history.retain(|k, _| {
                    NaiveDate::parse_from_str(k, "%Y-%m-%d")
                        .map(|d| d >= cutoff)
                        .unwrap_or(false)
                });
            }
        }
    }

    /// 记录一次粘贴操作
    pub fn record_paste(&mut self, char_count: usize) {
        self.check_and_reset_daily();

        self.total_chars += char_count;
        self.total_pastes += 1;
        self.today_chars += char_count;

        // 同步写入每日历史
        let today = self.last_date.clone();
        let entry = self.daily_history.entry(today).or_default();
        entry.chars += char_count;
        entry.pastes += 1;

        tracing::info!(
            "📈 使用量统计更新: +{} 字符，今日累计: {}",
            char_count,
            self.today_chars
        );
    }

    /// 重置所有统计数据
    pub fn reset(&mut self) {
        self.total_chars = 0;
        self.total_pastes = 0;
        self.today_chars = 0;
        self.last_date = Self::current_date();
        self.daily_history.clear();
        tracing::warn!("⚠️ 统计数据已手动清空");
    }
}
