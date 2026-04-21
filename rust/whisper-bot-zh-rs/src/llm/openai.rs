use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::heartbeat::run_with_heartbeat;
use super::prompt;
use super::RefinementResult;

#[derive(Clone)]
pub struct ChatRefiner {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
    system_prompt: String,
    timeout: std::time::Duration,
    heartbeat_interval: std::time::Duration,
    temperature: f32,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    reasoning_effort: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 2],
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatAssistantMessage,
}

#[derive(Debug, Deserialize)]
struct ChatAssistantMessage {
    content: String,
}

impl ChatRefiner {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: reqwest::Client,
        base_url: &str,
        api_key: String,
        model: &str,
        system_prompt: String,
        timeout: std::time::Duration,
        heartbeat_interval: std::time::Duration,
        temperature: f32,
        top_p: Option<f32>,
        max_tokens: Option<u32>,
        reasoning_effort: Option<String>,
    ) -> Self {
        Self {
            client,
            endpoint: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            api_key,
            model: model.to_owned(),
            system_prompt,
            timeout,
            heartbeat_interval,
            temperature,
            top_p,
            max_tokens,
            reasoning_effort,
        }
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Refine a transcript by calling the configured chat-completions endpoint.
    ///
    /// # Errors
    /// Returns an error if the request times out, the HTTP call fails, the
    /// response body is invalid, or the model returns an empty/non-success result.
    pub async fn refine(&self, transcript: &str) -> Result<RefinementResult> {
        let started = Instant::now();
        let request = self.request(transcript);
        let response_body = run_with_heartbeat(self.model(), self.heartbeat_interval, async {
            tokio::time::timeout(self.timeout, request)
                .await
                .with_context(|| format!("LLM request timed out for model {}", self.model))?
        })
        .await?;

        let response: ChatResponse =
            serde_json::from_str(&response_body).context("failed to parse LLM json response")?;
        let content = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_owned())
            .filter(|text| !text.is_empty())
            .context("LLM returned an empty response")?;

        Ok(RefinementResult {
            ok: true,
            text: content,
            duration: started.elapsed(),
            model: self.model.clone(),
        })
    }

    async fn request(&self, transcript: &str) -> Result<String> {
        let user_message = prompt::user_message(transcript);
        let payload = ChatRequest {
            model: &self.model,
            messages: [
                ChatMessage {
                    role: "system",
                    content: &self.system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: &user_message,
                },
            ],
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            reasoning_effort: self.reasoning_effort.as_deref(),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("failed to call LLM model {}", self.model))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read LLM response body")?;

        if !status.is_success() {
            bail!("LLM request failed with {status}: {body}");
        }

        Ok(body)
    }
}

#[cfg(test)]
#[path = "openai_test.rs"]
mod tests;
