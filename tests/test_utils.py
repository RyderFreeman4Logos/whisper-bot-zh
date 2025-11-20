from pathlib import Path
from unittest.mock import MagicMock, patch
import pytest
import io
from whisper_bot.utils import convert_to_wav, convert_audio_memory

@patch("ffmpeg.input")
def test_convert_to_wav(mock_input):
    """Test file-based conversion logic."""
    mock_output = MagicMock()
    mock_input.return_value.output.return_value = mock_output
    
    input_path = Path("test.ogg")
    output_path = convert_to_wav(input_path)
    
    assert output_path == Path("test.wav")
    mock_input.assert_called_with("test.ogg")
    mock_output.run.assert_called_once()

@patch("ffmpeg.input")
def test_convert_audio_memory(mock_input):
    """Test memory-based conversion logic."""
    mock_run = MagicMock()
    # Mock process behavior
    mock_process = MagicMock()
    mock_process.communicate.return_value = (b"wav_data", b"")
    mock_process.returncode = 0
    
    mock_run.run_async.return_value = mock_process
    mock_input.return_value.output.return_value = mock_run
    
    input_bytes = b"ogg_data"
    result = convert_audio_memory(input_bytes)
    
    assert isinstance(result, io.BytesIO)
    assert result.getvalue() == b"wav_data"
    
    mock_input.assert_called_with('pipe:0')
    # Verify args for pipe output
    args, kwargs = mock_input.return_value.output.call_args
    assert args[0] == 'pipe:1'
    assert kwargs['format'] == 'wav'