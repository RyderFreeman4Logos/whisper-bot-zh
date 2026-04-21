use bytes::Bytes;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use whisper_bot_zh::asr::AsrService;

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
