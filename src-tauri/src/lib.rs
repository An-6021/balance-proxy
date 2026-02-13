use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::PathBuf;
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
use tauri::menu::MenuBuilder;
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
    upstream_base_url: String,
    tavily_api_keys: Vec<String>,
    tavily_upstream_base_url: String,
    request_timeout_ms: u64,
    key_cooldown_seconds: u64,
    host: String,
    port: u16,
    tavily_port: u16,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_token: "your-local-token".to_string(),
            firecrawl_api_keys: Vec::new(),
            upstream_base_url: "https://api.firecrawl.dev".to_string(),
            tavily_api_keys: Vec::new(),
            tavily_upstream_base_url: "https://api.tavily.com".to_string(),
            request_timeout_ms: 60_000,
            key_cooldown_seconds: 60,
            host: "127.0.0.1".to_string(),
            port: 8787,
            tavily_port: 8788,
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
        self.tavily_api_keys = split_and_dedupe_keys(&self.tavily_api_keys);
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
        if self.port == self.tavily_port {
            return Err("PORT and TAVILY_PORT must be different".to_string());
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
        if !self.firecrawl_enabled() && !self.tavily_enabled() {
            return Err(
                "At least one provider must be fully configured (Firecrawl or Tavily)".to_string(),
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyStatus {
    index: usize,
    key_preview: String,
    is_cooling_down: bool,
    cooldown_remaining_secs: u64,
    fail_count: u64,
}

fn truncate_key(key: &str) -> String {
    if key.len() <= 14 {
        key.to_string()
    } else {
        format!("{}...{}", &key[..8], &key[key.len() - 5..])
    }
}

fn idle_key_statuses(keys: &[String]) -> Vec<KeyStatus> {
    keys.iter()
        .enumerate()
        .map(|(i, k)| KeyStatus {
            index: i,
            key_preview: truncate_key(k),
            is_cooling_down: false,
            cooldown_remaining_secs: 0,
            fail_count: 0,
        })
        .collect()
}

fn derive_status_flags(
    config: &ProxyConfig,
    firecrawl_running: bool,
    tavily_running: bool,
) -> (bool, bool, bool, bool, bool) {
    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();

    let enabled_count = firecrawl_enabled as usize + tavily_enabled as usize;
    let running_enabled_count = (firecrawl_enabled && firecrawl_running) as usize
        + (tavily_enabled && tavily_running) as usize;
    let any_running = firecrawl_running || tavily_running;
    let running = enabled_count > 0 && running_enabled_count == enabled_count;
    let degraded = any_running && !running;

    (
        running,
        any_running,
        degraded,
        firecrawl_enabled,
        tavily_enabled,
    )
}

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<ProxyConfig>>,
    runtime: Arc<Mutex<ProxyRuntime>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    active_key_managers: Arc<Mutex<ActiveKeyManagers>>,
}

#[derive(Default)]
struct ProxyRuntime {
    firecrawl_handle: Option<ServerHandle>,
    tavily_handle: Option<ServerHandle>,
}

#[derive(Default)]
struct ActiveKeyManagers {
    firecrawl: Option<Arc<Mutex<RoundRobinKeyManager>>>,
    tavily: Option<Arc<Mutex<RoundRobinKeyManager>>>,
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
    firecrawl_enabled: bool,
    tavily_enabled: bool,
    firecrawl_running: bool,
    tavily_running: bool,
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
}

#[derive(Clone)]
struct ProxyServerState {
    provider: &'static str,
    proxy_token: String,
    upstream_base_url: String,
    key_manager: Arc<Mutex<RoundRobinKeyManager>>,
    http_client: Client,
    logs: Arc<Mutex<VecDeque<String>>>,
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

struct RoundRobinKeyManager {
    keys: Vec<String>,
    next_index: usize,
    cooldown_until: Vec<Option<Instant>>,
    fail_count: Vec<u64>,
    cooldown_seconds: u64,
}

impl RoundRobinKeyManager {
    fn new(keys: Vec<String>, cooldown_seconds: u64) -> Self {
        let key_count = keys.len();
        Self {
            keys,
            next_index: 0,
            cooldown_until: vec![None; key_count],
            fail_count: vec![0; key_count],
            cooldown_seconds,
        }
    }

    fn key_count(&self) -> usize {
        self.keys.len()
    }

    fn select_key(&mut self) -> SelectedKey {
        let now = Instant::now();
        let count = self.keys.len();
        let start = self.next_index % count;

        let mut earliest_idx = start;
        let mut earliest_wait = Duration::MAX;

        for offset in 0..count {
            let idx = (start + offset) % count;
            let wait = match self.cooldown_until[idx] {
                Some(deadline) if deadline > now => deadline - now,
                _ => Duration::ZERO,
            };

            if wait == Duration::ZERO {
                self.next_index = (idx + 1) % count;
                return SelectedKey {
                    index: idx,
                    value: self.keys[idx].clone(),
                };
            }

            if wait < earliest_wait {
                earliest_wait = wait;
                earliest_idx = idx;
            }
        }

        self.next_index = (earliest_idx + 1) % count;
        SelectedKey {
            index: earliest_idx,
            value: self.keys[earliest_idx].clone(),
        }
    }

    fn mark_retryable_failure(&mut self, key_index: usize) {
        self.fail_count[key_index] += 1;
        self.cooldown_until[key_index] =
            Some(Instant::now() + Duration::from_secs(self.cooldown_seconds));
    }

    fn get_statuses(&self) -> Vec<KeyStatus> {
        let now = Instant::now();
        self.keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let (is_cooling_down, remaining) = match self.cooldown_until[i] {
                    Some(deadline) if deadline > now => (true, (deadline - now).as_secs()),
                    _ => (false, 0),
                };
                KeyStatus {
                    index: i,
                    key_preview: truncate_key(key),
                    is_cooling_down,
                    cooldown_remaining_secs: remaining,
                    fail_count: self.fail_count[i],
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

fn tavily_local_mcp_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data_dir.join(TAVILY_LOCAL_MCP_SCRIPT_FILENAME))
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
    if provider.eq_ignore_ascii_case("tavily") {
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

    let max_attempts = {
        let manager = state.key_manager.lock().await;
        manager.key_count()
    };

    for attempt in 0..max_attempts {
        let selected = {
            let mut manager = state.key_manager.lock().await;
            manager.select_key()
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
        if RETRYABLE_STATUS_CODES.contains(&status.as_u16()) {
            {
                let mut manager = state.key_manager.lock().await;
                manager.mark_retryable_failure(selected.index);
            }
            if attempt < max_attempts - 1 {
                retry_count += 1;
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
    append_log(
        &state.logs,
        "INFO",
        format!("Config saved: {}", path.to_string_lossy()),
    )
    .await;

    Ok(path.to_string_lossy().to_string())
}

fn compose_proxy_status(runtime: &ProxyRuntime, config: &ProxyConfig) -> ProxyStatus {
    let firecrawl_listen_url = runtime
        .firecrawl_handle
        .as_ref()
        .map(|h| h.listen_url.clone());
    let tavily_listen_url = runtime.tavily_handle.as_ref().map(|h| h.listen_url.clone());
    let firecrawl_running = firecrawl_listen_url.is_some();
    let tavily_running = tavily_listen_url.is_some();
    let (running, any_running, degraded, firecrawl_enabled, tavily_enabled) =
        derive_status_flags(config, firecrawl_running, tavily_running);

    ProxyStatus {
        running,
        any_running,
        degraded,
        listen_url: firecrawl_listen_url,
        tavily_listen_url,
        firecrawl_enabled,
        tavily_enabled,
        firecrawl_running,
        tavily_running,
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
    let config = state.config.read().await.clone();
    config.validate()?;
    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();

    let (start_firecrawl, start_tavily) = {
        let runtime = state.runtime.lock().await;
        (
            firecrawl_enabled && runtime.firecrawl_handle.is_none(),
            tavily_enabled && runtime.tavily_handle.is_none(),
        )
    };

    if !start_firecrawl && !start_tavily {
        let runtime = state.runtime.lock().await;
        return Ok(compose_proxy_status(&runtime, &config));
    }

    let http_client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut new_firecrawl_handle: Option<ServerHandle> = None;
    let mut new_tavily_handle: Option<ServerHandle> = None;
    let mut new_firecrawl_manager: Option<Arc<Mutex<RoundRobinKeyManager>>> = None;
    let mut new_tavily_manager: Option<Arc<Mutex<RoundRobinKeyManager>>> = None;

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
            config.key_cooldown_seconds,
        )));
        new_firecrawl_manager = Some(firecrawl_key_manager.clone());

        let firecrawl_state = ProxyServerState {
            provider: "firecrawl",
            proxy_token: config.proxy_token.clone(),
            upstream_base_url: config.upstream_base_url.clone(),
            key_manager: firecrawl_key_manager,
            http_client: http_client.clone(),
            logs: state.logs.clone(),
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
            config.key_cooldown_seconds,
        )));
        new_tavily_manager = Some(tavily_key_manager.clone());

        let tavily_state = ProxyServerState {
            provider: "tavily",
            proxy_token: config.proxy_token.clone(),
            upstream_base_url: config.tavily_upstream_base_url.clone(),
            key_manager: tavily_key_manager,
            http_client: http_client.clone(),
            logs: state.logs.clone(),
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

    let status = {
        let mut runtime = state.runtime.lock().await;
        if let Some(handle) = new_firecrawl_handle {
            runtime.firecrawl_handle = Some(handle);
        }
        if let Some(handle) = new_tavily_handle {
            runtime.tavily_handle = Some(handle);
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
    }

    Ok(status)
}

#[tauri::command]
async fn stop_proxy(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    let config = state.config.read().await.clone();
    let (firecrawl_handle, tavily_handle) = {
        let mut runtime = state.runtime.lock().await;
        (
            runtime.firecrawl_handle.take(),
            runtime.tavily_handle.take(),
        )
    };

    if firecrawl_handle.is_none() && tavily_handle.is_none() {
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

    // Clear active key manager references
    {
        let mut active = state.active_key_managers.lock().await;
        active.firecrawl = None;
        active.tavily = None;
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
    configured: bool,
    running: bool,
    keys: &[String],
    active_manager: Option<Arc<Mutex<RoundRobinKeyManager>>>,
) -> ProviderKeyStatusSnapshot {
    let keys = if let Some(manager) = active_manager {
        manager.lock().await.get_statuses()
    } else {
        idle_key_statuses(keys)
    };

    ProviderKeyStatusSnapshot {
        configured,
        running,
        keys,
    }
}

async fn build_key_status_snapshot_inner(state: &AppState) -> KeyStatusSnapshot {
    let config = state.config.read().await.clone();
    let firecrawl_configured = config.firecrawl_enabled();
    let tavily_configured = config.tavily_enabled();

    let (firecrawl_running, tavily_running) = {
        let runtime = state.runtime.lock().await;
        (
            runtime.firecrawl_handle.is_some(),
            runtime.tavily_handle.is_some(),
        )
    };

    let (active_firecrawl, active_tavily) = {
        let active = state.active_key_managers.lock().await;
        (active.firecrawl.clone(), active.tavily.clone())
    };

    let firecrawl = build_provider_key_status(
        firecrawl_configured,
        firecrawl_running,
        &config.firecrawl_api_keys,
        active_firecrawl,
    )
    .await;
    let tavily = build_provider_key_status(
        tavily_configured,
        tavily_running,
        &config.tavily_api_keys,
        active_tavily,
    )
    .await;

    KeyStatusSnapshot { firecrawl, tavily }
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
    let payload = build_mcp_payload(
        &config,
        target
            .unwrap_or_else(|| "both".to_string())
            .to_ascii_lowercase()
            .as_str(),
        tavily_launcher.as_ref(),
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

fn build_mcp_payload(
    config: &ProxyConfig,
    target: &str,
    tavily_launcher: Option<&TavilyMcpLaunchConfig>,
) -> Result<serde_json::Value, String> {
    config.validate_common()?;
    config.validate_provider_completeness()?;

    let firecrawl_enabled = config.firecrawl_enabled();
    let tavily_enabled = config.tavily_enabled();
    let proxy_token = config.proxy_token.clone();
    let mut servers = serde_json::Map::new();

    match target {
        "both" => {
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
        _ => return Err("Invalid MCP target, expected firecrawl/tavily/both".to_string()),
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
            upstream_base_url: String::new(),
            tavily_api_keys: Vec::new(),
            tavily_upstream_base_url: String::new(),
            request_timeout_ms: 60_000,
            key_cooldown_seconds: 60,
            host: "127.0.0.1".to_string(),
            port: 8787,
            tavily_port: 8788,
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

        let payload = build_mcp_payload(&config, "both", None).expect("mcp payload should build");
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

        let err = build_mcp_payload(&config, "tavily", None)
            .expect_err("tavily target should fail when not configured");
        assert!(err.contains("Tavily is not fully configured"));
    }

    #[test]
    fn build_mcp_payload_tavily_uses_local_launcher() {
        let mut config = base_config();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];
        config.tavily_upstream_base_url = "https://api.tavily.com".to_string();

        let launcher = tavily_launcher();
        let payload = build_mcp_payload(&config, "tavily", Some(&launcher))
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

        let err = build_mcp_payload(&config, "tavily", None)
            .expect_err("tavily payload should require local launcher");
        assert!(err.contains("launcher"));
    }

    #[test]
    fn derive_status_flags_handles_running_and_degraded_states() {
        let mut config = base_config();
        config.firecrawl_api_keys = vec!["fc-key-1".to_string()];
        config.upstream_base_url = "https://api.firecrawl.dev".to_string();
        config.tavily_api_keys = vec!["tvly-key-1".to_string()];
        config.tavily_upstream_base_url = "https://api.tavily.com".to_string();

        let all_running = derive_status_flags(&config, true, true);
        assert_eq!(all_running, (true, true, false, true, true));

        let degraded = derive_status_flags(&config, true, false);
        assert_eq!(degraded, (false, true, true, true, true));

        let all_stopped = derive_status_flags(&config, false, false);
        assert_eq!(all_stopped, (false, false, false, true, true));
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();

                #[cfg(target_os = "macos")]
                let _ = window.app_handle().set_dock_visibility(false);
            }
        })
        .on_menu_event(|app, event| {
            if event.id() == "tray_show" {
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
        .setup(|app| {
            let config = load_or_init_config(&app.handle())?;
            let mut logs = VecDeque::new();
            logs.push_back(format!(
                "{} [INFO] App initialized. Config path is in app data directory.",
                now_ts()
            ));

            app.manage(AppState {
                config: Arc::new(RwLock::new(config)),
                runtime: Arc::new(Mutex::new(ProxyRuntime::default())),
                logs: Arc::new(Mutex::new(logs)),
                active_key_managers: Arc::new(Mutex::new(ActiveKeyManagers::default())),
            });

            let tray_menu = MenuBuilder::new(app)
                .text("tray_show", "Show Window")
                .separator()
                .text("tray_quit", "Quit")
                .build()
                .map_err(|e| format!("Failed to build tray menu: {}", e))?;

            let mut tray = TrayIconBuilder::with_id("main-tray")
                .menu(&tray_menu)
                .tooltip("Balance Proxy")
                .show_menu_on_left_click(true);

            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }

            tray.build(app)
                .map_err(|e| format!("Failed to create tray icon: {}", e))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_proxy_config,
            save_proxy_config,
            get_proxy_status,
            start_proxy,
            stop_proxy,
            get_recent_logs,
            get_key_status,
            get_key_status_snapshot,
            build_mcp_config,
            get_launch_on_login_enabled,
            set_launch_on_login_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
