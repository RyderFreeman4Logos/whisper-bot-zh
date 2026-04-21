use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use teloxide::types::{ChatId, Seconds};
use teloxide::{Bot, RequestError};
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::governor::TelegramGovernor;

fn governor(min_interval: Duration) -> TelegramGovernor {
    TelegramGovernor::new(Bot::new("123456:TEST-TOKEN"), min_interval)
}

#[tokio::test(start_paused = true)]
async fn telegram_governor_respects_min_interval() {
    let governor = governor(Duration::from_millis(500));
    let observed = Arc::new(Mutex::new(Vec::<Instant>::new()));

    governor
        .run_edit_operation(ChatId(7), || {
            let observed = Arc::clone(&observed);
            async move {
                observed.lock().await.push(Instant::now());
                Ok::<(), RequestError>(())
            }
        })
        .await
        .expect("first edit should succeed");

    let waiter = tokio::spawn({
        let governor = governor.clone();
        let observed = Arc::clone(&observed);
        async move {
            governor
                .run_edit_operation(ChatId(7), || {
                    let observed = Arc::clone(&observed);
                    async move {
                        observed.lock().await.push(Instant::now());
                        Ok::<(), RequestError>(())
                    }
                })
                .await
                .expect("second edit should succeed");
        }
    });

    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(499)).await;
    tokio::task::yield_now().await;
    assert_eq!(observed.lock().await.len(), 1);

    tokio::time::advance(Duration::from_millis(1)).await;
    waiter.await.expect("task should complete");

    let observed = observed.lock().await;
    assert_eq!(observed.len(), 2);
    assert_eq!(
        observed[1].duration_since(observed[0]),
        Duration::from_millis(500)
    );
}

#[tokio::test(start_paused = true)]
async fn telegram_governor_retries_on_retry_after() {
    let governor = governor(Duration::ZERO);
    let attempts = Arc::new(AtomicUsize::new(0));

    let task = tokio::spawn({
        let governor = governor.clone();
        let attempts = Arc::clone(&attempts);
        async move {
            governor
                .run_write_operation(|| {
                    let attempts = Arc::clone(&attempts);
                    async move {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt < 2 {
                            Err(RequestError::RetryAfter(Seconds::from_seconds(1)))
                        } else {
                            Ok::<(), RequestError>(())
                        }
                    }
                })
                .await
                .expect("request should eventually succeed");
        }
    });

    tokio::task::yield_now().await;
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    tokio::time::advance(Duration::from_secs(1)).await;
    task.await.expect("task should complete");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}
