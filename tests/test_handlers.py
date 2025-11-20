import pytest
from unittest.mock import AsyncMock, MagicMock, patch, ANY
from aiogram.types import Message, User, Chat
from aiogram.filters.command import CommandObject

from src.bot.handlers import auth_command, voice_message_handler
from src.services.auth import AuthService
from src.services.asr import WhisperEngine

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
    mock_message.answer.assert_called_with("✅ 认证成功！您现在可以使用语音转文字服务了。")

@pytest.mark.asyncio
async def test_auth_command_fail(mock_message, mock_auth_service):
    command = CommandObject(prefix="/", command="auth", args="wrong_password")
    mock_auth_service.authenticate_user.return_value = False
    
    await auth_command(mock_message, command, mock_auth_service)
    
    mock_message.answer.assert_called_with("❌ 认证失败，密码错误。")

@pytest.mark.asyncio
async def test_voice_handler_success(mock_message, mock_asr_engine, tmp_path):
    # Mock dependencies
    mock_bot = AsyncMock()
    mock_file_info = MagicMock()
    mock_file_info.file_path = "voice.ogg"
    mock_bot.get_file.return_value = mock_file_info
    mock_bot.download_file = AsyncMock()
    
    mock_asr_engine.transcribe = AsyncMock(return_value="转写文本")
    mock_llm_service = MagicMock()
    mock_llm_service.is_enabled = False # Disable LLM for basic test
    
    # Setup message
    mock_message.voice = MagicMock()
    mock_message.voice.file_id = "file_123"
    mock_message.audio = None
    
    # Setup processing message
    processing_msg = AsyncMock()
    mock_message.reply.return_value = processing_msg
    
    # Run handler
    with patch("src.bot.handlers.convert_to_wav", return_value=tmp_path / "voice.wav") as mock_convert:
        # Patch time to avoid issues? Not strictly needed if we just check inclusion string
        await voice_message_handler(mock_message, mock_bot, mock_asr_engine, mock_llm_service)
        
        # Verify
        mock_message.reply.assert_called()
        mock_bot.get_file.assert_called_with("file_123")
        mock_bot.download_file.assert_called()
        mock_asr_engine.transcribe.assert_called()
        
        # Verify edit_text calls
        # We look for the call containing the result
        calls = processing_msg.edit_text.call_args_list
        assert len(calls) >= 1
        
        # The last call should contain the result + duration
        args, kwargs = calls[-1]
        text_sent = args[0]
        assert "```\n转写文本\n```" in text_sent
        assert "⏱️ 耗时:" in text_sent
        assert kwargs['parse_mode'] == "Markdown"
