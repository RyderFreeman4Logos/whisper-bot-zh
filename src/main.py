import logging
import sys
from pathlib import Path

from telegram.ext import ApplicationBuilder, CommandHandler, MessageHandler, filters
import structlog

from src.config import get_settings
from src.services.auth import AuthService
from src.services.asr import SenseVoiceEngine
from src.bot.handlers import start_command, auth_command, voice_message_handler

# Configure logging
logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO
)
logger = structlog.get_logger(__name__)

def main():
    settings = get_settings()
    
    # Set log level
    logging.getLogger().setLevel(settings.LOG_LEVEL)

    logger.info("Starting Whisper Bot (SenseVoice)...")
    logger.info(f"Model Path: {settings.SENSEVOICE_MODEL_PATH}")
    logger.info(f"Max Concurrent Tasks: {settings.MAX_CONCURRENT_TASKS}")

    # 1. Initialize Services
    try:
        auth_service = AuthService(
            storage_file=settings.ALLOWED_USERS_FILE,
            admin_password=settings.ACCESS_PASSWORD
        )
        
        asr_engine = SenseVoiceEngine(
            model_path=settings.SENSEVOICE_MODEL_PATH,
            max_concurrent=settings.MAX_CONCURRENT_TASKS
        )
    except Exception as e:
        logger.critical(f"Failed to initialize services: {e}")
        sys.exit(1)

    # 2. Build Application
    application = ApplicationBuilder().token(settings.BOT_TOKEN).build()

    # 3. Inject Dependencies
    application.bot_data["auth_service"] = auth_service
    application.bot_data["asr_engine"] = asr_engine

    # 4. Register Handlers
    application.add_handler(CommandHandler("start", start_command))
    application.add_handler(CommandHandler("auth", auth_command))
    
    # Voice and Audio files
    voice_filter = filters.VOICE | filters.AUDIO
    application.add_handler(MessageHandler(voice_filter, voice_message_handler))

    # 5. Run
    logger.info("Bot is polling...")
    application.run_polling()

if __name__ == "__main__":
    main()
