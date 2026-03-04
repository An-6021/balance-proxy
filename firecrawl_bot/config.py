"""
firecrawl_bot 配置

优先级（高 -> 低）：
  1) 环境变量
  2) local_config.json（默认路径：firecrawl_bot/local_config.json，可用 FIRECRAWL_BOT_LOCAL_CONFIG 覆盖）
  3) 代码默认值
"""

import os
from typing import Any, Optional

from local_config import load_local_config


_LOCAL_CONFIG = load_local_config()


def _get_str(key: str, *, default: str = "", alias: Optional[str] = None) -> str:
    env_val = (os.environ.get(key) or "").strip()
    if env_val:
        return env_val

    for candidate in (key, alias) if alias else (key,):
        if not candidate:
            continue
        val = _LOCAL_CONFIG.get(candidate)
        if isinstance(val, str) and val.strip():
            return val.strip()

    return default


def _get_optional_str(
    key: str, *, default: Optional[str] = None, alias: Optional[str] = None
) -> Optional[str]:
    val = _get_str(key, default="", alias=alias)
    return val or default


def _get_any(key: str, *, default: Any = None, alias: Optional[str] = None) -> Any:
    env_val = (os.environ.get(key) or "").strip()
    if env_val:
        return env_val

    for candidate in (key, alias) if alias else (key,):
        if not candidate:
            continue
        if candidate in _LOCAL_CONFIG:
            return _LOCAL_CONFIG.get(candidate)

    return default


# ============== 邮箱服务配置 ==============

DUCKMAIL_API_KEY = _get_str("DUCKMAIL_API_KEY", alias="duckmail_api_key")

# ============== Tavily 注册配置 ==============

TAVILY_PASSWORD = _get_str("TAVILY_PASSWORD", default="TavilyBot2026!", alias="tavily_password")

# ============== OCR 配置 ==============

SILICON_FLOW_KEY = _get_str("SILICON_FLOW_KEY", alias="silicon_flow_key")
OCR_MODEL = _get_str(
    "OCR_MODEL",
    default="Qwen/Qwen3-VL-235B-A22B-Instruct",
    alias="ocr_model",
)

# ============== 代理配置 (可选) ==============

DEFAULT_PROXY = _get_optional_str("DEFAULT_PROXY", alias="default_proxy")
