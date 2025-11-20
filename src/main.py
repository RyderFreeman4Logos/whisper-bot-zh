import logging
import sys
import asyncio

from aiogram import Bot, Dispatcher
from aiogram.client.session.aiohttp import AiohttpSession
import structlog

from src.config import get_settings

from src.services.auth import AuthService

from src.services.asr import WhisperEngine

from src.bot.handlers import router

from src.bot.middlewares import AuthMiddleware



# Configure logging

logging.basicConfig(

    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",

    level=logging.INFO

)

logger = structlog.get_logger(__name__)



async def main():

    settings = get_settings()

    

    # Set log level

    logging.getLogger().setLevel(settings.LOG_LEVEL)



    logger.info("Starting Whisper Bot (faster-whisper) with Aiogram...")



    # 1. Initialize Services

    try:

        auth_service = AuthService(

            storage_file=settings.ALLOWED_USERS_FILE,

            admin_password=settings.ACCESS_PASSWORD

        )

        

        asr_engine = WhisperEngine(

            model_size=settings.WHISPER_MODEL_SIZE,

            compute_type=settings.WHISPER_COMPUTE_TYPE,

            max_concurrent=settings.MAX_CONCURRENT_TASKS

        )

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
    # Register on router or dispatcher.
    # Use message middleware to intercept messages.
    dp.message.middleware(AuthMiddleware(auth_service))

    # 5. Register Routers
    dp.include_router(router)

    # 6. Run Polling
    # Pass services as kwargs to be injected into handlers
    logger.info("Bot is polling...")
    await dp.start_polling(bot, auth_service=auth_service, asr_engine=asr_engine)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except (KeyboardInterrupt, SystemExit):
        logger.info("Bot stopped!")
