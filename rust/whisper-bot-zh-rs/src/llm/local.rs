use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Semaphore;

use crate::config::Settings;

use super::{ChatRefiner, RefinementResult};

#[derive(Clone)]
pub struct LocalRefiner {
    inner: ChatRefiner,
    limiter: Arc<Semaphore>,
}

impl LocalRefiner {
    #[must_use]
    pub fn new(inner: ChatRefiner, limiter: Arc<Semaphore>) -> Self {
        Self { inner, limiter }
    }

    /// Build the local refiner from the configured local endpoint settings.
    ///
    /// # Errors
    /// Returns an error if the local configuration is incomplete or the outbound
    /// HTTP client cannot be constructed.
    pub fn from_settings(settings: &Settings) -> Result<Option<Self>> {
        let Some(base_url) = settings.llm_local_base_url.as_deref() else {
            return Ok(None);
        };
        let api_key = settings
            .llm_local_api_key
            .clone()
            .context("LLM_LOCAL_API_KEY must be set for local refinement")?;
        let model = settings
            .llm_local_model
            .as_deref()
            .context("LLM_LOCAL_MODEL must be set for local refinement")?;
        let limiter = Arc::new(Semaphore::new(settings.llm_local_max_concurrent));

        Ok(Some(Self::new(
            ChatRefiner::new(
                settings.outbound_http_client()?,
                base_url,
                api_key,
                model,
                settings
                    .llm_system_prompt
                    .clone()
                    .unwrap_or_else(|| super::prompt::SYSTEM_PROMPT.to_owned()),
                settings.local_timeout(),
                settings.heartbeat_interval(),
                settings.llm_temperature,
                settings.llm_top_p,
                settings.llm_max_tokens,
                settings.llm_local_reasoning_effort.clone(),
            ),
            limiter,
        )))
    }

    #[must_use]
    pub fn model_name(&self) -> &str {
        self.inner.model()
    }

    /// Refine a transcript through the configured local model endpoint.
    ///
    /// # Errors
    /// Returns an error if the concurrency limiter is closed or the underlying
    /// chat refiner request fails.
    pub async fn refine(&self, transcript: &str) -> Result<RefinementResult> {
        let _permit = self
            .limiter
            .clone()
            .acquire_owned()
            .await
            .context("local LLM semaphore closed unexpectedly")?;
        self.inner.refine(transcript).await
    }
}
