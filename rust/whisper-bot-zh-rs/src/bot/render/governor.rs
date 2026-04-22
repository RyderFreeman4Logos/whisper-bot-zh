use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::{ApiError, RequestError};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio::time::Instant;

use super::super::ThrottledBot;

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
    bot: ThrottledBot,
    edit_states: Arc<Mutex<HashMap<ChatId, Arc<AsyncMutex<EditState>>>>>,
    min_interval: Duration,
    max_retries: usize,
}

impl TelegramGovernor {
    #[must_use]
    pub fn new(bot: ThrottledBot, min_interval: Duration) -> Self {
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

    /// Send a document to a Telegram chat.
    ///
    /// Rate limiting and `RetryAfter` retries are handled by the underlying
    /// [`Throttle`](teloxide::adaptors::throttle::Throttle) adaptor.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the send for any reason other than
    /// rate limiting (which the adaptor retries automatically).
    pub async fn send_document(
        &self,
        chat_id: ChatId,
        document: InputFile,
        caption: String,
    ) -> ResponseResult<Message> {
        self.bot
            .send_document(chat_id, document)
            .caption(caption)
            .await
    }

    /// Send a text message to a Telegram chat.
    ///
    /// Rate limiting and `RetryAfter` retries are handled by the underlying
    /// [`Throttle`](teloxide::adaptors::throttle::Throttle) adaptor.
    ///
    /// # Errors
    /// Returns an error if Telegram rejects the send for any reason other than
    /// rate limiting (which the adaptor retries automatically).
    pub async fn send_message(
        &self,
        chat_id: ChatId,
        text: impl Into<String>,
    ) -> ResponseResult<Message> {
        self.bot.send_message(chat_id, text).await
    }

    pub(super) async fn run_edit_operation<T, Op, Fut>(
        &self,
        chat_id: ChatId,
        mut operation: Op,
    ) -> ResponseResult<T>
    where
        Op: FnMut() -> Fut,
        Fut: Future<Output = ResponseResult<T>>,
    {
        let mut edit_state_guard = self.wait_for_edit_slot(chat_id).await;
        let mut retries = 0;
        loop {
            match operation().await {
                Err(RequestError::RetryAfter(delay)) => {
                    Self::record_retry_after(&mut edit_state_guard, delay.duration());

                    if retries >= self.max_retries {
                        return Err(RequestError::RetryAfter(delay));
                    }

                    retries += 1;
                    tokio::time::sleep(delay.duration()).await;
                }
                Ok(value) => {
                    self.record_edit(&mut edit_state_guard);
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
                Ok(state) => state.next_allowed_at.is_some_and(|next_allowed_at| {
                    now.saturating_duration_since(next_allowed_at) <= stale_after
                }),
                Err(_) => true,
            }
        });
    }

    #[cfg(test)]
    pub(super) fn tracked_chat_count(&self) -> usize {
        self.edit_states.lock().expect("edit governor lock").len()
    }
}
