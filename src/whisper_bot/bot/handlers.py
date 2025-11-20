import os
import time
import io
from pathlib import Path

from aiogram import Router, F, Bot
from aiogram.filters import Command, CommandStart
from aiogram.types import Message
from aiogram.filters.command import CommandObject
import structlog

from whisper_bot.utils import convert_audio_memory
from whisper_bot.services.auth import AuthService
from whisper_bot.services.asr import WhisperEngine
from whisper_bot.services.llm import LLMService

logger = structlog.get_logger(__name__)

router = Router()

def _format_duration(seconds: float) -> str:
    m, s = divmod(seconds, 60)
    h, m = divmod(m, 60)
    return f"{int(h):02d}:{int(m):02d}:{s:05.2f}"

@router.message(CommandStart())
async def start_command(message: Message):
    """Send a welcome message."""
    await message.answer(
        "👋 欢迎使用 Whisper 语音转文字机器人！\n\n"  
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
        await message.answer("✅ 认证成功！您现在可以使用语音转文字服务了。", parse_mode="Markdown")
    else:
        await message.answer("❌ 认证失败，密码错误。", parse_mode="Markdown")

@router.message(F.voice | F.audio)
async def voice_message_handler(
    message: Message, 
    bot: Bot, 
    asr_engine: WhisperEngine, 
    llm_service: LLMService
):
    """Handle voice and audio messages entirely in memory."""
    # Prefer voice, then audio
    attachment = message.voice or message.audio
    if not attachment:
        return

    # Reply "Processing..."
    processing_msg = await message.reply("⏳ 正在接收并处理音频...")

    audio_memory = io.BytesIO()
    wav_memory = None

    try:
        # 1. Download file to memory
        file_id = attachment.file_id
        file_info = await bot.get_file(file_id)
        
        await bot.download_file(file_info.file_path, destination=audio_memory)
        audio_memory.seek(0)
        logger.info(f"Downloaded audio to memory ({len(audio_memory.getbuffer())} bytes)")

        # 2. Convert to wav (in-memory)
        # Read all bytes for conversion
        wav_memory = convert_audio_memory(audio_memory.read())
        wav_memory.seek(0) # Rewind for Whisper

        # 3. Transcribe
        await processing_msg.edit_text("🔄 正在进行语音识别 (排队中)...")
        
        start_time = time.time()
        text = await asr_engine.transcribe(wav_memory)
        duration = time.time() - start_time

        # 4. Reply raw result
        if not text:
            await processing_msg.edit_text("⚠️ 未能识别出文字。")
            return 
        else:
            formatted_duration = _format_duration(duration)
            footer_text = (
                f"🎙️ 由 Whisper 模型 ({asr_engine.model_size}) "
                f"以 {asr_engine.compute_type} 精度转录，耗时: {formatted_duration}"
            )
            await processing_msg.edit_text(
                f"```\n{text}\n```\n\n{footer_text}", 
                parse_mode="Markdown"
            )
        
        # 5. LLM Refinement
        if llm_service.is_enabled:
            refining_msg = await processing_msg.reply("✨ 正在进行智能润色...")
            refined_text = await llm_service.refine_text(text)
            
            llm_footer = f"✨ 由模型 {llm_service.model} 修正错别字并添加段落和标点"
            await refining_msg.edit_text(
                f"```\n{refined_text}\n```\n\n{llm_footer}", 
                parse_mode="Markdown"
            )

    except Exception as e:
        logger.error(f"Error handling voice message: {e}")
        await processing_msg.edit_text(f"❌ 处理出错: {str(e)}")
    finally:
        # Explicitly close buffers to free memory immediately
        audio_memory.close()
        if wav_memory:
            wav_memory.close()
