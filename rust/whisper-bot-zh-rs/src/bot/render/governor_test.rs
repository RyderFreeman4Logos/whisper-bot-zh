use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use teloxide::types::{ChatId, Seconds};
use teloxide::{Bot, RequestError};
use tokio::sync::{Mutex, Notify};
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

#[tokio::test(start_paused = true)]
async fn telegram_governor_shares_retry_after_window_across_same_chat_edits() {
    let governor = governor(Duration::ZERO);
    let first_attempts = Arc::new(AtomicUsize::new(0));
    let second_attempts = Arc::new(AtomicUsize::new(0));
    let first_observed = Arc::new(Mutex::new(Vec::<Instant>::new()));
    let second_observed = Arc::new(Mutex::new(Vec::<Instant>::new()));

    let first = tokio::spawn({
        let governor = governor.clone();
        let first_attempts = Arc::clone(&first_attempts);
        let first_observed = Arc::clone(&first_observed);
        async move {
            governor
                .run_edit_operation(ChatId(7), || {
                    let first_attempts = Arc::clone(&first_attempts);
                    let first_observed = Arc::clone(&first_observed);
                    async move {
                        first_observed.lock().await.push(Instant::now());
                        let attempt = first_attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt == 0 {
                            Err(RequestError::RetryAfter(Seconds::from_seconds(1)))
                        } else {
                            Ok::<(), RequestError>(())
                        }
                    }
                })
                .await
                .expect("first edit should eventually succeed");
        }
    });

    tokio::task::yield_now().await;
    assert_eq!(first_attempts.load(Ordering::SeqCst), 1);

    let second = tokio::spawn({
        let governor = governor.clone();
        let second_attempts = Arc::clone(&second_attempts);
        let second_observed = Arc::clone(&second_observed);
        async move {
            governor
                .run_edit_operation(ChatId(7), || {
                    let second_attempts = Arc::clone(&second_attempts);
                    let second_observed = Arc::clone(&second_observed);
                    async move {
                        second_observed.lock().await.push(Instant::now());
                        second_attempts.fetch_add(1, Ordering::SeqCst);
                        Ok::<(), RequestError>(())
                    }
                })
                .await
                .expect("second edit should succeed");
        }
    });

    tokio::task::yield_now().await;
    assert_eq!(second_attempts.load(Ordering::SeqCst), 0);

    tokio::time::advance(Duration::from_millis(999)).await;
    tokio::task::yield_now().await;
    assert_eq!(second_attempts.load(Ordering::SeqCst), 0);

    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;

    first.await.expect("first task should complete");
    second.await.expect("second task should complete");

    assert_eq!(first_attempts.load(Ordering::SeqCst), 2);
    assert_eq!(second_attempts.load(Ordering::SeqCst), 1);

    let first_observed = first_observed.lock().await;
    let second_observed = second_observed.lock().await;
    assert_eq!(first_observed.len(), 2);
    assert_eq!(second_observed.len(), 1);
    assert!(second_observed[0].duration_since(first_observed[0]) >= Duration::from_secs(1));
}

#[tokio::test(start_paused = true)]
async fn telegram_governor_serializes_concurrent_same_chat_edits() {
    let min_interval = Duration::from_millis(500);
    let governor = governor(min_interval);
    let first_started = Arc::new(Notify::new());
    let release_first = Arc::new(Notify::new());
    let first_finished_at = Arc::new(Mutex::new(None::<Instant>));
    let second_started_at = Arc::new(Mutex::new(None::<Instant>));

    let first = tokio::spawn({
        let governor = governor.clone();
        let first_started = Arc::clone(&first_started);
        let release_first = Arc::clone(&release_first);
        let first_finished_at = Arc::clone(&first_finished_at);
        async move {
            governor
                .run_edit_operation(ChatId(7), || {
                    let first_started = Arc::clone(&first_started);
                    let release_first = Arc::clone(&release_first);
                    let first_finished_at = Arc::clone(&first_finished_at);
                    async move {
                        first_started.notify_one();
                        release_first.notified().await;
                        *first_finished_at.lock().await = Some(Instant::now());
                        Ok::<(), RequestError>(())
                    }
                })
                .await
                .expect("first concurrent edit should succeed");
        }
    });

    first_started.notified().await;

    let second = tokio::spawn({
        let governor = governor.clone();
        let second_started_at = Arc::clone(&second_started_at);
        async move {
            governor
                .run_edit_operation(ChatId(7), || {
                    let second_started_at = Arc::clone(&second_started_at);
                    async move {
                        *second_started_at.lock().await = Some(Instant::now());
                        Ok::<(), RequestError>(())
                    }
                })
                .await
                .expect("second concurrent edit should succeed");
        }
    });

    tokio::task::yield_now().await;
    assert!(second_started_at.lock().await.is_none());

    tokio::time::advance(min_interval.saturating_mul(2)).await;
    tokio::task::yield_now().await;
    assert!(second_started_at.lock().await.is_none());

    release_first.notify_one();
    tokio::task::yield_now().await;
    assert!(second_started_at.lock().await.is_none());

    tokio::time::advance(min_interval.saturating_sub(Duration::from_millis(1))).await;
    tokio::task::yield_now().await;
    assert!(second_started_at.lock().await.is_none());

    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;

    first.await.expect("first task should complete");
    second.await.expect("second task should complete");

    let first_finished_at = first_finished_at
        .lock()
        .await
        .expect("first edit should record completion");
    let second_started_at = second_started_at
        .lock()
        .await
        .expect("second edit should record start");
    assert!(second_started_at >= first_finished_at + min_interval);
}
