use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::Semaphore;
use wiremock::ResponseTemplate;

use whisper_bot_zh::llm::prompt::SYSTEM_PROMPT;
use whisper_bot_zh::llm::ChatRefiner;

pub fn refiner(base_url: &str, api_key: &str, model: &str) -> ChatRefiner {
    ChatRefiner::new(
        reqwest::Client::new(),
        base_url,
        api_key.to_owned(),
        model,
        SYSTEM_PROMPT.to_owned(),
        Duration::from_secs(2),
        Duration::from_secs(1),
        0.2,
        None,
        None,
        None,
    )
}

pub fn limiter(size: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(size))
}

pub fn success_template(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(chat_body(content))
}

pub fn chat_body(content: &str) -> serde_json::Value {
    json!({
        "choices": [
            {
                "message": {
                    "content": content
                }
            }
        ]
    })
}
