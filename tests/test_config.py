from pathlib import Path

import pytest
from pydantic import ValidationError

from whisper_bot.config import Settings


def test_settings_defaults():
    """Test that settings have expected default values where applicable."""
    env_vars = {"BOT_TOKEN": "test_token", "ACCESS_PASSWORD": "test_password", "LOG_LEVEL": "INFO"}

    with pytest.MonkeyPatch.context() as mp:
        for k, v in env_vars.items():
            mp.setenv(k, v)

        settings = Settings()
        assert settings.BOT_TOKEN == "test_token"
        assert settings.ACCESS_PASSWORD == "test_password"
        assert settings.MAX_CONCURRENT_TASKS == 1
        assert settings.LOG_LEVEL == "INFO"

        # ASR defaults target Groq
        assert settings.ASR_BASE_URL == "https://api.groq.com/openai/v1"
        assert settings.ASR_MODEL == "whisper-large-v3"
        assert settings.ASR_LANGUAGE == "zh"
        assert settings.ASR_PROMPT == "以下是一段简体中文内容:"
        assert settings.ASR_TEMPERATURE == 0.0
        assert settings.ASR_API_KEY is None  # falls back at runtime

        # LLM tuning knobs
        assert settings.LLM_TEMPERATURE == 0.2
        assert settings.LLM_TOP_P is None
        assert settings.LLM_MAX_TOKENS is None

        # Paths
        assert settings.DATA_DIR == Path.home() / ".config" / "whisper-bot-zh"
        assert settings.CACHE_DIR == Path.home() / ".cache" / "whisper-bot-zh"
        assert settings.ALLOWED_USERS_FILE == settings.DATA_DIR / "allowed_users.json"
        assert settings.TEMP_DIR == settings.CACHE_DIR / "temp"


def test_settings_validation_error():
    """Test that missing required fields raises ValidationError."""

    class EmptyEnvSettings(Settings):
        model_config = {"env_file": None}

    with pytest.MonkeyPatch.context() as mp:
        mp.delenv("BOT_TOKEN", raising=False)
        mp.delenv("ACCESS_PASSWORD", raising=False)

        with pytest.raises(ValidationError):
            EmptyEnvSettings()


def test_settings_custom_values():
    """Test that environment variables override defaults."""
    with pytest.MonkeyPatch.context() as mp:
        mp.setenv("BOT_TOKEN", "custom_token")
        mp.setenv("ACCESS_PASSWORD", "custom_pass")
        mp.setenv("MAX_CONCURRENT_TASKS", "4")
        mp.setenv("ASR_BASE_URL", "http://localhost:9000/v1")
        mp.setenv("ASR_MODEL", "whisper-cpp")
        mp.setenv("ASR_LANGUAGE", "en")
        mp.setenv("ASR_PROMPT", "Custom Prompt")
        mp.setenv("ASR_TEMPERATURE", "0.2")
        mp.setenv("LLM_TEMPERATURE", "0.5")
        mp.setenv("LLM_TOP_P", "0.9")
        mp.setenv("LLM_MAX_TOKENS", "2048")

        # Custom paths
        mp.setenv("DATA_DIR", "/tmp/config_test")
        mp.setenv("CACHE_DIR", "/tmp/cache_test")

        settings = Settings()
        assert settings.MAX_CONCURRENT_TASKS == 4
        assert settings.ASR_BASE_URL == "http://localhost:9000/v1"
        assert settings.ASR_MODEL == "whisper-cpp"
        assert settings.ASR_LANGUAGE == "en"
        assert settings.ASR_PROMPT == "Custom Prompt"
        assert settings.ASR_TEMPERATURE == 0.2
        assert settings.LLM_TEMPERATURE == 0.5
        assert settings.LLM_TOP_P == 0.9
        assert settings.LLM_MAX_TOKENS == 2048

        assert settings.DATA_DIR == Path("/tmp/config_test")
        assert settings.CACHE_DIR == Path("/tmp/cache_test")
        assert settings.ALLOWED_USERS_FILE == Path("/tmp/config_test/allowed_users.json")
