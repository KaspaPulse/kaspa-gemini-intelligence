use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;
use chrono::{TimeZone, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::network::analyze_dag::AnalyzeDagUseCase;
use crate::wallet::wallet_use_cases::UtxoMonitorService;

pub fn start_utxo_monitor(
    bot: Bot,
    node: Arc<KaspaRpcAdapter>,
    db: Arc<PostgresRepository>,
    token: CancellationToken,
) {
    let analyzer = Arc::new(AnalyzeDagUseCase::new(node.clone()));
    let utxo_service = Arc::new(UtxoMonitorService::new(node.clone(), db.clone(), analyzer));
    let semaphore = Arc::new(Semaphore::new(10));

    tokio::spawn(async move {
        info!("🚀 [WORKER] UTXO monitor started.");

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    info!("[WORKER] UTXO monitor shutdown requested.");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            }

            if let Ok((is_online, _)) = node.get_node_health().await {
                if !is_online {
                    continue;
                }
            }

            let wallets = match db.get_all_tracked_wallets().await {
                Ok(w) => w,
                Err(e) => {
                    error!("[DATABASE ERROR] Failed to fetch wallets: {}", e);
                    continue;
                }
            };

            if wallets.is_empty() {
                continue;
            }

            let mut recipients_by_wallet: HashMap<String, Vec<i64>> = HashMap::new();

            for wallet in wallets {
                recipients_by_wallet
                    .entry(wallet.address)
                    .or_default()
                    .push(wallet.chat_id);
            }

            for chat_ids in recipients_by_wallet.values_mut() {
                let mut seen = HashSet::new();
                chat_ids.retain(|chat_id| seen.insert(*chat_id));
                chat_ids.sort_unstable();
            }

            let mut join_set = tokio::task::JoinSet::new();

            for (wallet_address, chat_ids) in recipients_by_wallet {
                let sem = semaphore.clone();
                let service = utxo_service.clone();
                let bot_clone = bot.clone();
                let db_clone = db.clone();

                join_set.spawn(async move {
                    let _permit = match sem.acquire_owned().await {
                        Ok(p) => p,
                        Err(_) => return,
                    };

                    match service.check_wallet_utxos(&wallet_address).await {
                        Ok(events) => {
                            for event in events {
                                let log_time = if event.block_time_ms > 0 {
                                    Utc.timestamp_millis_opt(event.block_time_ms as i64)
                                        .single()
                                        .map(|dt| dt.format("%H:%M:%S.%3f").to_string())
                                        .unwrap_or_else(|| "Unknown".to_string())
                                } else {
                                    "Real-time".to_string()
                                };

                                let final_msg =
                                    crate::presentation::telegram::formatting::events_formatter::format_live_event(&event);

                                info!(
                                    "💎 [LIVE BLOCK] | Amount: +{:.4} KAS | Wallet: {} | Time: {} | Recipients: {}",
                                    event.amount_kas,
                                    crate::utils::format_short_wallet(&event.wallet_address),
                                    log_time,
                                    chat_ids.len()
                                );

                                let wallet_masked = crate::utils::format_short_wallet(&event.wallet_address);
                                let txid_masked = crate::utils::format_short_wallet(&event.tx_id);
                                let block_masked = event
                                    .mined_block_hash
                                    .as_ref()
                                    .map(|h| crate::utils::format_short_wallet(h));

                                let _ = db_clone
                                    .record_bot_event(
                                        "ALERT_DETECTED",
                                        "info",
                                        None,
                                        None,
                                        None,
                                        None,
                                        Some(&wallet_masked),
                                        Some(&txid_masked),
                                        block_masked.as_deref(),
                                        Some("detected"),
                                        None,
                                        None,
                                        &format!(
                                            r#"{{"amount_kas":{},"recipients":{},"daa_score":{}}}"#,
                                            event.amount_kas,
                                            chat_ids.len(),
                                            event.daa_score
                                        ),
                                    )
                                    .await;
for chat_id in &chat_ids {
                                    crate::utils::log_multiline(
                                        &format!("📤 [BOT OUT] Chat: {}", chat_id),
                                        &final_msg,
                                        true,
                                    );

                                    match bot_clone
                                        .send_message(ChatId(*chat_id), &final_msg)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .link_preview_options(teloxide::types::LinkPreviewOptions {
                                            url: None,
                                            is_disabled: true,
                                            show_above_text: false,
                                            prefer_small_media: false,
                                            prefer_large_media: false,
                                        })
                                        .await
                                    {
                                        Ok(_) => {
                                            info!(
                                                "✅ [ALERT DELIVERED] Wallet: {} | Chat: {}",
                                                crate::utils::format_short_wallet(&event.wallet_address),
                                                chat_id
                                            );

                                            let _ = db_clone
                                                .record_bot_event(
                                                    "ALERT_DELIVERED",
                                                    "info",
                                                    Some(*chat_id),
                                                    None,
                                                    None,
                                                    None,
                                                    Some(&wallet_masked),
                                                    Some(&txid_masked),
                                                    block_masked.as_deref(),
                                                    Some("delivered"),
                                                    None,
                                                    None,
                                                    &format!(
                                                        r#"{{"amount_kas":{},"daa_score":{}}}"#,
                                                        event.amount_kas,
                                                        event.daa_score
                                                    ),
                                                )
                                                .await;
}
                                        Err(e) => {
                                            error!(
                                                "[TELEGRAM ERROR] Failed to send wallet alert to chat {}: {}",
                                                chat_id, e
                                            );

                                            let err_text = e.to_string();
                                            let _ = db_clone
                                                .record_bot_event(
                                                    "ALERT_DELIVERY_FAILED",
                                                    "error",
                                                    Some(*chat_id),
                                                    None,
                                                    None,
                                                    None,
                                                    Some(&wallet_masked),
                                                    Some(&txid_masked),
                                                    block_masked.as_deref(),
                                                    Some("failed"),
                                                    Some(&err_text),
                                                    None,
                                                    &format!(
                                                        r#"{{"amount_kas":{},"daa_score":{}}}"#,
                                                        event.amount_kas,
                                                        event.daa_score
                                                    ),
                                                )
                                                .await;
}
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to check UTXOs for {}: {}", wallet_address, e);
                        }
                    }
                });
            }

            while join_set.join_next().await.is_some() {}
        }
    });
}
