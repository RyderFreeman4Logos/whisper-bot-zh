//! teloxide entrypoint + Dispatcher wiring.
//!
//! TODO: implement. Mirror `src/whisper_bot/main.py` + `src/whisper_bot/bot/`.

use crate::config::Settings;

pub mod commands;
pub mod flow;
pub mod render;
pub mod telegram_limit;
pub mod voice;

pub async fn run(_settings: Settings) -> anyhow::Result<()> {
    tracing::warn!("bot::run is a placeholder — Rust port not yet implemented");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
