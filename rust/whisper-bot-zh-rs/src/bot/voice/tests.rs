use std::time::Duration;

use crate::bot::{flow::DualProgress, render};
use crate::llm::RefinementResult;

use super::handler::should_defer_dual_file_delivery;

fn result(model: &str, text: String) -> RefinementResult {
    RefinementResult {
        ok: true,
        text,
        duration: Duration::from_secs(1),
        model: model.to_owned(),
    }
}

#[test]
fn defers_partial_dual_updates_that_would_send_files() {
    let long_text = "中".repeat(4_100);
    let snapshot = DualProgress {
        cloud: Some(result("cloud", long_text.clone())),
        local: None,
    };
    let reply = render::dual_refinement_reply(snapshot.cloud.as_ref(), snapshot.local.as_ref());

    assert!(should_defer_dual_file_delivery(&snapshot, &reply));
}

#[test]
fn allows_final_dual_updates_to_send_files() {
    let long_text = "中".repeat(4_100);
    let snapshot = DualProgress {
        cloud: Some(result("cloud", long_text.clone())),
        local: Some(result("local", long_text)),
    };
    let reply = render::dual_refinement_reply(snapshot.cloud.as_ref(), snapshot.local.as_ref());

    assert!(!should_defer_dual_file_delivery(&snapshot, &reply));
}

#[test]
fn allows_partial_dual_updates_that_stay_under_text_limit() {
    let snapshot = DualProgress {
        cloud: Some(result("cloud", "短文本".to_owned())),
        local: None,
    };
    let reply = render::dual_refinement_reply(snapshot.cloud.as_ref(), snapshot.local.as_ref());

    assert!(!should_defer_dual_file_delivery(&snapshot, &reply));
}
