use std::time::{Duration, Instant};

use serde_json::json;
use std::sync::Arc;
use tokio::sync::Semaphore;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use whisper_bot_zh::llm::prompt::SYSTEM_PROMPT;
use whisper_bot_zh::llm::{ChatRefiner, CloudRefiner, LocalRefiner};

fn limiter(size: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(size))
}

fn refiner(base_url: &str, api_key: &str, model: &str) -> ChatRefiner {
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

#[tokio::test]
async fn cloud_semaphore_serializes_when_configured_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "groq-fast" })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(200))
                .set_body_json(
                    json!({ "choices": [{ "message": { "content": "cloud output" } }] }),
                ),
        )
        .mount(&server)
        .await;

    let refiner = CloudRefiner::new(
        vec![refiner(&server.uri(), "cloud-secret", "groq-fast")],
        limiter(1),
    );
    let started = Instant::now();
    let first = refiner.refine("原始文本 1");
    let second = refiner.refine("原始文本 2");
    let (first, second) = tokio::join!(first, second);

    assert!(first.is_ok());
    assert!(second.is_ok());
    assert!(started.elapsed() >= Duration::from_millis(380));
}

#[tokio::test]
async fn local_semaphore_serializes_when_configured_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "qwen-local" })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(200))
                .set_body_json(
                    json!({ "choices": [{ "message": { "content": "local output" } }] }),
                ),
        )
        .mount(&server)
        .await;

    let refiner = LocalRefiner::new(
        refiner(&server.uri(), "local-secret", "qwen-local"),
        limiter(1),
    );
    let started = Instant::now();
    let first = refiner.refine("原始文本 1");
    let second = refiner.refine("原始文本 2");
    let (first, second) = tokio::join!(first, second);

    assert!(first.is_ok());
    assert!(second.is_ok());
    assert!(started.elapsed() >= Duration::from_millis(380));
}
