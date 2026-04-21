use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use super::paths::default_data_dir;

pub fn resolve_env_file_path(
    cli_env_file: Option<&Path>,
    cli_data_dir: Option<&Path>,
) -> Result<Option<PathBuf>> {
    if let Some(path) = cli_env_file {
        return Ok(Some(path.to_path_buf()));
    }

    if let Some(data_dir) = cli_data_dir {
        return Ok(env_file_in(data_dir));
    }

    if let Some(data_dir) = optional("DATA_DIR") {
        return Ok(env_file_in(Path::new(&data_dir)));
    }

    let default_dir = default_data_dir()?;
    Ok(env_file_in(&default_dir))
}

pub fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set in environment"))
}

pub fn optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

pub fn optional_parsed<T>(key: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match optional(key) {
        Some(value) => value
            .parse::<T>()
            .map(Some)
            .map_err(|error| anyhow!("{key}={value:?} parse error: {error}")),
        None => Ok(None),
    }
}

fn env_file_in(dir: &Path) -> Option<PathBuf> {
    let candidate = dir.join(".env");
    candidate.exists().then_some(candidate)
}
