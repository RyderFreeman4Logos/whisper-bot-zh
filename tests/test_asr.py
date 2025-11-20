import pytest
import asyncio
from unittest.mock import MagicMock, patch
from pathlib import Path

from src.services.asr import WhisperEngine

@pytest.fixture
def mock_model():
    with patch("src.services.asr.WhisperModel") as MockClass:
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
    dummy_file.touch()
    
    result = await engine.transcribe(dummy_file)
    
    assert result == "测试结果"
    
    # Verify call args
    args, kwargs = mock_model.return_value.transcribe.call_args
    assert str(dummy_file) == args[0]
    assert kwargs['language'] == "zh"
    assert kwargs['initial_prompt'] == "Prompt"
    assert kwargs['vad_filter'] is True

@pytest.mark.asyncio
async def test_concurrency_limit(mock_model, tmp_path):
    engine = WhisperEngine(model_size="tiny", max_concurrent=1)
    assert engine._semaphore._value == 1
