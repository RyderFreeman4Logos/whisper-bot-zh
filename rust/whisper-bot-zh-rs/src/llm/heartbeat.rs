use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::time::MissedTickBehavior;

/// Run a fallible future while logging periodic heartbeat messages.
///
/// # Errors
/// Returns any error produced by `future`.
pub async fn run_with_heartbeat<T, F>(model: &str, interval: Duration, future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    if interval.is_zero() {
        return future.await;
    }

    let started = Instant::now();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticker.tick().await;

    tokio::pin!(future);

    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = ticker.tick() => {
                tracing::info!("still waiting, elapsed={}s model={model}", started.elapsed().as_secs());
            }
        }
    }
}
