import argparse
import asyncio
import logging
import os
import socket
import sys
from pathlib import Path

import structlog
from aiogram import Bot, Dispatcher
from aiogram.client.session.aiohttp import AiohttpSession
from pydantic import ValidationError

from whisper_bot.bot.handlers import router
from whisper_bot.bot.middlewares import AuthMiddleware
from whisper_bot.config import get_settings, reset_settings
from whisper_bot.services.asr import AsrClient
from whisper_bot.services.auth import AuthService
from whisper_bot.services.llm import LLMService

# --- Patch: Force IPv4 ---
# Monkeypatch socket.getaddrinfo to force IPv4.
# Fixes connection timeouts when IPv6 is available but not routed correctly.
_old_getaddrinfo = socket.getaddrinfo


def _new_getaddrinfo(*args, **kwargs):
    responses = _old_getaddrinfo(*args, **kwargs)
    return [response for response in responses if response[0] == socket.AF_INET]


socket.getaddrinfo = _new_getaddrinfo

# Configure logging
logging.basicConfig(format="%(asctime)s - %(name)s - %(levelname)s - %(message)s", level=logging.INFO)
logger = structlog.get_logger(__name__)


def resolve_env_file(cli_env_file: str | None) -> Path | None:
    """
    Resolve configuration file in priority order:
    1. CLI argument (--env-file)
    2. Current working directory (.env)
    3. Default config directory (~/.config/whisper-bot-zh/.env)
    """
    if cli_env_file:
        path = Path(cli_env_file)
        if not path.exists():
            logger.warning(f"Specified config file not found: {path}")
        return path

    cwd_env = Path.cwd() / ".env"
    if cwd_env.exists():
        return cwd_env

    default_env = Path.home() / ".config" / "whisper-bot-zh" / ".env"
    if default_env.exists():
        return default_env

    return None


async def async_main(cli_env_file: str | None = None):
    # Reset settings to ensure fresh load with potential env overrides
    reset_settings()

    env_file_path = resolve_env_file(cli_env_file)

    try:
        env_path_str = str(env_file_path) if env_file_path else None
        settings = get_settings(env_file=env_path_str)
    except ValidationError:
        print("\n" + "=" * 60)
        print("❌ Configuration Error: Missing required settings")
        print("=" * 60)
        print("Please ensure you have configured 'BOT_TOKEN' and 'ACCESS_PASSWORD'.")
        print("You can do this via:")
        print("  1. An .env file in the current directory")
        print("  2. An .env file in ~/.config/whisper-bot-zh/.env")
        print("  3. Environment variables")
        print("=" * 60 + "\n")
        sys.exit(1)

    # Set log level
    logging.getLogger().setLevel(settings.LOG_LEVEL)

    logger.info("Starting Whisper Bot...")
    logger.info(f"Config File: {env_file_path or 'Env Vars / Default'}")
    logger.info(f"Data Dir: {settings.DATA_DIR}")
    logger.info(f"ASR Endpoint: {settings.ASR_BASE_URL} (model={settings.ASR_MODEL})")

    # 1. Initialize Services
    try:
        auth_service = AuthService(storage_file=settings.ALLOWED_USERS_FILE, admin_password=settings.ACCESS_PASSWORD)

        asr_client = AsrClient(
            base_url=settings.ASR_BASE_URL,
            api_key=settings.ASR_API_KEY,
            model=settings.ASR_MODEL,
            language=settings.ASR_LANGUAGE,
            prompt=settings.ASR_PROMPT,
            temperature=settings.ASR_TEMPERATURE,
            max_concurrent=settings.MAX_CONCURRENT_TASKS,
        )

        llm_service = LLMService(settings)

    except Exception as e:
        logger.critical(f"Failed to initialize services: {e}")
        sys.exit(1)

    # 2. Initialize Bot with Proxy Support
    session = None
    if settings.PROXY_URL:
        session = AiohttpSession(proxy=settings.PROXY_URL)
        logger.info(f"Using proxy: {settings.PROXY_URL}")

    bot = Bot(token=settings.BOT_TOKEN, session=session)

    # 3. Initialize Dispatcher
    dp = Dispatcher()

    # 4. Register Middlewares
    dp.message.middleware(AuthMiddleware(auth_service))

    # 5. Register Routers
    dp.include_router(router)

    # 6. Run Polling
    logger.info("Bot is polling...")
    await dp.start_polling(bot, auth_service=auth_service, asr_client=asr_client, llm_service=llm_service)


def main_cli():
    """Entry point for CLI."""
    parser = argparse.ArgumentParser(description="Whisper Telegram Bot")
    parser.add_argument("--env-file", help="Path to .env configuration file", default=None)
    parser.add_argument("--data-dir", help="Directory for persistent data (users.json)", default=None)

    args, _ = parser.parse_known_args()

    if args.data_dir:
        os.environ["DATA_DIR"] = str(Path(args.data_dir).resolve())

    try:
        asyncio.run(async_main(cli_env_file=args.env_file))
    except (KeyboardInterrupt, SystemExit):
        pass


if __name__ == "__main__":
    main_cli()
