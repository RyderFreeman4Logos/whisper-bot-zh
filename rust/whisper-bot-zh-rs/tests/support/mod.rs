use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tracing_subscriber::fmt::MakeWriter;
use wiremock::ResponseTemplate;

use whisper_bot_zh::llm::ChatRefiner;

pub fn refiner(base_url: &str, api_key: &str, model: &str) -> ChatRefiner {
    ChatRefiner::new(
        reqwest::Client::new(),
        base_url,
        api_key.to_owned(),
        model,
        Duration::from_secs(2),
        Duration::from_secs(1),
        0.2,
        None,
        None,
    )
}

pub fn slow_refiner(base_url: &str) -> ChatRefiner {
    ChatRefiner::new(
        reqwest::Client::new(),
        base_url,
        "cloud-secret".to_owned(),
        "slow-model",
        Duration::from_secs(2),
        Duration::from_millis(20),
        0.2,
        None,
        None,
    )
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

#[derive(Clone, Default)]
pub struct SharedWriter {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl SharedWriter {
    pub fn contents(&self) -> String {
        String::from_utf8(self.inner.lock().expect("buffer lock").clone()).expect("utf8 logs")
    }
}

impl<'a> MakeWriter<'a> for SharedWriter {
    type Writer = BufferGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferGuard {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct BufferGuard {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl io::Write for BufferGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner
            .lock()
            .expect("buffer lock")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
