import os
import glob
import re
import time
import random
import string
import json
import requests
from playwright.sync_api import sync_playwright
from playwright_stealth import Stealth
from mail_tm_utils import MailTM

# Firecrawl Next.js Server Action Config (Update if it breaks)
NEXT_ACTION_ID = "7083b486ac91e8b3d3adf631ff9eef3be52b52b813"
DEPLOYMENT_ID = "dpl_3oYF1KqgFSpHA1i33byZWVP7nNaC"

WAIT_UNTIL_MODE = "domcontentloaded"
BROWSER_SLOW_MO_MS = 0
POST_VERIFY_WAIT_S = 1.2
POST_LOGIN_WAIT_S = 0.8
ONBOARDING_STEP_WAIT_S = 0.8
API_PAGE_WAIT_S = 1.2
POST_POPUP_CLOSE_WAIT_S = 0.2
POST_SHOW_KEY_WAIT_S = 0.5


def run_registration():
    api_key = None
    with sync_playwright() as p:
        # Initialize Mail.tm
        mail = MailTM()
        if not mail.create_account():
            print("Failed to create temporary email.")
            return

        print(f"Created account: {mail.address}")

        # 1. API Signup (Bypasses UI Antispam)
        print("Sending Signup API request...")
        signup_url = "https://www.firecrawl.dev/signin"
        headers = {
            "Accept": "text/x-component",
            "Content-Type": "text/plain;charset=UTF-8",
            "Next-Action": NEXT_ACTION_ID,
            "Origin": "https://www.firecrawl.dev",
            "Referer": "https://www.firecrawl.dev/signup",
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            "X-Deployment-Id": DEPLOYMENT_ID
        }
        payload = [
            mail.address,
            mail.password,
            {
                "teamInvitationCode": None,
                "teamInvitationName": None,
                "redirect": None,
                "fingerprint": {
                    "requestId": f"{int(time.time()*1000)}.{''.join(random.choices(string.ascii_lowercase + string.digits, k=6))}"
                }
            }
        ]
        
        try:
            response = requests.post(signup_url, headers=headers, data=json.dumps(payload))
            if response.status_code != 200:
                print(f"API Signup failed with status {response.status_code}")
                print(response.text[:500])
                return
            
            if "error" in response.text and "Password is not strong enough" in response.text:
                print("ERROR: Password not strong enough.")
                return
                
            print("API Signup request sent successfully.")
        except Exception as e:
            print(f"API Signup error: {e}")
            return

        # 2. Wait for Verification Email
        print("Waiting for verification email...")
        email_content = mail.wait_for_email(timeout=300, poll_interval=1.0)
        if not email_content:
            print("Timeout waiting for verification email.")
            return

        # Extract confirmation link
        confirm_match = re.search(r'href="(https://[^"]+/auth/v1/verify\?[^"]+)"', email_content)
        if not confirm_match:
            print("Could not find confirmation link in email.")
            with open("debug_email.html", "w") as f:
                f.write(email_content)
            return

        confirm_url = confirm_match.group(1).replace('&amp;', '&')
        print(f"Verification link found: {confirm_url}")

        # 3. Browser Automation for Verification and Onboarding
        print("Launching browser for verification and onboarding...")
        browser = p.chromium.launch(headless=True, slow_mo=BROWSER_SLOW_MO_MS)
        context = browser.new_context(
            user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
        )
        page = context.new_page()
        Stealth().apply_stealth_sync(page)

        print("Navigating to verification link...")
        page.goto(confirm_url, wait_until=WAIT_UNTIL_MODE)
        time.sleep(POST_VERIFY_WAIT_S)
        
        print(f"Page after verification: {page.url}")
        page.screenshot(path="after_verify.png")

        if "/signin" in page.url or "/signup" in page.url or "auth" in page.url:
            print("Redirected to auth page. Attempting manual login...")
            try:
                # 1. Switch to Login Tab if needed
                login_tab = page.locator('button:has-text("Log In")').first
                if login_tab.is_visible():
                    print("Switching to Log In tab...")
                    login_tab.click()
                    time.sleep(POST_LOGIN_WAIT_S)

                # 2. Fill credentials
                page.wait_for_selector('input[type="email"]', timeout=10000)
                page.fill('input[type="email"]', mail.address)
                page.fill('input[type="password"]', mail.password)
                
                # 3. Click Submit button
                submit_btn = page.locator('button[type="submit"]').first
                if submit_btn.is_visible():
                    submit_btn.click()
                else:
                    page.keyboard.press("Enter")
                
                # Wait for navigation to app or dashboard
                print("Waiting for login to complete...")
                page.wait_for_url(re.compile(r".*/(app|onboarding).*"), timeout=15000)
                time.sleep(POST_LOGIN_WAIT_S)
            except Exception as e:
                print(f"Manual login failed or took too long: {e}")
                page.screenshot(path="login_error.png")

        # 4. Handle OnboardingFlow
        if "onboarding" in page.url:
            print("Handling onboarding...")
            for _ in range(5):
                try:
                    # Dismiss any survey or skip
                    btn = page.locator('button:has-text("Skip"), button:has-text("Continue"), button:has-text("Accept"), button:has-text("Next")').filter(has_not_text="GitHub").filter(has_not_text="Google").first
                    if btn.is_visible():
                        btn_text = btn.inner_text()
                        print(f"Clicking onboarding: {btn_text}")
                        btn.click()
                        time.sleep(ONBOARDING_STEP_WAIT_S)
                        if "api-keys" in page.url: break
                    else:
                        break
                except:
                    break

        # 5. Extract API Key
        print("Navigating to API Keys page...")
        for attempt in range(3):
            if "api-keys" not in page.url:
                page.goto("https://www.firecrawl.dev/app/api-keys", wait_until=WAIT_UNTIL_MODE)
                time.sleep(API_PAGE_WAIT_S)
            
            # Dismiss "What's New" popup
            try:
                close_popup = page.locator('button[aria-label="Close"]').first
                if close_popup.is_visible():
                    close_popup.click()
                    time.sleep(POST_POPUP_CLOSE_WAIT_S)
            except:
                pass

            # Click eye icon to reveal key - Updated Selector
            try:
                # The eye icon button often has a specific class or SVG
                eye_btn = page.locator('button:has(svg)').filter(has_text="").first
                # Finding by title if available
                eye_btn = page.locator('button[title="Show key"]').first
                if not eye_btn.is_visible():
                    # Try by class found in subagent run
                    eye_btn = page.locator('button.group\\/eye').first
                
                if eye_btn.is_visible():
                    print("Clicking Show Key button...")
                    eye_btn.click()
                    time.sleep(POST_SHOW_KEY_WAIT_S)
            except:
                pass

            # Try to find API key using the specific selector
            try:
                # Look for the span that looks like a key
                key_el = page.locator("span.font-mono").filter(has_text="fc-").first
                if key_el.is_visible():
                    api_key = key_el.inner_text().strip()
                    if api_key.startswith("fc-") and not api_key.endswith("..."):
                        break
            except:
                pass

            # Fallback to regex on content
            content = page.content()
            api_key_match = re.search(r'fc-[a-zA-Z0-9]{32,}', content)
            if api_key_match:
                api_key = api_key_match.group(0)
                break
            
            print(f"API key not found, retry {attempt+1}...")
            page.screenshot(path=f"api_key_retry_{attempt}.png")
            # If we are on signin page, login failed
            if "/signin" in page.url:
                print("Redirected back to signin. Login likely failed.")
                break


        browser.close()

        if api_key:
            # Clean up screenshots on success
            for f in glob.glob("*.png"):
                try: os.remove(f)
                except: pass
            
            return {
                "email": mail.address,
                "password": mail.password,
                "api_key": api_key
            }
        else:
            return {
                "email": mail.address,
                "password": mail.password,
                "api_key": None
            }

if __name__ == "__main__":
    result = run_registration()
    if result and result["api_key"]:
        print(f"\nSUCCESS! Key: {result['api_key']}")
        with open("accounts.txt", "a") as f:
            f.write(f"{result['email']}:{result['password']}:{result['api_key']}\n")
        with open("keys.txt", "a") as f:
            f.write(f"{result['api_key']}\n")
    else:
        print("\nFailed to get API key.")
