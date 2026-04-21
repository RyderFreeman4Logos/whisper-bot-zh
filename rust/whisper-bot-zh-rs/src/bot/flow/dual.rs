use anyhow::Result;
use tokio::sync::mpsc;

use crate::llm::{LlmService, RefinementResult};

use super::DualProgress;

#[must_use]
pub fn collect(service: LlmService, transcript: String) -> mpsc::Receiver<Result<DualProgress>> {
    let (tx, rx) = mpsc::channel(2);

    tokio::spawn(async move {
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
                    match result {
                        Ok(result) => {
                            cloud_result = Some(result);
                            cloud_done = true;
                            if tx.send(Ok(DualProgress {
                                cloud: cloud_result.clone(),
                                local: local_result.clone(),
                            })).await.is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error)).await;
                            return;
                        }
                    }
                }
                result = &mut local_future, if !local_done => {
                    match result {
                        Ok(result) => {
                            local_result = Some(result);
                            local_done = true;
                            if tx.send(Ok(DualProgress {
                                cloud: cloud_result.clone(),
                                local: local_result.clone(),
                            })).await.is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error)).await;
                            return;
                        }
                    }
                }
            }
        }
    });

    rx
}
