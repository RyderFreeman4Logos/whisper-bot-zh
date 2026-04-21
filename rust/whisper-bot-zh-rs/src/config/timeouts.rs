use std::time::Duration;

use anyhow::{bail, Result};

use super::Settings;

pub(super) fn validate(settings: &Settings) -> Result<()> {
    validate_positive_f64("LLM_CLOUD_TIMEOUT_SEC", settings.llm_cloud_timeout_sec)?;
    validate_positive_f64("LLM_LOCAL_TIMEOUT_SEC", settings.llm_local_timeout_sec)?;
    validate_positive_f64(
        "LLM_HEARTBEAT_INTERVAL_SEC",
        settings.llm_heartbeat_interval_sec,
    )?;
    Ok(())
}

impl Settings {
    #[must_use]
    pub fn cloud_timeout(&self) -> Duration {
        Duration::from_secs_f64(self.llm_cloud_timeout_sec)
    }

    #[must_use]
    pub fn local_timeout(&self) -> Duration {
        Duration::from_secs_f64(self.llm_local_timeout_sec)
    }

    #[must_use]
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs_f64(self.llm_heartbeat_interval_sec)
    }
}

fn validate_positive_f64(key: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        bail!("{key} must be a finite positive number, got {value}");
    }
    Ok(())
}
