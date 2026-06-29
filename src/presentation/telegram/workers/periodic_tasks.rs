use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::application::background_jobs::SystemTasksUseCase;

pub fn start_system_monitors(sys_tasks: Arc<SystemTasksUseCase>, token: CancellationToken) {
    let sys_gc = sys_tasks.clone();
    let sys_price = sys_tasks.clone();
    let price_token = token.clone();

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

    crate::infrastructure::resilience::runtime::spawn_resilient(
        "periodic_kas_price_sync",
        async move {
            sys_price.execute_kas_price_sync().await;

            loop {
                tokio::select! {
                    _ = price_token.cancelled() => {
                        info!("[KAS PRICE] Scheduled price sync worker shutdown requested.");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(env_u64(
                        "KAS_PRICE_REFRESH_INTERVAL_SECS",
                        3600,
                    ))) => {
                        info!("[KAS PRICE] Running scheduled stored KAS/USD price sync.");
                        sys_price.execute_kas_price_sync().await;
                    }
                }
            }
        },
    );
}

fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default_value)
}
