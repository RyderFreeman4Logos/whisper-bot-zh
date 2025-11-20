import os
from pathlib import Path
from typing import Optional

from telegram import Update
from telegram.ext import ContextTypes
import structlog

from src.utils import convert_to_wav
from src.config import get_settings

logger = structlog.get_logger(__name__)
settings = get_settings()

async def start_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Send a welcome message."""
    if not update.message:
        return
    await update.message.reply_text(
        "👋 欢迎使用 SenseVoice 语音转文字机器人！\n\n"
        "请先使用 `/auth <password>` 进行认证，然后发送语音消息即可。",
        parse_mode="Markdown"
    )

async def auth_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle user authentication."""
    if not update.message or not update.effective_user:
        return

    if not context.args or len(context.args) < 1:
        await update.message.reply_text("⚠️ 请输入密码，格式：`/auth <password>`", parse_mode="Markdown")
        return

    password = context.args[0]
    user_id = update.effective_user.id
    auth_service = context.bot_data["auth_service"]

    if auth_service.authenticate_user(user_id, password):
        await update.message.reply_text("✅ 认证成功！您现在可以使用语音转文字服务了。", parse_mode="Markdown")
    else:
        await update.message.reply_text("❌ 认证失败，密码错误。", parse_mode="Markdown")

async def _check_auth(update: Update, context: ContextTypes.DEFAULT_TYPE) -> bool:
    """Check if user is authenticated."""
    if not update.effective_user:
        return False
        
    auth_service = context.bot_data["auth_service"]
    if not auth_service.is_user_allowed(update.effective_user.id):
        if update.message:
            await update.message.reply_text("⛔️ 未授权，请先使用 `/auth <password>` 进行认证。", parse_mode="Markdown")
        return False
    return True

async def voice_message_handler(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle voice and audio messages."""
    if not update.message:
        return

    if not await _check_auth(update, context):
        return

    # Determine file type
    voice = update.message.voice
    audio = update.message.audio
    
    # Prefer voice, then audio
    attachment = voice or audio
    if not attachment:
        return

    # Reply "Processing..."
    processing_msg = await update.message.reply_text("⏳ 正在接收并处理音频...", reply_to_message_id=update.message.message_id)

    try:
        # 1. Download file
        file_id = attachment.file_id
        new_file = await attachment.get_file()
        
        # Create temp file path
        # Use original extension if possible, or verify from mime_type
        # Telegram Voice is usually .ogg
        ext = ".ogg" if voice else Path(attachment.file_name or "audio").suffix or ".mp3"
        if not ext.startswith("."):
            ext = "." + ext
            
        temp_input_path = settings.TEMP_DIR / f"{file_id}{ext}"
        
        await new_file.download_to_drive(custom_path=temp_input_path)
        logger.info(f"Downloaded audio to {temp_input_path}")

        # 2. Convert to wav (if not already wav, but SenseVoice prefers 16k wav anyway)
        # We always run conversion to ensure 16k sample rate
        wav_path = convert_to_wav(temp_input_path)

        # 3. Transcribe
        asr_engine = context.bot_data["asr_engine"]
        # Update status
        await processing_msg.edit_text("🔄 正在进行语音识别 (排队中)...")
        
        text = await asr_engine.transcribe(wav_path)

        # 4. Reply result
        if not text:
            await processing_msg.edit_text("⚠️ 未能识别出文字。", parse_mode="Markdown")
        else:
            await processing_msg.edit_text(text)

        # Cleanup
        try:
            if temp_input_path.exists(): temp_input_path.unlink()
            if wav_path.exists() and wav_path != temp_input_path: wav_path.unlink()
        except Exception as e:
            logger.warning(f"Failed to clean up temp files: {e}")

    except Exception as e:
        logger.error(f"Error handling voice message: {e}")
        await processing_msg.edit_text(f"❌ 处理出错: {str(e)}")
