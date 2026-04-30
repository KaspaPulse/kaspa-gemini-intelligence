use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::application::background_jobs::SystemTasksUseCase;

pub fn start_system_monitors(sys_tasks: Arc<SystemTasksUseCase>) {
    let sys_gc = sys_tasks.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            info!("[MEMORY CLEANER] Running scheduled cleanup.");
            sys_gc.execute_memory_cleanup().await;
        }
    });
}
