use anyhow::{anyhow, Result};
use std::path::PathBuf;

use super::Settings;

pub fn default_data_dir() -> Result<PathBuf> {
    Ok(required_home_dir(
        "default data dir; pass --env-file and --data-dir explicitly if running without HOME",
    )?
    .join(".config")
    .join("whisper-bot-zh"))
}

pub fn default_cache_dir() -> Result<PathBuf> {
    Ok(
        required_home_dir("default cache dir; pass CACHE_DIR explicitly if running without HOME")?
            .join(".cache")
            .join("whisper-bot-zh"),
    )
}

fn required_home_dir(help: &str) -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("HOME is required to resolve {help}"))
}

impl Settings {
    #[must_use]
    pub fn allowed_users_file(&self) -> PathBuf {
        self.data_dir.join("allowed_users.json")
    }

    #[must_use]
    pub fn bot_token_suffix(&self) -> String {
        let token = &self.bot_token;
        if token.len() > 6 {
            format!("...{}", &token[token.len() - 6..])
        } else {
            "******".into()
        }
    }
}
