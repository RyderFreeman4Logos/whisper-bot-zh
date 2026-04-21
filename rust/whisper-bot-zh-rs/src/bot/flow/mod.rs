//! Voice-message orchestration helpers for single and dual LLM flows.

use crate::llm::RefinementResult;

pub mod dual;
pub mod single;

#[derive(Debug, Clone, Default)]
pub struct DualProgress {
    pub cloud: Option<RefinementResult>,
    pub local: Option<RefinementResult>,
}

impl DualProgress {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.cloud.is_some() && self.local.is_some()
    }
}
