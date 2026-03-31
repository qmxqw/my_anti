use super::{quota::QuotaData, token::TokenData};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 旧数据缺少 last_used_at 时，默认视为 72 小时前
fn default_last_used_at() -> i64 {
    chrono::Utc::now().timestamp() - 72 * 3600
}

/// 账号数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub token: TokenData,
    /// 绑定的指纹ID（必须绑定，默认为 "original"）
    #[serde(default = "default_fingerprint_id")]
    pub fingerprint_id: Option<String>,
    pub quota: Option<QuotaData>,
    /// Disabled accounts are ignored by the proxy token pool (e.g. revoked refresh_token -> invalid_grant).
    #[serde(default)]
    pub disabled: bool,
    /// Optional human-readable reason for disabling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
    /// Unix timestamp when the account was disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at: Option<i64>,
    /// 受配额保护禁用的模型列表
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub protected_models: HashSet<String>,
    /// 最近一次配额错误信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_error: Option<QuotaErrorInfo>,
    pub created_at: i64,
    /// 账号被消耗计数（切换时 Claude 模型额度 <= 20% 的次数）
    #[serde(default)]
    pub usage_count: u32,
    /// 当前计数周期的配额重置截止时间（Unix 时间戳秒）
    /// 在此时间点之前切换不重复计数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_count_reset_at: Option<i64>,
    /// 最近一次被切换为当前账号的时间戳（Unix 时间戳秒）
    /// 旧数据不存在时反序列化默认为 72 小时前（非 None）
    #[serde(default = "default_last_used_at")]
    pub last_used_at: i64,
}

fn default_fingerprint_id() -> Option<String> {
    Some("original".to_string())
}

impl Account {
    pub fn new(id: String, email: String, token: TokenData) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            email,
            name: None,
            tags: Vec::new(),
            token,
            fingerprint_id: Some("original".to_string()),
            quota: None,
            disabled: false,
            disabled_reason: None,
            disabled_at: None,
            protected_models: HashSet::new(),
            quota_error: None,
            created_at: now,
            usage_count: 0,
            usage_count_reset_at: None,
            last_used_at: now - 72 * 3600,
        }
    }


    pub fn update_quota(&mut self, quota: QuotaData) {
        self.quota = Some(quota);
    }
}

/// 配额错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaErrorInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    pub message: String,
    pub timestamp: i64,
}

/// 账号索引数据（accounts.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountIndex {
    pub version: String,
    pub accounts: Vec<AccountSummary>,
    pub current_account_id: Option<String>,
}

/// 账号摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub created_at: i64,
}

impl AccountIndex {
    pub fn new() -> Self {
        Self {
            version: "2.0".to_string(),
            accounts: Vec::new(),
            current_account_id: None,
        }
    }
}

impl Default for AccountIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 设备指纹（storage.json 中 telemetry 相关字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub machine_id: String,
    pub mac_machine_id: String,
    pub dev_device_id: String,
    pub sqm_id: String,
    #[serde(default)]
    pub service_machine_id: String,
}

/// 指纹历史版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfileVersion {
    pub id: String,
    pub created_at: i64,
    pub label: String,
    pub profile: DeviceProfile,
    #[serde(default)]
    pub is_current: bool,
}
