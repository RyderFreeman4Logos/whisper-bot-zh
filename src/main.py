import logging
import sys
import socket
import os
from pathlib import Path
from typing import List

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
            new_pythonpath = f"{current_cwd}:{current_pythonpath}" if current_pythonpath else current_cwd
            os.environ["PYTHONPATH"] = new_pythonpath
            
            print(f"Updating LD_LIBRARY_PATH with NVIDIA libs and restarting...")
            # Re-execute the script with updated environment
            os.execv(sys.executable, [sys.executable] + sys.argv)
            
    except ImportError:
        pass 

_ensure_cuda_libs_in_ld_path()

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
            max_concurrent=settings.MAX_CONCURRENT_TASKS,
            initial_prompt=settings.WHISPER_INITIAL_PROMPT,
            vad_filter=settings.WHISPER_VAD_FILTER
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
    dp.message.middleware(AuthMiddleware(auth_service))

    # 5. Register Routers
    dp.include_router(router)

    # 6. Run Polling
    logger.info("Bot is polling...")
    await dp.start_polling(bot, auth_service=auth_service, asr_engine=asr_engine)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except (KeyboardInterrupt, SystemExit):
        logger.info("Bot stopped!")