use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[test]
fn main_owns_signal_and_database_shutdown_lifecycle() {
    let source = include_str!("../src/main.rs");

    assert!(source.contains("async fn wait_for_shutdown_signal()"));
    assert!(source.contains("cancel_token.cancel();"));
    assert!(source.contains("drain_tracked_tasks(drain_timeout).await"));
    assert!(source.contains("pool.close().await;"));
    assert!(!source.contains(".enable_ctrlc_handler()"));
    assert!(!source.contains(
        "Waiting {} seconds for background workers to drain before closing database pool"
    ));
}

#[test]
fn resilient_workers_are_registered_for_bounded_drain() {
    let runtime = include_str!("../src/infrastructure/resilience/runtime.rs");
    let health = include_str!("../src/infrastructure/webhook_security.rs");

    assert!(runtime.contains("AtomicUsize"));
    assert!(runtime.contains("active_task_count().fetch_add"));
    assert!(runtime.contains("task_completion_notify().notify_one()"));
    assert!(runtime.contains("task_completion_notify().notified().await"));

    assert!(health.contains("spawn_resilient("));
    assert!(health.contains("\"health_endpoint\""));
}

#[tokio::test]
async fn drain_waits_for_a_registered_worker_to_finish() {
    let finished = Arc::new(AtomicBool::new(false));
    let worker_finished = Arc::clone(&finished);

    kaspa_pulse::infrastructure::resilience::runtime::spawn_resilient(
        "graceful_shutdown_contract_test",
        async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            worker_finished.store(true, Ordering::Release);
        },
    );

    assert!(
        kaspa_pulse::infrastructure::resilience::runtime::drain_tracked_tasks(Duration::from_secs(
            1
        ))
        .await
    );
    assert!(finished.load(Ordering::Acquire));
}
