import asyncio
import os
from typing import BinaryIO

import structlog
from openai import AsyncOpenAI

logger = structlog.get_logger(__name__)


class AsrConfigError(RuntimeError):
    """Raised when ASR client cannot be constructed from config."""


class AsrClient:
    """OpenAI-compatible audio transcription client.

    Defaults target Groq (`https://api.groq.com/openai/v1`, `whisper-large-v3`).
    Point at any OpenAI-compat server (self-hosted whisper.cpp, vLLM-whisper,
    etc.) by changing ASR_BASE_URL / ASR_API_KEY / ASR_MODEL in the env.
    """

    def __init__(
        self,
        base_url: str,
        api_key: str | None,
        model: str,
        language: str = "zh",
        prompt: str | None = None,
        temperature: float = 0.0,
        max_concurrent: int = 1,
    ):
        resolved_key = api_key or os.environ.get("GROQ_API_KEY") or os.environ.get("OPENAI_API_KEY")
        if not resolved_key:
            raise AsrConfigError("No ASR API key configured. Set ASR_API_KEY, GROQ_API_KEY, or OPENAI_API_KEY.")

        self.model = model
        self.language = language
        self.prompt = prompt
        self.temperature = temperature
        self.max_concurrent = max_concurrent
        self.base_url = base_url

        self._semaphore = asyncio.Semaphore(max_concurrent)
        self._client = AsyncOpenAI(base_url=base_url, api_key=resolved_key)

        logger.info(
            "AsrClient initialized",
            base_url=base_url,
            model=model,
            language=language,
            temperature=temperature,
        )

    async def transcribe(self, audio: BinaryIO | bytes) -> str:
        """Transcribe raw audio bytes (or a seekable file-like) via the remote API."""
        payload = audio.read() if hasattr(audio, "read") else bytes(audio)

        async with self._semaphore:
            logger.info("Starting transcription", bytes=len(payload))
            try:
                response = await self._client.audio.transcriptions.create(
                    model=self.model,
                    file=("audio.wav", payload, "audio/wav"),
                    language=self.language,
                    prompt=self.prompt or "",
                    temperature=self.temperature,
                    response_format="text",
                )
            except Exception as e:
                logger.error("Transcription failed", error=str(e))
                raise

        text = response if isinstance(response, str) else getattr(response, "text", str(response))
        return text.strip()
