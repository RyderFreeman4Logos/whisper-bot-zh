use std::fs;

use bytes::Bytes;
use tempfile::tempdir;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use whisper_bot_zh::asr::AsrService;
use whisper_bot_zh::config::Settings;

use env_support::EnvGuard;

#[path = "support/env.rs"]
mod env_support;

#[tokio::test]
async fn sends_groq_compatible_multipart_request() {
    let server = MockServer::start().await;
    let response =
        ResponseTemplate::new(200).set_body_raw(r#"{"text":"你好，世界"}"#, "application/json");

    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .and(header("authorization", "Bearer secret-token"))
        .and(body_string_contains(
            "name=\"model\"\r\n\r\nwhisper-large-v3",
        ))
        .and(body_string_contains("name=\"language\"\r\n\r\nzh"))
        .and(body_string_contains(
            "name=\"prompt\"\r\n\r\n以下是一段简体中文内容:",
        ))
        .and(body_string_contains("name=\"temperature\"\r\n\r\n0"))
        .and(body_string_contains("name=\"response_format\"\r\n\r\njson"))
        .and(body_string_contains(
            "name=\"file\"; filename=\"audio.wav\"",
        ))
        .respond_with(response)
        .mount(&server)
        .await;

    let service = AsrService::from_parts(
        reqwest::Client::new(),
        &server.uri(),
        "secret-token".to_owned(),
        "whisper-large-v3",
        "zh",
        "以下是一段简体中文内容:",
        0.0,
        1,
    );

    let transcript = service
        .transcribe(Bytes::from_static(b"fake-audio"))
        .await
        .expect("ASR request should succeed");

    assert_eq!(transcript, "你好，世界");
}

#[test]
fn accepts_openai_api_key_when_groq_keys_are_absent() {
    let _env = EnvGuard::set(&[
        ("BOT_TOKEN", None),
        ("ACCESS_PASSWORD", None),
        ("ASR_API_KEY", None),
        ("GROQ_API", None),
        ("GROQ_API_KEY", None),
        ("OPENAI_API_KEY", None),
    ]);
    let temp_dir = tempdir().expect("temp dir");
    let env_file = temp_dir.path().join(".env");
    fs::write(
        &env_file,
        "BOT_TOKEN=test-token\nACCESS_PASSWORD=test-password\nOPENAI_API_KEY=openai-secret\n",
    )
    .expect("write env file");

    let settings = Settings::load(Some(&env_file), None).expect("load settings");
    let service = AsrService::new(&settings).expect("OPENAI_API_KEY should satisfy ASR auth");

    assert_eq!(settings.asr_effective_api_key(), Some("openai-secret"));
    assert_eq!(service.model(), "whisper-large-v3");
}
