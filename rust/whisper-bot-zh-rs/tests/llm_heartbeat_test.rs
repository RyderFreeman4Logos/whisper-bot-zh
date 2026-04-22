use std::time::Duration;

#[path = "support/llm_heartbeat.rs"]
mod support;

use serde_json::json;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use support::{chat_body, slow_refiner, SharedWriter};

#[tokio::test(flavor = "current_thread")]
async fn heartbeat_logs_while_waiting_for_slow_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "slow-model" })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(90))
                .set_body_json(chat_body("slow output")),
        )
        .mount(&server)
        .await;

    let writer = SharedWriter::default();
    let subscriber = FmtSubscriber::builder()
        .with_ansi(false)
        .with_max_level(Level::INFO)
        .without_time()
        .with_writer(writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let result = slow_refiner(&server.uri())
        .refine("原始文本")
        .await
        .expect("slow request should still succeed");

    let logs = writer.contents();
    assert_eq!(result.text, "slow output");
    assert!(logs.contains("still waiting, elapsed="));
    assert!(logs.contains("slow-model"));
}
