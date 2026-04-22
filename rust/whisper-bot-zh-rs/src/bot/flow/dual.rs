use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::mpsc;

use crate::llm::{LlmService, RefinementResult};

use super::DualProgress;

#[must_use]
pub fn collect(service: LlmService, transcript: String) -> mpsc::Receiver<Result<DualProgress>> {
    let (tx, rx) = mpsc::channel(4);

    let cloud_model = service.cloud_display_model().unwrap_or_default();
    let local_model = service.local_display_model().unwrap_or_default();

    tokio::spawn(async move {
        let cloud_started = Instant::now();
        let local_started = Instant::now();
        let cloud_future = service.refine_cloud(&transcript);
        let local_future = service.refine_local(&transcript);

        tokio::pin!(cloud_future);
        tokio::pin!(local_future);

        let mut cloud_done = false;
        let mut local_done = false;
        let mut cloud_result: Option<RefinementResult> = None;
        let mut local_result: Option<RefinementResult> = None;

        while !cloud_done || !local_done {
            tokio::select! {
                result = &mut cloud_future, if !cloud_done => {
                    cloud_result = Some(into_soft_result(result, &cloud_model, cloud_started.elapsed()));
                    cloud_done = true;
                    if tx.send(Ok(DualProgress {
                        cloud: cloud_result.clone(),
                        local: local_result.clone(),
                    })).await.is_err() {
                        return;
                    }
                }
                result = &mut local_future, if !local_done => {
                    local_result = Some(into_soft_result(result, &local_model, local_started.elapsed()));
                    local_done = true;
                    if tx.send(Ok(DualProgress {
                        cloud: cloud_result.clone(),
                        local: local_result.clone(),
                    })).await.is_err() {
                        return;
                    }
                }
            }
        }
    });

    rx
}

fn into_soft_result(
    result: Result<RefinementResult>,
    fallback_model: &str,
    elapsed: Duration,
) -> RefinementResult {
    match result {
        Ok(r) => r,
        Err(error) => RefinementResult {
            ok: false,
            text: format!("{error:#}"),
            duration: elapsed,
            model: fallback_model.to_owned(),
        },
    }
}
