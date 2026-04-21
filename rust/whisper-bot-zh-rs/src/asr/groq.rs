use std::sync::Arc;

use crate::config::Settings;

/// Placeholder ASR service. Will wrap a `reqwest::Client` and expose a single
/// `transcribe` method that multipart-POSTs an in-memory audio buffer to
/// `{asr_base_url}/audio/transcriptions`.
#[derive(Clone)]
pub struct AsrService {
    #[allow(dead_code)]
    settings: Arc<Settings>,
}

impl AsrService {
    #[must_use]
    pub fn new(settings: Arc<Settings>) -> Self {
        Self { settings }
    }
}
