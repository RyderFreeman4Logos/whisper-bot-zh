from pathlib import Path
from typing import Literal, Optional
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    # Required
    BOT_TOKEN: str
    ACCESS_PASSWORD: str

    # Optional with defaults
    WHISPER_MODEL_SIZE: str = "large-v2"
    WHISPER_COMPUTE_TYPE: str = "int8"
    WHISPER_INITIAL_PROMPT: str = "以下是一段简体中文内容:"
    WHISPER_VAD_FILTER: bool = True
    MAX_CONCURRENT_TASKS: int = 1
    LOG_LEVEL: Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] = "INFO"
    PROXY_URL: Optional[str] = None
    
    # LLM Settings
    LLM_MODEL: Optional[str] = None  # e.g. "gemini/gemini-2.5-flash"
    LLM_SYSTEM_PROMPT: str = (
        "你是一位精通中文的专业编辑。你的任务是接收一段语音转录的粗糙文本，对其进行润色。"
        "请执行以下操作：\n"
        "1. 修正错别字和明显的语音识别错误。\n"
        "2. 添加正确的标点符号。\n"
        "3. 根据语义进行合理的分段，使其易于阅读。\n"
        "4. 保持原意和语气不变，不要删减关键信息。\n"
        "请直接输出润色后的文本，不要包含任何解释或前缀。"
    )
    
    # LLM API Keys (Mapped from user preference)
    ANTHROPIC_API: Optional[str] = None
    GEMINI_API: Optional[str] = None
    GROQ_API: Optional[str] = None
    XAI_API: Optional[str] = None
    ZENMUX_API: Optional[str] = None
    
    # Derived paths (not set by env directly usually, but good to have)
    ALLOWED_USERS_FILE: Path = Path("data/allowed_users.json")
    TEMP_DIR: Path = Path("temp")

    def model_post_init(self, __context: object) -> None:
        # Ensure directories exist
        self.ALLOWED_USERS_FILE.parent.mkdir(parents=True, exist_ok=True)
        self.TEMP_DIR.mkdir(parents=True, exist_ok=True)

from functools import lru_cache


@lru_cache
def get_settings() -> Settings:
    return Settings()