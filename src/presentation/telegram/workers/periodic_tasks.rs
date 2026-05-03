use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::application::background_jobs::SystemTasksUseCase;

pub fn start_system_monitors(sys_tasks: Arc<SystemTasksUseCase>, token: CancellationToken) {
    let sys_gc = sys_tasks.clone();

    crate::infrastructure::resilience::runtime::spawn_resilient(
        "periodic_system_task",
        async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        info!("[MEMORY CLEANER] Scheduled cleanup worker shutdown requested.");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                        info!("[MEMORY CLEANER] Running scheduled cleanup.");
                        sys_gc.execute_memory_cleanup().await;
                    }
                }
            }
        },
    );
}
