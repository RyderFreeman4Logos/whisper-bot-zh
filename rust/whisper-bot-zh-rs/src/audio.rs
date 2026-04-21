//! `FFmpeg` subprocess wrapper — transcodes arbitrary Telegram audio buffers
//! (OGG/Opus, M4A, MP3, …) into a format Groq's Whisper endpoint accepts.
//!
//! TODO: implement. Mirror `src/whisper_bot/bot/handlers.py` ffmpeg pipe path.

use bytes::Bytes;

/// Placeholder transcode: returns input unchanged.
pub fn transcode_to_wav(input: Bytes) -> anyhow::Result<Bytes> {
    Ok(input)
}
