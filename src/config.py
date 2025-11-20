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