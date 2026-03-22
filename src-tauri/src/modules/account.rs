use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::models::{
    Account, AccountIndex, AccountSummary, DeviceProfile, DeviceProfileVersion, QuotaData,
    QuotaErrorInfo, TokenData,
};
use crate::modules;

static ACCOUNT_INDEX_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));
static LIST_ACCOUNTS_CACHE: std::sync::LazyLock<Mutex<Option<ListAccountsCacheEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
static LIST_ACCOUNTS_LOAD_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

const LIST_ACCOUNTS_CACHE_TTL_MS: u64 = 800;

// 使用与 AntigravityCockpit 插件相同的数据目录
const DATA_DIR: &str = ".antigravity_cockpit";
const ACCOUNTS_INDEX: &str = "accounts.json";
const ACCOUNTS_DIR: &str = "accounts";
const DELETED_ACCOUNT_FP_BINDINGS: &str = "deleted_account_fingerprint_bindings.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct DeletedAccountFingerprintBindings {
    #[serde(default)]
    by_email: HashMap<String, String>,
}

#[derive(Clone)]
struct ListAccountsCacheEntry {
    cached_at: Instant,
    accounts: Vec<Account>,
}

fn invalidate_list_accounts_cache() {
    if let Ok(mut cache) = LIST_ACCOUNTS_CACHE.lock() {
        *cache = None;
    }
}

fn read_list_accounts_cache() -> Option<Vec<Account>> {
    let Ok(cache) = LIST_ACCOUNTS_CACHE.lock() else {
        return None;
    };

    let Some(entry) = cache.as_ref() else {
        return None;
    };

    if entry.cached_at.elapsed() > Duration::from_millis(LIST_ACCOUNTS_CACHE_TTL_MS) {
        return None;
    }

    Some(entry.accounts.clone())
}

fn write_list_accounts_cache(accounts: &[Account]) {
    if let Ok(mut cache) = LIST_ACCOUNTS_CACHE.lock() {
        *cache = Some(ListAccountsCacheEntry {
            cached_at: Instant::now(),
            accounts: accounts.to_vec(),
        });
    }
}

fn normalize_account_email_key(email: &str) -> String {
    email.trim().to_lowercase()
}

fn get_deleted_account_fp_bindings_path() -> Result<PathBuf, String> {
    Ok(get_data_dir()?.join(DELETED_ACCOUNT_FP_BINDINGS))
}

fn load_deleted_account_fp_bindings() -> Result<DeletedAccountFingerprintBindings, String> {
    let path = get_deleted_account_fp_bindings_path()?;
    if !path.exists() {
        return Ok(DeletedAccountFingerprintBindings::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("读取指纹映射失败: {}", e))?;
    if content.trim().is_empty() {
        return Ok(DeletedAccountFingerprintBindings::default());
    }

    match serde_json::from_str::<DeletedAccountFingerprintBindings>(&content) {
        Ok(bindings) => Ok(bindings),
        Err(e) => {
            modules::logger::log_warn(&format!("指纹映射文件损坏，已重置为空: {}", e));
            Ok(DeletedAccountFingerprintBindings::default())
        }
    }
}

fn save_deleted_account_fp_bindings(
    bindings: &DeletedAccountFingerprintBindings,
) -> Result<(), String> {
    let path = get_deleted_account_fp_bindings_path()?;
    let content =
        serde_json::to_string_pretty(bindings).map_err(|e| format!("序列化指纹映射失败: {}", e))?;
    fs::write(path, content).map_err(|e| format!("保存指纹映射失败: {}", e))
}

fn remember_deleted_account_fingerprint(account: &Account) -> Result<(), String> {
    let key = normalize_account_email_key(&account.email);
    if key.is_empty() {
        return Ok(());
    }

    let Some(fp_id) = account.fingerprint_id.as_ref() else {
        return Ok(());
    };

    if crate::modules::fingerprint::get_fingerprint(fp_id).is_err() {
        modules::logger::log_warn(&format!(
            "删除账号时发现指纹不存在，跳过映射记录: email={}, fingerprint_id={}",
            account.email, fp_id
        ));
        return Ok(());
    }

    let mut bindings = load_deleted_account_fp_bindings()?;
    bindings.by_email.insert(key, fp_id.clone());
    save_deleted_account_fp_bindings(&bindings)?;
    Ok(())
}

fn lookup_deleted_account_fingerprint(email: &str) -> Result<Option<String>, String> {
    let key = normalize_account_email_key(email);
    if key.is_empty() {
        return Ok(None);
    }

    let bindings = load_deleted_account_fp_bindings()?;
    let Some(fp_id) = bindings.by_email.get(&key).cloned() else {
        return Ok(None);
    };

    if crate::modules::fingerprint::get_fingerprint(&fp_id).is_ok() {
        Ok(Some(fp_id))
    } else {
        modules::logger::log_warn(&format!(
            "账号重建时命中过期指纹映射，已忽略: email={}, fingerprint_id={}",
            email, fp_id
        ));
        let _ = clear_deleted_account_fingerprint(email);
        Ok(None)
    }
}

fn clear_deleted_account_fingerprint(email: &str) -> Result<(), String> {
    let key = normalize_account_email_key(email);
    if key.is_empty() {
        return Ok(());
    }

    let mut bindings = load_deleted_account_fp_bindings()?;
    if bindings.by_email.remove(&key).is_some() {
        save_deleted_account_fp_bindings(&bindings)?;
    }
    Ok(())
}

/// 获取数据目录路径
pub fn get_data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    let data_dir = home.join(DATA_DIR);

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    }

    Ok(data_dir)
}

/// 获取账号目录路径
pub fn get_accounts_dir() -> Result<PathBuf, String> {
    let data_dir = get_data_dir()?;
    let accounts_dir = data_dir.join(ACCOUNTS_DIR);

    if !accounts_dir.exists() {
        fs::create_dir_all(&accounts_dir).map_err(|e| format!("创建账号目录失败: {}", e))?;
    }

    Ok(accounts_dir)
}

/// 加载账号索引
pub fn load_account_index() -> Result<AccountIndex, String> {
    let data_dir = get_data_dir()?;
    let index_path = data_dir.join(ACCOUNTS_INDEX);

    if !index_path.exists() {
        return Ok(AccountIndex::new());
    }

    let content =
        fs::read_to_string(&index_path).map_err(|e| format!("读取账号索引失败: {}", e))?;

    if content.trim().is_empty() {
        return Ok(AccountIndex::new());
    }

    serde_json::from_str(&content).map_err(|e| {
        crate::error::file_corrupted_error(
            ACCOUNTS_INDEX,
            &index_path.to_string_lossy(),
            &e.to_string(),
        )
    })
}

/// 保存账号索引
pub fn save_account_index(index: &AccountIndex) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let index_path = data_dir.join(ACCOUNTS_INDEX);
    let temp_path = data_dir.join(format!("{}.tmp", ACCOUNTS_INDEX));

    let content =
        serde_json::to_string_pretty(index).map_err(|e| format!("序列化账号索引失败: {}", e))?;

    fs::write(&temp_path, content).map_err(|e| format!("写入临时索引文件失败: {}", e))?;

    fs::rename(temp_path, index_path).map_err(|e| format!("替换索引文件失败: {}", e))?;
    invalidate_list_accounts_cache();
    Ok(())
}

/// 加载账号数据
pub fn load_account(account_id: &str) -> Result<Account, String> {
    let accounts_dir = get_accounts_dir()?;
    let account_path = accounts_dir.join(format!("{}.json", account_id));

    if !account_path.exists() {
        return Err(format!("账号不存在: {}", account_id));
    }

    let content =
        fs::read_to_string(&account_path).map_err(|e| format!("读取账号数据失败: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("解析账号数据失败: {}", e))
}

/// 保存账号数据
pub fn save_account(account: &Account) -> Result<(), String> {
    let accounts_dir = get_accounts_dir()?;
    let account_path = accounts_dir.join(format!("{}.json", account.id));

    let content =
        serde_json::to_string_pretty(account).map_err(|e| format!("序列化账号数据失败: {}", e))?;

    fs::write(&account_path, content).map_err(|e| format!("保存账号数据失败: {}", e))?;
    invalidate_list_accounts_cache();
    Ok(())
}

/// 记录账号使用消耗（如有需要）
/// 规则：若账号有任意模型配额 < 60%，则计为消耗一次；
///       取这些低配额模型的 reset_time 最大值作为当前周期截止；
///       若当前时间 < usage_count_reset_at（当前周期未到期），跳过计数（去重）。
pub fn record_account_usage_if_needed(account: &mut Account) -> Result<(), String> {
    let Some(quota) = account.quota.as_ref() else {
        return Ok(());
    };

    const USAGE_THRESHOLD: i32 = 60;

    // 收集所有低于阈值的模型
    let low_models: Vec<_> = quota
        .models
        .iter()
        .filter(|m| m.percentage < USAGE_THRESHOLD)
        .collect();

    if low_models.is_empty() {
        return Ok(());
    }

    // 取低配额模型的最晚 reset_time 作为本次周期截止时间
    let new_reset_at: Option<i64> = low_models
        .iter()
        .filter_map(|m| {
            chrono::DateTime::parse_from_rfc3339(&m.reset_time)
                .ok()
                .map(|dt| dt.timestamp())
        })
        .max();

    let now = chrono::Utc::now().timestamp();

    // 去重：如果当前时间仍在上次记录的重置周期内，跳过
    if let Some(reset_at) = account.usage_count_reset_at {
        if now < reset_at {
            modules::logger::log_info(&format!(
                "[UsageCount] 账号 {} 仍在计数周期内（reset_at={}），跳过计数",
                account.email, reset_at
            ));
            return Ok(());
        }
    }

    account.usage_count = account.usage_count.saturating_add(1);
    account.usage_count_reset_at = new_reset_at;

    modules::logger::log_info(&format!(
        "[UsageCount] 账号 {} 使用计数 +1 → {}（reset_at={:?}）",
        account.email, account.usage_count, account.usage_count_reset_at
    ));

    save_account(account)?;
    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for raw in tags {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("标签不能为空".to_string());
        }
        if trimmed.chars().count() > 20 {
            return Err("标签长度不能超过 20 个字符".to_string());
        }
        let normalized = trimmed.to_lowercase();
        if seen.insert(normalized.clone()) {
            result.push(normalized);
        }
    }

    if result.len() > 10 {
        return Err("标签数量不能超过 10 个".to_string());
    }

    Ok(result)
}

/// 更新账号标签
pub fn update_account_tags(account_id: &str, tags: Vec<String>) -> Result<Account, String> {
    let mut account = load_account(account_id)?;
    let normalized = normalize_tags(tags)?;
    account.tags = normalized;
    save_account(&account)?;
    Ok(account)
}

/// 列出所有账号
pub fn list_accounts() -> Result<Vec<Account>, String> {
    if let Some(accounts) = read_list_accounts_cache() {
        return Ok(accounts);
    }

    let _load_guard = LIST_ACCOUNTS_LOAD_LOCK
        .lock()
        .map_err(|e| format!("获取账号列表锁失败: {}", e))?;

    if let Some(accounts) = read_list_accounts_cache() {
        return Ok(accounts);
    }

    modules::logger::log_info("开始列出账号...");
    let index = load_account_index()?;
    let mut accounts = Vec::new();

    for summary in &index.accounts {
        match load_account(&summary.id) {
            Ok(mut account) => {
                let _ = modules::quota_cache::apply_cached_quota(&mut account, "authorized");
                accounts.push(account);
            }
            Err(e) => {
                modules::logger::log_error(&format!("加载账号失败: {}", e));
            }
        }
    }

    write_list_accounts_cache(&accounts);
    Ok(accounts)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

fn is_strict_account_identity_match(existing: &Account, email: &str, token: &TokenData) -> bool {
    if let Some(session_id) = non_empty(token.session_id.as_deref()) {
        if non_empty(existing.token.session_id.as_deref()) == Some(session_id) {
            return true;
        }
    }

    if let Some(refresh_token) = non_empty(Some(token.refresh_token.as_str())) {
        if non_empty(Some(existing.token.refresh_token.as_str())) == Some(refresh_token) {
            return true;
        }
    }

    if existing.email == email {
        if let Some(project_id) = non_empty(token.project_id.as_deref()) {
            if non_empty(existing.token.project_id.as_deref()) == Some(project_id) {
                return true;
            }
        }
    }

    false
}

fn find_matching_account_id(
    index: &AccountIndex,
    email: &str,
    token: &TokenData,
) -> Result<Option<String>, String> {
    for summary in &index.accounts {
        let existing = match load_account(&summary.id) {
            Ok(account) => account,
            Err(err) => {
                modules::logger::log_warn(&format!(
                    "账号匹配时跳过损坏账号文件: id={}, error={}",
                    summary.id, err
                ));
                continue;
            }
        };

        if is_strict_account_identity_match(&existing, email, token) {
            return Ok(Some(existing.id));
        }
    }

    Ok(None)
}

/// 添加账号
pub fn add_account(
    email: String,
    name: Option<String>,
    token: TokenData,
) -> Result<Account, String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;

    if find_matching_account_id(&index, &email, &token)?.is_some() {
        return Err(format!("账号已存在: {}", email));
    }

    let account_id = Uuid::new_v4().to_string();
    let mut account = Account::new(account_id.clone(), email.clone(), token);
    account.name = name.clone();

    let reused_fp_id = match lookup_deleted_account_fingerprint(&email) {
        Ok(fp_id) => fp_id,
        Err(e) => {
            modules::logger::log_warn(&format!(
                "读取已删除账号指纹映射失败，回退为新建指纹: email={}, error={}",
                email, e
            ));
            None
        }
    };

    if let Some(ref fp_id) = reused_fp_id {
        account.fingerprint_id = Some(fp_id.clone());
        modules::logger::log_info(&format!(
            "账号复用已删除映射指纹: email={}, fingerprint_id={}",
            email, fp_id
        ));
    } else {
        let fingerprint = crate::modules::fingerprint::generate_fingerprint(email.clone())?;
        account.fingerprint_id = Some(fingerprint.id.clone());
    }

    save_account(&account)?;

    index.accounts.push(AccountSummary {
        id: account_id.clone(),
        email: email.clone(),
        name: name.clone(),
        created_at: account.created_at,
    });

    if index.current_account_id.is_none() {
        index.current_account_id = Some(account_id);
    }

    save_account_index(&index)?;

    if reused_fp_id.is_some() {
        if let Err(e) = clear_deleted_account_fingerprint(&email) {
            modules::logger::log_warn(&format!(
                "清理已删除账号指纹映射失败: email={}, error={}",
                email, e
            ));
        }
    }

    Ok(account)
}

/// 添加或更新账号
pub fn upsert_account(
    email: String,
    name: Option<String>,
    token: TokenData,
) -> Result<Account, String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;

    let existing_account_id = find_matching_account_id(&index, &email, &token)?;

    if let Some(account_id) = existing_account_id {
        match load_account(&account_id) {
            Ok(mut account) => {
                account.token = token;
                account.name = name.clone();
                if account.disabled {
                    account.disabled = false;
                    account.disabled_reason = None;
                    account.disabled_at = None;
                }
                save_account(&account)?;

                if let Some(idx_summary) = index.accounts.iter_mut().find(|s| s.id == account_id) {
                    idx_summary.name = name;
                    save_account_index(&index)?;
                }

                return Ok(account);
            }
            Err(e) => {
                modules::logger::log_warn(&format!("账号文件缺失，正在重建: {}", e));
                let mut account = Account::new(account_id.clone(), email.clone(), token);
                account.name = name.clone();
                let fingerprint = crate::modules::fingerprint::generate_fingerprint(email.clone())?;
                account.fingerprint_id = Some(fingerprint.id.clone());
                save_account(&account)?;

                if let Some(idx_summary) = index.accounts.iter_mut().find(|s| s.id == account_id) {
                    idx_summary.name = name;
                    save_account_index(&index)?;
                }

                return Ok(account);
            }
        }
    }

    drop(_lock);
    add_account(email, name, token)
}

/// 删除账号
pub fn delete_account(account_id: &str) -> Result<(), String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;

    if let Ok(account) = load_account(account_id) {
        if let Err(e) = remember_deleted_account_fingerprint(&account) {
            modules::logger::log_warn(&format!(
                "记录删除账号指纹映射失败: account_id={}, email={}, error={}",
                account_id, account.email, e
            ));
        }
    }

    let original_len = index.accounts.len();
    index.accounts.retain(|s| s.id != account_id);

    if index.accounts.len() == original_len {
        return Err(format!("找不到账号 ID: {}", account_id));
    }

    if index.current_account_id.as_deref() == Some(account_id) {
        index.current_account_id = index.accounts.first().map(|s| s.id.clone());
    }

    save_account_index(&index)?;

    let accounts_dir = get_accounts_dir()?;
    let account_path = accounts_dir.join(format!("{}.json", account_id));

    if account_path.exists() {
        fs::remove_file(&account_path).map_err(|e| format!("删除账号文件失败: {}", e))?;
    }

    Ok(())
}

/// 批量删除账号
pub fn delete_accounts(account_ids: &[String]) -> Result<(), String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;

    let accounts_dir = get_accounts_dir()?;

    for account_id in account_ids {
        if let Ok(account) = load_account(account_id) {
            if let Err(e) = remember_deleted_account_fingerprint(&account) {
                modules::logger::log_warn(&format!(
                    "批量删除时记录账号指纹映射失败: account_id={}, email={}, error={}",
                    account_id, account.email, e
                ));
            }
        }

        index.accounts.retain(|s| &s.id != account_id);

        if index.current_account_id.as_deref() == Some(account_id) {
            index.current_account_id = None;
        }

        let account_path = accounts_dir.join(format!("{}.json", account_id));
        if account_path.exists() {
            let _ = fs::remove_file(&account_path);
        }
    }

    if index.current_account_id.is_none() {
        index.current_account_id = index.accounts.first().map(|s| s.id.clone());
    }

    save_account_index(&index)
}

/// 重新排序账号列表
pub fn reorder_accounts(account_ids: &[String]) -> Result<(), String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;

    let id_to_summary: std::collections::HashMap<_, _> = index
        .accounts
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();

    let mut new_accounts = Vec::new();
    for id in account_ids {
        if let Some(summary) = id_to_summary.get(id) {
            new_accounts.push(summary.clone());
        }
    }

    for summary in &index.accounts {
        if !account_ids.contains(&summary.id) {
            new_accounts.push(summary.clone());
        }
    }

    index.accounts = new_accounts;

    save_account_index(&index)
}

/// 获取当前账号 ID
pub fn get_current_account_id() -> Result<Option<String>, String> {
    let index = load_account_index()?;
    Ok(index.current_account_id)
}

/// 获取当前激活账号
pub fn get_current_account() -> Result<Option<Account>, String> {
    if let Some(id) = get_current_account_id()? {
        let mut account = load_account(&id)?;
        let _ = modules::quota_cache::apply_cached_quota(&mut account, "authorized");
        Ok(Some(account))
    } else {
        Ok(None)
    }
}

/// 设置当前激活账号 ID
pub fn set_current_account_id(account_id: &str) -> Result<(), String> {
    let _lock = ACCOUNT_INDEX_LOCK
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let mut index = load_account_index()?;
    index.current_account_id = Some(account_id.to_string());
    save_account_index(&index)?;

    // 同时写入 current_account.json 供扩展读取
    if let Ok(account) = load_account(account_id) {
        let _ = save_current_account_file(&account.email);
    }

    Ok(())
}

/// 保存当前账号信息到共享文件（供扩展启动时读取）
fn save_current_account_file(email: &str) -> Result<(), String> {
    use std::fs;
    use std::io::Write;

    let data_dir = get_data_dir()?;
    let file_path = data_dir.join("current_account.json");

    let content = serde_json::json!({
        "email": email,
        "updated_at": chrono::Utc::now().timestamp()
    });

    let json = serde_json::to_string_pretty(&content).map_err(|e| format!("序列化失败: {}", e))?;

    let mut file = fs::File::create(&file_path).map_err(|e| format!("创建文件失败: {}", e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("写入文件失败: {}", e))?;

    modules::logger::log_info("已保存当前账号");
    Ok(())
}



/// 更新账号配额
pub fn update_account_quota(account_id: &str, quota: QuotaData) -> Result<(), String> {
    let mut account = load_account(account_id)?;

    // 容错：如果新获取的 models 为空，但之前有数据，保留原来的 models
    if quota.models.is_empty() {
        if let Some(ref existing_quota) = account.quota {
            if !existing_quota.models.is_empty() {
                modules::logger::log_warn(&format!(
                    "⚠️ 新配额 models 为空，保留原有 {} 个模型数据",
                    existing_quota.models.len()
                ));
                // 只更新非 models 字段（subscription_tier, is_forbidden 等）
                let mut merged_quota = existing_quota.clone();
                merged_quota.subscription_tier = quota.subscription_tier.clone();
                merged_quota.is_forbidden = quota.is_forbidden;
                merged_quota.last_updated = quota.last_updated;
                account.update_quota(merged_quota);
                save_account(&account)?;
                return Ok(());
            }
        }
    }


    account.update_quota(quota);
    save_account(&account)?;
    if let Some(ref quota) = account.quota {
        let _ = modules::quota_cache::write_quota_cache("authorized", &account.email, quota);
    }
    Ok(())
}

/// 设备指纹信息（兼容旧 API）
#[derive(Debug, Serialize)]
pub struct DeviceProfiles {
    pub current_storage: Option<DeviceProfile>,
    pub bound_profile: Option<DeviceProfile>,
    pub history: Vec<DeviceProfileVersion>,
    pub baseline: Option<DeviceProfile>,
}

pub fn get_device_profiles(account_id: &str) -> Result<DeviceProfiles, String> {
    let storage_path = crate::modules::device::get_storage_path()?;
    let current = crate::modules::device::read_profile(&storage_path).ok();
    let account = load_account(account_id)?;

    // 获取账号绑定的指纹
    let bound = account
        .fingerprint_id
        .as_ref()
        .and_then(|fp_id| crate::modules::fingerprint::get_fingerprint(fp_id).ok())
        .map(|fp| fp.profile);

    // 获取原始指纹
    let baseline = crate::modules::fingerprint::load_fingerprint_store()
        .ok()
        .and_then(|store| store.original_baseline)
        .map(|fp| fp.profile);

    Ok(DeviceProfiles {
        current_storage: current,
        bound_profile: bound,
        history: Vec::new(), // 历史功能已移除
        baseline,
    })
}

/// 绑定设备指纹（兼容旧 API，现在会创建新指纹并绑定）
pub fn bind_device_profile(account_id: &str, mode: &str) -> Result<DeviceProfile, String> {
    let name = format!("自动生成 {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));

    let fingerprint = match mode {
        "capture" => crate::modules::fingerprint::capture_fingerprint(name)?,
        "generate" => crate::modules::fingerprint::generate_fingerprint(name)?,
        _ => return Err("mode 只能是 capture 或 generate".to_string()),
    };

    // 绑定到账号
    let mut account = load_account(account_id)?;
    account.fingerprint_id = Some(fingerprint.id.clone());
    save_account(&account)?;

    Ok(fingerprint.profile)
}

/// 使用指定的 profile 绑定（创建新指纹并绑定）
pub fn bind_device_profile_with_profile(
    account_id: &str,
    profile: DeviceProfile,
) -> Result<DeviceProfile, String> {
    use crate::modules::fingerprint;

    let name = format!("自动生成 {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));

    // 创建新指纹
    let mut store = fingerprint::load_fingerprint_store()?;
    let fp = fingerprint::Fingerprint::new(name, profile.clone());
    store.fingerprints.push(fp.clone());
    fingerprint::save_fingerprint_store(&store)?;

    // 绑定到账号
    let mut account = load_account(account_id)?;
    account.fingerprint_id = Some(fp.id.clone());
    save_account(&account)?;

    // 应用到系统
    if let Ok(storage_path) = crate::modules::device::get_storage_path() {
        let _ = crate::modules::device::write_profile(&storage_path, &fp.profile);
    }

    Ok(fp.profile)
}

/// 列出指纹版本（兼容旧 API）
pub fn list_device_versions(account_id: &str) -> Result<DeviceProfiles, String> {
    get_device_profiles(account_id)
}

/// 恢复指纹版本（兼容旧 API）
pub fn restore_device_version(
    _account_id: &str,
    version_id: &str,
) -> Result<DeviceProfile, String> {
    // 直接应用指定的指纹
    let fingerprint = crate::modules::fingerprint::get_fingerprint(version_id)?;
    let _ = crate::modules::fingerprint::apply_fingerprint(version_id);
    Ok(fingerprint.profile)
}

/// 删除历史指纹（兼容旧 API - 已废弃）

pub fn delete_device_version(_account_id: &str, version_id: &str) -> Result<(), String> {
    crate::modules::fingerprint::delete_fingerprint(version_id)
}

#[derive(Serialize)]
pub struct RefreshStats {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuotaAlertPayload {
    pub platform: String,
    pub current_account_id: String,
    pub current_email: String,
    pub threshold: i32,
    pub lowest_percentage: i32,
    pub low_models: Vec<String>,
    pub recommended_account_id: Option<String>,
    pub recommended_email: Option<String>,
    pub triggered_at: i64,
}



fn build_quota_alert_notification_text(payload: &QuotaAlertPayload) -> (String, String) {
    let title = format!(
        "{} 配额预警",
        match payload.platform.as_str() {
            "codex" => "Codex",
            "github_copilot" => "GitHub Copilot",
            "windsurf" => "Windsurf",
            _ => "Antigravity",
        }
    );
    let model_text = if payload.low_models.is_empty() {
        "未知模型".to_string()
    } else {
        payload.low_models.join(", ")
    };
    let mut body = format!(
        "{} 低于 {}%（最低 {}%，模型：{}）",
        payload.current_email, payload.threshold, payload.lowest_percentage, model_text
    );
    if let Some(email) = payload.recommended_email.as_ref() {
        body.push_str(&format!("，建议切换到 {}", email));
    }
    (title, body)
}

fn focus_main_window_and_emit_quota_alert(
    app_handle: &tauri::AppHandle,
    payload: &QuotaAlertPayload,
) {
    use tauri::Manager;

    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    emit_quota_alert(app_handle, payload);
}

pub fn emit_quota_alert(app_handle: &tauri::AppHandle, payload: &QuotaAlertPayload) {
    use tauri::Emitter;
    let _ = app_handle.emit("quota:alert", payload);
}

#[cfg(not(target_os = "macos"))]
pub fn send_quota_alert_native_notification(payload: &QuotaAlertPayload) {
    let Some(app_handle) = crate::get_app_handle() else {
        return;
    };

    use tauri_plugin_notification::NotificationExt;

    let (title, body) = build_quota_alert_notification_text(payload);

    if let Err(e) = app_handle
        .notification()
        .builder()
        .title(&title)
        .body(body)
        .show()
    {
        modules::logger::log_warn(&format!("[QuotaAlert] 原生通知发送失败: {}", e));
    }
}

#[cfg(target_os = "macos")]
pub fn send_quota_alert_native_notification(payload: &QuotaAlertPayload) {
    let Some(app_handle) = crate::get_app_handle().cloned() else {
        return;
    };
    let payload_for_click = payload.clone();
    let (title, body) = build_quota_alert_notification_text(payload);

    std::thread::spawn(move || {
        let mut notification = mac_notification_sys::Notification::new();
        notification
            .title(title.as_str())
            .message(body.as_str())
            .wait_for_click(true)
            .asynchronous(false);

        if let Err(e) = mac_notification_sys::set_application(&app_handle.config().identifier) {
            modules::logger::log_warn(&format!("[QuotaAlert] 设置通知应用标识失败: {}", e));
        }

        match notification.send() {
            Ok(mac_notification_sys::NotificationResponse::Click)
            | Ok(mac_notification_sys::NotificationResponse::ActionButton(_)) => {
                focus_main_window_and_emit_quota_alert(&app_handle, &payload_for_click);
            }
            Ok(_) => {}
            Err(e) => {
                modules::logger::log_warn(&format!("[QuotaAlert] 原生通知发送失败: {}", e));
            }
        }
    });
}

pub fn dispatch_quota_alert(payload: &QuotaAlertPayload) {
    modules::logger::log_warn(&format!(
        "[QuotaAlert] 触发配额预警: platform={}, current_id={}, threshold={}%, lowest={}%",
        payload.platform, payload.current_account_id, payload.threshold, payload.lowest_percentage
    ));

    if let Some(app_handle) = crate::get_app_handle() {
        emit_quota_alert(app_handle, payload);
    }
    send_quota_alert_native_notification(payload);
}



/// 批量刷新所有账号配额
pub async fn refresh_all_quotas_logic() -> Result<RefreshStats, String> {
    use futures::future::join_all;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    const MAX_CONCURRENT: usize = 5;
    let start = std::time::Instant::now();

    modules::logger::log_info(&format!(
        "开始批量刷新所有账号配额 (并发模式, 最大并发: {})",
        MAX_CONCURRENT
    ));
    let accounts = list_accounts()?;

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));

    let tasks: Vec<_> = accounts
        .into_iter()
        .filter(|account| {
            if account.disabled {
                modules::logger::log_info("  - Skipping Disabled account");
                return false;
            }
            if let Some(ref q) = account.quota {
                if q.is_forbidden {
                    modules::logger::log_info("  - Skipping Forbidden account");
                    return false;
                }
            }
            true
        })
        .map(|mut account| {
            let email = account.email.clone();
            let account_id = account.id.clone();
            let permit = semaphore.clone();
            async move {
                let _guard = permit.acquire().await.unwrap();
                match fetch_quota_with_retry(&mut account, false).await {
                    Ok(quota) => {
                        if let Err(e) = update_account_quota(&account_id, quota) {
                            let msg = format!("Account {}: Save quota failed - {}", email, e);
                            Err(msg)
                        } else {
                            Ok(())
                        }
                    }
                    Err(e) => {
                        let msg = format!("Account {}: Fetch quota failed - {}", email, e);
                        Err(msg)
                    }
                }
            }
        })
        .collect();

    let total = tasks.len();
    let results = join_all(tasks).await;

    let mut success = 0;
    let mut failed = 0;
    let mut details = Vec::new();

    for result in results {
        match result {
            Ok(()) => success += 1,
            Err(msg) => {
                failed += 1;
                details.push(msg);
            }
        }
    }

    let elapsed = start.elapsed();
    modules::logger::log_info(&format!(
        "批量刷新完成: {} 成功, {} 失败, 耗时: {}ms",
        success,
        failed,
        elapsed.as_millis()
    ));

    Ok(RefreshStats {
        total,
        success,
        failed,
        details,
    })
}

/// 带重试的配额查询
/// skip_cache: 是否跳过缓存，单个账号刷新应传 true
pub async fn fetch_quota_with_retry(
    account: &mut Account,
    skip_cache: bool,
) -> crate::error::AppResult<QuotaData> {
    use crate::error::AppError;
    use crate::modules::oauth;

    let token = match oauth::ensure_fresh_token(&account.token).await {
        Ok(t) => t,
        Err(e) => {
            if e.contains("invalid_grant") {
                account.disabled = true;
                account.disabled_at = Some(chrono::Utc::now().timestamp());
                account.disabled_reason = Some(format!("invalid_grant: {}", e));
                let _ = save_account(account);
            }
            account.quota_error = Some(QuotaErrorInfo {
                code: None,
                message: format!("OAuth error: {}", e),
                timestamp: chrono::Utc::now().timestamp(),
            });
            let _ = save_account(account);
            return Err(AppError::OAuth(e));
        }
    };

    if token.access_token != account.token.access_token {
        account.token = token.clone();
        let _ = upsert_account(account.email.clone(), account.name.clone(), token.clone());
    }

    let result =
        modules::quota::fetch_quota_for_token(&account.token, &account.email, skip_cache).await;
    match result {
        Ok(payload) => {
            account.quota_error = payload.error.map(|err| QuotaErrorInfo {
                code: err.code,
                message: err.message,
                timestamp: chrono::Utc::now().timestamp(),
            });
            let _ = save_account(account);
            Ok(payload.quota)
        }
        Err(err) => {
            account.quota_error = Some(QuotaErrorInfo {
                code: None,
                message: err.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            });
            let _ = save_account(account);
            Err(err)
        }
    }
}

/// 内部切换账号函数（供 WebSocket 调用）
/// 完整流程：Token刷新 + 关闭程序 + 注入 + 指纹同步 + 重启
pub async fn switch_account_internal(account_id: &str) -> Result<Account, String> {
    modules::logger::log_info("[Switch] 开始切换账号");

    // 路径缺失时不执行关闭/注入，避免破坏当前运行态。
    modules::process::ensure_antigravity_launch_path_configured()?;

    // 1. 加载并验证账号存在
    let mut account = prepare_account_for_injection(account_id).await?;
    modules::logger::log_info("[Switch] 正在切换到账号");

    // 3. 写入设备指纹到 storage.json
    if let Ok(storage_path) = modules::device::get_storage_path() {
        if let Some(ref fp_id) = account.fingerprint_id {
            // 优先使用绑定的指纹
            if let Ok(fingerprint) = modules::fingerprint::get_fingerprint(fp_id) {
                modules::logger::log_info("[Switch] 写入设备指纹");
                let _ = modules::device::write_profile(&storage_path, &fingerprint.profile);
                let _ =
                    modules::db::write_service_machine_id(&fingerprint.profile.service_machine_id);
            }
        }
    }

    // 4. 记录当前账号使用消耗（切换前判断当前账号配额是否低于阈值）
    if let Ok(Some(mut current_account)) = get_current_account() {
        if let Err(e) = record_account_usage_if_needed(&mut current_account) {
            modules::logger::log_warn(&format!("[Switch] 记录使用消耗失败: {}", e));
        }
    }

    // 5. 更新工具内部状态
    set_current_account_id(account_id)?;
    save_account(&account)?;

    // 5. 同步更新默认实例绑定账号，确保默认实例注入目标明确
    if let Err(e) = modules::instance::update_default_settings(
        Some(Some(account_id.to_string())),
        None,
        Some(false),
    ) {
        modules::logger::log_warn(&format!("[Switch] 更新默认实例绑定账号失败: {}", e));
    }

    // 6. 对齐默认实例启动逻辑：按默认实例目录关闭受管进程，再注入默认实例目录
    let default_dir = modules::instance::get_default_user_data_dir()?;
    let default_dir_str = default_dir.to_string_lossy().to_string();
    modules::process::close_antigravity_instances(&[default_dir_str], 20)?;
    let _ = modules::instance::update_default_pid(None);
    modules::instance::inject_account_to_profile(&default_dir, account_id)?;

    // 7. 启动 Antigravity（启动失败不阻断切号，保持原行为）
    modules::logger::log_info("[Switch] 正在启动 Antigravity 默认实例...");
    match modules::process::start_antigravity() {
        Ok(pid) => {
            let _ = modules::instance::update_default_pid(Some(pid));
        }
        Err(e) => {
            modules::logger::log_warn(&format!("[Switch] Antigravity 启动失败: {}", e));
            // 不中断流程，允许用户手动启动
        }
    }

    modules::logger::log_info("[Switch] 账号切换完成");
    Ok(account)
}

/// 准备账号注入：确保 Token 新鲜并落盘
pub async fn prepare_account_for_injection(account_id: &str) -> Result<Account, String> {
    let mut account = load_account(account_id)?;
    let fresh_token = modules::oauth::ensure_fresh_token(&account.token)
        .await
        .map_err(|e| format!("Token 刷新失败: {}", e))?;
    if fresh_token.access_token != account.token.access_token {
        modules::logger::log_info("[Account] Token 已刷新");
        account.token = fresh_token.clone();
        save_account(&account)?;
    }
    Ok(account)
}

/// Ctrl+F1 热键触发的智能切号
/// 1. 刷新当前帐号配额
/// 2. 若当前帐号所有 claude* 模型最低 percentage > 20%，跳过
/// 3. 否则按 findSmartRefreshCandidates 逻辑选取 1 个候选帐号
/// 4. 调用 switch_account_internal 执行切换
pub async fn hotkey_smart_switch() -> Result<String, String> {
    const SKIP_THRESHOLD: i32 = 20;

    modules::logger::log_info("[Hotkey] Ctrl+F1 智能切号开始");

    // 1. 获取当前帐号
    let current_account = match get_current_account()? {
        Some(acc) => acc,
        None => {
            modules::logger::log_warn("[Hotkey] 无当前帐号，跳过");
            return Ok("no_current_account".to_string());
        }
    };
    let current_id = current_account.id.clone();
    let current_email = current_account.email.clone();

    // 2. 刷新当前帐号配额
    {
        let mut acc = load_account(&current_id)?;
        match fetch_quota_with_retry(&mut acc, true).await {
            Ok(quota) => {
                update_account_quota(&current_id, quota)?;
                modules::logger::log_info(&format!(
                    "[Hotkey] 当前帐号 {} 配额已刷新",
                    current_email
                ));
            }
            Err(e) => {
                modules::logger::log_warn(&format!(
                    "[Hotkey] 当前帐号 {} 配额刷新失败: {}，继续执行切号逻辑",
                    current_email, e
                ));
            }
        }
    }

    // 3. 重新加载刷新后的帐号，检查 claude* 模型最低 percentage
    let refreshed_account = load_account(&current_id)?;
    if let Some(ref quota) = refreshed_account.quota {
        let claude_percentages: Vec<i32> = quota
            .models
            .iter()
            .filter(|m| m.name.to_lowercase().starts_with("claude"))
            .map(|m| m.percentage)
            .collect();

        if !claude_percentages.is_empty() {
            let min_percentage = *claude_percentages.iter().min().unwrap();
            if min_percentage > SKIP_THRESHOLD {
                modules::logger::log_info(&format!(
                    "[Hotkey] 当前帐号 {} 配额充足（最低 {}% > {}%），跳过切换",
                    current_email, min_percentage, SKIP_THRESHOLD
                ));
                return Ok(format!("skipped:{}:{}", current_email, min_percentage));
            }
            modules::logger::log_info(&format!(
                "[Hotkey] 当前帐号 {} 配额不足（最低 {}% <= {}%），寻找候选",
                current_email, min_percentage, SKIP_THRESHOLD
            ));
        } else {
            modules::logger::log_info(&format!(
                "[Hotkey] 当前帐号 {} 无 claude 模型配额数据，继续寻找候选",
                current_email
            ));
        }
    } else {
        modules::logger::log_info(&format!(
            "[Hotkey] 当前帐号 {} 无配额数据，继续寻找候选",
            current_email
        ));
    }

    // 4. 获取所有帐号，按 findSmartRefreshCandidates 逻辑筛选
    let all_accounts = list_accounts()?;
    let now = chrono::Utc::now().timestamp();

    let mut candidates: Vec<&Account> = all_accounts
        .iter()
        .filter(|acc| {
            // 非当前帐号
            if acc.id == current_id {
                return false;
            }
            // 未禁用
            if acc.disabled {
                return false;
            }
            // 需要有配额数据
            let quota = match acc.quota.as_ref() {
                Some(q) => q,
                None => return false,
            };
            if quota.models.is_empty() {
                return false;
            }
            // 任意 claude* 模型额度 100% 或 reset_time 已过
            quota.models.iter().any(|m| {
                let name = m.name.to_lowercase();
                if !name.starts_with("claude") {
                    return false;
                }
                if m.percentage == 100 {
                    return true;
                }
                if !m.reset_time.is_empty() {
                    if let Ok(reset) = chrono::DateTime::parse_from_rfc3339(&m.reset_time) {
                        if reset.timestamp() <= now {
                            return true;
                        }
                    }
                }
                false
            })
        })
        .collect();

    // 热键切号：仅按 created_at 排序（方向取决于配置 refresh_sort_oldest_first）
    let sort_oldest_first = crate::modules::config::get_user_config().refresh_sort_oldest_first;
    candidates.sort_by(|a, b| {
        if sort_oldest_first {
            a.created_at.cmp(&b.created_at)
        } else {
            b.created_at.cmp(&a.created_at)
        }
    });

    // 5. 取第 1 个候选帐号
    let candidate = match candidates.first() {
        Some(c) => c,
        None => {
            modules::logger::log_warn("[Hotkey] 无符合条件的候选帐号");
            return Ok("no_candidate".to_string());
        }
    };

    let candidate_id = candidate.id.clone();
    let candidate_email = candidate.email.clone();
    modules::logger::log_info(&format!(
        "[Hotkey] 选中候选帐号: {} (ID: {})",
        candidate_email, candidate_id
    ));

    // 6. 执行切换
    match switch_account_internal(&candidate_id).await {
        Ok(account) => {
            modules::logger::log_info(&format!(
                "[Hotkey] 智能切号完成: {} -> {}",
                current_email, account.email
            ));
            // 广播切换完成通知
            modules::websocket::broadcast_account_switched(&account.id, &account.email);
            Ok(format!("switched:{}:{}", current_email, account.email))
        }
        Err(e) => {
            modules::logger::log_error(&format!(
                "[Hotkey] 智能切号失败: {} -> {}, error={}",
                current_email, candidate_email, e
            ));
            Err(format!("切换失败: {}", e))
        }
    }
}
