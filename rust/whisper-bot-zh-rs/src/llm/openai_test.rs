use serde_json::Value;

use super::{ChatMessage, ChatRequest};

#[test]
fn reasoning_effort_none_serializes_field() {
    let payload = ChatRequest {
        model: "qwen-local",
        messages: [
            ChatMessage {
                role: "system",
                content: "system",
            },
            ChatMessage {
                role: "user",
                content: "user",
            },
        ],
        temperature: 0.2,
        top_p: None,
        max_tokens: None,
        reasoning_effort: Some("none"),
    };
    let json = serde_json::to_value(payload).expect("serialize request");

    assert_eq!(
        json.get("model"),
        Some(&Value::String("qwen-local".to_owned()))
    );
    assert_eq!(
        json.get("reasoning_effort"),
        Some(&Value::String("none".to_owned()))
    );
}

#[test]
fn reasoning_effort_none_omits_field_when_absent() {
    let payload = ChatRequest {
        model: "qwen-local",
        messages: [
            ChatMessage {
                role: "system",
                content: "system",
            },
            ChatMessage {
                role: "user",
                content: "user",
            },
        ],
        temperature: 0.2,
        top_p: None,
        max_tokens: None,
        reasoning_effort: None,
    };
    let json = serde_json::to_value(payload).expect("serialize request");

    assert_eq!(json.get("reasoning_effort"), None::<&Value>);
}
