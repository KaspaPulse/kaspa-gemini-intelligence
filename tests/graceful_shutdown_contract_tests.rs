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
fn resilient_workers_are_registered_with_a_task_tracker() {
    let runtime = include_str!("../src/infrastructure/resilience/runtime.rs");
    let health = include_str!("../src/infrastructure/webhook_security.rs");

    assert!(runtime.contains("TaskTracker"));
    assert!(runtime.contains("task_tracker().spawn"));
    assert!(runtime.contains("tracker.close();"));
    assert!(runtime.contains("timeout(duration, tracker.wait()).await.is_ok()"));

    assert!(health.contains("spawn_resilient("));
    assert!(health.contains("\"health_endpoint\""));
}
