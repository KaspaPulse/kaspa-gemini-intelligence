use kaspa_pulse::domain::errors::AppError;
use kaspa_pulse::infrastructure::telegram_delivery_queue::{
    mark_failed, mark_sent, max_delivery_attempts,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use std::time::Duration;

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

async fn test_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required for database tests");

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("test PostgreSQL must be reachable")
}

#[test]
fn wallet_repository_does_not_hide_lookup_or_quota_errors() {
    let source = include_str!("../src/infrastructure/database/wallets_repo.rs");

    assert!(!source.contains(".unwrap_or(false)"));
    assert!(!source.contains("count_user_wallets(wallet.chat_id).await.unwrap_or(0)"));
    assert!(source.contains("let already_exists = self"));
    assert!(source.contains("count_user_wallets(wallet.chat_id).await?"));
}

#[test]
fn transactional_outbox_failure_is_not_treated_as_permission_to_send() {
    let wallet_source = include_str!("../src/wallet/wallet_use_cases.rs");
    let monitor_source = include_str!("../src/presentation/telegram/workers/utxo_monitor.rs");

    assert!(!wallet_source.contains("try_claim_alert_key("));
    assert!(monitor_source.contains("commit_alert_outbox"));
    assert!(monitor_source.contains("alert_outbox_commit_failed"));
    assert!(monitor_source.contains("retry_next_scan"));
    assert!(!monitor_source.contains("BOT OUT FALLBACK"));
    assert!(!monitor_source.contains("Falling back to direct send"));
}

#[test]
fn confirmed_rewards_defer_seen_persistence_to_the_transactional_outbox() {
    let wallet_source = include_str!("../src/wallet/wallet_use_cases.rs");
    let queue_source = include_str!("../src/infrastructure/telegram_delivery_queue.rs");

    assert!(!wallet_source.contains("if reward_is_confirmed || seen_before || is_first_run"));
    assert!(wallet_source.contains("if seen_before || is_first_run"));
    assert!(wallet_source.contains("source_outpoint: utxo.outpoint"));
    assert!(queue_source.contains("INSERT INTO wallet_seen_utxos"));
    assert!(queue_source.contains("execute(&mut *transaction)"));
}

#[test]
fn delivery_queue_decoding_does_not_fall_back_to_default_values() {
    let source = include_str!("../src/infrastructure/telegram_delivery_queue.rs");

    assert!(source.contains("query_as::<_, QueuedTelegramMessage>"));
    assert!(!source.contains("try_get::<i64, _>(\"id\").unwrap_or_default()"));
    assert!(!source.contains("try_get::<String, _>(\"status\").unwrap_or_default()"));
    assert!(source.contains("Unexpected Telegram delivery queue status"));
}

#[test]
fn delivery_max_attempts_has_a_safe_default_and_bounds() {
    let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");

    std::env::remove_var("TELEGRAM_DELIVERY_MAX_ATTEMPTS");
    assert_eq!(max_delivery_attempts(), 5);

    std::env::set_var("TELEGRAM_DELIVERY_MAX_ATTEMPTS", "0");
    assert_eq!(max_delivery_attempts(), 5);

    std::env::set_var("TELEGRAM_DELIVERY_MAX_ATTEMPTS", "7");
    assert_eq!(max_delivery_attempts(), 7);

    std::env::set_var("TELEGRAM_DELIVERY_MAX_ATTEMPTS", "1000");
    assert_eq!(max_delivery_attempts(), 100);

    std::env::remove_var("TELEGRAM_DELIVERY_MAX_ATTEMPTS");
}

#[tokio::test]
async fn mark_sent_returns_not_found_for_a_missing_queue_row() {
    let pool = test_pool().await;
    let missing_id = i64::MAX - 11;

    let result = mark_sent(&pool, missing_id).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn mark_failed_returns_not_found_for_a_missing_queue_row() {
    let pool = test_pool().await;
    let missing_id = i64::MAX - 12;

    let result = mark_failed(&pool, missing_id, "stage3 batch1 missing row").await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}
