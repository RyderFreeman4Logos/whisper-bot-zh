use std::sync::Arc;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::config::Settings;

#[derive(Clone)]
pub struct AsrService {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
    endpoint: String,
    api_key: String,
    model: String,
    language: String,
    prompt: String,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    text: String,
}

impl AsrService {
    pub fn new(settings: &Settings) -> Result<Self> {
        let api_key = settings
            .asr_effective_api_key()
            .map(ToOwned::to_owned)
            .context("ASR_API_KEY, GROQ_API, GROQ_API_KEY, or OPENAI_API_KEY must be configured")?;

        Ok(Self::from_parts(
            reqwest::Client::new(),
            &settings.asr_base_url,
            api_key,
            &settings.asr_model,
            &settings.asr_language,
            &settings.asr_prompt,
            settings.asr_temperature,
            settings.max_concurrent_tasks,
        ))
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        client: reqwest::Client,
        base_url: &str,
        api_key: String,
        model: &str,
        language: &str,
        prompt: &str,
        temperature: f32,
        max_concurrent: usize,
    ) -> Self {
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
            endpoint: format!("{}/audio/transcriptions", base_url.trim_end_matches('/')),
            api_key,
            model: model.to_owned(),
            language: language.to_owned(),
            prompt: prompt.to_owned(),
            temperature,
        }
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn transcribe(&self, audio: Bytes) -> Result<String> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .context("ASR semaphore closed unexpectedly")?;

        let form = Form::new()
            .part(
                "file",
                Part::bytes(audio.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .context("invalid ASR mime type")?,
            )
            .text("model", self.model.clone())
            .text("language", self.language.clone())
            .text("prompt", self.prompt.clone())
            .text("temperature", self.temperature.to_string())
            .text("response_format", "json");

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .context("failed to send ASR request")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read ASR response body")?;

        if !status.is_success() {
            bail!("ASR request failed with {status}: {body}");
        }

        let parsed: TranscriptResponse =
            serde_json::from_str(&body).context("failed to parse ASR json response")?;
        Ok(parsed.text.trim().to_owned())
    }
}
