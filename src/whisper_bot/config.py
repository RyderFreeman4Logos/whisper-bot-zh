from pathlib import Path
from typing import Literal, Optional
import os
from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache

class Settings(BaseSettings):
    # Allow overriding env_file via arguments to constructor
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
    LLM_MODEL: Optional[str] = None
    LLM_SYSTEM_PROMPT: str = (
        "你是一位精通中文的专业编辑。你的任务是接收一段语音转录的粗糙文本，对其进行润色。"
        "请执行以下操作：\n"
        "1. 修正错别字和明显的语音识别错误。\n"
        "2. 添加正确的标点符号。\n"
        "3. 根据语义进行合理的分段，使其易于阅读。\n"
        "4. 保持原意和语气不变，不要删减关键信息。\n"
        "请直接输出润色后的文本，不要包含任何解释或前缀。"
    )
    
    # LLM API Keys
    ANTHROPIC_API: Optional[str] = None
    GEMINI_API: Optional[str] = None
    GROQ_API: Optional[str] = None
    XAI_API: Optional[str] = None
    ZENMUX_API: Optional[str] = None
    
    # Paths Configuration
    # Default: ~/.config/whisper-bot-zh
    DATA_DIR: Path = Path.home() / ".config" / "whisper-bot-zh"
    # Default: ~/.cache/whisper-bot-zh
    CACHE_DIR: Path = Path.home() / ".cache" / "whisper-bot-zh"
    
    # These can now be set via env vars (e.g. ALLOWED_USERS_FILE=...)
    # or will default to structured paths based on DATA_DIR/CACHE_DIR
    ALLOWED_USERS_FILE: Optional[Path] = None 
    MODEL_DIR: Optional[Path] = None
    TEMP_DIR: Optional[Path] = None

    def model_post_init(self, __context: object) -> None:
        # Set defaults if not provided via env
        if self.ALLOWED_USERS_FILE is None:
            self.ALLOWED_USERS_FILE = self.DATA_DIR / "allowed_users.json"
        if self.MODEL_DIR is None:
            self.MODEL_DIR = self.CACHE_DIR / "models"
        if self.TEMP_DIR is None:
            self.TEMP_DIR = self.CACHE_DIR / "temp"
            
        # Ensure directories exist
        self.DATA_DIR.mkdir(parents=True, exist_ok=True)
        self.CACHE_DIR.mkdir(parents=True, exist_ok=True)
        if self.MODEL_DIR: self.MODEL_DIR.mkdir(parents=True, exist_ok=True)
        if self.TEMP_DIR: self.TEMP_DIR.mkdir(parents=True, exist_ok=True)
        if self.ALLOWED_USERS_FILE: self.ALLOWED_USERS_FILE.parent.mkdir(parents=True, exist_ok=True)

# Global variable to hold the singleton, can be reset
_settings_instance: Optional[Settings] = None

def get_settings(env_file: Optional[str] = None) -> Settings:
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
