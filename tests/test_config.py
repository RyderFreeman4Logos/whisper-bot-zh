import os
from pathlib import Path
import pytest
from pydantic import ValidationError

from src.config import Settings

def test_settings_defaults():
    """Test that settings have expected default values where applicable."""
    env_vars = {
        "BOT_TOKEN": "test_token",
        "ACCESS_PASSWORD": "test_password",
        "LOG_LEVEL": "INFO"
    }
    
    with pytest.MonkeyPatch.context() as mp:
        for k, v in env_vars.items():
            mp.setenv(k, v)
            
        settings = Settings()
        assert settings.BOT_TOKEN == "test_token"
        assert settings.ACCESS_PASSWORD == "test_password"
        assert settings.MAX_CONCURRENT_TASKS == 1
        assert settings.LOG_LEVEL == "INFO"
        assert settings.WHISPER_MODEL_SIZE == "large-v2"
        assert settings.WHISPER_COMPUTE_TYPE == "int8" # Updated default
        assert settings.WHISPER_VAD_FILTER is True
        assert settings.WHISPER_INITIAL_PROMPT == "以下是一段简体中文内容:"

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
        
        settings = Settings()
        assert settings.MAX_CONCURRENT_TASKS == 4
        assert settings.WHISPER_MODEL_SIZE == "medium"
        assert settings.WHISPER_COMPUTE_TYPE == "float32"
        assert settings.WHISPER_VAD_FILTER is False
        assert settings.WHISPER_INITIAL_PROMPT == "Custom Prompt"
