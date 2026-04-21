use std::time::Duration;

#[path = "support/llm.rs"]
mod support;

use serde_json::json;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use support::{chat_body, refiner, slow_refiner, success_template, SharedWriter};
use whisper_bot_zh::llm::{CloudRefiner, LlmService, LocalRefiner};

#[tokio::test]
async fn cloud_refiner_falls_back_to_next_model() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "broken-model" })))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer cloud-secret"))
        .and(body_partial_json(json!({ "model": "working-model" })))
        .respond_with(success_template("working-model output"))
        .mount(&server)
        .await;

    let cloud = CloudRefiner::new(vec![
        refiner(&server.uri(), "cloud-secret", "broken-model"),
        refiner(&server.uri(), "cloud-secret", "working-model"),
    ]);

    let result = cloud
        .refine("原始文本")
        .await
        .expect("cloud fallback chain should recover");

    assert!(result.ok);
    assert_eq!(result.model, "working-model");
    assert_eq!(result.text, "working-model output");
}

#[tokio::test]
async fn local_refiner_works_against_openai_compatible_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer local-secret"))
        .and(body_partial_json(json!({ "model": "qwen-local" })))
        .respond_with(success_template("local output"))
        .mount(&server)
        .await;

    let local = LocalRefiner::new(refiner(&server.uri(), "local-secret", "qwen-local"));
    let result = local
        .refine("原始文本")
        .await
        .expect("local refinement should succeed");

    assert!(result.ok);
    assert_eq!(result.model, "qwen-local");
    assert_eq!(result.text, "local output");
}

#[tokio::test]
async fn dual_mode_returns_both_results() {
    let cloud_server = MockServer::start().await;
    let local_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "groq-fast" })))
        .respond_with(success_template("cloud output"))
        .mount(&cloud_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "qwen-local" })))
        .respond_with(success_template("local output"))
        .mount(&local_server)
        .await;

    let service = LlmService::from_refiners(
        Some(CloudRefiner::new(vec![refiner(
            &cloud_server.uri(),
            "cloud-secret",
            "groq-fast",
        )])),
        Some(LocalRefiner::new(refiner(
            &local_server.uri(),
            "local-secret",
            "qwen-local",
        ))),
    );

    let results = service
        .refine_dual("原始文本")
        .await
        .expect("dual refinement should return both outputs");

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|result| result.model == "groq-fast"));
    assert!(results.iter().any(|result| result.model == "qwen-local"));
}

#[tokio::test]
async fn dual_progress_streams_first_completed_model() {
    let cloud_server = MockServer::start().await;
    let local_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "groq-fast" })))
        .respond_with(success_template("cloud output"))
        .mount(&cloud_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "qwen-local" })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(120))
                .set_body_json(chat_body("local output")),
        )
        .mount(&local_server)
        .await;

    let service = LlmService::from_refiners(
        Some(CloudRefiner::new(vec![refiner(
            &cloud_server.uri(),
            "cloud-secret",
            "groq-fast",
        )])),
        Some(LocalRefiner::new(refiner(
            &local_server.uri(),
            "local-secret",
            "qwen-local",
        ))),
    );

    let mut updates = whisper_bot_zh::bot::flow::dual::collect(service, "原始文本".to_owned());

    let first = tokio::time::timeout(Duration::from_millis(60), updates.recv())
        .await
        .expect("cloud update should stream before local response")
        .expect("first progress update should exist")
        .expect("first progress update should succeed");
    assert_eq!(
        first.cloud.as_ref().map(|result| result.model.as_str()),
        Some("groq-fast")
    );
    assert!(first.local.is_none());

    let second = tokio::time::timeout(Duration::from_millis(200), updates.recv())
        .await
        .expect("local update should arrive after the first streamed result")
        .expect("second progress update should exist")
        .expect("second progress update should succeed");
    assert_eq!(
        second.local.as_ref().map(|result| result.model.as_str()),
        Some("qwen-local")
    );
    assert_eq!(
        second.cloud.as_ref().map(|result| result.model.as_str()),
        Some("groq-fast")
    );
}

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
