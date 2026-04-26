use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::application::background_jobs::CrawlNewsUseCase;
use crate::application::background_jobs::SystemTasksUseCase;

pub fn start_system_monitors(sys_tasks: Arc<SystemTasksUseCase>) {
    // 1. Memory Cleaner (Runs every 1 hour)
    let sys_gc = sys_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            info!("[MEMORY CLEANER] Purging UTXO cache, inactive rate limits...");
            sys_gc.execute_memory_cleanup().await;
        }
    });

    // 2. AI Vectorizer (Runs every 10 seconds)
    let sys_ai = sys_tasks.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            sys_ai.execute_ai_vectorizer().await;
        }
    });
}

pub fn start_rss_crawler(rss_use_case: Arc<CrawlNewsUseCase>) {
    tokio::spawn(async move {
        loop {
            // STRICT BOOT CHECK: Check the database BEFORE taking any action or logging
            let is_enabled = rss_use_case.db.get_setting("ENABLE_RSS_WORKER", "false").await.unwrap_or_else(|_| "false".to_string());
            
            if is_enabled != "true" {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }

            tracing::info!("📡 [RSS CRAWLER] Worker activated. Fetching news items...");
            rss_use_case.execute().await;
            tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await; // Run every 6 hours
        }
    });
}


