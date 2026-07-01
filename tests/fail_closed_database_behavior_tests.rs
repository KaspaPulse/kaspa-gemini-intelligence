use kaspa_pulse::domain::errors::AppError;
use kaspa_pulse::infrastructure::telegram_delivery_queue::{
    mark_failed, mark_sent, max_delivery_attempts,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
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
fn alert_dedup_failure_is_not_treated_as_permission_to_send() {
    let source = include_str!("../src/wallet/wallet_use_cases.rs");

    assert!(!source.contains(".await\n                    .unwrap_or(true)"));
    assert!(source.contains("alert_dedup_claim_failed"));
    assert!(source.contains("retry_next_scan"));
}

#[test]
fn unseen_confirmed_rewards_are_marked_seen_only_after_processing() {
    let source = include_str!("../src/wallet/wallet_use_cases.rs");

    assert!(!source.contains("if reward_is_confirmed || seen_before || is_first_run"));
    assert!(source.contains("if seen_before || is_first_run"));
    assert!(
        source
            .matches("std::slice::from_ref(&utxo.outpoint)")
            .count()
            >= 2
    );
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
