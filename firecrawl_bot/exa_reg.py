import time
import re
import random
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
            email_input.click()
            # 逐字输入模拟人类 (50ms - 150ms 延迟)
            for char in email_addr:
                page.keyboard.type(char, delay=random.randint(50, 150))
            
            time.sleep(random.uniform(0.5, 1.2)) # 输入完停顿一下
            
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
        
        # 增加随机等待，模拟查看邮件的间隔
        time.sleep(random.uniform(2.0, 5.0))

        # --- 步骤 3: 检查邮件并提取验证码 ---
        email_content = mail.wait_for_email(timeout=180, poll_interval=1.0)
        if not email_content:
            print("[!] 未能在 180s 内收到验证邮件。")
            browser.close()
            return None

        # 提取 6 位验证码
        code = None
        codes = re.findall(r'\b\d{6}\b', email_content)
        if codes:
            code = codes[-1]
            print(f"[+] 提取到验证码: {code}")
        else:
            print("[!] 邮件内容中未找到 6 位数字验证码。")
            browser.close()
            return None

        # --- 步骤 4: 填写验证码 ---
        try:
            print("[*] 尝试填写验证码...")
            code_input = page.wait_for_selector('input[placeholder="Enter verification code"]', timeout=15000)
            code_input.click()
            # 同样逐字输入验证码
            for char in code:
                page.keyboard.type(char, delay=random.randint(100, 300))
            
            time.sleep(random.uniform(0.8, 1.5))
            
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
            
        # --- 严格按照用户提供的 3 步截图处理 Onboarding ---
        print("[*] 开始执行严格的引导流程 (确保 $10 赠金)...")
        api_key = "ERROR_EXTRACTING"
        
        try:
            # 增加一些初始加载等待
            time.sleep(random.uniform(2.0, 3.5))

            # --- 页面 1 ---
            print("[*] 正在处理 Page 1: 选择环境配置...")
            page.wait_for_selector('text="Create your setup prompt"', timeout=15000)
            time.sleep(random.uniform(1.0, 2.0)) # 假装看一眼标题

            print("[*] 选择: Cursor")
            page.locator('button, div').filter(has_text=re.compile(r'^Cursor$', re.I)).first.click()
            time.sleep(random.uniform(0.6, 1.2))

            print("[*] 选择: Python")
            page.locator('button, div').filter(has_text=re.compile(r'^Python$', re.I)).first.click()
            time.sleep(random.uniform(0.6, 1.2))

            print("[*] 选择: Web search tool")
            page.locator('button, div').filter(has_text=re.compile(r'^Web search tool$', re.I)).first.click()
            time.sleep(random.uniform(1.0, 2.0))

            print("[*] 点击: Next")
            page.locator('button:has-text("Next")').first.click()
            
            # --- 页面 2 ---
            print("[*] 正在处理 Page 2: 配置搜索选项...")
            page.wait_for_selector('text="Configure your search"', timeout=15000)
            time.sleep(random.uniform(1.5, 3.0)) # 模拟人类阅读

            print("[*] 选择: Full text")
            full_text_card = page.locator('div').filter(has_text=re.compile(r'^Full text$', re.I)).first
            if full_text_card.is_visible():
                full_text_card.click()
                time.sleep(random.uniform(0.8, 1.5))

            print("[*] 点击: Generate Code")
            page.locator('button:has-text("Generate Code")').first.click()
            time.sleep(random.uniform(2.0, 4.0)) # 生成代码需要时间，人类也会等
            
            # --- 页面 3: 提取密钥 ---
            print("[*] 正在处理 Page 3: 提取密钥...")
            try:
                # 不再强求 URL 匹配，直接等内容。URL 变化在 SPA 中可能不触发常规 navigation
                page.wait_for_selector('text="You\'re all set!"', timeout=20000)
            except:
                print("[!] 未检测到 'You're all set!'，尝试直接寻找 API Key 区域...")
            
            time.sleep(random.uniform(1.5, 2.5))

            # 赠金检测
            if page.locator('text="You received $10 in free credits!"').is_visible():
                print("[+] 成功检测到 $10 赠金到账提示！")

            # 1. 揭示 API Key
            print("[*] 尝试点击眼睛图标以揭示 API Key...")
            # 更加精确的眼睛图标选择：它通常是 "Your API Key" 标签所在容器内的第一个按钮
            eye_btn = page.locator('button:has(svg)').filter(has_not_text=re.compile(r'Copy|Back|Dashboard', re.I)).first
            # 或者尝试通过相邻文本定位
            if not eye_btn.is_visible():
                eye_btn = page.locator('div:has-text("Your API Key") >> button').first

            if eye_btn.is_visible():
                eye_btn.hover()
                time.sleep(random.uniform(0.3, 0.6))
                eye_btn.click()
                print("[*] 已点击揭示按钮")
                time.sleep(random.uniform(1.2, 2.0))



            # 2. 提取 API Key (UUID 格式)
            # 通过全页面源码正则匹配是最稳妥的
            html_content = page.content()
            match = re.search(r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}', html_content)
            if match:
                api_key = match.group(0)
                print(f"[+] 成功提取到 API Key: {api_key}")
            else:
                print("[!] DOM 中未发现 UUID，尝试从只读 input 提取...")
                api_key = page.evaluate("""() => {
                    const el = document.querySelector('input[readonly]');
                    return el ? el.value : null;
                }""")
            
            # 3. 点击 "Go to Dashboard" 按钮
            print("[*] 完成！点击 Go to Dashboard...")
            go_btn = page.locator('button:has-text("Go to Dashboard"), a:has-text("Go to Dashboard")').first
            if go_btn.is_visible():
                go_btn.click()
                time.sleep(2)

        except Exception as e:
            print(f"[-] 引导流程或提取过程中发生错误: {e}")
            page.screenshot(path="exa_onboarding_error.png")

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
