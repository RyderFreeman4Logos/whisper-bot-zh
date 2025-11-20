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
