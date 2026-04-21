# ruff: noqa: RUF001

import io
import time

import structlog
from aiogram import Bot, F, Router
from aiogram.filters import Command, CommandStart
from aiogram.filters.command import CommandObject
from aiogram.types import BufferedInputFile, Message

from whisper_bot.services.asr import AsrClient
from whisper_bot.services.auth import AuthService
from whisper_bot.services.llm import LLMService
from whisper_bot.utils import convert_audio_memory

logger = structlog.get_logger(__name__)

router = Router()

# Telegram messages cap at ~4096 characters; leave headroom for formatting and footer text.
TELEGRAM_TEXT_LIMIT = 3800


def _format_duration(seconds: float) -> str:
    m, s = divmod(seconds, 60)
    h, m = divmod(m, 60)
    return f"{int(h):02d}:{int(m):02d}:{s:05.2f}"


async def _send_text_with_limit(
    bot: Bot,
    target_message: Message,
    content: str,
    footer: str,
    filename: str,
) -> None:
    """Send text if under limit, otherwise fall back to a txt document."""

    formatted_text = f"```\n{content}\n```\n\n{footer}"

    if len(formatted_text) <= TELEGRAM_TEXT_LIMIT:
        await target_message.edit_text(formatted_text, parse_mode="Markdown")
        return

    await target_message.edit_text("文本较长，已作为文件发送。")
    document = BufferedInputFile(content.encode("utf-8"), filename=filename)
    if target_message.chat is None:
        raise ValueError("Chat missing on target message.")

    await bot.send_document(chat_id=target_message.chat.id, document=document, caption=footer)


@router.message(CommandStart())
async def start_command(message: Message):
    """Send a welcome message."""
    await message.answer(
        "👋 欢迎使用 Whisper 语音转文字机器人！\n\n请先使用 `/auth <password>` 进行认证，然后发送语音消息即可。",
        parse_mode="Markdown",
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
async def voice_message_handler(message: Message, bot: Bot, asr_client: AsrClient, llm_service: LLMService):
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
        wav_memory.seek(0)  # Rewind for Whisper

        # 3. Transcribe
        await processing_msg.edit_text("🔄 正在进行语音识别 (排队中)...")

        start_time = time.time()
        text = await asr_client.transcribe(wav_memory)
        duration = time.time() - start_time

        # 4. Reply raw result
        if not text:
            await processing_msg.edit_text("⚠️ 未能识别出文字。")
            return

        formatted_duration = _format_duration(duration)
        footer_text = f"🎙️ 由模型 {asr_client.model} 转录，耗时: {formatted_duration}"

        await _send_text_with_limit(
            bot=bot,
            target_message=processing_msg,
            content=text,
            footer=footer_text,
            filename="transcript.txt",
        )

        # 5. LLM Refinement
        if llm_service.is_enabled:
            refining_msg = await processing_msg.reply("✨ 正在进行智能润色...")
            refined_text, llm_duration = await llm_service.refine_text(text)

            llm_duration_str = _format_duration(llm_duration)
            llm_footer = f"✨ 由模型 {llm_service.model} 修正错别字并添加段落和标点 (耗时: {llm_duration_str})"
            await _send_text_with_limit(
                bot=bot,
                target_message=refining_msg,
                content=refined_text,
                footer=llm_footer,
                filename="refined_transcript.txt",
            )

    except Exception as e:
        logger.error(f"Error handling voice message: {e}")
        await processing_msg.edit_text(f"❌ 处理出错: {e!s}")
    finally:
        # Explicitly close buffers to free memory immediately
        audio_memory.close()
        if wav_memory:
            wav_memory.close()
