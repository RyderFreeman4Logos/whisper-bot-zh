use anyhow::{anyhow, Result};
use std::path::PathBuf;

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
