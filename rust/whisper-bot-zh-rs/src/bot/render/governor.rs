use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::{ApiError, RequestError};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio::time::Instant;

struct EditState {
    next_allowed_at: Option<Instant>,
}

impl EditState {
    fn delay_until_allowed(&self, now: Instant) -> Duration {
        self.next_allowed_at
            .map_or(Duration::ZERO, |next_allowed_at| {
                next_allowed_at.saturating_duration_since(now)
            })
    }

    fn extend_to(&mut self, next_allowed_at: Instant) {
        self.next_allowed_at = Some(
            self.next_allowed_at
                .map_or(next_allowed_at, |current| current.max(next_allowed_at)),
        );
    }
}

#[derive(Clone)]
pub struct TelegramGovernor {
    bot: Bot,
    edit_states: Arc<Mutex<HashMap<ChatId, Arc<AsyncMutex<EditState>>>>>,
    min_interval: Duration,
    max_retries: usize,
}

impl TelegramGovernor {
    #[must_use]
    pub fn new(bot: Bot, min_interval: Duration) -> Self {
        Self {
            bot,
            edit_states: Arc::new(Mutex::new(HashMap::new())),
            min_interval,
            max_retries: 3,
        }
    }

    /// Edit a Telegram message with plain text while honoring per-chat pacing.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the edit with an error other than
    /// `MessageNotModified`, or if retries are exhausted after `RetryAfter`.
    pub async fn edit_plain_text(&self, target: &Message, text: &str) -> ResponseResult<()> {
        let chat_id = target.chat.id;
        let message_id = target.id;
        let text = text.to_owned();
        match self
            .run_edit_operation(chat_id, || {
                let bot = self.bot.clone();
                let text = text.clone();
                async move {
                    bot.edit_message_text(chat_id, message_id, text)
                        .await
                        .map(|_| ())
                }
            })
            .await
        {
            Ok(()) | Err(RequestError::Api(ApiError::MessageNotModified)) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Edit a Telegram message with HTML-formatted text while honoring per-chat pacing.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the edit with an error other than
    /// `MessageNotModified`, or if retries are exhausted after `RetryAfter`.
    pub async fn edit_html_text(&self, target: &Message, html: String) -> ResponseResult<()> {
        let chat_id = target.chat.id;
        let message_id = target.id;
        match self
            .run_edit_operation(chat_id, || {
                let bot = self.bot.clone();
                let html = html.clone();
                async move {
                    bot.edit_message_text(chat_id, message_id, html)
                        .parse_mode(ParseMode::Html)
                        .await
                        .map(|_| ())
                }
            })
            .await
        {
            Ok(()) | Err(RequestError::Api(ApiError::MessageNotModified)) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Send a document to a Telegram chat with retry handling for write limits.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the send operation, or if retries
    /// are exhausted after `RetryAfter`.
    pub async fn send_document(
        &self,
        chat_id: ChatId,
        document: InputFile,
        caption: String,
    ) -> ResponseResult<Message> {
        self.run_write_operation(|| {
            let bot = self.bot.clone();
            let document = document.clone();
            let caption = caption.clone();
            async move { bot.send_document(chat_id, document).caption(caption).await }
        })
        .await
    }

    /// Send a text message to a Telegram chat with retry handling for write limits.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the send operation, or if retries
    /// are exhausted after `RetryAfter`.
    pub async fn send_message(
        &self,
        chat_id: ChatId,
        text: impl Into<String>,
    ) -> ResponseResult<Message> {
        let text = text.into();
        self.run_write_operation(|| {
            let bot = self.bot.clone();
            let text = text.clone();
            async move { bot.send_message(chat_id, text).await }
        })
        .await
    }

    pub(super) async fn run_edit_operation<T, Op, Fut>(
        &self,
        chat_id: ChatId,
        operation: Op,
    ) -> ResponseResult<T>
    where
        Op: FnMut() -> Fut,
        Fut: Future<Output = ResponseResult<T>>,
    {
        let edit_state_guard = self.wait_for_edit_slot(chat_id).await;
        self.run_write_operation_with_edit_state(operation, Some(edit_state_guard))
            .await
    }

    pub(super) async fn run_write_operation<T, Op, Fut>(&self, operation: Op) -> ResponseResult<T>
    where
        Op: FnMut() -> Fut,
        Fut: Future<Output = ResponseResult<T>>,
    {
        self.run_write_operation_with_edit_state(operation, None)
            .await
    }

    async fn run_write_operation_with_edit_state<T, Op, Fut>(
        &self,
        mut operation: Op,
        mut edit_state_guard: Option<OwnedMutexGuard<EditState>>,
    ) -> ResponseResult<T>
    where
        Op: FnMut() -> Fut,
        Fut: Future<Output = ResponseResult<T>>,
    {
        let mut retries = 0;
        loop {
            match operation().await {
                Err(RequestError::RetryAfter(delay)) => {
                    if let Some(state) = edit_state_guard.as_mut() {
                        Self::record_retry_after(state, delay.duration());
                    }

                    if retries >= self.max_retries {
                        return Err(RequestError::RetryAfter(delay));
                    }

                    retries += 1;
                    tokio::time::sleep(delay.duration()).await;
                }
                Ok(value) => {
                    if let Some(state) = edit_state_guard.as_mut() {
                        self.record_edit(state);
                    }
                    return Ok(value);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn wait_for_edit_slot(&self, chat_id: ChatId) -> OwnedMutexGuard<EditState> {
        let edit_state = self.edit_state(chat_id);
        loop {
            let state = Arc::clone(&edit_state).lock_owned().await;
            let delay = state.delay_until_allowed(Instant::now());

            if delay.is_zero() {
                return state;
            }

            drop(state);
            tokio::time::sleep(delay).await;
        }
    }

    fn record_edit(&self, state: &mut EditState) {
        state.extend_to(Instant::now() + self.min_interval);
    }

    fn record_retry_after(state: &mut EditState, retry_after: Duration) {
        state.extend_to(Instant::now() + retry_after);
    }

    fn edit_state(&self, chat_id: ChatId) -> Arc<AsyncMutex<EditState>> {
        let now = Instant::now();
        let mut edit_states = self.edit_states.lock().expect("edit governor lock");
        Self::cleanup(&mut edit_states, now, self.min_interval);
        Arc::clone(edit_states.entry(chat_id).or_insert_with(|| {
            Arc::new(AsyncMutex::new(EditState {
                next_allowed_at: None,
            }))
        }))
    }

    fn cleanup(
        edit_states: &mut HashMap<ChatId, Arc<AsyncMutex<EditState>>>,
        now: Instant,
        min_interval: Duration,
    ) {
        let stale_after = min_interval.max(Duration::from_secs(60));
        edit_states.retain(|_, state| {
            if Arc::strong_count(state) > 1 {
                return true;
            }

            match state.try_lock() {
                Ok(state) => state.next_allowed_at.is_none_or(|next_allowed_at| {
                    now.saturating_duration_since(next_allowed_at) <= stale_after
                }),
                Err(_) => true,
            }
        });
    }
}
