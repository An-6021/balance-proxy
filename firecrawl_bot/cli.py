import os
import sys
import subprocess
import time

from local_config import LOCAL_CONFIG_PATH, load_local_config, save_local_config

BOT_OUTPUT_FILES = {
    "Firecrawl": {
        "accounts": "firecrawl_accounts.txt",
        "keys": "firecrawl_keys.txt",
        "failed": "firecrawl_accounts_failed.txt",
    },
    "Tavily": {
        "accounts": "tavily_accounts.txt",
        "keys": "tavily_keys.txt",
        "failed": "tavily_accounts_failed.txt",
    },
    "Exa": {
        "accounts": "exa_accounts.txt",
        "keys": "exa_keys.txt",
        "failed": "exa_accounts_failed.txt",
    },
}

BATCH_COOLDOWN_SECONDS = 3


def append_line(path, line):
    if not line.endswith("\n"):
        line += "\n"

    needs_newline = False
    if os.path.exists(path) and os.path.getsize(path) > 0:
        with open(path, "rb") as existing:
            existing.seek(-1, os.SEEK_END)
            needs_newline = existing.read(1) != b"\n"

    with open(path, "a", encoding="utf-8") as f:
        if needs_newline:
            f.write("\n")
        f.write(line)


def _get_local_config_str(local_config: dict, key: str, *, aliases=None, default: str = "") -> str:
    aliases = aliases or []
    candidates = [key, *aliases]
    for candidate in candidates:
        val = local_config.get(candidate)
        if isinstance(val, str) and val.strip():
            return val.strip()
    return default


def _ensure_tavily_protocol_ocr_config(local_config: dict) -> None:
    """
    Protocol 模式默认走 OCR；缺 key 时会导致落回手填验证码。
    这里在 CLI 入口提前做一次配置，避免跑到验证码环节才发现没配。
    """

    if (os.environ.get("TAVILY_PROTOCOL_OCR", "1") or "").strip() == "0":
        return

    default_model = "Qwen/Qwen3-VL-235B-A22B-Instruct"

    key = (os.environ.get("SILICON_FLOW_KEY") or "").strip() or _get_local_config_str(
        local_config,
        "SILICON_FLOW_KEY",
        aliases=["silicon_flow_key"],
    )
    model = (os.environ.get("OCR_MODEL") or "").strip() or _get_local_config_str(
        local_config,
        "OCR_MODEL",
        aliases=["ocr_model"],
        default=default_model,
    )

    if key:
        os.environ.setdefault("SILICON_FLOW_KEY", key)
        os.environ.setdefault("OCR_MODEL", model or default_model)
        return

    print("\n\033[1mTavily Protocol OCR (SiliconFlow)\033[0m")
    key_input = input("SiliconFlow API key (SILICON_FLOW_KEY) (press Enter to use manual captcha): ").strip()
    if not key_input:
        return

    model_input = input(f"OCR model (OCR_MODEL) (default {model or default_model}): ").strip()
    model_input = model_input or (model or default_model)

    os.environ["SILICON_FLOW_KEY"] = key_input
    os.environ["OCR_MODEL"] = model_input

    save_choice = input(f"Save OCR config to {LOCAL_CONFIG_PATH}? (Y/n): ").strip().lower()
    if save_choice in ("", "y", "yes"):
        local_config["SILICON_FLOW_KEY"] = key_input
        local_config["OCR_MODEL"] = model_input
        if save_local_config(local_config):
            print(f"[*] Saved OCR config to {LOCAL_CONFIG_PATH}")
        else:
            print(f"[!] Failed to save OCR config to {LOCAL_CONFIG_PATH}")


def check_env():
    print("\033[94m--- Environment Check ---\033[0m")
    all_ok = True
    try:
        import requests
        print("[v] requests installed.")
    except ImportError:
        print("[x] requests MISSING.")
        all_ok = False

    try:
        from PIL import Image  # noqa: F401
        print("[v] Pillow installed.")
    except ImportError:
        print("[x] Pillow MISSING.")
        all_ok = False
        
    try:
        from playwright.sync_api import sync_playwright
        print("[v] playwright installed.")
    except ImportError:
        print("[x] playwright MISSING.")
        all_ok = False

    try:
        import playwright_stealth
        print("[v] playwright-stealth installed.")
    except ImportError:
        print("[x] playwright-stealth MISSING.")
        all_ok = False

    if not all_ok:
        print("\n\033[91mPlease run: pip install -r requirements.txt\033[0m")
        return False

    # Check for playwright browsers
    try:
        with sync_playwright() as p:
            # Check if chromium is available by trying to launch with a short timeout
            browser = p.chromium.launch(headless=True)
            browser.close()
        print("[v] Chromium browser available.")
    except Exception as e:
        print(f"\033[93m[!] Chromium not found or error: {e}\033[0m")
        print("Running: playwright install chromium...")
        subprocess.run([sys.executable, "-m", "playwright", "install", "chromium"])
    
    return True

def main():
    if not check_env():
        return

    local_config = load_local_config()

    print("\n\033[1mSelect Email Provider\033[0m")
    print("1. mail.tm (default)")
    print("2. DuckMail (https://api.duckmail.sbs)")
    print("3. mail.gw (https://api.mail.gw)")
    print("4. InboxKitten (https://inboxkitten.com)")
    print("5. Self-host Mail API (MAIL_API_BASE_URL)")

    email_choice = input("\nSelect email provider (1/2/3/4/5, default 1): ").strip()
    mail_factory_kwargs = {}
    if email_choice == "2":
        from duckmail_utils import DuckMail

        saved_key = (local_config.get("DUCKMAIL_API_KEY") or local_config.get("duckmail_api_key") or "").strip()

        duckmail_api_key = (os.environ.get("DUCKMAIL_API_KEY") or saved_key or "").strip()
        if duckmail_api_key:
            duckmail_api_key = (
                input("DuckMail API key (optional, Enter to keep saved): ").strip()
                or duckmail_api_key
            )
        else:
            duckmail_api_key = input("DuckMail API key (optional, press Enter to skip): ").strip()

        if duckmail_api_key:
            if duckmail_api_key != saved_key:
                save_choice = input(f"Save DuckMail API key to {LOCAL_CONFIG_PATH}? (Y/n): ").strip().lower()
                if save_choice in ("", "y", "yes"):
                    local_config["DUCKMAIL_API_KEY"] = duckmail_api_key
                    if save_local_config(local_config):
                        print(f"[*] Saved config to {LOCAL_CONFIG_PATH}")
                    else:
                        print(f"[!] Failed to save config to {LOCAL_CONFIG_PATH}")

            mail_factory_kwargs["mail_factory"] = lambda: DuckMail(api_key=duckmail_api_key)
        else:
            mail_factory_kwargs["mail_factory"] = DuckMail
    elif email_choice == "3":
        from mail_gw_utils import MailGW
        mail_factory_kwargs["mail_factory"] = MailGW
    elif email_choice == "4":
        from inboxkitten_utils import InboxKitten
        mail_factory_kwargs["mail_factory"] = InboxKitten
    elif email_choice == "5":
        from mail_api_utils import MailAPI

        saved = local_config.get("mail_api") if isinstance(local_config.get("mail_api"), dict) else {}

        has_saved = bool(saved.get("base_url") and saved.get("admin_auth") and saved.get("domain"))
        if has_saved:
            use_saved = input(
                f"Use saved Mail API config ({saved.get('base_url')} / {saved.get('domain')})? (Y/n): "
            ).strip().lower()
        else:
            use_saved = "n"

        base_url = (
            os.environ.get("MAIL_API_BASE_URL")
            or os.environ.get("SELFMAIL_BASE_URL")
            or saved.get("base_url")
            or ""
        ).strip()
        if use_saved not in ("", "y", "yes"):
            if base_url:
                base_url = input(f"Mail API base url (Enter to keep {base_url}): ").strip() or base_url
            else:
                base_url = input("Mail API base url (e.g. https://your-worker): ").strip()

        admin_auth = (
            os.environ.get("MAIL_API_ADMIN_AUTH")
            or os.environ.get("SELFMAIL_ADMIN_AUTH")
            or saved.get("admin_auth")
            or ""
        ).strip()
        if use_saved not in ("", "y", "yes"):
            if admin_auth:
                admin_auth = (
                    input("Mail API admin auth (x-admin-auth) (Enter to keep saved): ").strip()
                    or admin_auth
                )
            else:
                admin_auth = input("Mail API admin auth (x-admin-auth): ").strip()

        domain = (
            os.environ.get("MAIL_API_DOMAIN")
            or os.environ.get("SELFMAIL_DOMAIN")
            or saved.get("domain")
            or ""
        ).strip()
        if use_saved not in ("", "y", "yes"):
            if domain:
                domain = input(f"Mail API domain (Enter to keep {domain}): ").strip() or domain
            else:
                domain = input("Mail API domain (e.g. example.com): ").strip()

        enable_prefix_raw = (os.environ.get("MAIL_API_ENABLE_PREFIX") or "").strip().lower()
        if enable_prefix_raw:
            enable_prefix = enable_prefix_raw not in ("0", "false", "no", "n", "off")
        else:
            enable_prefix = bool(saved.get("enable_prefix", True))
        if use_saved not in ("", "y", "yes"):
            enable_prefix_default = "Y" if enable_prefix else "n"
            enable_prefix_input = input(
                f"Mail API enablePrefix? (Y/n, default {enable_prefix_default}): "
            ).strip().lower()
            if enable_prefix_input in ("y", "yes"):
                enable_prefix = True
            elif enable_prefix_input in ("n", "no"):
                enable_prefix = False

        name = (os.environ.get("MAIL_API_NAME") or saved.get("name") or "bot").strip() or "bot"
        if use_saved not in ("", "y", "yes"):
            name = input(f"Mail API name (default {name}): ").strip() or name

        custom_auth = (os.environ.get("MAIL_API_CUSTOM_AUTH") or saved.get("custom_auth") or "").strip()
        if use_saved not in ("", "y", "yes"):
            if custom_auth:
                custom_auth = (
                    input("Mail API custom auth (optional, Enter to keep saved): ").strip() or custom_auth
                )
            else:
                custom_auth = input("Mail API custom auth (optional, press Enter to skip): ").strip()
        custom_auth = custom_auth or None

        if use_saved not in ("", "y", "yes"):
            save_choice = input(f"Save Mail API config to {LOCAL_CONFIG_PATH}? (Y/n): ").strip().lower()
            if save_choice in ("", "y", "yes"):
                local_config["mail_api"] = {
                    "base_url": base_url,
                    "admin_auth": admin_auth,
                    "domain": domain,
                    "enable_prefix": enable_prefix,
                    "name": name,
                    "custom_auth": custom_auth,
                }
                if save_local_config(local_config):
                    print(f"[*] Saved Mail API config to {LOCAL_CONFIG_PATH}")
                else:
                    print(f"[!] Failed to save config to {LOCAL_CONFIG_PATH}")

        mail_factory_kwargs["mail_factory"] = lambda: MailAPI(
            base_url=base_url,
            admin_auth=admin_auth,
            domain=domain,
            enable_prefix=enable_prefix,
            name=name,
            custom_auth=custom_auth,
        )

    print("\n\033[1mWelcome to Multi-Bot Registration\033[0m")
    print("1. Firecrawl")
    print("2. Tavily")
    print("3. Exa")
    
    choice = input("\nSelect bot (1/2/3, default 1): ").strip()
    if choice == "2":
        bot_name = "Tavily"
        print("\n\033[1mSelect Tavily registration mode\033[0m")
        print("1. Protocol (HTTP) + OCR captcha (default, fallback to manual)")
        print("2. Browser (Playwright, legacy)")
        tavily_mode = input("\nSelect mode (1/2, default 1): ").strip()
        if tavily_mode == "2":
            from tavily_reg import run_registration as run_tavily
            run_func = lambda: run_tavily(headless=False, **mail_factory_kwargs)
        else:
            _ensure_tavily_protocol_ocr_config(local_config)
            from tavily_protocol_reg import run_registration as run_tavily_protocol
            run_func = lambda: run_tavily_protocol(headless=False, **mail_factory_kwargs)
    elif choice == "3":
        from exa_reg import run_registration as run_exa
        bot_name = "Exa"
        run_func = lambda: run_exa(headless=False, **mail_factory_kwargs)
    else:
        from firecrawl_reg import run_registration as run_firecrawl
        bot_name = "Firecrawl"
        run_func = lambda: run_firecrawl(**mail_factory_kwargs)

    output_files = BOT_OUTPUT_FILES[bot_name]

    try:
        count_str = input(f"How many {bot_name} accounts? (default 1): ").strip()
        count = int(count_str) if count_str else 1
    except ValueError:
        print("Invalid number, defaulting to 1.")
        count = 1

    print(f"\nStarting registration of \033[92m{count}\033[0m {bot_name} account(s)...\n")
    
    success_count = 0
    for i in range(count):
        print(f"\033[95m--- Task {i+1}/{count} ---\033[0m")
        try:
            result = run_func()
            if result and result.get("api_key"):
                success_count += 1
                api_key = result["api_key"]
                email = result["email"]
                password = result.get("password", "")
                
                # Keep account credentials and API keys in separate files per bot.
                if password:
                    append_line(output_files["accounts"], f"{email}:{password}")
                else:
                    append_line(output_files["accounts"], f"{email}")
                append_line(output_files["keys"], api_key)
                
                print(f"\033[92mSUCCESS: {api_key}\033[0m")
            else:
                email = result.get("email", "Unknown") if result else "Unknown"
                print(f"\033[91mFAILED to extract API key for {email}.\033[0m")
        except Exception as e:
            print(f"\033[91mCRITICAL ERROR: {e}\033[0m")
        
        if i < count - 1:
            print(f"Cooling down {BATCH_COOLDOWN_SECONDS}s...")
            time.sleep(BATCH_COOLDOWN_SECONDS)

    print(f"\n\033[1m--- {bot_name} Batch Finished ---\033[0m")
    print(f"Total: {count} | Success: \033[92m{success_count}\033[0m")
    print(
        "Check "
        f"\033[94m{output_files['keys']}\033[0m and "
        f"\033[94m{output_files['accounts']}\033[0m"
    )

if __name__ == "__main__":
    main()
