use kaspa_pulse::domain::errors::AppError;
use kaspa_pulse::infrastructure::telegram_delivery_queue::{
    commit_alert_outbox, AlertOutboxOutcome, AlertOutboxRequest,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};

static NEXT_ID: AtomicI64 = AtomicI64::new(9_300_000_000);

fn database_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

async fn test_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required for outbox tests");

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("runtime test PostgreSQL must be reachable")
}

async fn admin_pool() -> PgPool {
    let database_admin_url = std::env::var("DATABASE_ADMIN_URL")
        .expect("DATABASE_ADMIN_URL is required for outbox test setup");

    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_admin_url)
        .await
        .expect("administrative test PostgreSQL must be reachable")
}

async fn reset_tables(admin_pool: &PgPool) {
    sqlx::query(
        "DROP TRIGGER IF EXISTS stage3b2_reject_queue_insert_trigger
         ON telegram_delivery_queue",
    )
    .execute(admin_pool)
    .await
    .expect("failed to remove a stale queue failure trigger");

    sqlx::query("DROP FUNCTION IF EXISTS stage3b2_reject_queue_insert()")
        .execute(admin_pool)
        .await
        .expect("failed to remove a stale queue failure function");

    sqlx::query("TRUNCATE TABLE telegram_delivery_queue, wallet_alert_dedup, wallet_seen_utxos")
        .execute(admin_pool)
        .await
        .expect("failed to reset outbox tables");

    sqlx::query(
        "INSERT INTO system_settings (key_name, value_data)
         VALUES ('ENABLE_ALERT_DELIVERY', 'true')
         ON CONFLICT (key_name)
         DO UPDATE SET value_data = EXCLUDED.value_data, updated_at = NOW()",
    )
    .execute(admin_pool)
    .await
    .expect("failed to enable alert delivery");
}

fn request<'a>(
    wallet: &'a str,
    outpoint: &'a str,
    alert_key: &'a str,
    message: &'a str,
    chat_ids: &'a [i64],
) -> AlertOutboxRequest<'a> {
    AlertOutboxRequest {
        wallet,
        source_outpoint: outpoint,
        alert_key,
        message_html: message,
        chat_ids,
        wallet_masked: Some("kaspa:stage3b2...wallet"),
        txid_masked: Some("stage3b2...txid"),
        block_hash_masked: Some("stage3b2...block"),
        amount_kas: Some(2.59565436),
        daa_score: Some(474_800_104),
    }
}

#[tokio::test]
async fn commits_dedup_seen_and_recipient_queue_rows_atomically() {
    let _guard: MutexGuard<'static, ()> = database_test_lock().lock().await;
    let pool = test_pool().await;
    let privileged_pool = admin_pool().await;
    reset_tables(&privileged_pool).await;

    let id = next_id();
    let wallet = format!("kaspa:stage3b2-wallet-{id}");
    let outpoint = format!("stage3b2-outpoint-{id}:0");
    let alert_key = format!("stage3b2-alert-{id}");
    let chat_ids = [id, id + 1, id];

    let outcome = commit_alert_outbox(
        &pool,
        request(&wallet, &outpoint, &alert_key, "<b>stage3b2</b>", &chat_ids),
    )
    .await
    .expect("transactional outbox commit should succeed");

    assert_eq!(outcome, AlertOutboxOutcome::Enqueued { recipients: 2 });

    let dedup_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM wallet_alert_dedup WHERE wallet = $1 AND alert_key = $2",
    )
    .bind(&wallet)
    .bind(&alert_key)
    .fetch_one(&pool)
    .await
    .expect("dedup count should succeed");

    let seen_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM wallet_seen_utxos WHERE wallet = $1 AND outpoint = $2",
    )
    .bind(&wallet)
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("seen count should succeed");

    let queue_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM telegram_delivery_queue WHERE event_key = $1",
    )
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("queue count should succeed");

    assert_eq!(dedup_count, 1);
    assert_eq!(seen_count, 1);
    assert_eq!(queue_count, 2);
}

#[tokio::test]
async fn repeated_commit_does_not_duplicate_queue_rows() {
    let _guard: MutexGuard<'static, ()> = database_test_lock().lock().await;
    let pool = test_pool().await;
    let privileged_pool = admin_pool().await;
    reset_tables(&privileged_pool).await;

    let id = next_id();
    let wallet = format!("kaspa:stage3b2-wallet-{id}");
    let outpoint = format!("stage3b2-outpoint-{id}:0");
    let alert_key = format!("stage3b2-alert-{id}");
    let chat_ids = [id, id + 1];

    let first = commit_alert_outbox(
        &pool,
        request(&wallet, &outpoint, &alert_key, "<b>first</b>", &chat_ids),
    )
    .await
    .expect("first commit should succeed");
    assert_eq!(first, AlertOutboxOutcome::Enqueued { recipients: 2 });

    let second = commit_alert_outbox(
        &pool,
        request(&wallet, &outpoint, &alert_key, "<b>second</b>", &chat_ids),
    )
    .await
    .expect("repeated commit should be idempotent");
    assert_eq!(second, AlertOutboxOutcome::Duplicate);

    let queue_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM telegram_delivery_queue WHERE event_key = $1",
    )
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("queue count should succeed");

    assert_eq!(queue_count, 2);
}

#[tokio::test]
async fn queue_insert_failure_rolls_back_dedup_and_seen() {
    let _guard: MutexGuard<'static, ()> = database_test_lock().lock().await;
    let pool = test_pool().await;
    let privileged_pool = admin_pool().await;
    reset_tables(&privileged_pool).await;

    sqlx::query(
        "CREATE OR REPLACE FUNCTION stage3b2_reject_queue_insert()
         RETURNS trigger
         LANGUAGE plpgsql
         AS $$
         BEGIN
             RAISE EXCEPTION 'stage3b2 forced queue failure';
         END;
         $$",
    )
    .execute(&privileged_pool)
    .await
    .expect("failed to create queue failure function");

    sqlx::query(
        "CREATE TRIGGER stage3b2_reject_queue_insert_trigger
         BEFORE INSERT ON telegram_delivery_queue
         FOR EACH ROW
         EXECUTE FUNCTION stage3b2_reject_queue_insert()",
    )
    .execute(&privileged_pool)
    .await
    .expect("failed to create queue failure trigger");

    let id = next_id();
    let wallet = format!("kaspa:stage3b2-wallet-{id}");
    let outpoint = format!("stage3b2-outpoint-{id}:0");
    let alert_key = format!("stage3b2-alert-{id}");
    let chat_ids = [id];

    let result = commit_alert_outbox(
        &pool,
        request(
            &wallet,
            &outpoint,
            &alert_key,
            "<b>forced failure</b>",
            &chat_ids,
        ),
    )
    .await;

    sqlx::query(
        "DROP TRIGGER IF EXISTS stage3b2_reject_queue_insert_trigger
         ON telegram_delivery_queue",
    )
    .execute(&privileged_pool)
    .await
    .expect("failed to drop queue failure trigger");

    sqlx::query("DROP FUNCTION IF EXISTS stage3b2_reject_queue_insert()")
        .execute(&privileged_pool)
        .await
        .expect("failed to drop queue failure function");

    assert!(matches!(result, Err(AppError::DatabaseError(_))));

    let dedup_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM wallet_alert_dedup WHERE wallet = $1 AND alert_key = $2",
    )
    .bind(&wallet)
    .bind(&alert_key)
    .fetch_one(&pool)
    .await
    .expect("dedup count should succeed");

    let seen_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM wallet_seen_utxos WHERE wallet = $1 AND outpoint = $2",
    )
    .bind(&wallet)
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("seen count should succeed");

    assert_eq!(dedup_count, 0);
    assert_eq!(seen_count, 0);
}

#[tokio::test]
async fn existing_dedup_without_queue_is_reconciled() {
    let _guard: MutexGuard<'static, ()> = database_test_lock().lock().await;
    let pool = test_pool().await;
    let privileged_pool = admin_pool().await;
    reset_tables(&privileged_pool).await;

    let id = next_id();
    let wallet = format!("kaspa:stage3b2-wallet-{id}");
    let outpoint = format!("stage3b2-outpoint-{id}:0");
    let alert_key = format!("stage3b2-alert-{id}");
    let chat_ids = [id, id + 1];

    sqlx::query("INSERT INTO wallet_alert_dedup (wallet, alert_key) VALUES ($1, $2)")
        .bind(&wallet)
        .bind(&alert_key)
        .execute(&privileged_pool)
        .await
        .expect("dedup fixture should insert");

    let outcome = commit_alert_outbox(
        &pool,
        request(
            &wallet,
            &outpoint,
            &alert_key,
            "<b>reconcile</b>",
            &chat_ids,
        ),
    )
    .await
    .expect("reconciliation should succeed");

    assert_eq!(outcome, AlertOutboxOutcome::Reconciled { recipients: 2 });

    let row = sqlx::query(
        "SELECT COUNT(*)::BIGINT AS queue_count,
                COUNT(DISTINCT chat_id)::BIGINT AS recipient_count
         FROM telegram_delivery_queue
         WHERE event_key = $1",
    )
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("reconciled queue query should succeed");

    assert_eq!(row.try_get::<i64, _>("queue_count").unwrap(), 2);
    assert_eq!(row.try_get::<i64, _>("recipient_count").unwrap(), 2);
}

#[tokio::test]
async fn disabled_delivery_commits_dedup_and_seen_without_queue_rows() {
    let _guard: MutexGuard<'static, ()> = database_test_lock().lock().await;
    let pool = test_pool().await;
    let privileged_pool = admin_pool().await;
    reset_tables(&privileged_pool).await;

    sqlx::query(
        "UPDATE system_settings
         SET value_data = 'false', updated_at = NOW()
         WHERE key_name = 'ENABLE_ALERT_DELIVERY'",
    )
    .execute(&privileged_pool)
    .await
    .expect("failed to disable alert delivery");

    let id = next_id();
    let wallet = format!("kaspa:stage3b2-wallet-{id}");
    let outpoint = format!("stage3b2-outpoint-{id}:0");
    let alert_key = format!("stage3b2-alert-{id}");
    let chat_ids = [id, id + 1];

    let outcome = commit_alert_outbox(
        &pool,
        request(
            &wallet,
            &outpoint,
            &alert_key,
            "<b>suppressed</b>",
            &chat_ids,
        ),
    )
    .await
    .expect("suppressed outbox commit should succeed");

    assert_eq!(outcome, AlertOutboxOutcome::Suppressed { recipients: 2 });

    let queue_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM telegram_delivery_queue WHERE event_key = $1",
    )
    .bind(&outpoint)
    .fetch_one(&pool)
    .await
    .expect("queue count should succeed");

    assert_eq!(queue_count, 0);
}

#[tokio::test]
async fn alert_delivery_setting_lookup_fails_closed_during_database_outage() {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(250))
        .connect_lazy("postgres://stage3b2@127.0.0.1:1/stage3b2?sslmode=disable")
        .expect("invalid-endpoint pool URL should parse");

    let result = kaspa_pulse::wallet::alert_delivery_gate::is_alert_delivery_enabled(&pool).await;

    assert!(matches!(result, Err(AppError::DatabaseError(_))));
}

#[test]
fn monitor_no_longer_falls_back_to_direct_send_after_queue_failure() {
    let source = include_str!("../src/presentation/telegram/workers/utxo_monitor.rs");

    assert!(source.contains("commit_alert_outbox"));
    assert!(source.contains("retry_next_scan"));
    assert!(!source.contains("BOT OUT FALLBACK"));
    assert!(!source.contains("Falling back to direct send"));
}

#[test]
fn wallet_processing_defers_dedup_and_seen_to_the_transactional_outbox() {
    let source = include_str!("../src/wallet/wallet_use_cases.rs");

    assert!(!source.contains("try_claim_alert_key("));
    assert!(source.contains("source_outpoint: utxo.outpoint"));
    assert!(source.contains("alert_key,"));
    assert!(!source.contains("processed_reward_seen_upsert_failed"));
}
