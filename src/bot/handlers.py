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

from src.services.asr import WhisperEngine

from src.services.llm import LLMService



logger = structlog.get_logger(__name__)

settings = get_settings()



router = Router()

# ... (start_command and auth_command remain unchanged) ...



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

        await message.answer("✅ 认证成功！您现在可以使用语音转文字服务了。")

    else:

        await message.answer("❌ 认证失败，密码错误。")



import os



import time



from pathlib import Path







from aiogram import Router, F, Bot



# ... (imports) ...







def _format_duration(seconds: float) -> str:



    m, s = divmod(seconds, 60)



    h, m = divmod(m, 60)



    return f"{int(h):02d}:{int(m):02d}:{s:05.2f}"







@router.message(F.voice | F.audio)



async def voice_message_handler(



    message: Message, 



    bot: Bot, 



    asr_engine: WhisperEngine, 



    llm_service: LLMService



):



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



        



        ext = Path(file_info.file_path).suffix if file_info.file_path else ""



        if not ext:



             ext = ".ogg" if message.voice else ".mp3"



            



        temp_input_path = settings.TEMP_DIR / f"{file_id}{ext}"



        



        await bot.download_file(file_info.file_path, destination=temp_input_path)



        logger.info(f"Downloaded audio to {temp_input_path}")







        # 2. Convert to wav



        wav_path = convert_to_wav(temp_input_path)







        # 3. Transcribe with timing



        await processing_msg.edit_text("🔄 正在进行语音识别 (排队中)...")



        



        start_time = time.time()



        text = await asr_engine.transcribe(wav_path)



        duration = time.time() - start_time







        # 4. Reply raw result



        if not text:



            await processing_msg.edit_text("⚠️ 未能识别出文字。")



            return 



        else:



            # Use Markdown code block for text, metadata outside



            formatted_duration = _format_duration(duration)



            await processing_msg.edit_text(



                f"```\n{text}\n```\n\n⏱️ 耗时: {formatted_duration}", 



                parse_mode="Markdown"



            )



        



        # 5. LLM Refinement



        if llm_service.is_enabled:



            refining_msg = await processing_msg.reply("✨ 正在进行智能润色...")



            refined_text = await llm_service.refine_text(text)



            



            await refining_msg.edit_text(



                f"```\n{refined_text}\n```\n\n🤖 模型: {llm_service.model}", 



                parse_mode="Markdown"



            )







        # Cleanup

        try:

            if temp_input_path.exists(): temp_input_path.unlink()

            if wav_path.exists() and wav_path != temp_input_path: wav_path.unlink()

        except Exception as e:

            logger.warning(f"Failed to clean up temp files: {e}")



    except Exception as e:

        logger.error(f"Error handling voice message: {e}")

        await processing_msg.edit_text(f"❌ 处理出错: {str(e)}")
