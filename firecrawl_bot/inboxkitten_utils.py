import requests
import time
import random
import string

DEFAULT_EMAIL_POLL_INTERVAL = 1.0
DEFAULT_HTTP_TIMEOUT_S = 15
DEFAULT_ACCOUNT_RETRIES = 5

class InboxKitten:
    BASE_URL = "https://inboxkitten.com/api/v1"

    def __init__(self):
        self.session = requests.Session()
        self.domain = "inboxkitten.com"
        self.address = None
        self.password = None
        self.login = None

    def create_account(self, max_retries=DEFAULT_ACCOUNT_RETRIES):
        # Generate dummy password and login
        chars = string.ascii_letters + string.digits + "!@#$%^&*"
        self.password = ''.join(random.choices(chars, k=16))
        self.password += random.choice(string.ascii_uppercase)
        self.password += random.choice(string.ascii_lowercase)
        self.password += random.choice(string.digits)
        self.password += random.choice("!@#$%^&*")
        self.password = ''.join(random.sample(self.password, len(self.password)))

        self.login = ''.join(random.choices(string.ascii_lowercase, k=10))
        self.address = f"{self.login}@{self.domain}"

        # InboxKitten does not require account creation API calls. It's just address generation.
        return True

    def wait_for_email(self, timeout=300, poll_interval=DEFAULT_EMAIL_POLL_INTERVAL):
        poll_interval = max(0.5, float(poll_interval))
        print(
            f"Waiting for email for {self.address} "
            f"(timeout: {timeout}s, poll: {poll_interval}s)..."
        )
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                response = self.session.get(
                    f"{self.BASE_URL}/mail/list?recipient={self.login}",
                    timeout=DEFAULT_HTTP_TIMEOUT_S,
                )
                response.raise_for_status()
                messages = response.json()
            except Exception as e:
                print(f"InboxKitten poll error: {e}")
                messages = []
            
            if messages:
                # Messages are sorted newest first, pick the first one
                message_key = messages[0]['message']['key']
                return self._get_message_content(message_key)
                
            remaining = deadline - time.time()
            if remaining <= 0:
                break
            time.sleep(min(poll_interval, remaining))
        return None

    def _get_message_content(self, message_key):
        response = self.session.get(
            f"{self.BASE_URL}/mail/getHtml?mailKey={message_key}",
            timeout=DEFAULT_HTTP_TIMEOUT_S,
        )
        response.raise_for_status()
        return response.text
