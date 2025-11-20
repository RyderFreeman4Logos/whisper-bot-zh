import os
from pathlib import Path
import pytest
from pydantic import ValidationError

# We need to import Settings after setting env vars in some tests,
# so we might need to reload or patch.
# Easier approach for pydantic-settings is to instantiate the class directly.
from src.config import Settings


def test_settings_defaults():
    """Test that settings have expected default values where applicable."""
    # To test defaults, we must ensure required env vars are set or we mock them.
    # Let's mock the minimal required env vars.
    os.environ["BOT_TOKEN"] = "test_token"
    os.environ["ACCESS_PASSWORD"] = "test_password"

    settings = Settings()
    assert settings.BOT_TOKEN == "test_token"
    assert settings.ACCESS_PASSWORD == "test_password"
    assert settings.MAX_CONCURRENT_TASKS == 1  # Default
    assert settings.LOG_LEVEL == "INFO"  # Default

    # Cleanup
    del os.environ["BOT_TOKEN"]
    del os.environ["ACCESS_PASSWORD"]


def test_settings_validation_error():
    """Test that missing required fields raises ValidationError."""
    # Ensure env is clean
    if "BOT_TOKEN" in os.environ:
        del os.environ["BOT_TOKEN"]
    if "ACCESS_PASSWORD" in os.environ:
        del os.environ["ACCESS_PASSWORD"]

    with pytest.raises(ValidationError):
        Settings()


def test_settings_custom_values(tmp_path):
    """Test that environment variables override defaults."""
    custom_model_path = tmp_path / "custom_models"

    os.environ["BOT_TOKEN"] = "custom_token"
    os.environ["ACCESS_PASSWORD"] = "custom_pass"
    os.environ["MAX_CONCURRENT_TASKS"] = "4"
    os.environ["SENSEVOICE_MODEL_PATH"] = str(custom_model_path)

    settings = Settings()
    assert settings.MAX_CONCURRENT_TASKS == 4
    assert settings.SENSEVOICE_MODEL_PATH == custom_model_path
    assert custom_model_path.exists()  # Check if dir was created

    # Cleanup
    del os.environ["BOT_TOKEN"]
    del os.environ["ACCESS_PASSWORD"]
    del os.environ["MAX_CONCURRENT_TASKS"]
    del os.environ["SENSEVOICE_MODEL_PATH"]
