import json
import os
from typing import Any, Dict


LOCAL_CONFIG_PATH = os.environ.get("FIRECRAWL_BOT_LOCAL_CONFIG") or os.path.join(
    os.path.dirname(__file__), "local_config.json"
)


def load_local_config() -> Dict[str, Any]:
    try:
        with open(LOCAL_CONFIG_PATH, "r", encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def save_local_config(data: Dict[str, Any]) -> bool:
    try:
        tmp_path = f"{LOCAL_CONFIG_PATH}.tmp"
        with open(tmp_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
            f.write("\n")
        os.replace(tmp_path, LOCAL_CONFIG_PATH)
        try:
            os.chmod(LOCAL_CONFIG_PATH, 0o600)
        except Exception:
            pass
        return True
    except Exception:
        return False

