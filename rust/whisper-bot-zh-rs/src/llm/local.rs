use anyhow::{Context, Result};

use crate::config::Settings;

use super::{ChatRefiner, RefinementResult};

#[derive(Clone)]
pub struct LocalRefiner {
    inner: ChatRefiner,
}

impl LocalRefiner {
    #[must_use]
    pub fn new(inner: ChatRefiner) -> Self {
        Self { inner }
    }

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

        Ok(Some(Self::new(ChatRefiner::new(
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
        ))))
    }

    #[must_use]
    pub fn model_name(&self) -> &str {
        self.inner.model()
    }

    pub async fn refine(&self, transcript: &str) -> Result<RefinementResult> {
        self.inner.refine(transcript).await
    }
}
