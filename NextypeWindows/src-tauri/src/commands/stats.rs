// ============================================================
// 统计数据 Commands
// ============================================================

use tauri::{Emitter, State};
use tauri_plugin_store::StoreExt;

use crate::services::stats::Statistics;
use crate::state::SharedAppState;

const STATS_STORE_KEY: &str = "stats";

/// 获取统计数据
#[tauri::command]
pub async fn get_stats(app: tauri::AppHandle, state: State<'_, SharedAppState>) -> Result<Statistics, String> {
    let config = state.get_config();

    // 如果统计功能被禁用，返回空数据
    if !config.enable_stats {
        return Ok(Statistics::default());
    }

    // 从 store 加载统计数据
    let store = app
        .store("stats.json")
        .map_err(|e| format!("无法打开统计数据存储: {}", e))?;

    if let Some(stats_value) = store.get(STATS_STORE_KEY) {
        let mut stats: Statistics =
            serde_json::from_value(stats_value.clone()).unwrap_or_default();

        // 检查并重置今日统计
        stats.check_and_reset_daily();

        // 一次性迁移：将字节数修正为字符数
        if !config.bytes_to_chars_migrated {
            migrate_bytes_to_chars(&mut stats);

            // 保存迁移后的统计数据
            save_stats_to_store(&app, &stats)?;

            // 标记迁移完成并保存配置
            state.update_config(|c| c.bytes_to_chars_migrated = true);
            let updated_config = state.get_config();
            let config_store = app
                .store("config.json")
                .map_err(|e| format!("无法打开配置存储: {}", e))?;
            config_store.set(
                "config",
                serde_json::to_value(&updated_config).map_err(|e| e.to_string())?,
            );
            config_store.save().map_err(|e| e.to_string())?;

            tracing::info!(
                "✅ 字符统计历史数据迁移完成，total_chars 已从字节数修正为字符数: {}",
                stats.total_chars
            );

            return Ok(stats);
        }

        // 如果日期有变化，保存回去
        save_stats_to_store(&app, &stats)?;

        Ok(stats)
    } else {
        // 新用户直接标记无需迁移
        if !config.bytes_to_chars_migrated {
            state.update_config(|c| c.bytes_to_chars_migrated = true);
            let updated_config = state.get_config();
            let config_store = app
                .store("config.json")
                .map_err(|e| format!("无法打开配置存储: {}", e))?;
            config_store.set(
                "config",
                serde_json::to_value(&updated_config).map_err(|e| e.to_string())?,
            );
            config_store.save().map_err(|e| e.to_string())?;
        }
        // 返回默认统计
        Ok(Statistics::default())
    }
}

/// 记录一次粘贴操作
#[tauri::command]
pub async fn record_paste(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    char_count: usize,
) -> Result<(), String> {
    let config = state.get_config();

    // 如果统计功能被禁用，直接返回
    if !config.enable_stats {
        return Ok(());
    }

    // 获取当前统计
    let mut stats = get_stats(app.clone(), state).await?;

    // 记录粘贴
    stats.record_paste(char_count);

    // 保存到 store
    save_stats_to_store(&app, &stats)?;

    // 触发前端更新事件
    let _ = app.emit("stats_updated", &stats);

    Ok(())
}

/// 重置统计数据
#[tauri::command]
pub async fn reset_stats(app: tauri::AppHandle) -> Result<(), String> {
    let mut stats = Statistics::default();
    stats.reset();

    save_stats_to_store(&app, &stats)?;

    // 触发前端更新事件
    let _ = app.emit("stats_updated", &stats);

    Ok(())
}

/// 设置是否启用统计
#[tauri::command]
pub async fn set_stats_enabled(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    enabled: bool,
) -> Result<(), String> {
    // 更新配置
    state.update_config(|c| c.enable_stats = enabled);

    // 保存配置
    let config = state.get_config();
    let config_store = app
        .store("config.json")
        .map_err(|e| format!("无法打开配置存储: {}", e))?;

    config_store
        .set("config", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    config_store.save().map_err(|e| e.to_string())?;

    tracing::info!("📊 统计功能已{}", if enabled { "启用" } else { "禁用" });

    Ok(())
}

/// 一次性迁移：将字节数修正为字符数
fn migrate_bytes_to_chars(stats: &mut Statistics) {
    const BYTES_TO_CHARS_RATIO: f64 = 0.357;

    // 修正 total_chars
    stats.total_chars = (stats.total_chars as f64 * BYTES_TO_CHARS_RATIO) as usize;

    // 修正每日历史记录
    for (_, record) in stats.daily_history.iter_mut() {
        record.chars = (record.chars as f64 * BYTES_TO_CHARS_RATIO) as usize;
    }

    // 修正 today_chars
    stats.today_chars = (stats.today_chars as f64 * BYTES_TO_CHARS_RATIO) as usize;

    tracing::info!(
        "🔧 字符统计迁移完成，新 total={}",
        stats.total_chars
    );
}

/// 获取最近 N 天的每日历史记录
#[tauri::command]
pub async fn get_daily_history(
    app: tauri::AppHandle,
    state: State<'_, SharedAppState>,
    days: usize,
) -> Result<std::collections::HashMap<String, crate::services::stats::DailyRecord>, String> {
    use chrono::{Datelike, Duration, Local};

    let config = state.get_config();
    if !config.enable_stats {
        return Ok(std::collections::HashMap::new());
    }

    let store = app
        .store("stats.json")
        .map_err(|e| format!("无法打开统计数据存储: {}", e))?;

    let stats: Statistics = if let Some(v) = store.get(STATS_STORE_KEY) {
        serde_json::from_value(v).unwrap_or_default()
    } else {
        Statistics::default()
    };

    // 只返回最近 days 天的数据
    let today = {
        let now = Local::now();
        format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
    };
    let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let cutoff = today_date - Duration::days(days as i64);

    let result = stats
        .daily_history
        .into_iter()
        .filter(|(k, _)| {
            chrono::NaiveDate::parse_from_str(k, "%Y-%m-%d")
                .map(|d| d > cutoff)
                .unwrap_or(false)
        })
        .collect();

    Ok(result)
}

/// 辅助函数：保存统计数据到 store
fn save_stats_to_store(app: &tauri::AppHandle, stats: &Statistics) -> Result<(), String> {
    let store = app
        .store("stats.json")
        .map_err(|e| format!("无法打开统计数据存储: {}", e))?;

    store.set(
        STATS_STORE_KEY,
        serde_json::to_value(stats).map_err(|e| e.to_string())?,
    );

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}
