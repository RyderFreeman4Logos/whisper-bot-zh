import asyncio
from pathlib import Path
from typing import Optional

from faster_whisper import WhisperModel
import structlog

logger = structlog.get_logger(__name__)

class WhisperEngine:
    def __init__(
        self, 
        model_size: str = "large-v2",
        compute_type: str = "float16",
        max_concurrent: int = 1,
        device: Optional[str] = None
    ):
        # faster-whisper handles device automatically usually, but good to be explicit
        # If device is not provided, let faster-whisper decide (usually cuda if avail)
        # faster-whisper's device arg: "cuda" or "cpu" or "auto"
        self.device = device or "cuda" # We force cuda as per requirement, fallback handled by user or lib if cuda missing? 
        # actually faster-whisper raises error if cuda requested but not found.
        # User said "optimize for 7G VRAM", implying GPU availability.
        
        self.model_size = model_size
        self.compute_type = compute_type
        self.max_concurrent = max_concurrent
        self._semaphore = asyncio.Semaphore(max_concurrent)
        
        logger.info(f"Initializing WhisperEngine (faster-whisper)...")
        logger.info(f"Model: {model_size}, Device: {self.device}, Compute: {compute_type}")
        
        try:
            # Initializing the model downloads it automatically if not present
            self.model = WhisperModel(
                model_size_or_path=self.model_size,
                device=self.device,
                compute_type=self.compute_type
            )
            logger.info("Whisper model loaded successfully.")
        except Exception as e:
            logger.critical(f"Failed to load Whisper model: {e}")
            raise

    async def transcribe(self, file_path: Path) -> str:
        """
        Transcribe an audio file asynchronously.
        """
        if not file_path.exists():
            raise FileNotFoundError(f"Audio file not found: {file_path}")

        logger.debug(f"Waiting for slot to transcribe {file_path.name}...")
        async with self._semaphore:
            logger.info(f"Starting transcription for {file_path.name}")
            try:
                # Run blocking inference in thread
                text = await asyncio.to_thread(self._run_inference, str(file_path))
                logger.info(f"Transcription completed for {file_path.name}")
                return text
            except Exception as e:
                logger.error(f"Transcription failed for {file_path.name}: {e}")
                raise

    def _run_inference(self, file_path_str: str) -> str:
        """
        Blocking inference method using faster-whisper.
        """
        # beam_size=5 is default.
        # language="zh" forces Chinese.
        segments, info = self.model.transcribe(
            file_path_str, 
            beam_size=5,
            language="zh"
        )
        
        # Gather all segments
        # segments is a generator, so list() triggers inference
        result_text = "".join([segment.text for segment in segments])
        return result_text.strip()
