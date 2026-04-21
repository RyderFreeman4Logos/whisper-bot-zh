//! teloxide entrypoint + dispatcher wiring.

use std::sync::Arc;

use anyhow::Result;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;

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

pub async fn run(settings: Settings) -> Result<()> {
    let settings = Arc::new(settings);
    let bot = Bot::new(settings.bot_token.clone());
    let asr = Arc::new(AsrService::new(settings.as_ref())?);
    let auth = Arc::new(AuthService::new(settings.as_ref()).await?);
    let llm = Arc::new(LlmService::new(settings.as_ref())?);

    let handler = Update::filter_message().endpoint(dispatch_message);

    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![asr, auth, llm])
        .enable_ctrlc_handler()
        .build();

    Box::pin(dispatcher.dispatch()).await;

    Ok(())
}

async fn dispatch_message(
    bot: Bot,
    message: Message,
    asr: Arc<AsrService>,
    auth: Arc<AuthService>,
    llm: Arc<LlmService>,
) -> HandlerResult {
    if let Some(text) = message.text() {
        if commands::is_supported_command(text) {
            return commands::handle_command(&bot, &message, text, auth.as_ref()).await;
        }
    }

    if message.voice().is_some() || message.audio().is_some() {
        return Box::pin(voice::handle_audio(
            &bot,
            &message,
            asr.as_ref(),
            llm.as_ref(),
            auth.as_ref(),
        ))
        .await;
    }

    Ok(())
}
