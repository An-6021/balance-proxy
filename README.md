# Balance Proxy (Tauri)

桌面版本地代理（Rust + Tauri），支持：
- 多 Key 严格轮询
- `401/402/429` 自动切 key 重试
- Firecrawl `v1/*` 与 `v2/*` 透明转发
- Tavily 全路径透明转发
- 按已配置 provider 启动（双配置时同时拉起 Firecrawl / Tavily）
- 配置可视化编辑
- MCP 配置下拉选择（Firecrawl / Tavily / 两者）并复制

## 项目结构

- `src-tauri/`：Rust 代理内核 + Tauri 后端命令
- `ui/`：静态前端页面（配置编辑、启动停止、日志、复制配置）
- `firecrawl_bot/`：Firecrawl/Tavily 批量注册辅助脚本（独立于桌面代理）

## 运行（开发）

前置：
- Rust toolchain
- Tauri CLI（任选其一）
  - `cargo install tauri-cli --version '^2.0'`
  - 或 `npm i -g @tauri-apps/cli`

启动：

```bash
./dev
```

## 打包（macOS 双击即用）

```bash
./pkg
```

产物示例：
- `src-tauri/target/release/bundle/macos/Balance Proxy.app`
- `src-tauri/target/release/bundle/dmg/*.dmg`

终端用户只需要 `.app` 或 `.dmg`，不需要 Python/Node/Rust 运行时依赖。

## 配置与控制

在桌面 UI 中可编辑：
- `PROXY_TOKEN`
- `FIRECRAWL_API_KEYS`
- `UPSTREAM_BASE_URL`
- `TAVILY_API_KEYS`
- `TAVILY_UPSTREAM_BASE_URL`
- `REQUEST_TIMEOUT_MS`
- `KEY_COOLDOWN_SECONDS`
- `HOST` / `PORT` / `TAVILY_PORT`

保存后点击“启动代理”生效（会仅启动已完整配置的 provider）。

配置文件会保存在系统应用数据目录（macOS 下对应 `~/Library/Application Support/...`）。

## 一键复制 MCP 配置

应用内可通过下拉选择复制 Firecrawl / Tavily / 两者配置。若两者均已配置，`both` 结构示例：

```json
{
  "mcpServers": {
    "firecrawl": {
      "command": "npx",
      "args": ["-y", "firecrawl-mcp"],
      "env": {
        "FIRECRAWL_API_URL": "http://127.0.0.1:8787",
        "FIRECRAWL_API_KEY": "your-local-token"
      }
    },
    "tavily": {
      "command": "node",
      "args": ["/Users/you/Library/Application Support/com.balance.proxy/tavily-local-proxy-mcp.mjs"],
      "env": {
        "TAVILY_API_URL": "http://127.0.0.1:8788",
        "TAVILY_API_KEY": "your-local-token"
      }
    }
  }
}
```

其中：
- `FIRECRAWL_API_URL` 指向本地代理地址
- `FIRECRAWL_API_KEY` 使用你设置的 `PROXY_TOKEN`
- Tavily 使用应用生成的本地脚本 `tavily-local-proxy-mcp.mjs`（位于应用数据目录）以确保请求走本地代理
- `TAVILY_API_URL` 指向 Tavily 本地代理地址
- `TAVILY_API_KEY` 同样使用 `PROXY_TOKEN`

## firecrawl_bot 说明

`firecrawl_bot` 目录提供 Firecrawl/Tavily 的自动化注册脚本，和桌面代理主程序解耦，可独立运行。

运行方式：

```bash
cd firecrawl_bot
python3 -m pip install -r requirements.txt
python3 -m playwright install chromium
python3 cli.py
```

产出文件（本地使用，不提交 Git）：
- `firecrawl_accounts.txt`
- `firecrawl_keys.txt`
- `firecrawl_accounts_failed.txt`
- `tavily_accounts.txt`
- `tavily_keys.txt`
- `tavily_accounts_failed.txt`
