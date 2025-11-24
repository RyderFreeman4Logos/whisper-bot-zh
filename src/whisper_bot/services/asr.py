import asyncio
from pathlib import Path
from typing import BinaryIO

import numpy as np
import structlog
from faster_whisper import WhisperModel

logger = structlog.get_logger(__name__)


class WhisperEngine:
    def __init__(
        self,
        model_size: str = "large-v2",
        compute_type: str = "int8",
        max_concurrent: int = 1,
        device: str | None = None,
        initial_prompt: str | None = None,
        vad_filter: bool = False,
    ):
        self.device = device or "cuda"
        self.model_size = model_size
        self.compute_type = compute_type
        self.max_concurrent = max_concurrent
        self.initial_prompt = initial_prompt
        self.vad_filter = vad_filter

        self._semaphore = asyncio.Semaphore(max_concurrent)

        logger.info("Initializing WhisperEngine (faster-whisper)...")
        logger.info(f"Model: {model_size}, Device: {self.device}, Compute: {compute_type}")
        logger.info(f"VAD: {vad_filter}, Prompt: {initial_prompt}")

        try:
            # Initializing the model downloads it automatically if not present
            self.model = WhisperModel(
                model_size_or_path=self.model_size, device=self.device, compute_type=self.compute_type
            )
            logger.info("Whisper model loaded successfully.")
        except Exception as e:
            logger.critical(f"Failed to load Whisper model: {e}")
            raise

    async def transcribe(self, audio_input: str | Path | BinaryIO | np.ndarray) -> str:
        """
        Transcribe audio input asynchronously.
        audio_input: Path to file, file-like object, or numpy array.
        """
        logger.debug("Waiting for slot to transcribe...")
        async with self._semaphore:
            logger.info("Starting transcription")
            try:
                # Run blocking inference in thread
                text = await asyncio.to_thread(self._run_inference, audio_input)
                logger.info("Transcription completed")
                return text
            except Exception as e:
                logger.error(f"Transcription failed: {e}")
                raise

    def _run_inference(self, audio_input: str | Path | BinaryIO | np.ndarray) -> str:
        """
        Blocking inference method using faster-whisper.
        """
        # faster-whisper accepts str (path), binaryIO, or np.ndarray
        if isinstance(audio_input, Path):
            audio_input = str(audio_input)

        segments, info = self.model.transcribe(
            audio_input, beam_size=5, language="zh", initial_prompt=self.initial_prompt, vad_filter=self.vad_filter
        )

        # Gather all segments
        result_text = "".join([segment.text for segment in segments])

        # Regular cleanup
        if isinstance(audio_input, str):
            # remove special tokens if any? faster-whisper output is usually clean text but might have spaces
            pass

        return result_text.strip()
