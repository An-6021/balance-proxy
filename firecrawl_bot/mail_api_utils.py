import os
import random
import string
import time
from email import message_from_string
from typing import Any, Dict, List, Optional

import requests

from local_config import load_local_config

DEFAULT_EMAIL_POLL_INTERVAL = 1.0
DEFAULT_HTTP_TIMEOUT_S = 15
DEFAULT_ACCOUNT_RETRIES = 3
DEFAULT_RETRY_BASE_DELAY_S = 1.5


def _parse_bool(value: Any, *, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    text = str(value).strip().lower()
    if text in ("1", "true", "yes", "y", "on"):
        return True
    if text in ("0", "false", "no", "n", "off"):
        return False
    return default


def _random_suffix(length: int = 6) -> str:
    alphabet = string.ascii_lowercase + string.digits
    return "".join(random.choices(alphabet, k=length))


def _generate_password() -> str:
    # Used as the *service signup password* in some bots (e.g. Firecrawl),
    # not for the mail system itself.
    alphabet = string.ascii_letters + string.digits + "!@#$%^&*"
    parts = [
        random.choice(string.ascii_uppercase),
        random.choice(string.ascii_lowercase),
        random.choice(string.digits),
        random.choice("!@#$%^&*"),
    ]
    parts.extend(random.choice(alphabet) for _ in range(12))
    random.shuffle(parts)
    return "".join(parts)


def _decode_email_body(raw: str) -> str:
    """
    Best-effort decode of RFC822 raw email to get readable HTML/text body.
    If parsing fails, fall back to the original raw string.
    """
    if not raw:
        return ""

    try:
        msg = message_from_string(raw)
        body_parts: List[str] = []

        def _decode_payload(part) -> Optional[str]:
            payload = part.get_payload(decode=True)
            if not payload:
                return None
            charset = part.get_content_charset() or "utf-8"
            try:
                return payload.decode(charset, errors="replace")
            except Exception:
                return payload.decode("utf-8", errors="replace")

        if msg.is_multipart():
            for part in msg.walk():
                content_type = part.get_content_type()
                if content_type not in ("text/plain", "text/html"):
                    continue
                decoded = _decode_payload(part)
                if decoded:
                    body_parts.append(decoded)
        else:
            decoded = _decode_payload(msg)
            if decoded:
                body_parts.append(decoded)

        return "\n".join(body_parts).strip() or raw
    except Exception:
        return raw


class MailAPI:
    """
    Self-hosted temp mail provider (Mail API).

    Docs:
      - POST /admin/new_address (x-admin-auth)
      - GET  /api/mails (Authorization: Bearer <jwt>)
    """

    def __init__(
        self,
        *,
        base_url: Optional[str] = None,
        admin_auth: Optional[str] = None,
        domain: Optional[str] = None,
        enable_prefix: Optional[bool] = None,
        name: Optional[str] = None,
        custom_auth: Optional[str] = None,
    ):
        self.session = requests.Session()

        local_cfg = load_local_config()
        mail_api_cfg = local_cfg.get("mail_api") if isinstance(local_cfg.get("mail_api"), dict) else {}

        self.base_url = (
            base_url
            or os.environ.get("MAIL_API_BASE_URL")
            or os.environ.get("SELFMAIL_BASE_URL")
            or mail_api_cfg.get("base_url")
            or ""
        ).strip().rstrip("/")
        self._admin_auth = (
            admin_auth
            or os.environ.get("MAIL_API_ADMIN_AUTH")
            or os.environ.get("SELFMAIL_ADMIN_AUTH")
            or mail_api_cfg.get("admin_auth")
            or ""
        ).strip()
        self._domain = (
            domain
            or os.environ.get("MAIL_API_DOMAIN")
            or os.environ.get("SELFMAIL_DOMAIN")
            or mail_api_cfg.get("domain")
            or ""
        ).strip().lower()

        env_enable_prefix = os.environ.get("MAIL_API_ENABLE_PREFIX")
        if enable_prefix is None:
            if env_enable_prefix is None:
                enable_prefix = _parse_bool(mail_api_cfg.get("enable_prefix", True), default=True)
            else:
                enable_prefix = _parse_bool(env_enable_prefix, default=True)
        self._enable_prefix = bool(enable_prefix)

        self._name = (
            name
            or os.environ.get("MAIL_API_NAME")
            or mail_api_cfg.get("name")
            or "bot"
        ).strip() or "bot"
        self._custom_auth = (
            custom_auth
            or os.environ.get("MAIL_API_CUSTOM_AUTH")
            or mail_api_cfg.get("custom_auth")
            or ""
        ).strip() or None

        self.address = None
        self.password = None
        self.jwt = None

    def _configured(self) -> bool:
        return bool(self.base_url and self._admin_auth and self._domain)

    def create_account(self, max_retries: int = DEFAULT_ACCOUNT_RETRIES) -> bool:
        if not self._configured():
            return False

        self.password = _generate_password()

        url = f"{self.base_url}/admin/new_address"
        headers = {
            "x-admin-auth": self._admin_auth,
            "Content-Type": "application/json",
        }

        for attempt in range(1, max_retries + 1):
            name = f"{self._name}-{_random_suffix()}"
            payload = {"enablePrefix": self._enable_prefix, "name": name, "domain": self._domain}

            try:
                response = self.session.post(
                    url,
                    json=payload,
                    headers=headers,
                    timeout=DEFAULT_HTTP_TIMEOUT_S,
                )
            except requests.RequestException as e:
                print(f"mail api address create error ({attempt}/{max_retries}): {e}")
                if attempt < max_retries:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
                continue

            if response.status_code != 200:
                reason = response.text.strip().replace("\n", " ")[:200]
                print(
                    f"mail api address create failed ({attempt}/{max_retries}), "
                    f"status={response.status_code}, body={reason}"
                )
                if attempt < max_retries:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
                continue

            try:
                data = response.json()
            except Exception:
                print("mail api address create failed: non-json response")
                if attempt < max_retries:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
                continue

            jwt = (data.get("jwt") or data.get("token") or "").strip() if isinstance(data, dict) else ""
            address = (data.get("address") or "").strip() if isinstance(data, dict) else ""
            if not address and not self._enable_prefix:
                address = f"{name}@{self._domain}"

            if not jwt or not address:
                print(f"mail api address create failed: missing jwt/address: {data}")
                if attempt < max_retries:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
                continue

            self.jwt = jwt
            self.address = address
            return True

        return False

    def wait_for_email(self, timeout: int = 300, poll_interval: float = DEFAULT_EMAIL_POLL_INTERVAL) -> Optional[str]:
        poll_interval = max(0.2, float(poll_interval))
        if not self.jwt:
            return None

        headers = {"Authorization": f"Bearer {self.jwt}"}
        if self._custom_auth:
            headers["x-custom-auth"] = self._custom_auth

        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                response = self.session.get(
                    f"{self.base_url}/api/mails?limit=10&offset=0",
                    headers=headers,
                    timeout=DEFAULT_HTTP_TIMEOUT_S,
                )
                response.raise_for_status()
                payload = response.json()
                results = payload.get("results", []) if isinstance(payload, dict) else []
            except Exception as e:
                print(f"mail api poll error: {e}")
                results = []

            if results:
                item = results[0] if isinstance(results, list) else None
                if isinstance(item, dict):
                    raw = item.get("raw") or item.get("html") or item.get("text") or ""
                    raw = str(raw)
                    return _decode_email_body(raw)

            remaining = deadline - time.time()
            if remaining <= 0:
                break
            time.sleep(min(poll_interval, remaining))

        return None
