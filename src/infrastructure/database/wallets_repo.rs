use std::collections::HashSet;

use crate::domain::entities::TrackedWallet;
use crate::domain::errors::AppError;

use super::postgres_adapter::PostgresRepository;

impl PostgresRepository {
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

    pub async fn get_seen_utxos(&self, wallet: &str) -> Result<HashSet<String>, AppError> {
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
