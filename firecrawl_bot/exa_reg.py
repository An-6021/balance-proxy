import time
import re
from playwright.sync_api import sync_playwright
from playwright_stealth import Stealth
from mail_tm_utils import MailTM

def run_registration(headless=False, mail_factory=MailTM):
    with sync_playwright() as p:
        # 1. Initialize temp email API
        mail = mail_factory()
        if not mail.create_account():
            print("Failed to create temporary email.")
            return None
        
        email_addr = mail.address
        print(f"[+] 获取到有效邮箱: {email_addr}")

        # 启动浏览器
        browser = p.chromium.launch(
            headless=headless,
            args=['--disable-blink-features=AutomationControlled']
        )
        context = browser.new_context(
            user_agent='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36',
            viewport={'width': 1280, 'height': 800}
        )
        page = context.new_page()
        Stealth().apply_stealth_sync(page)
        
        print("[*] 正在打开 Exa Dashboard...")
        try:
            page.goto("https://dashboard.exa.ai/login", wait_until="networkidle")
        except Exception as e:
            print(f"[-] 页面加载失败: {e}")
            
        print("[*] 模仿人类输入邮箱...")
        try:
            # 等待 Auth0 或登录界面的邮箱输入框
            email_input = page.wait_for_selector('input[type="email"], input[name="email"]', timeout=15000)
            email_input.fill(email_addr)
            
            # 点击 Continue
            continue_btn = page.query_selector('button[type="submit"], button[name="action"]')
            if continue_btn:
                continue_btn.click()
            else:
                page.keyboard.press("Enter")
        except Exception as e:
            print(f"[-] 邮箱填写或提交失败: {e}")
            browser.close()
            return None

        print("[*] 注册提交完成，开始检查验证邮件...")

        # --- 步骤 3: 检查邮件并提取验证码 ---
        email_content = mail.wait_for_email(timeout=180, poll_interval=1.0)
        if not email_content:
            print("[!] 未能在 180s 内收到验证邮件。")
            browser.close()
            return None

        # 提取 6 位验证码
        code = None
        # 寻找诸如 123456 这样的 6 位数字
        codes = re.findall(r'\b\d{6}\b', email_content)
        if codes:
            # 邮件中可能只有 1 个 6 位数，或者最后一个是
            code = codes[-1]
            print(f"[+] 提取到验证码: {code}")
        else:
            print("[!] 邮件内容中未找到 6 位数字验证码。")
            with open("exa_debug_mail.html", "w") as f:
                f.write(email_content)
            browser.close()
            return None

        # --- 步骤 4: 填写验证码 ---
        try:
            print("[*] 尝试填写验证码...")
            # 根据页面 DOM 分析，存在一个确切的验证码输入框： 
            # <input placeholder="Enter verification code" maxlength="6" inputmode="numeric" pattern="\d{6}" ... type="text">
            code_input = page.wait_for_selector('input[placeholder="Enter verification code"]', timeout=15000)
            code_input.fill(code)
            
            # 等待 1 秒确认输入完成
            time.sleep(1)
            
            # 点击 VERIFY CODE 按钮
            verify_btn = page.query_selector('button[type="submit"], button[name="action"]')
            if verify_btn:
                verify_btn.click()
            else:
                page.keyboard.press("Enter")
        except Exception as e:
            print(f"[-] 填写验证码失败: {e}")
            page.screenshot(path="debug_exa_code_error.png")

        # --- 步骤 5: 处理跳过 Onboarding 并获取 API Key ---
        print("[*] 等待登录完成并重定向...")
        try:
            # 增加对 Auth0 返回 exa.ai 域名的显式等待
            page.wait_for_url("**/dashboard.exa.ai/**", timeout=20000)
            page.wait_for_load_state("networkidle", timeout=15000)
        except Exception as e:
            print(f"[*] 等待重定向超时或不需要重定向: {e}")
            
        # 尝试跳过 Onboarding (Subagent clicked X:675, Y:48 maybe a "Skip" button or simple link)
        skip_selectors = ['button:has-text("Skip")', 'a:has-text("Skip")']
        for sel in skip_selectors:
            skip_btn = page.query_selector(sel)
            if skip_btn and skip_btn.is_visible():
                print("[*] 点击跳过向导...")
                skip_btn.click()
                time.sleep(2)
                break

        api_key = "ERROR_EXTRACTING"
        try:
            print("[*] 在首页尝试显示和复制 API Key...")
            # 找到包含密码点点或 API Key 的行，并点击揭示按钮 (通常是眼睛图标)
            page.evaluate("""() => {
                // 点击页面上所有的眼睛图标（通常在 td 里的最后，或者 button）
                const buttons = document.querySelectorAll('button');
                buttons.forEach(btn => {
                    const html = btn.innerHTML.toLowerCase();
                    if (html.includes('svg') || html.includes('eye') || html.includes('reveal')) {
                        btn.click();
                    }
                });
            }""")
            time.sleep(1)

            # 提取 API Key (通常为 UUID 格式 aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee)
            for _ in range(10):
                val = page.evaluate("""() => {
                    // 查询包含 - 的很长的字符串，或者直接找内容
                    const elements = document.querySelectorAll('td, span, div, input');
                    for (const el of elements) {
                        const text = (el.value || el.textContent || "").trim();
                        // 匹配 UUID (8-4-4-4-12)
                        if (text.match(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i)) {
                            return text;
                        }
                    }
                    return null;
                }""")
                if val:
                    api_key = val
                    break
                time.sleep(0.5)

            if api_key == "ERROR_EXTRACTING":
                print("[!] 无法通过 DOM 直接查找到 UUID，尝试正则匹配全网页源码...")
                # Fallback: RegExp over the whole HTML content
                html_content = page.content()
                # Exa keys usually look like: cccc1234-abcd-4fge-b4ef-1234567890ab
                match = re.search(r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}', html_content)
                if match:
                    api_key = match.group(0)
                    print(f"[+] 正则匹配成功获取 API Key: {api_key}")
                else:
                    page.screenshot(path="exa_debug_api_key.png")
                    api_key = "WAITING_MANUAL"

        except Exception as e:
            print(f"[-] 提取 API Key 阶段发生错误: {e}")
            # 进行最后一次正则尝试，以防 evaluate 抛出 Execution Context Destroyed 但实际上加载出来了源码
            try:
                html_content = page.content()
                match = re.search(r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}', html_content)
                if match:
                    api_key = match.group(0)
                    print(f"[+] (异常兜底) 正则匹配成功获取 API Key: {api_key}")
            except:
                pass
                
            if api_key in ("ERROR_EXTRACTING", "WAITING_MANUAL"):
                page.screenshot(path="debug_exa_step5_error.png")
                api_key = "WAITING_MANUAL"

        browser.close()

        if api_key and api_key not in ("ERROR_EXTRACTING", "WAITING_MANUAL"):
            return {
                "email": email_addr,
                "api_key": api_key
            }
        else:
            return {
                "email": email_addr,
                "api_key": None
            }

if __name__ == "__main__":
    res = run_registration(headless=False)
    print(res)
