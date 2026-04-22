use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use tokio::sync::Semaphore;

use crate::config::Settings;

use super::provider::resolve_model;
use super::{ChatRefiner, RefinementResult};

#[derive(Clone)]
pub struct CloudRefiner {
    limiter: Arc<Semaphore>,
    models: Vec<ChatRefiner>,
}

impl CloudRefiner {
    #[must_use]
    pub fn new(models: Vec<ChatRefiner>, limiter: Arc<Semaphore>) -> Self {
        Self { limiter, models }
    }

    /// Build the cloud refiner chain from the configured cloud model list.
    ///
    /// # Errors
    /// Returns an error if the outbound HTTP client cannot be built or any
    /// configured cloud model cannot be resolved into a chat client.
    pub fn from_settings(settings: &Settings) -> Result<Option<Self>> {
        let client = settings.outbound_http_client()?;
        let limiter = Arc::new(Semaphore::new(settings.llm_cloud_max_concurrent));
        let models = settings
            .cloud_models()
            .into_iter()
            .map(|configured_model| build_model_client(settings, client.clone(), &configured_model))
            .collect::<Result<Vec<_>>>()?;

        if models.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self::new(models, limiter)))
    }

    #[must_use]
    pub fn display_model(&self) -> String {
        self.models
            .first()
            .map(|m| m.model().to_owned())
            .unwrap_or_default()
    }

    /// Refine a transcript using the configured cloud fallback chain.
    ///
    /// # Errors
    /// Returns an error if the concurrency limiter is closed or every configured
    /// cloud model fails to produce a refinement.
    pub async fn refine(&self, transcript: &str) -> Result<RefinementResult> {
        let _permit = self
            .limiter
            .clone()
            .acquire_owned()
            .await
            .context("cloud LLM semaphore closed unexpectedly")?;
        let mut last_error = None;

        for refiner in &self.models {
            match refiner.refine(transcript).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    tracing::warn!(model = refiner.model(), %error, "cloud refinement attempt failed");
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("no cloud models are configured")))
    }
}

fn build_model_client(
    settings: &Settings,
    client: reqwest::Client,
    configured_model: &str,
) -> Result<ChatRefiner> {
    let resolved = resolve_model(settings, configured_model)
        .with_context(|| format!("failed to configure cloud model `{configured_model}`"))?;

    Ok(ChatRefiner::new(
        client,
        &resolved.base_url,
        resolved.api_key,
        &resolved.model_name,
        settings
            .llm_system_prompt
            .clone()
            .unwrap_or_else(|| super::prompt::SYSTEM_PROMPT.to_owned()),
        settings.cloud_timeout(),
        settings.heartbeat_interval(),
        settings.llm_temperature,
        settings.llm_top_p,
        settings.llm_max_tokens,
        None,
    ))
}
