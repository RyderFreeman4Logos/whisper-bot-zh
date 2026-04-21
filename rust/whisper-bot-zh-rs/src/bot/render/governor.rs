use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::{ApiError, RequestError};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::Instant;

struct LastEditState {
    last_edit_at: Option<Instant>,
}

#[derive(Clone)]
pub struct TelegramGovernor {
    bot: Bot,
    edit_states: Arc<Mutex<HashMap<ChatId, Arc<AsyncMutex<LastEditState>>>>>,
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
        let edit_state = self.wait_for_edit_slot(chat_id).await;
        let result = self.run_write_operation(operation).await;
        drop(edit_state);
        result
    }

    pub(super) async fn run_write_operation<T, Op, Fut>(
        &self,
        mut operation: Op,
    ) -> ResponseResult<T>
    where
        Op: FnMut() -> Fut,
        Fut: Future<Output = ResponseResult<T>>,
    {
        let mut retries = 0;
        loop {
            match operation().await {
                Err(RequestError::RetryAfter(delay)) if retries < self.max_retries => {
                    retries += 1;
                    tokio::time::sleep(delay.duration()).await;
                }
                result => return result,
            }
        }
    }

    async fn wait_for_edit_slot(&self, chat_id: ChatId) -> Arc<AsyncMutex<LastEditState>> {
        let edit_state = self.edit_state(chat_id);
        let mut state = edit_state.lock().await;
        let now = Instant::now();
        let delay = state.last_edit_at.map_or(Duration::ZERO, |last_edit_at| {
            self.min_interval
                .saturating_sub(now.saturating_duration_since(last_edit_at))
        });
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        state.last_edit_at = Some(Instant::now());
        drop(state);
        edit_state
    }

    fn edit_state(&self, chat_id: ChatId) -> Arc<AsyncMutex<LastEditState>> {
        let now = Instant::now();
        let mut edit_states = self.edit_states.lock().expect("edit governor lock");
        Self::cleanup(&mut edit_states, now, self.min_interval);
        Arc::clone(
            edit_states
                .entry(chat_id)
                .or_insert_with(|| Arc::new(AsyncMutex::new(LastEditState { last_edit_at: None }))),
        )
    }

    fn cleanup(
        edit_states: &mut HashMap<ChatId, Arc<AsyncMutex<LastEditState>>>,
        now: Instant,
        min_interval: Duration,
    ) {
        let stale_after = min_interval.max(Duration::from_secs(60));
        edit_states.retain(|_, state| {
            if Arc::strong_count(state) > 1 {
                return true;
            }

            match state.try_lock() {
                Ok(state) => state.last_edit_at.is_none_or(|last_edit_at| {
                    now.saturating_duration_since(last_edit_at) <= stale_after
                }),
                Err(_) => true,
            }
        });
    }
}
