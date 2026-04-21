use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use whisper_bot_zh::bot;
use whisper_bot_zh::config::Settings;

#[derive(Parser, Debug)]
#[command(
    name = "whisper-bot-zh",
    version,
    about = "Telegram bot: Groq ASR + cloud/local LLM refinement"
)]
struct Cli {
    /// Path to .env file. Overrides the XDG default `~/.config/whisper-bot-zh/.env`.
    #[arg(long, env = "WHISPER_BOT_ENV_FILE")]
    env_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let settings =
        Settings::load(cli.env_file.as_deref()).context("failed to load configuration")?;

    init_tracing(&settings.log_level);

    tracing::info!(
        bot_token_suffix = %settings.bot_token_suffix(),
        has_cloud = settings.has_cloud(),
        has_local = settings.has_local(),
        "starting whisper-bot-zh (rust)"
    );

    bot::run(settings).await
}

fn init_tracing(level: &str) {
    let filter = EnvFilter::try_new(level)
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("valid default env filter");

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_line_number(false))
        .init();
}
