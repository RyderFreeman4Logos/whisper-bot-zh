import io
from pathlib import Path

import ffmpeg
import structlog

logger = structlog.get_logger(__name__)


def convert_to_wav(input_path: Path, output_path: Path = None) -> Path:
    """
    Convert audio file to 16k mono wav.
    If output_path is not provided, appends .wav to input_path.
    """
    if output_path is None:
        output_path = input_path.with_suffix(".wav")

    logger.debug(f"Converting {input_path} to {output_path}...")

    try:
        (ffmpeg.input(str(input_path)).output(str(output_path), ar=16000, ac=1).run(quiet=True, overwrite_output=True))
        return output_path
    except ffmpeg.Error as e:
        logger.error(f"FFmpeg conversion failed: {e}")
        # If stderr is available, log it
        if hasattr(e, "stderr") and e.stderr:
            logger.error(f"FFmpeg stderr: {e.stderr.decode('utf8')}")
        raise


def convert_audio_memory(input_data: bytes) -> io.BytesIO:
    """
    Convert audio bytes to WAV (16kHz, mono, s16le) in memory using FFmpeg.
    Returns a BytesIO object containing the WAV data.
    """
    logger.debug("Converting audio in memory...")
    try:
        # pipe:0 is stdin, pipe:1 is stdout
        process = (
            ffmpeg.input("pipe:0")
            .output("pipe:1", format="wav", ac=1, ar=16000)
            .run_async(pipe_stdin=True, pipe_stdout=True, pipe_stderr=True)
        )
        # Write input to stdin and read output from stdout
        out, err = process.communicate(input=input_data)

        if process.returncode != 0:
            error_msg = err.decode("utf-8") if err else "Unknown error"
            logger.error(f"FFmpeg memory conversion failed: {error_msg}")
            raise RuntimeError(f"FFmpeg failed: {error_msg}")

        return io.BytesIO(out)

    except Exception as e:
        logger.error(f"Audio conversion failed: {e}")
        raise
