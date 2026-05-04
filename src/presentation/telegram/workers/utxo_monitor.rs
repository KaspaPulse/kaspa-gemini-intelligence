use crate::domain::entities::TrackedWallet;
use crate::domain::models::{BotEventType, EventSeverity};
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::network::analyze_dag::AnalyzeDagUseCase;
use crate::wallet::wallet_use_cases::UtxoMonitorService;

pub(crate) fn group_wallet_subscribers(wallets: Vec<TrackedWallet>) -> HashMap<String, Vec<i64>> {
    let mut recipients_by_wallet: HashMap<String, Vec<i64>> = HashMap::new();

    for wallet in wallets {
        recipients_by_wallet
            .entry(wallet.address)
            .or_default()
            .push(wallet.chat_id);
    }

    for chat_ids in recipients_by_wallet.values_mut() {
        chat_ids.sort_unstable();
        chat_ids.dedup();
    }

    recipients_by_wallet
}

pub fn start_utxo_monitor(
    bot: Bot,
    node: Arc<KaspaRpcAdapter>,
    db: Arc<PostgresRepository>,
    token: CancellationToken,
) {
    let analyzer = Arc::new(AnalyzeDagUseCase::new(node.clone()));
    let utxo_service = Arc::new(UtxoMonitorService::new(node.clone(), db.clone(), analyzer));
    let semaphore = Arc::new(Semaphore::new(10));

    crate::infrastructure::resilience::runtime::spawn_resilient("utxo_monitor_task", async move {
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
            let recipients_by_wallet = group_wallet_subscribers(wallets);

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
                                    .record_bot_event_typed(
                                        BotEventType::AlertDetected,
                                        EventSeverity::Info,
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
                                    if !crate::wallet::alert_delivery_gate::is_alert_delivery_enabled(&db_clone.pool).await {
                                        crate::infrastructure::metrics::inc_alerts_suppressed();

                                        info!(
                                            "🔕 [ALERT SUPPRESSED] Wallet: {} | Chat: {} | Reason: alert delivery disabled",
                                            crate::utils::format_short_wallet(&event.wallet_address),
                                            chat_id
                                        );

                                        let _ = db_clone
                                            .record_bot_event_typed(
                                                BotEventType::AlertDeliverySuppressed,
                                                EventSeverity::Info,
                                                Some(*chat_id),
                                                None,
                                                None,
                                                None,
                                                Some(&wallet_masked),
                                                Some(&txid_masked),
                                                block_masked.as_deref(),
                                                Some("suppressed"),
                                                None,
                                                None,
                                                &format!(
                                                    r#"{{"amount_kas":{},"daa_score":{},"reason":"alert_delivery_disabled"}}"#,
                                                    event.amount_kas,
                                                    event.daa_score
                                                ),
                                            )
                                            .await;

                                        continue;
                                    }

                                    if crate::infrastructure::telegram_delivery_queue::delivery_queue_enabled() {
                                        match crate::infrastructure::telegram_delivery_queue::enqueue_alert_message(
                                            &db_clone.pool,
                                            *chat_id,
                                            &final_msg,
                                            Some(&wallet_masked),
                                            Some(&txid_masked),
                                            block_masked.as_deref(),
                                            Some(event.amount_kas),
                                            Some(event.daa_score as i64),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                info!(
                                                    "📥 [ALERT QUEUED] Wallet: {} | Chat: {}",
                                                    crate::utils::format_short_wallet(&event.wallet_address),
                                                    chat_id
                                                );
                                                continue;
                                            }
                                            Err(e) => {
                                                crate::infrastructure::metrics::inc_db_errors();
                                                error!(
                                                    "[DELIVERY QUEUE] Failed to enqueue alert for chat {}: {}. Falling back to direct send.",
                                                    chat_id, e
                                                );
                                            }
                                        }
                                    }

                                    crate::utils::log_multiline(
                                        &format!("📤 [BOT OUT FALLBACK] Chat: {}", chat_id),
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
                                            );                                            let delivery_outcome =
                                                crate::wallet::alert_delivery::delivery_outcome(
                                                    crate::wallet::alert_delivery::AlertDeliveryAttempt::SendSucceeded,
                                                );

                                            if !crate::wallet::alert_delivery::should_record_delivered(delivery_outcome) {
                                                tracing::warn!(
                                                    "[ALERT DELIVERY] Unexpected non-delivered outcome after successful Telegram send. chat_id={}",
                                                    chat_id
                                                );
                                            }



                                            let _ = db_clone
                                                .record_bot_event_typed(
                                                    BotEventType::AlertDelivered,
                                                    EventSeverity::Info,
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
                                            );                                            let delivery_outcome =
                                                crate::wallet::alert_delivery::delivery_outcome(
                                                    crate::wallet::alert_delivery::AlertDeliveryAttempt::SendFailed,
                                                );

                                            if !crate::wallet::alert_delivery::should_record_failed(delivery_outcome) {
                                                tracing::warn!(
                                                    "[ALERT DELIVERY] Unexpected non-failed outcome after Telegram send error. chat_id={}",
                                                    chat_id
                                                );
                                            }



                                            let err_text = e.to_string();
                                            let _ = db_clone
                                                .record_bot_event_typed(
                                                    BotEventType::AlertDeliveryFailed,
                                                    EventSeverity::Error,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn wallet(address: &str, chat_id: i64) -> TrackedWallet {
        TrackedWallet {
            address: address.to_string(),
            chat_id,
        }
    }

    #[test]
    fn groups_same_wallet_for_multiple_chats() {
        let grouped = group_wallet_subscribers(vec![
            wallet("kaspa:wallet_a", 484901117),
            wallet("kaspa:wallet_a", 1307244272),
            wallet("kaspa:wallet_a", 1792588801),
        ]);

        let subscribers = grouped.get("kaspa:wallet_a").expect("wallet_a exists");

        assert_eq!(subscribers, &vec![484901117, 1307244272, 1792588801]);
    }

    #[test]
    fn deduplicates_duplicate_chat_ids_for_same_wallet() {
        let grouped = group_wallet_subscribers(vec![
            wallet("kaspa:wallet_a", 484901117),
            wallet("kaspa:wallet_a", 484901117),
            wallet("kaspa:wallet_a", 1307244272),
        ]);

        let subscribers = grouped.get("kaspa:wallet_a").expect("wallet_a exists");

        assert_eq!(subscribers, &vec![484901117, 1307244272]);
    }

    #[test]
    fn keeps_different_wallets_separate() {
        let grouped = group_wallet_subscribers(vec![
            wallet("kaspa:wallet_a", 1),
            wallet("kaspa:wallet_b", 2),
            wallet("kaspa:wallet_a", 3),
        ]);

        assert_eq!(grouped.get("kaspa:wallet_a"), Some(&vec![1, 3]));
        assert_eq!(grouped.get("kaspa:wallet_b"), Some(&vec![2]));
    }
}
