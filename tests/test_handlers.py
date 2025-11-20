import pytest
import io
from unittest.mock import AsyncMock, MagicMock, patch, ANY
from aiogram.types import Message, User, Chat
from aiogram.filters.command import CommandObject

from whisper_bot.bot.handlers import auth_command, voice_message_handler
from whisper_bot.services.auth import AuthService
from whisper_bot.services.asr import WhisperEngine

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
    # Mock dependencies
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
    
    # Setup message
    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None
    
    # Setup processing message
    processing_msg = AsyncMock()
    mock_message.reply.return_value = processing_msg
    
    # Run handler
    # Mock convert_audio_memory to return a dummy buffer
    mock_wav_buffer = io.BytesIO(b"fake_wav_data")
    
    with patch("whisper_bot.bot.handlers.convert_audio_memory", return_value=mock_wav_buffer) as mock_convert:
        
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)
        
        # Verify
        mock_message.reply.assert_called()
        mock_bot.get_file.assert_called_with("file_123")
        # Check that download_file was called with destination=BytesIO
        args, kwargs = mock_bot.download_file.call_args
        assert isinstance(kwargs['destination'], io.BytesIO)
        
        # Check conversion was called with bytes
        mock_convert.assert_called()
        
        # Check transcribe was called with buffer
        mock_asr_engine.transcribe.assert_called_with(mock_wav_buffer)
        
        # Verify edit_text calls
        calls = processing_msg.edit_text.call_args_list
        assert len(calls) >= 1
        
        args, kwargs = calls[-1]
        text_sent = args[0]
        assert "```\n转写文本\n```" in text_sent
        assert "🎙️ 由 Whisper 模型" in text_sent