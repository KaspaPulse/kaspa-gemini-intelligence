use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use std::sync::Arc;
use tracing::{error, info};

pub struct SystemTasksUseCase {
    db: Arc<PostgresRepository>,
}

impl SystemTasksUseCase {
    pub fn new(db: Arc<PostgresRepository>) -> Self {
        Self { db }
    }

    pub async fn execute_memory_cleanup(&self) {
        let is_enabled = self
            .db
            .get_setting("ENABLE_MEMORY_CLEANER", "false")
            .await
            .unwrap_or_else(|_| "false".to_string());

        if is_enabled != "true" {
            return;
        }

        info!("[MEMORY CLEANER] Starting cleanup.");

        if let Err(e) = self.db.run_memory_cleaner().await {
            error!("[DATABASE ERROR] Failed to purge old chat rows: {}", e);
        } else {
            info!("[MEMORY CLEANER] Cleanup complete.");
        }
    }
}
