//! teloxide entrypoint + dispatcher wiring.

use std::sync::Arc;

use anyhow::{Context, Result};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use tokio::sync::Semaphore;

use crate::asr::AsrService;
use crate::auth::AuthService;
use crate::config::Settings;
use crate::llm::LlmService;

pub mod commands;
pub mod flow;
pub mod render;
pub mod telegram_limit;
pub mod voice;

type HandlerResult = ResponseResult<()>;

#[derive(Clone)]
struct DispatchDeps {
    asr: Arc<AsrService>,
    auth: Arc<AuthService>,
    governor: Arc<render::TelegramGovernor>,
    llm: Arc<LlmService>,
    voice_limiter: Arc<Semaphore>,
}

pub async fn run(settings: Settings) -> Result<()> {
    let settings = Arc::new(settings);
    let bot = Bot::with_client(settings.bot_token.clone(), settings.telegram_client()?);
    let bot_username = Arc::new(
        bot.get_me()
            .await
            .context("failed to fetch Telegram bot profile")?
            .user
            .username
            .context("Telegram bot username is required")?,
    );
    let asr = Arc::new(AsrService::new(settings.as_ref())?);
    let auth = Arc::new(AuthService::new(settings.as_ref()).await?);
    let llm = Arc::new(LlmService::new(settings.as_ref())?);
    let governor = Arc::new(render::TelegramGovernor::new(
        bot.clone(),
        settings.telegram_edit_min_interval(),
    ));
    let voice_limiter = Arc::new(Semaphore::new(settings.max_concurrent_tasks));
    let deps = Arc::new(DispatchDeps {
        asr,
        auth,
        governor,
        llm,
        voice_limiter,
    });

    let handler = Update::filter_message().endpoint(dispatch_message);

    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![deps, bot_username])
        .enable_ctrlc_handler()
        .build();

    Box::pin(dispatcher.dispatch()).await;

    Ok(())
}

async fn dispatch_message(
    bot: Bot,
    message: Message,
    deps: Arc<DispatchDeps>,
    bot_username: Arc<String>,
) -> HandlerResult {
    if let Some(text) = message.text() {
        if commands::is_supported_command(text, bot_username.as_str()) {
            return commands::handle_command(
                deps.governor.as_ref(),
                &message,
                text,
                bot_username.as_str(),
                deps.auth.as_ref(),
            )
            .await;
        }
    }

    if message.voice().is_some() || message.audio().is_some() {
        // Acquire before spawn so the limiter bounds queued voice work too,
        // not just concurrently running handlers.
        let voice_permit = match deps.voice_limiter.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(tokio::sync::TryAcquireError::NoPermits) => {
                tracing::info!(chat_id = %message.chat.id, "voice handler limiter saturated");
                if let Err(error) = deps
                    .governor
                    .send_message(message.chat.id, "⏳ 当前任务较多，请稍后再试")
                    .await
                {
                    tracing::warn!(%error, "failed to send voice busy reply");
                }
                return Ok(());
            }
            Err(tokio::sync::TryAcquireError::Closed) => {
                tracing::warn!("voice handler semaphore closed");
                return Ok(());
            }
        };

        // Detach the voice handler so the dispatcher can keep receiving
        // subsequent messages while a slow local LLM is still refining.
        // Without this, a 1800s local-timeout on one voice silently blocks
        // every later voice on the same chat.
        tokio::spawn(async move {
            if let Err(error) = voice::handle_audio(
                &bot,
                deps.governor.as_ref(),
                &message,
                deps.asr.as_ref(),
                deps.llm.as_ref(),
                deps.auth.as_ref(),
                voice_permit,
            )
            .await
            {
                tracing::error!(%error, "voice handler task failed");
            }
        });
        return Ok(());
    }

    Ok(())
}
