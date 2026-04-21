use anyhow::Result;

use crate::llm::{LlmService, RefinementResult};

/// Run the single-model refinement flow for a transcript.
///
/// # Errors
/// Returns any error produced by the configured single-model refiner.
pub async fn collect(service: &LlmService, transcript: &str) -> Result<RefinementResult> {
    service.refine_single(transcript).await
}
