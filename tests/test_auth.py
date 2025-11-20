import pytest
import json
from pathlib import Path
from whisper_bot.services.auth import AuthService


@pytest.fixture
def auth_service(tmp_path):
    """Fixture to provide an AuthService with a temporary storage file."""
    storage_file = tmp_path / "allowed_users.json"
    password = "secret_password"
    return AuthService(storage_file=storage_file, admin_password=password)


def test_initial_state(auth_service):
    assert not auth_service.is_user_allowed(12345)


def test_authentication_success(auth_service):
    user_id = 12345
    assert auth_service.authenticate_user(user_id, "secret_password") is True
    assert auth_service.is_user_allowed(user_id) is True


def test_authentication_failure(auth_service):
    user_id = 67890
    assert auth_service.authenticate_user(user_id, "wrong_password") is False
    assert not auth_service.is_user_allowed(user_id)


def test_persistence(tmp_path):
    storage_file = tmp_path / "allowed_users.json"
    password = "secret_password"

    # First instance: add user
    service1 = AuthService(storage_file=storage_file, admin_password=password)
    service1.authenticate_user(111, password)

    # Second instance: should remember user
    service2 = AuthService(storage_file=storage_file, admin_password=password)
    assert service2.is_user_allowed(111) is True


def test_add_already_allowed_user(auth_service):
    auth_service.authenticate_user(123, "secret_password")
    # Authenticate again should still return True
    assert auth_service.authenticate_user(123, "secret_password") is True
    # Storage should still be valid
    assert auth_service.is_user_allowed(123)
