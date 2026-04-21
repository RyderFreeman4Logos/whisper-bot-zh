from pathlib import Path
from typing import Literal

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    # Allow overriding env_file via arguments to constructor
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    # Required
    BOT_TOKEN: str
    ACCESS_PASSWORD: str

    # Runtime
    MAX_CONCURRENT_TASKS: int = 1
    LOG_LEVEL: Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] = "INFO"
    PROXY_URL: str | None = None

    # ASR (OpenAI-compatible transcription endpoint; Groq by default)
    ASR_BASE_URL: str = "https://api.groq.com/openai/v1"
    ASR_API_KEY: str | None = None  # falls back to GROQ_API_KEY / OPENAI_API_KEY env at runtime
    ASR_MODEL: str = "whisper-large-v3"
    ASR_LANGUAGE: str = "zh"
    ASR_PROMPT: str = "以下是一段简体中文内容:"
    ASR_TEMPERATURE: float = 0.0

    # LLM Settings
    LLM_MODEL: str | None = None
    LLM_SYSTEM_PROMPT: str = (
        "你是一个严格的中文语音转写润色器。\n\n"
        "输入：一段由语音识别得到的原始文本。\n"
        "输出：且仅输出对输入文本的润色版本。\n\n"
        "润色 = 只做以下 3 件事：\n"
        "1. 改正错别字和语音识别错误（同音字、音近字）。\n"
        "2. 补上合理的标点符号。\n"
        "3. 按语义分段。\n\n"
        "严禁（出现即视为失败）：\n"
        "- 添加任何解释、建议、补充、点评、总结、备注、注释、推测、延伸、参考。\n"
        "- 改写原意、删减关键信息、补全原文没说完的话。\n"
        '- 输出前言（"好的"、"以下是..." 之类）。\n'
        "- 输出结束标记或结语。\n\n"
        "原文讲到哪，你就润色到哪；原文结束，你立即停止输出，不再多写一个字。"
    )
    LLM_TEMPERATURE: float = 0.2
    LLM_TOP_P: float | None = None
    LLM_MAX_TOKENS: int | None = None

    # LLM API Keys (for litellm provider dispatch)
    ANTHROPIC_API: str | None = None
    GEMINI_API: str | None = None
    GROQ_API: str | None = None
    XAI_API: str | None = None
    ZENMUX_API: str | None = None

    # Paths Configuration
    # Default: ~/.config/whisper-bot-zh
    DATA_DIR: Path = Path.home() / ".config" / "whisper-bot-zh"
    # Default: ~/.cache/whisper-bot-zh
    CACHE_DIR: Path = Path.home() / ".cache" / "whisper-bot-zh"

    ALLOWED_USERS_FILE: Path | None = None
    TEMP_DIR: Path | None = None

    def model_post_init(self, __context: object) -> None:
        # Set defaults if not provided via env
        if self.ALLOWED_USERS_FILE is None:
            self.ALLOWED_USERS_FILE = self.DATA_DIR / "allowed_users.json"
        if self.TEMP_DIR is None:
            self.TEMP_DIR = self.CACHE_DIR / "temp"

        # Ensure directories exist
        self.DATA_DIR.mkdir(parents=True, exist_ok=True)
        self.CACHE_DIR.mkdir(parents=True, exist_ok=True)
        if self.TEMP_DIR:
            self.TEMP_DIR.mkdir(parents=True, exist_ok=True)
        if self.ALLOWED_USERS_FILE:
            self.ALLOWED_USERS_FILE.parent.mkdir(parents=True, exist_ok=True)


# Global variable to hold the singleton, can be reset
_settings_instance: Settings | None = None


def get_settings(env_file: str | None = None) -> Settings:
    global _settings_instance
    if _settings_instance is None:
        # If env_file is provided, use it.
        # Otherwise pydantic uses default ".env" or env vars.
        kw = {"_env_file": env_file} if env_file else {}
        _settings_instance = Settings(**kw)
    return _settings_instance


def reset_settings():
    global _settings_instance
    _settings_instance = None
