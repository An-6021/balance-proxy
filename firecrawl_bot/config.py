"""
配置文件 - 管理所有 API Keys 和敏感信息
"""

# ============== 邮箱服务配置 ==============

# DuckMail API Key (必需)
DUCKMAIL_API_KEY = "dk_1377e081e2c45bf17ae2c66c27b25bde4c3d386edd0a25cedcfda1d2162662c2"

# ============== Tavily 注册配置 ==============

# 注册密码
TAVILY_PASSWORD = "TavilyBot2026!"

# ============== OCR 配置 ==============

# 硅基流动 API Key (用于验证码识别)
SILICON_FLOW_KEY = "sk-gmoldzqdqyzapzsdqifmwmyyqiehkhnhgcdtarhotyhukbzt"

# OCR 模型
OCR_MODEL = "Qwen/Qwen3-VL-235B-A22B-Instruct"

# ============== 代理配置 (可选) ==============

# 默认代理 (设为 None 禁用)
# 格式: "http://127.0.0.1:7890" 或 "socks5://127.0.0.1:1080"
DEFAULT_PROXY = None
