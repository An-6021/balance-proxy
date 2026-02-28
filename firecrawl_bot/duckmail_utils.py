import requests
import time
import random
import string

DEFAULT_EMAIL_POLL_INTERVAL = 1.0
DEFAULT_HTTP_TIMEOUT_S = 15
DEFAULT_DOMAIN_RETRIES = 3
DEFAULT_ACCOUNT_RETRIES = 5
DEFAULT_RETRY_BASE_DELAY_S = 1.5


class DuckMail:
    BASE_URL = "https://api.duckmail.sbs"

    def __init__(self, api_key=None):
        self.session = requests.Session()
        self.domain = None
        self.address = None
        self.password = None
        self.token = None
        self.account_id = None
        self._api_key = (api_key or "").strip() or None

    def _auth_headers(self):
        if self.token:
            return {"Authorization": f"Bearer {self.token}"}
        if self._api_key:
            return {"Authorization": f"Bearer {self._api_key}"}
        return {}

    def _get_domain(self):
        response = self.session.get(
            f"{self.BASE_URL}/domains",
            headers=self._auth_headers(),
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        response.raise_for_status()
        domains = response.json().get("hydra:member", [])
        if not domains:
            raise RuntimeError("duckmail returned no available domains")
        for item in domains:
            if item.get("domain") == "duckmail.sbs":
                return item["domain"]
        return domains[0]["domain"]

    def _ensure_domain(self):
        for attempt in range(1, DEFAULT_DOMAIN_RETRIES + 1):
            try:
                self.domain = self._get_domain()
                return True
            except Exception as e:
                print(
                    f"duckmail domain fetch failed ({attempt}/{DEFAULT_DOMAIN_RETRIES}): {e}"
                )
                if attempt < DEFAULT_DOMAIN_RETRIES:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
        return False

    def create_account(self, max_retries=DEFAULT_ACCOUNT_RETRIES):
        if not self.domain and not self._ensure_domain():
            return False

        for attempt in range(1, max_retries + 1):
            chars = string.ascii_letters + string.digits + "!@#$%^&*"
            self.password = "".join(random.choices(chars, k=16))
            self.password += random.choice(string.ascii_uppercase)
            self.password += random.choice(string.ascii_lowercase)
            self.password += random.choice(string.digits)
            self.password += random.choice("!@#$%^&*")
            self.password = "".join(random.sample(self.password, len(self.password)))

            username = "".join(random.choices(string.ascii_lowercase, k=10))
            self.address = f"{username}@{self.domain}"

            payload = {"address": self.address, "password": self.password}

            try:
                response = self.session.post(
                    f"{self.BASE_URL}/accounts",
                    json=payload,
                    headers=self._auth_headers(),
                    timeout=DEFAULT_HTTP_TIMEOUT_S,
                )
            except requests.RequestException as e:
                print(f"duckmail account create error ({attempt}/{max_retries}): {e}")
                if attempt < max_retries:
                    time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)
                continue

            if response.status_code == 201:
                self.account_id = response.json().get("id")
                return self._get_token()

            reason = response.text.strip().replace("\n", " ")[:200]
            print(
                f"duckmail account create failed ({attempt}/{max_retries}), "
                f"status={response.status_code}, body={reason}"
            )
            if attempt < max_retries:
                time.sleep(DEFAULT_RETRY_BASE_DELAY_S * attempt)

        return False

    def _get_token(self):
        payload = {"address": self.address, "password": self.password}
        response = self.session.post(
            f"{self.BASE_URL}/token",
            json=payload,
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        if response.status_code != 200:
            reason = response.text.strip().replace("\n", " ")[:200]
            print(
                f"duckmail token request failed: status={response.status_code}, body={reason}"
            )
            return False
        self.token = response.json().get("token")
        if not self.token:
            print("duckmail token request failed: empty token.")
            return False
        return True

    def wait_for_email(self, timeout=300, poll_interval=DEFAULT_EMAIL_POLL_INTERVAL):
        poll_interval = max(0.2, float(poll_interval))
        print(
            f"Waiting for email for {self.address} "
            f"(timeout: {timeout}s, poll: {poll_interval}s)..."
        )
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                response = self.session.get(
                    f"{self.BASE_URL}/messages",
                    headers=self._auth_headers(),
                    timeout=DEFAULT_HTTP_TIMEOUT_S,
                )
                response.raise_for_status()
                messages = response.json().get("hydra:member", [])
            except Exception as e:
                print(f"duckmail poll error: {e}")
                messages = []

            if messages:
                message_id = messages[0]["id"]
                return self._get_message_content(message_id)

            remaining = deadline - time.time()
            if remaining <= 0:
                break
            time.sleep(min(poll_interval, remaining))

        return None

    def _get_message_content(self, message_id):
        response = self.session.get(
            f"{self.BASE_URL}/messages/{message_id}",
            headers=self._auth_headers(),
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        response.raise_for_status()
        payload = response.json()
        html = payload.get("html") or []
        if html:
            return html[0]
        return payload.get("text") or ""
