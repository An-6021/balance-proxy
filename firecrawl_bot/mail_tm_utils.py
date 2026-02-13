import requests
import time
import random
import string

DEFAULT_EMAIL_POLL_INTERVAL = 1.0


class MailTM:
    BASE_URL = "https://api.mail.tm"

    def __init__(self):
        self.session = requests.Session()
        self.domain = self._get_domain()
        self.address = None
        self.password = None
        self.token = None
        self.account_id = None

    def _get_domain(self):
        response = self.session.get(f"{self.BASE_URL}/domains")
        return response.json()['hydra:member'][0]['domain']

    def create_account(self):
        chars = string.ascii_letters + string.digits + "!@#$%^&*"
        self.password = ''.join(random.choices(chars, k=16))
        # Ensure at least one of each
        self.password += random.choice(string.ascii_uppercase)
        self.password += random.choice(string.ascii_lowercase)
        self.password += random.choice(string.digits)
        self.password += random.choice("!@#$%^&*")
        self.password = ''.join(random.sample(self.password, len(self.password)))
        
        username = ''.join(random.choices(string.ascii_lowercase, k=10))
        self.address = f"{username}@{self.domain}"
        
        payload = {
            "address": self.address,
            "password": self.password
        }
        response = self.session.post(f"{self.BASE_URL}/accounts", json=payload)
        if response.status_code == 201:
            self.account_id = response.json()['id']
            self._get_token()
            return True
        return False

    def _get_token(self):
        payload = {
            "address": self.address,
            "password": self.password
        }
        response = self.session.post(f"{self.BASE_URL}/token", json=payload)
        self.token = response.json()['token']
        self.session.headers.update({"Authorization": f"Bearer {self.token}"})

    def wait_for_email(self, timeout=300, poll_interval=DEFAULT_EMAIL_POLL_INTERVAL):
        poll_interval = max(0.2, float(poll_interval))
        print(
            f"Waiting for email for {self.address} "
            f"(timeout: {timeout}s, poll: {poll_interval}s)..."
        )
        deadline = time.time() + timeout
        while time.time() < deadline:
            response = self.session.get(f"{self.BASE_URL}/messages")
            messages = response.json()['hydra:member']
            if messages:
                message_id = messages[0]['id']
                return self._get_message_content(message_id)
            remaining = deadline - time.time()
            if remaining <= 0:
                break
            time.sleep(min(poll_interval, remaining))
        return None

    def _get_message_content(self, message_id):
        response = self.session.get(f"{self.BASE_URL}/messages/{message_id}")
        return response.json()['html'][0] # Usually the first HTML content
