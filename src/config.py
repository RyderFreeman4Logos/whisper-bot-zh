from pathlib import Path
from typing import Literal
from pydantic_settings import BaseSettings, SettingsConfigDict

class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore"
    )

    # Required
    BOT_TOKEN: str
    ACCESS_PASSWORD: str

    # Optional with defaults
    SENSEVOICE_MODEL_PATH: Path = Path("models/sensevoice")
    MAX_CONCURRENT_TASKS: int = 1
    LOG_LEVEL: Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] = "INFO"
    
    # Derived paths (not set by env directly usually, but good to have)
    ALLOWED_USERS_FILE: Path = Path("data/allowed_users.json")
    TEMP_DIR: Path = Path("temp")

    def model_post_init(self, __context: object) -> None:
        # Ensure directories exist
        self.SENSEVOICE_MODEL_PATH.mkdir(parents=True, exist_ok=True)
        self.ALLOWED_USERS_FILE.parent.mkdir(parents=True, exist_ok=True)
        self.TEMP_DIR.mkdir(parents=True, exist_ok=True)

from functools import lru_cache

@lru_cache
def get_settings() -> Settings:
    return Settings()

# Global instance for easy access, but use lazy loading logic if possible. 
# However, for simplicity in most app parts, we often just want 'settings'.
# To support import-time safety, we will NOT instantiate it globally here 
# if we want strictly safe imports. But standard practice often accepts it.
# Given the test failure, let's just expose the class and the getter.
# If you really need a global variable, do:
# settings = get_settings() 
# But that brings back the crash. 
# So we will strictly use `get_settings()` in the app.
