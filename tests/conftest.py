import os
import pytest

# Set env vars immediately upon module load to satisfy pydantic validation during collection
os.environ.setdefault("BOT_TOKEN", "test_token")
os.environ.setdefault("ACCESS_PASSWORD", "test_password")
os.environ.setdefault("SENSEVOICE_MODEL_PATH", "models/sensevoice")


@pytest.fixture(scope="session", autouse=True)
def set_test_env():
    """Set required environment variables for tests (redundant but keeps intent clear)."""
    yield
