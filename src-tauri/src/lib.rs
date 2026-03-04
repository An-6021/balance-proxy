use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt as _;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex, RwLock};
use uuid::Uuid;

const RETRYABLE_STATUS_CODES: [u16; 3] = [401, 402, 429];
const MAX_LOG_LINES: usize = 500;
const TAVILY_LOCAL_MCP_SCRIPT_FILENAME: &str = "tavily-local-proxy-mcp.mjs";
const TAVILY_LOCAL_MCP_SCRIPT: &str = include_str!("../mcp/tavily-local-proxy-mcp.mjs");
const USAGE_CACHE_FILENAME: &str = "usage-cache.json";
const USAGE_CACHE_TTL_SECS: u64 = 60;
const USAGE_429_MAX_RETRIES: u64 = 3;
const EXA_KEY_BUDGET_USD: f64 = 10.0;
const EXA_FREE_REQUESTS_PER_MONTH: u64 = 1_000;
const EXA_USAGE_LEDGER_FILENAME: &str = "exa-usage-ledger.json";
const KEY_HEALTH_FILENAME: &str = "key-health.json";
const DASHBOARD_STATE_FILENAME: &str = "dashboard-state.json";

const REQUEST_HEADER_BLOCKLIST: [&str; 11] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "authorization",
    "host",
    "content-length",
];

const RESPONSE_HEADER_BLOCKLIST: [&str; 9] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "content-length",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ProxyConfig {
    proxy_token: String,
    firecrawl_api_keys: Vec<String>,
    firecrawl_disabled_api_keys: Vec<String>,
    upstream_base_url: String,
    tavily_api_keys: Vec<String>,
    tavily_disabled_api_keys: Vec<String>,
    tavily_upstream_base_url: String,
    request_timeout_ms: u64,
    key_cooldown_seconds: u64,
    auto_start: bool,
    silent_start: bool,
    host: String,
    port: u16,
    tavily_port: u16,
    exa_api_keys: Vec<String>,
    exa_disabled_api_keys: Vec<String>,
    exa_upstream_base_url: String,
    exa_port: u16,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_token: "your-local-token".to_string(),
            firecrawl_api_keys: Vec::new(),
            firecrawl_disabled_api_keys: Vec::new(),
            upstream_base_url: "https://api.firecrawl.dev".to_string(),
            tavily_api_keys: Vec::new(),
            tavily_disabled_api_keys: Vec::new(),
            tavily_upstream_base_url: "https://api.tavily.com".to_string(),
            request_timeout_ms: 60_000,
            key_cooldown_seconds: 60,
            auto_start: true,
            silent_start: false,
            host: "127.0.0.1".to_string(),
            port: 8787,
            tavily_port: 8788,
            exa_api_keys: Vec::new(),
            exa_disabled_api_keys: Vec::new(),
            exa_upstream_base_url: "https://api.exa.ai".to_string(),
            exa_port: 8789,
        }
    }
}

impl ProxyConfig {
    fn firecrawl_enabled(&self) -> bool {
        !self.firecrawl_api_keys.is_empty() && !self.upstream_base_url.is_empty()
    }

    fn tavily_enabled(&self) -> bool {
        !self.tavily_api_keys.is_empty() && !self.tavily_upstream_base_url.is_empty()
    }

    fn firecrawl_partially_configured(&self) -> bool {
        self.firecrawl_api_keys.is_empty() != self.upstream_base_url.is_empty()
    }

    fn tavily_partially_configured(&self) -> bool {
        self.tavily_api_keys.is_empty() != self.tavily_upstream_base_url.is_empty()
    }

    fn exa_enabled(&self) -> bool {
        !self.exa_api_keys.is_empty() && !self.exa_upstream_base_url.is_empty()
    }

    fn exa_partially_configured(&self) -> bool {
        self.exa_api_keys.is_empty() != self.exa_upstream_base_url.is_empty()
    }

    fn normalized(mut self) -> Self {
        self.proxy_token = self.proxy_token.trim().to_string();
        self.upstream_base_url = self
            .upstream_base_url
            .trim()
            .trim_end_matches('/')
            .to_string();
        self.tavily_upstream_base_url = self
            .tavily_upstream_base_url
            .trim()
            .trim_end_matches('/')
            .to_string();
        self.host = self.host.trim().to_string();
        self.firecrawl_api_keys = split_and_dedupe_keys(&self.firecrawl_api_keys);
        self.firecrawl_disabled_api_keys =
            filter_disabled_keys(&self.firecrawl_api_keys, &self.firecrawl_disabled_api_keys);
        self.tavily_api_keys = split_and_dedupe_keys(&self.tavily_api_keys);
        self.tavily_disabled_api_keys =
            filter_disabled_keys(&self.tavily_api_keys, &self.tavily_disabled_api_keys);
        self.exa_api_keys = split_and_dedupe_keys(&self.exa_api_keys);
        self.exa_disabled_api_keys =
            filter_disabled_keys(&self.exa_api_keys, &self.exa_disabled_api_keys);
        self.exa_upstream_base_url = self
            .exa_upstream_base_url
            .trim()
            .trim_end_matches('/')
            .to_string();
        self
    }

    fn validate_common(&self) -> Result<(), String> {
        if self.proxy_token.is_empty() {
            return Err("PROXY_TOKEN is required".to_string());
        }
        if self.request_timeout_ms == 0 {
            return Err("REQUEST_TIMEOUT_MS must be greater than 0".to_string());
        }
        if self.key_cooldown_seconds == 0 {
            return Err("KEY_COOLDOWN_SECONDS must be greater than 0".to_string());
        }
        if self.host.is_empty() {
            return Err("HOST cannot be empty".to_string());
        }
        if self.port == self.tavily_port
            || self.port == self.exa_port
            || self.tavily_port == self.exa_port
        {
            return Err("PORT, TAVILY_PORT, and EXA_PORT must all be different".to_string());
        }
        Ok(())
    }

    fn validate_provider_completeness(&self) -> Result<(), String> {
        if self.firecrawl_partially_configured() {
            return Err(
                "Firecrawl config is incomplete: FIRECRAWL_API_KEYS and UPSTREAM_BASE_URL must both be set"
                    .to_string(),
            );
        }
        if self.tavily_partially_configured() {
            return Err(
                "Tavily config is incomplete: TAVILY_API_KEYS and TAVILY_UPSTREAM_BASE_URL must both be set"
                    .to_string(),
            );
        }
        if self.exa_partially_configured() {
            return Err(
                "Exa config is incomplete: EXA_API_KEYS and EXA_UPSTREAM_BASE_URL must both be set"
                    .to_string(),
            );
        }
        if !self.firecrawl_enabled() && !self.tavily_enabled() && !self.exa_enabled() {
            return Err(
                "At least one provider must be fully configured (Firecrawl, Tavily, or Exa)"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        self.validate_common()?;
        self.validate_provider_completeness()
    }

    fn listen_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn tavily_listen_url(&self) -> String {
        format!("http://{}:{}", self.host, self.tavily_port)
    }

    fn exa_listen_url(&self) -> String {
        format!("http://{}:{}", self.host, self.exa_port)
    }
}

fn split_and_dedupe_keys(raw_keys: &[String]) -> Vec<String> {
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for raw in raw_keys {
        for part in raw.split(|c| c == ',' || c == '\n' || c == '\r') {
            let key = part.trim();
            if key.is_empty() {
                continue;
            }
            if seen.insert(key.to_string()) {
                deduped.push(key.to_string());
            }
        }
    }
    deduped
}

fn filter_disabled_keys(keys: &[String], raw_disabled: &[String]) -> Vec<String> {
    let key_set: HashSet<&str> = keys.iter().map(String::as_str).collect();
    split_and_dedupe_keys(raw_disabled)
        .into_iter()
        .filter(|key| key_set.contains(key.as_str()))
        .collect()
}

fn persist_config_to_path(path: &PathBuf, config: &ProxyConfig) -> Result<(), String> {
    let text = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

fn provider_keys<'a>(config: &'a ProxyConfig, provider: &str) -> Option<&'a Vec<String>> {
    match provider {
        "firecrawl" => Some(&config.firecrawl_api_keys),
        "tavily" => Some(&config.tavily_api_keys),
        "exa" => Some(&config.exa_api_keys),
        _ => None,
    }
}

fn provider_disabled_keys<'a>(config: &'a ProxyConfig, provider: &str) -> Option<&'a Vec<String>> {
    match provider {
        "firecrawl" => Some(&config.firecrawl_disabled_api_keys),
        "tavily" => Some(&config.tavily_disabled_api_keys),
        "exa" => Some(&config.exa_disabled_api_keys),
        _ => None,
    }
}

fn provider_disabled_keys_mut<'a>(
    config: &'a mut ProxyConfig,
    provider: &str,
) -> Option<&'a mut Vec<String>> {
    match provider {
        "firecrawl" => Some(&mut config.firecrawl_disabled_api_keys),
        "tavily" => Some(&mut config.tavily_disabled_api_keys),
        "exa" => Some(&mut config.exa_disabled_api_keys),
        _ => None,
    }
}

fn set_key_disabled_in_config(
    config: &mut ProxyConfig,
    provider: &str,
    key: &str,
    disabled: bool,
) -> Result<bool, String> {
    let key_exists = match provider {
        "firecrawl" => config.firecrawl_api_keys.iter().any(|k| k == key),
        "tavily" => config.tavily_api_keys.iter().any(|k| k == key),
        "exa" => config.exa_api_keys.iter().any(|k| k == key),
        _ => {
            return Err(format!("Unknown provider: {}", provider));
        }
    };
    if !key_exists {
        return Ok(false);
    }

    let Some(disabled_keys) = provider_disabled_keys_mut(config, provider) else {
        return Err(format!("Unknown provider: {}", provider));
    };

    if disabled {
        if disabled_keys.iter().any(|k| k == key) {
            return Ok(false);
        }
        disabled_keys.push(key.to_string());
        return Ok(true);
    }

    let before = disabled_keys.len();
    disabled_keys.retain(|existing| existing != key);
    Ok(disabled_keys.len() != before)
}

fn enabled_keys_for_provider(config: &ProxyConfig, provider: &str) -> Vec<String> {
    let keys = provider_keys(config, provider).cloned().unwrap_or_default();
    let disabled_set: HashSet<&str> = provider_disabled_keys(config, provider)
        .map(|keys| keys.iter().map(String::as_str).collect())
        .unwrap_or_default();
    keys.into_iter()
        .filter(|key| !disabled_set.contains(key.as_str()))
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyStatus {
    index: usize,
    key_preview: String,
    is_cooling_down: bool,
    cooldown_remaining_secs: u64,
    cooldown_reason_status: Option<u16>,
    is_disabled: bool,
    disabled_reason: Option<String>,
    disabled_reason_detail: Option<String>,
    disabled_at_ts: Option<u64>,
    fail_count: u64,
    verification_state: String,
    last_ok_ts: Option<u64>,
    last_error_ts: Option<u64>,
    last_status_code: Option<u16>,
    last_error: Option<String>,
}

fn truncate_key(key: &str) -> String {
    if key.len() <= 14 {
        key.to_string()
    } else {
        format!("{}...{}", &key[..8], &key[key.len() - 5..])
    }
}

fn idle_key_statuses(keys: &[String], disabled_keys: &[String]) -> Vec<KeyStatus> {
    let disabled_set: HashSet<&str> = disabled_keys.iter().map(String::as_str).collect();
    keys.iter()
        .enumerate()
        .map(|(i, k)| KeyStatus {
            index: i,
            key_preview: truncate_key(k),
            is_cooling_down: false,
            cooldown_remaining_secs: 0,
            cooldown_reason_status: None,
            is_disabled: disabled_set.contains(k.as_str()),
            disabled_reason: None,
            disabled_reason_detail: None,
            disabled_at_ts: None,
            fail_count: 0,
            verification_state: "unknown".to_string(),
            last_ok_ts: None,
            last_error_ts: None,
            last_status_code: None,
            last_error: None,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum KeyVerificationState {
    Unknown,
    Ok,
    Invalid,
}

impl Default for KeyVerificationState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct KeyHealthEntry {
    first_seen_ts: u64,
    verification_state: KeyVerificationState,
    last_ok_ts: Option<u64>,
    last_error_ts: Option<u64>,
    last_status_code: Option<u16>,
    last_error: Option<String>,
    usage_fetched_at: Option<u64>,
    usage_used: Option<f64>,
    usage_limit: Option<f64>,
    usage_remaining: Option<f64>,
    usage_unit: Option<String>,
    usage_source: Option<String>,
    usage_429_fail_count: u64,
    disabled_reason: Option<String>,
    disabled_reason_detail: Option<String>,
    disabled_at_ts: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct KeyHealthStore {
    updated_at: u64,
    firecrawl: HashMap<String, KeyHealthEntry>,
    tavily: HashMap<String, KeyHealthEntry>,
    exa: HashMap<String, KeyHealthEntry>,
}

fn key_health_provider_map<'a>(
    store: &'a KeyHealthStore,
    provider: &str,
) -> Option<&'a HashMap<String, KeyHealthEntry>> {
    match provider {
        "firecrawl" => Some(&store.firecrawl),
        "tavily" => Some(&store.tavily),
        "exa" => Some(&store.exa),
        _ => None,
    }
}

fn key_health_provider_map_mut<'a>(
    store: &'a mut KeyHealthStore,
    provider: &str,
) -> Option<&'a mut HashMap<String, KeyHealthEntry>> {
    match provider {
        "firecrawl" => Some(&mut store.firecrawl),
        "tavily" => Some(&mut store.tavily),
        "exa" => Some(&mut store.exa),
        _ => None,
    }
}

fn ensure_key_health_entry<'a>(
    store: &'a mut KeyHealthStore,
    provider: &str,
    key: &str,
) -> Option<&'a mut KeyHealthEntry> {
    let map = key_health_provider_map_mut(store, provider)?;
    let now = now_ts();
    let entry = map.entry(key.to_string()).or_default();
    if entry.first_seen_ts == 0 {
        entry.first_seen_ts = now;
    }
    Some(entry)
}

fn mark_key_health_ok(store: &mut KeyHealthStore, provider: &str, key: &str, status: u16) {
    let Some(entry) = ensure_key_health_entry(store, provider, key) else {
        return;
    };

    entry.verification_state = KeyVerificationState::Ok;
    entry.last_ok_ts = Some(now_ts());
    entry.last_status_code = Some(status);
    entry.last_error_ts = None;
    entry.last_error = None;
    store.updated_at = now_ts();
}

fn mark_key_health_error(
    store: &mut KeyHealthStore,
    provider: &str,
    key: &str,
    status: Option<u16>,
    error: &str,
    mark_invalid_on_401: bool,
) {
    let Some(entry) = ensure_key_health_entry(store, provider, key) else {
        return;
    };

    if mark_invalid_on_401 && status == Some(401) {
        entry.verification_state = KeyVerificationState::Invalid;
    }

    entry.last_error_ts = Some(now_ts());
    entry.last_status_code = status;
    entry.last_error = Some(error.to_string());
    store.updated_at = now_ts();
}

fn key_health_has_usage_metrics(entry: &KeyHealthEntry) -> bool {
    entry.usage_fetched_at.unwrap_or(0) > 0
        && (entry.usage_used.is_some()
            || entry.usage_limit.is_some()
            || entry.usage_remaining.is_some())
}

fn key_health_usage_is_fresh(entry: &KeyHealthEntry, now: u64) -> bool {
    let fetched_at = entry.usage_fetched_at.unwrap_or(0);
    fetched_at > 0 && now.saturating_sub(fetched_at) < USAGE_CACHE_TTL_SECS
}

fn key_health_usage_cooldown_remaining_secs(entry: &KeyHealthEntry, now: u64) -> Option<u64> {
    if entry.last_status_code != Some(429) {
        return None;
    }
    let last_error_ts = entry.last_error_ts?;
    let retry_after = entry
        .last_error
        .as_deref()
        .and_then(parse_retry_after_secs_from_usage_error)
        .unwrap_or(USAGE_CACHE_TTL_SECS);
    let until = last_error_ts.saturating_add(retry_after);
    until.checked_sub(now).filter(|v| *v > 0)
}

fn mark_key_health_usage_snapshot(
    store: &mut KeyHealthStore,
    provider: &str,
    key: &str,
    snapshot: &ProviderUsageSnapshot,
) {
    let Some(entry) = ensure_key_health_entry(store, provider, key) else {
        return;
    };

    entry.usage_fetched_at = Some(snapshot.fetched_at);
    entry.usage_used = snapshot.used;
    entry.usage_limit = snapshot.limit;
    entry.usage_remaining = snapshot.remaining;
    entry.usage_unit = snapshot.unit.clone();
    entry.usage_source = snapshot.source.clone();
    entry.usage_429_fail_count = 0;
    store.updated_at = now_ts();
}

fn disabled_reason_priority(code: &str) -> u8 {
    match code {
        "account_deactivated" => 30,
        "upstream_401" => 20,
        "usage_401" => 10,
        "usage_429" => 5,
        _ => 0,
    }
}

fn mark_key_health_disabled(
    store: &mut KeyHealthStore,
    provider: &str,
    key: &str,
    reason_code: &str,
    reason_detail: Option<String>,
) {
    let Some(entry) = ensure_key_health_entry(store, provider, key) else {
        return;
    };

    entry.verification_state = KeyVerificationState::Invalid;
    let now = now_ts();
    let should_override = match entry.disabled_reason.as_deref() {
        None => true,
        Some(existing) if existing == reason_code => true,
        Some(existing) => {
            disabled_reason_priority(reason_code) > disabled_reason_priority(existing)
        }
    };

    if should_override {
        entry.disabled_reason = Some(reason_code.to_string());
        entry.disabled_reason_detail = reason_detail;
        entry.disabled_at_ts = Some(now);
    } else if entry.disabled_at_ts.is_none() {
        entry.disabled_at_ts = Some(now);
    }

    store.updated_at = now;
}

fn prune_key_health_for_config(store: &mut KeyHealthStore, config: &ProxyConfig) -> bool {
    let mut changed = false;

    let firecrawl_keys: HashSet<&str> = config
        .firecrawl_api_keys
        .iter()
        .map(String::as_str)
        .collect();
    let before = store.firecrawl.len();
    store
        .firecrawl
        .retain(|k, _| firecrawl_keys.contains(k.as_str()));
    changed |= store.firecrawl.len() != before;

    let tavily_keys: HashSet<&str> = config.tavily_api_keys.iter().map(String::as_str).collect();
    let before = store.tavily.len();
    store.tavily.retain(|k, _| tavily_keys.contains(k.as_str()));
    changed |= store.tavily.len() != before;

    let exa_keys: HashSet<&str> = config.exa_api_keys.iter().map(String::as_str).collect();
    let before = store.exa.len();
    store.exa.retain(|k, _| exa_keys.contains(k.as_str()));
    changed |= store.exa.len() != before;

    if changed {
        store.updated_at = now_ts();
    }
    changed
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ExaUsageLedgerEntry {
    usd_used_total: f64,
    requests_month: i32,
    requests_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ExaUsageLedger {
    updated_at: u64,
    keys: HashMap<String, ExaUsageLedgerEntry>,
}

fn prune_exa_usage_ledger_for_config(store: &mut ExaUsageLedger, config: &ProxyConfig) -> bool {
    let key_set: HashSet<&str> = config.exa_api_keys.iter().map(String::as_str).collect();
    let before = store.keys.len();
    store.keys.retain(|k, _| key_set.contains(k.as_str()));
    let changed = store.keys.len() != before;
    if changed {
        store.updated_at = now_ts();
    }
    changed
}

fn derive_status_flags(
    config: &ProxyConfig,
    firecrawl_running: bool,
    tavily_running: bool,
    exa_running: bool,
) -> (bool, bool, bool, bool, bool, bool) {
    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();
    let exa_enabled = config.exa_enabled();

    let enabled_count = firecrawl_enabled as usize + tavily_enabled as usize + exa_enabled as usize;
    let running_enabled_count = (firecrawl_enabled && firecrawl_running) as usize
        + (tavily_enabled && tavily_running) as usize
        + (exa_enabled && exa_running) as usize;
    let any_running = firecrawl_running || tavily_running || exa_running;
    let running = enabled_count > 0 && running_enabled_count == enabled_count;
    let degraded = any_running && !running;

    (
        running,
        any_running,
        degraded,
        firecrawl_enabled,
        tavily_enabled,
        exa_enabled,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DashboardPersistedState {
    version: u32,
    usage_baselines: Option<serde_json::Value>,
    metrics_state: Option<serde_json::Value>,
}

impl Default for DashboardPersistedState {
    fn default() -> Self {
        Self {
            version: 1,
            usage_baselines: None,
            metrics_state: None,
        }
    }
}

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<ProxyConfig>>,
    config_file_path: PathBuf,
    usage_cache_file_path: PathBuf,
    usage_cache: Arc<RwLock<Option<UsageSnapshot>>>,
    dashboard_state_file_path: PathBuf,
    dashboard_state: Arc<RwLock<DashboardPersistedState>>,
    key_health_file_path: PathBuf,
    key_health: Arc<RwLock<KeyHealthStore>>,
    exa_usage_ledger_file_path: PathBuf,
    exa_usage_ledger: Arc<RwLock<ExaUsageLedger>>,
    runtime: Arc<Mutex<ProxyRuntime>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    metrics: Arc<Mutex<RuntimeMetrics>>,
    active_key_managers: Arc<Mutex<ActiveKeyManagers>>,
}

#[derive(Default)]
struct ProxyRuntime {
    firecrawl_handle: Option<ServerHandle>,
    tavily_handle: Option<ServerHandle>,
    exa_handle: Option<ServerHandle>,
}

#[derive(Default)]
struct ActiveKeyManagers {
    firecrawl: Option<Arc<Mutex<RoundRobinKeyManager>>>,
    tavily: Option<Arc<Mutex<RoundRobinKeyManager>>>,
    exa: Option<Arc<Mutex<RoundRobinKeyManager>>>,
}

struct ServerHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: tauri::async_runtime::JoinHandle<()>,
    listen_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyStatus {
    running: bool,
    any_running: bool,
    degraded: bool,
    listen_url: Option<String>,
    tavily_listen_url: Option<String>,
    exa_listen_url: Option<String>,
    firecrawl_enabled: bool,
    tavily_enabled: bool,
    exa_enabled: bool,
    firecrawl_running: bool,
    tavily_running: bool,
    exa_running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderKeyStatusSnapshot {
    configured: bool,
    running: bool,
    keys: Vec<KeyStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyStatusSnapshot {
    firecrawl: ProviderKeyStatusSnapshot,
    tavily: ProviderKeyStatusSnapshot,
    exa: ProviderKeyStatusSnapshot,
}

#[derive(Clone)]
struct ProxyServerState {
    provider: &'static str,
    proxy_token: String,
    upstream_base_url: String,
    config: Arc<RwLock<ProxyConfig>>,
    config_file_path: PathBuf,
    key_health: Arc<RwLock<KeyHealthStore>>,
    key_health_file_path: PathBuf,
    exa_usage_ledger_file_path: PathBuf,
    exa_usage_ledger: Arc<RwLock<ExaUsageLedger>>,
    key_manager: Arc<Mutex<RoundRobinKeyManager>>,
    http_client: Client,
    logs: Arc<Mutex<VecDeque<String>>>,
    metrics: Arc<Mutex<RuntimeMetrics>>,
}

#[derive(Debug, Clone, Default)]
struct ProviderRuntimeMetrics {
    request_count: u64,
    retry_count: u64,
    last_request_ts: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct RuntimeMetrics {
    firecrawl: ProviderRuntimeMetrics,
    tavily: ProviderRuntimeMetrics,
    exa: ProviderRuntimeMetrics,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderRuntimeMetricsSnapshot {
    request_count: u64,
    retry_count: u64,
    last_request_ts: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeMetricsSnapshot {
    firecrawl: ProviderRuntimeMetricsSnapshot,
    tavily: ProviderRuntimeMetricsSnapshot,
    exa: ProviderRuntimeMetricsSnapshot,
}

impl RuntimeMetrics {
    fn snapshot(&self) -> RuntimeMetricsSnapshot {
        RuntimeMetricsSnapshot {
            firecrawl: ProviderRuntimeMetricsSnapshot {
                request_count: self.firecrawl.request_count,
                retry_count: self.firecrawl.retry_count,
                last_request_ts: self.firecrawl.last_request_ts,
            },
            tavily: ProviderRuntimeMetricsSnapshot {
                request_count: self.tavily.request_count,
                retry_count: self.tavily.retry_count,
                last_request_ts: self.tavily.last_request_ts,
            },
            exa: ProviderRuntimeMetricsSnapshot {
                request_count: self.exa.request_count,
                retry_count: self.exa.retry_count,
                last_request_ts: self.exa.last_request_ts,
            },
        }
    }
}

fn provider_runtime_metrics_mut<'a>(
    metrics: &'a mut RuntimeMetrics,
    provider: &str,
) -> Option<&'a mut ProviderRuntimeMetrics> {
    match provider {
        "firecrawl" => Some(&mut metrics.firecrawl),
        "tavily" => Some(&mut metrics.tavily),
        "exa" => Some(&mut metrics.exa),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct TavilyMcpLaunchConfig {
    command: String,
    args: Vec<String>,
}

#[derive(Clone)]
struct SelectedKey {
    index: usize,
    value: String,
}

#[derive(Clone)]
struct ManagedKeyState {
    value: String,
    cooldown_until: Option<Instant>,
    cooldown_reason_status: Option<u16>,
    fail_count: u64,
    is_disabled: bool,
    disabled_reason: Option<String>,
    disabled_reason_detail: Option<String>,
    disabled_at_ts: Option<u64>,
}

enum KeyFailureAction {
    None,
    Cooldown,
    DisabledBy401,
}

struct RoundRobinKeyManager {
    keys: Vec<ManagedKeyState>,
    next_index: usize,
    cooldown_seconds: u64,
}

impl RoundRobinKeyManager {
    fn new(keys: Vec<String>, disabled_keys: Vec<String>, cooldown_seconds: u64) -> Self {
        let disabled_set: HashSet<String> = disabled_keys.into_iter().collect();
        let key_states = keys
            .into_iter()
            .map(|key| ManagedKeyState {
                is_disabled: disabled_set.contains(&key),
                value: key,
                cooldown_until: None,
                cooldown_reason_status: None,
                fail_count: 0,
                disabled_reason: None,
                disabled_reason_detail: None,
                disabled_at_ts: None,
            })
            .collect();
        Self {
            keys: key_states,
            next_index: 0,
            cooldown_seconds,
        }
    }

    fn active_key_count(&self) -> usize {
        self.keys.iter().filter(|k| !k.is_disabled).count()
    }

    fn select_key(&mut self) -> Option<SelectedKey> {
        let now = Instant::now();
        let count = self.keys.len();
        if count == 0 {
            return None;
        }

        if self.active_key_count() == 0 {
            return None;
        }

        let start = self.next_index % count;

        let mut earliest_idx: Option<usize> = None;
        let mut earliest_wait = Duration::MAX;

        for offset in 0..count {
            let idx = (start + offset) % count;
            let key_state = &self.keys[idx];
            if key_state.is_disabled {
                continue;
            }

            let wait = match key_state.cooldown_until {
                Some(deadline) if deadline > now => deadline - now,
                _ => Duration::ZERO,
            };

            if wait == Duration::ZERO {
                self.next_index = (idx + 1) % count;
                return Some(SelectedKey {
                    index: idx,
                    value: self.keys[idx].value.clone(),
                });
            }

            if wait < earliest_wait {
                earliest_wait = wait;
                earliest_idx = Some(idx);
            }
        }

        let Some(earliest_idx) = earliest_idx else {
            return None;
        };
        self.next_index = (earliest_idx + 1) % count;
        Some(SelectedKey {
            index: earliest_idx,
            value: self.keys[earliest_idx].value.clone(),
        })
    }

    fn mark_retryable_failure(&mut self, key_index: usize, status_code: u16) -> KeyFailureAction {
        let Some(key_state) = self.keys.get_mut(key_index) else {
            return KeyFailureAction::None;
        };

        key_state.fail_count += 1;

        if status_code == 401 {
            key_state.is_disabled = true;
            key_state.cooldown_until = None;
            key_state.cooldown_reason_status = None;
            key_state.disabled_reason = Some("upstream_401".to_string());
            key_state.disabled_reason_detail = None;
            key_state.disabled_at_ts = Some(now_ts());
            return KeyFailureAction::DisabledBy401;
        }

        key_state.cooldown_until =
            Some(Instant::now() + Duration::from_secs(self.cooldown_seconds));
        key_state.cooldown_reason_status = Some(status_code);
        KeyFailureAction::Cooldown
    }

    fn disable_key(&mut self, key: &str, reason_code: &str, reason_detail: Option<String>) -> bool {
        for state in &mut self.keys {
            if state.value != key {
                continue;
            }
            if state.is_disabled {
                return false;
            }
            state.is_disabled = true;
            state.cooldown_until = None;
            state.cooldown_reason_status = None;
            state.disabled_reason = Some(reason_code.to_string());
            state.disabled_reason_detail = reason_detail;
            state.disabled_at_ts = Some(now_ts());
            return true;
        }
        false
    }

    fn get_statuses(&self) -> Vec<KeyStatus> {
        let now = Instant::now();
        self.keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let (is_cooling_down, remaining) = match key.cooldown_until {
                    Some(deadline) if deadline > now => (true, (deadline - now).as_secs()),
                    _ => (false, 0),
                };
                KeyStatus {
                    index: i,
                    key_preview: truncate_key(&key.value),
                    is_cooling_down,
                    cooldown_remaining_secs: remaining,
                    cooldown_reason_status: is_cooling_down
                        .then_some(key.cooldown_reason_status)
                        .flatten(),
                    is_disabled: key.is_disabled,
                    disabled_reason: key.disabled_reason.clone(),
                    disabled_reason_detail: key.disabled_reason_detail.clone(),
                    disabled_at_ts: key.disabled_at_ts,
                    fail_count: key.fail_count,
                    verification_state: "unknown".to_string(),
                    last_ok_ts: None,
                    last_error_ts: None,
                    last_status_code: None,
                    last_error: None,
                }
            })
            .collect()
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |v| v.as_secs())
}

fn month_key_from_ts(ts: u64) -> i32 {
    // Based on Howard Hinnant's "civil_from_days" algorithm.
    let days = (ts / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32) * 100 + (month as i32)
}

async fn append_log(logs: &Arc<Mutex<VecDeque<String>>>, level: &str, message: String) {
    let line = format!("{} [{}] {}", now_ts(), level, message);
    println!("{}", line);
    let mut guard = logs.lock().await;
    if guard.len() >= MAX_LOG_LINES {
        guard.pop_front();
    }
    guard.push_back(line);
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join("proxy-config.json"))
}

fn usage_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(USAGE_CACHE_FILENAME))
}

fn load_usage_cache_from_path(path: &PathBuf) -> Option<UsageSnapshot> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn persist_usage_cache_to_path(path: &PathBuf, snapshot: &UsageSnapshot) -> Result<(), String> {
    let text = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize usage cache: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write usage cache: {}", e))?;
    Ok(())
}

fn dashboard_state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(DASHBOARD_STATE_FILENAME))
}

fn load_dashboard_state_from_path(path: &PathBuf) -> DashboardPersistedState {
    let Ok(text) = fs::read_to_string(path) else {
        return DashboardPersistedState::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn persist_dashboard_state_to_path(
    path: &PathBuf,
    snapshot: &DashboardPersistedState,
) -> Result<(), String> {
    let text = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize dashboard state: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write dashboard state: {}", e))?;
    Ok(())
}

fn key_health_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(KEY_HEALTH_FILENAME))
}

fn load_key_health_from_path(path: &PathBuf) -> Option<KeyHealthStore> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn persist_key_health_to_path(path: &PathBuf, snapshot: &KeyHealthStore) -> Result<(), String> {
    let text = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize key health: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write key health: {}", e))?;
    Ok(())
}

fn exa_usage_ledger_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(EXA_USAGE_LEDGER_FILENAME))
}

fn load_exa_usage_ledger_from_path(path: &PathBuf) -> ExaUsageLedger {
    let Ok(text) = fs::read_to_string(path) else {
        return ExaUsageLedger::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn persist_exa_usage_ledger_to_path(path: &PathBuf, snapshot: &ExaUsageLedger) -> Result<(), String> {
    let text = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize Exa usage ledger: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Failed to write Exa usage ledger: {}", e))?;
    Ok(())
}

async fn reset_usage_cache(state: &AppState) {
    *state.usage_cache.write().await = None;
    let _ = fs::remove_file(&state.usage_cache_file_path);
}

fn ensure_key_health_entries(
    map: &mut HashMap<String, KeyHealthEntry>,
    keys: &[String],
    now: u64,
) -> bool {
    let mut changed = false;
    for key in keys {
        let entry = map.entry(key.to_string()).or_default();
        if entry.first_seen_ts == 0 {
            entry.first_seen_ts = now;
            changed = true;
        }
    }
    changed
}

async fn sync_key_health_with_config(state: &AppState) {
    let config = state.config.read().await.clone();
    let now = now_ts();
    let mut snapshot: Option<KeyHealthStore> = None;

    {
        let mut store = state.key_health.write().await;
        let mut changed = prune_key_health_for_config(&mut store, &config);
        changed |= ensure_key_health_entries(&mut store.firecrawl, &config.firecrawl_api_keys, now);
        changed |= ensure_key_health_entries(&mut store.tavily, &config.tavily_api_keys, now);
        changed |= ensure_key_health_entries(&mut store.exa, &config.exa_api_keys, now);

        if changed {
            store.updated_at = now;
            snapshot = Some(store.clone());
        }
    }

    if let Some(snapshot) = snapshot {
        let _ = persist_key_health_to_path(&state.key_health_file_path, &snapshot);
    }
}

async fn sync_exa_usage_ledger_with_config(state: &AppState) {
    let config = state.config.read().await.clone();
    let mut snapshot: Option<ExaUsageLedger> = None;

    {
        let mut store = state.exa_usage_ledger.write().await;
        let changed = prune_exa_usage_ledger_for_config(&mut store, &config);
        if changed {
            snapshot = Some(store.clone());
        }
    }

    if let Some(snapshot) = snapshot {
        let _ = persist_exa_usage_ledger_to_path(&state.exa_usage_ledger_file_path, &snapshot);
    }
}

fn tavily_local_mcp_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(TAVILY_LOCAL_MCP_SCRIPT_FILENAME))
}

const EXA_LOCAL_MCP_SCRIPT_FILENAME: &str = "exa-local-proxy-mcp.mjs";
const EXA_LOCAL_MCP_SCRIPT: &str = include_str!("../mcp/exa-local-proxy-mcp.mjs");

fn exa_local_mcp_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(EXA_LOCAL_MCP_SCRIPT_FILENAME))
}

fn ensure_tavily_local_mcp_launcher(
    app: &tauri::AppHandle,
) -> Result<TavilyMcpLaunchConfig, String> {
    let script_path = tavily_local_mcp_script_path(app)?;
    let should_write = match fs::read_to_string(&script_path) {
        Ok(existing) => existing != TAVILY_LOCAL_MCP_SCRIPT,
        Err(err) if err.kind() == ErrorKind::NotFound => true,
        Err(err) => {
            return Err(format!(
                "Failed to read Tavily MCP script {}: {}",
                script_path.to_string_lossy(),
                err
            ))
        }
    };

    if should_write {
        fs::write(&script_path, TAVILY_LOCAL_MCP_SCRIPT).map_err(|e| {
            format!(
                "Failed to write Tavily MCP script {}: {}",
                script_path.to_string_lossy(),
                e
            )
        })?;
    }

    Ok(TavilyMcpLaunchConfig {
        command: "node".to_string(),
        args: vec![script_path.to_string_lossy().to_string()],
    })
}

#[derive(Debug, Clone)]
struct ExaMcpLaunchConfig {
    command: String,
    args: Vec<String>,
}

fn ensure_exa_local_mcp_launcher(app: &tauri::AppHandle) -> Result<ExaMcpLaunchConfig, String> {
    let script_path = exa_local_mcp_script_path(app)?;
    let should_write = match fs::read_to_string(&script_path) {
        Ok(existing) => existing != EXA_LOCAL_MCP_SCRIPT,
        Err(err) if err.kind() == ErrorKind::NotFound => true,
        Err(err) => {
            return Err(format!(
                "Failed to read Exa MCP script {}: {}",
                script_path.to_string_lossy(),
                err
            ))
        }
    };

    if should_write {
        fs::write(&script_path, EXA_LOCAL_MCP_SCRIPT).map_err(|e| {
            format!(
                "Failed to write Exa MCP script {}: {}",
                script_path.to_string_lossy(),
                e
            )
        })?;
    }

    Ok(ExaMcpLaunchConfig {
        command: "node".to_string(),
        args: vec![script_path.to_string_lossy().to_string()],
    })
}

fn load_or_init_config(app: &tauri::AppHandle) -> Result<ProxyConfig, String> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = ProxyConfig::default().normalized();
        let text = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;
        fs::write(&path, text).map_err(|e| format!("Failed to write default config: {}", e))?;
        return Ok(config);
    }

    let text = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
    let config: ProxyConfig =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse config: {}", e))?;
    Ok(config.normalized())
}

fn build_firecrawl_router(state: ProxyServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1", any(proxy_v1_root))
        .route("/v1/*path", any(proxy_v1_path))
        .route("/v2", any(proxy_v2_root))
        .route("/v2/*path", any(proxy_v2_path))
        .with_state(state)
}

fn build_tavily_router(state: ProxyServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", any(proxy_tavily_root))
        .route("/*path", any(proxy_tavily_path))
        .with_state(state)
}

fn build_exa_router(state: ProxyServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", any(proxy_exa_root))
        .route("/*path", any(proxy_exa_path))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

async fn proxy_v1_root(
    State(state): State<ProxyServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_versioned_target_url(&state.upstream_base_url, "v1", "", uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_v1_path(
    State(state): State<ProxyServerState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_versioned_target_url(&state.upstream_base_url, "v1", &path, uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_v2_root(
    State(state): State<ProxyServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_versioned_target_url(&state.upstream_base_url, "v2", "", uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_v2_path(
    State(state): State<ProxyServerState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_versioned_target_url(&state.upstream_base_url, "v2", &path, uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_tavily_root(
    State(state): State<ProxyServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_raw_target_url(&state.upstream_base_url, "", uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_tavily_path(
    State(state): State<ProxyServerState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_raw_target_url(&state.upstream_base_url, &path, uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_exa_root(
    State(state): State<ProxyServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_raw_target_url(&state.upstream_base_url, "", uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

async fn proxy_exa_path(
    State(state): State<ProxyServerState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target_url = build_raw_target_url(&state.upstream_base_url, &path, uri.query());
    proxy_request_to_target(
        state,
        method,
        uri.path().to_string(),
        headers,
        body,
        target_url,
    )
    .await
}

fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    let Some(auth) = headers.get("authorization") else {
        return false;
    };
    let Ok(auth_value) = auth.to_str() else {
        return false;
    };
    let mut parts = auth_value.splitn(2, ' ');
    let Some(scheme) = parts.next() else {
        return false;
    };
    let Some(token) = parts.next() else {
        return false;
    };
    scheme.eq_ignore_ascii_case("bearer") && token == expected_token
}

fn build_versioned_target_url(
    base_url: &str,
    api_version: &str,
    path: &str,
    query: Option<&str>,
) -> String {
    let mut target = if path.is_empty() {
        format!("{}/{}", base_url, api_version)
    } else {
        format!("{}/{}/{}", base_url, api_version, path)
    };
    if let Some(query) = query {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn build_raw_target_url(base_url: &str, path: &str, query: Option<&str>) -> String {
    let mut target = if path.is_empty() {
        base_url.to_string()
    } else {
        format!("{}/{}", base_url, path)
    };
    if let Some(query) = query {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn json_error(status: StatusCode, detail: &str) -> Response {
    (status, Json(json!({ "detail": detail }))).into_response()
}

fn sanitize_request_headers(
    headers: &HeaderMap,
    selected_key: &str,
    provider: &str,
) -> Result<HeaderMap, String> {
    let mut sanitized = HeaderMap::new();
    for (name, value) in headers {
        let lower = name.as_str().to_ascii_lowercase();
        if REQUEST_HEADER_BLOCKLIST.contains(&lower.as_str()) {
            continue;
        }
        sanitized.insert(name, value.clone());
    }

    let auth_value = HeaderValue::from_str(&format!("Bearer {}", selected_key))
        .map_err(|_| "Invalid selected API key".to_string())?;
    sanitized.insert("authorization", auth_value);
    if provider.eq_ignore_ascii_case("tavily") || provider.eq_ignore_ascii_case("exa") {
        let api_key_value = HeaderValue::from_str(selected_key)
            .map_err(|_| "Invalid selected API key".to_string())?;
        sanitized.insert("x-api-key", api_key_value);
    }
    Ok(sanitized)
}

fn sanitize_response_headers(headers: &HeaderMap) -> HeaderMap {
    let mut sanitized = HeaderMap::new();
    for (name, value) in headers {
        let lower = name.as_str().to_ascii_lowercase();
        if RESPONSE_HEADER_BLOCKLIST.contains(&lower.as_str()) {
            continue;
        }
        sanitized.insert(name, value.clone());
    }
    sanitized
}

fn extract_exa_cost_dollars_total(payload: &[u8]) -> Option<f64> {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) else {
        return None;
    };
    let Some(cost) = value.get("costDollars") else {
        return None;
    };

    match cost {
        serde_json::Value::Number(num) => num.as_f64(),
        serde_json::Value::Object(map) => map.get("total").and_then(json_as_f64),
        _ => None,
    }
}

async fn record_exa_usage_ledger_after_success(
    state: &ProxyServerState,
    api_key: &str,
    cost_usd: Option<f64>,
) {
    let now = now_ts();
    let month = month_key_from_ts(now);
    let mut store = state.exa_usage_ledger.write().await;
    let entry = store.keys.entry(api_key.to_string()).or_default();

    if entry.requests_month != month {
        entry.requests_month = month;
        entry.requests_used = 0;
    }

    let is_free_request = entry.requests_used < EXA_FREE_REQUESTS_PER_MONTH;
    entry.requests_used = entry.requests_used.saturating_add(1);

    if !is_free_request {
        if let Some(cost_usd) = cost_usd.filter(|v| v.is_finite() && *v > 0.0) {
            entry.usd_used_total += cost_usd;
        }
    }

    store.updated_at = now;
    if let Err(err) = persist_exa_usage_ledger_to_path(&state.exa_usage_ledger_file_path, &store) {
        append_log(
            &state.logs,
            "WARN",
            format!("Failed to persist Exa usage ledger: {}", err),
        )
        .await;
    }
}

async fn auto_disable_key_after_401(state: &ProxyServerState, key: &str) -> Result<bool, String> {
    let changed = {
        let mut config = state.config.write().await;
        let updated = set_key_disabled_in_config(&mut config, state.provider, key, true)?;
        if !updated {
            return Ok(false);
        }
        *config = config.clone().normalized();
        persist_config_to_path(&state.config_file_path, &config)?;
        true
    };

    if changed {
        let snapshot = {
            let mut store = state.key_health.write().await;
            mark_key_health_disabled(&mut store, state.provider, key, "upstream_401", None);
            store.clone()
        };
        let _ = persist_key_health_to_path(&state.key_health_file_path, &snapshot);
        append_log(
            &state.logs,
            "WARN",
            format!(
                "key_auto_disabled provider={} key={} reason=upstream_401",
                state.provider,
                truncate_key(key)
            ),
        )
        .await;
    }

    Ok(changed)
}

async fn mark_key_verified_after_non401_response(state: &ProxyServerState, key: &str, status: u16) {
    if status == 401 {
        return;
    }

    let mut should_persist = false;
    let snapshot = {
        let mut store = state.key_health.write().await;
        let Some(entry) = ensure_key_health_entry(&mut store, state.provider, key) else {
            return;
        };

        if !matches!(entry.verification_state, KeyVerificationState::Ok) {
            mark_key_health_ok(&mut store, state.provider, key, status);
            should_persist = true;
        } else {
            entry.last_ok_ts = Some(now_ts());
            entry.last_status_code = Some(status);
            store.updated_at = now_ts();
        }
        store.clone()
    };

    if should_persist {
        let _ = persist_key_health_to_path(&state.key_health_file_path, &snapshot);
    }
}

async fn proxy_request_to_target(
    state: ProxyServerState,
    method: Method,
    request_path: String,
    headers: HeaderMap,
    body: Bytes,
    target_url: String,
) -> Response {
    if !is_authorized(&headers, &state.proxy_token) {
        return json_error(StatusCode::UNAUTHORIZED, "Unauthorized");
    }

    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let started = Instant::now();
    let mut retry_count = 0usize;
    {
        let mut metrics = state.metrics.lock().await;
        if let Some(provider_metrics) = provider_runtime_metrics_mut(&mut metrics, state.provider) {
            provider_metrics.request_count += 1;
            provider_metrics.last_request_ts = Some(now_ts());
        }
    }

    let max_attempts = {
        let manager = state.key_manager.lock().await;
        manager.active_key_count()
    };
    if max_attempts == 0 {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "No active API keys available. Enable at least one key in Configuration.",
        );
    }

    for attempt in 0..max_attempts {
        let selected = {
            let mut manager = state.key_manager.lock().await;
            manager.select_key()
        };
        let Some(selected) = selected else {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "No active API keys available. Enable at least one key in Configuration.",
            );
        };

        let request_headers =
            match sanitize_request_headers(&headers, &selected.value, state.provider) {
                Ok(value) => value,
                Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &err),
            };

        let mut request = state
            .http_client
            .request(method.clone(), &target_url)
            .headers(request_headers);

        if !body.is_empty() {
            request = request.body(body.clone());
        }

        let response = match request.send().await {
            Ok(value) => value,
            Err(err) => {
                append_log(
                    &state.logs,
                    "WARN",
                    format!(
                        "proxy_upstream_error provider={} request_id={} method={} path={} key_index={} attempt={} retries={} err={}",
                        state.provider,
                        request_id,
                        method,
                        request_path,
                        selected.index + 1,
                        attempt + 1,
                        retry_count,
                        err
                    ),
                )
                .await;
                return json_error(StatusCode::BAD_GATEWAY, "Upstream request failed");
            }
        };

        let status = response.status();
        mark_key_verified_after_non401_response(&state, &selected.value, status.as_u16()).await;
        if RETRYABLE_STATUS_CODES.contains(&status.as_u16()) {
            let action = {
                let mut manager = state.key_manager.lock().await;
                manager.mark_retryable_failure(selected.index, status.as_u16())
            };

            if let KeyFailureAction::DisabledBy401 = action {
                if let Err(err) = auto_disable_key_after_401(&state, &selected.value).await {
                    append_log(
                        &state.logs,
                        "ERROR",
                        format!(
                            "failed_to_persist_auto_disabled_key provider={} key={} err={}",
                            state.provider,
                            truncate_key(&selected.value),
                            err
                        ),
                    )
                    .await;
                }
            }

            if attempt < max_attempts - 1 {
                retry_count += 1;
                {
                    let mut metrics = state.metrics.lock().await;
                    if let Some(provider_metrics) =
                        provider_runtime_metrics_mut(&mut metrics, state.provider)
                    {
                        provider_metrics.retry_count += 1;
                        provider_metrics.last_request_ts = Some(now_ts());
                    }
                }
                append_log(
                    &state.logs,
                    "INFO",
                    format!(
                        "proxy_retry provider={} request_id={} method={} path={} status={} key_index={} retries={}",
                        state.provider,
                        request_id,
                        method,
                        request_path,
                        status.as_u16(),
                        selected.index + 1,
                        retry_count
                    ),
                )
                .await;
                continue;
            }
        }

        let response_headers = sanitize_response_headers(response.headers());
        let payload = match response.bytes().await {
            Ok(value) => value,
            Err(_) => return json_error(StatusCode::BAD_GATEWAY, "Failed to read upstream body"),
        };

        if state.provider == "exa" && status.is_success() {
            let cost_usd = extract_exa_cost_dollars_total(payload.as_ref());
            record_exa_usage_ledger_after_success(&state, &selected.value, cost_usd).await;
        }

        append_log(
            &state.logs,
            "INFO",
            format!(
                "proxy_done provider={} request_id={} method={} path={} status={} key_index={} retries={} total_ms={}",
                state.provider,
                request_id,
                method,
                request_path,
                status.as_u16(),
                selected.index + 1,
                retry_count,
                started.elapsed().as_millis()
            ),
        )
        .await;

        let mut builder = Response::builder().status(status);
        for (name, value) in response_headers {
            if let Some(name) = name {
                builder = builder.header(name, value);
            }
        }
        builder = builder.header("X-Proxy-Key-Index", (selected.index + 1).to_string());
        builder = builder.header("X-Proxy-Retry-Count", retry_count.to_string());
        builder = builder.header("X-Proxy-Provider", state.provider);

        return builder.body(Body::from(payload)).unwrap_or_else(|_| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
        });
    }

    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected routing state",
    )
}

#[tauri::command]
async fn load_proxy_config(state: tauri::State<'_, AppState>) -> Result<ProxyConfig, String> {
    Ok(state.config.read().await.clone())
}

async fn has_running_proxy(state: &AppState) -> bool {
    let runtime = state.runtime.lock().await;
    runtime.firecrawl_handle.is_some()
        || runtime.tavily_handle.is_some()
        || runtime.exa_handle.is_some()
}

async fn restart_proxy_if_running(state: &AppState, reason: &str) -> Result<bool, String> {
    if !has_running_proxy(state).await {
        return Ok(false);
    }

    append_log(
        &state.logs,
        "INFO",
        format!("Applying config change, restarting proxy: {}", reason),
    )
    .await;

    stop_proxy_internal(state).await?;
    start_proxy_internal(state).await?;

    append_log(
        &state.logs,
        "INFO",
        "Proxy restarted with updated config".to_string(),
    )
    .await;

    Ok(true)
}

#[tauri::command]
async fn reload_proxy_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ProxyConfig, String> {
    let config = load_or_init_config(&app)?;
    let path = config_path(&app)?;
    let is_running = has_running_proxy(state.inner()).await;

    // Scheme 2: when proxy is running, validate first to avoid downtime.
    if is_running {
        config.validate()?;
    }

    *state.config.write().await = config.clone();
    reset_usage_cache(state.inner()).await;
    sync_key_health_with_config(state.inner()).await;
    sync_exa_usage_ledger_with_config(state.inner()).await;
    append_log(
        &state.logs,
        "INFO",
        format!("Config reloaded: {}", path.to_string_lossy()),
    )
    .await;

    if let Err(err) = restart_proxy_if_running(state.inner(), "reload_proxy_config").await {
        return Err(format!(
            "Config reloaded but failed to restart proxy: {}",
            err
        ));
    }

    Ok(config)
}

#[tauri::command]
async fn save_proxy_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: ProxyConfig,
) -> Result<String, String> {
    let normalized = config.normalized();
    normalized.validate()?;

    let path = config_path(&app)?;
    let text = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, text).map_err(|e| format!("Failed to write config: {}", e))?;

    *state.config.write().await = normalized;
    reset_usage_cache(state.inner()).await;
    sync_key_health_with_config(state.inner()).await;
    sync_exa_usage_ledger_with_config(state.inner()).await;
    append_log(
        &state.logs,
        "INFO",
        format!("Config saved: {}", path.to_string_lossy()),
    )
    .await;

    if let Err(err) = restart_proxy_if_running(state.inner(), "save_proxy_config").await {
        return Err(format!("Config saved but failed to restart proxy: {}", err));
    }

    Ok(path.to_string_lossy().to_string())
}

fn compose_proxy_status(runtime: &ProxyRuntime, config: &ProxyConfig) -> ProxyStatus {
    let firecrawl_listen_url = runtime
        .firecrawl_handle
        .as_ref()
        .map(|h| h.listen_url.clone());
    let tavily_listen_url = runtime.tavily_handle.as_ref().map(|h| h.listen_url.clone());
    let exa_listen_url = runtime.exa_handle.as_ref().map(|h| h.listen_url.clone());
    let firecrawl_running = firecrawl_listen_url.is_some();
    let tavily_running = tavily_listen_url.is_some();
    let exa_running = exa_listen_url.is_some();
    let (running, any_running, degraded, firecrawl_enabled, tavily_enabled, exa_enabled) =
        derive_status_flags(config, firecrawl_running, tavily_running, exa_running);

    ProxyStatus {
        running,
        any_running,
        degraded,
        listen_url: firecrawl_listen_url,
        tavily_listen_url,
        exa_listen_url,
        firecrawl_enabled,
        tavily_enabled,
        exa_enabled,
        firecrawl_running,
        tavily_running,
        exa_running,
    }
}

#[tauri::command]
async fn get_proxy_status(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    let config = state.config.read().await.clone();
    let runtime = state.runtime.lock().await;
    Ok(compose_proxy_status(&runtime, &config))
}

#[tauri::command]
async fn start_proxy(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    start_proxy_internal(&state).await
}

async fn start_proxy_internal(state: &AppState) -> Result<ProxyStatus, String> {
    let config = state.config.read().await.clone();
    config.validate()?;
    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();
    let exa_enabled = config.exa_enabled();

    let (start_firecrawl, start_tavily, start_exa) = {
        let runtime = state.runtime.lock().await;
        (
            firecrawl_enabled && runtime.firecrawl_handle.is_none(),
            tavily_enabled && runtime.tavily_handle.is_none(),
            exa_enabled && runtime.exa_handle.is_none(),
        )
    };

    if !start_firecrawl && !start_tavily && !start_exa {
        let runtime = state.runtime.lock().await;
        return Ok(compose_proxy_status(&runtime, &config));
    }

    let http_client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut new_firecrawl_handle: Option<ServerHandle> = None;
    let mut new_tavily_handle: Option<ServerHandle> = None;
    let mut new_exa_handle: Option<ServerHandle> = None;
    let mut new_firecrawl_manager: Option<Arc<Mutex<RoundRobinKeyManager>>> = None;
    let mut new_tavily_manager: Option<Arc<Mutex<RoundRobinKeyManager>>> = None;
    let mut new_exa_manager: Option<Arc<Mutex<RoundRobinKeyManager>>> = None;

    if start_firecrawl {
        let firecrawl_addr: SocketAddr = format!("{}:{}", config.host, config.port)
            .parse()
            .map_err(|e| format!("Invalid HOST/PORT: {}", e))?;
        let firecrawl_listener = TcpListener::bind(firecrawl_addr)
            .await
            .map_err(|e| format!("Failed to bind {}: {}", config.listen_url(), e))?;
        let firecrawl_local_addr = firecrawl_listener
            .local_addr()
            .map_err(|e| format!("Failed to resolve local addr: {}", e))?;
        let firecrawl_listen_url = format!("http://{}", firecrawl_local_addr);

        let firecrawl_key_manager = Arc::new(Mutex::new(RoundRobinKeyManager::new(
            config.firecrawl_api_keys.clone(),
            config.firecrawl_disabled_api_keys.clone(),
            config.key_cooldown_seconds,
        )));
        new_firecrawl_manager = Some(firecrawl_key_manager.clone());

        let firecrawl_state = ProxyServerState {
            provider: "firecrawl",
            proxy_token: config.proxy_token.clone(),
            upstream_base_url: config.upstream_base_url.clone(),
            config: state.config.clone(),
            config_file_path: state.config_file_path.clone(),
            key_health: state.key_health.clone(),
            key_health_file_path: state.key_health_file_path.clone(),
            exa_usage_ledger_file_path: state.exa_usage_ledger_file_path.clone(),
            exa_usage_ledger: state.exa_usage_ledger.clone(),
            key_manager: firecrawl_key_manager,
            http_client: http_client.clone(),
            logs: state.logs.clone(),
            metrics: state.metrics.clone(),
        };
        let firecrawl_router = build_firecrawl_router(firecrawl_state);
        let (firecrawl_shutdown_tx, firecrawl_shutdown_rx) = oneshot::channel::<()>();

        append_log(
            &state.logs,
            "INFO",
            format!("Firecrawl proxy starting at {}", firecrawl_listen_url),
        )
        .await;

        let logs = state.logs.clone();
        let firecrawl_join_handle = tauri::async_runtime::spawn(async move {
            let server = axum::serve(firecrawl_listener, firecrawl_router).with_graceful_shutdown(
                async move {
                    let _ = firecrawl_shutdown_rx.await;
                },
            );

            if let Err(err) = server.await {
                append_log(&logs, "ERROR", format!("Firecrawl proxy crashed: {}", err)).await;
            }
        });

        new_firecrawl_handle = Some(ServerHandle {
            shutdown_tx: Some(firecrawl_shutdown_tx),
            join_handle: firecrawl_join_handle,
            listen_url: firecrawl_listen_url,
        });
    }

    if start_tavily {
        let tavily_addr: SocketAddr = format!("{}:{}", config.host, config.tavily_port)
            .parse()
            .map_err(|e| format!("Invalid HOST/TAVILY_PORT: {}", e))?;
        let tavily_listener = TcpListener::bind(tavily_addr)
            .await
            .map_err(|e| format!("Failed to bind {}: {}", config.tavily_listen_url(), e))?;
        let tavily_local_addr = tavily_listener
            .local_addr()
            .map_err(|e| format!("Failed to resolve tavily local addr: {}", e))?;
        let tavily_listen_url = format!("http://{}", tavily_local_addr);

        let tavily_key_manager = Arc::new(Mutex::new(RoundRobinKeyManager::new(
            config.tavily_api_keys.clone(),
            config.tavily_disabled_api_keys.clone(),
            config.key_cooldown_seconds,
        )));
        new_tavily_manager = Some(tavily_key_manager.clone());

        let tavily_state = ProxyServerState {
            provider: "tavily",
            proxy_token: config.proxy_token.clone(),
            upstream_base_url: config.tavily_upstream_base_url.clone(),
            config: state.config.clone(),
            config_file_path: state.config_file_path.clone(),
            key_health: state.key_health.clone(),
            key_health_file_path: state.key_health_file_path.clone(),
            exa_usage_ledger_file_path: state.exa_usage_ledger_file_path.clone(),
            exa_usage_ledger: state.exa_usage_ledger.clone(),
            key_manager: tavily_key_manager,
            http_client: http_client.clone(),
            logs: state.logs.clone(),
            metrics: state.metrics.clone(),
        };
        let tavily_router = build_tavily_router(tavily_state);
        let (tavily_shutdown_tx, tavily_shutdown_rx) = oneshot::channel::<()>();

        append_log(
            &state.logs,
            "INFO",
            format!("Tavily proxy starting at {}", tavily_listen_url),
        )
        .await;

        let logs = state.logs.clone();
        let tavily_join_handle = tauri::async_runtime::spawn(async move {
            let server =
                axum::serve(tavily_listener, tavily_router).with_graceful_shutdown(async move {
                    let _ = tavily_shutdown_rx.await;
                });

            if let Err(err) = server.await {
                append_log(&logs, "ERROR", format!("Tavily proxy crashed: {}", err)).await;
            }
        });

        new_tavily_handle = Some(ServerHandle {
            shutdown_tx: Some(tavily_shutdown_tx),
            join_handle: tavily_join_handle,
            listen_url: tavily_listen_url,
        });
    }

    if start_exa {
        let exa_addr: SocketAddr = format!("{}:{}", config.host, config.exa_port)
            .parse()
            .map_err(|e| format!("Invalid HOST/EXA_PORT: {}", e))?;
        let exa_listener = TcpListener::bind(exa_addr)
            .await
            .map_err(|e| format!("Failed to bind {}: {}", config.exa_listen_url(), e))?;
        let exa_local_addr = exa_listener
            .local_addr()
            .map_err(|e| format!("Failed to resolve exa local addr: {}", e))?;
        let exa_listen_url = format!("http://{}", exa_local_addr);

        let exa_key_manager = Arc::new(Mutex::new(RoundRobinKeyManager::new(
            config.exa_api_keys.clone(),
            config.exa_disabled_api_keys.clone(),
            config.key_cooldown_seconds,
        )));
        new_exa_manager = Some(exa_key_manager.clone());

        let exa_state = ProxyServerState {
            provider: "exa",
            proxy_token: config.proxy_token.clone(),
            upstream_base_url: config.exa_upstream_base_url.clone(),
            config: state.config.clone(),
            config_file_path: state.config_file_path.clone(),
            key_health: state.key_health.clone(),
            key_health_file_path: state.key_health_file_path.clone(),
            exa_usage_ledger_file_path: state.exa_usage_ledger_file_path.clone(),
            exa_usage_ledger: state.exa_usage_ledger.clone(),
            key_manager: exa_key_manager,
            http_client: http_client.clone(),
            logs: state.logs.clone(),
            metrics: state.metrics.clone(),
        };
        let exa_router = build_exa_router(exa_state);
        let (exa_shutdown_tx, exa_shutdown_rx) = oneshot::channel::<()>();

        append_log(
            &state.logs,
            "INFO",
            format!("Exa proxy starting at {}", exa_listen_url),
        )
        .await;

        let logs = state.logs.clone();
        let exa_join_handle = tauri::async_runtime::spawn(async move {
            let server = axum::serve(exa_listener, exa_router).with_graceful_shutdown(async move {
                let _ = exa_shutdown_rx.await;
            });

            if let Err(err) = server.await {
                append_log(&logs, "ERROR", format!("Exa proxy crashed: {}", err)).await;
            }
        });

        new_exa_handle = Some(ServerHandle {
            shutdown_tx: Some(exa_shutdown_tx),
            join_handle: exa_join_handle,
            listen_url: exa_listen_url,
        });
    }

    let status = {
        let mut runtime = state.runtime.lock().await;
        if let Some(handle) = new_firecrawl_handle {
            runtime.firecrawl_handle = Some(handle);
        }
        if let Some(handle) = new_tavily_handle {
            runtime.tavily_handle = Some(handle);
        }
        if let Some(handle) = new_exa_handle {
            runtime.exa_handle = Some(handle);
        }
        compose_proxy_status(&runtime, &config)
    };

    {
        let mut active = state.active_key_managers.lock().await;
        if let Some(manager) = new_firecrawl_manager {
            active.firecrawl = Some(manager);
        } else if !firecrawl_enabled {
            active.firecrawl = None;
        }

        if let Some(manager) = new_tavily_manager {
            active.tavily = Some(manager);
        } else if !tavily_enabled {
            active.tavily = None;
        }

        if let Some(manager) = new_exa_manager {
            active.exa = Some(manager);
        } else if !exa_enabled {
            active.exa = None;
        }
    }

    Ok(status)
}

#[tauri::command]
async fn stop_proxy(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    stop_proxy_internal(&state).await
}

async fn stop_proxy_internal(state: &AppState) -> Result<ProxyStatus, String> {
    let config = state.config.read().await.clone();
    let (firecrawl_handle, tavily_handle, exa_handle) = {
        let mut runtime = state.runtime.lock().await;
        (
            runtime.firecrawl_handle.take(),
            runtime.tavily_handle.take(),
            runtime.exa_handle.take(),
        )
    };

    if firecrawl_handle.is_none() && tavily_handle.is_none() && exa_handle.is_none() {
        let runtime = state.runtime.lock().await;
        return Ok(compose_proxy_status(&runtime, &config));
    }

    if let Some(mut handle) = firecrawl_handle {
        if let Some(shutdown_tx) = handle.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let _ = handle.join_handle.await;
    }

    if let Some(mut handle) = tavily_handle {
        if let Some(shutdown_tx) = handle.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let _ = handle.join_handle.await;
    }

    if let Some(mut handle) = exa_handle {
        if let Some(shutdown_tx) = handle.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let _ = handle.join_handle.await;
    }

    // Clear active key manager references
    {
        let mut active = state.active_key_managers.lock().await;
        active.firecrawl = None;
        active.tavily = None;
        active.exa = None;
    }

    append_log(&state.logs, "INFO", "All proxies stopped".to_string()).await;
    let runtime = state.runtime.lock().await;
    Ok(compose_proxy_status(&runtime, &config))
}

#[tauri::command]
async fn get_recent_logs(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let logs = state.logs.lock().await;
    Ok(logs.iter().cloned().collect())
}

async fn build_provider_key_status(
    provider: &str,
    configured: bool,
    running: bool,
    keys: &[String],
    disabled_keys: &[String],
    active_manager: Option<Arc<Mutex<RoundRobinKeyManager>>>,
    key_health: &KeyHealthStore,
) -> ProviderKeyStatusSnapshot {
    let now = now_ts();
    let mut statuses = if let Some(manager) = active_manager {
        manager.lock().await.get_statuses()
    } else {
        idle_key_statuses(keys, disabled_keys)
    };

    if let Some(health_map) = key_health_provider_map(key_health, provider) {
        for status in &mut statuses {
            let Some(full_key) = keys.get(status.index) else {
                continue;
            };
            let Some(entry) = health_map.get(full_key) else {
                continue;
            };

            status.verification_state = match entry.verification_state {
                KeyVerificationState::Unknown => "unknown",
                KeyVerificationState::Ok => "ok",
                KeyVerificationState::Invalid => "invalid",
            }
            .to_string();
            status.last_ok_ts = entry.last_ok_ts;
            status.last_error_ts = entry.last_error_ts;
            status.last_status_code = entry.last_status_code;
            status.last_error = entry.last_error.clone();

            if !status.is_disabled && !status.is_cooling_down {
                if let Some(remaining) = key_health_usage_cooldown_remaining_secs(entry, now) {
                    status.is_cooling_down = true;
                    status.cooldown_remaining_secs = remaining;
                    status.cooldown_reason_status = Some(429);
                }
            }

            if status.is_disabled {
                if entry.disabled_reason.is_some() {
                    status.disabled_reason = entry.disabled_reason.clone();
                }
                if entry.disabled_reason_detail.is_some() {
                    status.disabled_reason_detail = entry.disabled_reason_detail.clone();
                }
                if entry.disabled_at_ts.is_some() {
                    status.disabled_at_ts = entry.disabled_at_ts;
                }
            }
        }
    }

    ProviderKeyStatusSnapshot {
        configured,
        running,
        keys: statuses,
    }
}

async fn build_key_status_snapshot_inner(state: &AppState) -> KeyStatusSnapshot {
    let config = state.config.read().await.clone();
    let key_health = state.key_health.read().await.clone();
    let firecrawl_configured = config.firecrawl_enabled();
    let tavily_configured = config.tavily_enabled();
    let exa_configured = config.exa_enabled();

    let (firecrawl_running, tavily_running, exa_running) = {
        let runtime = state.runtime.lock().await;
        (
            runtime.firecrawl_handle.is_some(),
            runtime.tavily_handle.is_some(),
            runtime.exa_handle.is_some(),
        )
    };

    let (active_firecrawl, active_tavily, active_exa) = {
        let active = state.active_key_managers.lock().await;
        (
            active.firecrawl.clone(),
            active.tavily.clone(),
            active.exa.clone(),
        )
    };

    let firecrawl = build_provider_key_status(
        "firecrawl",
        firecrawl_configured,
        firecrawl_running,
        &config.firecrawl_api_keys,
        &config.firecrawl_disabled_api_keys,
        active_firecrawl,
        &key_health,
    )
    .await;
    let tavily = build_provider_key_status(
        "tavily",
        tavily_configured,
        tavily_running,
        &config.tavily_api_keys,
        &config.tavily_disabled_api_keys,
        active_tavily,
        &key_health,
    )
    .await;
    let exa = build_provider_key_status(
        "exa",
        exa_configured,
        exa_running,
        &config.exa_api_keys,
        &config.exa_disabled_api_keys,
        active_exa,
        &key_health,
    )
    .await;

    KeyStatusSnapshot {
        firecrawl,
        tavily,
        exa,
    }
}

#[tauri::command]
async fn get_key_status(state: tauri::State<'_, AppState>) -> Result<Vec<KeyStatus>, String> {
    let snapshot = build_key_status_snapshot_inner(state.inner()).await;
    Ok(snapshot.firecrawl.keys)
}

#[tauri::command]
async fn get_key_status_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<KeyStatusSnapshot, String> {
    Ok(build_key_status_snapshot_inner(state.inner()).await)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderUsageSnapshot {
    configured: bool,
    has_enabled_key: bool,
    ok: bool,
    fetched_at: u64,
    used: Option<f64>,
    limit: Option<f64>,
    remaining: Option<f64>,
    unit: Option<String>,
    secondary_used: Option<f64>,
    secondary_limit: Option<f64>,
    secondary_remaining: Option<f64>,
    secondary_unit: Option<String>,
    source: Option<String>,
    summary: String,
    error: Option<String>,
}

impl ProviderUsageSnapshot {
    fn not_configured() -> Self {
        Self {
            configured: false,
            has_enabled_key: false,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: None,
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: None,
            summary: "Not configured".to_string(),
            error: None,
        }
    }

    fn pending() -> Self {
        Self {
            configured: true,
            has_enabled_key: true,
            ok: false,
            fetched_at: 0,
            used: None,
            limit: None,
            remaining: None,
            unit: None,
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: None,
            summary: "".to_string(),
            error: None,
        }
    }

    fn no_enabled_key() -> Self {
        Self {
            configured: true,
            has_enabled_key: false,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: None,
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: None,
            summary: "All keys are disabled".to_string(),
            error: Some("No enabled key available for usage query".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageSnapshot {
    fetched_at: u64,
    firecrawl: ProviderUsageSnapshot,
    tavily: ProviderUsageSnapshot,
    exa: ProviderUsageSnapshot,
}

fn json_as_f64(value: &serde_json::Value) -> Option<f64> {
    if let Some(num) = value.as_f64() {
        return Some(num);
    }
    if let Some(text) = value.as_str() {
        return text.trim().parse::<f64>().ok();
    }
    None
}

fn json_at_path<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn pick_first_f64(value: &serde_json::Value, paths: &[&[&str]]) -> Option<f64> {
    paths
        .iter()
        .find_map(|path| json_at_path(value, path).and_then(json_as_f64))
}

fn format_usage_summary(
    used: Option<f64>,
    limit: Option<f64>,
    remaining: Option<f64>,
    unit: &str,
) -> String {
    if let (Some(u), Some(l)) = (used, limit) {
        return format!("{:.2}/{:.2} {}", u, l, unit);
    }
    if let Some(r) = remaining {
        return format!("remaining {:.2} {}", r, unit);
    }
    if let Some(u) = used {
        return format!("used {:.2} {}", u, unit);
    }
    "Usage data unavailable".to_string()
}

fn parse_status_code_from_usage_error(error: &str) -> Option<u16> {
    let rest = error.trim_start().strip_prefix("status ")?;
    let token = rest.split_whitespace().next()?;
    token.parse().ok()
}

fn parse_retry_after_secs_from_usage_error(error: &str) -> Option<u64> {
    let lower = error.to_ascii_lowercase();
    let Some(idx) = lower.find("retry after") else {
        return None;
    };
    let tail = &lower[idx + "retry after".len()..];
    let mut started = false;
    let mut value: u64 = 0;
    for c in tail.chars() {
        if c.is_ascii_digit() {
            started = true;
            value = value
                .saturating_mul(10)
                .saturating_add((c as u8 - b'0') as u64);
            continue;
        }
        if started {
            break;
        }
    }
    started.then_some(value).filter(|v| *v > 0)
}

fn is_tavily_account_deactivated_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("deactivated")
}

async fn persist_provider_usage_snapshot(
    state: &AppState,
    config: &ProxyConfig,
    provider: &str,
    snapshot: ProviderUsageSnapshot,
) {
    let mut cache = state.usage_cache.write().await;
    let mut base = cache.clone().unwrap_or_else(|| UsageSnapshot {
        fetched_at: 0,
        firecrawl: if !config.firecrawl_enabled() {
            ProviderUsageSnapshot::not_configured()
        } else if enabled_keys_for_provider(config, "firecrawl").is_empty() {
            ProviderUsageSnapshot::no_enabled_key()
        } else {
            ProviderUsageSnapshot::pending()
        },
        tavily: if !config.tavily_enabled() {
            ProviderUsageSnapshot::not_configured()
        } else if enabled_keys_for_provider(config, "tavily").is_empty() {
            ProviderUsageSnapshot::no_enabled_key()
        } else {
            ProviderUsageSnapshot::pending()
        },
        exa: if !config.exa_enabled() {
            ProviderUsageSnapshot::not_configured()
        } else if enabled_keys_for_provider(config, "exa").is_empty() {
            ProviderUsageSnapshot::no_enabled_key()
        } else {
            ProviderUsageSnapshot::pending()
        },
    });

    match provider {
        "firecrawl" => base.firecrawl = snapshot,
        "tavily" => base.tavily = snapshot,
        "exa" => base.exa = snapshot,
        _ => return,
    }
    base.fetched_at = now_ts();
    *cache = Some(base.clone());
    let _ = persist_usage_cache_to_path(&state.usage_cache_file_path, &base);
}

fn schedule_provider_usage_retry(
    state: AppState,
    provider: &'static str,
    trigger_fetched_at: u64,
    delay_secs: u64,
) {
    let delay_secs = delay_secs.clamp(1, 600);
    tauri::async_runtime::spawn(async move {
        append_log(
            &state.logs,
            "INFO",
            format!(
                "Usage retry scheduled provider={} in {}s",
                provider, delay_secs
            ),
        )
        .await;
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;

        let now = now_ts();
        let cached = state.usage_cache.read().await.clone();
        if let Some(cached) = cached.as_ref() {
            let current = match provider {
                "firecrawl" => &cached.firecrawl,
                "tavily" => &cached.tavily,
                "exa" => &cached.exa,
                _ => return,
            };
            if current.fetched_at != trigger_fetched_at {
                return;
            }
            if current.ok
                && (current.used.is_some()
                    || current.limit.is_some()
                    || current.remaining.is_some())
            {
                return;
            }
            if current
                .error
                .as_deref()
                .is_some_and(|v| !v.contains("status 429"))
                && now.saturating_sub(current.fetched_at) < USAGE_CACHE_TTL_SECS
            {
                return;
            }
        }

        let config = state.config.read().await.clone();
        let snapshot = match provider {
            "firecrawl" => {
                if !config.firecrawl_enabled() {
                    return;
                }
                let keys = enabled_keys_for_provider(&config, "firecrawl");
                if keys.is_empty() {
                    return;
                }
                fetch_firecrawl_usage_for_keys(&state, &config, &keys, false).await
            }
            "tavily" => {
                if !config.tavily_enabled() {
                    return;
                }
                let keys = enabled_keys_for_provider(&config, "tavily");
                if keys.is_empty() {
                    return;
                }
                fetch_tavily_usage_for_keys(&state, &config, &keys, false).await
            }
            "exa" => {
                if !config.exa_enabled() {
                    return;
                }
                let keys = enabled_keys_for_provider(&config, "exa");
                if keys.is_empty() {
                    return;
                }
                fetch_exa_usage_for_keys(&state, &config, &keys).await
            }
            _ => return,
        };

        persist_provider_usage_snapshot(&state, &config, provider, snapshot.clone()).await;
        let level = if snapshot.ok { "INFO" } else { "WARN" };
        append_log(
            &state.logs,
            level,
            format!(
                "Usage retry finished provider={} ok={}",
                provider, snapshot.ok
            ),
        )
        .await;
    });
}

fn usage_numeric_field_paths_hint(value: &serde_json::Value) -> String {
    fn collect(value: &serde_json::Value, prefix: &str, depth: usize, out: &mut Vec<String>) {
        if out.len() >= 20 || depth == 0 {
            return;
        }
        match value {
            serde_json::Value::Number(_) => {
                if !prefix.is_empty() {
                    out.push(prefix.to_string());
                }
            }
            serde_json::Value::String(s) => {
                if s.trim().parse::<f64>().is_ok() && !prefix.is_empty() {
                    out.push(prefix.to_string());
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, v) in arr.iter().take(2).enumerate() {
                    let next = if prefix.is_empty() {
                        format!("[{}]", i)
                    } else {
                        format!("{}[{}]", prefix, i)
                    };
                    collect(v, &next, depth - 1, out);
                    if out.len() >= 20 {
                        break;
                    }
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let next = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    collect(v, &next, depth - 1, out);
                    if out.len() >= 20 {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    let mut out: Vec<String> = Vec::new();
    collect(value, "", 5, &mut out);
    if out.is_empty() {
        return "none".to_string();
    }
    let mut text = out.join(", ");
    if text.len() > 220 {
        text.truncate(220);
        text.push_str("...");
    }
    text
}

fn tavily_extract_usage_numbers(
    value: &serde_json::Value,
    api_key: &str,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<&'static str>) {
    const CREDIT_USED_FIELDS: &[&str] = &[
        "total_credits_used",
        "credits_used",
        "creditsUsed",
        "monthly_credits_used",
        "monthlyCreditsUsed",
        "credit_used",
        "creditUsed",
    ];
    const CREDIT_LIMIT_FIELDS: &[&str] = &[
        "monthly_credit_limit",
        "monthlyCreditLimit",
        "credit_limit",
        "creditLimit",
    ];
    const CREDIT_REMAIN_FIELDS: &[&str] = &[
        "monthly_credits_remaining",
        "monthlyCreditsRemaining",
        "credits_remaining",
        "creditsRemaining",
        "credit_remaining",
        "creditRemaining",
    ];

    const REQ_USED_FIELDS: &[&str] = &[
        "total_requests",
        "totalRequests",
        "monthly_requests",
        "monthlyRequests",
        "requests",
    ];
    const REQ_LIMIT_FIELDS: &[&str] = &[
        "monthly_request_limit",
        "monthlyRequestLimit",
        "request_limit",
        "requestLimit",
    ];
    const REQ_REMAIN_FIELDS: &[&str] = &[
        "monthly_requests_remaining",
        "monthlyRequestsRemaining",
        "requests_remaining",
        "requestsRemaining",
    ];

    fn pick_first_f64_field(value: &serde_json::Value, fields: &[&str]) -> Option<f64> {
        fields
            .iter()
            .find_map(|key| value.get(*key).and_then(json_as_f64))
    }

    let mut used_credits: Option<f64> = None;
    let mut limit_credits: Option<f64> = None;
    let mut remaining_credits: Option<f64> = None;
    let mut used_requests: Option<f64> = None;
    let mut limit_requests: Option<f64> = None;
    let mut remaining_requests: Option<f64> = None;

    // Tavily official `/usage` response schema:
    // - Account scope: `account.plan_usage` / `account.plan_limit` (+ optional paygo fields)
    // - Key scope: `key.usage` / `key.limit`
    //
    // We prefer account limits since key limits can be `null` (unlimited) even when plan limits exist.
    let plan_used = json_at_path(value, &["account", "plan_usage"]).and_then(json_as_f64);
    let plan_limit = json_at_path(value, &["account", "plan_limit"]).and_then(json_as_f64);
    let paygo_used = json_at_path(value, &["account", "paygo_usage"]).and_then(json_as_f64);
    let paygo_limit = json_at_path(value, &["account", "paygo_limit"]).and_then(json_as_f64);

    let used_total = match (plan_used, paygo_used) {
        (None, None) => None,
        _ => Some(plan_used.unwrap_or(0.0) + paygo_used.unwrap_or(0.0)),
    };
    let limit_total = match (plan_limit, paygo_limit) {
        (None, None) => None,
        _ => Some(plan_limit.unwrap_or(0.0) + paygo_limit.unwrap_or(0.0)),
    };

    used_credits = used_credits.or(used_total);
    limit_credits = limit_credits.or(limit_total);

    if used_credits.is_none() && limit_credits.is_none() && remaining_credits.is_none() {
        let key_used = json_at_path(value, &["key", "usage"]).and_then(json_as_f64);
        let key_limit = json_at_path(value, &["key", "limit"]).and_then(json_as_f64);
        used_credits = used_credits.or(key_used);
        limit_credits = limit_credits.or(key_limit);
    }

    let mut apply_scope = |scope: &serde_json::Value| {
        used_credits = used_credits.or_else(|| pick_first_f64_field(scope, CREDIT_USED_FIELDS));
        limit_credits = limit_credits.or_else(|| pick_first_f64_field(scope, CREDIT_LIMIT_FIELDS));
        remaining_credits =
            remaining_credits.or_else(|| pick_first_f64_field(scope, CREDIT_REMAIN_FIELDS));
        used_requests = used_requests.or_else(|| pick_first_f64_field(scope, REQ_USED_FIELDS));
        limit_requests = limit_requests.or_else(|| pick_first_f64_field(scope, REQ_LIMIT_FIELDS));
        remaining_requests =
            remaining_requests.or_else(|| pick_first_f64_field(scope, REQ_REMAIN_FIELDS));
    };

    if let Some(scope) = value
        .get("usage_by_api_key")
        .and_then(|v| v.get(api_key))
        .filter(|v| v.is_object())
    {
        apply_scope(scope);
    }
    if let Some(scope) = value.get("account_usage").filter(|v| v.is_object()) {
        apply_scope(scope);
    }
    if let Some(scope) = value.get("usage_by_api_key").filter(|v| v.is_object()) {
        apply_scope(scope);
    }
    apply_scope(value);

    fn normalize_token(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    fn matches_any(hay: &str, needles: &[&str]) -> bool {
        let h = normalize_token(hay);
        needles.iter().any(|n| h.contains(&normalize_token(n)))
    }

    fn find_metric_in_scope(
        scope: &serde_json::Value,
        want_used: bool,
        want_limit: bool,
        want_remaining: bool,
        depth: usize,
    ) -> (Option<f64>, Option<f64>, Option<f64>) {
        if depth == 0 {
            return (None, None, None);
        }
        let Some(obj) = scope.as_object() else {
            return (None, None, None);
        };

        let mut used: Option<f64> = None;
        let mut limit: Option<f64> = None;
        let mut remaining: Option<f64> = None;

        if want_used {
            for (k, v) in obj {
                if used.is_some() {
                    break;
                }
                if matches_any(
                    k,
                    &["usage", "used", "creditsused", "planusage", "keyusage"],
                ) {
                    used = json_as_f64(v);
                }
            }
        }
        if want_limit {
            for (k, v) in obj {
                if limit.is_some() {
                    break;
                }
                if matches_any(
                    k,
                    &["limit", "quota", "creditslimit", "planlimit", "keylimit"],
                ) {
                    limit = json_as_f64(v);
                }
            }
        }
        if want_remaining {
            for (k, v) in obj {
                if remaining.is_some() {
                    break;
                }
                if matches_any(k, &["remaining", "left", "available"]) {
                    remaining = json_as_f64(v);
                }
            }
        }

        if used.is_some() || limit.is_some() || remaining.is_some() {
            return (used, limit, remaining);
        }

        // Shallow recursive scan (common schema changes: nesting under `plan`, `credits`, etc.)
        for (_k, v) in obj {
            if let Some(child_obj) = v.as_object() {
                if child_obj.is_empty() {
                    continue;
                }
                let (u, l, r) =
                    find_metric_in_scope(v, want_used, want_limit, want_remaining, depth - 1);
                used = used.or(u);
                limit = limit.or(l);
                remaining = remaining.or(r);
                if used.is_some() || limit.is_some() || remaining.is_some() {
                    break;
                }
            }
        }

        (used, limit, remaining)
    }

    // Fallback: if Tavily changed response fields, try to locate (usage/limit/remaining) within
    // known scopes.
    if used_credits.is_none()
        && limit_credits.is_none()
        && remaining_credits.is_none()
        && used_requests.is_none()
        && limit_requests.is_none()
        && remaining_requests.is_none()
    {
        for scope in [
            value.get("key"),
            value.get("account"),
            value.get("data"),
            Some(value),
        ]
        .into_iter()
        .flatten()
        {
            let (u, l, r) = find_metric_in_scope(scope, true, true, true, 3);
            if u.is_some() || l.is_some() || r.is_some() {
                used_credits = used_credits.or(u);
                limit_credits = limit_credits.or(l);
                remaining_credits = remaining_credits.or(r);
                break;
            }
        }
    }

    let has_credits =
        used_credits.is_some() || limit_credits.is_some() || remaining_credits.is_some();
    let has_requests =
        used_requests.is_some() || limit_requests.is_some() || remaining_requests.is_some();

    if has_credits {
        let mut used = used_credits;
        let mut limit = limit_credits;
        let mut remaining = remaining_credits;

        if remaining.is_none() {
            if let (Some(l), Some(u)) = (limit, used) {
                remaining = Some((l - u).max(0.0));
            }
        }
        if used.is_none() {
            if let (Some(l), Some(r)) = (limit, remaining) {
                used = Some((l - r).max(0.0));
            }
        }
        if limit.is_none() {
            if let (Some(u), Some(r)) = (used, remaining) {
                limit = Some(u + r);
            }
        }

        return (used, limit, remaining, Some("credits"));
    }

    if has_requests {
        let mut used = used_requests;
        let mut limit = limit_requests;
        let mut remaining = remaining_requests;

        if remaining.is_none() {
            if let (Some(l), Some(u)) = (limit, used) {
                remaining = Some((l - u).max(0.0));
            }
        }
        if used.is_none() {
            if let (Some(l), Some(r)) = (limit, remaining) {
                used = Some((l - r).max(0.0));
            }
        }
        if limit.is_none() {
            if let (Some(u), Some(r)) = (used, remaining) {
                limit = Some(u + r);
            }
        }

        return (used, limit, remaining, Some("requests"));
    }

    (None, None, None, None)
}

async fn persist_key_health_after_update<F>(state: &AppState, updater: F)
where
    F: FnOnce(&mut KeyHealthStore),
{
    let snapshot = {
        let mut store = state.key_health.write().await;
        updater(&mut store);
        store.clone()
    };
    let _ = persist_key_health_to_path(&state.key_health_file_path, &snapshot);
}

async fn auto_disable_key_in_state(
    state: &AppState,
    provider: &str,
    key: &str,
    reason_code: &str,
    reason_detail: Option<String>,
) -> Result<bool, String> {
    let changed = {
        let mut config = state.config.write().await;
        let updated = set_key_disabled_in_config(&mut config, provider, key, true)?;
        if updated {
            *config = config.clone().normalized();
            persist_config_to_path(&state.config_file_path, &config)?;
        }
        updated
    };

    // Disable immediately in active key manager (if running) to avoid using invalid keys.
    let manager = {
        let active = state.active_key_managers.lock().await;
        match provider {
            "firecrawl" => active.firecrawl.clone(),
            "tavily" => active.tavily.clone(),
            "exa" => active.exa.clone(),
            _ => None,
        }
    };
    if let Some(manager) = manager {
        let mut guard = manager.lock().await;
        let _ = guard.disable_key(key, reason_code, reason_detail.clone());
    }

    persist_key_health_after_update(state, |store| {
        mark_key_health_disabled(store, provider, key, reason_code, reason_detail);
    })
    .await;

    if changed {
        append_log(
            &state.logs,
            "WARN",
            format!(
                "key_auto_disabled provider={} key={} reason={}",
                provider,
                truncate_key(key),
                reason_code
            ),
        )
        .await;
    }

    Ok(changed)
}

async fn fetch_usage_json(
    client: &Client,
    url: String,
    api_key: &str,
    include_x_api_key: bool,
) -> Result<serde_json::Value, String> {
    let mut request = client
        .get(&url)
        .bearer_auth(api_key)
        .header("accept", "application/json");
    if include_x_api_key {
        request = request.header("x-api-key", api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("read body failed: {}", e))?;

    if !status.is_success() {
        let body = if text.len() > 240 {
            format!("{}...", &text[..240])
        } else {
            text
        };
        return Err(format!("status {} body {}", status.as_u16(), body));
    }

    serde_json::from_str(&text).map_err(|e| format!("invalid json: {}", e))
}

async fn fetch_firecrawl_usage(config: &ProxyConfig, api_key: &str) -> ProviderUsageSnapshot {
    let http_client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .build();
    let Ok(http_client) = http_client else {
        return ProviderUsageSnapshot {
            configured: true,
            has_enabled_key: true,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: Some("credits".to_string()),
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: Some("/v2/team/credit-usage".to_string()),
            summary: "Usage data unavailable".to_string(),
            error: Some("Failed to build usage HTTP client".to_string()),
        };
    };

    let url = format!("{}/v2/team/credit-usage", config.upstream_base_url);
    match fetch_usage_json(&http_client, url, api_key, false).await {
        Ok(value) => {
            let mut used = pick_first_f64(
                &value,
                &[
                    &["usedCredits"],
                    &["used_credits"],
                    &["data", "usedCredits"],
                    &["data", "used_credits"],
                ],
            );
            let mut limit = pick_first_f64(
                &value,
                &[
                    &["totalCredits"],
                    &["total_credits"],
                    &["monthlyCredits"],
                    &["monthly_credits"],
                    &["data", "totalCredits"],
                    &["data", "total_credits"],
                ],
            );
            let plan_credits = pick_first_f64(
                &value,
                &[
                    &["planCredits"],
                    &["plan_credits"],
                    &["data", "planCredits"],
                    &["data", "plan_credits"],
                ],
            );
            let mut remaining = pick_first_f64(
                &value,
                &[
                    &["remainingCredits"],
                    &["remaining_credits"],
                    &["data", "remainingCredits"],
                    &["data", "remaining_credits"],
                ],
            );
            if remaining.is_none() {
                remaining = pick_first_f64(
                    &value,
                    &[
                        &["creditsRemaining"],
                        &["credits_remaining"],
                        &["data", "creditsRemaining"],
                        &["data", "credits_remaining"],
                    ],
                );
            }

            if limit.is_none() {
                if let Some(p) = plan_credits {
                    if p > 0.0 {
                        let plausible = match (remaining, used) {
                            (Some(r), _) if r > p + 1e-6 => false,
                            (_, Some(u)) if u > p + 1e-6 => false,
                            _ => true,
                        };
                        if plausible {
                            limit = Some(p);
                        }
                    }
                }
            }

            if let (Some(l), Some(r)) = (limit, remaining) {
                if l > 0.0 && r > l + 1e-6 {
                    limit = None;
                }
            }

            if limit.is_none() {
                if let (Some(u), Some(r)) = (used, remaining) {
                    limit = Some(u + r);
                }
            }
            if used.is_none() {
                if let (Some(l), Some(r)) = (limit, remaining) {
                    if r <= l + 1e-6 {
                        used = Some((l - r).max(0.0));
                    }
                }
            }
            if remaining.is_none() {
                if let (Some(l), Some(u)) = (limit, used) {
                    if u <= l + 1e-6 {
                        remaining = Some((l - u).max(0.0));
                    }
                }
            }

            if used.is_none() && limit.is_none() && remaining.is_none() {
                return ProviderUsageSnapshot {
                    configured: true,
                    has_enabled_key: true,
                    ok: false,
                    fetched_at: now_ts(),
                    used: None,
                    limit: None,
                    remaining: None,
                    unit: None,
                    secondary_used: None,
                    secondary_limit: None,
                    secondary_remaining: None,
                    secondary_unit: None,
                    source: Some("/v2/team/credit-usage".to_string()),
                    summary: "Usage data unavailable".to_string(),
                    error: Some("No usage fields found in upstream response".to_string()),
                };
            }

            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: true,
                fetched_at: now_ts(),
                used,
                limit,
                remaining,
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/v2/team/credit-usage".to_string()),
                summary: format_usage_summary(used, limit, remaining, "credits"),
                error: None,
            }
        }
        Err(error) => ProviderUsageSnapshot {
            configured: true,
            has_enabled_key: true,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: Some("credits".to_string()),
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: Some("/v2/team/credit-usage".to_string()),
            summary: "Usage data unavailable".to_string(),
            error: Some(error),
        },
    }
}

async fn fetch_firecrawl_usage_for_keys(
    state: &AppState,
    config: &ProxyConfig,
    keys: &[String],
    force_refresh: bool,
) -> ProviderUsageSnapshot {
    if keys.is_empty() {
        return ProviderUsageSnapshot::no_enabled_key();
    }

    let provider_health = {
        let store = state.key_health.read().await;
        store.firecrawl.clone()
    };

    let mut used_sum = 0.0f64;
    let mut limit_sum = 0.0f64;
    let mut remaining_sum = 0.0f64;
    let mut has_used = false;
    let mut has_limit = false;
    let mut has_remaining = false;
    let mut last_error: Option<String> = None;
    let mut total_keys = keys.len();
    let now = now_ts();
    let mut verified_keys: HashSet<&str> = HashSet::new();
    let mut min_cooldown_remaining: Option<u64> = None;
    let mut retry_after_secs: Option<u64> = None;

    for key in keys {
        let Some(entry) = provider_health.get(key) else {
            continue;
        };
        if force_refresh || !key_health_has_usage_metrics(entry) || !key_health_usage_is_fresh(entry, now) {
            continue;
        }
        verified_keys.insert(key.as_str());
        if let Some(v) = entry.usage_used {
            used_sum += v;
            has_used = true;
        }
        if let Some(v) = entry.usage_limit {
            limit_sum += v;
            has_limit = true;
        }
        if let Some(v) = entry.usage_remaining {
            remaining_sum += v;
            has_remaining = true;
        }
    }

    for key in keys {
        if verified_keys.len() >= total_keys {
            break;
        }
        if verified_keys.contains(key.as_str()) {
            continue;
        }

        if let Some(entry) = provider_health.get(key) {
            if let Some(remaining) = key_health_usage_cooldown_remaining_secs(entry, now) {
                min_cooldown_remaining =
                    Some(min_cooldown_remaining.map_or(remaining, |v| v.min(remaining)));
                continue;
            }
        }

        let usage = fetch_firecrawl_usage(config, key).await;
        if usage.ok {
            verified_keys.insert(key.as_str());
            persist_key_health_after_update(state, |store| {
                mark_key_health_ok(store, "firecrawl", key, 200);
                mark_key_health_usage_snapshot(store, "firecrawl", key, &usage);
            })
            .await;

            if let Some(v) = usage.used {
                used_sum += v;
                has_used = true;
            }
            if let Some(v) = usage.limit {
                limit_sum += v;
                has_limit = true;
            }
            if let Some(v) = usage.remaining {
                remaining_sum += v;
                has_remaining = true;
            }
            continue;
        }

        last_error = usage.error.clone();
        let Some(err) = usage.error.as_deref() else {
            continue;
        };

        let status = parse_status_code_from_usage_error(err);
        let usage_429_fail_count = if status == Some(429) {
            let previous = provider_health.get(key);
            let previous_count = previous.map(|v| v.usage_429_fail_count).unwrap_or(0);
            let previous_was_429 = previous
                .as_ref()
                .is_some_and(|v| v.last_status_code == Some(429));
            Some(if previous_was_429 {
                previous_count.saturating_add(1).max(1)
            } else {
                1
            })
        } else {
            None
        };
        persist_key_health_after_update(state, |store| {
            mark_key_health_error(store, "firecrawl", key, status, err, true);
            if let Some(entry) = ensure_key_health_entry(store, "firecrawl", key) {
                entry.usage_429_fail_count = usage_429_fail_count.unwrap_or(0);
            }
        })
        .await;

        if status == Some(401) {
            if let Ok(disabled) = auto_disable_key_in_state(
                state,
                "firecrawl",
                key,
                "usage_401",
                Some(err.to_string()),
            )
            .await
            {
                if disabled {
                    total_keys = total_keys.saturating_sub(1);
                }
            }
            continue;
        }

        if status == Some(429)
            && usage_429_fail_count
                .map(|v| v.saturating_sub(1) >= USAGE_429_MAX_RETRIES)
                .unwrap_or(false)
        {
            if let Ok(disabled) = auto_disable_key_in_state(
                state,
                "firecrawl",
                key,
                "usage_429",
                Some(err.to_string()),
            )
            .await
            {
                if disabled {
                    total_keys = total_keys.saturating_sub(1);
                }
            }
            continue;
        }

        let level = if err.contains("status 429") {
            "WARN"
        } else {
            "ERROR"
        };
        append_log(
            &state.logs,
            level,
            format!(
                "Usage fetch failed (firecrawl {}): {}",
                truncate_key(key),
                err
            ),
        )
        .await;

        if err.contains("status 429") {
            retry_after_secs =
                Some(parse_retry_after_secs_from_usage_error(err).unwrap_or(USAGE_CACHE_TTL_SECS));
            break;
        }
    }

    let used = has_used.then_some(used_sum);
    let limit = has_limit.then_some(limit_sum);
    let remaining = has_remaining.then_some(remaining_sum);
    let mut summary = format_usage_summary(used, limit, remaining, "credits");
    if verified_keys.len() < total_keys {
        summary = format!(
            "{} ({} / {} keys)",
            summary,
            verified_keys.len(),
            total_keys
        );
    }

    let snapshot = ProviderUsageSnapshot {
        configured: true,
        has_enabled_key: true,
        ok: verified_keys.len() == total_keys,
        fetched_at: now_ts(),
        used,
        limit,
        remaining,
        unit: Some("credits".to_string()),
        secondary_used: None,
        secondary_limit: None,
        secondary_remaining: None,
        secondary_unit: None,
        source: Some("/v2/team/credit-usage".to_string()),
        summary,
        error: (verified_keys.is_empty())
            .then_some(last_error.unwrap_or_else(|| "No valid usage payload".to_string())),
    };

    let delay = retry_after_secs.or(min_cooldown_remaining);
    if verified_keys.len() < total_keys {
        if let Some(delay) = delay {
            schedule_provider_usage_retry(state.clone(), "firecrawl", snapshot.fetched_at, delay);
        }
    }

    snapshot
}

async fn fetch_tavily_usage(config: &ProxyConfig, api_key: &str) -> ProviderUsageSnapshot {
    let http_client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .build();
    let Ok(http_client) = http_client else {
        return ProviderUsageSnapshot {
            configured: true,
            has_enabled_key: true,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: Some("credits".to_string()),
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: Some("/usage".to_string()),
            summary: "Usage data unavailable".to_string(),
            error: Some("Failed to build usage HTTP client".to_string()),
        };
    };

    let url = format!("{}/usage", config.tavily_upstream_base_url);
    match fetch_usage_json(&http_client, url, api_key, true).await {
        Ok(value) => {
            let (used, limit, remaining, unit) = tavily_extract_usage_numbers(&value, api_key);
            if used.is_none() && limit.is_none() && remaining.is_none() {
                let hint = usage_numeric_field_paths_hint(&value);
                return ProviderUsageSnapshot {
                    configured: true,
                    has_enabled_key: true,
                    ok: false,
                    fetched_at: now_ts(),
                    used: None,
                    limit: None,
                    remaining: None,
                    unit: None,
                    secondary_used: None,
                    secondary_limit: None,
                    secondary_remaining: None,
                    secondary_unit: None,
                    source: Some("/usage".to_string()),
                    summary: "Usage data unavailable".to_string(),
                    error: Some(format!(
                        "No usage fields found in upstream response; numeric_fields={}",
                        hint
                    )),
                };
            }

            let unit = unit.unwrap_or("credits");
            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: true,
                fetched_at: now_ts(),
                used,
                limit,
                remaining,
                unit: Some(unit.to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: format_usage_summary(used, limit, remaining, unit),
                error: None,
            }
        }
        Err(error) => ProviderUsageSnapshot {
            configured: true,
            has_enabled_key: true,
            ok: false,
            fetched_at: now_ts(),
            used: None,
            limit: None,
            remaining: None,
            unit: Some("credits".to_string()),
            secondary_used: None,
            secondary_limit: None,
            secondary_remaining: None,
            secondary_unit: None,
            source: Some("/usage".to_string()),
            summary: "Usage data unavailable".to_string(),
            error: Some(error),
        },
    }
}

async fn fetch_tavily_usage_for_keys(
    state: &AppState,
    config: &ProxyConfig,
    keys: &[String],
    force_refresh: bool,
) -> ProviderUsageSnapshot {
    if keys.is_empty() {
        return ProviderUsageSnapshot::no_enabled_key();
    }

    let provider_health = {
        let store = state.key_health.read().await;
        store.tavily.clone()
    };

    let mut unit: Option<String> = None;
    let mut used_sum = 0.0f64;
    let mut limit_sum = 0.0f64;
    let mut remaining_sum = 0.0f64;
    let mut has_used = false;
    let mut has_limit = false;
    let mut has_remaining = false;
    let mut last_error: Option<String> = None;
    let mut total_keys = keys.len();
    let now = now_ts();
    let mut verified_keys: HashSet<&str> = HashSet::new();
    let mut min_cooldown_remaining: Option<u64> = None;
    let mut retry_after_secs: Option<u64> = None;

    for key in keys {
        let Some(entry) = provider_health.get(key) else {
            continue;
        };
        if force_refresh || !key_health_has_usage_metrics(entry) || !key_health_usage_is_fresh(entry, now) {
            continue;
        }
        verified_keys.insert(key.as_str());
        if unit.is_none() {
            unit = entry.usage_unit.clone();
        }
        if let Some(v) = entry.usage_used {
            used_sum += v;
            has_used = true;
        }
        if let Some(v) = entry.usage_limit {
            limit_sum += v;
            has_limit = true;
        }
        if let Some(v) = entry.usage_remaining {
            remaining_sum += v;
            has_remaining = true;
        }
    }

    for key in keys {
        if verified_keys.len() >= total_keys {
            break;
        }
        if verified_keys.contains(key.as_str()) {
            continue;
        }

        if let Some(entry) = provider_health.get(key) {
            if let Some(remaining) = key_health_usage_cooldown_remaining_secs(entry, now) {
                min_cooldown_remaining =
                    Some(min_cooldown_remaining.map_or(remaining, |v| v.min(remaining)));
                continue;
            }
        }

        let snapshot = fetch_tavily_usage(config, key).await;
        if snapshot.ok {
            verified_keys.insert(key.as_str());
            if unit.is_none() {
                unit = snapshot.unit.clone();
            }
            persist_key_health_after_update(state, |store| {
                mark_key_health_ok(store, "tavily", key, 200);
                mark_key_health_usage_snapshot(store, "tavily", key, &snapshot);
            })
            .await;

            if let Some(v) = snapshot.used {
                used_sum += v;
                has_used = true;
            }
            if let Some(v) = snapshot.limit {
                limit_sum += v;
                has_limit = true;
            }
            if let Some(v) = snapshot.remaining {
                remaining_sum += v;
                has_remaining = true;
            }
            continue;
        }

        last_error = snapshot.error.clone();
        let Some(err) = snapshot.error.as_deref() else {
            continue;
        };

        let status = parse_status_code_from_usage_error(err);
        let usage_429_fail_count = if status == Some(429) {
            let previous = provider_health.get(key);
            let previous_count = previous.map(|v| v.usage_429_fail_count).unwrap_or(0);
            let previous_was_429 = previous
                .as_ref()
                .is_some_and(|v| v.last_status_code == Some(429));
            Some(if previous_was_429 {
                previous_count.saturating_add(1).max(1)
            } else {
                1
            })
        } else {
            None
        };
        persist_key_health_after_update(state, |store| {
            mark_key_health_error(store, "tavily", key, status, err, true);
            if let Some(entry) = ensure_key_health_entry(store, "tavily", key) {
                entry.usage_429_fail_count = usage_429_fail_count.unwrap_or(0);
            }
        })
        .await;

        if status == Some(401) {
            let reason_code = if is_tavily_account_deactivated_error(err) {
                "account_deactivated"
            } else {
                "usage_401"
            };
            if let Ok(disabled) =
                auto_disable_key_in_state(state, "tavily", key, reason_code, Some(err.to_string()))
                    .await
            {
                if disabled {
                    total_keys = total_keys.saturating_sub(1);
                }
            }
            continue;
        }

        if status == Some(429)
            && usage_429_fail_count
                .map(|v| v.saturating_sub(1) >= USAGE_429_MAX_RETRIES)
                .unwrap_or(false)
        {
            if let Ok(disabled) =
                auto_disable_key_in_state(state, "tavily", key, "usage_429", Some(err.to_string()))
                    .await
            {
                if disabled {
                    total_keys = total_keys.saturating_sub(1);
                }
            }
            continue;
        }

        let level = if err.contains("status 429") {
            "WARN"
        } else {
            "ERROR"
        };
        append_log(
            &state.logs,
            level,
            format!("Usage fetch failed (tavily {}): {}", truncate_key(key), err),
        )
        .await;

        if err.contains("status 429") {
            retry_after_secs =
                Some(parse_retry_after_secs_from_usage_error(err).unwrap_or(USAGE_CACHE_TTL_SECS));
            break;
        }
    }

    let used = has_used.then_some(used_sum);
    let limit = has_limit.then_some(limit_sum);
    let remaining = has_remaining.then_some(remaining_sum);
    let unit = unit.unwrap_or_else(|| "credits".to_string());
    let mut summary = format_usage_summary(used, limit, remaining, &unit);
    if verified_keys.len() < total_keys {
        summary = format!(
            "{} ({} / {} keys)",
            summary,
            verified_keys.len(),
            total_keys
        );
    }

    let snapshot = ProviderUsageSnapshot {
        configured: true,
        has_enabled_key: true,
        ok: verified_keys.len() == total_keys,
        fetched_at: now_ts(),
        used,
        limit,
        remaining,
        unit: Some(unit),
        secondary_used: None,
        secondary_limit: None,
        secondary_remaining: None,
        secondary_unit: None,
        source: Some("/usage".to_string()),
        summary,
        error: (verified_keys.is_empty())
            .then_some(last_error.unwrap_or_else(|| "No valid usage payload".to_string())),
    };

    let delay = retry_after_secs.or(min_cooldown_remaining);
    if verified_keys.len() < total_keys {
        if let Some(delay) = delay {
            schedule_provider_usage_retry(state.clone(), "tavily", snapshot.fetched_at, delay);
        }
    }

    snapshot
}

async fn fetch_exa_usage_for_keys(
    state: &AppState,
    _config: &ProxyConfig,
    keys: &[String],
) -> ProviderUsageSnapshot {
    if keys.is_empty() {
        return ProviderUsageSnapshot::no_enabled_key();
    }

    let now = now_ts();
    let month = month_key_from_ts(now);
    let mut requests_used_sum = 0u64;
    let mut usd_used_sum = 0.0f64;
    let mut should_persist = false;

    {
        let mut store = state.exa_usage_ledger.write().await;
        for key in keys {
            let entry = store.keys.entry(key.to_string()).or_default();
            if entry.requests_month != month {
                entry.requests_month = month;
                entry.requests_used = 0;
                should_persist = true;
            }
            requests_used_sum = requests_used_sum.saturating_add(entry.requests_used);
            usd_used_sum += entry.usd_used_total;
        }

        if should_persist {
            store.updated_at = now;
            let _ = persist_exa_usage_ledger_to_path(&state.exa_usage_ledger_file_path, &store);
        }
    }

    let total_keys = keys.len() as u64;
    let request_limit = EXA_FREE_REQUESTS_PER_MONTH.saturating_mul(total_keys);
    let request_remaining = request_limit.saturating_sub(requests_used_sum);
    let usd_limit = EXA_KEY_BUDGET_USD * total_keys as f64;
    let usd_remaining = (usd_limit - usd_used_sum).max(0.0);

    let used = Some(requests_used_sum as f64);
    let limit = Some(request_limit as f64);
    let remaining = Some(request_remaining as f64);
    let unit = "requests".to_string();

    ProviderUsageSnapshot {
        configured: true,
        has_enabled_key: true,
        ok: true,
        fetched_at: now,
        used,
        limit,
        remaining,
        unit: Some(unit.clone()),
        secondary_used: Some(usd_used_sum),
        secondary_limit: Some(usd_limit),
        secondary_remaining: Some(usd_remaining),
        secondary_unit: Some("usd".to_string()),
        source: Some("local/ledger".to_string()),
        summary: format_usage_summary(used, limit, remaining, &unit),
        error: None,
    }
}

fn cached_provider_usage_if_fresh(
    cached: &UsageSnapshot,
    provider: &str,
    now: u64,
) -> Option<ProviderUsageSnapshot> {
    let snapshot = match provider {
        "firecrawl" => &cached.firecrawl,
        "tavily" => &cached.tavily,
        "exa" => &cached.exa,
        _ => return None,
    };
    if snapshot.fetched_at == 0 {
        return None;
    }
    let has_metrics =
        snapshot.used.is_some() || snapshot.limit.is_some() || snapshot.remaining.is_some();
    if now.saturating_sub(snapshot.fetched_at) < USAGE_CACHE_TTL_SECS
        && (has_metrics
            || snapshot
                .error
                .as_deref()
                .is_some_and(|v| v.contains("status 429")))
    {
        return Some(snapshot.clone());
    }
    None
}

#[tauri::command]
async fn get_usage_snapshot(state: tauri::State<'_, AppState>) -> Result<UsageSnapshot, String> {
    let config = state.config.read().await.clone();

    let firecrawl_keys = enabled_keys_for_provider(&config, "firecrawl");
    let tavily_keys = enabled_keys_for_provider(&config, "tavily");
    let exa_keys = enabled_keys_for_provider(&config, "exa");

    let now = now_ts();
    let cached = state.usage_cache.read().await.clone();
    let cached_firecrawl = cached.as_ref().map(|v| v.firecrawl.clone());
    let cached_tavily = cached.as_ref().map(|v| v.tavily.clone());
    let mut refreshed_any = false;

    let firecrawl = if !config.firecrawl_enabled() {
        ProviderUsageSnapshot::not_configured()
    } else if firecrawl_keys.is_empty() {
        ProviderUsageSnapshot::no_enabled_key()
    } else if cached_firecrawl
        .as_ref()
        .is_some_and(|v| now.saturating_sub(v.fetched_at) < USAGE_CACHE_TTL_SECS)
    {
        cached_firecrawl.unwrap()
    } else {
        refreshed_any = true;
        fetch_firecrawl_usage_for_keys(state.inner(), &config, &firecrawl_keys, false).await
    };

    let tavily = if !config.tavily_enabled() {
        ProviderUsageSnapshot::not_configured()
    } else if tavily_keys.is_empty() {
        ProviderUsageSnapshot::no_enabled_key()
    } else if cached_tavily
        .as_ref()
        .is_some_and(|v| now.saturating_sub(v.fetched_at) < USAGE_CACHE_TTL_SECS)
    {
        cached_tavily.unwrap()
    } else {
        refreshed_any = true;
        fetch_tavily_usage_for_keys(state.inner(), &config, &tavily_keys, false).await
    };

    let exa = if !config.exa_enabled() {
        ProviderUsageSnapshot::not_configured()
    } else if exa_keys.is_empty() {
        ProviderUsageSnapshot::no_enabled_key()
    } else {
        refreshed_any = true;
        fetch_exa_usage_for_keys(state.inner(), &config, &exa_keys).await
    };

    if !refreshed_any {
        if let Some(cached) = cached {
            return Ok(cached);
        }
    }

    let snapshot = UsageSnapshot {
        fetched_at: now_ts(),
        firecrawl,
        tavily,
        exa,
    };
    *state.usage_cache.write().await = Some(snapshot.clone());
    let _ = persist_usage_cache_to_path(&state.usage_cache_file_path, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
async fn get_provider_usage(
    state: tauri::State<'_, AppState>,
    provider: String,
    force: Option<bool>,
) -> Result<ProviderUsageSnapshot, String> {
    let provider = provider.to_ascii_lowercase();
    let config = state.config.read().await.clone();
    let now = now_ts();
    let force = force.unwrap_or(false);
    let cached = state.usage_cache.read().await.clone();
    let snapshot = match provider.as_str() {
        "firecrawl" => {
            if !config.firecrawl_enabled() {
                return Ok(ProviderUsageSnapshot::not_configured());
            }
            let keys = enabled_keys_for_provider(&config, "firecrawl");
            if keys.is_empty() {
                return Ok(ProviderUsageSnapshot::no_enabled_key());
            }
            if !force {
                if let Some(hit) = cached
                    .as_ref()
                    .and_then(|v| cached_provider_usage_if_fresh(v, "firecrawl", now))
                {
                    return Ok(hit);
                }
            }
            fetch_firecrawl_usage_for_keys(state.inner(), &config, &keys, force).await
        }
        "tavily" => {
            if !config.tavily_enabled() {
                return Ok(ProviderUsageSnapshot::not_configured());
            }
            let keys = enabled_keys_for_provider(&config, "tavily");
            if keys.is_empty() {
                return Ok(ProviderUsageSnapshot::no_enabled_key());
            }
            if !force {
                if let Some(hit) = cached
                    .as_ref()
                    .and_then(|v| cached_provider_usage_if_fresh(v, "tavily", now))
                {
                    return Ok(hit);
                }
            }
            fetch_tavily_usage_for_keys(state.inner(), &config, &keys, force).await
        }
        "exa" => {
            if !config.exa_enabled() {
                return Ok(ProviderUsageSnapshot::not_configured());
            }
            let keys = enabled_keys_for_provider(&config, "exa");
            if keys.is_empty() {
                return Ok(ProviderUsageSnapshot::no_enabled_key());
            }
            fetch_exa_usage_for_keys(state.inner(), &config, &keys).await
        }
        _ => return Err("Invalid provider, expected firecrawl/tavily/exa".to_string()),
    };

    persist_provider_usage_snapshot(state.inner(), &config, provider.as_str(), snapshot.clone())
        .await;

    Ok(snapshot)
}

#[tauri::command]
async fn get_runtime_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<RuntimeMetricsSnapshot, String> {
    let metrics = state.metrics.lock().await;
    Ok(metrics.snapshot())
}

#[tauri::command]
async fn load_dashboard_state(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardPersistedState, String> {
    Ok(state.dashboard_state.read().await.clone())
}

#[tauri::command]
async fn save_dashboard_state(
    state: tauri::State<'_, AppState>,
    payload: DashboardPersistedState,
) -> Result<(), String> {
    let mut normalized = payload;
    normalized.version = normalized.version.max(1);
    *state.dashboard_state.write().await = normalized.clone();
    persist_dashboard_state_to_path(&state.dashboard_state_file_path, &normalized)?;
    Ok(())
}

#[tauri::command]
async fn build_mcp_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    target: Option<String>,
) -> Result<String, String> {
    let config = state.config.read().await.clone();
    let tavily_launcher = if config.tavily_enabled() {
        Some(ensure_tavily_local_mcp_launcher(&app)?)
    } else {
        None
    };
    let exa_launcher = if config.exa_enabled() {
        Some(ensure_exa_local_mcp_launcher(&app)?)
    } else {
        None
    };
    let payload = build_mcp_payload(
        &config,
        target
            .unwrap_or_else(|| "all".to_string())
            .to_ascii_lowercase()
            .as_str(),
        tavily_launcher.as_ref(),
        exa_launcher.as_ref(),
    )?;

    serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to serialize MCP config: {}", e))
}

fn build_tavily_mcp_server(
    config: &ProxyConfig,
    proxy_token: &str,
    launcher: &TavilyMcpLaunchConfig,
) -> serde_json::Value {
    json!({
      "command": launcher.command,
      "args": launcher.args,
      "env": {
        "TAVILY_API_URL": config.tavily_listen_url(),
        "TAVILY_API_KEY": proxy_token
      }
    })
}

fn build_exa_mcp_server(
    config: &ProxyConfig,
    proxy_token: &str,
    launcher: &ExaMcpLaunchConfig,
) -> serde_json::Value {
    json!({
      "command": launcher.command,
      "args": launcher.args,
      "env": {
        "EXA_API_URL": config.exa_listen_url(),
        "EXA_API_KEY": proxy_token
      }
    })
}

fn build_mcp_payload(
    config: &ProxyConfig,
    target: &str,
    tavily_launcher: Option<&TavilyMcpLaunchConfig>,
    exa_launcher: Option<&ExaMcpLaunchConfig>,
) -> Result<serde_json::Value, String> {
    config.validate_common()?;
    config.validate_provider_completeness()?;

    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();
    let exa_enabled = config.exa_enabled();
    let proxy_token = config.proxy_token.clone();
    let mut servers = serde_json::Map::new();

    match target {
        "both" | "all" => {
            if firecrawl_enabled {
                servers.insert(
                    "firecrawl".to_string(),
                    json!({
                      "command": "npx",
                      "args": ["-y", "firecrawl-mcp"],
                      "env": {
                        "FIRECRAWL_API_URL": config.listen_url(),
                        "FIRECRAWL_API_KEY": proxy_token.clone()
                      }
                    }),
                );
            }
            if tavily_enabled {
                let launcher = tavily_launcher
                    .ok_or_else(|| "Tavily MCP launcher is not ready".to_string())?;
                servers.insert(
                    "tavily".to_string(),
                    build_tavily_mcp_server(config, &proxy_token, launcher),
                );
            }
            if exa_enabled {
                let launcher =
                    exa_launcher.ok_or_else(|| "Exa MCP launcher is not ready".to_string())?;
                servers.insert(
                    "exa".to_string(),
                    build_exa_mcp_server(config, &proxy_token, launcher),
                );
            }
        }
        "firecrawl" => {
            if !firecrawl_enabled {
                return Err("Firecrawl is not fully configured".to_string());
            }
            servers.insert(
                "firecrawl".to_string(),
                json!({
                  "command": "npx",
                  "args": ["-y", "firecrawl-mcp"],
                  "env": {
                    "FIRECRAWL_API_URL": config.listen_url(),
                    "FIRECRAWL_API_KEY": proxy_token.clone()
                  }
                }),
            );
        }
        "tavily" => {
            if !tavily_enabled {
                return Err("Tavily is not fully configured".to_string());
            }
            let launcher =
                tavily_launcher.ok_or_else(|| "Tavily MCP launcher is not ready".to_string())?;
            servers.insert(
                "tavily".to_string(),
                build_tavily_mcp_server(config, &proxy_token, launcher),
            );
        }
        "exa" => {
            if !exa_enabled {
                return Err("Exa is not fully configured".to_string());
            }
            let launcher =
                exa_launcher.ok_or_else(|| "Exa MCP launcher is not ready".to_string())?;
            servers.insert(
                "exa".to_string(),
                build_exa_mcp_server(config, &proxy_token, launcher),
            );
        }
        _ => return Err("Invalid MCP target, expected firecrawl/tavily/exa/all".to_string()),
    }

    if servers.is_empty() {
        return Err("No configured MCP providers are available for this target".to_string());
    }

    Ok(json!({ "mcpServers": servers }))
}

#[tauri::command]
async fn get_launch_on_login_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Failed to read launch-on-login state: {}", e))
}

#[tauri::command]
async fn set_launch_on_login_enabled(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| format!("Failed to enable launch-on-login: {}", e))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable launch-on-login: {}", e))?;
    }
    manager
        .is_enabled()
        .map_err(|e| format!("Failed to verify launch-on-login state: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> ProxyConfig {
        ProxyConfig {
            proxy_token: "token".to_string(),
            firecrawl_api_keys: Vec::new(),
            firecrawl_disabled_api_keys: Vec::new(),
            upstream_base_url: String::new(),
            tavily_api_keys: Vec::new(),
            tavily_disabled_api_keys: Vec::new(),
            tavily_upstream_base_url: String::new(),
            exa_api_keys: Vec::new(),
            exa_disabled_api_keys: Vec::new(),
            exa_upstream_base_url: String::new(),
            request_timeout_ms: 60_000,
            key_cooldown_seconds: 60,
            auto_start: true,
            silent_start: false,
            host: "127.0.0.1".to_string(),
            port: 8787,
            tavily_port: 8788,
            exa_port: 8789,
        }
    }

    fn tavily_launcher() -> TavilyMcpLaunchConfig {
        TavilyMcpLaunchConfig {
            command: "node".to_string(),
            args: vec!["/tmp/tavily-local-proxy-mcp.mjs".to_string()],
        }
    }

    #[test]
    fn validate_allows_single_firecrawl_provider() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_partial_tavily_provider() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];

        let err = config
            .validate()
            .expect_err("expected partial config error");
        assert!(err.contains("Tavily config is incomplete"));
    }

    #[test]
    fn build_mcp_payload_both_returns_only_configured_provider() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();

        let payload =
            build_mcp_payload(&config, "both", None, None).expect("mcp payload should build");
        let servers = payload
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .expect("mcpServers should be an object");

        assert!(servers.contains_key("firecrawl"));
        assert!(!servers.contains_key("tavily"));
    }

    #[test]
    fn build_mcp_payload_rejects_unconfigured_target() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();

        let err = build_mcp_payload(&config, "tavily", None, None)
            .expect_err("tavily target should fail when not configured");
        assert!(err.contains("Tavily is not fully configured"));
    }

    #[test]
    fn build_mcp_payload_tavily_uses_local_launcher() {
        let mut config = base_config();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];
        config.tavily_upstream_base_url = "https://api.tavily.com".to_string();

        let launcher = tavily_launcher();
        let payload = build_mcp_payload(&config, "tavily", Some(&launcher), None)
            .expect("tavily payload should build");
        let tavily = payload
            .get("mcpServers")
            .and_then(|v| v.get("tavily"))
            .and_then(|v| v.as_object())
            .expect("tavily mcp server should exist");

        assert_eq!(tavily.get("command").and_then(|v| v.as_str()), Some("node"));
        assert_eq!(
            tavily
                .get("args")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.as_str()),
            Some("/tmp/tavily-local-proxy-mcp.mjs")
        );
        assert_eq!(
            tavily
                .get("env")
                .and_then(|v| v.get("TAVILY_API_URL"))
                .and_then(|v| v.as_str()),
            Some("http://127.0.0.1:8788")
        );
        assert_eq!(
            tavily
                .get("env")
                .and_then(|v| v.get("TAVILY_API_KEY"))
                .and_then(|v| v.as_str()),
            Some("token")
        );
    }

    #[test]
    fn build_mcp_payload_tavily_requires_launcher() {
        let mut config = base_config();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];
        config.tavily_upstream_base_url = "https://api.tavily.com".to_string();

        let err = build_mcp_payload(&config, "tavily", None, None)
            .expect_err("tavily payload should require local launcher");
        assert!(err.contains("launcher"));
    }

    #[test]
    fn normalized_filters_disabled_keys_not_in_provider_list() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string(), "fc-key-2".to_string()];
        config.firecrawl_disabled_api_keys = vec!["fc-key-2".to_string(), "fc-key-9".to_string()];

        let normalized = config.normalized();
        assert_eq!(
            normalized.firecrawl_disabled_api_keys,
            vec!["fc-key-2".to_string()]
        );
    }

    #[test]
    fn retryable_401_disables_key() {
        let mut manager = RoundRobinKeyManager::new(vec!["fc-key-1".to_string()], Vec::new(), 60);
        let selected = manager.select_key().expect("one key should be selectable");
        let action = manager.mark_retryable_failure(selected.index, 401);
        assert!(matches!(action, KeyFailureAction::DisabledBy401));
        assert_eq!(manager.active_key_count(), 0);
    }

    #[test]
    fn derive_status_flags_handles_running_and_degraded_states() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];
        config.tavily_upstream_base_url = "https://api.tavily.com".to_string();

        let all_running = derive_status_flags(&config, true, true, false);
        assert_eq!(all_running, (true, true, false, true, true, false));

        let degraded = derive_status_flags(&config, true, false, false);
        assert_eq!(degraded, (false, true, true, true, true, false));

        let all_stopped = derive_status_flags(&config, false, false, false);
        assert_eq!(all_stopped, (false, false, false, true, true, false));
    }

    #[test]
    fn cached_provider_usage_if_fresh_hits_within_ttl() {
        fn make_usage(fetched_at: u64) -> ProviderUsageSnapshot {
            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: true,
                fetched_at,
                used: Some(1.0),
                limit: Some(10.0),
                remaining: Some(9.0),
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: "ok".to_string(),
                error: None,
            }
        }

        let now = 1_000u64;
        let fetched_at = now - 10;
        let cached = UsageSnapshot {
            fetched_at,
            firecrawl: make_usage(fetched_at),
            tavily: make_usage(fetched_at),
            exa: make_usage(fetched_at),
        };

        let hit = cached_provider_usage_if_fresh(&cached, "firecrawl", now)
            .expect("expected usage cache hit within ttl");
        assert_eq!(hit.fetched_at, fetched_at);
    }

    #[test]
    fn cached_provider_usage_if_fresh_misses_after_ttl() {
        fn make_usage(fetched_at: u64) -> ProviderUsageSnapshot {
            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: true,
                fetched_at,
                used: Some(1.0),
                limit: Some(10.0),
                remaining: Some(9.0),
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: "ok".to_string(),
                error: None,
            }
        }

        let now = 1_000u64;
        let fetched_at = now - (USAGE_CACHE_TTL_SECS + 1);
        let cached = UsageSnapshot {
            fetched_at,
            firecrawl: make_usage(fetched_at),
            tavily: make_usage(fetched_at),
            exa: make_usage(fetched_at),
        };

        assert!(
            cached_provider_usage_if_fresh(&cached, "firecrawl", now).is_none(),
            "expected usage cache miss after ttl"
        );
    }

    #[test]
    fn cached_provider_usage_if_fresh_hits_for_429_errors() {
        fn make_usage_error(fetched_at: u64, error: &str) -> ProviderUsageSnapshot {
            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: false,
                fetched_at,
                used: None,
                limit: None,
                remaining: None,
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: "Usage data unavailable".to_string(),
                error: Some(error.to_string()),
            }
        }

        let now = 1_000u64;
        let fetched_at = now - 10;
        let cached = UsageSnapshot {
            fetched_at,
            firecrawl: make_usage_error(fetched_at, "status 429 body {\"error\":\"Rate limit\"}"),
            tavily: make_usage_error(fetched_at, "status 429 body {}"),
            exa: make_usage_error(fetched_at, "status 429 body {}"),
        };

        assert!(
            cached_provider_usage_if_fresh(&cached, "firecrawl", now).is_some(),
            "expected usage cache hit for 429 within ttl"
        );
    }

    #[test]
    fn cached_provider_usage_if_fresh_hits_for_partial_metrics() {
        let now = 1_000u64;
        let fetched_at = now - 10;
        let cached = UsageSnapshot {
            fetched_at,
            firecrawl: ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: false,
                fetched_at,
                used: Some(1.0),
                limit: Some(10.0),
                remaining: Some(9.0),
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: "partial".to_string(),
                error: None,
            },
            tavily: ProviderUsageSnapshot::pending(),
            exa: ProviderUsageSnapshot::pending(),
        };

        assert!(
            cached_provider_usage_if_fresh(&cached, "firecrawl", now).is_some(),
            "expected usage cache hit for partial metrics within ttl"
        );
    }

    #[test]
    fn parse_retry_after_secs_from_usage_error_extracts_seconds() {
        assert_eq!(
            parse_retry_after_secs_from_usage_error(
                "status 429 body {\"error\":\"Rate limit exceeded, please retry after 50s\"}"
            ),
            Some(50)
        );
        assert_eq!(
            parse_retry_after_secs_from_usage_error("status 429 body retry after 7s, resets soon"),
            Some(7)
        );
        assert_eq!(
            parse_retry_after_secs_from_usage_error("status 429 body {\"error\":\"Rate limit\"}"),
            None
        );
    }

    #[test]
    fn cached_provider_usage_if_fresh_skips_non_429_errors() {
        fn make_usage_error(fetched_at: u64, error: &str) -> ProviderUsageSnapshot {
            ProviderUsageSnapshot {
                configured: true,
                has_enabled_key: true,
                ok: false,
                fetched_at,
                used: None,
                limit: None,
                remaining: None,
                unit: Some("credits".to_string()),
                secondary_used: None,
                secondary_limit: None,
                secondary_remaining: None,
                secondary_unit: None,
                source: Some("/usage".to_string()),
                summary: "Usage data unavailable".to_string(),
                error: Some(error.to_string()),
            }
        }

        let now = 1_000u64;
        let fetched_at = now - 10;
        let cached = UsageSnapshot {
            fetched_at,
            firecrawl: make_usage_error(
                fetched_at,
                "status 401 body {\"detail\":\"Unauthorized\"}",
            ),
            tavily: make_usage_error(fetched_at, "status 500 body {\"detail\":\"Server\"}"),
            exa: make_usage_error(fetched_at, "request failed: timeout"),
        };

        assert!(
            cached_provider_usage_if_fresh(&cached, "firecrawl", now).is_none(),
            "expected cache bypass for 401 within ttl"
        );
        assert!(
            cached_provider_usage_if_fresh(&cached, "tavily", now).is_none(),
            "expected cache bypass for non-429 errors within ttl"
        );
    }
}

fn show_main_window<R: tauri::Runtime, M: Manager<R>>(manager: &M) {
    #[cfg(target_os = "macos")]
    let _ = manager.app_handle().set_dock_visibility(true);

    if let Some(window) = manager.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn macos_tray_template_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::new(include_bytes!("../icons/trayTemplate.rgba"), 64, 64)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let silent_start_enabled = Arc::new(AtomicBool::new(false));
    let silent_start_applied = Arc::new(AtomicBool::new(false));
    let main_page_load_started = Arc::new(AtomicBool::new(false));

    let silent_start_enabled_for_page = silent_start_enabled.clone();
    let silent_start_applied_for_page = silent_start_applied.clone();
    let main_page_load_started_for_page = main_page_load_started.clone();

    tauri::Builder::default()
        .on_page_load(move |webview, payload| {
            use tauri::webview::PageLoadEvent;

            let window = webview.window();
            if window.label() != "main" {
                return;
            }

            if payload.event() != PageLoadEvent::Started {
                return;
            }

            main_page_load_started_for_page.store(true, Ordering::SeqCst);

            if !silent_start_enabled_for_page.load(Ordering::SeqCst) {
                return;
            }

            if silent_start_applied_for_page.swap(true, Ordering::SeqCst) {
                return;
            }

            let _ = window.hide();
            #[cfg(target_os = "macos")]
            let _ = window.app_handle().set_dock_visibility(false);
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();

                #[cfg(target_os = "macos")]
                let _ = window.app_handle().set_dock_visibility(false);
            }
        })
        .on_menu_event(|app, event| {
            if event.id() == "tray_toggle_proxy" {
                let app_state = match app.try_state::<AppState>() {
                    Some(state) => state.inner().clone(),
                    None => return,
                };

                tauri::async_runtime::spawn(async move {
                    let should_stop = has_running_proxy(&app_state).await;
                    let result = if should_stop {
                        stop_proxy_internal(&app_state).await.map(|_| ())
                    } else {
                        start_proxy_internal(&app_state).await.map(|_| ())
                    };

                    if let Err(err) = result {
                        let action = if should_stop { "stop" } else { "start" };
                        append_log(
                            &app_state.logs,
                            "ERROR",
                            format!("Tray failed to {} proxy: {}", action, err),
                        )
                        .await;
                    }
                });
            } else if event.id() == "tray_show" {
                show_main_window(app);
            } else if event.id() == "tray_quit" {
                app.exit(0);
            }
        })
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup({
            let silent_start_enabled = silent_start_enabled.clone();
            let silent_start_applied = silent_start_applied.clone();
            let main_page_load_started = main_page_load_started.clone();

            move |app| {
                let config = load_or_init_config(&app.handle())?;
                let config_file_path = config_path(&app.handle())?;
                let usage_cache_file_path = usage_cache_path(&app.handle())?;
                let usage_cache = load_usage_cache_from_path(&usage_cache_file_path);
                let dashboard_state_file_path = dashboard_state_path(&app.handle())?;
                let dashboard_state = load_dashboard_state_from_path(&dashboard_state_file_path);
                let key_health_file_path = key_health_path(&app.handle())?;
                let key_health =
                    load_key_health_from_path(&key_health_file_path).unwrap_or_default();
                let exa_usage_ledger_file_path = exa_usage_ledger_path(&app.handle())?;
                let exa_usage_ledger =
                    load_exa_usage_ledger_from_path(&exa_usage_ledger_file_path);
                let mut logs = VecDeque::new();
                logs.push_back(format!(
                    "{} [INFO] App initialized. Config path is in app data directory.",
                    now_ts()
                ));

                let app_state = AppState {
                    config: Arc::new(RwLock::new(config.clone())),
                    config_file_path,
                    usage_cache_file_path,
                    usage_cache: Arc::new(RwLock::new(usage_cache)),
                    dashboard_state_file_path,
                    dashboard_state: Arc::new(RwLock::new(dashboard_state)),
                    key_health_file_path,
                    key_health: Arc::new(RwLock::new(key_health)),
                    exa_usage_ledger_file_path,
                    exa_usage_ledger: Arc::new(RwLock::new(exa_usage_ledger)),
                    runtime: Arc::new(Mutex::new(ProxyRuntime::default())),
                    logs: Arc::new(Mutex::new(logs)),
                    metrics: Arc::new(Mutex::new(RuntimeMetrics::default())),
                    active_key_managers: Arc::new(Mutex::new(ActiveKeyManagers::default())),
                };

                app.manage(app_state.clone());
                let sync_state = app_state.clone();
                tauri::async_runtime::spawn(async move {
                    sync_key_health_with_config(&sync_state).await;
                    sync_exa_usage_ledger_with_config(&sync_state).await;
                });

                // 提前为托盘后台任务 clone，避免 auto_start 闭包 move 后无法借用
                let tray_state = app_state.clone();

                if config.auto_start {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = start_proxy_internal(&app_state).await {
                            println!("Failed to auto-start proxy: {}", e);
                        }
                    });
                }

                // 静默启动：避免在 WebView 首次加载前 hide 导致 macOS 白屏。
                // 具体做法：先记录配置；等到主窗口触发 PageLoadEvent::Started 后再 hide。
                silent_start_enabled.store(config.silent_start, Ordering::SeqCst);
                if config.silent_start
                    && main_page_load_started.load(Ordering::SeqCst)
                    && !silent_start_applied.swap(true, Ordering::SeqCst)
                {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                    #[cfg(target_os = "macos")]
                    let _ = app.set_dock_visibility(false);
                }

                // 托盘菜单：状态项 + 常规操作
                let status_item =
                    MenuItem::with_id(app, "tray_status", "⏹ 代理已停止", false, None::<&str>)
                        .map_err(|e| format!("Failed to create tray status item: {}", e))?;

                let toggle_item = MenuItem::with_id(
                    app,
                    "tray_toggle_proxy",
                    "▶️ 启动代理",
                    true,
                    None::<&str>,
                )
                .map_err(|e| format!("Failed to create tray toggle item: {}", e))?;

                let tray_menu = MenuBuilder::new(app)
                    .item(&status_item)
                    .item(&toggle_item)
                    .separator()
                    .text("tray_show", "显示窗口")
                    .separator()
                    .text("tray_quit", "退出")
                    .build()
                    .map_err(|e| format!("Failed to build tray menu: {}", e))?;

                let mut tray = TrayIconBuilder::with_id("main-tray")
                    .menu(&tray_menu)
                    .tooltip("Balance Proxy - 代理已停止")
                    .show_menu_on_left_click(true);

                #[cfg(target_os = "macos")]
                {
                    tray = tray.icon_as_template(true);
                    tray = tray.icon(macos_tray_template_icon());
                }

                #[cfg(not(target_os = "macos"))]
                if let Some(icon) = app.default_window_icon().cloned() {
                    tray = tray.icon(icon);
                }

                tray.build(app)
                    .map_err(|e| format!("Failed to create tray icon: {}", e))?;

                // 后台定时刷新托盘状态
                let tray_app = app.handle().clone();
                let status_item_bg = status_item.clone();
                let toggle_item_bg = toggle_item.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(3)).await;

                        let config = tray_state.config.read().await.clone();
                        let runtime = tray_state.runtime.lock().await;
                        let status = compose_proxy_status(&runtime, &config);
                        drop(runtime);

                        let (label, tooltip) = if status.running {
                            let urls: Vec<String> = [
                                status
                                    .listen_url
                                    .as_deref()
                                    .map(|u| format!("FC {}", u.trim_start_matches("http://"))),
                                status
                                    .tavily_listen_url
                                    .as_deref()
                                    .map(|u| format!("TV {}", u.trim_start_matches("http://"))),
                                status
                                    .exa_listen_url
                                    .as_deref()
                                    .map(|u| format!("EXA {}", u.trim_start_matches("http://"))),
                            ]
                            .into_iter()
                            .flatten()
                            .collect();
                            let url_str = urls.join(" | ");
                            (
                                format!("🟢 运行中"),
                                format!("Balance Proxy - 运行中\n{}", url_str),
                            )
                        } else if status.any_running {
                            (
                                "🟡 部分运行".to_string(),
                                "Balance Proxy - 部分运行".to_string(),
                            )
                        } else {
                            (
                                "⏹ 代理已停止".to_string(),
                                "Balance Proxy - 代理已停止".to_string(),
                            )
                        };

                        let _ = status_item_bg.set_text(&label);
                        let _ = toggle_item_bg.set_text(if status.any_running {
                            "⏹ 停止代理"
                        } else {
                            "▶️ 启动代理"
                        });
                        if let Some(t) = tray_app.tray_by_id("main-tray") {
                            let _ = t.set_tooltip(Some(tooltip.as_str()));
                        }
                    }
                });

                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_proxy_config,
            reload_proxy_config,
            save_proxy_config,
            get_proxy_status,
            start_proxy,
            stop_proxy,
            get_recent_logs,
            get_key_status,
            get_key_status_snapshot,
            get_usage_snapshot,
            get_provider_usage,
            get_runtime_metrics,
            load_dashboard_state,
            save_dashboard_state,
            build_mcp_config,
            get_launch_on_login_enabled,
            set_launch_on_login_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
