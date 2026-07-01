use crate::domain::entities::TrackedWallet;
use crate::domain::models::{BotEventType, EventSeverity};
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;
use crate::infrastructure::telegram_delivery_queue::{
    commit_alert_outbox, AlertOutboxOutcome, AlertOutboxRequest,
};
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

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
    _bot: Bot,
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
            crate::infrastructure::metrics::mark_utxo_scan();
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
                Ok(wallets) => wallets,
                Err(error) => {
                    error!("[DATABASE ERROR] Failed to fetch wallets: {}", error);
                    continue;
                }
            };

            if wallets.is_empty() {
                continue;
            }

            let recipients_by_wallet = group_wallet_subscribers(wallets);
            let mut join_set = tokio::task::JoinSet::new();

            for (wallet_address, chat_ids) in recipients_by_wallet {
                let semaphore = semaphore.clone();
                let service = utxo_service.clone();
                let db = db.clone();

                join_set.spawn(async move {
                    let _permit = match semaphore.acquire_owned().await {
                        Ok(permit) => permit,
                        Err(_) => return,
                    };

                    let events = match service.check_wallet_utxos(&wallet_address).await {
                        Ok(events) => events,
                        Err(error) => {
                            error!("Failed to check UTXOs for {}: {}", wallet_address, error);
                            return;
                        }
                    };

                    for event in events {
                        let log_time = if event.block_time_ms > 0 {
                            Utc.timestamp_millis_opt(event.block_time_ms as i64)
                                .single()
                                .map(|datetime| datetime.format("%H:%M:%S.%3f").to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Real-time".to_string()
                        };

                        let message = crate::presentation::telegram::formatting::events_formatter::format_live_event(&event);
                        let wallet_masked = crate::utils::format_short_wallet(&event.wallet_address);
                        let txid_masked = crate::utils::format_short_wallet(&event.tx_id);
                        let block_masked = event
                            .mined_block_hash
                            .as_ref()
                            .map(|hash| crate::utils::format_short_wallet(hash));

                        info!(
                            "💎 [LIVE BLOCK] | Amount: +{:.4} KAS | Wallet: {} | Time: {} | Recipients: {}",
                            event.amount_kas,
                            wallet_masked,
                            log_time,
                            chat_ids.len()
                        );

                        let request = AlertOutboxRequest {
                            wallet: &event.wallet_address,
                            source_outpoint: &event.source_outpoint,
                            alert_key: &event.alert_key,
                            message_html: &message,
                            chat_ids: &chat_ids,
                            wallet_masked: Some(&wallet_masked),
                            txid_masked: Some(&txid_masked),
                            block_hash_masked: block_masked.as_deref(),
                            amount_kas: Some(event.amount_kas),
                            daa_score: Some(event.daa_score as i64),
                        };

                        match commit_alert_outbox(&db.pool, request).await {
                            Ok(AlertOutboxOutcome::Enqueued { recipients }) => {
                                crate::infrastructure::metrics::mark_alert_detected();

                                info!(
                                    "📥 [ALERT OUTBOX COMMITTED] Wallet: {} | Recipients: {}",
                                    wallet_masked,
                                    recipients
                                );

                                let _ = db
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
                                        Some("outbox_committed"),
                                        None,
                                        None,
                                        &format!(
                                            r#"{{"amount_kas":{},"recipients":{},"daa_score":{},"outpoint":"{}"}}"#,
                                            event.amount_kas,
                                            recipients,
                                            event.daa_score,
                                            crate::utils::format_short_wallet(&event.source_outpoint)
                                        ),
                                    )
                                    .await;
                            }
                            Ok(AlertOutboxOutcome::Reconciled { recipients }) => {
                                warn!(
                                    "[ALERT OUTBOX RECONCILED] Wallet: {} | Restored recipients: {}",
                                    wallet_masked,
                                    recipients
                                );
                            }
                            Ok(AlertOutboxOutcome::Duplicate) => {
                                info!(
                                    "[ALERT DUPLICATE SKIPPED] Wallet: {} | TX: {}",
                                    wallet_masked,
                                    txid_masked
                                );

                                let _ = db
                                    .record_bot_event_typed(
                                        BotEventType::AlertDuplicateSkipped,
                                        EventSeverity::Info,
                                        None,
                                        None,
                                        None,
                                        None,
                                        Some(&wallet_masked),
                                        Some(&txid_masked),
                                        block_masked.as_deref(),
                                        Some("duplicate_skipped"),
                                        None,
                                        None,
                                        "{}",
                                    )
                                    .await;
                            }
                            Ok(AlertOutboxOutcome::Suppressed { recipients }) => {
                                for _ in 0..recipients {
                                    crate::infrastructure::metrics::inc_alerts_suppressed();
                                }

                                info!(
                                    "🔕 [ALERT SUPPRESSED] Wallet: {} | Recipients: {}",
                                    wallet_masked,
                                    recipients
                                );

                                let _ = db
                                    .record_bot_event_typed(
                                        BotEventType::AlertDeliverySuppressed,
                                        EventSeverity::Info,
                                        None,
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
                                            r#"{{"amount_kas":{},"recipients":{},"daa_score":{},"reason":"alert_delivery_disabled"}}"#,
                                            event.amount_kas,
                                            recipients,
                                            event.daa_score
                                        ),
                                    )
                                    .await;
                            }
                            Err(error) => {
                                crate::infrastructure::metrics::inc_db_errors();
                                let error_text = error.to_string();

                                error!(
                                    "[ALERT OUTBOX] Atomic commit failed. Wallet: {} | TX: {} | Error: {}. The event will be retried on the next scan.",
                                    wallet_masked,
                                    txid_masked,
                                    error_text
                                );

                                let _ = db
                                    .record_bot_event_typed(
                                        BotEventType::DbError,
                                        EventSeverity::Error,
                                        None,
                                        None,
                                        None,
                                        None,
                                        Some(&wallet_masked),
                                        Some(&txid_masked),
                                        block_masked.as_deref(),
                                        Some("alert_outbox_commit_failed"),
                                        Some(&error_text),
                                        None,
                                        r#"{"operation":"commit_alert_outbox","action":"retry_next_scan"}"#,
                                    )
                                    .await;
                            }
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
