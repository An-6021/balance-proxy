import os
import sys
import subprocess
import time

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
}


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

    print("\n\033[1mWelcome to Multi-Bot Registration\033[0m")
    print("1. Firecrawl")
    print("2. Tavily")
    
    choice = input("\nSelect bot (1/2, default 1): ").strip()
    if choice == "2":
        from tavily_reg import run_registration as run_tavily
        bot_name = "Tavily"
        run_func = lambda: run_tavily(headless=False)
    else:
        from firecrawl_reg import run_registration as run_firecrawl
        bot_name = "Firecrawl"
        run_func = run_firecrawl

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
                password = result["password"]
                
                # Keep account credentials and API keys in separate files per bot.
                append_line(output_files["accounts"], f"{email}:{password}")
                append_line(output_files["keys"], api_key)
                
                print(f"\033[92mSUCCESS: {api_key}\033[0m")
            else:
                email = result.get("email", "Unknown") if result else "Unknown"
                password = result.get("password", "Unknown") if result else "Unknown"
                print(f"\033[91mFAILED to extract API key for {email}.\033[0m")
                append_line(output_files["failed"], f"{email}:{password}")
        except Exception as e:
            print(f"\033[91mCRITICAL ERROR: {e}\033[0m")
        
        if i < count - 1:
            print("Cooling down 10s...")
            time.sleep(10)

    print(f"\n\033[1m--- {bot_name} Batch Finished ---\033[0m")
    print(f"Total: {count} | Success: \033[92m{success_count}\033[0m")
    print(
        "Check "
        f"\033[94m{output_files['keys']}\033[0m and "
        f"\033[94m{output_files['accounts']}\033[0m"
    )

if __name__ == "__main__":
    main()
