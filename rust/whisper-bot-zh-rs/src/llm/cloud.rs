use anyhow::{anyhow, Context, Result};

use crate::config::Settings;

use super::provider::resolve_model;
use super::{ChatRefiner, RefinementResult};

#[derive(Clone)]
pub struct CloudRefiner {
    models: Vec<ChatRefiner>,
}

impl CloudRefiner {
    #[must_use]
    pub fn new(models: Vec<ChatRefiner>) -> Self {
        Self { models }
    }

    pub fn from_settings(settings: &Settings) -> Result<Option<Self>> {
        let models = settings
            .cloud_models()
            .into_iter()
            .map(|configured_model| build_model_client(settings, &configured_model))
            .collect::<Result<Vec<_>>>()?;

        if models.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self::new(models)))
    }

    pub async fn refine(&self, transcript: &str) -> Result<RefinementResult> {
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

fn build_model_client(settings: &Settings, configured_model: &str) -> Result<ChatRefiner> {
    let resolved = resolve_model(settings, configured_model)
        .with_context(|| format!("failed to configure cloud model `{configured_model}`"))?;

    Ok(ChatRefiner::new(
        reqwest::Client::new(),
        &resolved.base_url,
        resolved.api_key,
        &resolved.model_name,
        settings.cloud_timeout(),
        settings.heartbeat_interval(),
        settings.llm_temperature,
        settings.llm_top_p,
        settings.llm_max_tokens,
    ))
}
