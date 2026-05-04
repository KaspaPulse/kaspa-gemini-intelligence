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

pub fn delivery_queue_enabled() -> bool {
    match std::env::var("ENABLE_TELEGRAM_DELIVERY_QUEUE") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "true" | "1" | "yes" | "on" | "enabled")
        }
        Err(_) => true,
    }
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
         (chat_id, message_html, status, wallet_masked, txid_masked, block_hash_masked, amount_kas, daa_score)
         VALUES ($1, $2, 'pending', $3, $4, $5, $6, $7)",
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

    let rows = sqlx::query(
        "SELECT
            id,
            chat_id,
            message_html,
            wallet_masked,
            txid_masked,
            block_hash_masked,
            amount_kas,
            daa_score
         FROM telegram_delivery_queue
         WHERE status = 'pending'
         AND attempts < 5
         ORDER BY created_at ASC
         LIMIT $1",
    )
    .bind(limit)
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
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: i64, error: &str) -> Result<(), AppError> {
    let safe_error = crate::utils::sanitize_event_text_for_storage(error);

    sqlx::query(
        "UPDATE telegram_delivery_queue
         SET status = CASE WHEN attempts + 1 >= 5 THEN 'failed' ELSE 'pending' END,
             attempts = attempts + 1,
             last_error = $2,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .bind(safe_error)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_queue_is_enabled_by_default() {
        std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
        assert!(delivery_queue_enabled());
    }

    #[test]
    fn delivery_queue_can_be_disabled_by_env() {
        std::env::set_var("ENABLE_TELEGRAM_DELIVERY_QUEUE", "false");
        assert!(!delivery_queue_enabled());
        std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
    }
}
