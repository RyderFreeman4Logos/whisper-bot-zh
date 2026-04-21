//! ASR service — Groq-compatible `OpenAI` audio transcription endpoint.
//!
//! TODO: implement. See `src/whisper_bot/services/asr.py` in the Python tree
//! for the reference behaviour.

pub mod groq;

pub use groq::AsrService;
