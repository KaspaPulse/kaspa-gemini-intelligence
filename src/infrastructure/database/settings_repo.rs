use crate::domain::errors::AppError;

use super::postgres_adapter::PostgresRepository;

impl PostgresRepository {
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
        let deleted_events = self.purge_old_bot_events(60).await?;

        tracing::info!(
            "[MEMORY CLEANER] Old bot events cleanup complete. Deleted rows: {}",
            deleted_events
        );

        Ok(())
    }
}
