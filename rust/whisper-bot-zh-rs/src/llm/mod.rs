//! LLM refinement service — dual-path: cloud fallback chain + local `OpenAI`
//! compatible endpoint, run in parallel when both are configured.
//!
//! TODO: implement. Mirror `src/whisper_bot/services/llm.py`.

pub mod cloud;
pub mod heartbeat;
pub mod local;
pub mod prompt;

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RefinementResult {
    pub ok: bool,
    pub text: String,
    pub duration: Duration,
    pub model: String,
}
