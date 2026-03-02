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
const globalProxyLabel = document.getElementById("globalProxyLabel");
const globalProxyToggle = document.getElementById("globalProxyToggle");

// ============================================
// 2. I18N
// ============================================
const I18N = {
  zh: {
    // Sidebar nav
    "nav.dashboard": "仪表盘",
    "nav.config": "配置",
    "nav.mcp": "MCP 配置",
    "nav.logs": "日志",

    // Sidebar status
    "status.running": "运行中",
    "status.degraded": "部分运行",
    "status.stopped": "已停止",

    // Dashboard
    "dash.title": "仪表盘",
    "dash.totalKeys": "Key 总数",
    "dash.availableKeys": "可用 Key",
    "dash.coolingDown": "冷却中",
    "dash.disabledKeys": "已禁用",
    "dash.stoppedKeys": "已停止",
    "dash.unverifiedKeys": "待验证",
    "dash.keyOverview": "Key 状态",
    "dash.usage": "使用额度",
    "dash.usageFetchFailed": "额度更新失败",
    "dash.notConfigured": "未配置",
    "dash.notAvailable": "不可用",
    "dash.firecrawlUsage": "Firecrawl 额度",
    "dash.tavilyUsage": "Tavily 额度",
    "dash.exaUsage": "Exa 用量",
    "dash.used": "已用",
    "dash.limit": "上限",
    "dash.remaining": "剩余",
    "dash.lastUpdated": "更新于",
    "dash.requests": "请求",
    "dash.estimated": "估算",
    "dash.manualRefresh": "刷新",
    "dash.refreshing": "刷新中...",
    "dash.viewLogs": "查看日志",
    "dash.noProblemKeys": "暂无异常 Key",
    "dash.todayUsage": "今日使用（阶段2）",
    "dash.todayFromRuntime": "按运行时指标统计",
    "dash.todayRequests": "请求总数",
    "dash.todayRetries": "重试总数",
    "dash.todayProviders": "有流量 Provider",
    "dash.todayNoTraffic": "今日暂无调用数据",
    "dash.futureStats": "统计看板（预留）",
    "dash.futureStatsHint": "后续将加入今日使用与趋势图。",
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
    "cfg.exaUpstreamUrl": "Exa 上游 Base URL",
    "cfg.networkSettings": "网络设置",
    "cfg.systemSettings": "系统设置",
    "cfg.host": "主机",
    "cfg.firecrawlPort": "Firecrawl 端口",
    "cfg.tavilyPort": "Tavily 端口",
    "cfg.exaPort": "Exa 端口",
    "cfg.requestTimeout": "请求超时",
    "cfg.keyCooldown": "Key 冷却时间",
    "cfg.autoStart": "自动启动代理",
    "cfg.autoStartHint": "应用启动时自动开启代理服务器",
    "cfg.silentStart": "静默启动",
    "cfg.silentStartHint": "启动时不显示主窗口（仅在系统托盘运行）",
    "cfg.launchOnLogin": "开机自启",
    "cfg.launchOnLoginHint": "系统登录后自动启动本应用",
    "cfg.apiKeysSection": "API 密钥",
    "cfg.disableByToggle": "通过开关启用/禁用 key（保存后生效）",
    "cfg.disabled": "禁用",
    "cfg.enabled": "启用",
    "cfg.addKey": "新增",
    "cfg.addPrompt": "请输入 Key（单个或多个，逗号/换行分隔）",
    "cfg.batchAdd": "批量导入",
    "cfg.remove": "删除",
    "cfg.colKey": "Key",
    "cfg.colStatus": "状态",
    "cfg.colAction": "操作",
    "cfg.batchPrompt": "请输入 Keys（每行一个，或逗号分隔）",
    "cfg.noKeys": "暂无 key，点击右上角新增。",
    "cfg.save": "保存配置",
    "cfg.reload": "重新读取配置",
    "cfg.unsaved": "有未保存的更改",
    "cfg.saved": "配置已保存",
    "cfg.reloaded": "配置已重新读取",
    "cfg.saveFailed": "保存失败: ",
    "cfg.loadFailed": "加载配置失败: ",
    "cfg.reloadConfirm": "当前有未保存的更改，确定重新读取并覆盖吗？",
    "cfg.launchOnLoginFailed": "设置开机自启失败: ",
    "cfg.keySummary": "${total} 个 key / ${disabled} 个禁用",
    "cfg.keySummaryAll": "${total} 个 key",
    "cfg.selectAll": "全选",
    "cfg.batchEnable": "批量启用",
    "cfg.batchDisable": "批量禁用",
    "cfg.batchRemove": "批量删除",
    "cfg.batchRemoveConfirm": "确定删除选中的 ${count} 个 key 吗？",
    "cfg.viewList": "列表",
    "cfg.viewText": "文本",
    "cfg.textModeHint": "一行一个 key，# 开头表示禁用",
    "cfg.noKeysAccordion": "暂无 key",

    // MCP
    "mcp.title": "MCP 配置",
    "mcp.desc": "选择要生成的配置并复制到 MCP 客户端配置文件。",
    "mcp.scopeLabel": "配置范围",
    "mcp.scopeBoth": "Firecrawl + Tavily",
    "mcp.scopeAll": "全部 (Firecrawl + Tavily + Exa)",
    "mcp.scopeFirecrawl": "仅 Firecrawl",
    "mcp.scopeTavily": "仅 Tavily",
    "mcp.scopeExa": "仅 Exa",
    "mcp.unavailable": "未配置",
    "mcp.copyJson": "复制当前 JSON",
    "mcp.instructions": "使用说明",
    "mcp.errFirecrawl": "Firecrawl 未完全配置",
    "mcp.errTavily": "Tavily 未完全配置",
    "mcp.errExa": "Exa 未完全配置",
    "mcp.errInvalid": "无效的 MCP 目标",
    "mcp.errNone": "没有可用配置",
    "mcp.step1": "在 <a id=\"mcpGoDash\">仪表盘</a> 启动代理",
    "mcp.step2": "从下拉框选择配置范围并复制 JSON",
    "mcp.step3": "粘贴到 MCP 客户端配置文件（Claude Desktop、Cursor 等）",
    "mcp.step4": "重启 MCP 客户端使配置生效",
    "mcp.note": "<code>FIRECRAWL_API_URL</code>/<code>FIRECRAWL_API_KEY</code> 与 <code>TAVILY_API_URL</code>/<code>TAVILY_API_KEY</code> 都指向本地代理与代理 Token。",
    "mcp.loadFailed": "加载 MCP 配置失败: ",

    // Logs
    "logs.title": "最近日志",
    "logs.all": "全部",
    "logs.search": "搜索日志...",
    "logs.refresh": "刷新",
    "logs.showing": "显示 {0} / {1} 条",
    "logs.autoRefresh": "自动刷新: 2s",

    // Global proxy bar
    "global.proxy": "代理开关",
    "global.running": "运行中",
    "global.degraded": "部分运行",
    "global.stopped": "已停止",
    "global.starting": "启动中...",
    "global.stopping": "停止中...",

    // Common
    "copied": "已复制到剪贴板",
    "copyFailed": "复制失败: ",
    "modal.ok": "确定",
    "modal.cancel": "取消",

    // Units
    "unit.credits": "额度",
    "unit.requests": "请求",
    "unit.usd": "USD",

    // Disabled reasons
    "reason.account_deactivated": "账号已停用",
    "reason.upstream_401": "上游鉴权失败（401）",
    "reason.usage_401": "用量接口鉴权失败（401）",
    "reason.usage_429": "用量接口限频（429）",

    "key.unverified": "待验证",
  },
  en: {
    "nav.dashboard": "Dashboard",
    "nav.config": "Configuration",
    "nav.mcp": "MCP Config",
    "nav.logs": "Logs",

    "status.running": "Running",
    "status.degraded": "Degraded",
    "status.stopped": "Stopped",

    "dash.title": "Dashboard",
    "dash.totalKeys": "Total Keys",
    "dash.availableKeys": "Available Keys",
    "dash.coolingDown": "Cooling Down",
    "dash.disabledKeys": "Disabled",
    "dash.stoppedKeys": "Stopped",
    "dash.unverifiedKeys": "Unverified",
    "dash.keyOverview": "Key Status",
    "dash.usage": "Usage",
    "dash.usageFetchFailed": "Failed to refresh usage",
    "dash.notConfigured": "Not configured",
    "dash.notAvailable": "Unavailable",
    "dash.firecrawlUsage": "Firecrawl Usage",
    "dash.tavilyUsage": "Tavily Usage",
    "dash.exaUsage": "Exa Usage",
    "dash.used": "Used",
    "dash.limit": "Limit",
    "dash.remaining": "Remaining",
    "dash.lastUpdated": "Updated",
    "dash.requests": "Requests",
    "dash.estimated": "Estimated",
    "dash.manualRefresh": "Refresh",
    "dash.refreshing": "Refreshing...",
    "dash.viewLogs": "View logs",
    "dash.noProblemKeys": "No problematic keys",
    "dash.todayUsage": "Today Usage (Phase 2)",
    "dash.todayFromRuntime": "Derived from runtime metrics",
    "dash.todayRequests": "Total Requests",
    "dash.todayRetries": "Total Retries",
    "dash.todayProviders": "Providers with Traffic",
    "dash.todayNoTraffic": "No traffic recorded today",
    "dash.futureStats": "Future Analytics",
    "dash.futureStatsHint": "Daily usage and charts will be added later.",
    "dash.proxyStarted": "Proxy started",
    "dash.proxyStopped": "Proxy stopped",
    "dash.startFailed": "Start failed: ",
    "dash.stopFailed": "Stop failed: ",

    "cfg.title": "Configuration",
    "cfg.proxySettings": "Proxy Settings",
    "cfg.proxyToken": "Proxy Token",
    "cfg.firecrawlUpstreamUrl": "Firecrawl Upstream Base URL",
    "cfg.tavilyUpstreamUrl": "Tavily Upstream Base URL",
    "cfg.exaUpstreamUrl": "Exa Upstream Base URL",
    "cfg.networkSettings": "Network Settings",
    "cfg.systemSettings": "System Settings",
    "cfg.host": "Host",
    "cfg.firecrawlPort": "Firecrawl Port",
    "cfg.tavilyPort": "Tavily Port",
    "cfg.exaPort": "Exa Port",
    "cfg.requestTimeout": "Request Timeout",
    "cfg.keyCooldown": "Key Cooldown",
    "cfg.autoStart": "Auto-start Proxy",
    "cfg.autoStartHint": "Automatically start the proxy server when the app launches",
    "cfg.silentStart": "Silent Start",
    "cfg.silentStartHint": "Hide the main window on launch (run in system tray only)",
    "cfg.launchOnLogin": "Launch on login",
    "cfg.launchOnLoginHint": "Automatically start this app after user login",
    "cfg.apiKeysSection": "API Keys",
    "cfg.disableByToggle": "Use switches to enable/disable keys (applies after save)",
    "cfg.disabled": "Disabled",
    "cfg.enabled": "Enabled",
    "cfg.addKey": "Add",
    "cfg.addPrompt": "Enter key(s) (comma/newline separated)",
    "cfg.batchAdd": "Batch Import",
    "cfg.remove": "Remove",
    "cfg.colKey": "Key",
    "cfg.colStatus": "Status",
    "cfg.colAction": "Action",
    "cfg.batchPrompt": "Paste keys (one per line or comma-separated)",
    "cfg.noKeys": "No keys yet. Click Add in the header.",
    "cfg.save": "Save Configuration",
    "cfg.reload": "Reload Configuration",
    "cfg.unsaved": "Unsaved changes",
    "cfg.saved": "Configuration saved",
    "cfg.reloaded": "Configuration reloaded",
    "cfg.saveFailed": "Save failed: ",
    "cfg.loadFailed": "Failed to load config: ",
    "cfg.reloadConfirm": "You have unsaved changes. Reload and overwrite them?",
    "cfg.launchOnLoginFailed": "Failed to update launch-on-login: ",
    "cfg.keySummary": "${total} keys / ${disabled} disabled",
    "cfg.keySummaryAll": "${total} keys",
    "cfg.selectAll": "Select all",
    "cfg.batchEnable": "Enable",
    "cfg.batchDisable": "Disable",
    "cfg.batchRemove": "Delete",
    "cfg.batchRemoveConfirm": "Remove ${count} selected key(s)?",
    "cfg.viewList": "List",
    "cfg.viewText": "Text",
    "cfg.textModeHint": "One key per line. Prefix with # to disable.",
    "cfg.noKeysAccordion": "No keys",

    "mcp.title": "MCP Configuration",
    "mcp.desc": "Choose a scope and copy the generated JSON to your MCP client settings file.",
    "mcp.scopeLabel": "Scope",
    "mcp.scopeBoth": "Firecrawl + Tavily",
    "mcp.scopeAll": "All (Firecrawl + Tavily + Exa)",
    "mcp.scopeFirecrawl": "Firecrawl only",
    "mcp.scopeTavily": "Tavily only",
    "mcp.scopeExa": "Exa only",
    "mcp.unavailable": "Unavailable",
    "mcp.copyJson": "Copy current JSON",
    "mcp.instructions": "Instructions",
    "mcp.errFirecrawl": "Firecrawl is not fully configured",
    "mcp.errTavily": "Tavily is not fully configured",
    "mcp.errExa": "Exa is not fully configured",
    "mcp.errInvalid": "Invalid MCP target",
    "mcp.errNone": "No configured MCP providers",
    "mcp.step1": "Start the proxy on the <a id=\"mcpGoDash\">Dashboard</a>",
    "mcp.step2": "Select a scope from the dropdown and copy the JSON",
    "mcp.step3": "Paste it into your MCP client config file (Claude Desktop, Cursor, etc.)",
    "mcp.step4": "Restart your MCP client to apply changes",
    "mcp.note": "<code>FIRECRAWL_API_URL</code>/<code>FIRECRAWL_API_KEY</code> and <code>TAVILY_API_URL</code>/<code>TAVILY_API_KEY</code> both point to your local proxies and proxy token.",
    "mcp.loadFailed": "Failed to load MCP config: ",

    "logs.title": "Recent Logs",
    "logs.all": "All",
    "logs.search": "Search logs...",
    "logs.refresh": "Refresh",
    "logs.showing": "Showing {0} of {1} lines",
    "logs.autoRefresh": "Auto-refresh: 2s",

    "global.proxy": "Proxy",
    "global.running": "Running",
    "global.degraded": "Degraded",
    "global.stopped": "Stopped",
    "global.starting": "Starting...",
    "global.stopping": "Stopping...",

    "copied": "Copied to clipboard",
    "copyFailed": "Copy failed: ",
    "modal.ok": "OK",
    "modal.cancel": "Cancel",

    "unit.credits": "credits",
    "unit.requests": "requests",
    "unit.usd": "USD",

    "reason.account_deactivated": "Account deactivated",
    "reason.upstream_401": "Upstream 401 (Unauthorized)",
    "reason.usage_401": "Usage 401 (Unauthorized)",
    "reason.usage_429": "Usage 429 (Rate limited)",

    "key.unverified": "Unverified",
  },
};

let currentLang = localStorage.getItem("lang") || "zh";

const DASHBOARD_STATE_VERSION = 1;
// Legacy localStorage keys (migrated to dashboard-state.json via Tauri)
const LEGACY_STORAGE_USAGE_BASELINES_KEY = "balanceProxy.usageBaselines.v1";
const LEGACY_STORAGE_METRICS_STATE_KEY = "balanceProxy.metricsState.v1";

function readLegacyStorageJson(key) {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function removeLegacyStorageKey(key) {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}

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
  const navLabels = { dashboard: "nav.dashboard", config: "nav.config", mcp: "nav.mcp", logs: "nav.logs" };
  sidebarNav.querySelectorAll(".nav-item").forEach((btn) => {
    const key = navLabels[btn.dataset.page];
    if (key) btn.querySelector("span").textContent = t(key);
  });

  if (globalProxyLabel) {
    globalProxyLabel.textContent = t("global.proxy");
  }

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

// ---- Modal dialogs (WebView-safe) ----
let activeModal = null;

function openModal({
  title = "",
  message = "",
  bodyEl = null,
  okText = null,
  cancelText = null,
  okClass = "btn-primary",
  submitOnEnter = false,
  submitOnCtrlEnter = false,
  initialFocusEl = null,
} = {}) {
  if (activeModal && typeof activeModal.close === "function") {
    activeModal.close(null);
  }

  return new Promise((resolve) => {
    const lastFocus = document.activeElement;
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    const overlay = document.createElement("div");
    overlay.className = "modal-overlay";

    const dialog = document.createElement("div");
    dialog.className = "modal";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");

    const titleEl = document.createElement("div");
    titleEl.className = "modal-title";
    if (title) titleEl.textContent = title;
    else titleEl.style.display = "none";

    const msgEl = document.createElement("div");
    msgEl.className = "modal-message";
    if (message) msgEl.textContent = message;
    else msgEl.style.display = "none";

    dialog.appendChild(titleEl);
    dialog.appendChild(msgEl);

    if (bodyEl) {
      const bodyWrap = document.createElement("div");
      bodyWrap.className = "modal-body";
      bodyWrap.appendChild(bodyEl);
      dialog.appendChild(bodyWrap);
    }

    const actionsEl = document.createElement("div");
    actionsEl.className = "modal-actions";

    const cancelBtn = document.createElement("button");
    cancelBtn.type = "button";
    cancelBtn.className = "btn";
    cancelBtn.textContent = cancelText || t("modal.cancel");

    const okBtn = document.createElement("button");
    okBtn.type = "button";
    okBtn.className = `btn ${okClass}`;
    okBtn.textContent = okText || t("modal.ok");

    actionsEl.appendChild(cancelBtn);
    actionsEl.appendChild(okBtn);
    dialog.appendChild(actionsEl);

    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    let closed = false;
    const close = (value) => {
      if (closed) return;
      closed = true;
      document.removeEventListener("keydown", onKeyDown, true);
      overlay.remove();
      document.body.style.overflow = prevOverflow;
      activeModal = null;
      if (lastFocus && typeof lastFocus.focus === "function") {
        setTimeout(() => {
          try { lastFocus.focus(); } catch { }
        }, 0);
      }
      resolve(value);
    };

    const onOk = () => close(true);
    const onCancel = () => close(null);

    cancelBtn.addEventListener("click", onCancel);
    okBtn.addEventListener("click", onOk);
    overlay.addEventListener("mousedown", (e) => {
      if (e.target === overlay) onCancel();
    });

    const onKeyDown = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
        return;
      }
      if (e.key !== "Enter") return;

      const isTextArea = e.target && e.target.tagName === "TEXTAREA";
      if (submitOnEnter && !isTextArea) {
        e.preventDefault();
        onOk();
        return;
      }
      if (submitOnCtrlEnter && isTextArea && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        onOk();
      }
    };
    document.addEventListener("keydown", onKeyDown, true);

    activeModal = { close };

    const focusEl =
      initialFocusEl ||
      (bodyEl && (bodyEl.querySelector?.("input, textarea") || null)) ||
      okBtn;
    setTimeout(() => {
      if (focusEl && typeof focusEl.focus === "function") {
        try { focusEl.focus(); } catch { }
      }
    }, 0);
  });
}

async function uiConfirm(message, { title = "", okText = null, cancelText = null, okClass = "btn-primary" } = {}) {
  const ok = await openModal({
    title,
    message,
    okText,
    cancelText,
    okClass,
    submitOnEnter: true,
  });
  return ok === true;
}

async function uiPrompt(
  message,
  {
    title = "",
    defaultValue = "",
    placeholder = "",
    multiline = false,
    okText = null,
    cancelText = null,
  } = {},
) {
  const field = multiline ? document.createElement("textarea") : document.createElement("input");
  field.className = multiline ? "form-textarea modal-textarea" : "form-input";
  if (!multiline) field.type = "text";
  field.placeholder = placeholder || "";
  field.spellcheck = false;
  field.value = defaultValue || "";

  const body = document.createElement("div");
  body.appendChild(field);

  const ok = await openModal({
    title,
    message,
    bodyEl: body,
    okText,
    cancelText,
    okClass: "btn-primary",
    submitOnEnter: !multiline,
    submitOnCtrlEnter: multiline,
    initialFocusEl: field,
  });

  if (ok !== true) return null;
  return field.value;
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

function idleStatusesFromKeys(keys, disabledKeys = []) {
  const disabledSet = new Set(disabledKeys || []);
  return (keys || []).map((key, index) => ({
    index,
    keyPreview: truncateKey(key),
    isCoolingDown: false,
    cooldownRemainingSecs: 0,
    isDisabled: disabledSet.has(key),
    disabledReason: null,
    disabledAtTs: null,
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
  if (provider === "exa") {
    return !!((config?.exaApiKeys || []).length && (config?.exaUpstreamBaseUrl || "").trim());
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
      keys: firecrawlConfigured
        ? idleStatusesFromKeys(config?.firecrawlApiKeys, config?.firecrawlDisabledApiKeys)
        : [],
    },
    tavily: {
      configured: tavilyConfigured,
      running: !!status?.tavilyRunning,
      keys: tavilyConfigured
        ? idleStatusesFromKeys(config?.tavilyApiKeys, config?.tavilyDisabledApiKeys)
        : [],
    },
    exa: {
      configured: isProviderConfigured(config, "exa"),
      running: !!status?.exaRunning,
      keys: isProviderConfigured(config, "exa")
        ? idleStatusesFromKeys(config?.exaApiKeys, config?.exaDisabledApiKeys)
        : [],
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
  if (snapshot?.exa?.configured) merged.push(...(snapshot.exa.keys || []));
  return merged;
}

function formatMetricValue(value) {
  if (typeof value !== "number" || Number.isNaN(value)) return "-";
  return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
}

function localizeUsageUnit(unit) {
  const raw = String(unit ?? "").trim();
  if (!raw) return "";
  const normalized = raw.toLowerCase();
  if (normalized === "credits" || normalized === "credit") return t("unit.credits");
  if (normalized === "requests" || normalized === "request") return t("unit.requests");
  if (normalized === "usd") return t("unit.usd");
  return raw;
}

function parsePartialKeyCounts(summary) {
  const text = String(summary ?? "");
  const match = text.match(/\(\s*(\d+)\s*\/\s*(\d+)\s*keys\s*\)/i);
  if (!match) return null;
  const ok = Number(match[1]);
  const total = Number(match[2]);
  if (!Number.isFinite(ok) || !Number.isFinite(total)) return null;
  return { ok, total };
}

function parseStatusBodyError(err) {
  const text = String(err ?? "");
  const match = text.match(/^status\s+(\d+)\s+body\s+([\s\S]+)$/i);
  if (!match) return null;
  const status = Number(match[1]);
  const body = match[2];
  if (!Number.isFinite(status)) return null;
  return { status, body };
}

function localizeUsageError(err) {
  const parsed = parseStatusBodyError(err);
  if (!parsed) {
    const text = String(err ?? "");
    if (currentLang === "zh") {
      if (text.includes("No usage fields found")) return "上游返回成功但未包含用量字段";
      if (text.includes("No valid usage payload")) return "上游未返回有效用量数据";
    }
    return text;
  }

  const statusText = String(parsed.status);
  const body = String(parsed.body ?? "");

  if (currentLang !== "zh") {
    return `status ${statusText} ${body ? `body ${body}` : ""}`.trim();
  }

  const statusLabel = (() => {
    if (parsed.status === 401) return "未授权 / Key 无效";
    if (parsed.status === 402) return "余额不足 / 需要订阅";
    if (parsed.status === 429) return "速率限制（建议 60s 后重试）";
    return "请求失败";
  })();

  let msg = "";
  try {
    const json = JSON.parse(body);
    msg =
      json?.detail?.error ||
      json?.detail ||
      json?.error ||
      json?.message ||
      "";
    if (typeof msg !== "string") msg = "";
  } catch {
    // ignore
  }

  let tail = (msg || body || "").trim();

  const tailLower = tail.toLowerCase();
  if (tailLower.includes("the account associated with this api key has been deactivated")) {
    const supportEmail = (tail.match(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/i) || [])[0] || "support@tavily.com";
    tail = `该 API Key 所属账号已被停用（如需恢复请联系 ${supportEmail}）`;
  } else if (tailLower.includes("rate limit exceeded")) {
    tail = "触发速率限制，请稍后重试";
  }

  return tail ? `${statusText}：${statusLabel} · ${tail}` : `${statusText}：${statusLabel}`;
}

function localizeDisableReason(reasonCode) {
  const raw = String(reasonCode ?? "").trim();
  if (!raw) return "";
  const i18nKey = `reason.${raw}`;
  const translated = t(i18nKey);
  return translated === i18nKey ? raw : translated;
}

function formatTsDisplay(ts) {
  if (!ts) return "-";
  const d = new Date(ts * 1000);
  if (Number.isNaN(d.getTime())) return "-";
  return d.toLocaleString();
}

function escapeHtml(text) {
  return String(text ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

// ---- JSON syntax highlight ----
function highlightJSON(json) {
  return json
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"([^"]+)"(\s*:)/g, '<span class="json-key">"$1"</span>$2')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/[{}\[\]]/g, '<span class="json-bracket">$&</span>');
}

function translateMcpError(errStr) {
  if (!errStr) return errStr;
  if (errStr.includes("Firecrawl is not fully configured")) return t("mcp.errFirecrawl");
  if (errStr.includes("Tavily is not fully configured")) return t("mcp.errTavily");
  if (errStr.includes("Exa is not fully configured")) return t("mcp.errExa");
  if (errStr.includes("Invalid MCP target")) return t("mcp.errInvalid");
  if (errStr.includes("No configured MCP providers")) return t("mcp.errNone");
  return errStr;
}

// ============================================
// 3. ROUTER
// ============================================
let currentPage = null;
let currentPageId = null;
let globalTimer = null;
let globalToggleBusy = false;

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
  return !!(status && (status.anyRunning || status.running || status.firecrawlRunning || status.tavilyRunning || status.exaRunning));
}

function formatProxyUrls(status) {
  const urls = [];
  if (status?.listenUrl) urls.push(`FC ${status.listenUrl.replace("http://", "")}`);
  if (status?.tavilyListenUrl) urls.push(`TV ${status.tavilyListenUrl.replace("http://", "")}`);
  if (status?.exaListenUrl) urls.push(`EXA ${status.exaListenUrl.replace("http://", "")}`);
  return urls.join("<br>") || "-";
}

function updateGlobalSwitch(status, pendingTextKey = null) {
  if (!globalProxyLabel || !globalProxyToggle || !sidebarBadge) return;
  globalProxyLabel.textContent = t("global.proxy");
  const isOn = isAnyProxyRunning(status);
  globalProxyToggle.classList.toggle("on", isOn);
  globalProxyToggle.disabled = !!pendingTextKey || globalToggleBusy;

  if (pendingTextKey) {
    sidebarBadge.className = "badge badge-muted";
    sidebarBadge.textContent = t(pendingTextKey);
  }
}

async function toggleProxyFromGlobal() {
  if (globalToggleBusy) return;
  globalToggleBusy = true;
  const currentStatus = await updateSidebarStatus();
  const shouldStop = isAnyProxyRunning(currentStatus);
  updateGlobalSwitch(currentStatus, shouldStop ? "global.stopping" : "global.starting");

  try {
    if (shouldStop) {
      await invoke("stop_proxy");
      showToast(t("dash.proxyStopped"), "success");
    } else {
      await invoke("start_proxy");
      showToast(t("dash.proxyStarted"), "success");
    }
  } catch (e) {
    showToast((shouldStop ? t("dash.stopFailed") : t("dash.startFailed")) + e, "error");
  } finally {
    globalToggleBusy = false;
    await updateSidebarStatus();
    if (currentPage && currentPage._refresh) {
      currentPage._refresh();
    }
  }
}

async function updateSidebarStatus() {
  try {
    const status = await invoke("get_proxy_status");
    if (status?.running) {
      sidebarBadge.className = "badge badge-success";
      sidebarBadge.textContent = t("status.running");
      sidebarUrl.innerHTML = formatProxyUrls(status);
    } else if (isAnyProxyRunning(status)) {
      sidebarBadge.className = "badge badge-warning";
      sidebarBadge.textContent = t("status.degraded");
      sidebarUrl.innerHTML = formatProxyUrls(status);
    } else {
      sidebarBadge.className = "badge badge-danger";
      sidebarBadge.textContent = t("status.stopped");
      sidebarUrl.innerHTML = "-";
    }
    updateGlobalSwitch(status);
    return status;
  } catch {
    updateGlobalSwitch({ running: false });
    return { running: false };
  }
}

// ============================================
// 4. PAGE: Dashboard
// ============================================
pages.dashboard = {
  _timer: null,
  _usageTimer: null,
  _persistTimer: null,
  _persistDirty: false,
  _persistInFlight: false,
  _keySnapshot: null,
  _keyDetailsInitialized: false,
  _usageByProvider: {
    firecrawl: null,
    tavily: null,
    exa: null,
  },
  _usageBaselines: {
    firecrawl: {
      used: null,
      limit: null,
      remaining: null,
      requestCount: 0,
      secondaryUsed: null,
      secondaryLimit: null,
      secondaryRemaining: null,
      seededAt: 0,
    },
    tavily: {
      used: null,
      limit: null,
      remaining: null,
      requestCount: 0,
      secondaryUsed: null,
      secondaryLimit: null,
      secondaryRemaining: null,
      seededAt: 0,
    },
    exa: {
      used: null,
      limit: null,
      remaining: null,
      requestCount: 0,
      secondaryUsed: null,
      secondaryLimit: null,
      secondaryRemaining: null,
      seededAt: 0,
    },
  },
  _metricsState: null,
  _metrics: null,
  _refreshingProvider: new Set(),

  template() {
    return `
      <div class="dash-hero">
        <div class="dash-hero-number" id="statAvailable">-</div>
        <div class="dash-hero-label">${t("dash.availableKeys")}</div>
        <div class="dash-hero-sub">
          <span><span id="statTotalKeys">-</span> ${t("dash.totalKeys")}</span>
          <span class="dash-hero-dot"></span>
          <span><span id="statUnverified">-</span> ${t("dash.unverifiedKeys")}</span>
          <span class="dash-hero-dot"></span>
          <span><span id="statCooldown">-</span> ${t("dash.coolingDown")}</span>
          <span class="dash-hero-dot"></span>
          <span><span id="statStopped">-</span> ${t("dash.stoppedKeys")}</span>
          <span class="dash-hero-dot"></span>
          <span><span id="statDisabled">-</span> ${t("dash.disabledKeys")}</span>
        </div>
        <div class="dash-hero-today">
          <span id="dashTodayTotal">0</span> ${t("dash.todayRequests")}
          <span class="dash-hero-dot"></span>
          <span id="dashTodayRetries">0</span> ${t("dash.todayRetries")}
        </div>
      </div>

      <div class="dash-section-head">
        <span id="dashUsageUpdated" class="dash-updated">-</span>
      </div>
      <div id="dashUsageGrid" class="dash-providers"></div>

      <details class="dash-key-details" id="dashKeyDetails">
        <summary>
          <span>${t("dash.keyOverview")}</span>
          <svg class="dash-details-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 12 15 18 9"/></svg>
        </summary>
        <div id="dashKeyOverview" class="dash-key-overview"></div>
      </details>
    `;
  },

  async init() {
    contentEl.addEventListener("click", this._onUsageClick);
    await this._restoreUsageState();
    this._keyDetailsInitialized = false;
    await this._refresh();
    await this._refreshUsageAll();
    this._renderUsage();
    this._timer = setInterval(() => this._refresh(), 2000);
    // Exa usage is computed locally, so we can refresh it periodically
    // to keep the USD budget numbers up-to-date without hitting upstream APIs.
    this._usageTimer = setInterval(() => {
      const exa = this._usageByProvider?.exa;
      if (!exa?.configured || !exa?.hasEnabledKey) return;
      this._refreshProviderUsage("exa", { silent: true });
    }, 10_000);
  },

  async _refresh() {
    try {
      const [status, config, keySnapshotRaw, metrics] = await Promise.all([
        invoke("get_proxy_status"),
        invoke("load_proxy_config"),
        invoke("get_key_status_snapshot").catch(() => null),
        invoke("get_runtime_metrics").catch(() => null),
      ]);
      const keySnapshot = keySnapshotRaw || buildFallbackKeySnapshot(config, status);
      this._keySnapshot = keySnapshot;
      this._metrics = this._applyMetricsPersistence(metrics);

      const mergedKeyStatuses = mergeConfiguredKeys(keySnapshot);
      const totalKeys = mergedKeyStatuses.length;
      const unverifiedCount = mergedKeyStatuses.filter((k) => {
        const state = String(k.verificationState || "unknown").toLowerCase();
        return !k.isCoolingDown && !k.isDisabled && state === "unknown";
      }).length;
      const availableCount = mergedKeyStatuses.filter((k) => {
        const state = String(k.verificationState || "unknown").toLowerCase();
        return !k.isCoolingDown && !k.isDisabled && state !== "unknown";
      }).length;
      const stoppedCount = mergedKeyStatuses.filter((k) => {
        const reason = String(k.disabledReason || "").toLowerCase();
        return !!k.isDisabled && reason === "account_deactivated";
      }).length;
      const disabledCount = mergedKeyStatuses.filter((k) => {
        const reason = String(k.disabledReason || "").toLowerCase();
        return !!k.isDisabled && reason !== "account_deactivated";
      }).length;
      const cooldownCount = mergedKeyStatuses.filter((k) => k.isCoolingDown && !k.isDisabled).length;

      const totalEl = document.getElementById("statTotalKeys");
      const activeEl = document.getElementById("statAvailable");
      const unverifiedEl = document.getElementById("statUnverified");
      const disabledEl = document.getElementById("statDisabled");
      const stoppedEl = document.getElementById("statStopped");
      const cooldownEl = document.getElementById("statCooldown");
      if (totalEl) totalEl.textContent = String(totalKeys);
      if (activeEl) activeEl.textContent = String(availableCount);
      if (unverifiedEl) unverifiedEl.textContent = String(unverifiedCount);
      if (stoppedEl) stoppedEl.textContent = String(stoppedCount);
      if (disabledEl) disabledEl.textContent = String(disabledCount);
      if (cooldownEl) cooldownEl.textContent = String(cooldownCount);
      this._renderKeyOverview();
      this._renderUsage();
      this._renderTodayStats();
    } catch {
      // silently ignore
    }
  },

  async _restoreUsageState() {
    let storedBaselines = null;
    let storedMetricsState = null;

    if (invoke) {
      try {
        const persisted = await invoke("load_dashboard_state");
        storedBaselines = persisted?.usageBaselines || null;
        storedMetricsState = persisted?.metricsState || null;
      } catch {
        // ignore
      }
    }

    const legacyBaselines = readLegacyStorageJson(LEGACY_STORAGE_USAGE_BASELINES_KEY);
    const legacyMetricsState = readLegacyStorageJson(LEGACY_STORAGE_METRICS_STATE_KEY);
    const migratedBaselines = !storedBaselines && legacyBaselines && typeof legacyBaselines === "object";
    const migratedMetrics = !storedMetricsState && legacyMetricsState && typeof legacyMetricsState === "object";
    if ((migratedBaselines || migratedMetrics) && invoke) {
      if (migratedBaselines) storedBaselines = legacyBaselines;
      if (migratedMetrics) storedMetricsState = legacyMetricsState;
      try {
        await invoke("save_dashboard_state", {
          payload: {
            version: DASHBOARD_STATE_VERSION,
            usageBaselines: storedBaselines,
            metricsState: storedMetricsState,
          },
        });
        removeLegacyStorageKey(LEGACY_STORAGE_USAGE_BASELINES_KEY);
        removeLegacyStorageKey(LEGACY_STORAGE_METRICS_STATE_KEY);
      } catch {
        // ignore
      }
    }

    if (storedBaselines && typeof storedBaselines === "object") {
      ["firecrawl", "tavily", "exa"].forEach((provider) => {
        const baseline = storedBaselines?.[provider];
        if (!baseline || typeof baseline !== "object") return;
        const seededAt = Number(baseline.seededAt || 0);
        if (!Number.isFinite(seededAt) || seededAt <= 0) return;
        const hasAnyNumber =
          typeof baseline.used === "number"
          || typeof baseline.limit === "number"
          || typeof baseline.remaining === "number"
          || typeof baseline.secondaryUsed === "number"
          || typeof baseline.secondaryLimit === "number"
          || typeof baseline.secondaryRemaining === "number";
        if (!hasAnyNumber) return;
        this._usageBaselines[provider] = {
          used: typeof baseline.used === "number" ? baseline.used : null,
          limit: typeof baseline.limit === "number" ? baseline.limit : null,
          remaining: typeof baseline.remaining === "number" ? baseline.remaining : null,
          requestCount: Number(baseline.requestCount || 0),
          secondaryUsed: typeof baseline.secondaryUsed === "number" ? baseline.secondaryUsed : null,
          secondaryLimit: typeof baseline.secondaryLimit === "number" ? baseline.secondaryLimit : null,
          secondaryRemaining: typeof baseline.secondaryRemaining === "number" ? baseline.secondaryRemaining : null,
          seededAt,
        };
      });
    }

    if (storedMetricsState && typeof storedMetricsState === "object") {
      this._metricsState = storedMetricsState;
    } else {
      this._metricsState = null;
    }
  },

  _persistUsageBaselines() {
    this._queuePersistDashboardState();
  },

  _persistMetricsState() {
    if (!this._metricsState) return;
    this._queuePersistDashboardState();
  },

  _queuePersistDashboardState() {
    if (!invoke) return;
    this._persistDirty = true;
    if (this._persistTimer) clearTimeout(this._persistTimer);
    this._persistTimer = setTimeout(() => {
      this._persistTimer = null;
      void this._flushPersistDashboardState();
    }, 400);
  },

  async _flushPersistDashboardState() {
    if (!invoke) return;
    if (this._persistInFlight || !this._persistDirty) return;

    this._persistInFlight = true;
    this._persistDirty = false;
    try {
      await invoke("save_dashboard_state", {
        payload: {
          version: DASHBOARD_STATE_VERSION,
          usageBaselines: this._usageBaselines,
          metricsState: this._metricsState,
        },
      });
    } catch {
      this._persistDirty = true;
    } finally {
      this._persistInFlight = false;
    }
    if (this._persistDirty) {
      this._queuePersistDashboardState();
    }
  },

  _applyMetricsPersistence(metrics) {
    if (!metrics || typeof metrics !== "object") {
      return metrics;
    }

    const providers = ["firecrawl", "tavily", "exa"];
    const state = (this._metricsState && typeof this._metricsState === "object")
      ? this._metricsState
      : {};

    const adjusted = {};
    let stateChanged = false;
    providers.forEach((provider) => {
      const raw = metrics?.[provider] || {};
      const rawRequest = Number(raw.requestCount || 0);
      const rawRetry = Number(raw.retryCount || 0);

      const entry = (state[provider] && typeof state[provider] === "object")
        ? state[provider]
        : {};

      let requestOffset = Number(entry.requestOffset || 0);
      let retryOffset = Number(entry.retryOffset || 0);
      const lastRawRequest = Number(entry.lastRawRequest || 0);
      const lastRawRetry = Number(entry.lastRawRetry || 0);

      if (rawRequest < lastRawRequest) {
        requestOffset += lastRawRequest;
      }
      if (rawRetry < lastRawRetry) {
        retryOffset += lastRawRetry;
      }

      const nextEntry = {
        requestOffset,
        retryOffset,
        lastRawRequest: rawRequest,
        lastRawRetry: rawRetry,
      };
      if (
        Number(entry.requestOffset || 0) !== requestOffset
        || Number(entry.retryOffset || 0) !== retryOffset
        || lastRawRequest !== rawRequest
        || lastRawRetry !== rawRetry
      ) {
        stateChanged = true;
      }
      state[provider] = nextEntry;

      adjusted[provider] = {
        requestCount: rawRequest + requestOffset,
        retryCount: rawRetry + retryOffset,
        lastRequestTs: raw.lastRequestTs || null,
      };
    });

    this._metricsState = state;
    if (stateChanged) {
      this._persistMetricsState();
    }
    return adjusted;
  },

  _onUsageClick: (event) => {
    const logsLink = event.target.closest(".usage-open-logs");
    if (logsLink) {
      event.preventDefault();
      navigate("logs");
      return;
    }
    const btn = event.target.closest(".usage-refresh-btn");
    if (!btn) return;
    const provider = btn.dataset.provider;
    if (!provider) return;
    pages.dashboard._refreshProviderUsage(provider);
  },

  _metricFor(provider) {
    return this._metrics?.[provider] || { requestCount: 0, retryCount: 0, lastRequestTs: null };
  },

  _setUsageBaseline(provider, snapshot) {
    const metric = this._metricFor(provider);
    const prevBaseline = this._usageBaselines?.[provider] || {};
    const hasPrimaryMetrics =
      typeof snapshot?.used === "number"
      || typeof snapshot?.limit === "number"
      || typeof snapshot?.remaining === "number";

    const isFirecrawlRemainingOnly =
      provider === "firecrawl"
      && hasPrimaryMetrics
      && typeof snapshot?.used !== "number"
      && typeof snapshot?.limit !== "number"
      && typeof snapshot?.remaining === "number";

    let used = typeof snapshot?.used === "number" ? snapshot.used : null;
    let limit = typeof snapshot?.limit === "number" ? snapshot.limit : null;
    let remaining = typeof snapshot?.remaining === "number" ? snapshot.remaining : null;

    if (isFirecrawlRemainingOnly && typeof remaining === "number") {
      const prevLimit = typeof prevBaseline.limit === "number" ? prevBaseline.limit : null;
      limit = typeof prevLimit === "number" ? Math.max(prevLimit, remaining) : remaining;
      used = Math.max(0, limit - remaining);
    } else if (limit == null && typeof prevBaseline.limit === "number") {
      limit = prevBaseline.limit;
    }

    this._usageBaselines[provider] = {
      used,
      limit,
      remaining,
      requestCount: Number(metric.requestCount || 0),
      secondaryUsed:
        typeof snapshot?.secondaryUsed === "number" ? snapshot.secondaryUsed : null,
      secondaryLimit:
        typeof snapshot?.secondaryLimit === "number"
          ? snapshot.secondaryLimit
          : (typeof prevBaseline.secondaryLimit === "number" ? prevBaseline.secondaryLimit : null),
      secondaryRemaining:
        typeof snapshot?.secondaryRemaining === "number"
          ? snapshot.secondaryRemaining
          : null,
      seededAt: Date.now(),
    };
    this._persistUsageBaselines();
  },

  _estimateUsage(provider, snapshot) {
    const metric = this._metricFor(provider);
    const baseline = this._usageBaselines[provider] || {
      used: null,
      limit: null,
      remaining: null,
      requestCount: 0,
      secondaryUsed: null,
      secondaryLimit: null,
      secondaryRemaining: null,
    };
    const requestCount = Number(metric.requestCount || 0);
    const deltaRequests = Math.max(0, requestCount - Number(baseline.requestCount || 0));

    const baselineUsed = typeof baseline.used === "number" ? baseline.used : (typeof snapshot?.used === "number" ? snapshot.used : null);
    const used = typeof baselineUsed === "number" ? baselineUsed + deltaRequests : null;
    const limit = typeof baseline.limit === "number" ? baseline.limit : (typeof snapshot?.limit === "number" ? snapshot.limit : null);
    let remaining = null;
    if (typeof limit === "number" && typeof used === "number") {
      remaining = Math.max(0, limit - used);
    } else if (typeof baseline.remaining === "number") {
      remaining = Math.max(0, baseline.remaining - deltaRequests);
    } else if (typeof snapshot?.remaining === "number") {
      remaining = Math.max(0, snapshot.remaining - deltaRequests);
    }

    const secondaryUsed =
      typeof snapshot?.secondaryUsed === "number"
        ? snapshot.secondaryUsed
        : (typeof baseline.secondaryUsed === "number" ? baseline.secondaryUsed : null);
    const secondaryLimit =
      typeof baseline.secondaryLimit === "number"
        ? baseline.secondaryLimit
        : (typeof snapshot?.secondaryLimit === "number" ? snapshot.secondaryLimit : null);
    const secondaryRemaining = (() => {
      if (typeof snapshot?.secondaryRemaining === "number") {
        return snapshot.secondaryRemaining;
      }
      if (typeof secondaryLimit === "number" && typeof secondaryUsed === "number") {
        return Math.max(0, secondaryLimit - secondaryUsed);
      }
      if (typeof baseline.secondaryRemaining === "number") {
        return baseline.secondaryRemaining;
      }
      return null;
    })();

    return {
      used,
      limit,
      remaining,
      requestCount,
      deltaRequests,
      unit: snapshot?.unit || "",
      secondaryUsed,
      secondaryLimit,
      secondaryRemaining,
      secondaryUnit: snapshot?.secondaryUnit || "",
    };
  },

  async _refreshUsageAll() {
    try {
      const usage = await invoke("get_usage_snapshot");
      this._usageByProvider = {
        firecrawl: usage?.firecrawl || null,
        tavily: usage?.tavily || null,
        exa: usage?.exa || null,
      };
      ["firecrawl", "tavily", "exa"].forEach((provider) => {
        if (Number(this._usageBaselines?.[provider]?.seededAt || 0) > 0) return;
        const snapshot = this._usageByProvider[provider];
        const hasMetrics = snapshot
          && snapshot.configured
          && snapshot.hasEnabledKey
          && (
            typeof snapshot.used === "number"
            || typeof snapshot.limit === "number"
            || typeof snapshot.remaining === "number"
            || typeof snapshot.secondaryUsed === "number"
            || typeof snapshot.secondaryLimit === "number"
            || typeof snapshot.secondaryRemaining === "number"
          );
        if (!hasMetrics) return;
        this._setUsageBaseline(provider, snapshot);
      });
    } catch (e) {
      showToast(`${t("dash.usageFetchFailed")}: ${e}`, "error");
    }
  },

  async _refreshProviderUsage(provider, options = {}) {
    if (this._refreshingProvider.has(provider)) return;
    const silent = !!options.silent;
    const force = options.force !== undefined ? !!options.force : !silent;
    this._refreshingProvider.add(provider);
    this._renderUsage();
    try {
      const snapshot = await invoke("get_provider_usage", { provider, force });
      this._usageByProvider[provider] = snapshot;
      const hasMetrics = snapshot
        && snapshot.configured
        && snapshot.hasEnabledKey
        && (
          typeof snapshot.used === "number"
          || typeof snapshot.limit === "number"
          || typeof snapshot.remaining === "number"
          || typeof snapshot.secondaryUsed === "number"
          || typeof snapshot.secondaryLimit === "number"
          || typeof snapshot.secondaryRemaining === "number"
        );
      if (hasMetrics) {
        this._setUsageBaseline(provider, snapshot);
      }
    } catch (e) {
      if (!silent) {
        showToast(`${t("dash.usageFetchFailed")}: ${e}`, "error");
      }
    } finally {
      this._refreshingProvider.delete(provider);
      this._renderUsage();
    }
  },

  _renderKeyOverview() {
    const overviewEl = document.getElementById("dashKeyOverview");
    const detailsEl = document.getElementById("dashKeyDetails");
    if (!overviewEl || !detailsEl) return;

    const snapshot = this._keySnapshot;
    if (!snapshot) {
      overviewEl.innerHTML = `<div class="dash-key-empty">${t("dash.notAvailable")}</div>`;
      return;
    }

    const providerOrder = [
      { id: "firecrawl", name: "Firecrawl", data: snapshot.firecrawl },
      { id: "tavily", name: "Tavily", data: snapshot.tavily },
      { id: "exa", name: "Exa", data: snapshot.exa },
    ];

    let totalProblemKeys = 0;
    const html = providerOrder.map((p) => {
      const configured = !!p.data?.configured;
      const running = !!p.data?.running;
      const keys = (p.data?.keys || []);

      if (!configured) {
        return `
          <div class="dash-key-provider">
            <div class="dash-key-provider-head">
              <div class="dash-key-provider-name">${escapeHtml(p.name)}</div>
              <div class="dash-key-provider-meta">${t("dash.notConfigured")}</div>
            </div>
          </div>
        `;
      }

      const stopped = keys.filter((k) => {
        const reason = String(k.disabledReason || "").toLowerCase();
        return !!k.isDisabled && reason === "account_deactivated";
      });
      const disabled = keys.filter((k) => {
        const reason = String(k.disabledReason || "").toLowerCase();
        return !!k.isDisabled && reason !== "account_deactivated";
      });
      const cooling = keys.filter((k) => !!k.isCoolingDown && !k.isDisabled);
      const available = keys.filter((k) => !k.isDisabled && !k.isCoolingDown);
      const unverified = keys.filter((k) => {
        const state = String(k.verificationState || "").toLowerCase();
        return state === "unknown" && !k.isDisabled && !k.isCoolingDown;
      });
      const problems = [...disabled, ...stopped, ...cooling];
      totalProblemKeys += problems.length;

      const verifiedAvailableCount = Math.max(0, available.length - unverified.length);
      const meta = `${verifiedAvailableCount} ${t("dash.availableKeys")} · ${unverified.length} ${t("dash.unverifiedKeys")} · ${cooling.length} ${t("dash.coolingDown")} · ${stopped.length} ${t("dash.stoppedKeys")} · ${disabled.length} ${t("dash.disabledKeys")}${running ? "" : ` · ${t("dash.proxyStopped")}`}`;

      const chips = (problems.length || unverified.length)
        ? `<div class="dash-key-chips">
            ${disabled.slice(0, 50).map((k) => {
              const reasonLabel = localizeDisableReason(k.disabledReason);
              const detailText = k.disabledReasonDetail ? localizeUsageError(k.disabledReasonDetail) : "";
              let titleText = reasonLabel;
              if (detailText && detailText !== reasonLabel) {
                titleText = titleText ? `${titleText} — ${detailText}` : detailText;
              }
              const title = titleText ? ` title="${escapeHtml(titleText)}"` : "";
              return `<span class="dash-key-chip disabled"${title}>${escapeHtml(k.keyPreview)}</span>`;
            }).join("")}
            ${stopped.slice(0, 50).map((k) => {
              const detailText = k.disabledReasonDetail ? localizeUsageError(k.disabledReasonDetail) : "";
              let titleText = t("dash.stoppedKeys");
              if (detailText && detailText !== titleText) {
                titleText = `${titleText} — ${detailText}`;
              }
              const title = titleText ? ` title="${escapeHtml(titleText)}"` : "";
              return `<span class="dash-key-chip stopped"${title}>${escapeHtml(k.keyPreview)}</span>`;
            }).join("")}
            ${cooling.slice(0, 50).map((k) => {
              const secs = Number(k.cooldownRemainingSecs || 0);
              const title = secs > 0 ? ` title="${escapeHtml(String(secs))}s"` : "";
              return `<span class="dash-key-chip cooling"${title}>${escapeHtml(k.keyPreview)}${secs > 0 ? ` (${escapeHtml(String(secs))}s)` : ""}</span>`;
            }).join("")}
            ${unverified.slice(0, 50).map((k) => {
              const title = ` title="${escapeHtml(t("key.unverified"))}"`;
              return `<span class="dash-key-chip unverified"${title}>${escapeHtml(k.keyPreview)}</span>`;
            }).join("")}
          </div>`
        : `<div class="dash-key-empty">${t("dash.noProblemKeys")}</div>`;

      return `
        <div class="dash-key-provider">
          <div class="dash-key-provider-head">
            <div class="dash-key-provider-name">${escapeHtml(p.name)}</div>
            <div class="dash-key-provider-meta">${escapeHtml(meta)}</div>
          </div>
          ${chips}
        </div>
      `;
    }).join("");

    overviewEl.innerHTML = html;
    if (!this._keyDetailsInitialized) {
      detailsEl.open = totalProblemKeys > 0;
      this._keyDetailsInitialized = true;
    }
  },

  _renderUsage() {
    const gridEl = document.getElementById("dashUsageGrid");
    const updatedEl = document.getElementById("dashUsageUpdated");
    if (!gridEl || !updatedEl) return;

    const lastUpdatedTs = Math.max(
      this._usageByProvider.firecrawl?.fetchedAt || 0,
      this._usageByProvider.tavily?.fetchedAt || 0,
      this._usageByProvider.exa?.fetchedAt || 0
    );
    updatedEl.textContent = `${t("dash.lastUpdated")}: ${formatTsDisplay(lastUpdatedTs)}`;
    const providers = [
      { id: "firecrawl", name: t("dash.firecrawlUsage"), value: this._usageByProvider.firecrawl },
      { id: "tavily", name: t("dash.tavilyUsage"), value: this._usageByProvider.tavily },
      { id: "exa", name: t("dash.exaUsage"), value: this._usageByProvider.exa },
    ];

    gridEl.innerHTML = providers.map((provider) => {
      const value = provider.value || {};
      const ok = !!value.ok;
      const configured = !!value.configured;
      const hasEnabledKey = !!value.hasEnabledKey;
      const cardClass = ok ? "provider-ok" : "provider-error";
      const estimate = this._estimateUsage(provider.id, value);
      const unitText = localizeUsageUnit(estimate.unit);
      const secondaryUnitText = localizeUsageUnit(estimate.secondaryUnit);
      const partial = parsePartialKeyCounts(value.summary);
      const showLogs = configured && hasEnabledKey && !ok && !!value.error;
      const summary = (() => {
        if (!configured) return t("dash.notConfigured");
        if (!hasEnabledKey) return t("dash.notAvailable");
        if (!ok && value.error) return localizeUsageError(value.error);

        const remaining = typeof estimate.remaining === "number" ? estimate.remaining : null;
        const used = typeof estimate.used === "number" ? estimate.used : null;
        const limit = typeof estimate.limit === "number" ? estimate.limit : null;

        let base = "";
        if (typeof remaining === "number") {
          base = `${t("dash.remaining")} ${formatMetricValue(remaining)}${unitText ? ` ${unitText}` : ""}`;
        } else if (typeof used === "number" && typeof limit === "number") {
          base = `${t("dash.used")} ${formatMetricValue(used)} / ${formatMetricValue(limit)}${unitText ? ` ${unitText}` : ""}`;
        } else if (typeof used === "number") {
          base = `${t("dash.used")} ${formatMetricValue(used)}${unitText ? ` ${unitText}` : ""}`;
        } else {
          base = t("dash.notAvailable");
        }

        if (partial) {
          base += currentLang === "zh"
            ? ` (${partial.ok}/${partial.total})`
            : ` (${partial.ok}/${partial.total} keys)`;
        }
        return base;
      })();
      const refreshing = this._refreshingProvider.has(provider.id);
      const metric = this._metricFor(provider.id);
      const todayReq = Number(metric.requestCount || 0);
      const todayRetry = Number(metric.retryCount || 0);

      return `
        <div class="provider-card ${cardClass}">
          <div class="provider-head">
            <div class="provider-name">${provider.name}</div>
            <button
              type="button"
              class="btn btn-sm usage-refresh-btn"
              data-provider="${provider.id}"
              ${refreshing ? "disabled" : ""}
            >${refreshing ? t("dash.refreshing") : t("dash.manualRefresh")}</button>
          </div>
          <div class="provider-summary">${escapeHtml(summary)}${showLogs ? ` · <a href=\"#\" class=\"usage-open-logs\">${t("dash.viewLogs")}</a>` : ""}</div>
          <div class="provider-divider"></div>
          <div class="provider-metrics">
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.used")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.used)}${unitText ? ` ${unitText}` : ""}</span>
            </div>
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.limit")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.limit)}${unitText ? ` ${unitText}` : ""}</span>
            </div>
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.remaining")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.remaining)}${unitText ? ` ${unitText}` : ""}</span>
            </div>
          </div>
          ${(typeof estimate.secondaryUsed === "number" || typeof estimate.secondaryLimit === "number" || typeof estimate.secondaryRemaining === "number") ? `
          <div class="provider-metrics provider-metrics-secondary">
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.used")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.secondaryUsed)}${secondaryUnitText ? ` ${secondaryUnitText}` : ""}</span>
            </div>
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.limit")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.secondaryLimit)}${secondaryUnitText ? ` ${secondaryUnitText}` : ""}</span>
            </div>
            <div class="provider-metric">
              <span class="provider-metric-label">${t("dash.remaining")}</span>
              <span class="provider-metric-value">${formatMetricValue(estimate.secondaryRemaining)}${secondaryUnitText ? ` ${secondaryUnitText}` : ""}</span>
            </div>
          </div>
          ` : ""}
          ${estimate.deltaRequests > 0 ? `<div class="provider-note">${t("dash.estimated")} +${estimate.deltaRequests} ${t("dash.requests")}</div>` : ""}
          <div class="provider-today">
            <strong>${formatMetricValue(todayReq)}</strong> ${t("dash.requests")}${todayRetry > 0 ? ` · <strong>${formatMetricValue(todayRetry)}</strong> ${t("dash.todayRetries")}` : ""}
          </div>
        </div>
      `;
    }).join("");
  },

  _renderTodayStats() {
    const totalEl = document.getElementById("dashTodayTotal");
    const retriesEl = document.getElementById("dashTodayRetries");
    if (!totalEl || !retriesEl) return;

    const providerIds = ["firecrawl", "tavily", "exa"];
    const totalRequests = providerIds.reduce((sum, p) => sum + Number(this._metricFor(p).requestCount || 0), 0);
    const totalRetries = providerIds.reduce((sum, p) => sum + Number(this._metricFor(p).retryCount || 0), 0);

    totalEl.textContent = formatMetricValue(totalRequests);
    retriesEl.textContent = formatMetricValue(totalRetries);
  },

  destroy() {
    if (this._timer) { clearInterval(this._timer); this._timer = null; }
    if (this._usageTimer) { clearInterval(this._usageTimer); this._usageTimer = null; }
    if (this._persistTimer) {
      clearTimeout(this._persistTimer);
      this._persistTimer = null;
    }
    void this._flushPersistDashboardState();
    contentEl.removeEventListener("click", this._onUsageClick);
  },
};

// ============================================
// 5. PAGE: Configuration
// ============================================
pages.config = {
  _savedConfig: null,
  _savedLaunchOnLogin: false,
  _keySnapshot: null,
  _providerRows: {
    firecrawl: [],
    tavily: [],
    exa: [],
  },
  _rowSeed: 0,

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
        <div class="form-group">
          <label class="form-label">${t("cfg.exaUpstreamUrl")} <span class="form-hint">EXA_UPSTREAM_BASE_URL</span></label>
          <input id="cfgExaUpstreamUrl" class="form-input" type="url" placeholder="https://api.exa.ai" />
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
        <div class="form-row">
          <div class="form-group">
            <label class="form-label">${t("cfg.tavilyPort")} <span class="form-hint">TAVILY_PORT</span></label>
            <input id="cfgTavilyPort" class="form-input" type="number" min="1" max="65535" />
          </div>
          <div class="form-group">
            <label class="form-label">${t("cfg.exaPort")} <span class="form-hint">EXA_PORT</span></label>
            <input id="cfgExaPort" class="form-input" type="number" min="1" max="65535" />
          </div>
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
        <p class="form-note">${t("cfg.disableByToggle")}</p>
        ${["firecrawl", "tavily", "exa"].map((p) => `
        <div class="cfg-accordion" data-provider="${p}">
          <div class="cfg-accordion-header" data-provider="${p}">
            <svg class="cfg-accordion-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
            <span class="cfg-accordion-title">${p.charAt(0).toUpperCase() + p.slice(1)}</span>
            <span class="cfg-accordion-summary" id="cfgSummary_${p}"></span>
            <span class="cfg-accordion-actions">
              <button type="button" class="btn btn-sm cfg-key-add-btn" data-provider="${p}">${t("cfg.addKey")}</button>
              <button type="button" class="btn btn-sm cfg-key-batch-btn" data-provider="${p}">${t("cfg.batchAdd")}</button>
              <span class="cfg-view-toggle" data-provider="${p}" data-view="list">
                <span class="cfg-view-opt active" data-v="list">${t("cfg.viewList")}</span>
                <span class="cfg-view-opt" data-v="text">${t("cfg.viewText")}</span>
              </span>
            </span>
          </div>
          <div class="cfg-accordion-body" id="cfgAccBody_${p}">
            <div class="cfg-batch-bar" id="cfgBatchBar_${p}">
              <label class="cfg-batch-select-all"><input type="checkbox" class="cfg-select-all-cb" data-provider="${p}" /> ${t("cfg.selectAll")}</label>
              <span class="cfg-batch-actions" id="cfgBatchActions_${p}" style="visibility:hidden;pointer-events:none;">
                <button type="button" class="btn btn-sm cfg-batch-enable-btn" data-provider="${p}">${t("cfg.batchEnable")}</button>
                <button type="button" class="btn btn-sm cfg-batch-disable-btn" data-provider="${p}">${t("cfg.batchDisable")}</button>
                <button type="button" class="btn btn-sm btn-danger cfg-batch-remove-btn" data-provider="${p}">${t("cfg.batchRemove")}</button>
              </span>
            </div>
            <div id="cfgKeyView_${p}" class="cfg-key-view"></div>
          </div>
        </div>
        `).join("")}
      </div>

      <div class="card">
        <div class="card-header">${t("cfg.systemSettings")}</div>
        <label class="toggle-row" for="cfgAutoStart">
          <span>${t("cfg.autoStart")}</span>
          <input id="cfgAutoStart" type="checkbox" />
        </label>
        <p class="form-note">${t("cfg.autoStartHint")}</p>

        <label class="toggle-row" for="cfgSilentStart">
          <span>${t("cfg.silentStart")}</span>
          <input id="cfgSilentStart" type="checkbox" />
        </label>
        <p class="form-note">${t("cfg.silentStartHint")}</p>

        <label class="toggle-row" for="cfgLaunchOnLogin">
          <span>${t("cfg.launchOnLogin")}</span>
          <input id="cfgLaunchOnLogin" type="checkbox" />
        </label>
        <p class="form-note">${t("cfg.launchOnLoginHint")}</p>
      </div>

      <div class="config-actions">
        <button id="cfgReloadBtn" class="btn">${t("cfg.reload")}</button>
        <button id="cfgSaveBtn" class="btn btn-primary">${t("cfg.save")}</button>
        <span id="cfgDirtyBadge" class="dirty-badge">${t("cfg.unsaved")}</span>
      </div>
    `;
  },

  async init() {
    try {
      await this._reloadFromStorage();
    } catch (e) {
      showToast(t("cfg.loadFailed") + e, "error");
    }

    const inputs = [
      "cfgProxyToken",
      "cfgUpstreamUrl",
      "cfgTavilyUpstreamUrl",
      "cfgExaUpstreamUrl",
      "cfgHost",
      "cfgPort",
      "cfgTavilyPort",
      "cfgExaPort",
      "cfgTimeout",
      "cfgCooldown",
    ];
    inputs.forEach((id) => {
      const el = document.getElementById(id);
      if (el) el.addEventListener("input", () => this._checkDirty());
    });
    const autoStartEl = document.getElementById("cfgAutoStart");
    if (autoStartEl) autoStartEl.addEventListener("change", () => this._checkDirty());
    const silentStartEl = document.getElementById("cfgSilentStart");
    if (silentStartEl) silentStartEl.addEventListener("change", () => this._checkDirty());
    const launchOnLoginEl = document.getElementById("cfgLaunchOnLogin");
    if (launchOnLoginEl) launchOnLoginEl.addEventListener("change", () => this._checkDirty());
    contentEl.addEventListener("click", this._onClick);
    contentEl.addEventListener("input", this._onInput);
    contentEl.addEventListener("change", this._onChange);

    document.getElementById("cfgReloadBtn").addEventListener("click", async () => {
      const btn = document.getElementById("cfgReloadBtn");
      const isDirty = document.getElementById("cfgDirtyBadge")?.classList.contains("visible");
      if (isDirty) {
        const ok = await uiConfirm(t("cfg.reloadConfirm"), { title: t("cfg.reload") });
        if (!ok) return;
      }
      setLoading(btn, true);
      try {
        await this._reloadFromStorage();
        showToast(t("cfg.reloaded"), "success");
      } catch (e) {
        showToast(t("cfg.loadFailed") + e, "error");
      }
      setLoading(btn, false);
    });

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

  _newRow(key = "", enabled = true) {
    this._rowSeed += 1;
    return { id: `row-${this._rowSeed}`, key, enabled };
  },

  _providerViewMode: { firecrawl: "list", tavily: "list", exa: "list" },
  _providerSelected: { firecrawl: new Set(), tavily: new Set(), exa: new Set() },
  _providerOpen: { firecrawl: true, tavily: false, exa: false },

  _providerListId(provider) {
    return `cfgKeyView_${provider}`;
  },

  _updateSummary(provider) {
    const el = document.getElementById(`cfgSummary_${provider}`);
    if (!el) return;
    const rows = this._providerRows[provider] || [];
    const total = rows.length;
    const disabled = rows.filter((r) => !r.enabled).length;
    if (!total) { el.textContent = t("cfg.noKeysAccordion"); return; }
    if (disabled > 0) {
      el.textContent = t("cfg.keySummary").replace("${total}", total).replace("${disabled}", disabled);
    } else {
      el.textContent = t("cfg.keySummaryAll").replace("${total}", total);
    }
  },

  _renderProviderList(provider) {
    const viewEl = document.getElementById(this._providerListId(provider));
    if (!viewEl) return;
    const rows = this._providerRows[provider] || [];
    this._updateSummary(provider);
    this._providerSelected[provider] = new Set();
    this._updateBatchBar(provider);

    if (this._providerViewMode[provider] === "text") {
      this._renderTextMode(provider, viewEl, rows);
    } else {
      this._renderListMode(provider, viewEl, rows);
    }
  },

  _renderListMode(provider, viewEl, rows) {
    if (!rows.length) {
      viewEl.innerHTML = `<div class="cfg-key-empty">${t("cfg.noKeys")}</div>`;
      return;
	    }
	    viewEl.innerHTML = `<div class="cfg-key-compact-list">${rows.map((row, idx) => {
	      const status = this._keySnapshot?.[provider]?.keys?.[idx];
	      const verificationState = String(status?.verificationState || "").toLowerCase();
	      const isUnverified = row.enabled && verificationState === "unknown";
	      const isDisabled = !row.enabled && !!status?.isDisabled;
	      const disabledReason = String(status?.disabledReason || "").toLowerCase();
	      const isStopped = isDisabled && disabledReason === "account_deactivated";
	      const reasonLabel = isDisabled
	        ? (isStopped ? t("dash.stoppedKeys") : localizeDisableReason(status?.disabledReason))
	        : (isUnverified ? t("key.unverified") : "");
	      const detailText = isDisabled && status?.disabledReasonDetail ? localizeUsageError(status.disabledReasonDetail) : "";
	      const tooltip = detailText || reasonLabel;
	      const reasonClass = isUnverified ? "cfg-row-reason muted" : "cfg-row-reason";
	      const reasonHtml = reasonLabel
	        ? `<span class="${reasonClass}" title="${escapeHtml(tooltip)}">${escapeHtml(reasonLabel)}</span>`
	        : "";
	      return `
	      <div class="cfg-key-row" data-provider="${provider}" data-row-id="${row.id}">
	        <label class="cfg-row-cb-wrap"><input type="checkbox" class="cfg-row-cb" data-row-id="${row.id}" /></label>
        <div class="cfg-row-key ${row.enabled ? "" : "cfg-row-key-disabled"}" title="${escapeHtml(row.key || "")}">${escapeHtml(row.key ? (row.key.length > 20 ? row.key.slice(0, 8) + "..." + row.key.slice(-8) : row.key) : "")}</div>
        ${reasonHtml}
        <div class="cfg-key-switch-wrap">
          <label class="cfg-key-switch">
            <input type="checkbox" class="cfg-key-enable-switch" ${row.enabled ? "checked" : ""} />
            <span class="cfg-key-switch-track"><span class="cfg-key-switch-thumb"></span></span>
          </label>
        </div>
        <button type="button" class="cfg-row-remove cfg-key-remove-btn" title="${t("cfg.remove")}">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
    `;
    }).join("")}</div>`;
  },

  _renderTextMode(provider, viewEl, rows) {
    const text = rows.map((row) => (row.enabled ? "" : "# ") + (row.key || "")).join("\n");
    viewEl.innerHTML = `
      <div class="cfg-text-mode">
        <div class="cfg-text-hint">${t("cfg.textModeHint")}</div>
        <textarea class="cfg-text-area" data-provider="${provider}" spellcheck="false">${escapeHtml(text)}</textarea>
      </div>`;
  },

  _parseTextMode(provider) {
    const ta = document.querySelector(`.cfg-text-area[data-provider="${provider}"]`);
    if (!ta) return;
    const lines = ta.value.split("\n");
    const newRows = [];
    lines.forEach((line) => {
      const trimmed = line.trim();
      if (!trimmed) return;
      const disabled = trimmed.startsWith("#");
      const key = disabled ? trimmed.slice(1).trim() : trimmed;
      if (key) newRows.push(this._newRow(key, !disabled));
    });
    this._providerRows[provider] = newRows;
    this._updateSummary(provider);
    this._checkDirty();
  },

  _updateBatchBar(provider) {
    const bar = document.getElementById(`cfgBatchBar_${provider}`);
    if (!bar) return;
    const rows = this._providerRows[provider] || [];
    // Hide bar in text mode or when no keys
    const isText = this._providerViewMode[provider] === "text";
    bar.style.display = (isText || !rows.length) ? "none" : "flex";
    if (isText || !rows.length) return;

    const selected = this._providerSelected[provider];
    const actionsEl = document.getElementById(`cfgBatchActions_${provider}`);
    if (actionsEl) {
      actionsEl.style.visibility = selected.size > 0 ? "visible" : "hidden";
      actionsEl.style.pointerEvents = selected.size > 0 ? "auto" : "none";
    }
    const allCb = bar.querySelector(".cfg-select-all-cb");
    if (allCb) {
      const rows = this._providerRows[provider] || [];
      allCb.checked = rows.length > 0 && selected.size === rows.length;
      allCb.indeterminate = selected.size > 0 && selected.size < rows.length;
    }
  },

  _renderAllProviderLists() {
    ["firecrawl", "tavily", "exa"].forEach((p) => {
      this._renderProviderList(p);
      const acc = document.querySelector(`.cfg-accordion[data-provider="${p}"]`);
      if (acc) acc.classList.toggle("open", !!this._providerOpen[p]);
    });
  },

  _setProviderRowsFromConfig(provider, keys, disabledKeys) {
    const disabledSet = new Set(disabledKeys || []);
    const rows = (keys || []).map((key) => this._newRow(key, !disabledSet.has(key)));
    this._providerRows[provider] = rows.length ? rows : [];
  },

  _collectProviderRows(provider) {
    const rows = this._providerRows[provider] || [];
    const merged = new Map();
    rows.forEach((row) => {
      const key = (row.key || "").trim();
      if (!key) return;
      const enabled = !!row.enabled;
      if (!merged.has(key)) {
        merged.set(key, enabled);
      } else {
        merged.set(key, merged.get(key) || enabled);
      }
    });
    const keys = [...merged.keys()];
    const disabled = keys.filter((key) => !merged.get(key));
    return { keys, disabled };
  },

  _onClick: async (event) => {
    const page = pages.config;

    // Accordion toggle
    const accHeader = event.target.closest(".cfg-accordion-header");
    if (accHeader && !event.target.closest(".cfg-accordion-actions")) {
      const provider = accHeader.dataset.provider;
      if (provider) {
        page._providerOpen[provider] = !page._providerOpen[provider];
        const acc = accHeader.closest(".cfg-accordion");
        if (acc) acc.classList.toggle("open", page._providerOpen[provider]);
      }
      return;
    }

    // View toggle (list / text)
    const viewToggle = event.target.closest(".cfg-view-toggle");
    if (viewToggle) {
      const provider = viewToggle.dataset.provider;
      if (!provider) return;
      const currentView = page._providerViewMode[provider];
      const nextView = currentView === "list" ? "text" : "list";
      // Set mode before parsing to prevent recursion (_parseTextMode → _checkDirty → _readForm → _parseTextMode)
      page._providerViewMode[provider] = nextView;
      if (currentView === "text") {
        page._parseTextMode(provider);
      }
      viewToggle.querySelectorAll(".cfg-view-opt").forEach((o) => {
        o.classList.toggle("active", o.dataset.v === nextView);
      });
      page._renderProviderList(provider);
      return;
    }

    // Add key
    const addBtn = event.target.closest(".cfg-key-add-btn");
    if (addBtn) {
      const provider = addBtn.dataset.provider;
      if (!provider) return;
      const raw = await uiPrompt(t("cfg.addPrompt"), { title: t("cfg.addKey") });
      if (raw === null) return;
      const parsed = parseKeys(raw);
      if (!parsed.length) return;
      if (!page._providerRows[provider]) page._providerRows[provider] = [];
      parsed.forEach((key) => {
        page._providerRows[provider].push(page._newRow(key, true));
      });
      page._providerOpen[provider] = true;
      const acc = addBtn.closest(".cfg-accordion");
      if (acc) acc.classList.add("open");
      page._renderProviderList(provider);
      page._checkDirty();
      if (page._providerViewMode[provider] === "text") {
        setTimeout(() => {
          const ta = document.querySelector(`.cfg-text-area[data-provider="${provider}"]`);
          if (ta) { ta.focus(); ta.selectionStart = ta.selectionEnd = ta.value.length; }
        }, 50);
      }
      return;
    }

    // Batch import
    const batchBtn = event.target.closest(".cfg-key-batch-btn");
    if (batchBtn) {
      const provider = batchBtn.dataset.provider;
      if (!provider) return;
      const raw = await uiPrompt(t("cfg.batchPrompt"), { title: t("cfg.batchAdd"), multiline: true });
      if (raw === null) return;
      const parsed = parseKeys(raw);
      if (!parsed.length) return;
      if (!page._providerRows[provider]) page._providerRows[provider] = [];
      parsed.forEach((key) => {
        page._providerRows[provider].push(page._newRow(key, true));
      });
      page._providerOpen[provider] = true;
      const acc = batchBtn.closest(".cfg-accordion");
      if (acc) acc.classList.add("open");
      page._renderProviderList(provider);
      page._checkDirty();
      if (page._providerViewMode[provider] === "text") {
        setTimeout(() => {
          const ta = document.querySelector(`.cfg-text-area[data-provider="${provider}"]`);
          if (ta) { ta.focus(); ta.selectionStart = ta.selectionEnd = ta.value.length; }
        }, 50);
      }
      return;
    }

    // Remove single key
    const removeBtn = event.target.closest(".cfg-key-remove-btn");
    if (removeBtn) {
      const rowEl = removeBtn.closest(".cfg-key-row");
      if (!rowEl) return;
      const provider = rowEl.dataset.provider;
      const rowId = rowEl.dataset.rowId;
      if (!provider || !rowId) return;
      page._providerRows[provider] = (page._providerRows[provider] || []).filter((row) => row.id !== rowId);
      page._providerSelected[provider].delete(rowId);
      page._renderProviderList(provider);
      page._checkDirty();
      return;
    }

    // Batch enable
    const batchEnableBtn = event.target.closest(".cfg-batch-enable-btn");
    if (batchEnableBtn) {
      const provider = batchEnableBtn.dataset.provider;
      if (!provider) return;
      const sel = page._providerSelected[provider];
      (page._providerRows[provider] || []).forEach((r) => { if (sel.has(r.id)) r.enabled = true; });
      page._renderProviderList(provider);
      page._checkDirty();
      return;
    }

    // Batch disable
    const batchDisableBtn = event.target.closest(".cfg-batch-disable-btn");
    if (batchDisableBtn) {
      const provider = batchDisableBtn.dataset.provider;
      if (!provider) return;
      const sel = page._providerSelected[provider];
      (page._providerRows[provider] || []).forEach((r) => { if (sel.has(r.id)) r.enabled = false; });
      page._renderProviderList(provider);
      page._checkDirty();
      return;
    }

    // Batch remove
    const batchRemoveBtn = event.target.closest(".cfg-batch-remove-btn");
    if (batchRemoveBtn) {
      const provider = batchRemoveBtn.dataset.provider;
      if (!provider) return;
      const sel = page._providerSelected[provider];
      if (!sel.size) return;
      const msg = t("cfg.batchRemoveConfirm").replace("${count}", sel.size);
      const ok = await uiConfirm(msg, { title: t("cfg.batchRemove"), okClass: "btn-danger" });
      if (!ok) return;
      page._providerRows[provider] = (page._providerRows[provider] || []).filter((r) => !sel.has(r.id));
      page._renderProviderList(provider);
      page._checkDirty();
      return;
    }
  },

  _onInput: (event) => {
    // Text mode textarea
    const ta = event.target.closest(".cfg-text-area");
    if (ta) {
      const provider = ta.dataset.provider;
      if (provider) {
        pages.config._parseTextMode(provider);
      }
      return;
    }
  },

  _onChange: (event) => {
    const page = pages.config;

    // Select-all checkbox
    const selectAllCb = event.target.closest(".cfg-select-all-cb");
    if (selectAllCb) {
      const provider = selectAllCb.dataset.provider;
      if (!provider) return;
      const rows = page._providerRows[provider] || [];
      if (selectAllCb.checked) {
        page._providerSelected[provider] = new Set(rows.map((r) => r.id));
      } else {
        page._providerSelected[provider] = new Set();
      }
      const viewEl = document.getElementById(page._providerListId(provider));
      if (viewEl) viewEl.querySelectorAll(".cfg-row-cb").forEach((cb) => {
        cb.checked = selectAllCb.checked;
      });
      page._updateBatchBar(provider);
      return;
    }

    // Row checkbox
    const rowCb = event.target.closest(".cfg-row-cb");
    if (rowCb) {
      const rowEl = rowCb.closest(".cfg-key-row");
      if (!rowEl) return;
      const provider = rowEl.dataset.provider;
      const rowId = rowEl.dataset.rowId;
      if (!provider || !rowId) return;
      if (rowCb.checked) {
        page._providerSelected[provider].add(rowId);
      } else {
        page._providerSelected[provider].delete(rowId);
      }
      page._updateBatchBar(provider);
      return;
    }

    // Enable/disable toggle
    const toggle = event.target.closest(".cfg-key-enable-switch");
    if (!toggle) return;
    const rowEl = toggle.closest(".cfg-key-row");
    if (!rowEl) return;
    const provider = rowEl.dataset.provider;
    const rowId = rowEl.dataset.rowId;
    if (!provider || !rowId) return;
    const row = (page._providerRows[provider] || []).find((r) => r.id === rowId);
    if (!row) return;
    row.enabled = !!toggle.checked;
    page._renderProviderList(provider);
    page._checkDirty();
  },

  async _reloadFromStorage() {
    const config = await invoke("reload_proxy_config");
    const [launchOnLogin, keySnapshot] = await Promise.all([
      invoke("get_launch_on_login_enabled").catch(() => false),
      invoke("get_key_status_snapshot").catch(() => null),
    ]);
    this._savedConfig = config;
    this._savedLaunchOnLogin = !!launchOnLogin;
    this._keySnapshot = keySnapshot;
    this._writeForm({ ...config, launchOnLogin: this._savedLaunchOnLogin });
    this._checkDirty();
  },

  _readForm() {
    // Parse any open text mode areas before reading
    ["firecrawl", "tavily", "exa"].forEach((p) => {
      if (this._providerViewMode[p] === "text") this._parseTextMode(p);
    });
    const firecrawl = this._collectProviderRows("firecrawl");
    const tavily = this._collectProviderRows("tavily");
    const exa = this._collectProviderRows("exa");

    return {
      proxyToken: document.getElementById("cfgProxyToken").value.trim(),
      firecrawlApiKeys: firecrawl.keys,
      firecrawlDisabledApiKeys: firecrawl.disabled,
      upstreamBaseUrl: document.getElementById("cfgUpstreamUrl").value.trim(),
      tavilyApiKeys: tavily.keys,
      tavilyDisabledApiKeys: tavily.disabled,
      tavilyUpstreamBaseUrl: document.getElementById("cfgTavilyUpstreamUrl").value.trim(),
      exaApiKeys: exa.keys,
      exaDisabledApiKeys: exa.disabled,
      exaUpstreamBaseUrl: document.getElementById("cfgExaUpstreamUrl").value.trim(),
      requestTimeoutMs: Number(document.getElementById("cfgTimeout").value),
      keyCooldownSeconds: Number(document.getElementById("cfgCooldown").value),
      host: document.getElementById("cfgHost").value.trim(),
      port: Number(document.getElementById("cfgPort").value),
      tavilyPort: Number(document.getElementById("cfgTavilyPort").value),
      exaPort: Number(document.getElementById("cfgExaPort").value),
      autoStart: !!document.getElementById("cfgAutoStart").checked,
      silentStart: !!document.getElementById("cfgSilentStart").checked,
      launchOnLogin: !!document.getElementById("cfgLaunchOnLogin").checked,
    };
  },

  _writeForm(c) {
    document.getElementById("cfgProxyToken").value = c.proxyToken || "";
    document.getElementById("cfgUpstreamUrl").value = c.upstreamBaseUrl || "";
    document.getElementById("cfgTavilyUpstreamUrl").value = c.tavilyUpstreamBaseUrl || "";
    document.getElementById("cfgExaUpstreamUrl").value = c.exaUpstreamBaseUrl || "";
    document.getElementById("cfgHost").value = c.host || "127.0.0.1";
    document.getElementById("cfgPort").value = String(c.port || 8787);
    document.getElementById("cfgTavilyPort").value = String(c.tavilyPort || 8788);
    document.getElementById("cfgExaPort").value = String(c.exaPort || 8789);
    document.getElementById("cfgTimeout").value = String(c.requestTimeoutMs || 60000);
    document.getElementById("cfgCooldown").value = String(c.keyCooldownSeconds || 60);
    this._setProviderRowsFromConfig("firecrawl", c.firecrawlApiKeys || [], c.firecrawlDisabledApiKeys || []);
    this._setProviderRowsFromConfig("tavily", c.tavilyApiKeys || [], c.tavilyDisabledApiKeys || []);
    this._setProviderRowsFromConfig("exa", c.exaApiKeys || [], c.exaDisabledApiKeys || []);
    this._renderAllProviderLists();
    document.getElementById("cfgAutoStart").checked = c.autoStart ?? true;
    document.getElementById("cfgSilentStart").checked = c.silentStart ?? false;
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
      cur.exaUpstreamBaseUrl !== (saved.exaUpstreamBaseUrl || "") ||
      cur.host !== (saved.host || "127.0.0.1") ||
      cur.port !== (saved.port || 8787) ||
      cur.tavilyPort !== (saved.tavilyPort || 8788) ||
      cur.exaPort !== (saved.exaPort || 8789) ||
      cur.requestTimeoutMs !== (saved.requestTimeoutMs || 60000) ||
      cur.keyCooldownSeconds !== (saved.keyCooldownSeconds || 60) ||
      JSON.stringify(cur.firecrawlApiKeys) !== JSON.stringify(saved.firecrawlApiKeys || []) ||
      JSON.stringify(cur.firecrawlDisabledApiKeys) !== JSON.stringify(saved.firecrawlDisabledApiKeys || []) ||
      JSON.stringify(cur.tavilyApiKeys) !== JSON.stringify(saved.tavilyApiKeys || []) ||
      JSON.stringify(cur.tavilyDisabledApiKeys) !== JSON.stringify(saved.tavilyDisabledApiKeys || []) ||
      JSON.stringify(cur.exaApiKeys) !== JSON.stringify(saved.exaApiKeys || []) ||
      JSON.stringify(cur.exaDisabledApiKeys) !== JSON.stringify(saved.exaDisabledApiKeys || []) ||
      cur.autoStart !== (saved.autoStart ?? true) ||
      cur.silentStart !== (saved.silentStart ?? false) ||
      cur.launchOnLogin !== this._savedLaunchOnLogin;
    badge.classList.toggle("visible", dirty);
  },

  destroy() {
    contentEl.removeEventListener("click", this._onClick);
    contentEl.removeEventListener("input", this._onInput);
    contentEl.removeEventListener("change", this._onChange);
  },
};

// ============================================
// 6. PAGE: MCP Config
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
      const exaConfigured = isProviderConfigured(config, "exa");
      const options = [
        { value: "all", labelKey: "mcp.scopeAll", available: firecrawlConfigured || tavilyConfigured || exaConfigured },
        { value: "firecrawl", labelKey: "mcp.scopeFirecrawl", available: firecrawlConfigured },
        { value: "tavily", labelKey: "mcp.scopeTavily", available: tavilyConfigured },
        { value: "exa", labelKey: "mcp.scopeExa", available: exaConfigured },
      ];

      const optionLabel = (option) => {
        if (!option) return "";
        return option.available
          ? t(option.labelKey)
          : `${t(option.labelKey)} (${t("mcp.unavailable")})`;
      };

      if (firecrawlConfigured && tavilyConfigured && exaConfigured) {
        currentTarget = "all";
      } else if (firecrawlConfigured && tavilyConfigured) {
        currentTarget = "all";
      } else if (firecrawlConfigured) {
        currentTarget = "firecrawl";
      } else if (tavilyConfigured) {
        currentTarget = "tavily";
      } else if (exaConfigured) {
        currentTarget = "exa";
      } else {
        currentTarget = "all";
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
          if (el) el.innerHTML = `<div style="padding:12px;color:var(--color-danger)">${translateMcpError(String(e || ""))}</div>`;
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
// 7. PAGE: Logs
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
    } catch { }
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
// 8. BOOTSTRAP
// ============================================
async function bootstrap() {
  // Apply saved language on startup
  langToggle.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === currentLang);
  });

  // Set sidebar nav labels to current language
  const navLabels = { dashboard: "nav.dashboard", config: "nav.config", mcp: "nav.mcp", logs: "nav.logs" };
  sidebarNav.querySelectorAll(".nav-item").forEach((btn) => {
    const key = navLabels[btn.dataset.page];
    if (key) btn.querySelector("span").textContent = t(key);
  });

  if (globalProxyToggle) {
    globalProxyToggle.addEventListener("click", () => toggleProxyFromGlobal());
  }
  if (globalProxyLabel) {
    globalProxyLabel.textContent = t("global.proxy");
  }

  try {
    await updateSidebarStatus();
  } catch { }

  navigate("dashboard");

  globalTimer = setInterval(() => {
    updateSidebarStatus();
  }, 3000);
}

window.addEventListener("beforeunload", () => {
  if (globalTimer) clearInterval(globalTimer);
  try {
    pages.dashboard._persistDirty = true;
    void pages.dashboard._flushPersistDashboardState();
  } catch { }
});

bootstrap();
