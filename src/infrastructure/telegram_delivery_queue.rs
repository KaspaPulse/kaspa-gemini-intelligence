use crate::domain::errors::AppError;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct QueuedTelegramMessage {
    pub id: i64,
    pub chat_id: i64,
    pub message_html: String,
    pub wallet_masked: Option<String>,
    pub txid_masked: Option<String>,
    pub block_hash_masked: Option<String>,
    pub amount_kas: Option<f64>,
    pub daa_score: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct DeliveryQueueStats {
    pub pending: i64,
    pub processing: i64,
    pub sent: i64,
    pub failed: i64,
    pub suppressed: i64,
}

pub fn delivery_queue_enabled() -> bool {
    match std::env::var("ENABLE_TELEGRAM_DELIVERY_QUEUE") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "true" | "1" | "yes" | "on" | "enabled")
        }
        Err(_) => true,
    }
}

pub fn worker_id() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string());

    format!("{}:{}", host, std::process::id())
}

#[allow(dead_code)]
pub async fn enqueue_message(
    pool: &PgPool,
    chat_id: i64,
    message_html: &str,
) -> Result<(), AppError> {
    enqueue_alert_message(pool, chat_id, message_html, None, None, None, None, None).await
}

#[allow(clippy::too_many_arguments)]
pub async fn enqueue_alert_message(
    pool: &PgPool,
    chat_id: i64,
    message_html: &str,
    wallet_masked: Option<&str>,
    txid_masked: Option<&str>,
    block_hash_masked: Option<&str>,
    amount_kas: Option<f64>,
    daa_score: Option<i64>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO telegram_delivery_queue
         (chat_id, message_html, status, wallet_masked, txid_masked, block_hash_masked, amount_kas, daa_score, next_attempt_at)
         VALUES ($1, $2, 'pending', $3, $4, $5, $6, $7, NOW())",
    )
    .bind(chat_id)
    .bind(message_html)
    .bind(wallet_masked)
    .bind(txid_masked)
    .bind(block_hash_masked)
    .bind(amount_kas)
    .bind(daa_score)
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub async fn fetch_pending_batch(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<QueuedTelegramMessage>, AppError> {
    let limit = limit.clamp(1, 100);
    let locked_by = worker_id();

    let rows = sqlx::query(
        "WITH picked AS (
            SELECT id
            FROM telegram_delivery_queue
            WHERE
                (
                    status = 'pending'
                    OR (
                        status = 'processing'
                        AND locked_at < NOW() - INTERVAL '120 seconds'
                    )
                )
                AND attempts < 5
                AND next_attempt_at <= NOW()
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $1
         )
         UPDATE telegram_delivery_queue q
         SET status = 'processing',
             locked_at = NOW(),
             locked_by = $2,
             updated_at = NOW()
         FROM picked
         WHERE q.id = picked.id
         RETURNING
            q.id,
            q.chat_id,
            q.message_html,
            q.wallet_masked,
            q.txid_masked,
            q.block_hash_masked,
            q.amount_kas,
            q.daa_score",
    )
    .bind(limit)
    .bind(locked_by)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut messages = Vec::with_capacity(rows.len());

    for row in rows {
        messages.push(QueuedTelegramMessage {
            id: row.try_get::<i64, _>("id").unwrap_or_default(),
            chat_id: row.try_get::<i64, _>("chat_id").unwrap_or_default(),
            message_html: row.try_get::<String, _>("message_html").unwrap_or_default(),
            wallet_masked: row
                .try_get::<Option<String>, _>("wallet_masked")
                .ok()
                .flatten(),
            txid_masked: row
                .try_get::<Option<String>, _>("txid_masked")
                .ok()
                .flatten(),
            block_hash_masked: row
                .try_get::<Option<String>, _>("block_hash_masked")
                .ok()
                .flatten(),
            amount_kas: row.try_get::<Option<f64>, _>("amount_kas").ok().flatten(),
            daa_score: row.try_get::<Option<i64>, _>("daa_score").ok().flatten(),
        });
    }

    Ok(messages)
}

pub async fn mark_sent(pool: &PgPool, id: i64) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE telegram_delivery_queue
         SET status = 'sent',
             attempts = attempts + 1,
             locked_at = NULL,
             locked_by = NULL,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn retry_after_seconds(error: &str) -> Option<i64> {
    let lower = error.to_ascii_lowercase();

    if !lower.contains("retry_after") && !lower.contains("too many requests") {
        return None;
    }

    lower
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<i64>().ok())
        .find(|value| *value > 0 && *value <= 3600)
}

pub fn retry_delay_seconds(attempts_before_increment: i32, error: &str) -> i64 {
    if let Some(retry_after) = retry_after_seconds(error) {
        return retry_after.clamp(1, 3600);
    }

    match attempts_before_increment {
        0 => 5,
        1 => 15,
        2 => 60,
        3 => 300,
        _ => 900,
    }
}

pub async fn mark_failed(pool: &PgPool, id: i64, error: &str) -> Result<(), AppError> {
    let safe_error = crate::utils::sanitize_event_text_for_storage(error);

    let attempts: i32 =
        sqlx::query_scalar("SELECT attempts FROM telegram_delivery_queue WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .unwrap_or(0);

    let delay = retry_delay_seconds(attempts, error);

    sqlx::query(
        "UPDATE telegram_delivery_queue
         SET status = CASE WHEN attempts + 1 >= 5 THEN 'failed' ELSE 'pending' END,
             attempts = attempts + 1,
             last_error = $2,
             locked_at = NULL,
             locked_by = NULL,
             next_attempt_at = NOW() + ($3::TEXT || ' seconds')::INTERVAL,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .bind(safe_error)
    .bind(delay)
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

#[allow(dead_code)]
pub async fn pending_count(pool: &PgPool) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM telegram_delivery_queue WHERE status = 'pending'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))
}

pub async fn queue_stats(pool: &PgPool) -> Result<DeliveryQueueStats, AppError> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*)::BIGINT AS count
         FROM telegram_delivery_queue
         GROUP BY status",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut stats = DeliveryQueueStats::default();

    for row in rows {
        let status = row.try_get::<String, _>("status").unwrap_or_default();
        let count = row.try_get::<i64, _>("count").unwrap_or_default();

        match status.as_str() {
            "pending" => stats.pending = count,
            "processing" => stats.processing = count,
            "sent" => stats.sent = count,
            "failed" => stats.failed = count,
            "suppressed" => stats.suppressed = count,
            _ => {}
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn delivery_queue_is_enabled_by_default() {
        let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");
        std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
        assert!(delivery_queue_enabled());
    }

    #[test]
    fn delivery_queue_can_be_disabled_by_env() {
        let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");
        std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
        std::env::set_var("ENABLE_TELEGRAM_DELIVERY_QUEUE", "false");
        assert!(!delivery_queue_enabled());
        std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
    }

    #[test]
    fn retry_after_is_extracted_from_error_text() {
        assert_eq!(
            retry_after_seconds("Too Many Requests: retry_after 17"),
            Some(17)
        );
        assert_eq!(retry_after_seconds("normal error"), None);
    }

    #[test]
    fn retry_delay_uses_backoff() {
        assert_eq!(retry_delay_seconds(0, "network error"), 5);
        assert_eq!(retry_delay_seconds(2, "network error"), 60);
    }
}
