import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from telegram import Update, User, Message, Chat
from telegram.ext import ContextTypes

from src.bot.handlers import auth_command, voice_message_handler
from src.services.auth import AuthService
from src.services.asr import SenseVoiceEngine

@pytest.fixture
def mock_update():
    update = MagicMock(spec=Update)
    update.effective_user = MagicMock(spec=User)
    update.effective_user.id = 12345
    update.effective_chat = MagicMock(spec=Chat)
    update.effective_chat.id = 67890
    update.message = AsyncMock(spec=Message)
    update.message.reply_text = AsyncMock()
    update.message.reply_to_message = None
    return update

@pytest.fixture
def mock_context():
    context = MagicMock(spec=ContextTypes.DEFAULT_TYPE)
    # Inject dependencies into bot_data
    context.bot_data = {
        "auth_service": MagicMock(spec=AuthService),
        "asr_engine": MagicMock(spec=SenseVoiceEngine)
    }
    return context

@pytest.mark.asyncio
async def test_auth_command_success(mock_update, mock_context):
    mock_context.args = ["correct_password"]
    mock_context.bot_data["auth_service"].authenticate_user.return_value = True
    
    await auth_command(mock_update, mock_context)
    
    mock_context.bot_data["auth_service"].authenticate_user.assert_called_with(12345, "correct_password")
    mock_update.message.reply_text.assert_called_with("✅ 认证成功！您现在可以使用语音转文字服务了。", parse_mode="Markdown")

@pytest.mark.asyncio
async def test_auth_command_fail(mock_update, mock_context):
    mock_context.args = ["wrong_password"]
    mock_context.bot_data["auth_service"].authenticate_user.return_value = False
    
    await auth_command(mock_update, mock_context)
    
    mock_update.message.reply_text.assert_called_with("❌ 认证失败，密码错误。", parse_mode="Markdown")

@pytest.mark.asyncio
async def test_voice_handler_unauthorized(mock_update, mock_context):
    mock_context.bot_data["auth_service"].is_user_allowed.return_value = False
    
    await voice_message_handler(mock_update, mock_context)
    
    # Should reply asking for auth
    args, _ = mock_update.message.reply_text.call_args
    assert "未授权" in args[0]
    # Should NOT call transcribe
    mock_context.bot_data["asr_engine"].transcribe.assert_not_called()

@pytest.mark.asyncio
async def test_voice_handler_success(mock_update, mock_context, tmp_path):
    mock_context.bot_data["auth_service"].is_user_allowed.return_value = True
    mock_context.bot_data["asr_engine"].transcribe.return_value = "转写文本"
    
    # Mock file download
    mock_new_file = AsyncMock()
    mock_new_file.download_to_drive = AsyncMock(return_value=tmp_path / "voice.ogg")
    mock_update.message.voice = MagicMock()
    mock_update.message.voice.get_file = AsyncMock(return_value=mock_new_file)
    
    # Mock processing message
    processing_msg = AsyncMock()
    mock_update.message.reply_text.return_value = processing_msg
    
    # Mock config TEMP_DIR
    with patch("src.bot.handlers.convert_to_wav", return_value=tmp_path / "voice.wav") as mock_convert:
        await voice_message_handler(mock_update, mock_context)
        
        # Verify flow
        mock_update.message.reply_text.assert_called() # "Processing..."
        mock_new_file.download_to_drive.assert_called()
        mock_convert.assert_called()
        mock_context.bot_data["asr_engine"].transcribe.assert_called()
        processing_msg.edit_text.assert_called_with("转写文本")
