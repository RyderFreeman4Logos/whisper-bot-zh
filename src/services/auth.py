import json
from pathlib import Path
from typing import Set, Union

import structlog

logger = structlog.get_logger(__name__)

class AuthService:
    def __init__(self, storage_file: Union[str, Path], admin_password: str):
        self.storage_file = Path(storage_file)
        self.admin_password = admin_password
        self.allowed_users: Set[int] = set()
        self._load_users()

    def _load_users(self) -> None:
        if not self.storage_file.exists():
            self.allowed_users = set()
            return

        try:
            with open(self.storage_file, "r", encoding="utf-8") as f:
                data = json.load(f)
                if isinstance(data, list):
                    self.allowed_users = set(data)
                else:
                    logger.warning("Auth file format error, resetting allowed users.")
                    self.allowed_users = set()
        except (json.JSONDecodeError, IOError) as e:
            logger.error(f"Failed to load auth file: {e}")
            self.allowed_users = set()

    def _save_users(self) -> None:
        try:
            # Ensure directory exists
            self.storage_file.parent.mkdir(parents=True, exist_ok=True)
            
            # Atomic write via temp file usually safer, but simple open(w) is okay for now
            with open(self.storage_file, "w", encoding="utf-8") as f:
                json.dump(list(self.allowed_users), f)
        except IOError as e:
            logger.error(f"Failed to save auth file: {e}")

    def authenticate_user(self, user_id: int, password: str) -> bool:
        """
        Attempt to authenticate a user. If successful, persists the user ID.
        """
        if password == self.admin_password:
            if user_id not in self.allowed_users:
                self.allowed_users.add(user_id)
                self._save_users()
                logger.info(f"User {user_id} authenticated and saved.")
            return True
        
        logger.warning(f"Authentication failed for user {user_id}.")
        return False

    def is_user_allowed(self, user_id: int) -> bool:
        return user_id in self.allowed_users
