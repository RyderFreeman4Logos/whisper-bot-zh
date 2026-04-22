//! teloxide entrypoint + dispatcher wiring.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{Context, Result};
use teloxide::adaptors::throttle::{Limits, Throttle};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::requests::RequesterExt;
use tokio::sync::{oneshot, Semaphore};

use crate::asr::AsrService;
use crate::auth::AuthService;
use crate::config::Settings;
use crate::llm::LlmService;

/// Bot adaptor stack: a [`Throttle`]-wrapped [`Bot`] that enforces Telegram's
/// documented per-chat / global send rate limits and auto-retries `RetryAfter`
/// for `send_*` calls. `edit_*` calls pass through unchanged and are still
/// paced by [`render::TelegramGovernor`].
pub type ThrottledBot = Throttle<Bot>;

pub mod commands;
pub mod flow;
pub mod render;
pub mod telegram_limit;
pub mod voice;

type HandlerResult = ResponseResult<()>;
const VOICE_AUTH_WARNING: &str = "🔒 请先使用 /start <password> 或 /auth <password> 完成认证。";

#[derive(Clone)]
struct DispatchDeps {
    asr: Arc<AsrService>,
    auth: Arc<AuthService>,
    governor: Arc<render::TelegramGovernor>,
    llm: Arc<LlmService>,
    voice_limiter: Arc<Semaphore>,
    /// Per-chat completion oneshot chain. Each new voice in a chat takes the
    /// previous task's `Receiver` (paired with that task's "done" `Sender`) and
    /// inserts its own. Awaiting the prior `Receiver` enforces FIFO processing
    /// per chat while still letting different chats run in parallel up to
    /// `voice_limiter`'s capacity.
    chat_chain: Arc<StdMutex<HashMap<ChatId, oneshot::Receiver<()>>>>,
}

/// Start the Telegram bot dispatcher and serve updates until shutdown.
///
/// # Errors
/// Returns an error if startup dependencies cannot be constructed or if the bot
/// profile lookup fails before the dispatcher starts.
pub async fn run(settings: Settings) -> Result<()> {
    let settings = Arc::new(settings);
    let bot: ThrottledBot =
        Bot::with_client(settings.bot_token.clone(), settings.telegram_client()?)
            .throttle(Limits::default());
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
    let chat_chain = Arc::new(StdMutex::new(HashMap::new()));
    let deps = Arc::new(DispatchDeps {
        asr,
        auth,
        governor,
        llm,
        voice_limiter,
        chat_chain,
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
    bot: ThrottledBot,
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
        let Some(user) = message.from.as_ref() else {
            if let Err(error) = deps
                .governor
                .send_reply(message.chat.id, message.id, VOICE_AUTH_WARNING)
                .await
            {
                tracing::warn!(%error, "failed to send voice auth warning");
            }
            return Ok(());
        };
        if !deps.auth.is_user_allowed(user.id.0).await {
            if let Err(error) = deps
                .governor
                .send_reply(message.chat.id, message.id, VOICE_AUTH_WARNING)
                .await
            {
                tracing::warn!(%error, "failed to send voice auth warning");
            }
            return Ok(());
        }

        // Reserve this voice's slot in the per-chat FIFO queue synchronously,
        // so arrival order at the dispatcher (which is itself serial) is the
        // order voices get processed for any given chat. Different chats can
        // still progress in parallel up to `voice_limiter`'s capacity.
        let (prev_done_rx, my_done_tx) = {
            let mut chain = deps.chat_chain.lock().expect("chat chain lock poisoned");
            let (my_done_tx, my_done_rx) = oneshot::channel::<()>();
            let prev = chain.insert(message.chat.id, my_done_rx);
            (prev, my_done_tx)
        };

        // Detach the voice handler so the dispatcher can keep receiving
        // subsequent messages while a slow local LLM is still refining.
        // Permit acquisition happens INSIDE the spawned task so that excess
        // voices queue up (waiting on the semaphore) instead of being rejected.
        tokio::spawn(async move {
            // Wait for the previous voice in this chat to finish (FIFO order).
            // A dropped predecessor (panic/abort) yields Err which we ignore so
            // the chain self-heals instead of permanently stalling the chat.
            if let Some(rx) = prev_done_rx {
                let _ = rx.await;
            }
            let voice_permit = match deps.voice_limiter.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(error) => {
                    tracing::error!(%error, "voice handler semaphore closed");
                    let _ = my_done_tx.send(());
                    return;
                }
            };
            if let Err(error) = voice::handle_audio(
                &bot,
                deps.governor.as_ref(),
                &message,
                deps.asr.as_ref(),
                deps.llm.as_ref(),
                voice_permit,
            )
            .await
            {
                tracing::error!(%error, "voice handler task failed");
            }
            let _ = my_done_tx.send(());
        });
        return Ok(());
    }

    Ok(())
}
