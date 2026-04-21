import io
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from whisper_bot.services.asr import AsrClient, AsrConfigError


@pytest.fixture
def mock_openai():
    with patch("whisper_bot.services.asr.AsyncOpenAI") as MockClient:
        instance = MockClient.return_value
        instance.audio = MagicMock()
        instance.audio.transcriptions = MagicMock()
        instance.audio.transcriptions.create = AsyncMock(return_value="测试结果")
        yield MockClient


@pytest.mark.asyncio
async def test_asr_initialization_uses_configured_base_url(mock_openai):
    AsrClient(
        base_url="https://api.groq.com/openai/v1",
        api_key="gsk_fake",
        model="whisper-large-v3",
    )
    mock_openai.assert_called_once_with(
        base_url="https://api.groq.com/openai/v1",
        api_key="gsk_fake",
    )


@pytest.mark.asyncio
async def test_asr_initialization_falls_back_to_groq_env(monkeypatch, mock_openai):
    monkeypatch.setenv("GROQ_API_KEY", "env_key")
    AsrClient(base_url="https://api.groq.com/openai/v1", api_key=None, model="whisper-large-v3")
    _, kwargs = mock_openai.call_args
    assert kwargs["api_key"] == "env_key"


@pytest.mark.asyncio
async def test_asr_initialization_raises_without_key(monkeypatch, mock_openai):
    monkeypatch.delenv("GROQ_API_KEY", raising=False)
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    with pytest.raises(AsrConfigError):
        AsrClient(base_url="https://api.groq.com/openai/v1", api_key=None, model="whisper-large-v3")


@pytest.mark.asyncio
async def test_transcribe_passes_config_to_api(mock_openai):
    client = AsrClient(
        base_url="https://api.groq.com/openai/v1",
        api_key="gsk_fake",
        model="whisper-large-v3",
        language="zh",
        prompt="Prompt",
        temperature=0.0,
    )
    audio = io.BytesIO(b"fake_wav_bytes")

    result = await client.transcribe(audio)

    assert result == "测试结果"
    mock_openai.return_value.audio.transcriptions.create.assert_awaited_once()
    _, kwargs = mock_openai.return_value.audio.transcriptions.create.call_args
    assert kwargs["model"] == "whisper-large-v3"
    assert kwargs["language"] == "zh"
    assert kwargs["prompt"] == "Prompt"
    assert kwargs["temperature"] == 0.0
    assert kwargs["response_format"] == "text"
    # file=(filename, payload, content_type)
    filename, payload, content_type = kwargs["file"]
    assert filename == "audio.wav"
    assert payload == b"fake_wav_bytes"
    assert content_type == "audio/wav"


@pytest.mark.asyncio
async def test_transcribe_accepts_raw_bytes(mock_openai):
    client = AsrClient(base_url="https://api.groq.com/openai/v1", api_key="gsk_fake", model="whisper-large-v3")
    result = await client.transcribe(b"raw_bytes")
    assert result == "测试结果"


@pytest.mark.asyncio
async def test_concurrency_limit(mock_openai):
    client = AsrClient(
        base_url="https://api.groq.com/openai/v1",
        api_key="gsk_fake",
        model="whisper-large-v3",
        max_concurrent=2,
    )
    assert client._semaphore._value == 2
