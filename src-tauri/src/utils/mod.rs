pub mod http;
pub mod protobuf;

/// 日志中对 email 脱敏：取前 6 个字符并追加 "..."
pub fn mask_email(email: &str) -> String {
    let s = email.trim();
    if s.is_empty() {
        return s.to_string();
    }
    let prefix: String = s.chars().take(6).collect();
    format!("{}...", prefix)
}
