use crate::domain::entities::{MinedBlock, TrackedWallet};
use crate::domain::errors::AppError;
use crate::domain::models::LiveBlockEvent;
use crate::domain::models::{BotEventRecord, BotEventType, EventSeverity};
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;
use crate::network::analyze_dag::AnalyzeDagUseCase;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;

pub struct WalletManagementUseCase {
    db: Arc<PostgresRepository>,
}

impl WalletManagementUseCase {
    pub fn new(db: Arc<PostgresRepository>) -> Self {
        Self { db }
    }

    pub async fn add_wallet(&self, address: &str, chat_id: i64) -> Result<(), AppError> {
        let wallet = TrackedWallet {
            address: address.to_string(),
            chat_id,
        };

        self.db.add_tracked_wallet(wallet).await
    }

    pub async fn remove_wallet(&self, address: &str, chat_id: i64) -> Result<(), AppError> {
        self.db.remove_tracked_wallet(address, chat_id).await
    }
}

#[derive(Debug, Clone)]
pub struct WalletBalanceDetail {
    pub address: String,
    pub balance_sompi: u64,
    pub utxos: usize,
    pub is_online: bool,
}

#[derive(Debug, Clone)]
pub struct WalletBlocksDetail {
    pub address: String,
    pub blocks_1h: i64,
    pub blocks_24h: i64,
    pub blocks_7d: i64,
    pub lifetime_blocks: i64,
    pub daily_blocks: Vec<(String, i64)>,
}

pub struct WalletQueriesUseCase {
    db: Arc<PostgresRepository>,
    node: Arc<KaspaRpcAdapter>,
}

impl WalletQueriesUseCase {
    pub fn new(db: Arc<PostgresRepository>, node: Arc<KaspaRpcAdapter>) -> Self {
        Self { db, node }
    }

    pub async fn get_list(&self, chat_id: i64) -> Result<Vec<String>, AppError> {
        let wallets = self.db.get_all_tracked_wallets().await?;

        let user_wallets = wallets
            .into_iter()
            .filter(|wallet| wallet.chat_id == chat_id)
            .map(|wallet| wallet.address)
            .collect();

        Ok(user_wallets)
    }

    pub async fn get_wallet_balances(
        &self,
        chat_id: i64,
    ) -> Result<Vec<WalletBalanceDetail>, AppError> {
        let wallets = self.get_list(chat_id).await?;
        let mut details = Vec::new();

        for wallet in wallets {
            match self.node.get_balance(&wallet).await {
                Ok((balance_sompi, utxos)) => {
                    details.push(WalletBalanceDetail {
                        address: wallet,
                        balance_sompi,
                        utxos,
                        is_online: true,
                    });
                }
                Err(_) => {
                    details.push(WalletBalanceDetail {
                        address: wallet,
                        balance_sompi: 0,
                        utxos: 0,
                        is_online: false,
                    });
                }
            }
        }

        Ok(details)
    }

    pub async fn get_wallet_blocks_details(
        &self,
        chat_id: i64,
    ) -> Result<Vec<WalletBlocksDetail>, AppError> {
        let wallets = self.get_list(chat_id).await?;
        let mut details = Vec::new();

        for wallet in wallets {
            let stats_wallet_masked = crate::utils::format_short_wallet(&wallet);

            let blocks_1h = match self.db.get_blocks_count_1h(&wallet).await {
                Ok(value) => value,
                Err(e) => {
                    let error_message = e.to_string();
                    let mut db_event =
                        BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                    db_event.wallet_masked = Some(&stats_wallet_masked);
                    db_event.status = Some("stats_1h_fallback");
                    db_event.error_message = Some(&error_message);
                    db_event.metadata_json = r#"{"operation":"get_blocks_count_1h","fallback":0}"#;

                    let _ = self.db.record_bot_event_record(db_event).await;

                    0
                }
            };

            let blocks_24h = match self.db.get_blocks_count_24h(&wallet).await {
                Ok(value) => value,
                Err(e) => {
                    let error_message = e.to_string();
                    let mut db_event =
                        BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                    db_event.wallet_masked = Some(&stats_wallet_masked);
                    db_event.status = Some("stats_24h_fallback");
                    db_event.error_message = Some(&error_message);
                    db_event.metadata_json = r#"{"operation":"get_blocks_count_24h","fallback":0}"#;

                    let _ = self.db.record_bot_event_record(db_event).await;

                    0
                }
            };

            let blocks_7d = match self.db.get_blocks_count_7d(&wallet).await {
                Ok(value) => value,
                Err(e) => {
                    let error_message = e.to_string();
                    let mut db_event =
                        BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                    db_event.wallet_masked = Some(&stats_wallet_masked);
                    db_event.status = Some("stats_7d_fallback");
                    db_event.error_message = Some(&error_message);
                    db_event.metadata_json = r#"{"operation":"get_blocks_count_7d","fallback":0}"#;

                    let _ = self.db.record_bot_event_record(db_event).await;

                    0
                }
            };
            let lifetime_blocks = match self.db.get_lifetime_stats(&wallet).await {
                Ok((count, _)) => count,
                Err(e) => {
                    let error_message = e.to_string();
                    let lifetime_wallet_masked = crate::utils::format_short_wallet(&wallet);

                    let mut db_event =
                        BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                    db_event.wallet_masked = Some(&lifetime_wallet_masked);
                    db_event.status = Some("lifetime_stats_fallback");
                    db_event.error_message = Some(&error_message);
                    db_event.metadata_json = r#"{"operation":"get_lifetime_stats","fallback":0}"#;

                    let _ = self.db.record_bot_event_record(db_event).await;

                    0
                }
            };
            let daily_blocks = match self.db.get_daily_blocks(&wallet).await {
                Ok(value) => value,
                Err(e) => {
                    let error_message = e.to_string();
                    let daily_wallet_masked = crate::utils::format_short_wallet(&wallet);

                    let mut db_event =
                        BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                    db_event.wallet_masked = Some(&daily_wallet_masked);
                    db_event.status = Some("daily_blocks_fallback");
                    db_event.error_message = Some(&error_message);
                    db_event.metadata_json =
                        r#"{"operation":"get_daily_blocks","fallback":"empty_list"}"#;

                    let _ = self.db.record_bot_event_record(db_event).await;

                    Vec::new()
                }
            };

            details.push(WalletBlocksDetail {
                address: wallet,
                blocks_1h,
                blocks_24h,
                blocks_7d,
                lifetime_blocks,
                daily_blocks,
            });
        }

        Ok(details)
    }
}

pub struct UtxoMonitorService {
    node: Arc<KaspaRpcAdapter>,
    db: Arc<PostgresRepository>,
    analyzer: Arc<AnalyzeDagUseCase>,
    known_utxos: DashMap<String, HashSet<String>>,
}

impl UtxoMonitorService {
    pub fn new(
        node: Arc<KaspaRpcAdapter>,
        db: Arc<PostgresRepository>,
        analyzer: Arc<AnalyzeDagUseCase>,
    ) -> Self {
        Self {
            node,
            db,
            analyzer,
            known_utxos: DashMap::new(),
        }
    }

    pub async fn check_wallet_utxos(
        &self,
        wallet_address: &str,
    ) -> Result<Vec<LiveBlockEvent>, AppError> {
        let utxos = self.node.get_utxos(wallet_address).await?;

        let mut current_outpoints = HashSet::new();
        let mut current_outpoints_vec = Vec::new();
        let mut new_rewards = Vec::new();

        let mut known_db = match self.db.get_seen_utxos(wallet_address).await {
            Ok(value) => value,
            Err(e) => {
                let wallet_masked = crate::utils::format_short_wallet(wallet_address);
                let error_text = e.to_string();

                let mut db_error_event =
                    BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                db_error_event.wallet_masked = Some(&wallet_masked);
                db_error_event.status = Some("seen_utxo_load_failed");
                db_error_event.error_message = Some(&error_text);
                db_error_event.metadata_json =
                    r#"{"operation":"get_seen_utxos","action":"abort_wallet_scan"}"#;

                let _ = self.db.record_bot_event_record(db_error_event).await;

                tracing::error!(
                    "[DATABASE ERROR] Failed to load seen UTXOs for wallet {}: {}",
                    wallet_masked,
                    error_text
                );

                return Err(e);
            }
        };
        let mut known_mem = self
            .known_utxos
            .entry(wallet_address.to_string())
            .or_default();

        if known_mem.is_empty() && !known_db.is_empty() {
            for outpoint in &known_db {
                known_mem.insert(outpoint.clone());
            }
        }

        let is_first_run = known_mem.is_empty() && known_db.is_empty();

        for utxo in utxos {
            current_outpoints.insert(utxo.outpoint.clone());
            current_outpoints_vec.push(utxo.outpoint.clone());

            let seen_before =
                known_mem.contains(&utxo.outpoint) || known_db.contains(&utxo.outpoint);

            if !is_first_run && !seen_before {
                new_rewards.push(utxo.clone());
            }

            known_mem.insert(utxo.outpoint.clone());
            known_db.insert(utxo.outpoint.clone());
        }

        known_mem.retain(|outpoint| current_outpoints.contains(outpoint));

        if let Err(e) = self
            .db
            .upsert_seen_utxos(wallet_address, &current_outpoints_vec)
            .await
        {
            let wallet_masked = crate::utils::format_short_wallet(wallet_address);
            let error_text = e.to_string();

            let mut db_error_event =
                BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
            db_error_event.wallet_masked = Some(&wallet_masked);
            db_error_event.status = Some("seen_utxo_upsert_failed");
            db_error_event.error_message = Some(&error_text);

            let _ = self.db.record_bot_event_record(db_error_event).await;

            tracing::error!("[DATABASE ERROR] Failed to persist seen UTXOs: {}", e);
        }

        if let Err(e) = self
            .db
            .prune_seen_utxos(wallet_address, &current_outpoints_vec)
            .await
        {
            tracing::warn!("[DATABASE WARNING] Failed to prune seen UTXOs: {}", e);
        }

        if new_rewards.is_empty() {
            return Ok(vec![]);
        }

        let mut join_set = tokio::task::JoinSet::new();

        for utxo in new_rewards {
            let analyzer = self.analyzer.clone();
            let db = self.db.clone();
            let node = self.node.clone();
            let wallet = wallet_address.to_string();

            join_set.spawn(async move {
                if utxo.is_coinbase {
                    let block = MinedBlock {
                        wallet_address: wallet.clone(),
                        outpoint: utxo.outpoint.clone(),
                        amount: utxo.amount as i64,
                        daa_score: utxo.block_daa_score,
                    };

                    if let Err(e) = db.record_mined_block(block).await {
                        let wallet_masked = crate::utils::format_short_wallet(&wallet);
                        let txid_masked = crate::utils::format_short_wallet(&utxo.transaction_id);
                        let error_text = e.to_string();

                        let mut db_error_event =
                            BotEventRecord::new(BotEventType::DbError, EventSeverity::Error);
                        db_error_event.wallet_masked = Some(&wallet_masked);
                        db_error_event.txid_masked = Some(&txid_masked);
                        db_error_event.status = Some("record_mined_block_failed");
                        db_error_event.error_message = Some(&error_text);

                        let _ = db.record_bot_event_record(db_error_event).await;

                        tracing::error!("[DATABASE ERROR] Failed to record mined block: {}", e);
                    }
                }

                let analysis = analyzer
                    .execute(
                        &utxo.transaction_id,
                        &wallet,
                        utxo.block_daa_score,
                        utxo.is_coinbase,
                    )
                    .await;

                let (acc_block_hash, actual_mined_blocks, _nonce, extracted_worker, block_time_ms) =
                    match analysis {
                        Ok(data) => data,
                        Err(e) => {
                            let wallet_masked = crate::utils::format_short_wallet(&wallet);
                            let txid_masked =
                                crate::utils::format_short_wallet(&utxo.transaction_id);
                            let error_text = e.to_string();

                            let mut rpc_error_event =
                                BotEventRecord::new(BotEventType::RpcError, EventSeverity::Error);
                            rpc_error_event.wallet_masked = Some(&wallet_masked);
                            rpc_error_event.txid_masked = Some(&txid_masked);
                            rpc_error_event.status = Some("dag_analysis_failed");
                            rpc_error_event.error_message = Some(&error_text);

                            let _ = db.record_bot_event_record(rpc_error_event).await;

                            tracing::error!(
                                "[DAG ERROR] Failed to analyze reward {} for {}: {}",
                                crate::utils::format_short_wallet(&utxo.transaction_id),
                                crate::utils::format_short_wallet(&wallet),
                                e
                            );

                            return None;
                        }
                    };

                let mined_block_hash = actual_mined_blocks.first().cloned();
                let alert_key = mined_block_hash
                    .clone()
                    .unwrap_or_else(|| utxo.transaction_id.clone());

                let txid_masked = crate::utils::format_short_wallet(&utxo.transaction_id);
                let block_masked = mined_block_hash
                    .as_ref()
                    .map(|h| crate::utils::format_short_wallet(h));

                let should_send = db
                    .try_claim_alert_key(
                        &wallet,
                        &alert_key,
                        Some(&txid_masked),
                        block_masked.as_deref(),
                    )
                    .await
                    .unwrap_or(true);

                if !should_send {
                    let wallet_masked = crate::utils::format_short_wallet(&wallet);

                    let mut duplicate_event = BotEventRecord::new(
                        BotEventType::AlertDuplicateSkipped,
                        EventSeverity::Info,
                    );
                    duplicate_event.wallet_masked = Some(&wallet_masked);
                    duplicate_event.txid_masked = Some(&txid_masked);
                    duplicate_event.block_hash_masked = block_masked.as_deref();
                    duplicate_event.status = Some("duplicate_skipped");

                    let _ = db.record_bot_event_record(duplicate_event).await;

                    return None;
                }

                let live_balance = match node.get_balance(&wallet).await {
                    Ok((balance, _)) => balance,
                    Err(e) => {
                        let error_message = e.to_string();
                        let wallet_masked = crate::utils::format_short_wallet(&wallet);
                        let txid_masked_for_balance =
                            crate::utils::format_short_wallet(&utxo.transaction_id);

                        let mut rpc_event =
                            BotEventRecord::new(BotEventType::RpcError, EventSeverity::Error);
                        rpc_event.wallet_masked = Some(&wallet_masked);
                        rpc_event.txid_masked = Some(&txid_masked_for_balance);
                        rpc_event.status = Some("live_balance_failed");
                        rpc_event.error_message = Some(&error_message);
                        rpc_event.metadata_json =
                            r#"{"operation":"get_balance","fallback_balance_sompi":0}"#;

                        let _ = db.record_bot_event_record(rpc_event).await;

                        0
                    }
                };

                let event = LiveBlockEvent {
                    is_coinbase: utxo.is_coinbase,
                    wallet_address: wallet,
                    amount_kas: utxo.amount as f64 / 1e8,
                    live_balance_kas: live_balance as f64 / 1e8,
                    tx_id: utxo.transaction_id,
                    block_time_ms,
                    acc_block_hash,
                    mined_block_hash,
                    extracted_worker: if extracted_worker.is_empty() {
                        None
                    } else {
                        Some(extracted_worker)
                    },
                    daa_score: utxo.block_daa_score,
                };

                Some((block_time_ms, event))
            });
        }

        let mut sorted_events = Vec::new();

        while let Some(result) = join_set.join_next().await {
            if let Ok(Some(data)) = result {
                sorted_events.push(data);
            }
        }

        sorted_events.sort_by_key(|(time, _)| *time);

        Ok(sorted_events.into_iter().map(|(_, event)| event).collect())
    }
}
