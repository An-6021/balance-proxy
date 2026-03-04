import base64
import io
import os
import random
import re
import string
import time
from typing import Any, Dict, Optional
from urllib.parse import parse_qs, urljoin, urlparse

import requests

from duckmail_utils import DuckMail
from inboxkitten_utils import InboxKitten
from mail_gw_utils import MailGW
from mail_tm_utils import MailTM

try:
    from config import DUCKMAIL_API_KEY as DEFAULT_DUCKMAIL_API_KEY
except Exception:
    DEFAULT_DUCKMAIL_API_KEY = ""

try:
    from PIL import Image

    HAS_PIL = True
except Exception:
    HAS_PIL = False

try:
    from playwright.sync_api import sync_playwright

    HAS_PLAYWRIGHT = True
except Exception:
    HAS_PLAYWRIGHT = False


DEFAULT_HTTP_TIMEOUT_S = 30
CAPTCHA_MAX_RETRIES = 5
VERIFY_EMAIL_TIMEOUT_S = 180
VERIFY_EMAIL_POLL_INTERVAL_S = 1.0

SILICON_FLOW_OCR_ENDPOINT = "https://api.siliconflow.cn/v1/chat/completions"


def _friendly_error_hint(error_code: Optional[str]) -> Optional[str]:
    if not error_code:
        return None
    if error_code == "invalid-captcha":
        return "验证码错误"
    if error_code == "ip-signup-blocked":
        return "当前出口 IP 被禁止注册/触发风控"
    if error_code == "custom-script-error-code_extensibility_error":
        return "服务端风控/自定义规则拒绝（常见：一次性邮箱域名被拦或注册策略限制）"
    return None


def _load_ocr_config() -> Optional[Dict[str, str]]:
    api_key = (os.environ.get("SILICON_FLOW_KEY") or "").strip()
    model = (os.environ.get("OCR_MODEL") or "").strip()

    if not api_key or not model:
        try:
            from config import OCR_MODEL as DEFAULT_OCR_MODEL
            from config import SILICON_FLOW_KEY as DEFAULT_SILICON_FLOW_KEY

            api_key = api_key or (DEFAULT_SILICON_FLOW_KEY or "").strip()
            model = model or (DEFAULT_OCR_MODEL or "").strip()
        except Exception:
            pass

    if not api_key or not model:
        return None

    return {"api_key": api_key, "model": model}


def _ocr_solve_png(image_path: str) -> Optional[str]:
    if not HAS_PIL:
        return None

    cfg = _load_ocr_config()
    if not cfg:
        return None

    try:
        img = Image.open(image_path)
        width, height = img.size
        try:
            resampling = Image.Resampling.LANCZOS
        except Exception:
            resampling = Image.LANCZOS
        img = img.resize((width * 3, height * 3), resampling)

        buffered = io.BytesIO()
        img.save(buffered, format="PNG")
        base64_image = base64.b64encode(buffered.getvalue()).decode("utf-8")
    except Exception:
        return None

    payload = {
        "model": cfg["model"],
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": (
                            "Identify the 6-character alphanumeric CAPTCHA code in this image. "
                            "It contains uppercase letters, lowercase letters, and numbers. "
                            "Provide ONLY the 6 characters as your response."
                        ),
                    },
                    {
                        "type": "image_url",
                        "image_url": {"url": f"data:image/png;base64,{base64_image}"},
                    },
                ],
            }
        ],
        "stream": False,
        "max_tokens": 10,
        "temperature": 0.01,
    }

    headers = {"Authorization": f"Bearer {cfg['api_key']}", "Content-Type": "application/json"}

    try:
        response = requests.post(
            SILICON_FLOW_OCR_ENDPOINT,
            json=payload,
            headers=headers,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        result = response.json()
        content = result["choices"][0]["message"]["content"].strip()

        matches = re.findall(r"[a-zA-Z0-9]{6}", content)
        if matches:
            return matches[0]

        clean_code = re.sub(r"[^a-zA-Z0-9]", "", content)
        if len(clean_code) >= 6:
            return clean_code[:6]
        return clean_code or None
    except Exception:
        return None


def _make_session(proxy: Optional[str] = None) -> requests.Session:
    session = requests.Session()
    session.headers.update(
        {
            "User-Agent": (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/133.0.0.0 Safari/537.36"
            ),
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "Accept-Language": "en-US,en;q=0.9",
        }
    )
    if proxy:
        session.proxies.update({"http": proxy, "https": proxy})
    return session


def _generate_password() -> str:
    alphabet = string.ascii_letters + string.digits + "!@#$%^&*"
    # Ensure complexity: upper + lower + digit + symbol.
    base = [
        random.choice(string.ascii_uppercase),
        random.choice(string.ascii_lowercase),
        random.choice(string.digits),
        random.choice("!@#$%^&*"),
    ]
    base.extend(random.choice(alphabet) for _ in range(12))
    random.shuffle(base)
    return "".join(base)


def _extract_primary_form_html(html: str) -> str:
    if not html:
        return html
    m = re.search(
        r'(<form[^>]*data-form-primary="true"[^>]*>.*?</form>)',
        html,
        flags=re.IGNORECASE | re.DOTALL,
    )
    return m.group(1) if m else html


def _extract_hidden_inputs(form_html: str) -> dict:
    out: Dict[str, str] = {}
    if not form_html:
        return out

    for name, value in re.findall(
        r'<input[^>]+type="hidden"[^>]+name="([^"]+)"[^>]+value="([^"]*)"',
        form_html,
        flags=re.IGNORECASE,
    ):
        out[name] = value

    for name, value in re.findall(
        r'<input[^>]+name="([^"]+)"[^>]+type="hidden"[^>]+value="([^"]*)"',
        form_html,
        flags=re.IGNORECASE,
    ):
        out.setdefault(name, value)

    return out


def _extract_action_value(form_html: str) -> str:
    m = re.search(
        r'<button[^>]+name="action"[^>]+value="([^"]+)"',
        form_html or "",
        flags=re.IGNORECASE,
    )
    return m.group(1) if m else "default"


def _extract_error_code(html: str) -> Optional[str]:
    m = re.search(
        r'data-error-code="([^"]+)"',
        html or "",
        flags=re.IGNORECASE,
    )
    return m.group(1) if m else None


def _extract_captcha_svg_base64(html: str) -> Optional[str]:
    if not html:
        return None
    matches = re.findall(
        r"data:image/svg\+xml;base64,([A-Za-z0-9+/=]+)",
        html,
        flags=re.IGNORECASE,
    )
    if not matches:
        return None
    # Prefer the longest blob (usually the real captcha).
    return max(matches, key=len)


def _write_captcha_files(svg_base64: str, *, basename: str = "tavily_captcha") -> str:
    svg_path = f"{basename}.svg"
    png_path = f"{basename}.png"

    try:
        with open(svg_path, "wb") as f:
            f.write(base64.b64decode(svg_base64))
    except Exception:
        pass

    if HAS_PLAYWRIGHT:
        try:
            data_url = f"data:image/svg+xml;base64,{svg_base64}"
            with sync_playwright() as p:
                browser = p.chromium.launch(headless=True)
                context = browser.new_context()
                page = context.new_page()
                page.set_content(
                    (
                        "<html><body style='margin:0;background:#fff;"
                        "display:flex;align-items:center;justify-content:center;'>"
                        f"<img id='captcha' src='{data_url}' />"
                        "</body></html>"
                    )
                )
                el = page.locator("#captcha")
                el.wait_for(timeout=10_000)
                el.screenshot(path=png_path)
                browser.close()
            return png_path
        except Exception:
            pass

    return svg_path


def _prompt_captcha(svg_base64: str, *, attempt: int) -> str:
    path = _write_captcha_files(svg_base64)

    # Try OCR first (Protocol mode should be hands-free by default).
    if path.endswith(".png") and (os.environ.get("TAVILY_PROTOCOL_OCR", "1").strip() != "0"):
        code = _ocr_solve_png(path)
        if code:
            code = re.sub(r"[^A-Za-z0-9]", "", code)
            if len(code) >= 6:
                code = code[:6]
                print(f"[*] OCR captcha: {code}")
                return code

    print(f"[!] Please open captcha image: {path}")
    while True:
        code = input(f"Enter captcha (attempt {attempt}): ").strip()
        code = re.sub(r"[^A-Za-z0-9]", "", code)
        if len(code) >= 6:
            return code
        print("Captcha looks too short; try again.")


def _get_auth0_entry(session: requests.Session, *, return_to: str) -> str:
    url = f"https://app.tavily.com/api/auth/login?returnTo={return_to}"
    resp = session.get(url, allow_redirects=False, timeout=DEFAULT_HTTP_TIMEOUT_S)
    if resp.status_code != 302:
        raise RuntimeError(f"Unexpected login entry status: {resp.status_code}")
    loc = resp.headers.get("Location") or ""
    if not loc:
        raise RuntimeError("Missing Location header from login entry")
    return loc


def _get_auth0_identifier_url(session: requests.Session, auth0_url: str) -> str:
    resp = session.get(auth0_url, allow_redirects=False, timeout=DEFAULT_HTTP_TIMEOUT_S)
    if resp.status_code != 302:
        raise RuntimeError(f"Unexpected Auth0 redirect status: {resp.status_code}")
    loc = resp.headers.get("Location") or ""
    if not loc:
        raise RuntimeError("Missing Location header from Auth0 redirect")
    if loc.startswith("/"):
        loc = urljoin("https://auth.tavily.com", loc)
    return loc


def _convert_login_to_signup(identifier_url: str) -> str:
    # Auth0 hosted pages: /u/login/identifier  -> /u/signup/identifier
    return identifier_url.replace("/u/login/identifier", "/u/signup/identifier")


def _parse_state_from_url(url: str) -> Optional[str]:
    try:
        parsed = urlparse(url)
        qs = parse_qs(parsed.query)
        state = qs.get("state", [None])[0]
        return state
    except Exception:
        return None


def _submit_signup_identifier(
    session: requests.Session,
    signup_url: str,
    *,
    email: str,
    captcha: str,
    state: Optional[str],
    html: str,
) -> str:
    form_html = _extract_primary_form_html(html)
    hidden = _extract_hidden_inputs(form_html)
    action_value = _extract_action_value(form_html)

    form_data = dict(hidden)
    form_data["state"] = hidden.get("state") or state or ""
    form_data["email"] = email
    form_data["captcha"] = captcha
    form_data["action"] = action_value

    headers = {
        "Content-Type": "application/x-www-form-urlencoded",
        "Origin": "https://auth.tavily.com",
        "Referer": signup_url,
    }

    resp = session.post(
        signup_url,
        data=form_data,
        headers=headers,
        allow_redirects=False,
        timeout=DEFAULT_HTTP_TIMEOUT_S,
    )

    if resp.status_code != 302:
        error_code = _extract_error_code(resp.text)
        hint = _friendly_error_hint(error_code)
        raise RuntimeError(
            f"Signup submit failed: status={resp.status_code}"
            + (f", error_code={error_code}" if error_code else "")
            + (f", hint={hint}" if hint else "")
        )

    loc = resp.headers.get("Location") or ""
    if not loc:
        raise RuntimeError("Signup submit missing redirect Location")
    if loc.startswith("/"):
        loc = urljoin("https://auth.tavily.com", loc)
    return loc


def _submit_signup_password(
    session: requests.Session,
    password_url: str,
    *,
    email: str,
    password: str,
    state: Optional[str],
) -> str:
    page = session.get(password_url, timeout=DEFAULT_HTTP_TIMEOUT_S)
    if page.status_code != 200:
        raise RuntimeError(f"Password page fetch failed: {page.status_code}")

    form_html = _extract_primary_form_html(page.text)
    hidden = _extract_hidden_inputs(form_html)
    action_value = _extract_action_value(form_html)

    form_data = dict(hidden)
    form_data["state"] = hidden.get("state") or state or ""
    form_data["email"] = email
    form_data["password"] = password
    form_data["action"] = action_value

    headers = {
        "Content-Type": "application/x-www-form-urlencoded",
        "Origin": "https://auth.tavily.com",
        "Referer": password_url,
    }

    resp = session.post(
        password_url,
        data=form_data,
        headers=headers,
        allow_redirects=False,
        timeout=DEFAULT_HTTP_TIMEOUT_S,
    )

    if resp.status_code != 302:
        error_code = _extract_error_code(resp.text)
        hint = _friendly_error_hint(error_code)
        raise RuntimeError(
            f"Password submit failed: status={resp.status_code}"
            + (f", error_code={error_code}" if error_code else "")
            + (f", hint={hint}" if hint else "")
        )

    loc = resp.headers.get("Location") or ""
    if not loc:
        return password_url
    if loc.startswith("/"):
        loc = urljoin("https://auth.tavily.com", loc)
    return loc


def _extract_verification_link(email_content: str) -> Optional[str]:
    if not email_content:
        return None
    patterns = [
        r"https://auth\.tavily\.com/[^\s\"'>]+ticket=[^\s\"'>]+",
        r"https://auth\.tavily\.com/[^\s\"'>]+confirm[^\s\"'>]+",
    ]
    for pat in patterns:
        m = re.search(pat, email_content, flags=re.IGNORECASE)
        if m:
            return m.group(0).replace("&amp;", "&")
    return None


def _wait_for_verification_link(mail) -> str:
    email_content = mail.wait_for_email(
        timeout=VERIFY_EMAIL_TIMEOUT_S,
        poll_interval=VERIFY_EMAIL_POLL_INTERVAL_S,
    )
    if not email_content:
        raise RuntimeError("Timed out waiting for verification email")
    link = _extract_verification_link(email_content)
    if not link:
        debug_path = "tavily_debug_mail.html"
        try:
            with open(debug_path, "w", encoding="utf-8") as f:
                f.write(email_content)
        except Exception:
            pass
        raise RuntimeError(f"Verification link not found (saved {debug_path})")
    return link


def _verify_email(session: requests.Session, verification_link: str) -> str:
    resp = session.get(
        verification_link,
        allow_redirects=True,
        timeout=DEFAULT_HTTP_TIMEOUT_S,
    )
    return resp.url


def _warmup_app_session(session: requests.Session) -> None:
    headers = {
        "Accept": "application/json",
        "Origin": "https://app.tavily.com",
        "Referer": "https://app.tavily.com/home",
    }

    try:
        session.get("https://app.tavily.com/home", timeout=DEFAULT_HTTP_TIMEOUT_S)
    except Exception:
        pass

    try:
        session.get("https://app.tavily.com/api/account", headers=headers, timeout=DEFAULT_HTTP_TIMEOUT_S)
    except Exception:
        pass

    has_seen = None
    try:
        r = session.get(
            "https://app.tavily.com/api/hasSeenTour",
            headers=headers,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        if r.status_code == 200:
            payload = r.json()
            if isinstance(payload, dict):
                for k in ("hasSeenTour", "has_seen_tour", "seenTour", "seen_tour"):
                    v = payload.get(k)
                    if isinstance(v, bool):
                        has_seen = v
                        break
    except Exception:
        pass

    if has_seen is False:
        try:
            session.put(
                "https://app.tavily.com/api/hasSeenTour",
                json={"hasSeenTour": True},
                headers={**headers, "Content-Type": "application/json"},
                timeout=DEFAULT_HTTP_TIMEOUT_S,
            )
        except Exception:
            pass

    try:
        session.post(
            "https://app.tavily.com/api/marketing-optin",
            json={"opt_in": False},
            headers={**headers, "Content-Type": "application/json"},
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
    except Exception:
        pass


def _extract_first_key(payload: Any) -> Optional[str]:
    def _extract(item) -> Optional[str]:
        if not isinstance(item, dict):
            return None
        v = item.get("key") or item.get("api_key") or item.get("apiKey")
        if isinstance(v, str) and v.strip():
            return v.strip()
        return None

    if isinstance(payload, list):
        for item in payload:
            v = _extract(item)
            if v:
                return v
        return None

    if isinstance(payload, dict):
        v = _extract(payload)
        if v:
            return v
        for k in ("keys", "data", "results"):
            if isinstance(payload.get(k), list):
                v = _extract_first_key(payload[k])
                if v:
                    return v
        return None

    return None


def _get_api_key(
    session: requests.Session, *, retries: int = 3, delay_s: float = 2.0
) -> Optional[str]:
    _warmup_app_session(session)
    for attempt in range(retries):
        try:
            resp = session.get("https://app.tavily.com/api/keys", timeout=DEFAULT_HTTP_TIMEOUT_S)
            if resp.status_code == 200:
                payload = resp.json()
                key = _extract_first_key(payload)
                if key:
                    return key
            elif resp.status_code == 401:
                return None
        except Exception:
            pass
        if attempt < retries - 1:
            time.sleep(delay_s)
    return None


def _create_api_key(session: requests.Session, *, name: str = "default") -> Optional[str]:
    headers = {
        "Origin": "https://app.tavily.com",
        "Referer": "https://app.tavily.com/home",
        "Accept": "application/json",
    }
    payload = {
        "name": name,
        "limit": 2147483647,
        "key_type": "development",
        "search_egress_policy": "allow_external",
    }
    try:
        resp = session.post(
            "https://app.tavily.com/api/keys?oid=",
            json=payload,
            headers=headers,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        if resp.status_code not in (200, 201):
            return None
        data = resp.json()
        return _extract_first_key(data)
    except Exception:
        return None


def _ensure_app_login(session: requests.Session, *, email: str, password: str) -> None:
    me = None
    try:
        me = session.get("https://app.tavily.com/api/auth/me", timeout=DEFAULT_HTTP_TIMEOUT_S)
    except Exception:
        me = None

    if me is not None and me.status_code == 200:
        return

    # Trigger login redirect flow (Auth0 hosted pages).
    auth0_url = _get_auth0_entry(session, return_to="/home")
    identifier_url = _get_auth0_identifier_url(session, auth0_url)

    page = session.get(identifier_url, timeout=DEFAULT_HTTP_TIMEOUT_S)
    html = page.text
    state = _parse_state_from_url(page.url) or _parse_state_from_url(identifier_url)
    form_html = _extract_primary_form_html(html)
    hidden = _extract_hidden_inputs(form_html)
    action_value = _extract_action_value(form_html)

    captcha_svg = _extract_captcha_svg_base64(html)
    is_password_page = "/u/login/password" in (page.url or "")
    if captcha_svg and not is_password_page:
        captcha = _prompt_captcha(captcha_svg, attempt=1)

        form_data = dict(hidden)
        form_data["state"] = hidden.get("state") or state or ""
        form_data["username"] = email
        form_data["captcha"] = captcha
        form_data["action"] = action_value

        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Origin": "https://auth.tavily.com",
            "Referer": identifier_url,
        }
        resp = session.post(
            identifier_url,
            data=form_data,
            headers=headers,
            allow_redirects=False,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        if resp.status_code != 302:
            error_code = _extract_error_code(resp.text)
            raise RuntimeError(
                f"Login submit failed: status={resp.status_code}"
                + (f", error_code={error_code}" if error_code else "")
            )
        password_url = resp.headers.get("Location") or ""
        if password_url.startswith("/"):
            password_url = urljoin("https://auth.tavily.com", password_url)
    else:
        password_url = page.url

    pw_page = session.get(password_url, timeout=DEFAULT_HTTP_TIMEOUT_S)
    if pw_page.status_code != 200:
        raise RuntimeError(f"Login password page fetch failed: {pw_page.status_code}")

    pw_form_html = _extract_primary_form_html(pw_page.text)
    pw_hidden = _extract_hidden_inputs(pw_form_html)
    pw_action = _extract_action_value(pw_form_html)
    pw_state = pw_hidden.get("state") or state or ""

    form_data = dict(pw_hidden)
    form_data["state"] = pw_state
    form_data["username"] = email
    form_data["password"] = password
    form_data["action"] = pw_action

    headers = {
        "Content-Type": "application/x-www-form-urlencoded",
        "Origin": "https://auth.tavily.com",
        "Referer": password_url,
    }
    resp = session.post(
        password_url,
        data=form_data,
        headers=headers,
        allow_redirects=True,
        timeout=DEFAULT_HTTP_TIMEOUT_S,
    )
    if "app.tavily.com" not in (resp.url or ""):
        raise RuntimeError("Login did not redirect to app.tavily.com")


def run_registration(headless: bool = False, mail_factory=None, proxy: Optional[str] = None):
    # headless is accepted for compatibility with existing CLI; protocol mode is HTTP-based.
    _ = headless

    # Email providers (least common first).
    def _create_duckmail():
        key = (DEFAULT_DUCKMAIL_API_KEY or "").strip()
        if key:
            return DuckMail(api_key=key)
        return DuckMail()

    providers = [
        _create_duckmail,
        MailGW,
        InboxKitten,
        MailTM,
    ]

    mail = None
    if mail_factory:
        mail = mail_factory()
        if not mail.create_account():
            print("[-] Failed to create account with the selected mail provider.")
            return None
    else:
        for factory in providers:
            try:
                candidate = factory() if callable(factory) else factory()
                if candidate.create_account():
                    mail = candidate
                    break
            except Exception:
                continue

    if not mail:
        print("[-] All mail providers failed.")
        return None

    email_addr = mail.address
    password = _generate_password()
    print(f"[+] Email: {email_addr}")

    session = _make_session(proxy=proxy)

    auth0_url = _get_auth0_entry(session, return_to="/home")
    login_identifier_url = _get_auth0_identifier_url(session, auth0_url)
    signup_url = _convert_login_to_signup(login_identifier_url)
    state = _parse_state_from_url(signup_url)

    password_url = None
    for attempt in range(1, CAPTCHA_MAX_RETRIES + 1):
        page = session.get(signup_url, timeout=DEFAULT_HTTP_TIMEOUT_S)
        if page.status_code != 200:
            raise RuntimeError(f"Signup page fetch failed: {page.status_code}")

        captcha_svg = _extract_captcha_svg_base64(page.text)
        if not captcha_svg:
            debug_path = "tavily_debug_signup.html"
            try:
                with open(debug_path, "w", encoding="utf-8") as f:
                    f.write(page.text)
            except Exception:
                pass
            raise RuntimeError(f"Captcha not found on signup page (saved {debug_path})")

        captcha = _prompt_captcha(captcha_svg, attempt=attempt)

        try:
            next_url = _submit_signup_identifier(
                session,
                signup_url,
                email=email_addr,
                captcha=captcha,
                state=state,
                html=page.text,
            )
        except Exception as e:
            print(f"[!] Signup submit failed: {e}")
            time.sleep(1.0)
            continue

        if "/u/signup/password" in next_url:
            password_url = next_url
            break

        print("[!] Signup did not reach password step; retrying...")
        time.sleep(1.0)

    if not password_url:
        print("[-] Failed to pass captcha after retries.")
        return {"email": email_addr, "password": password, "api_key": None}

    try:
        _submit_signup_password(
            session,
            password_url,
            email=email_addr,
            password=password,
            state=state,
        )
    except Exception as e:
        print(f"[-] Password step failed: {e}")
        return {"email": email_addr, "password": password, "api_key": None}

    try:
        verification_link = _wait_for_verification_link(mail)
    except Exception as e:
        print(f"[-] Verification email failed: {e}")
        return {"email": email_addr, "password": password, "api_key": None}
    print(f"[+] Verification link: {verification_link[:60]}...")

    try:
        final_url = _verify_email(session, verification_link)
        print(f"[+] Verified. Final URL: {final_url[:60]}...")
    except Exception as e:
        print(f"[-] Verification link visit failed: {e}")
        return {"email": email_addr, "password": password, "api_key": None}

    # Try to enter app session.
    try:
        session.get(
            "https://app.tavily.com/api/auth/login?returnTo=/home",
            allow_redirects=True,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
    except Exception:
        pass

    api_key = _get_api_key(session)
    if not api_key:
        try:
            _ensure_app_login(session, email=email_addr, password=password)
            api_key = _get_api_key(session)
        except Exception as e:
            print(f"[!] Login to fetch key failed: {e}")

    if not api_key:
        api_key = _create_api_key(session)

    return {"email": email_addr, "password": password, "api_key": api_key}


if __name__ == "__main__":
    res = run_registration()
    print(res)
