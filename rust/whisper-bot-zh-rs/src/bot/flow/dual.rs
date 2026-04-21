use anyhow::Result;

use crate::llm::{LlmService, RefinementResult};

use super::DualProgress;

pub async fn collect(service: &LlmService, transcript: &str) -> Result<Vec<DualProgress>> {
    let cloud_future = service.refine_cloud(transcript);
    let local_future = service.refine_local(transcript);

    tokio::pin!(cloud_future);
    tokio::pin!(local_future);

    let mut cloud_done = false;
    let mut local_done = false;
    let mut cloud_result: Option<RefinementResult> = None;
    let mut local_result: Option<RefinementResult> = None;
    let mut updates = Vec::with_capacity(2);

    while !cloud_done || !local_done {
        tokio::select! {
            result = &mut cloud_future, if !cloud_done => {
                cloud_result = Some(result?);
                cloud_done = true;
                updates.push(DualProgress {
                    cloud: cloud_result.clone(),
                    local: local_result.clone(),
                });
            }
            result = &mut local_future, if !local_done => {
                local_result = Some(result?);
                local_done = true;
                updates.push(DualProgress {
                    cloud: cloud_result.clone(),
                    local: local_result.clone(),
                });
            }
        }
    }

    Ok(updates)
}
