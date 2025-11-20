from typing import Callable, Dict, Any, Awaitable
from aiogram import BaseMiddleware
from aiogram.types import Message
from src.services.auth import AuthService


class AuthMiddleware(BaseMiddleware):
    def __init__(self, auth_service: AuthService):
        self.auth_service = auth_service

    async def __call__(
        self, handler: Callable[[Message, Dict[str, Any]], Awaitable[Any]], event: Message, data: Dict[str, Any]
    ) -> Any:
        user = data.get("event_from_user")
        if not user:
            return await handler(event, data)

        # Allow /start and /auth commands to pass through without check
        if event.text and (event.text.startswith("/start") or event.text.startswith("/auth")):
            return await handler(event, data)

        # Check authentication
        if not self.auth_service.is_user_allowed(user.id):
            await event.answer("⛔️ 未授权，请先使用 `/auth <password>` 进行认证。", parse_mode="Markdown")
            return

        # If authorized, proceed
        return await handler(event, data)
