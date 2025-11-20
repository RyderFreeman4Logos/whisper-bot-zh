import os
from pathlib import Path

from aiogram import Router, F, Bot
from aiogram.filters import Command, CommandStart
from aiogram.types import Message
from aiogram.filters.command import CommandObject
import structlog

from src.utils import convert_to_wav
from src.config import get_settings
from src.services.auth import AuthService
from src.services.asr import SenseVoiceEngine

logger = structlog.get_logger(__name__)
settings = get_settings()

router = Router()

@router.message(CommandStart())
async def start_command(message: Message):
    """Send a welcome message."""
    await message.answer(
        "👋 欢迎使用 SenseVoice 语音转文字机器人！\n\n" 
        "请先使用 `/auth <password>` 进行认证，然后发送语音消息即可。",
        parse_mode="Markdown"
    )

@router.message(Command("auth"))
async def auth_command(message: Message, command: CommandObject, auth_service: AuthService):
    """Handle user authentication."""
    if not command.args:
        await message.answer("⚠️ 请输入密码，格式：`/auth <password>`", parse_mode="Markdown")
        return

    password = command.args
    user_id = message.from_user.id

    if auth_service.authenticate_user(user_id, password):
        await message.answer("✅ 认证成功！您现在可以使用语音转文字服务了。" )
    else:
        await message.answer("❌ 认证失败，密码错误。" )

@router.message(F.voice | F.audio)
async def voice_message_handler(message: Message, bot: Bot, asr_engine: SenseVoiceEngine):
    """Handle voice and audio messages."""
    # Prefer voice, then audio
    attachment = message.voice or message.audio
    if not attachment:
        return

    # Reply "Processing..."
    processing_msg = await message.reply("⏳ 正在接收并处理音频...")

    try:
        # 1. Download file
        file_id = attachment.file_id
        file_info = await bot.get_file(file_id)
        
        # Determine extension
        # telegram file_path often has extension. 
        # If not, fallback.
        ext = Path(file_info.file_path).suffix if file_info.file_path else ""
        if not ext:
             ext = ".ogg" if message.voice else ".mp3"
            
        temp_input_path = settings.TEMP_DIR / f"{file_id}{ext}"
        
        await bot.download_file(file_info.file_path, destination=temp_input_path)
        logger.info(f"Downloaded audio to {temp_input_path}")

        # 2. Convert to wav
        wav_path = convert_to_wav(temp_input_path)

        # 3. Transcribe
        await processing_msg.edit_text("🔄 正在进行语音识别 (排队中)...")
        text = await asr_engine.transcribe(wav_path)

        # 4. Reply result
        if not text:
            await processing_msg.edit_text("⚠️ 未能识别出文字。" )
        else:
            # Use Markdown code block for monospaced font and easy copying
            await processing_msg.edit_text(f"```\n{text}\n```", parse_mode="Markdown")

        # Cleanup
        try:
            if temp_input_path.exists(): temp_input_path.unlink()
            if wav_path.exists() and wav_path != temp_input_path: wav_path.unlink()
        except Exception as e:
            logger.warning(f"Failed to clean up temp files: {e}")

    except Exception as e:
        logger.error(f"Error handling voice message: {e}")
        await processing_msg.edit_text(f"❌ 处理出错: {str(e)}")
