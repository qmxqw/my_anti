use std::fs::{self, OpenOptions};
use std::io::Write;

use chrono::{FixedOffset, Utc};

/// 解析账号标识符：有标签则取第一个标签，否则取 email @ 前缀。
pub fn resolve_identifier(tags: Option<&[String]>, email: &str) -> String {
    if let Some(tags) = tags {
        if let Some(first) = tags.first() {
            if !first.is_empty() {
                return first.clone();
            }
        }
    }
    // 无标签：取 email @ 之前的部分
    match email.find('@') {
        Some(pos) => email[..pos].to_string(),
        None => email.to_string(),
    }
}

/// 将秒数格式化为重置时间标签。
/// - 总秒数 < 6小时（21600秒）→ 固定写 "5小时"
/// - 否则按 N天N小时 格式（省略为零的部分），最小写 "1小时"
fn format_duration(total_secs: i64) -> String {
    if total_secs < 6 * 3600 {
        return "5小时".to_string();
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;

    let mut parts: Vec<String> = Vec::new();
    if days > 0 {
        parts.push(format!("{}天", days));
    }
    if hours > 0 {
        parts.push(format!("{}小时", hours));
    }
    if parts.is_empty() {
        parts.push("1小时".to_string());
    }
    parts.join("")
}

/// 向 quota_reset_record.csv 追加一条额度恢复记录。
/// 条件由调用方保证：旧额度 < 100 且 新额度 > 旧额度。
/// reset_secs：最低额度模型的下次恢复时间距现在的秒数，None 时写 "-"。
pub fn append_record(
    platform: &str,
    email: &str,
    old_pct: i32,
    new_pct: i32,
    reset_secs: Option<i64>,
) {
    // 时间：+8 时区，格式 YYYY-MM-DD HH:MM:SS
    let tz = FixedOffset::east_opt(8 * 3600).expect("valid offset");
    let now = Utc::now().with_timezone(&tz);
    let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let reset_str = reset_secs
        .map(|s| format_duration(s))
        .unwrap_or_else(|| "-".to_string());

    let line = format!(
        "{},{},{},{},{},{}\n",
        platform, time_str, email, old_pct, new_pct, reset_str
    );

    match get_csv_path() {
        Ok(path) => {
            // 若文件不存在则写入表头
            let write_header = !path.exists();
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut file) => {
                    if write_header {
                        let _ = file
                            .write_all(b"platform,time,account,old_pct,new_pct,time_to_reset\n");
                    }
                    if let Err(e) = file.write_all(line.as_bytes()) {
                        crate::modules::logger::log_warn(&format!(
                            "[QuotaResetRecord] 写入失败: {}",
                            e
                        ));
                    } else {
                        crate::modules::logger::log_info(&format!(
                            "[QuotaResetRecord] 记录: platform={}, account={}, {}% -> {}%, reset_in={}",
                            platform, email, old_pct, new_pct, reset_str
                        ));
                    }
                }
                Err(e) => {
                    crate::modules::logger::log_warn(&format!(
                        "[QuotaResetRecord] 打开文件失败: {}",
                        e
                    ));
                }
            }
        }
        Err(e) => {
            crate::modules::logger::log_warn(&format!(
                "[QuotaResetRecord] 获取路径失败: {}",
                e
            ));
        }
    }
}

fn get_csv_path() -> Result<std::path::PathBuf, String> {
    let data_dir = crate::modules::account::get_data_dir()?;
    // 确保目录存在
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;
    }
    Ok(data_dir.join("quota_reset_record.csv"))
}
