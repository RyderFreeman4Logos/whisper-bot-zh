//! LLM refinement service — cloud fallback chain + optional local endpoint.

pub mod cloud;
pub mod heartbeat;
pub mod local;
mod openai;
pub mod prompt;
mod provider;

use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::config::Settings;

pub use cloud::CloudRefiner;
pub use local::LocalRefiner;
pub use openai::ChatRefiner;

#[derive(Debug, Clone)]
pub struct RefinementResult {
    pub ok: bool,
    pub text: String,
    pub duration: Duration,
    pub model: String,
}

#[derive(Clone, Default)]
pub struct LlmService {
    cloud: Option<CloudRefiner>,
    local: Option<LocalRefiner>,
}

impl LlmService {
    /// Build the LLM service from application settings.
    ///
    /// # Errors
    /// Returns an error if the configured cloud or local refiners cannot be
    /// constructed.
    pub fn new(settings: &Settings) -> Result<Self> {
        Ok(Self::from_refiners(
            CloudRefiner::from_settings(settings)?,
            LocalRefiner::from_settings(settings)?,
        ))
    }

    #[must_use]
    pub fn from_refiners(cloud: Option<CloudRefiner>, local: Option<LocalRefiner>) -> Self {
        Self { cloud, local }
    }

    #[must_use]
    pub fn has_cloud(&self) -> bool {
        self.cloud.is_some()
    }

    #[must_use]
    pub fn has_local(&self) -> bool {
        self.local.is_some()
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.has_cloud() || self.has_local()
    }

    #[must_use]
    pub fn cloud_display_model(&self) -> Option<String> {
        self.cloud.as_ref().map(CloudRefiner::display_model)
    }

    #[must_use]
    pub fn local_display_model(&self) -> Option<String> {
        self.local.as_ref().map(|l| l.model_name().to_owned())
    }

    /// Refine text with the cloud model chain.
    ///
    /// # Errors
    /// Returns an error if no cloud refiner is configured or if cloud refinement
    /// fails.
    pub async fn refine_cloud(&self, text: &str) -> Result<RefinementResult> {
        let cloud = self
            .cloud
            .as_ref()
            .context("cloud refinement requested but no cloud models are configured")?;
        cloud.refine(text).await
    }

    /// Refine text with the local model endpoint.
    ///
    /// # Errors
    /// Returns an error if no local refiner is configured or if local refinement
    /// fails.
    pub async fn refine_local(&self, text: &str) -> Result<RefinementResult> {
        let local = self
            .local
            .as_ref()
            .context("local refinement requested but no local model is configured")?;
        local.refine(text).await
    }

    /// Refine text with whichever single refiner is currently available.
    ///
    /// # Errors
    /// Returns an error if no refiner is configured or if the selected refiner
    /// fails.
    pub async fn refine_single(&self, text: &str) -> Result<RefinementResult> {
        if self.has_cloud() {
            return self.refine_cloud(text).await;
        }
        if self.has_local() {
            return self.refine_local(text).await;
        }
        bail!("LLM refinement requested but service is disabled");
    }

    /// Refine text with both cloud and local refiners in parallel.
    ///
    /// # Errors
    /// Returns an error if either refiner is missing or if either parallel
    /// refinement request fails.
    pub async fn refine_dual(&self, text: &str) -> Result<Vec<RefinementResult>> {
        let cloud = self
            .cloud
            .as_ref()
            .context("dual refinement requires cloud configuration")?;
        let local = self
            .local
            .as_ref()
            .context("dual refinement requires local configuration")?;

        let cloud_future = cloud.refine(text);
        let local_future = local.refine(text);
        let (cloud_result, local_result): (Result<RefinementResult>, Result<RefinementResult>) =
            tokio::join!(cloud_future, local_future);
        Ok(vec![cloud_result?, local_result?])
    }
}
