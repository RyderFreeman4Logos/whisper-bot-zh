use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::{ApiError, RequestError};
use tokio::time::Instant;

#[derive(Clone)]
pub struct TelegramGovernor {
    bot: Bot,
    last_edit_at: std::sync::Arc<Mutex<HashMap<ChatId, Instant>>>,
    min_interval: Duration,
    max_retries: usize,
}

impl TelegramGovernor {
    #[must_use]
    pub fn new(bot: Bot, min_interval: Duration) -> Self {
        Self {
            bot,
            last_edit_at: std::sync::Arc::new(Mutex::new(HashMap::new())),
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
        self.wait_for_edit_slot(chat_id).await;
        let value = self.run_write_operation(operation).await?;
        self.record_edit(chat_id);
        Ok(value)
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

    async fn wait_for_edit_slot(&self, chat_id: ChatId) {
        let now = Instant::now();
        let delay = {
            let mut last_edit_at = self.last_edit_at.lock().expect("edit governor lock");
            Self::cleanup(&mut last_edit_at, now, self.min_interval);
            last_edit_at.get(&chat_id).map(|previous| {
                self.min_interval
                    .saturating_sub(now.saturating_duration_since(*previous))
            })
        };
        if let Some(delay) = delay.filter(|delay| !delay.is_zero()) {
            tokio::time::sleep(delay).await;
        }
    }

    fn record_edit(&self, chat_id: ChatId) {
        let now = Instant::now();
        let mut last_edit_at = self.last_edit_at.lock().expect("edit governor lock");
        Self::cleanup(&mut last_edit_at, now, self.min_interval);
        last_edit_at.insert(chat_id, now);
    }

    fn cleanup(last_edit_at: &mut HashMap<ChatId, Instant>, now: Instant, min_interval: Duration) {
        let stale_after = min_interval.max(Duration::from_secs(60));
        last_edit_at.retain(|_, previous| now.saturating_duration_since(*previous) <= stale_after);
    }
}
