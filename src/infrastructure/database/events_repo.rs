use crate::domain::errors::AppError;
use crate::domain::models::{BotEventType, EventSeverity};

use super::postgres_adapter::PostgresRepository;

impl PostgresRepository {
    pub async fn record_bot_event_typed(
        &self,
        event_type: BotEventType,
        severity: EventSeverity,
        chat_id: Option<i64>,
        user_name: Option<&str>,
        command: Option<&str>,
        callback_data: Option<&str>,
        wallet_masked: Option<&str>,
        txid_masked: Option<&str>,
        block_hash_masked: Option<&str>,
        status: Option<&str>,
        error_message: Option<&str>,
        duration_ms: Option<i64>,
        metadata_json: &str,
    ) -> Result<(), AppError> {
        self.record_bot_event(
            event_type.as_str(),
            severity.as_str(),
            chat_id,
            user_name,
            command,
            callback_data,
            wallet_masked,
            txid_masked,
            block_hash_masked,
            status,
            error_message,
            duration_ms,
            metadata_json,
        )
        .await
    }

    pub async fn record_bot_event(
        &self,
        event_type: &str,
        severity: &str,
        chat_id: Option<i64>,
        user_name: Option<&str>,
        command: Option<&str>,
        callback_data: Option<&str>,
        wallet_masked: Option<&str>,
        txid_masked: Option<&str>,
        block_hash_masked: Option<&str>,
        status: Option<&str>,
        error_message: Option<&str>,
        duration_ms: Option<i64>,
        metadata_json: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO bot_event_log (
                event_type,
                severity,
                chat_id,
                user_name,
                command,
                callback_data,
                wallet_masked,
                txid_masked,
                block_hash_masked,
                status,
                error_message,
                duration_ms,
                metadata
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, COALESCE($13::jsonb, '{}'::jsonb)
            )
            "#,
        )
        .bind(event_type)
        .bind(severity)
        .bind(chat_id)
        .bind(user_name)
        .bind(command)
        .bind(callback_data)
        .bind(wallet_masked)
        .bind(txid_masked)
        .bind(block_hash_masked)
        .bind(status)
        .bind(error_message)
        .bind(duration_ms)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn purge_old_bot_events(&self, days: i64) -> Result<u64, AppError> {
        let days = days.clamp(1, 365);
        let result = sqlx::query(
            "DELETE FROM bot_event_log
             WHERE created_at < NOW() - ($1::text || ' days')::interval",
        )
        .bind(days.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    #[allow(dead_code)]
    pub async fn get_delivery_summary_24h(&self) -> Result<(i64, i64, i64), AppError> {
        let detected: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bot_event_log
             WHERE event_type = 'ALERT_DETECTED'
             AND created_at >= NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let delivered: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bot_event_log
             WHERE event_type = 'ALERT_DELIVERED'
             AND created_at >= NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let failed: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bot_event_log
             WHERE event_type = 'ALERT_DELIVERY_FAILED'
             AND created_at >= NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((detected, delivered, failed))
    }

    #[allow(dead_code)]
    pub async fn get_subscribers_for_wallet(&self, wallet: &str) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT chat_id FROM user_wallets WHERE wallet = $1 ORDER BY chat_id")
                .bind(wallet)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.0).collect())
    }
}
