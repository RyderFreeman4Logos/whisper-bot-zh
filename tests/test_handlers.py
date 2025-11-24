import io
from unittest.mock import AsyncMock, MagicMock, patch

# ruff: noqa: RUF001
import pytest
from aiogram.filters.command import CommandObject
from aiogram.types import Chat, Message, User

from whisper_bot.bot import handlers
from whisper_bot.bot.handlers import auth_command, voice_message_handler
from whisper_bot.services.asr import WhisperEngine
from whisper_bot.services.auth import AuthService


@pytest.fixture
def mock_message():
    message = AsyncMock(spec=Message)
    message.from_user = MagicMock(spec=User)
    message.from_user.id = 12345
    message.chat = MagicMock(spec=Chat)
    message.chat.id = 67890
    message.answer = AsyncMock()
    message.reply = AsyncMock()
    return message


@pytest.fixture
def mock_auth_service():
    return MagicMock(spec=AuthService)


@pytest.fixture
def mock_asr_engine():
    return MagicMock(spec=WhisperEngine)


@pytest.mark.asyncio
async def test_auth_command_success(mock_message, mock_auth_service):
    command = CommandObject(prefix="/", command="auth", args="correct_password")
    mock_auth_service.authenticate_user.return_value = True

    await auth_command(mock_message, command, mock_auth_service)

    mock_auth_service.authenticate_user.assert_called_with(12345, "correct_password")
    mock_message.answer.assert_called_with("✅ 认证成功！您现在可以使用语音转文字服务了。", parse_mode="Markdown")


@pytest.mark.asyncio
async def test_auth_command_fail(mock_message, mock_auth_service):
    command = CommandObject(prefix="/", command="auth", args="wrong_password")
    mock_auth_service.authenticate_user.return_value = False

    await auth_command(mock_message, command, mock_auth_service)

    mock_message.answer.assert_called_with("❌ 认证失败，密码错误。", parse_mode="Markdown")


@pytest.mark.asyncio
async def test_voice_handler_success(mock_message, mock_asr_engine):
    """Test basic ASR workflow without LLM."""
    mock_bot = AsyncMock()
    mock_file_info = MagicMock()
    mock_file_info.file_path = "voice.ogg"
    mock_bot.get_file.return_value = mock_file_info
    mock_bot.download_file = AsyncMock()

    mock_asr_engine.transcribe = AsyncMock(return_value="转写文本")
    mock_asr_engine.model_size = "large-v2"
    mock_asr_engine.compute_type = "int8"

    mock_llm_service = MagicMock()
    mock_llm_service.is_enabled = False

    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None

    processing_msg = AsyncMock()
    processing_msg.chat = mock_message.chat
    mock_message.reply.return_value = processing_msg

    mock_wav_buffer = io.BytesIO(b"fake_wav_data")

    with patch("whisper_bot.bot.handlers.convert_audio_memory", return_value=mock_wav_buffer):
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)

        mock_message.reply.assert_called()
        mock_asr_engine.transcribe.assert_called_with(mock_wav_buffer)

        calls = processing_msg.edit_text.call_args_list
        assert len(calls) >= 1
        args, _kwargs = calls[-1]
        text_sent = args[0]
        assert "```\n转写文本\n```" in text_sent
        assert "🎙️ 由 Whisper 模型" in text_sent


@pytest.mark.asyncio
async def test_voice_handler_with_llm(mock_message, mock_asr_engine):
    """Test ASR + LLM workflow."""
    mock_bot = AsyncMock()
    mock_file_info = MagicMock()
    mock_file_info.file_path = "voice.ogg"
    mock_bot.get_file.return_value = mock_file_info
    mock_bot.download_file = AsyncMock()

    mock_asr_engine.transcribe = AsyncMock(return_value="Raw Text")
    mock_asr_engine.model_size = "large-v2"
    mock_asr_engine.compute_type = "int8"

    mock_llm_service = MagicMock()
    mock_llm_service.is_enabled = True
    mock_llm_service.model = "gpt-test"
    # Return text AND duration
    mock_llm_service.refine_text = AsyncMock(return_value=("Refined Text", 1.5))

    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None

    processing_msg = AsyncMock()
    processing_msg.chat = mock_message.chat
    mock_message.reply.return_value = processing_msg
    # Simulate reply to processing msg returning a new message object for LLM step
    refining_msg = AsyncMock()
    refining_msg.chat = mock_message.chat
    processing_msg.reply.return_value = refining_msg

    mock_wav_buffer = io.BytesIO(b"fake_wav_data")

    with patch("whisper_bot.bot.handlers.convert_audio_memory", return_value=mock_wav_buffer):
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)

        # Check LLM call
        mock_llm_service.refine_text.assert_called_with("Raw Text")

        # Check Final Edit on refining_msg
        calls = refining_msg.edit_text.call_args_list
        assert len(calls) >= 1
        args, _kwargs = calls[-1]
        text_sent = args[0]

        assert "```\nRefined Text\n```" in text_sent
        assert "由模型 gpt-test 修正" in text_sent
        # 1.5 seconds -> 00:00:01.50
        assert "耗时: 00:00:01.50" in text_sent


@pytest.mark.asyncio
async def test_voice_handler_sends_file_when_text_too_long(monkeypatch, mock_message, mock_asr_engine):
    """Raw transcription should be sent as a file when it exceeds Telegram limit."""

    monkeypatch.setattr(handlers, "TELEGRAM_TEXT_LIMIT", 50)

    mock_bot = AsyncMock()
    mock_bot.send_document = AsyncMock()
    mock_file_info = MagicMock()
    mock_file_info.file_path = "voice.ogg"
    mock_bot.get_file.return_value = mock_file_info
    mock_bot.download_file = AsyncMock()

    long_text = "长" * 200
    mock_asr_engine.transcribe = AsyncMock(return_value=long_text)
    mock_asr_engine.model_size = "large-v2"
    mock_asr_engine.compute_type = "int8"

    mock_llm_service = MagicMock()
    mock_llm_service.is_enabled = False

    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None

    processing_msg = AsyncMock()
    processing_msg.chat = mock_message.chat
    mock_message.reply.return_value = processing_msg

    mock_wav_buffer = io.BytesIO(b"fake_wav_data")

    with patch("whisper_bot.bot.handlers.convert_audio_memory", return_value=mock_wav_buffer):
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)

        mock_bot.send_document.assert_awaited()
        _, kwargs = mock_bot.send_document.call_args
        assert kwargs["document"].filename == "transcript.txt"
        # The interim message should inform the user
        processing_msg.edit_text.assert_any_await("文本较长，已作为文件发送。")


@pytest.mark.asyncio
async def test_voice_handler_llm_result_too_long(monkeypatch, mock_message, mock_asr_engine):
    """LLM refinement should fall back to file when over the limit."""

    monkeypatch.setattr(handlers, "TELEGRAM_TEXT_LIMIT", 50)

    mock_bot = AsyncMock()
    mock_bot.send_document = AsyncMock()
    mock_file_info = MagicMock()
    mock_file_info.file_path = "voice.ogg"
    mock_bot.get_file.return_value = mock_file_info
    mock_bot.download_file = AsyncMock()

    mock_asr_engine.transcribe = AsyncMock(return_value="raw")
    mock_asr_engine.model_size = "large-v2"
    mock_asr_engine.compute_type = "int8"

    mock_llm_service = MagicMock()
    mock_llm_service.is_enabled = True
    mock_llm_service.model = "gpt-test"
    long_refined = "长" * 200
    mock_llm_service.refine_text = AsyncMock(return_value=(long_refined, 2.0))

    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None

    processing_msg = AsyncMock()
    processing_msg.chat = mock_message.chat
    mock_message.reply.return_value = processing_msg
    refining_msg = AsyncMock()
    refining_msg.chat = mock_message.chat
    processing_msg.reply.return_value = refining_msg

    mock_wav_buffer = io.BytesIO(b"fake_wav_data")

    with patch("whisper_bot.bot.handlers.convert_audio_memory", return_value=mock_wav_buffer):
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)

        mock_bot.send_document.assert_awaited()
        _, kwargs = mock_bot.send_document.call_args
        assert kwargs["document"].filename == "refined_transcript.txt"
        refining_msg.edit_text.assert_any_await("文本较长，已作为文件发送。")
