use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use crate::modules;

const DEFAULT_PROMPT: &str = "hi";
const RESET_TRIGGER_COOLDOWN_MS: i64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupTaskInput {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_run_at: Option<i64>,
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleConfig {
    pub repeat_mode: String,
    pub daily_times: Option<Vec<String>>,
    pub weekly_days: Option<Vec<i32>>,
    pub weekly_times: Option<Vec<String>>,
    pub interval_hours: Option<i32>,
    pub interval_start_time: Option<String>,
    pub interval_end_time: Option<String>,
    pub selected_models: Vec<String>,
    pub selected_accounts: Vec<String>,
    pub crontab: Option<String>,
    pub wake_on_reset: Option<bool>,
    pub custom_prompt: Option<String>,
    pub max_output_tokens: Option<i32>,
    pub time_window_enabled: Option<bool>,
    pub time_window_start: Option<String>,
    pub time_window_end: Option<String>,
    pub reset_threshold: Option<i32>,
    pub check_interval_minutes: Option<i32>,
    pub max_wake_count: Option<i32>,
}

#[derive(Debug, Clone)]
struct WakeupTask {
    id: String,
    name: String,
    enabled: bool,
    #[allow(dead_code)]
    created_at: i64,
    last_run_at: Option<i64>,
    schedule: ScheduleConfigNormalized,
}

#[derive(Debug, Clone)]
struct ScheduleConfigNormalized {
    repeat_mode: String,
    daily_times: Vec<String>,
    weekly_days: Vec<i32>,
    weekly_times: Vec<String>,
    interval_hours: i32,
    interval_start_time: String,
    interval_end_time: String,
    selected_models: Vec<String>,
    selected_accounts: Vec<String>,
    crontab: Option<String>,
    wake_on_reset: bool,
    custom_prompt: Option<String>,
    max_output_tokens: i32,
    time_window_enabled: bool,
    time_window_start: Option<String>,
    time_window_end: Option<String>,
    reset_threshold: i32,
    check_interval_minutes: i32,
    max_wake_count: i32,
}

#[derive(Default, Debug, Clone)]
struct ResetState {
    last_reset_trigger_timestamps: HashMap<String, String>,
    last_reset_trigger_at: HashMap<String, i64>,
    last_reset_remaining: HashMap<String, i32>,
}

#[derive(Default, Clone)]
struct SchedulerState {
    enabled: bool,
    tasks: Vec<WakeupTask>,
    running_tasks: HashSet<String>,
    reset_states: HashMap<String, ResetState>,
    /// 记录每个任务的实际执行时间，不会被前端 sync_state 覆盖
    last_executed_at: HashMap<String, i64>,
}

static STATE: OnceLock<Mutex<SchedulerState>> = OnceLock::new();
static STARTED: OnceLock<Mutex<bool>> = OnceLock::new();

fn state() -> &'static Mutex<SchedulerState> {
    STATE.get_or_init(|| Mutex::new(SchedulerState::default()))
}

fn started_flag() -> &'static Mutex<bool> {
    STARTED.get_or_init(|| Mutex::new(false))
}

fn normalize_schedule(raw: ScheduleConfig) -> ScheduleConfigNormalized {
    let daily_times = raw
        .daily_times
        .filter(|times| !times.is_empty())
        .unwrap_or_else(|| vec!["08:00".to_string()]);
    let weekly_days = raw
        .weekly_days
        .filter(|days| !days.is_empty())
        .unwrap_or_else(|| vec![1, 2, 3, 4, 5]);
    let weekly_times = raw
        .weekly_times
        .filter(|times| !times.is_empty())
        .unwrap_or_else(|| vec!["08:00".to_string()]);
    let interval_hours = raw.interval_hours.unwrap_or(4).max(1);
    let interval_start_time = raw
        .interval_start_time
        .unwrap_or_else(|| "07:00".to_string());
    let interval_end_time = raw.interval_end_time.unwrap_or_else(|| "22:00".to_string());
    let max_output_tokens = raw.max_output_tokens.unwrap_or(0).max(0);
    ScheduleConfigNormalized {
        repeat_mode: raw.repeat_mode,
        daily_times,
        weekly_days,
        weekly_times,
        interval_hours,
        interval_start_time,
        interval_end_time,
        selected_models: raw.selected_models,
        selected_accounts: raw.selected_accounts,
        crontab: raw.crontab,
        wake_on_reset: raw.wake_on_reset.unwrap_or(false),
        custom_prompt: raw.custom_prompt,
        max_output_tokens,
        time_window_enabled: raw.time_window_enabled.unwrap_or(false),
        time_window_start: raw.time_window_start,
        time_window_end: raw.time_window_end,
        reset_threshold: raw.reset_threshold.unwrap_or(100).clamp(0, 100),
        check_interval_minutes: raw.check_interval_minutes.unwrap_or(10).clamp(5, 60),
        max_wake_count: raw.max_wake_count.unwrap_or(0).max(0),
    }
}

pub fn sync_state(enabled: bool, tasks: Vec<WakeupTaskInput>) {
    let mut guard = state().lock().expect("wakeup state lock");
    guard.enabled = enabled;
    guard.tasks = tasks
        .into_iter()
        .map(|task| WakeupTask {
            id: task.id,
            name: task.name,
            enabled: task.enabled,
            created_at: task.created_at,
            last_run_at: task.last_run_at,
            schedule: normalize_schedule(task.schedule),
        })
        .collect();
}

pub fn ensure_started(app: AppHandle) {
    let mut started = started_flag().lock().expect("wakeup started lock");
    if *started {
        return;
    }
    *started = true;

    tauri::async_runtime::spawn(async move {
        loop {
            run_scheduler_once(&app).await;
            // 每分钟整点边界醒来，确保不漏掉任何 5/10/20/30/40/50/60 分钟的对齐点
            let now = Local::now();
            let cur_sec = now.second();
            let wait_secs = 60u64.saturating_sub(cur_sec as u64).max(1);
            sleep(Duration::from_secs(wait_secs)).await;
        }
    });
}

fn parse_time_to_minutes(value: &str) -> Option<i32> {
    let parts: Vec<&str> = value.trim().split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: i32 = parts[0].parse().ok()?;
    let m: i32 = parts[1].parse().ok()?;
    if h < 0 || h > 23 || m < 0 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

fn is_in_time_window(start: Option<&String>, end: Option<&String>, now: DateTime<Local>) -> bool {
    let Some(start) = start else {
        return true;
    };
    let Some(end) = end else {
        return true;
    };
    let Some(start_minutes) = parse_time_to_minutes(start) else {
        return true;
    };
    let Some(end_minutes) = parse_time_to_minutes(end) else {
        return true;
    };
    let current_minutes = (now.hour() as i32) * 60 + now.minute() as i32;

    if start_minutes <= end_minutes {
        current_minutes >= start_minutes && current_minutes < end_minutes
    } else {
        current_minutes >= start_minutes || current_minutes < end_minutes
    }
}

fn next_run_time(
    schedule: &ScheduleConfigNormalized,
    after: DateTime<Local>,
) -> Option<DateTime<Local>> {
    let mut results: Vec<DateTime<Local>> = Vec::new();
    if schedule.repeat_mode == "daily" && !schedule.daily_times.is_empty() {
        let mut times = schedule.daily_times.clone();
        times.sort();
        for day_offset in 0..7 {
            for time in &times {
                if let Some(candidate) = build_datetime(after, day_offset, &time) {
                    if candidate > after {
                        results.push(candidate);
                        if !results.is_empty() {
                            return results.into_iter().min();
                        }
                    }
                }
            }
        }
    } else if schedule.repeat_mode == "weekly"
        && !schedule.weekly_days.is_empty()
        && !schedule.weekly_times.is_empty()
    {
        let mut times = schedule.weekly_times.clone();
        times.sort();
        for day_offset in 0..14 {
            let date = after + chrono::Duration::days(day_offset);
            let weekday = date.weekday().num_days_from_sunday() as i32;
            if schedule.weekly_days.contains(&weekday) {
                for time in &times {
                    if let Some(candidate) = build_datetime_from_date(date, &time) {
                        if candidate > after {
                            results.push(candidate);
                            if !results.is_empty() {
                                return results.into_iter().min();
                            }
                        }
                    }
                }
            }
        }
    } else if schedule.repeat_mode == "interval" {
        let start_time = schedule.interval_start_time.clone();
        let end_hour: i32 = schedule
            .interval_end_time
            .split(':')
            .next()
            .and_then(|h| h.parse().ok())
            .unwrap_or(22);
        let interval = schedule.interval_hours.max(1);

        for day_offset in 0..7 {
            for h in (parse_time_to_minutes(&start_time).unwrap_or(0) / 60..=end_hour)
                .step_by(interval as usize)
            {
                let time = format!(
                    "{:02}:{:02}",
                    h,
                    parse_time_to_minutes(&start_time).unwrap_or(0) % 60
                );
                if let Some(candidate) = build_datetime(after, day_offset, &time) {
                    if candidate > after {
                        results.push(candidate);
                        if !results.is_empty() {
                            return results.into_iter().min();
                        }
                    }
                }
            }
        }
    }
    None
}

fn build_datetime(base: DateTime<Local>, day_offset: i64, time: &str) -> Option<DateTime<Local>> {
    let date = base + chrono::Duration::days(day_offset);
    build_datetime_from_date(date, time)
}

fn build_datetime_from_date(date: DateTime<Local>, time: &str) -> Option<DateTime<Local>> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let naive_date = date.date_naive();
    let naive = naive_date.and_hms_opt(h, m, 0)?;
    Local.from_local_datetime(&naive).single()
}

fn next_crontab_time(expr: &str, after: DateTime<Local>) -> Option<DateTime<Local>> {
    let parts: Vec<&str> = expr.trim().split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let minutes = parse_cron_field(parts[0], 59)?;
    let hours = parse_cron_field(parts[1], 23)?;

    for day_offset in 0..7 {
        for h in &hours {
            for m in &minutes {
                let candidate = build_datetime(after, day_offset, &format!("{:02}:{:02}", h, m));
                if let Some(candidate) = candidate {
                    if candidate > after {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

fn parse_cron_field(field: &str, max: i32) -> Option<Vec<i32>> {
    if field == "*" {
        return Some((0..=max).collect());
    }
    if field.contains(',') {
        let mut result = Vec::new();
        for part in field.split(',') {
            result.push(part.parse().ok()?);
        }
        return Some(result);
    }
    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() != 2 {
            return None;
        }
        let start: i32 = parts[0].parse().ok()?;
        let end: i32 = parts[1].parse().ok()?;
        if end < start {
            return None;
        }
        return Some((start..=end).collect());
    }
    if field.contains('/') {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let step: i32 = parts[1].parse().ok()?;
        if step <= 0 {
            return None;
        }
        let mut result = Vec::new();
        let mut value = 0;
        while value <= max {
            result.push(value);
            value += step;
        }
        return Some(result);
    }
    let value: i32 = field.parse().ok()?;
    Some(vec![value])
}

/// 与前端 matchModelName 保持一致的模型名称匹配逻辑：
/// normalize（全小写、只保留 a-z0-9）后，精确相等 或 有前缀包含关系 视为匹配。
/// 同时内置旧名称 → 新名称 的别名映射，与前端 MODEL_MATCH_REPLACEMENTS 对齐。
fn normalize_model_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn apply_model_alias(name: &str) -> String {
    let lower_name = name.trim().to_lowercase();
    let aliased = match lower_name.as_str() {
        "gemini-3-pro-high"             => "gemini-3.1-pro-high",
        "gemini-3-pro-low"              => "gemini-3.1-pro-low",
        "claude-sonnet-4-5"             => "claude-sonnet-4-6",
        "claude-sonnet-4-5-thinking"    => "claude-sonnet-4-6",
        "claude-opus-4-5-thinking"      => "claude-opus-4-6-thinking",
        _                               => lower_name.as_str(),
    };
    aliased.to_string()
}

fn model_name_matches(quota_model_name: &str, task_model_id: &str) -> bool {
    let left  = normalize_model_name(&apply_model_alias(quota_model_name));
    let right = normalize_model_name(&apply_model_alias(task_model_id));
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right
        || left.starts_with(&format!("{}.", right.replace('.', "")))
            // 宽泛前缀：left 以 right 开头（或反之），用于 x.1 系列兼容
        || left.starts_with(&right)
        || right.starts_with(&left)
}

fn resolve_custom_prompt(custom_prompt: Option<&String>) -> String {
    custom_prompt
        .and_then(|p| {
            let trimmed = p.trim();
            if trimmed.is_empty() {
                None
            } else if trimmed.contains('|') {
                let candidates: Vec<&str> = trimmed
                    .split('|')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if candidates.is_empty() {
                    None
                } else {
                    let mut rng = rand::thread_rng();
                    candidates.choose(&mut rng).map(|s| s.to_string())
                }
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| DEFAULT_PROMPT.to_string())
}

fn normalize_max_tokens(value: i32) -> u32 {
    if value > 0 {
        value as u32
    } else {
        0
    }
}

fn should_trigger_on_reset(
    state: &mut ResetState,
    model_key: &str,
    reset_at: &str,
    remaining_percent: i32,
    threshold: i32,
) -> bool {
    // ── 1. reset_time 必须是有效的时间戳 ──────────────────────────────────
    let reset_ts = match DateTime::parse_from_rfc3339(reset_at).map(|dt| dt.timestamp_millis()) {
        Ok(ts) => ts,
        Err(_) => return false,
    };

    // ── 2. 重置时间必须已过期（即重置事件已真正发生）───────────────────────
    let now = chrono::Utc::now().timestamp_millis();
    if reset_ts > now {
        // reset_time 还在未来，说明该帐号还没到重置时间，更新记录以便后续阈值判断
        state
            .last_reset_remaining
            .insert(model_key.to_string(), remaining_percent);
        return false;
    }

    // ── 3. 同一个 reset_time 只触发一次 ─────────────────────────────────
    if state.last_reset_trigger_timestamps.get(model_key) == Some(&reset_at.to_string()) {
        return false;
    }

    // ── 4. 冷却检查：10 分钟内不重复触发 ────────────────────────────────
    if let Some(last_trigger_at) = state.last_reset_trigger_at.get(model_key) {
        if now - *last_trigger_at < RESET_TRIGGER_COOLDOWN_MS {
            return false;
        }
    }

    // ── 5. 阈值检查：重置前的剩余额度需满足配置条件 ─────────────────────
    //    prev_remaining 记录了上次该 reset_time 未过期时观测到的剩余额度
    let prev_remaining = state.last_reset_remaining.get(model_key).copied();
    let meets_threshold = match prev_remaining {
        Some(prev) => prev <= threshold,
        // 无历史记录（程序刚启动 / 首次遇到该帐号）：
        // 无法知道重置前的剩余额度，直接触发一次。
        // 这样才能保证"打开程序时已重置的帐号"能被立即唤醒。
        None => true,
    };
    if !meets_threshold {
        // 阈值不满足，标记此 reset_at 防止反复判断
        state
            .last_reset_trigger_timestamps
            .insert(model_key.to_string(), reset_at.to_string());
        return false;
    }

    true
}

fn mark_reset_triggered(state: &mut ResetState, model_key: &str, reset_at: &str) {
    state
        .last_reset_trigger_timestamps
        .insert(model_key.to_string(), reset_at.to_string());
    state
        .last_reset_trigger_at
        .insert(model_key.to_string(), chrono::Utc::now().timestamp_millis());
}

async fn run_scheduler_once(app: &AppHandle) {
    let snapshot = {
        let guard = state().lock().expect("wakeup state lock");
        guard.clone()
    };

    if !snapshot.enabled {
        return;
    }

    let now = Local::now();
    let total_minutes = now.hour() as i64 * 60 + now.minute() as i64;

    for task in snapshot.tasks.iter() {
        if !task.enabled {
            continue;
        }
        if snapshot.running_tasks.contains(&task.id) {
            continue;
        }

        if task.schedule.wake_on_reset {
            // 按各任务自身的 check_interval_minutes 对齐触发
            let interval = task.schedule.check_interval_minutes as i64;
            if interval > 0 && total_minutes % interval == 0 {
                handle_quota_reset_task(app, task, now).await;
            }
            continue;
        }

        // 优先使用本地记录的执行时间，避免被前端同步覆盖导致重复执行
        let local_last_run = snapshot.last_executed_at.get(&task.id).copied();
        let after = local_last_run
            .or(task.last_run_at)
            .and_then(|ts| Local.timestamp_millis_opt(ts).single())
            .unwrap_or_else(|| now - chrono::Duration::minutes(1));

        let next_run = if let Some(expr) = &task.schedule.crontab {
            next_crontab_time(expr, after)
        } else {
            next_run_time(&task.schedule, after)
        };

        // 只有到达预定时间才触发（不再提前30秒）
        if let Some(next_run) = next_run {
            if next_run <= now {
                run_task(app, task, "scheduled").await;
            }
        }
    }
}

async fn handle_quota_reset_task(app: &AppHandle, task: &WakeupTask, now: DateTime<Local>) {
    if task.schedule.time_window_enabled
        && !is_in_time_window(
            task.schedule.time_window_start.as_ref(),
            task.schedule.time_window_end.as_ref(),
            now,
        )
    {
        return;
    }

    let accounts = match modules::list_accounts() {
        Ok(list) => list,
        Err(_) => return,
    };

    let selected_accounts: Vec<_> = task
        .schedule
        .selected_accounts
        .iter()
        .filter_map(|email| {
            accounts
                .iter()
                .find(|acc| acc.email.eq_ignore_ascii_case(email))
        })
        .collect();

    if selected_accounts.is_empty() {
        return;
    }

    let trigger_map = {
        let mut state_guard = state().lock().expect("wakeup state lock");
        let reset_state = state_guard
            .reset_states
            .entry(task.id.clone())
            .or_insert_with(ResetState::default);

        let mut trigger_map: HashMap<String, HashSet<String>> = HashMap::new();
        for model_id in &task.schedule.selected_models {
            for account in &selected_accounts {
                let model_key = format!("{}:{}", account.email, model_id);
                let quota_models = account
                    .quota
                    .as_ref()
                    .map(|q| q.models.as_slice())
                    .unwrap_or(&[]);
                if let Some(quota) = quota_models.iter().find(|item| model_name_matches(&item.name, model_id)) {
                    if should_trigger_on_reset(
                        reset_state,
                        &model_key,
                        &quota.reset_time,
                        quota.percentage,
                        task.schedule.reset_threshold,
                    ) {
                        trigger_map
                            .entry(account.email.clone())
                            .or_default()
                            .insert(model_id.clone());
                        mark_reset_triggered(reset_state, &model_key, &quota.reset_time);
                    }
                }
            }
        }
        trigger_map
    };

    if trigger_map.is_empty() {
        // 写入空结果历史，确保每次调度留痕
        let noop_item = modules::wakeup_history::WakeupHistoryItem {
            id: format!("{}-noop", chrono::Utc::now().timestamp_millis()),
            timestamp: chrono::Utc::now().timestamp_millis(),
            trigger_type: "auto".to_string(),
            trigger_source: "quota_reset".to_string(),
            task_name: Some(task.name.clone()),
            account_email: String::new(),
            model_id: String::new(),
            prompt: None,
            success: true,
            message: Some("无符合条件帐号".to_string()),
            duration: Some(0),
        };
        if let Err(e) = modules::wakeup_history::add_history_items(vec![noop_item.clone()]) {
            modules::logger::log_error(&format!("写入空结果唤醒历史失败: {}", e));
        }
        let payload = WakeupTaskResultPayload {
            task_id: task.id.clone(),
            last_run_at: chrono::Utc::now().timestamp_millis(),
            records: vec![noop_item],
        };
        let _ = app.emit("wakeup://task-result", payload);
        return;
    }

    // 按 max_wake_count 截断（0 = 不限）
    let max_wake = task.schedule.max_wake_count;
    let final_trigger_map = if max_wake > 0 {
        // 确定性排序：按 email 字母序
        let mut sorted_emails: Vec<String> = trigger_map.keys().cloned().collect();
        sorted_emails.sort();
        let mut limited_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut count = 0i32;
        for email in sorted_emails {
            if count >= max_wake {
                break;
            }
            if let Some(models) = trigger_map.get(&email) {
                limited_map.insert(email, models.clone());
                count += 1;
            }
        }
        limited_map
    } else {
        trigger_map
    };

    if !final_trigger_map.is_empty() {
        run_task_with_trigger_map(app, task, final_trigger_map).await;
    }
}

async fn run_task_with_trigger_map(
    app: &AppHandle,
    task: &WakeupTask,
    trigger_map: HashMap<String, HashSet<String>>,
) {
    if trigger_map.is_empty() {
        return;
    }

    let accounts = match modules::list_accounts() {
        Ok(list) => list,
        Err(_) => return,
    };

    {
        let mut guard = state().lock().expect("wakeup state lock");
        guard.running_tasks.insert(task.id.clone());
    }

    let max_tokens = normalize_max_tokens(task.schedule.max_output_tokens);

    let mut history: Vec<modules::wakeup_history::WakeupHistoryItem> = Vec::new();
    for (email, model_ids) in &trigger_map {
        let account = match accounts
            .iter()
            .find(|acc| acc.email.eq_ignore_ascii_case(email))
        {
            Some(acc) => acc,
            None => continue,
        };
        for model in model_ids {
            let prompt = resolve_custom_prompt(task.schedule.custom_prompt.as_ref());
            let started = chrono::Utc::now();
            let result =
                modules::wakeup::trigger_wakeup(&account.id, model, &prompt, max_tokens).await;
            let duration = chrono::Utc::now()
                .signed_duration_since(started)
                .num_milliseconds()
                .max(0) as u64;
            let (success, message) = match result {
                Ok(resp) => (true, Some(resp.reply)),
                Err(err) => (false, Some(err.to_string())),
            };
            history.push(modules::wakeup_history::WakeupHistoryItem {
                id: format!(
                    "{}-{}",
                    chrono::Utc::now().timestamp_millis(),
                    history.len()
                ),
                timestamp: chrono::Utc::now().timestamp_millis(),
                trigger_type: "auto".to_string(),
                trigger_source: "quota_reset".to_string(),
                task_name: Some(task.name.clone()),
                account_email: account.email.clone(),
                model_id: model.clone(),
                prompt: Some(prompt.clone()),
                success,
                message,
                duration: Some(duration),
            });
        }
    }

    {
        let mut guard = state().lock().expect("wakeup state lock");
        guard.running_tasks.remove(&task.id);
        let executed_at = chrono::Utc::now().timestamp_millis();
        guard.tasks.iter_mut().for_each(|item| {
            if item.id == task.id {
                item.last_run_at = Some(executed_at);
            }
        });
        // 记录本地执行时间，防止被前端同步覆盖导致重复执行
        guard.last_executed_at.insert(task.id.clone(), executed_at);
    }

    // 写入历史文件
    if let Err(e) = modules::wakeup_history::add_history_items(history.clone()) {
        modules::logger::log_error(&format!("写入唤醒历史失败: {}", e));
    }

    // 异步刷新被成功唤醒帐号的配额，使帐号列表状态及时更新
    let woken_emails: HashSet<String> = history
        .iter()
        .filter(|item| item.success)
        .map(|item| item.account_email.clone())
        .collect();
    if !woken_emails.is_empty() {
        tokio::spawn(async move {
            refresh_woken_account_quotas(woken_emails).await;
        });
    }

    let payload = WakeupTaskResultPayload {
        task_id: task.id.clone(),
        last_run_at: chrono::Utc::now().timestamp_millis(),
        records: history,
    };
    let _ = app.emit("wakeup://task-result", payload);
}

async fn run_task(app: &AppHandle, task: &WakeupTask, trigger_source: &str) {
    run_task_with_models(
        app,
        task,
        trigger_source,
        task.schedule.selected_models.clone(),
    )
    .await;
}

async fn run_task_with_models(
    app: &AppHandle,
    task: &WakeupTask,
    trigger_source: &str,
    models: Vec<String>,
) {
    if models.is_empty() {
        return;
    }

    let accounts = match modules::list_accounts() {
        Ok(list) => list,
        Err(_) => return,
    };

    let selected_accounts: Vec<_> = task
        .schedule
        .selected_accounts
        .iter()
        .filter_map(|email| {
            accounts
                .iter()
                .find(|acc| acc.email.eq_ignore_ascii_case(email))
        })
        .collect();

    if selected_accounts.is_empty() {
        return;
    }

    {
        let mut guard = state().lock().expect("wakeup state lock");
        guard.running_tasks.insert(task.id.clone());
    }

    let max_tokens = normalize_max_tokens(task.schedule.max_output_tokens);

    let mut history: Vec<modules::wakeup_history::WakeupHistoryItem> = Vec::new();
    for account in &selected_accounts {
        for model in &models {
            let prompt = resolve_custom_prompt(task.schedule.custom_prompt.as_ref());
            let started = chrono::Utc::now();
            let result =
                modules::wakeup::trigger_wakeup(&account.id, model, &prompt, max_tokens).await;
            let duration = chrono::Utc::now()
                .signed_duration_since(started)
                .num_milliseconds()
                .max(0) as u64;
            let (success, message) = match result {
                Ok(resp) => (true, Some(resp.reply)),
                Err(err) => (false, Some(err.to_string())),
            };
            history.push(modules::wakeup_history::WakeupHistoryItem {
                id: format!(
                    "{}-{}",
                    chrono::Utc::now().timestamp_millis(),
                    history.len()
                ),
                timestamp: chrono::Utc::now().timestamp_millis(),
                trigger_type: "auto".to_string(),
                trigger_source: trigger_source.to_string(),
                task_name: Some(task.name.clone()),
                account_email: account.email.clone(),
                model_id: model.clone(),
                prompt: Some(prompt.clone()),
                success,
                message,
                duration: Some(duration),
            });
        }
    }

    {
        let mut guard = state().lock().expect("wakeup state lock");
        guard.running_tasks.remove(&task.id);
        let executed_at = chrono::Utc::now().timestamp_millis();
        guard.tasks.iter_mut().for_each(|item| {
            if item.id == task.id {
                item.last_run_at = Some(executed_at);
            }
        });
        // 记录本地执行时间，防止被前端同步覆盖导致重复执行
        guard.last_executed_at.insert(task.id.clone(), executed_at);
    }

    // 写入历史文件
    if let Err(e) = modules::wakeup_history::add_history_items(history.clone()) {
        modules::logger::log_error(&format!("写入唤醒历史失败: {}", e));
    }

    let payload = WakeupTaskResultPayload {
        task_id: task.id.clone(),
        last_run_at: chrono::Utc::now().timestamp_millis(),
        records: history,
    };
    let _ = app.emit("wakeup://task-result", payload);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WakeupTaskResultPayload {
    task_id: String,
    last_run_at: i64,
    records: Vec<modules::wakeup_history::WakeupHistoryItem>,
}

async fn refresh_woken_account_quotas(emails: HashSet<String>) {
    // 延迟几秒再刷新，给服务端时间更新配额数据
    sleep(Duration::from_secs(5)).await;
    let accounts = match modules::list_accounts() {
        Ok(list) => list,
        Err(_) => return,
    };
    for account in accounts {
        if !emails.contains(&account.email) {
            continue;
        }
        let mut account = account;
        match modules::fetch_quota_with_retry(&mut account, true).await {
            Ok(quota) => {
                if let Err(e) = modules::update_account_quota(&account.id, quota) {
                    modules::logger::log_warn(&format!(
                        "[WakeupQuotaRefresh] 保存配额失败: {}: {}",
                        account.email, e
                    ));
                } else {
                    modules::logger::log_info(&format!(
                        "[WakeupQuotaRefresh] 已刷新配额: {}",
                        account.email
                    ));
                }
            }
            Err(e) => {
                modules::logger::log_warn(&format!(
                    "[WakeupQuotaRefresh] 获取配额失败: {}: {}",
                    account.email, e
                ));
            }
        }
    }
}
