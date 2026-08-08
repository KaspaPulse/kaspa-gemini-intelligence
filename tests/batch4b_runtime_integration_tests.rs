use std::fs;

#[test]
fn callback_execution_guard_is_injected_and_enforced() {
    let main_source = include_str!("../src/main.rs");
    let callback_source = include_str!("../src/presentation/telegram/handlers/callback.rs");
    let callback_inflight = include_str!("../src/presentation/telegram/callback_inflight.rs");

    assert!(main_source.contains("CallbackExecutionRegistry::default()"));
    assert!(main_source.contains("callback_execution_registry"));
    assert!(callback_source.contains("try_acquire"));
    assert!(callback_source.contains("Another action is already running"));
    assert!(callback_inflight.contains("CallbackExecutionGuard"));
}

#[test]
fn final_audit_fixes_preserve_contextual_callback_recovery() {
    let callback_source = include_str!("../src/presentation/telegram/handlers/callback.rs");

    assert!(callback_source.contains("recover_callback_ui"));
    assert!(callback_source.contains("restored_markup"));
    assert!(callback_source.contains("source_callback"));
}

#[test]
fn clippy_strictness_regressions_are_prevented() {
    let workflow = include_str!("../.github/workflows/rust-ci.yml");

    assert!(workflow.contains("cargo clippy --locked --all-targets --all-features -- -D warnings"));
}

#[test]
fn callback_ui_is_restored_and_database_failures_are_not_reported_as_success() {
    let callback_source = include_str!("../src/presentation/telegram/handlers/callback.rs");

    assert!(callback_source.contains("restore_callback_keyboard"));
    assert!(callback_source.contains("callback_error_message"));
    assert!(callback_source.contains("DatabaseError"));
}

#[test]
fn final_audit_followup_preserves_accurate_deletion_recovery() {
    let callback_source = include_str!("../src/presentation/telegram/handlers/callback.rs");

    assert!(callback_source.contains("delete_wallet"));
    assert!(callback_source.contains("remove_tracked_wallet"));
    assert!(callback_source.contains("recover_callback_ui"));
}

#[test]
fn final_scan_freshness_includes_nested_processing_failures() {
    let utxo_source = include_str!("../src/presentation/telegram/workers/utxo_monitor.rs");

    assert!(utxo_source.contains("wallet_task_succeeded"));
    assert!(utxo_source.contains("scan_succeeded"));
}

#[test]
fn operational_metrics_are_connected_to_runtime_paths() {
    let delivery_source = include_str!("../src/presentation/telegram/workers/telegram_delivery.rs");
    let queue_source = include_str!("../src/infrastructure/telegram_delivery_queue.rs");
    let health_source = include_str!("../src/infrastructure/webhook_security.rs");
    let metrics_source = include_str!("../src/infrastructure/metrics.rs");
    let observability_source = include_str!("../src/infrastructure/observability.rs");

    assert!(delivery_source.contains("DELIVERY_QUEUE_METRICS_INTERVAL_SECS"));
    assert!(delivery_source.contains("set_queue_snapshot"));
    assert!(delivery_source.contains("observe_delivery_latency"));
    assert!(queue_source.contains("oldest_active_age_seconds"));
    assert!(health_source.contains("ReadinessPolicy::from_env"));
    assert!(health_source.contains("StatusCode::SERVICE_UNAVAILABLE"));
    assert!(metrics_source.contains("render_prometheus"));
    assert!(observability_source.contains("subscription_runtime_restarts_total"));
}

#[test]
fn health_endpoint_is_started_in_polling_and_webhook_modes() {
    let main_source = include_str!("../src/main.rs");
    let health_call = "webhook_security::spawn_health_endpoint";

    assert_eq!(main_source.matches(health_call).count(), 1);
    let call_position = main_source.find(health_call).expect("health call missing");
    let webhook_branch_position = main_source
        .find("if startup.use_webhook")
        .expect("typed webhook branch missing");
    assert!(call_position < webhook_branch_position);
}

#[test]
fn timeout_metrics_and_deterministic_tokio_time_are_wired() {
    let cargo = include_str!("../Cargo.toml");
    let subscription = include_str!("../src/infrastructure/node/subscription.rs");
    let system = include_str!("../src/infrastructure/external_services/system.rs");
    let main_source = include_str!("../src/main.rs");

    let dev_dependencies = cargo
        .split("[dev-dependencies]")
        .nth(1)
        .expect("dev-dependencies section missing");
    assert!(dev_dependencies.contains("test-util"));
    let production_dependencies = cargo
        .split("[dev-dependencies]")
        .next()
        .expect("dependencies section missing");
    assert!(!production_dependencies.contains("test-util"));
    assert!(subscription.contains("tokio::time::advance"));
    assert!(system.contains("metrics::inc_rpc_timeouts"));
    assert!(main_source.contains("metrics::inc_rpc_timeouts"));
}

#[test]
fn rpc_outage_events_use_matching_sql_bind_parameters() {
    let events_repo = include_str!("../src/infrastructure/database/events_repo.rs");

    assert!(events_repo.contains("BotEventType::RpcOutage"));
    assert!(events_repo.contains("event_type.as_str()"));
}

#[test]
fn queue_stats_preserves_fail_closed_unknown_status_detection() {
    let queue_source = include_str!("../src/infrastructure/telegram_delivery_queue.rs");

    assert!(queue_source.contains("unknown_status_count"));
    assert!(queue_source.contains("unexpected delivery queue status"));
}

#[test]
fn readiness_configuration_is_fail_safe() {
    let health_source = include_str!("../src/infrastructure/webhook_security.rs");
    let observability_source = include_str!("../src/infrastructure/observability.rs");

    assert!(health_source.contains("ReadinessPolicy::from_env"));
    assert!(observability_source.contains("parse_bool_env"));
    assert!(observability_source.contains("Err("));
}

#[test]
fn subscription_runtime_is_wired_into_the_utxo_monitor() {
    let utxo_source = include_str!("../src/presentation/telegram/workers/utxo_monitor.rs");
    let subscription_source = include_str!("../src/infrastructure/node/subscription.rs");

    assert!(utxo_source.contains("NodeSubscriptionSupervisor"));
    assert!(subscription_source.contains("SubscriptionRuntimeState"));
}

#[test]
fn subscription_supervisor_restores_fallback_after_internal_failure() {
    let subscription_source = include_str!("../src/infrastructure/node/subscription.rs");

    assert!(subscription_source.contains("supervisor_restarts_failed_session_and_keeps_polling_fallback_active"));
}

#[test]
fn successful_scan_freshness_is_not_advanced_after_wallet_failures() {
    let utxo_source = include_str!("../src/presentation/telegram/workers/utxo_monitor.rs");

    assert!(utxo_source.contains("successful_scan_requires_every_wallet_task_to_succeed"));
}

#[test]
fn repository_contains_expected_runtime_files() {
    for path in [
        "src/infrastructure/observability.rs",
        "src/infrastructure/metrics.rs",
        "src/infrastructure/node/subscription.rs",
    ] {
        assert!(fs::metadata(path).is_ok(), "missing {path}");
    }
}
