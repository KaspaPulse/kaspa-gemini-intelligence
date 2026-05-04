use crate::domain::errors::AppError;
use sqlx::PgPool;

#[allow(dead_code)]
pub async fn enqueue_message(
    pool: &PgPool,
    chat_id: i64,
    message_html: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO telegram_delivery_queue (chat_id, message_html, status)
         VALUES ($1, $2, 'pending')",
    )
    .bind(chat_id)
    .bind(message_html)
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
