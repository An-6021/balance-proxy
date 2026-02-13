// ============================================
// 1. TAURI BRIDGE & UTILITIES
// ============================================
const invoke = window.__TAURI__?.core?.invoke;

if (!invoke) {
  document.body.innerHTML = '<div style="padding:40px;text-align:center;color:#666">Tauri runtime not found. Please run inside the desktop app.</div>';
}

const contentEl = document.getElementById("content");
const toastContainer = document.getElementById("toastContainer");
const sidebarBadge = document.getElementById("sidebarBadge");
const sidebarUrl = document.getElementById("sidebarUrl");
const sidebarNav = document.getElementById("sidebarNav");
const langToggle = document.getElementById("langToggle");

// ============================================
// 2. I18N
// ============================================
const I18N = {
  zh: {
    // Sidebar nav
    "nav.dashboard": "仪表盘",
    "nav.config": "配置",
    "nav.keys": "API Keys",
    "nav.mcp": "MCP 配置",
    "nav.logs": "日志",

    // Sidebar status
    "status.running": "运行中",
    "status.degraded": "部分运行",
    "status.stopped": "已停止",

    // Dashboard
    "dash.title": "仪表盘",
    "dash.startProxy": "启动代理",
    "dash.stopProxy": "停止代理",
    "dash.totalKeys": "Key 总数",
    "dash.active": "活跃",
    "dash.coolingDown": "冷却中",
    "dash.uptime": "运行时长",
    "dash.recentActivity": "最近活动",
    "dash.seeAllLogs": "查看全部日志 →",
    "dash.noLogs": "暂无日志",
    "dash.mcpConfig": "MCP 配置",
    "dash.copy": "复制",
    "dash.proxyStarted": "代理已启动",
    "dash.proxyStopped": "代理已停止",
    "dash.startFailed": "启动失败: ",
    "dash.stopFailed": "停止失败: ",

    // Config
    "cfg.title": "配置",
    "cfg.proxySettings": "代理设置",
    "cfg.proxyToken": "代理 Token",
    "cfg.firecrawlUpstreamUrl": "Firecrawl 上游 Base URL",
    "cfg.tavilyUpstreamUrl": "Tavily 上游 Base URL",
    "cfg.networkSettings": "网络设置",
    "cfg.systemSettings": "系统设置",
    "cfg.host": "主机",
    "cfg.firecrawlPort": "Firecrawl 端口",
    "cfg.tavilyPort": "Tavily 端口",
    "cfg.requestTimeout": "请求超时",
    "cfg.keyCooldown": "Key 冷却时间",
    "cfg.launchOnLogin": "开机自启",
    "cfg.launchOnLoginHint": "系统登录后自动启动本应用",
    "cfg.apiKeysSection": "API Keys",
    "cfg.firecrawlApiKeysHint": "Firecrawl Keys：每行一个，或逗号分隔",
    "cfg.tavilyApiKeysHint": "Tavily Keys：每行一个，或逗号分隔",
    "cfg.save": "保存配置",
    "cfg.unsaved": "有未保存的更改",
    "cfg.saved": "配置已保存",
    "cfg.saveFailed": "保存失败: ",
    "cfg.loadFailed": "加载配置失败: ",
    "cfg.launchOnLoginFailed": "设置开机自启失败: ",

    // Keys
    "keys.title": "API Keys",
    "keys.desc": "监控 Firecrawl 与 Tavily API Keys 的健康状态。",
    "keys.firecrawl": "Firecrawl Keys",
    "keys.tavily": "Tavily Keys",
    "keys.notConfigured": "未配置",
    "keys.active": "活跃",
    "keys.cooldown": "冷却中",
    "keys.idle": "空闲",
    "keys.failures": "次失败",
    "keys.editNote": "在 <a id=\"keysGoConfig\">配置页面</a> 编辑 Keys。",
    "keys.loadFailed": "加载 Keys 失败",

    // MCP
    "mcp.title": "MCP 配置",
    "mcp.desc": "选择要生成的配置并复制到 MCP 客户端配置文件。",
    "mcp.scopeLabel": "配置范围",
    "mcp.scopeBoth": "Firecrawl + Tavily",
    "mcp.scopeFirecrawl": "仅 Firecrawl",
    "mcp.scopeTavily": "仅 Tavily",
    "mcp.unavailable": "未配置",
    "mcp.copyJson": "复制当前 JSON",
    "mcp.instructions": "使用说明",
    "mcp.step1": "在 <a id=\"mcpGoDash\">仪表盘</a> 启动代理",
    "mcp.step2": "从下拉框选择配置范围并复制 JSON",
    "mcp.step3": "粘贴到 MCP 客户端配置文件（Claude Desktop、Cursor 等）",
    "mcp.step4": "重启 MCP 客户端使配置生效",
    "mcp.note": "<code>FIRECRAWL_API_URL</code>/<code>FIRECRAWL_API_KEY</code> 与 <code>TAVILY_API_URL</code>/<code>TAVILY_API_KEY</code> 都指向本地代理与代理 Token。",
    "mcp.loadFailed": "加载 MCP 配置失败: ",

    // Logs
    "logs.title": "日志",
    "logs.all": "全部",
    "logs.search": "搜索日志...",
    "logs.refresh": "刷新",
    "logs.showing": "显示 {0} / {1} 条",
    "logs.autoRefresh": "自动刷新: 2s",

    // Common
    "copied": "已复制到剪贴板",
    "copyFailed": "复制失败: ",
  },
  en: {
    "nav.dashboard": "Dashboard",
    "nav.config": "Configuration",
    "nav.keys": "API Keys",
    "nav.mcp": "MCP Config",
    "nav.logs": "Logs",

    "status.running": "Running",
    "status.degraded": "Degraded",
    "status.stopped": "Stopped",

    "dash.title": "Dashboard",
    "dash.startProxy": "Start Proxy",
    "dash.stopProxy": "Stop Proxy",
    "dash.totalKeys": "Total Keys",
    "dash.active": "Active",
    "dash.coolingDown": "Cooling Down",
    "dash.uptime": "Uptime",
    "dash.recentActivity": "Recent Activity",
    "dash.seeAllLogs": "See all logs →",
    "dash.noLogs": "No logs yet.",
    "dash.mcpConfig": "MCP Config",
    "dash.copy": "Copy",
    "dash.proxyStarted": "Proxy started",
    "dash.proxyStopped": "Proxy stopped",
    "dash.startFailed": "Start failed: ",
    "dash.stopFailed": "Stop failed: ",

    "cfg.title": "Configuration",
    "cfg.proxySettings": "Proxy Settings",
    "cfg.proxyToken": "Proxy Token",
    "cfg.firecrawlUpstreamUrl": "Firecrawl Upstream Base URL",
    "cfg.tavilyUpstreamUrl": "Tavily Upstream Base URL",
    "cfg.networkSettings": "Network Settings",
    "cfg.systemSettings": "System Settings",
    "cfg.host": "Host",
    "cfg.firecrawlPort": "Firecrawl Port",
    "cfg.tavilyPort": "Tavily Port",
    "cfg.requestTimeout": "Request Timeout",
    "cfg.keyCooldown": "Key Cooldown",
    "cfg.launchOnLogin": "Launch on login",
    "cfg.launchOnLoginHint": "Automatically start this app after user login",
    "cfg.apiKeysSection": "API Keys",
    "cfg.firecrawlApiKeysHint": "Firecrawl keys: one per line, or comma-separated",
    "cfg.tavilyApiKeysHint": "Tavily keys: one per line, or comma-separated",
    "cfg.save": "Save Configuration",
    "cfg.unsaved": "Unsaved changes",
    "cfg.saved": "Configuration saved",
    "cfg.saveFailed": "Save failed: ",
    "cfg.loadFailed": "Failed to load config: ",
    "cfg.launchOnLoginFailed": "Failed to update launch-on-login: ",

    "keys.title": "API Keys",
    "keys.desc": "Monitor the health and status of Firecrawl and Tavily API keys.",
    "keys.firecrawl": "Firecrawl Keys",
    "keys.tavily": "Tavily Keys",
    "keys.notConfigured": "Not configured",
    "keys.active": "Active",
    "keys.cooldown": "Cooldown",
    "keys.idle": "Idle",
    "keys.failures": " failures",
    "keys.editNote": "Edit keys on the <a id=\"keysGoConfig\">Configuration page</a>.",
    "keys.loadFailed": "Failed to load keys.",

    "mcp.title": "MCP Configuration",
    "mcp.desc": "Choose a scope and copy the generated JSON to your MCP client settings file.",
    "mcp.scopeLabel": "Scope",
    "mcp.scopeBoth": "Firecrawl + Tavily",
    "mcp.scopeFirecrawl": "Firecrawl only",
    "mcp.scopeTavily": "Tavily only",
    "mcp.unavailable": "Unavailable",
    "mcp.copyJson": "Copy current JSON",
    "mcp.instructions": "Instructions",
    "mcp.step1": "Start the proxy on the <a id=\"mcpGoDash\">Dashboard</a>",
    "mcp.step2": "Select a scope from the dropdown and copy the JSON",
    "mcp.step3": "Paste it into your MCP client config file (Claude Desktop, Cursor, etc.)",
    "mcp.step4": "Restart your MCP client to apply changes",
    "mcp.note": "<code>FIRECRAWL_API_URL</code>/<code>FIRECRAWL_API_KEY</code> and <code>TAVILY_API_URL</code>/<code>TAVILY_API_KEY</code> both point to your local proxies and proxy token.",
    "mcp.loadFailed": "Failed to load MCP config: ",

    "logs.title": "Logs",
    "logs.all": "All",
    "logs.search": "Search logs...",
    "logs.refresh": "Refresh",
    "logs.showing": "Showing {0} of {1} lines",
    "logs.autoRefresh": "Auto-refresh: 2s",

    "copied": "Copied to clipboard",
    "copyFailed": "Copy failed: ",
  },
};

let currentLang = localStorage.getItem("lang") || "zh";

function t(key, ...args) {
  let text = (I18N[currentLang] && I18N[currentLang][key]) || (I18N.en[key]) || key;
  args.forEach((arg, i) => {
    text = text.replace(`{${i}}`, arg);
  });
  return text;
}

function setLang(lang) {
  currentLang = lang;
  localStorage.setItem("lang", lang);

  // Update toggle buttons
  langToggle.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === lang);
  });

  // Update sidebar nav labels
  const navLabels = { dashboard: "nav.dashboard", config: "nav.config", keys: "nav.keys", mcp: "nav.mcp", logs: "nav.logs" };
  sidebarNav.querySelectorAll(".nav-item").forEach((btn) => {
    const key = navLabels[btn.dataset.page];
    if (key) btn.querySelector("span").textContent = t(key);
  });

  // Update sidebar status text
  updateSidebarStatus();

  // Re-render current page
  if (currentPageId) navigate(currentPageId);
}

langToggle.addEventListener("click", (e) => {
  const btn = e.target.closest(".lang-btn");
  if (!btn || btn.dataset.lang === currentLang) return;
  setLang(btn.dataset.lang);
});

// ---- Toast ----
function showToast(message, type = "info") {
  const el = document.createElement("div");
  el.className = `toast toast-${type}`;
  el.textContent = message;
  toastContainer.appendChild(el);
  setTimeout(() => {
    el.classList.add("toast-exit");
    el.addEventListener("animationend", () => el.remove());
  }, 2500);
}

// ---- Clipboard ----
function copyTextWithExecCommand(text) {
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.top = "-9999px";
  area.style.left = "-9999px";
  document.body.appendChild(area);
  area.focus();
  area.select();
  const copied = !!document.execCommand && document.execCommand("copy");
  area.remove();
  if (!copied) throw new Error("Copy command failed");
}

async function copyText(text) {
  const errors = [];

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (e) {
      // Some WebView contexts deny Clipboard API even on click; fallback below.
      errors.push(`navigator.clipboard: ${e}`);
    }
  }

  const tauriClipboardWrite =
    window.__TAURI__?.clipboardManager?.writeText ||
    window.__TAURI__?.clipboard?.writeText;
  if (typeof tauriClipboardWrite === "function") {
    try {
      await tauriClipboardWrite(text);
      return;
    } catch (e) {
      errors.push(`window.__TAURI__.clipboardManager: ${e}`);
    }
  }

  if (typeof invoke === "function") {
    try {
      const currentLabel = window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label;
      await invoke("plugin:clipboard-manager|write_text", {
        text,
        label: typeof currentLabel === "string" ? currentLabel : undefined,
      });
      return;
    } catch (e) {
      errors.push(`invoke(plugin:clipboard-manager|write_text): ${e}`);
    }
  }

  try {
    copyTextWithExecCommand(text);
    return;
  } catch (e) {
    errors.push(`execCommand: ${e}`);
  }

  throw new Error(errors.join(" | "));
}

async function copyWithFeedback(text, btnEl) {
  try {
    await copyText(text);
    const orig = btnEl.innerHTML;
    btnEl.innerHTML = "Copied!";
    btnEl.disabled = true;
    setTimeout(() => {
      btnEl.innerHTML = orig;
      btnEl.disabled = false;
    }, 1500);
    showToast(t("copied"), "success");
  } catch (e) {
    showToast(t("copyFailed") + e, "error");
  }
}

// ---- Button loading ----
function setLoading(btn, loading) {
  if (loading) {
    btn._origHTML = btn.innerHTML;
    btn.classList.add("btn-loading");
    btn.innerHTML = `<span class="btn-text">${btn._origHTML}</span>`;
    btn.disabled = true;
  } else {
    btn.classList.remove("btn-loading");
    btn.innerHTML = btn._origHTML || btn.innerHTML;
    btn.disabled = false;
  }
}

// ---- Key helpers ----
function truncateKey(key) {
  if (!key || key.length <= 14) return key || "";
  return key.slice(0, 8) + "..." + key.slice(-5);
}

function parseKeys(text) {
  return text.split(/[\n,]/g).map((v) => v.trim()).filter(Boolean);
}

function normalizeKeysText(arr) {
  return (arr || []).join("\n");
}

function idleStatusesFromKeys(keys) {
  return (keys || []).map((key, index) => ({
    index,
    keyPreview: truncateKey(key),
    isCoolingDown: false,
    cooldownRemainingSecs: 0,
    failCount: 0,
  }));
}

function isProviderConfigured(config, provider) {
  if (provider === "firecrawl") {
    return !!((config?.firecrawlApiKeys || []).length && (config?.upstreamBaseUrl || "").trim());
  }
  if (provider === "tavily") {
    return !!((config?.tavilyApiKeys || []).length && (config?.tavilyUpstreamBaseUrl || "").trim());
  }
  return false;
}

function buildFallbackKeySnapshot(config, status) {
  const firecrawlConfigured = isProviderConfigured(config, "firecrawl");
  const tavilyConfigured = isProviderConfigured(config, "tavily");

  return {
    firecrawl: {
      configured: firecrawlConfigured,
      running: !!status?.firecrawlRunning,
      keys: firecrawlConfigured ? idleStatusesFromKeys(config?.firecrawlApiKeys) : [],
    },
    tavily: {
      configured: tavilyConfigured,
      running: !!status?.tavilyRunning,
      keys: tavilyConfigured ? idleStatusesFromKeys(config?.tavilyApiKeys) : [],
    },
  };
}

function getStatusLabelKey(status) {
  if (!status) return "status.stopped";
  if (status.running) return "status.running";
  if (isAnyProxyRunning(status)) return "status.degraded";
  return "status.stopped";
}

function mergeConfiguredKeys(snapshot) {
  const merged = [];
  if (snapshot?.firecrawl?.configured) merged.push(...(snapshot.firecrawl.keys || []));
  if (snapshot?.tavily?.configured) merged.push(...(snapshot.tavily.keys || []));
  return merged;
}

// ---- JSON syntax highlight ----
function highlightJSON(json) {
  return json
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"([^"]+)"(\s*:)/g, '<span class="json-key">"$1"</span>$2')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/[{}\[\]]/g, '<span class="json-bracket">$&</span>');
}

// ============================================
// 3. ROUTER
// ============================================
let currentPage = null;
let currentPageId = null;
let globalTimer = null;

const pages = {};

function navigate(pageId) {
  if (currentPage && currentPage.destroy) currentPage.destroy();
  currentPageId = pageId;
  currentPage = pages[pageId];

  // Update sidebar active state
  sidebarNav.querySelectorAll(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.page === pageId);
  });

  contentEl.innerHTML = currentPage.template();
  contentEl.classList.remove("page-enter");
  void contentEl.offsetWidth; // force reflow
  contentEl.classList.add("page-enter");

  if (currentPage.init) currentPage.init();
}

sidebarNav.addEventListener("click", (e) => {
  const btn = e.target.closest(".nav-item");
  if (!btn || btn.dataset.page === currentPageId) return;
  navigate(btn.dataset.page);
});

// ---- Global status polling ----
function isAnyProxyRunning(status) {
  return !!(status && (status.anyRunning || status.running || status.firecrawlRunning || status.tavilyRunning));
}

function formatProxyUrls(status) {
  const urls = [];
  if (status?.listenUrl) urls.push(`FC ${status.listenUrl}`);
  if (status?.tavilyListenUrl) urls.push(`TV ${status.tavilyListenUrl}`);
  return urls.join(" | ") || "-";
}

async function updateSidebarStatus() {
  try {
    const status = await invoke("get_proxy_status");
    if (status?.running) {
      sidebarBadge.className = "badge badge-success";
      sidebarBadge.textContent = t("status.running");
      sidebarUrl.textContent = formatProxyUrls(status);
    } else if (isAnyProxyRunning(status)) {
      sidebarBadge.className = "badge badge-warning";
      sidebarBadge.textContent = t("status.degraded");
      sidebarUrl.textContent = formatProxyUrls(status);
    } else {
      sidebarBadge.className = "badge badge-danger";
      sidebarBadge.textContent = t("status.stopped");
      sidebarUrl.textContent = "-";
    }
    return status;
  } catch {
    return { running: false };
  }
}

// ============================================
// 4. PAGE: Dashboard
// ============================================
pages.dashboard = {
  _timer: null,
  _proxyStartTime: null,
  _uptimeTimer: null,

  template() {
    return `
      <h1>${t("dash.title")}</h1>
      <div class="dash-grid">
        <div class="card status-card">
          <div id="dashDot" class="status-dot"></div>
          <div id="dashStatusText" class="status-text">${t("status.stopped")}</div>
          <div id="dashUrl" class="status-url">-</div>
          <div class="status-actions">
            <button id="dashStartBtn" class="btn btn-primary">${t("dash.startProxy")}</button>
            <button id="dashStopBtn" class="btn btn-danger" disabled>${t("dash.stopProxy")}</button>
          </div>
        </div>
        <div class="card stats-card">
          <div class="stat-item">
            <span id="statTotalKeys" class="stat-value">-</span>
            <span class="stat-label">${t("dash.totalKeys")}</span>
          </div>
          <div class="stat-item">
            <span id="statActive" class="stat-value">-</span>
            <span class="stat-label">${t("dash.active")}</span>
          </div>
          <div class="stat-item">
            <span id="statCooldown" class="stat-value">-</span>
            <span class="stat-label">${t("dash.coolingDown")}</span>
          </div>
          <div class="stat-item">
            <span id="statUptime" class="stat-value">-</span>
            <span class="stat-label">${t("dash.uptime")}</span>
          </div>
        </div>
      </div>

      <div class="card mcp-snippet">
        <div class="flex-between">
          <h2 class="mb-0">${t("dash.mcpConfig")}</h2>
          <button id="dashCopyMcp" class="btn btn-sm btn-primary">${t("dash.copy")}</button>
        </div>
        <div id="dashMcp" class="code-block"></div>
      </div>

      <div class="card recent-card mt-4">
        <div class="card-head-row">
          <h2>${t("dash.recentActivity")}</h2>
          <button id="dashSeeAll" class="link-btn">${t("dash.seeAllLogs")}</button>
        </div>
        <div id="dashLogs" class="recent-logs"></div>
      </div>
    `;
  },

  async init() {
    const startBtn = document.getElementById("dashStartBtn");
    const stopBtn = document.getElementById("dashStopBtn");
    const seeAllBtn = document.getElementById("dashSeeAll");
    const copyBtn = document.getElementById("dashCopyMcp");

    startBtn.addEventListener("click", async () => {
      setLoading(startBtn, true);
      try {
        const config = await invoke("load_proxy_config");
        await invoke("save_proxy_config", { config });
        await invoke("start_proxy");
        this._proxyStartTime = Date.now();
        showToast(t("dash.proxyStarted"), "success");
      } catch (e) {
        showToast(t("dash.startFailed") + e, "error");
      }
      setLoading(startBtn, false);
      this._refresh();
    });

    stopBtn.addEventListener("click", async () => {
      setLoading(stopBtn, true);
      try {
        await invoke("stop_proxy");
        this._proxyStartTime = null;
        showToast(t("dash.proxyStopped"), "success");
      } catch (e) {
        showToast(t("dash.stopFailed") + e, "error");
      }
      setLoading(stopBtn, false);
      this._refresh();
    });

    seeAllBtn.addEventListener("click", () => navigate("logs"));

    copyBtn.addEventListener("click", async () => {
      const text = await invoke("build_mcp_config", { target: "both" });
      copyWithFeedback(text, copyBtn);
    });

    await this._refresh();
    this._timer = setInterval(() => this._refresh(), 2000);
    this._uptimeTimer = setInterval(() => this._updateUptime(), 1000);
  },

  async _refresh() {
    try {
      const [status, config, logs, mcpText, keySnapshotRaw] = await Promise.all([
        invoke("get_proxy_status"),
        invoke("load_proxy_config"),
        invoke("get_recent_logs"),
        invoke("build_mcp_config", { target: "both" }).catch((e) => String(e || "")),
        invoke("get_key_status_snapshot").catch(() => null),
      ]);
      const keySnapshot = keySnapshotRaw || buildFallbackKeySnapshot(config, status);

      const dot = document.getElementById("dashDot");
      const text = document.getElementById("dashStatusText");
      const url = document.getElementById("dashUrl");
      const startBtn = document.getElementById("dashStartBtn");
      const stopBtn = document.getElementById("dashStopBtn");

      const fullyRunning = !!status?.running;
      const anyRunning = isAnyProxyRunning(status);

      if (dot) {
        if (anyRunning) {
          dot.className = "status-dot running";
          text.textContent = t(getStatusLabelKey(status));
          url.textContent = formatProxyUrls(status);
          startBtn.disabled = fullyRunning;
          stopBtn.disabled = false;
          if (!this._proxyStartTime) this._proxyStartTime = Date.now();
        } else {
          dot.className = "status-dot";
          text.textContent = t("status.stopped");
          url.textContent = "-";
          startBtn.disabled = false;
          stopBtn.disabled = true;
          this._proxyStartTime = null;
        }
      }

      const mergedKeyStatuses = mergeConfiguredKeys(keySnapshot);
      const totalKeys = mergedKeyStatuses.length;
      const activeCount = mergedKeyStatuses.filter((k) => !k.isCoolingDown).length;
      const cooldownCount = mergedKeyStatuses.filter((k) => k.isCoolingDown).length;

      const totalEl = document.getElementById("statTotalKeys");
      const activeEl = document.getElementById("statActive");
      const cooldownEl = document.getElementById("statCooldown");
      if (totalEl) totalEl.textContent = String(totalKeys);
      if (activeEl) activeEl.textContent = String(activeCount);
      if (cooldownEl) cooldownEl.textContent = String(cooldownCount);

      this._updateUptime();

      const logsEl = document.getElementById("dashLogs");
      if (logsEl) {
        const recent = logs.slice(-10);
        logsEl.textContent = recent.join("\n") || t("dash.noLogs");
        logsEl.scrollTop = logsEl.scrollHeight;
      }

      const mcpEl = document.getElementById("dashMcp");
      if (mcpEl) mcpEl.innerHTML = highlightJSON(mcpText);
    } catch {
      // silently ignore
    }
  },

  _updateUptime() {
    const el = document.getElementById("statUptime");
    if (!el) return;
    if (!this._proxyStartTime) {
      el.textContent = "-";
      return;
    }
    const sec = Math.floor((Date.now() - this._proxyStartTime) / 1000);
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = sec % 60;
    el.textContent = h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s}s` : `${s}s`;
  },

  destroy() {
    if (this._timer) { clearInterval(this._timer); this._timer = null; }
    if (this._uptimeTimer) { clearInterval(this._uptimeTimer); this._uptimeTimer = null; }
  },
};

// ============================================
// 5. PAGE: Configuration
// ============================================
pages.config = {
  _savedConfig: null,
  _savedLaunchOnLogin: false,

  template() {
    return `
      <h1>${t("cfg.title")}</h1>

      <div class="card">
        <div class="card-header">${t("cfg.proxySettings")}</div>
        <div class="form-group">
          <label class="form-label">${t("cfg.proxyToken")} <span class="form-hint">PROXY_TOKEN</span></label>
          <input id="cfgProxyToken" class="form-input" type="text" placeholder="your-local-token" />
        </div>
        <div class="form-group">
          <label class="form-label">${t("cfg.firecrawlUpstreamUrl")} <span class="form-hint">UPSTREAM_BASE_URL</span></label>
          <input id="cfgUpstreamUrl" class="form-input" type="url" placeholder="https://api.firecrawl.dev" />
        </div>
        <div class="form-group">
          <label class="form-label">${t("cfg.tavilyUpstreamUrl")} <span class="form-hint">TAVILY_UPSTREAM_BASE_URL</span></label>
          <input id="cfgTavilyUpstreamUrl" class="form-input" type="url" placeholder="https://api.tavily.com" />
        </div>
      </div>

      <div class="card">
        <div class="card-header">${t("cfg.networkSettings")}</div>
        <div class="form-row">
          <div class="form-group">
            <label class="form-label">${t("cfg.host")} <span class="form-hint">HOST</span></label>
            <input id="cfgHost" class="form-input" type="text" />
          </div>
          <div class="form-group">
            <label class="form-label">${t("cfg.firecrawlPort")} <span class="form-hint">PORT</span></label>
            <input id="cfgPort" class="form-input" type="number" min="1" max="65535" />
          </div>
        </div>
        <div class="form-group">
          <label class="form-label">${t("cfg.tavilyPort")} <span class="form-hint">TAVILY_PORT</span></label>
          <input id="cfgTavilyPort" class="form-input" type="number" min="1" max="65535" />
        </div>
        <div class="form-row">
          <div class="form-group">
            <label class="form-label">${t("cfg.requestTimeout")} <span class="form-hint">REQUEST_TIMEOUT_MS</span></label>
            <div class="input-with-suffix">
              <input id="cfgTimeout" class="form-input" type="number" min="1" />
              <span class="input-suffix">ms</span>
            </div>
          </div>
          <div class="form-group">
            <label class="form-label">${t("cfg.keyCooldown")} <span class="form-hint">KEY_COOLDOWN_SECONDS</span></label>
            <div class="input-with-suffix">
              <input id="cfgCooldown" class="form-input" type="number" min="1" />
              <span class="input-suffix">sec</span>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-header">${t("cfg.apiKeysSection")}</div>
        <div class="form-group">
          <label class="form-label">${t("cfg.firecrawlApiKeysHint")}</label>
          <textarea id="cfgApiKeys" class="form-textarea" rows="5" placeholder="fc-key-1&#10;fc-key-2&#10;fc-key-3"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">${t("cfg.tavilyApiKeysHint")}</label>
          <textarea id="cfgTavilyApiKeys" class="form-textarea" rows="5" placeholder="tvly-key-1&#10;tvly-key-2&#10;tvly-key-3"></textarea>
        </div>
      </div>

      <div class="card">
        <div class="card-header">${t("cfg.systemSettings")}</div>
        <label class="toggle-row" for="cfgLaunchOnLogin">
          <span>${t("cfg.launchOnLogin")}</span>
          <input id="cfgLaunchOnLogin" type="checkbox" />
        </label>
        <p class="form-note">${t("cfg.launchOnLoginHint")}</p>
      </div>

      <div class="config-actions">
        <button id="cfgSaveBtn" class="btn btn-primary">${t("cfg.save")}</button>
        <span id="cfgDirtyBadge" class="dirty-badge">${t("cfg.unsaved")}</span>
      </div>
    `;
  },

  async init() {
    try {
      const [config, launchOnLogin] = await Promise.all([
        invoke("load_proxy_config"),
        invoke("get_launch_on_login_enabled").catch(() => false),
      ]);
      this._savedConfig = config;
      this._savedLaunchOnLogin = !!launchOnLogin;
      this._writeForm({ ...config, launchOnLogin: this._savedLaunchOnLogin });
    } catch (e) {
      showToast(t("cfg.loadFailed") + e, "error");
    }

    const inputs = [
      "cfgProxyToken",
      "cfgUpstreamUrl",
      "cfgTavilyUpstreamUrl",
      "cfgHost",
      "cfgPort",
      "cfgTavilyPort",
      "cfgTimeout",
      "cfgCooldown",
      "cfgApiKeys",
      "cfgTavilyApiKeys",
    ];
    inputs.forEach((id) => {
      const el = document.getElementById(id);
      if (el) el.addEventListener("input", () => this._checkDirty());
    });
    const launchOnLoginEl = document.getElementById("cfgLaunchOnLogin");
    if (launchOnLoginEl) launchOnLoginEl.addEventListener("change", () => this._checkDirty());

    document.getElementById("cfgSaveBtn").addEventListener("click", async () => {
      const btn = document.getElementById("cfgSaveBtn");
      setLoading(btn, true);
      try {
        const form = this._readForm();
        const { launchOnLogin, ...config } = form;
        await invoke("save_proxy_config", { config });
        try {
          const actual = await invoke("set_launch_on_login_enabled", { enabled: launchOnLogin });
          this._savedLaunchOnLogin = !!actual;
        } catch (e) {
          showToast(t("cfg.launchOnLoginFailed") + e, "error");
        }
        this._savedConfig = config;
        this._checkDirty();
        showToast(t("cfg.saved"), "success");
      } catch (e) {
        showToast(t("cfg.saveFailed") + e, "error");
      }
      setLoading(btn, false);
    });
  },

  _readForm() {
    return {
      proxyToken: document.getElementById("cfgProxyToken").value.trim(),
      firecrawlApiKeys: parseKeys(document.getElementById("cfgApiKeys").value),
      upstreamBaseUrl: document.getElementById("cfgUpstreamUrl").value.trim(),
      tavilyApiKeys: parseKeys(document.getElementById("cfgTavilyApiKeys").value),
      tavilyUpstreamBaseUrl: document.getElementById("cfgTavilyUpstreamUrl").value.trim(),
      requestTimeoutMs: Number(document.getElementById("cfgTimeout").value),
      keyCooldownSeconds: Number(document.getElementById("cfgCooldown").value),
      host: document.getElementById("cfgHost").value.trim(),
      port: Number(document.getElementById("cfgPort").value),
      tavilyPort: Number(document.getElementById("cfgTavilyPort").value),
      launchOnLogin: !!document.getElementById("cfgLaunchOnLogin").checked,
    };
  },

  _writeForm(c) {
    document.getElementById("cfgProxyToken").value = c.proxyToken || "";
    document.getElementById("cfgUpstreamUrl").value = c.upstreamBaseUrl || "";
    document.getElementById("cfgTavilyUpstreamUrl").value = c.tavilyUpstreamBaseUrl || "";
    document.getElementById("cfgHost").value = c.host || "127.0.0.1";
    document.getElementById("cfgPort").value = String(c.port || 8787);
    document.getElementById("cfgTavilyPort").value = String(c.tavilyPort || 8788);
    document.getElementById("cfgTimeout").value = String(c.requestTimeoutMs || 60000);
    document.getElementById("cfgCooldown").value = String(c.keyCooldownSeconds || 60);
    document.getElementById("cfgApiKeys").value = normalizeKeysText(c.firecrawlApiKeys);
    document.getElementById("cfgTavilyApiKeys").value = normalizeKeysText(c.tavilyApiKeys);
    document.getElementById("cfgLaunchOnLogin").checked = !!c.launchOnLogin;
  },

  _checkDirty() {
    const badge = document.getElementById("cfgDirtyBadge");
    if (!badge || !this._savedConfig) return;
    const cur = this._readForm();
    const saved = this._savedConfig;
    const dirty =
      cur.proxyToken !== (saved.proxyToken || "") ||
      cur.upstreamBaseUrl !== (saved.upstreamBaseUrl || "") ||
      cur.tavilyUpstreamBaseUrl !== (saved.tavilyUpstreamBaseUrl || "") ||
      cur.host !== (saved.host || "127.0.0.1") ||
      cur.port !== (saved.port || 8787) ||
      cur.tavilyPort !== (saved.tavilyPort || 8788) ||
      cur.requestTimeoutMs !== (saved.requestTimeoutMs || 60000) ||
      cur.keyCooldownSeconds !== (saved.keyCooldownSeconds || 60) ||
      JSON.stringify(cur.firecrawlApiKeys) !== JSON.stringify(saved.firecrawlApiKeys || []) ||
      JSON.stringify(cur.tavilyApiKeys) !== JSON.stringify(saved.tavilyApiKeys || []) ||
      cur.launchOnLogin !== this._savedLaunchOnLogin;
    badge.classList.toggle("visible", dirty);
  },

  destroy() {},
};

// ============================================
// 6. PAGE: API Keys
// ============================================
pages.keys = {
  _timer: null,

  template() {
    return `
      <h1>${t("keys.title")}</h1>
      <p style="font-size:13px;color:var(--text-secondary);margin-bottom:16px">
        ${t("keys.desc")}
      </p>
      <div id="keyListContainer" class="card">
        <div class="card-header">${t("keys.firecrawl")}</div>
        <div class="key-list" id="keyListFirecrawl"></div>
        <div class="card-header" style="margin-top:16px">${t("keys.tavily")}</div>
        <div class="key-list" id="keyListTavily"></div>
      </div>
      <div class="keys-legend">
        <span class="keys-legend-item"><span class="legend-dot green"></span> ${t("keys.active")}</span>
        <span class="keys-legend-item"><span class="legend-dot amber"></span> ${t("keys.cooldown")}</span>
        <span class="keys-legend-item"><span class="legend-dot gray"></span> ${t("keys.idle")}</span>
      </div>
      <p class="keys-note">${t("keys.editNote")}</p>
    `;
  },

  async init() {
    document.getElementById("keysGoConfig").addEventListener("click", () => navigate("config"));
    await this._refresh();
    this._timer = setInterval(() => this._refresh(), 2000);
  },

  _renderProviderRows(listEl, providerSnapshot) {
    if (!listEl) return;

    if (!providerSnapshot?.configured && !providerSnapshot?.running) {
      listEl.innerHTML = `<div style="padding:12px;color:var(--text-muted)">${t("keys.notConfigured")}</div>`;
      return;
    }

    const statuses = providerSnapshot.keys || [];
    if (!statuses.length) {
      listEl.innerHTML = `<div style="padding:12px;color:var(--text-muted)">${t("keys.notConfigured")}</div>`;
      return;
    }

    const providerRunning = !!providerSnapshot.running;
    listEl.innerHTML = statuses.map((k) => {
      let badgeClass = "badge-muted";
      let badgeText = t("keys.idle");
      if (k.isCoolingDown) {
        badgeClass = "badge-warning";
        badgeText = t("keys.cooldown");
      } else if (providerRunning) {
        badgeClass = "badge-success";
        badgeText = t("keys.active");
      }

      const cooldownHtml = k.cooldownRemainingSecs > 0
        ? `<span class="key-cooldown-timer">${k.cooldownRemainingSecs}s</span>`
        : "";

      return `
        <div class="key-row">
          <span class="key-preview" title="${k.keyPreview}">${k.keyPreview}</span>
          <span class="badge ${badgeClass}">${badgeText}</span>
          <span class="key-fail-count">${k.failCount > 0 ? k.failCount + t("keys.failures") : cooldownHtml}</span>
        </div>
      `;
    }).join("");
  },

  async _refresh() {
    const firecrawlListEl = document.getElementById("keyListFirecrawl");
    const tavilyListEl = document.getElementById("keyListTavily");
    if (!firecrawlListEl || !tavilyListEl) return;

    try {
      const [config, status, snapshotRaw] = await Promise.all([
        invoke("load_proxy_config"),
        invoke("get_proxy_status").catch(() => ({})),
        invoke("get_key_status_snapshot").catch(() => null),
      ]);
      const snapshot = snapshotRaw || buildFallbackKeySnapshot(config, status);
      this._renderProviderRows(firecrawlListEl, snapshot.firecrawl);
      this._renderProviderRows(tavilyListEl, snapshot.tavily);
    } catch {
      firecrawlListEl.innerHTML = `<div style="padding:12px;color:var(--text-muted)">${t("keys.loadFailed")}</div>`;
      tavilyListEl.innerHTML = `<div style="padding:12px;color:var(--text-muted)">${t("keys.loadFailed")}</div>`;
    }
  },

  destroy() {
    if (this._timer) { clearInterval(this._timer); this._timer = null; }
  },
};

// ============================================
// 7. PAGE: MCP Config
// ============================================
pages.mcp = {
  _cleanup: null,

  template() {
    return `
      <h1>${t("mcp.title")}</h1>
      <p class="mcp-page-desc">${t("mcp.desc")}</p>

      <div class="card">
        <div class="mcp-code-wrapper">
          <div class="mcp-toolbar">
            <label class="form-label mcp-scope-label" for="mcpTargetBtn">${t("mcp.scopeLabel")}</label>
            <div id="mcpTargetSelect" class="mcp-select">
              <button id="mcpTargetBtn" class="mcp-select-trigger" type="button" aria-haspopup="listbox" aria-expanded="false">
                <span id="mcpTargetText">${t("mcp.scopeBoth")}</span>
                <span class="mcp-select-caret">▾</span>
              </button>
              <div id="mcpTargetMenu" class="mcp-select-menu" role="listbox"></div>
            </div>
          </div>
          <div id="mcpCodeBlock" class="code-block"></div>
          <button id="mcpCopyBtn" class="btn btn-sm btn-primary code-copy-btn">${t("mcp.copyJson")}</button>
        </div>
      </div>

      <div class="card mcp-instructions">
        <div class="card-header">${t("mcp.instructions")}</div>
        <ol>
          <li>${t("mcp.step1")}</li>
          <li>${t("mcp.step2")}</li>
          <li>${t("mcp.step3")}</li>
          <li>${t("mcp.step4")}</li>
        </ol>
        <p class="mcp-note">${t("mcp.note")}</p>
      </div>
    `;
  },

  async init() {
    try {
      const selectRoot = document.getElementById("mcpTargetSelect");
      const targetBtn = document.getElementById("mcpTargetBtn");
      const targetTextEl = document.getElementById("mcpTargetText");
      const menuEl = document.getElementById("mcpTargetMenu");
      const el = document.getElementById("mcpCodeBlock");
      const copyBtn = document.getElementById("mcpCopyBtn");
      const config = await invoke("load_proxy_config");
      let currentRaw = "";
      let currentTarget = "both";

      const firecrawlConfigured = isProviderConfigured(config, "firecrawl");
      const tavilyConfigured = isProviderConfigured(config, "tavily");
      const options = [
        { value: "both", labelKey: "mcp.scopeBoth", available: firecrawlConfigured || tavilyConfigured },
        { value: "firecrawl", labelKey: "mcp.scopeFirecrawl", available: firecrawlConfigured },
        { value: "tavily", labelKey: "mcp.scopeTavily", available: tavilyConfigured },
      ];

      const optionLabel = (option) => {
        if (!option) return "";
        return option.available
          ? t(option.labelKey)
          : `${t(option.labelKey)} (${t("mcp.unavailable")})`;
      };

      if (firecrawlConfigured && tavilyConfigured) {
        currentTarget = "both";
      } else if (firecrawlConfigured) {
        currentTarget = "firecrawl";
      } else if (tavilyConfigured) {
        currentTarget = "tavily";
      }

      const setOpen = (open) => {
        if (!selectRoot || !targetBtn) return;
        selectRoot.classList.toggle("open", open);
        targetBtn.setAttribute("aria-expanded", open ? "true" : "false");
      };

      const renderSelect = () => {
        const current = options.find((opt) => opt.value === currentTarget) || options[0];
        if (targetTextEl) targetTextEl.textContent = optionLabel(current);
        if (!menuEl) return;
        menuEl.innerHTML = options.map((opt) => `
          <button
            type="button"
            class="mcp-select-option${opt.value === currentTarget ? " is-selected" : ""}${!opt.available ? " is-disabled" : ""}"
            data-value="${opt.value}"
            role="option"
            ${opt.value === currentTarget ? "aria-selected=\"true\"" : ""}
            ${!opt.available ? "disabled" : ""}
          >
            ${optionLabel(opt)}
          </button>
        `).join("");
      };

      const refreshMcpCode = async () => {
        try {
          currentRaw = await invoke("build_mcp_config", { target: currentTarget });
          if (el) el.innerHTML = highlightJSON(currentRaw);
          copyBtn.disabled = false;
        } catch (e) {
          currentRaw = "";
          if (el) el.textContent = String(e || "");
          copyBtn.disabled = true;
        }
      };

      const onMenuClick = async (event) => {
        const optionBtn = event.target.closest(".mcp-select-option");
        if (!optionBtn || optionBtn.disabled) return;
        const nextTarget = optionBtn.dataset.value;
        if (!nextTarget || nextTarget === currentTarget) {
          setOpen(false);
          return;
        }
        currentTarget = nextTarget;
        renderSelect();
        setOpen(false);
        await refreshMcpCode();
      };

      const onTriggerClick = () => {
        const isOpen = !!selectRoot?.classList.contains("open");
        setOpen(!isOpen);
      };

      const onOutsideClick = (event) => {
        if (!selectRoot) return;
        if (!selectRoot.contains(event.target)) {
          setOpen(false);
        }
      };

      const onEscape = (event) => {
        if (event.key === "Escape") {
          setOpen(false);
        }
      };

      if (menuEl) menuEl.addEventListener("click", onMenuClick);
      if (targetBtn) targetBtn.addEventListener("click", onTriggerClick);
      document.addEventListener("mousedown", onOutsideClick);
      document.addEventListener("keydown", onEscape);

      this._cleanup = () => {
        if (menuEl) menuEl.removeEventListener("click", onMenuClick);
        if (targetBtn) targetBtn.removeEventListener("click", onTriggerClick);
        document.removeEventListener("mousedown", onOutsideClick);
        document.removeEventListener("keydown", onEscape);
      };

      renderSelect();
      await refreshMcpCode();

      copyBtn.addEventListener("click", async () => {
        if (!currentRaw) {
          await refreshMcpCode();
        }
        copyWithFeedback(currentRaw, copyBtn);
      });

      document.getElementById("mcpGoDash").addEventListener("click", () => navigate("dashboard"));
    } catch (e) {
      showToast(t("mcp.loadFailed") + e, "error");
    }
  },

  destroy() {
    if (this._cleanup) {
      this._cleanup();
      this._cleanup = null;
    }
  },
};

// ============================================
// 8. PAGE: Logs
// ============================================
pages.logs = {
  _timer: null,
  _allLogs: [],
  _activeFilter: "all",
  _searchText: "",
  _userScrolled: false,

  template() {
    return `
      <h1>${t("logs.title")}</h1>
      <div class="logs-toolbar">
        <div class="filter-group">
          <button class="filter-btn active" data-filter="all">${t("logs.all")}</button>
          <button class="filter-btn" data-filter="INFO">INFO</button>
          <button class="filter-btn" data-filter="WARN">WARN</button>
          <button class="filter-btn" data-filter="ERROR">ERROR</button>
        </div>
        <input id="logsSearch" class="logs-search" type="text" placeholder="${t("logs.search")}" />
        <button id="logsRefreshBtn" class="btn btn-sm">${t("logs.refresh")}</button>
      </div>
      <div id="logsViewer" class="logs-viewer"></div>
      <div class="logs-footer">
        <span id="logsCount">-</span>
        <span>${t("logs.autoRefresh")}</span>
      </div>
    `;
  },

  async init() {
    this._activeFilter = "all";
    this._searchText = "";
    this._userScrolled = false;

    const filterGroup = contentEl.querySelector(".filter-group");
    filterGroup.addEventListener("click", (e) => {
      const btn = e.target.closest(".filter-btn");
      if (!btn) return;
      filterGroup.querySelectorAll(".filter-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      this._activeFilter = btn.dataset.filter;
      this._renderLogs();
    });

    const searchEl = document.getElementById("logsSearch");
    let debounce = null;
    searchEl.addEventListener("input", () => {
      clearTimeout(debounce);
      debounce = setTimeout(() => {
        this._searchText = searchEl.value.toLowerCase();
        this._renderLogs();
      }, 200);
    });

    const viewer = document.getElementById("logsViewer");
    viewer.addEventListener("scroll", () => {
      this._userScrolled = (viewer.scrollHeight - viewer.scrollTop - viewer.clientHeight) > 50;
    });

    document.getElementById("logsRefreshBtn").addEventListener("click", () => this._fetchLogs());

    await this._fetchLogs();
    this._timer = setInterval(() => this._fetchLogs(), 2000);
  },

  async _fetchLogs() {
    try {
      this._allLogs = await invoke("get_recent_logs");
      this._renderLogs();
    } catch {}
  },

  _renderLogs() {
    const viewer = document.getElementById("logsViewer");
    const countEl = document.getElementById("logsCount");
    if (!viewer) return;

    let lines = this._allLogs;

    if (this._activeFilter !== "all") {
      lines = lines.filter((l) => l.includes(`[${this._activeFilter}]`));
    }

    if (this._searchText) {
      lines = lines.filter((l) => l.toLowerCase().includes(this._searchText));
    }

    viewer.innerHTML = lines.map((line) => {
      let cls = "";
      if (line.includes("[WARN]")) cls = "warn";
      else if (line.includes("[ERROR]")) cls = "error";

      const colored = line
        .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
        .replace(/^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?)/, '<span class="log-ts">$1</span>')
        .replace(/\[(INFO|WARN|ERROR)\]/, '<span class="log-level">[$1]</span>');

      return `<div class="log-line ${cls}">${colored}</div>`;
    }).join("");

    if (countEl) {
      countEl.textContent = t("logs.showing", lines.length, this._allLogs.length);
    }

    if (!this._userScrolled) {
      viewer.scrollTop = viewer.scrollHeight;
    }
  },

  destroy() {
    if (this._timer) { clearInterval(this._timer); this._timer = null; }
  },
};

// ============================================
// 9. BOOTSTRAP
// ============================================
async function bootstrap() {
  // Apply saved language on startup
  langToggle.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === currentLang);
  });

  // Set sidebar nav labels to current language
  const navLabels = { dashboard: "nav.dashboard", config: "nav.config", keys: "nav.keys", mcp: "nav.mcp", logs: "nav.logs" };
  sidebarNav.querySelectorAll(".nav-item").forEach((btn) => {
    const key = navLabels[btn.dataset.page];
    if (key) btn.querySelector("span").textContent = t(key);
  });

  try {
    await updateSidebarStatus();
  } catch {}

  navigate("dashboard");

  globalTimer = setInterval(() => {
    updateSidebarStatus();
  }, 3000);
}

window.addEventListener("beforeunload", () => {
  if (globalTimer) clearInterval(globalTimer);
});

bootstrap();
