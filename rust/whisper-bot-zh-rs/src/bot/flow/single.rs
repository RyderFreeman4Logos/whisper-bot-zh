use anyhow::Result;

use crate::llm::{LlmService, RefinementResult};

pub async fn collect(service: &LlmService, transcript: &str) -> Result<RefinementResult> {
    service.refine_single(transcript).await
}
