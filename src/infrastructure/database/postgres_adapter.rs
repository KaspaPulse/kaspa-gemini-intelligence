use crate::domain::entities::{MinedBlock, TrackedWallet};
use crate::domain::errors::AppError;
use sqlx::postgres::PgPool;

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn count_user_wallets(&self, chat_id: i64) -> Result<i64, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*)
             FROM user_wallets
             WHERE chat_id = $1",
            chat_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn user_wallet_exists(&self, address: &str, chat_id: i64) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(
                SELECT 1
                FROM user_wallets
                WHERE wallet = $1
                AND chat_id = $2
             )",
            address,
            chat_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(false);

        Ok(exists)
    }
    pub async fn add_tracked_wallet(&self, wallet: TrackedWallet) -> Result<(), AppError> {
        let already_exists = self
            .user_wallet_exists(&wallet.address, wallet.chat_id)
            .await
            .unwrap_or(false);

        if !already_exists {
            let current_count = self.count_user_wallets(wallet.chat_id).await.unwrap_or(0);
            let max_wallets = crate::utils::max_wallets_per_user();

            if current_count >= max_wallets {
                return Err(AppError::Internal(format!(
                    "MAX_WALLETS_PER_USER limit reached. Current limit: {} wallets.",
                    max_wallets
                )));
            }
        }
        sqlx::query!(
            "INSERT INTO user_wallets (wallet, chat_id)
             VALUES ($1, $2)
             ON CONFLICT (wallet, chat_id)
             DO UPDATE SET last_active = CURRENT_TIMESTAMP",
            wallet.address,
            wallet.chat_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_tracked_wallet(&self, address: &str, chat_id: i64) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM user_wallets
             WHERE wallet = $1 AND chat_id = $2",
            address,
            chat_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_all_user_wallets(&self, chat_id: i64) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM user_wallets
             WHERE chat_id = $1",
            chat_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_all_user_data(&self, chat_id: i64) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM user_wallets
             WHERE chat_id = $1",
            chat_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_all_tracked_wallets(&self) -> Result<Vec<TrackedWallet>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT wallet, chat_id as "chat_id!"
            FROM user_wallets
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let wallets = rows
            .into_iter()
            .map(|row| TrackedWallet {
                address: row.wallet,
                chat_id: row.chat_id,
            })
            .collect();

        Ok(wallets)
    }

    pub async fn record_mined_block(&self, block: MinedBlock) -> Result<(), AppError> {
        sqlx::query!(
            "INSERT INTO mined_blocks (wallet, outpoint, amount, daa_score)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (outpoint) DO NOTHING",
            block.wallet_address,
            block.outpoint,
            block.amount,
            block.daa_score as i64
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_lifetime_stats(&self, address: &str) -> Result<(i64, i64), AppError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "count!",
                (COALESCE(SUM(amount), 0))::BIGINT as "sum!"
            FROM mined_blocks
            WHERE wallet = $1
            "#,
            address
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((row.count, row.sum))
    }

    pub async fn get_daily_blocks(&self, address: &str) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT
                TO_CHAR(timestamp, 'YYYY-MM-DD') as day,
                COUNT(*) as count
             FROM mined_blocks
             WHERE wallet = $1
             GROUP BY day
             ORDER BY day DESC
             LIMIT 7",
        )
        .bind(address)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(rows)
    }

    pub async fn get_blocks_count_1h(&self, address: &str) -> Result<i64, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*)
             FROM mined_blocks
             WHERE wallet = $1
             AND timestamp >= CURRENT_TIMESTAMP - INTERVAL '1 hour'",
            address
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn get_blocks_count_24h(&self, address: &str) -> Result<i64, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*)
             FROM mined_blocks
             WHERE wallet = $1
             AND timestamp >= CURRENT_TIMESTAMP - INTERVAL '24 hours'",
            address
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn get_blocks_count_7d(&self, address: &str) -> Result<i64, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*)
             FROM mined_blocks
             WHERE wallet = $1
             AND timestamp >= CURRENT_TIMESTAMP - INTERVAL '7 days'",
            address
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn get_setting(&self, key: &str, default_val: &str) -> Result<String, AppError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value_data FROM system_settings WHERE key_name = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(value) = value {
            return Ok(value);
        }

        sqlx::query(
            "INSERT INTO system_settings (key_name, value_data)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(key)
        .bind(default_val)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(default_val.to_string())
    }

    pub async fn update_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO system_settings (key_name, value_data)
             VALUES ($1, $2)
             ON CONFLICT (key_name)
             DO UPDATE SET
                value_data = EXCLUDED.value_data,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    pub async fn get_seen_utxos(
        &self,
        wallet: &str,
    ) -> Result<std::collections::HashSet<String>, AppError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT outpoint FROM wallet_seen_utxos WHERE wallet = $1")
                .bind(wallet)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.0).collect())
    }

    pub async fn upsert_seen_utxos(
        &self,
        wallet: &str,
        outpoints: &[String],
    ) -> Result<(), AppError> {
        for outpoint in outpoints {
            sqlx::query(
                "INSERT INTO wallet_seen_utxos (wallet, outpoint)
                 VALUES ($1, $2)
                 ON CONFLICT (wallet, outpoint)
                 DO UPDATE SET last_seen_at = CURRENT_TIMESTAMP",
            )
            .bind(wallet)
            .bind(outpoint)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn prune_seen_utxos(
        &self,
        wallet: &str,
        current_outpoints: &[String],
    ) -> Result<(), AppError> {
        if current_outpoints.is_empty() {
            return Ok(());
        }

        sqlx::query(
            "DELETE FROM wallet_seen_utxos
             WHERE wallet = $1
             AND NOT (outpoint = ANY($2))",
        )
        .bind(wallet)
        .bind(current_outpoints)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn try_claim_alert_key(
        &self,
        wallet: &str,
        alert_key: &str,
        txid_masked: Option<&str>,
        block_hash_masked: Option<&str>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT INTO wallet_alert_dedup (wallet, alert_key, txid_masked, block_hash_masked)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (wallet, alert_key) DO NOTHING",
        )
        .bind(wallet)
        .bind(alert_key)
        .bind(txid_masked)
        .bind(block_hash_masked)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() > 0)
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
    pub async fn record_bot_event_typed(
        &self,
        event_type: crate::domain::models::BotEventType,
        severity: crate::domain::models::EventSeverity,
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

    pub async fn run_memory_cleaner(&self) -> Result<(), AppError> {
        Ok(())
    }
}
