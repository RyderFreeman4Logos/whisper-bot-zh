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
        assert settings.WHISPER_MODEL_SIZE == "large-v2"
        assert settings.WHISPER_COMPUTE_TYPE == "int8"
        assert settings.WHISPER_VAD_FILTER is True
        assert settings.WHISPER_INITIAL_PROMPT == "以下是一段简体中文内容:"

        # Verify paths
        assert settings.DATA_DIR == Path.home() / ".config" / "whisper-bot-zh"
        assert settings.CACHE_DIR == Path.home() / ".cache" / "whisper-bot-zh"
        assert settings.ALLOWED_USERS_FILE == settings.DATA_DIR / "allowed_users.json"
        assert settings.MODEL_DIR == settings.CACHE_DIR / "models"
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
        mp.setenv("WHISPER_MODEL_SIZE", "medium")
        mp.setenv("WHISPER_COMPUTE_TYPE", "float32")
        mp.setenv("WHISPER_VAD_FILTER", "False")
        mp.setenv("WHISPER_INITIAL_PROMPT", "Custom Prompt")

        # Custom paths
        mp.setenv("DATA_DIR", "/tmp/config_test")
        mp.setenv("CACHE_DIR", "/tmp/cache_test")
        mp.setenv("MODEL_DIR", "/tmp/models_test")

        settings = Settings()
        assert settings.MAX_CONCURRENT_TASKS == 4
        assert settings.WHISPER_MODEL_SIZE == "medium"
        assert settings.WHISPER_COMPUTE_TYPE == "float32"
        assert settings.WHISPER_VAD_FILTER is False
        assert settings.WHISPER_INITIAL_PROMPT == "Custom Prompt"

        assert settings.DATA_DIR == Path("/tmp/config_test")
        assert settings.CACHE_DIR == Path("/tmp/cache_test")
        assert settings.MODEL_DIR == Path("/tmp/models_test")
        # Derived defaults should follow base
        assert settings.ALLOWED_USERS_FILE == Path("/tmp/config_test/allowed_users.json")
