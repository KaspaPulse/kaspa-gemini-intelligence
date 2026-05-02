use crate::domain::models::{BotEventType, EventSeverity};
use kaspa_rpc_core::api::rpc::RpcApi;
use std::sync::atomic::Ordering;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::domain::models::AppContext;

pub fn spawn_price_monitor(ctx: AppContext, token: CancellationToken) {
    tokio::spawn(async move {
        let client = build_http_client();

        // Fetch instantly on boot
        let mut p = 0.0;
        let mut m = 0.0;
        if let Ok(r) = client.get("https://api.kaspa.org/info/price").send().await {
            if let Ok(j) = r.json::<serde_json::Value>().await {
                p = j["price"].as_f64().unwrap_or(0.0);
            }
        }
        if let Ok(r) = client
            .get("https://api.kaspa.org/info/marketcap")
            .send()
            .await
        {
            if let Ok(j) = r.json::<serde_json::Value>().await {
                m = j["marketcap"].as_f64().unwrap_or(0.0);
            }
        }
        if p > 0.0 {
            let mut write_guard = ctx.price_cache.write().await;
            *write_guard = (p, m);
        }

        loop {
            tokio::select! {
                _ = token.cancelled() => { break; }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                    let mut p = 0.0;
                    let mut m = 0.0;
                    if let Ok(r) = client.get("https://api.kaspa.org/info/price").send().await {
                        if let Ok(j) = r.json::<serde_json::Value>().await { p = j["price"].as_f64().unwrap_or(0.0); }
                    }
                    if let Ok(r) = client.get("https://api.kaspa.org/info/marketcap").send().await {
                        if let Ok(j) = r.json::<serde_json::Value>().await { m = j["marketcap"].as_f64().unwrap_or(0.0); }
                    }
                    if p > 0.0 {
                        let mut write_guard = ctx.price_cache.write().await;
                        *write_guard = (p, m);
                    }
                }
            }
        }
    });
}

pub fn spawn_node_monitor(ctx: AppContext, bot: Bot, token: CancellationToken) {
    tokio::spawn(async move {
        let mut failed_attempts = 0;
        let mut is_disconnected = false;
        let _ = ctx.rpc.connect(None).await;

        tokio::time::sleep(Duration::from_secs(10)).await;

        loop {
            tokio::select! {
                _ = token.cancelled() => { break; }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    if ctx.rpc.get_server_info().await.is_err() {
                        failed_attempts += 1;
                        tracing::error!("[NODE ALERT] RPC Connection Lost! Attempt {}...", failed_attempts);
                        let _ = sqlx::query(
                            "INSERT INTO bot_event_log (event_type, severity, status, error_message, metadata)
                             VALUES ('RPC_ERROR', 'error', 'node_unreachable', 'RPC connection lost', $1::jsonb)"
                        )
                        .bind(BotEventType::RpcError.as_str())
                        .bind(EventSeverity::Error.as_str())
                        .bind(format!(r#"{{"attempt":{}}}"#, failed_attempts))
                        .execute(&ctx.pool)
                        .await;

                        if failed_attempts == 1 {
                            is_disconnected = true;
                            // Safe sleep mode
                            ctx.live_sync_enabled.store(false, Ordering::Relaxed);
                            if let Err(e) = bot.send_message(ChatId(ctx.admin_id), "⚠️ <b>WARNING:</b> Primary Node connection dropped!\n⏸️ UTXO Monitoring paused safely.\n🔄 Attempting background recovery...")
                                .parse_mode(teloxide::types::ParseMode::Html).await { tracing::error!("[TELEGRAM ERROR] Bot API request failed: {}", e); }
                        }

                        if failed_attempts % 10 == 0 {
                            if let Err(e) = bot.send_message(ChatId(ctx.admin_id), format!("🚨 <b>CRITICAL:</b> Node still unreachable after {} attempts. Continuing to retry quietly...", failed_attempts))
                                .parse_mode(teloxide::types::ParseMode::Html).await { tracing::error!("[TELEGRAM ERROR] Bot API request failed: {}", e); }
                        }

                        let _ = ctx.rpc.connect(None).await;
                    } else {
                        if is_disconnected {
                            tracing::info!("[NODE RECOVERED] RPC Tunnel stabilized.");
                            let _ = sqlx::query(
                                "INSERT INTO bot_event_log (event_type, severity, status, metadata)
                                 VALUES ('RPC_RECOVERED', 'info', 'recovered', $1::jsonb)"
                            )
                            .bind(BotEventType::RpcRecovered.as_str())
                                .bind(EventSeverity::Info.as_str())
                                .bind(format!(r#"{{"failed_attempts":{}}}"#, failed_attempts))
                                .execute(&ctx.pool)
                            .await;
                            ctx.live_sync_enabled.store(true, Ordering::Relaxed);
                            if let Err(e) = bot.send_message(ChatId(ctx.admin_id), "✅ <b>RECOVERED:</b> Node connection stabilized.\n▶️ UTXO Monitoring resumed smoothly.")
                                .parse_mode(teloxide::types::ParseMode::Html).await { tracing::error!("[TELEGRAM ERROR] Bot API request failed: {}", e); }

                            failed_attempts = 0;
                            is_disconnected = false;
                        }
                    }
                }
            }
        }
    });
}

pub fn spawn_memory_cleaner(ctx: AppContext, token: CancellationToken) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token.cancelled() => { break; }
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                    ctx.utxo_state.retain(|wallet, _| ctx.state.contains_key(wallet));
                    ctx.rate_limiter.retain_recent();
                    let retention_days: i64 = std::env::var("BOT_EVENT_LOG_RETENTION_DAYS")
                        .ok()
                        .and_then(|v| v.parse::<i64>().ok())
                        .unwrap_or(60)
                        .clamp(1, 365);

                    let purge_result = sqlx::query(
                        "DELETE FROM bot_event_log
                         WHERE created_at < NOW() - ($1::text || ' days')::interval"
                    )
                    .bind(retention_days.to_string())
                    .execute(&ctx.pool)
                    .await;

                    match purge_result {
                        Ok(result) => {
                            tracing::info!(
                                "[MEMORY CLEANER] Purged in-memory runtime state and {} old bot events.",
                                result.rows_affected()
                            );
                        }
                        Err(e) => {
                            tracing::error!("[DATABASE ERROR] Failed to purge old bot events: {}", e);
                        }
                    }
                }
            }
        }
    });
}
fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(env_u64("HTTP_TIMEOUT_SECS", 10)))
        .connect_timeout(Duration::from_secs(env_u64("HTTP_CONNECT_TIMEOUT_SECS", 5)))
        .user_agent("KaspaPulse/1.2")
        .build()
        .expect("failed to build HTTP client")
}

fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default_value)
}
