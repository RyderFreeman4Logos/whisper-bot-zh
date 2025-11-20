import asyncio
import functools
from pathlib import Path
from typing import Optional, List, Dict, Any

import torch
from funasr import AutoModel
import structlog

logger = structlog.get_logger(__name__)

class SenseVoiceEngine:
    def __init__(
        self, 
        model_path: Path, 
        max_concurrent: int = 1,
        device: Optional[str] = None
    ):
        self.model_path = model_path
        self.device = device or ("cuda" if torch.cuda.is_available() else "cpu")
        self.max_concurrent = max_concurrent
        self._semaphore = asyncio.Semaphore(max_concurrent)
        
        logger.info(f"Initializing SenseVoiceEngine on {self.device}...", model_path=str(model_path))
        
        # Suppress funasr noise if possible, or let it log
        try:
            self.model = AutoModel(
                model=str(self.model_path),
                device=self.device,
                disable_update=True,
                disable_pbar=True,
                # SenseVoice specific settings can be added here
                trust_remote_code=True, 
            )
            logger.info("SenseVoice model loaded successfully.")
        except Exception as e:
            logger.critical(f"Failed to load SenseVoice model: {e}")
            raise

    async def transcribe(self, file_path: Path) -> str:
        """
        Transcribe an audio file asynchronously.
        Processes are limited by a semaphore to prevent OOM.
        """
        if not file_path.exists():
            raise FileNotFoundError(f"Audio file not found: {file_path}")

        logger.debug(f"Waiting for slot to transcribe {file_path.name}...")
        async with self._semaphore:
            logger.info(f"Starting transcription for {file_path.name}")
            try:
                # Run the blocking inference in a separate thread
                text = await asyncio.to_thread(self._run_inference, str(file_path))
                logger.info(f"Transcription completed for {file_path.name}")
                return text
            except Exception as e:
                logger.error(f"Transcription failed for {file_path.name}: {e}")
                raise

    def _run_inference(self, file_path_str: str) -> str:
        """
        Blocking inference method.
        """
        # cache={} is often used in funasr examples to keep state, 
        # but for SenseVoice we usually just pass input.
        # language="zh" is default, but we explicitly set it if needed.
        # use_itn=False (Inverse Text Normalization) - keeping raw or normalized?
        # Usually we want text.
        
        res = self.model.generate(
            input=file_path_str,
            cache={},
            language="zh",  # Force Chinese for now as per req, or "auto"
            use_itn=True,
            batch_size_s=60,
            merge_vad=True,  
            merge_length_s=15,
        )
        
        # SenseVoice return format:
        # [{'key': '...', 'text': 'result text'}]
        if isinstance(res, list) and len(res) > 0:
            return res[0].get("text", "")
        return ""
