import pytest
import asyncio
import io
from unittest.mock import MagicMock, patch
from pathlib import Path

from whisper_bot.services.asr import WhisperEngine

@pytest.fixture
def mock_model():
    with patch("whisper_bot.services.asr.WhisperModel") as MockClass:
        mock_instance = MockClass.return_value
        Segment = MagicMock()
        Segment.text = "测试结果"
        mock_instance.transcribe.return_value = ([Segment], None)
        yield MockClass

@pytest.mark.asyncio
async def test_asr_initialization(mock_model):
    # Default compute_type is int8
    engine = WhisperEngine(model_size="tiny", device="cpu", max_concurrent=1)
    mock_model.assert_called_with(model_size_or_path="tiny", device="cpu", compute_type="int8")

@pytest.mark.asyncio
async def test_transcribe_success(mock_model, tmp_path):
    engine = WhisperEngine(
        model_size="tiny", 
        device="cpu",
        initial_prompt="Prompt",
        vad_filter=True
    )
    dummy_file = tmp_path / "test.wav"
    # dummy_file doesn't need to exist on disk now
    
    result = await engine.transcribe(dummy_file)
    assert result == "测试结果"
    
    # Verify call args with Path
    args, kwargs = mock_model.return_value.transcribe.call_args
    assert args[0] == str(dummy_file)
    assert kwargs['language'] == "zh"
    assert kwargs['initial_prompt'] == "Prompt"
    assert kwargs['vad_filter'] is True

@pytest.mark.asyncio
async def test_transcribe_bytes_success(mock_model):
    engine = WhisperEngine(model_size="tiny")
    dummy_bytes = io.BytesIO(b"wav data")
    
    result = await engine.transcribe(dummy_bytes)
    assert result == "测试结果"
    
    # Verify call args with BytesIO
    args, kwargs = mock_model.return_value.transcribe.call_args
    assert args[0] == dummy_bytes

@pytest.mark.asyncio
async def test_concurrency_limit(mock_model):
    engine = WhisperEngine(model_size="tiny", max_concurrent=1)
    assert engine._semaphore._value == 1