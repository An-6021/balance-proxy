import time
import re
import random
import os
from playwright.sync_api import sync_playwright
from playwright_stealth import Stealth
from mail_tm_utils import MailTM
from mail_gw_utils import MailGW
from duckmail_utils import DuckMail
from inboxkitten_utils import InboxKitten

# 导入配置
from config import (
    DUCKMAIL_API_KEY,
    TAVILY_PASSWORD,
    SILICON_FLOW_KEY,
    OCR_MODEL,
    DEFAULT_PROXY,
)

# 邮箱服务工厂函数（支持带参数的实例化）
def create_duckmail():
    return DuckMail(api_key=DUCKMAIL_API_KEY)

# 邮箱服务优先级列表（从最不常见到最常见）
# Auth0 可能已拉黑 mail.tm，尝试其他服务
MAIL_PROVIDERS = [
    create_duckmail,  # duckmail.sbs - 较新，可能未被拉黑
    MailGW,           # mail.gw - 动态域名
    InboxKitten,      # inboxkitten.com
    MailTM,           # mail.tm - 可能已被拉黑
]

# 兼容旧代码的变量名
PASSWORD = TAVILY_PASSWORD 

import base64
import requests
import io
from PIL import Image

def ocr_solve(image_path):
    """使用硅基流动接口识别验证码 (带图像放大处理)"""
    try:
        # 图像预处理: 放大 3 倍以应对 Qwen3-VL 的高密度输入要求
        img = Image.open(image_path)
        width, height = img.size
        img = img.resize((width * 3, height * 3), Image.Resampling.LANCZOS)
        
        buffered = io.BytesIO()
        img.save(buffered, format="PNG")
        base64_image = base64.b64encode(buffered.getvalue()).decode('utf-8')
    except Exception as e:
        print(f"[-] 图像处理失败: {e}")
        return None
    
    payload = {
        "model": OCR_MODEL,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text", 
                        "text": "Identify the 6-character alphanumeric CAPTCHA code in this image. It contains uppercase letters, lowercase letters, and numbers. Provide ONLY the 6 characters as your response."
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/png;base64,{base64_image}"
                        }
                    }
                ]
            }
        ],
        "stream": False,
        "max_tokens": 10,
        "temperature": 0.01  # 设置更低温度以获得确定的识别结果
    }
    headers = {
        "Authorization": f"Bearer {SILICON_FLOW_KEY}",
        "Content-Type": "application/json"
    }

    try:
        response = requests.post("https://api.siliconflow.cn/v1/chat/completions", json=payload, headers=headers)
        result = response.json()
        
        if 'choices' not in result:
             print(f"[-] API 响应异常: {result}")
             return None
             
        content = result['choices'][0]['message']['content'].strip()
        print(f"[*] OCR 原始输出: {content}")
        
        # 精确提取 6 位字母数字
        matches = re.findall(r'[a-zA-Z0-9]{6}', content)
        if matches:
            return matches[0]
            
        clean_code = re.sub(r'[^a-zA-Z0-9]', '', content)
        return clean_code[:6] if len(clean_code) >= 6 else clean_code
    except Exception as e:
        print(f"[-] OCR 识别逻辑出错: {e}")
        return None

def solve_captcha(page):
    """
    改进的验证码提取逻辑。
    """
    try:
        # 尝试多个可能的选择器
        selectors = [
            'img[src*="captcha"]',
            '.captcha-img img',
            'div[style*="background-image"]',
            'img[alt="Captcha"]'
        ]
        
        captcha_el = None
        for sel in selectors:
            captcha_el = page.query_selector(sel)
            if captcha_el: break
            
        if not captcha_el:
            # 如果没找到，尝试根据位置找 Auth0 容器内的图片
            captcha_el = page.query_selector('form img')

        if captcha_el:
            captcha_el.screenshot(path="captcha.png")
            print(f"[*] 验证码已截图，正在调用 OCR...")
            code = ocr_solve("captcha.png")
            print(f"[+] OCR 识别结果: {code}")
            return code
    except Exception as e:
        print(f"[-] 提取验证码失败: {e}")
    return None

def create_fresh_context(browser, p):
    """创建一个带有随机指纹的浏览器上下文"""
    # 随机选择常见的 User-Agent
    user_agents = [
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    ]
    
    # 随机视口大小
    viewports = [
        {"width": 1920, "height": 1080},
        {"width": 1680, "height": 1050},
        {"width": 1440, "height": 900},
        {"width": 1536, "height": 864},
        {"width": 1366, "height": 768},
    ]
    
    # 随机时区和语言
    locales = ["en-US", "en-GB", "en-CA"]
    timezones = ["America/New_York", "America/Los_Angeles", "Europe/London", "Europe/Paris"]
    
    context = browser.new_context(
        user_agent=random.choice(user_agents),
        viewport=random.choice(viewports),
        locale=random.choice(locales),
        timezone_id=random.choice(timezones),
        # 添加更多浏览器特征
        color_scheme=random.choice(["light", "dark"]),
        has_touch=random.choice([True, False]),
        is_mobile=False,
        java_script_enabled=True,
    )
    
    # 注入额外的反检测脚本
    context.add_init_script("""
        // 覆盖 webdriver 属性
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        
        // 覆盖 plugins 长度
        Object.defineProperty(navigator, 'plugins', { 
            get: () => [1, 2, 3, 4, 5] 
        });
        
        // 覆盖 languages
        Object.defineProperty(navigator, 'languages', { 
            get: () => ['en-US', 'en'] 
        });
        
        // 覆盖 platform
        Object.defineProperty(navigator, 'platform', { 
            get: () => 'Win32' 
        });
        
        // 覆盖 hardwareConcurrency
        Object.defineProperty(navigator, 'hardwareConcurrency', { 
            get: () => 8 
        });
        
        // 覆盖 deviceMemory
        Object.defineProperty(navigator, 'deviceMemory', { 
            get: () => 8 
        });
        
        // 隐藏自动化标志
        window.chrome = { runtime: {} };
        
        // 覆盖 permissions
        const originalQuery = window.navigator.permissions.query;
        window.navigator.permissions.query = (parameters) => (
            parameters.name === 'notifications' ?
                Promise.resolve({ state: Notification.permission }) :
                originalQuery(parameters)
        );
    """)
    
    return context


def run_registration(headless=False, mail_factory=None, proxy=None):
    """
    Tavily 注册函数
    
    参数:
        headless: 是否无头模式
        mail_factory: 邮箱服务类，None 表示自动尝试所有可用服务
        proxy: 代理设置，例如 "http://127.0.0.1:7890" 或 "socks5://127.0.0.1:1080"
    """
    with sync_playwright() as p:
        # 1. 初始化临时邮箱 API
        mail = None
        email_addr = None
        
        if mail_factory:
            # 使用指定的邮箱服务
            mail = mail_factory()
            if mail.create_account():
                email_addr = mail.address
                print(f"[+] 获取到有效邮箱: {email_addr}")
            else:
                print(f"[-] 指定的邮箱服务创建失败")
                return None
        else:
            # 自动尝试所有邮箱服务
            for provider in MAIL_PROVIDERS:
                # 获取名称（支持类和工厂函数）
                name = provider.__name__ if hasattr(provider, '__name__') else provider.__class__.__name__
                print(f"[*] 尝试使用 {name}...")
                try:
                    mail = provider()
                    if mail.create_account():
                        email_addr = mail.address
                        print(f"[+] 获取到有效邮箱: {email_addr} ({name})")
                        break
                except Exception as e:
                    print(f"[-] {name} 失败: {e}")
                    continue
            
            if not email_addr:
                print("[-] 所有邮箱服务均不可用")
                return None

        # 启动浏览器（带更多反检测参数）
        launch_args = [
            '--disable-blink-features=AutomationControlled',
            '--disable-infobars',
            '--disable-extensions',
            '--disable-gpu',
            '--no-sandbox',
            '--disable-dev-shm-usage',
            '--disable-web-security',
            '--disable-features=IsolateOrigins,site-per-process',
        ]
        
        # 如果提供了代理
        if proxy:
            launch_args.append(f'--proxy-server={proxy}')
            print(f"[*] 使用代理: {proxy}")
        
        browser = p.chromium.launch(
            headless=headless,
            args=launch_args,
            # 使用真实 Chrome 而非 Chromium（如果可用）
            channel="chrome" if os.path.exists("/Applications/Google Chrome.app") else "chromium",
        )
        
        context = create_fresh_context(browser, p)
        
        # --- 步骤 2: Tavily 注册 ---
        tavily_page = context.new_page()
        Stealth().apply_stealth_sync(tavily_page)
        
        # 模拟人类行为：随机延迟
        time.sleep(random.uniform(0.5, 1.5))
        
        print("[*] 正在打开 Tavily 网址...")
        # 先访问主页建立自然访问路径
        tavily_page.goto("https://tavily.com/", wait_until="networkidle", timeout=30000)
        time.sleep(random.uniform(1.0, 2.0))  # 模拟阅读页面
        
        # 然后导航到注册页
        tavily_page.goto("https://app.tavily.com/sign-up", wait_until="networkidle", timeout=30000)
        
        # 检查是否因为已登录被重定向到 /home
        if "/home" in tavily_page.url:
            print("[!] 检测到已登录状态，正在登出...")
            tavily_page.goto("https://app.tavily.com/api/auth/logout")
            tavily_page.goto("https://app.tavily.com/sign-up")

        # 确保我们在 "Sign Up" 页面而不是 "Log In"
        signup_toggle = tavily_page.query_selector('a:has-text("Sign up")')
        if signup_toggle:
            print("[*] 切换到注册页面...")
            signup_toggle.click()
            time.sleep(0.5)

        print("[*] 模仿人类输入邮箱...")
        # 模拟人类行为：先移动鼠标到输入框
        email_input = tavily_page.query_selector('input#email')
        if email_input:
            # 随机移动鼠标
            box = email_input.bounding_box()
            if box:
                tavily_page.mouse.move(
                    box['x'] + random.randint(0, int(box['width'])),
                    box['y'] + random.randint(0, int(box['height']))
                )
                time.sleep(random.uniform(0.1, 0.3))
        
        # 清空并逐字输入（带随机延迟）
        tavily_page.click('input#email')
        tavily_page.fill('input#email', '')  # 先清空
        
        # 模拟人类打字速度（50-150ms 随机延迟）
        for char in email_addr:
            tavily_page.type('input#email', char, delay=random.randint(50, 150))
            # 偶尔暂停（模拟人类思考）
            if random.random() < 0.1:
                time.sleep(random.uniform(0.1, 0.3))
        
        # 处理验证码逻辑
        max_retries = 5
        for i in range(max_retries):
            print(f"[*] 尝试识别验证码 ({i+1}/{max_retries})...")
            # 等待验证码元素出现 (Auth0 这里的 img alt="captcha" 是关键)
            try:
                tavily_page.wait_for_selector('img[alt="captcha"]', timeout=10000)
            except:
                print("[-] 未发现验证码图片，检查页面状态...")
            
            code = solve_captcha(tavily_page)
            if code:
                tavily_page.fill('input#captcha', code)
            
            # 点击 Continue (针对 Signup 的按钮)
            continue_btn = tavily_page.query_selector('button[name="action"][value="default"]')
            if not continue_btn:
                continue_btn = tavily_page.query_selector('button._button-signup-id')
            
            if continue_btn:
                continue_btn.click()
            else:
                print("[-] 未找到提交按钮，尝试按回车...")
                tavily_page.keyboard.press("Enter")

            # 循环检测是否进入密码设置环节
            passed = False
            for _ in range(15):
                if tavily_page.query_selector('input#password') or tavily_page.query_selector('input[name="password"]'):
                    print("[+] 验证码通过，进入密码环节!")
                    passed = True
                    break
                time.sleep(0.3)

            if passed:
                break

            # 检查错误
            error_el = tavily_page.query_selector('#error-element-captcha')
            if error_el:
                print(f"[-] 错误提示: {error_el.inner_text()}")

            print("[-] 仍在注册页，准备重试...")

        # 设置密码
        print("[*] 正在设置密码...")
        # 针对新页面的密码输入选择器
        password_field = tavily_page.wait_for_selector('input#password, input[name="password"]')
        password_field.fill(PASSWORD)
        
        # 点击最终提交
        submit_btn = tavily_page.query_selector('button[name="action"][value="default"]')
        if submit_btn:
            submit_btn.click()
        else:
            tavily_page.keyboard.press("Enter")
        
        print("[*] 注册提交完成，开始检查验证邮件...")

        # --- 步骤 3: 检查并点击验证邮件 (API Mode) ---
        email_content = mail.wait_for_email(timeout=180, poll_interval=1.0)
        if not email_content:
            print("[!] 未能在 180s 内收到验证邮件。")
            return None

        # Extract confirmation link for Tavily
        # Looks for links containing auth.tavily.com and confirm/ticket
        match = re.search(r'https://auth\.tavily\.com/[^\s"\'>]+(?:confirm|ticket=)[^\s"\'>]+', email_content)
        if not match:
            print("[!] 邮件内容中未找到验证链接。")
            with open("tavily_debug_mail.html", "w") as f:
                f.write(email_content)
            return None
        
        found_link = match.group(0).replace('&amp;', '&')
        print(f"[+] 识别到验证链接: {found_link}")
        
        tavily_page.goto(found_link)

        # 循环检测是否重定向到首页
        for _ in range(30):
            if "/home" in tavily_page.url:
                break
            time.sleep(0.3)

        # 确保页面加载完成
        tavily_page.wait_for_load_state("domcontentloaded")

        # --- 步骤 4: 提取 API Key ---
        
        try:
            # --- 4.1: 关闭所有弹窗 ---
            print("[*] 尝试关闭弹窗...")
            for _ in range(5):
                closed = False

                # Cookie 弹窗
                tavily_page.evaluate("""() => {
                    const btns = document.querySelectorAll('button');
                    for (const btn of btns) {
                        if (btn.textContent.includes('Reject All')) {
                            btn.click();
                            return true;
                        }
                    }
                    return false;
                }""")

                # Continue 按钮 (营销弹窗)
                continue_btn = tavily_page.query_selector('button:has-text("Continue")')
                if continue_btn and continue_btn.is_visible():
                    continue_btn.click()
                    closed = True

                # X 关闭按钮 (Welcome 弹窗)
                close_btn = tavily_page.query_selector('button[aria-label="Close"]')
                if close_btn and close_btn.is_visible():
                    close_btn.click()
                    closed = True

                if not closed:
                    break
                time.sleep(0.3)

            # --- 4.2: 等待 API Keys 区域出现 ---
            tavily_page.wait_for_selector('input[readonly]', timeout=10000)

            # --- 4.5: 点击眼睛图标显示完整 API Key ---
            # API Key 行的按钮顺序：👁显示 | 📋复制 | ✏️编辑 | 🗑️删除
            # 眼睛按钮是 Key 后面的第一个按钮
            print("[*] 尝试点击显示 API Key...")
            tavily_page.evaluate("""() => {
                const input = document.querySelector('input[readonly]');
                if (input) {
                    // 找到 input 所在行的父容器，然后找第一个 button（即眼睛按钮）
                    const row = input.closest('tr') || input.parentElement.parentElement;
                    if (row) {
                        const btns = row.querySelectorAll('button');
                        if (btns.length > 0) btns[0].click();
                    }
                }
            }""")
            time.sleep(0.5)

            # --- 4.3: 提取 API Key ---
            api_key = "ERROR_EXTRACTING"
            for _ in range(10):
                val = tavily_page.evaluate("""() => {
                    const inputs = document.querySelectorAll('input');
                    for (const inp of inputs) {
                        if (inp.value && inp.value.startsWith('tvly-') && !inp.value.includes('*')) {
                            return inp.value;
                        }
                    }
                    return null;
                }""")
                if val:
                    api_key = val
                    break
                time.sleep(0.3)
            
            # 兜底：即使 Key 仍被遮罩 (tvly-dev-****)，也先保存遮罩版本
            if api_key == "ERROR_EXTRACTING":
                val = tavily_page.evaluate("""() => {
                    const inputs = document.querySelectorAll('input');
                    for (const inp of inputs) {
                        if (inp.value && inp.value.startsWith('tvly-')) {
                            return inp.value;
                        }
                    }
                    return null;
                }""")
                if val:
                    api_key = val
                    print(f"[!] API Key 仍被遮罩，使用复制按钮获取...")
                    # 尝试通过复制按钮获取（点击并读取剪贴板）
                    tavily_page.evaluate("""() => {
                        const input = document.querySelector('input[readonly]');
                        if (input) {
                            const row = input.closest('tr') || input.parentElement.parentElement;
                            if (row) {
                                const btns = row.querySelectorAll('button');
                                if (btns.length > 1) btns[1].click(); // 复制按钮是第二个
                            }
                        }
                    }""")
                    time.sleep(0.3)

            # 最终兜底：正则爬取页面源码
            if api_key == "ERROR_EXTRACTING":
                match = re.search(r'tvly-dev-[a-zA-Z0-9]{20,}', tavily_page.content())
                if match:
                    api_key = match.group(0)

        except Exception as e:
            print(f"[-] 提取阶段发生错误: {e}")
            tavily_page.screenshot(path="debug_step4_error.png")
            api_key = "WAITING_MANUAL"

        browser.close()

        if api_key and api_key not in ("ERROR_EXTRACTING", "WAITING_MANUAL"):
            return {
                "email": email_addr,
                "password": PASSWORD,
                "api_key": api_key
            }
        else:
            return {
                "email": email_addr,
                "password": PASSWORD,
                "api_key": None
            }

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description='Tavily 自动注册脚本')
    parser.add_argument('--headless', action='store_true', help='无头模式运行')
    parser.add_argument('--proxy', type=str, default=None, help='代理服务器，例如 http://127.0.0.1:7890')
    parser.add_argument('--mail', type=str, default=None, 
                        choices=['mailtm', 'mailgw', 'duckmail', 'inboxkitten'],
                        help='指定邮箱服务')
    args = parser.parse_args()
    
    mail_factory = None
    if args.mail == 'mailtm':
        mail_factory = MailTM
    elif args.mail == 'mailgw':
        mail_factory = MailGW
    elif args.mail == 'duckmail':
        mail_factory = DuckMail
    elif args.mail == 'inboxkitten':
        mail_factory = InboxKitten
    
    res = run_registration(headless=args.headless, mail_factory=mail_factory, proxy=args.proxy)
    print(res)
