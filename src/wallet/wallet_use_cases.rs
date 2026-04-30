use crate::domain::entities::{MinedBlock, TrackedWallet};
use crate::domain::errors::AppError;
use crate::domain::models::LiveBlockEvent;
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
            let blocks_1h = self.db.get_blocks_count_1h(&wallet).await.unwrap_or(0);
            let blocks_24h = self.db.get_blocks_count_24h(&wallet).await.unwrap_or(0);
            let blocks_7d = self.db.get_blocks_count_7d(&wallet).await.unwrap_or(0);
            let lifetime_blocks = self
                .db
                .get_lifetime_stats(&wallet)
                .await
                .map(|(count, _)| count)
                .unwrap_or(0);
            let daily_blocks = self.db.get_daily_blocks(&wallet).await.unwrap_or_default();

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
        let mut new_rewards = Vec::new();
        let mut known = self
            .known_utxos
            .entry(wallet_address.to_string())
            .or_default();

        let is_first_run = known.is_empty();

        for utxo in utxos {
            current_outpoints.insert(utxo.outpoint.clone());

            if !is_first_run && !known.contains(&utxo.outpoint) {
                new_rewards.push(utxo.clone());
                known.insert(utxo.outpoint.clone());
            } else if is_first_run {
                known.insert(utxo.outpoint.clone());
            }
        }

        known.retain(|outpoint| current_outpoints.contains(outpoint));

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
                        tracing::error!("[DATABASE ERROR] Failed to record mined block: {}", e);
                    }
                }

                let (acc_block_hash, actual_mined_blocks, _nonce, extracted_worker, block_time_ms) =
                    analyzer
                        .execute(
                            &utxo.transaction_id,
                            &wallet,
                            utxo.block_daa_score,
                            utxo.is_coinbase,
                        )
                        .await
                        .unwrap_or_default();

                let live_balance = node.get_balance(&wallet).await.map(|(b, _)| b).unwrap_or(0);

                let event = LiveBlockEvent {
                    is_coinbase: utxo.is_coinbase,
                    wallet_address: wallet,
                    amount_kas: utxo.amount as f64 / 1e8,
                    live_balance_kas: live_balance as f64 / 1e8,
                    tx_id: utxo.transaction_id,
                    block_time_ms,
                    acc_block_hash,
                    mined_block_hash: actual_mined_blocks.first().cloned(),
                    extracted_worker: if extracted_worker.is_empty() {
                        None
                    } else {
                        Some(extracted_worker)
                    },
                    daa_score: utxo.block_daa_score,
                };

                (block_time_ms, event)
            });
        }

        let mut sorted_events = Vec::new();

        while let Some(result) = join_set.join_next().await {
            if let Ok(data) = result {
                sorted_events.push(data);
            }
        }

        sorted_events.sort_by_key(|(time, _)| *time);

        Ok(sorted_events.into_iter().map(|(_, event)| event).collect())
    }
}
