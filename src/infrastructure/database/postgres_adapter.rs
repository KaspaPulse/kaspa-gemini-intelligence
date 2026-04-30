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
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(0);

        Ok(count)
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

        let _ = sqlx::query!(
            "DELETE FROM chat_history
             WHERE chat_id = $1",
            chat_id
        )
        .execute(&self.pool)
        .await;

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
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(0);

        Ok(count)
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
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(0);

        Ok(count)
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
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(0);

        Ok(count)
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

    pub async fn run_memory_cleaner(&self) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM chat_history
             WHERE timestamp < CURRENT_TIMESTAMP - INTERVAL '30 days'"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
