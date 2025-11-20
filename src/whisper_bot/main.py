import logging
import sys
import socket
import os
import argparse
from pathlib import Path
from typing import List, Optional

# --- Patch 1: Force IPv4 ---
# Monkeypatch socket.getaddrinfo to force IPv4
# This fixes connection timeouts when IPv6 is available but not routed correctly
_old_getaddrinfo = socket.getaddrinfo
def _new_getaddrinfo(*args, **kwargs):
    responses = _old_getaddrinfo(*args, **kwargs)
    # Filter for IPv4 family (AF_INET)
    return [response for response in responses if response[0] == socket.AF_INET]
socket.getaddrinfo = _new_getaddrinfo

# --- Patch 2: Link NVIDIA libs for CTranslate2/Faster-Whisper ---
# faster-whisper needs libcudnn.so and libcublas.so. 
def _ensure_cuda_libs_in_ld_path():
    try:
        import nvidia.cudnn
        import nvidia.cublas
        
        # nvidia.* packages are namespace packages, use __path__
        cudnn_dir = list(nvidia.cudnn.__path__)[0]
        cublas_dir = list(nvidia.cublas.__path__)[0]
        
        lib_paths = [
            os.path.join(cudnn_dir, "lib"),
            os.path.join(cublas_dir, "lib")
        ]
        
        current_ld = os.environ.get("LD_LIBRARY_PATH", "")
        missing_paths = [p for p in lib_paths if p not in current_ld]
        
        if missing_paths:
            new_ld = ":".join(lib_paths + ([current_ld] if current_ld else []))
            os.environ["LD_LIBRARY_PATH"] = new_ld
            
            # Ensure PYTHONPATH includes CWD so re-exec works even if called as script
            current_cwd = os.getcwd()
            current_pythonpath = os.environ.get("PYTHONPATH", "")
            # Only add CWD if running with -m whisper_bot.main pattern, otherwise installation usually handles path
            if "whisper_bot.main" in sys.argv[0]: 
                new_pythonpath = f"{current_cwd}:{current_pythonpath}" if current_pythonpath else current_cwd
                os.environ["PYTHONPATH"] = new_pythonpath
            
            print(f"Updating LD_LIBRARY_PATH with NVIDIA libs and restarting...")
            print(f"Restarting command: {sys.argv}")
            # Use execvp to search PATH for the executable (handles 'whisper-bot-zh' command)
            # sys.argv[0] is the program name.
            try:
                os.execvp(sys.argv[0], sys.argv)
            except FileNotFoundError:
                # Fallback for python -m usage where argv[0] might be full path
                os.execv(sys.executable, [sys.executable] + sys.argv)
            
    except ImportError:
        pass 
    except Exception as e:
        print(f"Warning: Failed to patch LD_LIBRARY_PATH: {e}")

_ensure_cuda_libs_in_ld_path()

import asyncio
from aiogram import Bot, Dispatcher
from aiogram.client.session.aiohttp import AiohttpSession
import structlog

from whisper_bot.config import get_settings, reset_settings
from whisper_bot.services.auth import AuthService
from whisper_bot.services.asr import WhisperEngine
from whisper_bot.services.llm import LLMService
from whisper_bot.bot.handlers import router
from whisper_bot.bot.middlewares import AuthMiddleware

# Configure logging
logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO
)
logger = structlog.get_logger(__name__)

async def async_main(env_file: Optional[str] = None):
    # Reset settings to ensure fresh load with potential env overrides
    reset_settings()
    settings = get_settings(env_file=env_file)
    
    # Set log level
    logging.getLogger().setLevel(settings.LOG_LEVEL)

    logger.info(f"Starting Whisper Bot...")
    logger.info(f"Config File: {env_file or 'Default (.env or env vars)'}")
    logger.info(f"Model Dir: {settings.MODEL_DIR}")
    logger.info(f"Data Dir: {settings.DATA_DIR}")

    # 1. Initialize Services
    try:
        auth_service = AuthService(
            storage_file=settings.ALLOWED_USERS_FILE,
            admin_password=settings.ACCESS_PASSWORD
        )
        
        asr_engine = WhisperEngine(
            model_size=settings.WHISPER_MODEL_SIZE,
            compute_type=settings.WHISPER_COMPUTE_TYPE,
            max_concurrent=settings.MAX_CONCURRENT_TASKS,
            initial_prompt=settings.WHISPER_INITIAL_PROMPT,
            vad_filter=settings.WHISPER_VAD_FILTER
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
    await dp.start_polling(
        bot, 
        auth_service=auth_service, 
        asr_engine=asr_engine,
        llm_service=llm_service
    )

def main_cli():
    """Entry point for CLI."""
    parser = argparse.ArgumentParser(description="Whisper Telegram Bot")
    parser.add_argument("--env-file", help="Path to .env configuration file", default=None)
    parser.add_argument("--model-dir", help="Directory to store Whisper models", default=None)
    parser.add_argument("--data-dir", help="Directory for persistent data (users.json)", default=None)
    
    # Parse known args to avoid issues if aiogram or other libs try to parse args (unlikely but safe)
    args, _ = parser.parse_known_args()
    
    # Set environment variables for Config to pick up
    if args.model_dir:
        os.environ["MODEL_DIR"] = str(Path(args.model_dir).resolve())
    if args.data_dir:
        os.environ["DATA_DIR"] = str(Path(args.data_dir).resolve())

    try:
        asyncio.run(async_main(env_file=args.env_file))
    except (KeyboardInterrupt, SystemExit):
        logger.info("Bot stopped!")

if __name__ == "__main__":
    main_cli()