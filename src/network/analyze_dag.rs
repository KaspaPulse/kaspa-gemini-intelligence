use crate::domain::errors::AppError;
use crate::domain::models::BlockData;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;
use kaspa_hashes::Hash;
use kaspa_rpc_core::api::rpc::RpcApi;

use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct AnalyzeDagUseCase {
    pub node: Arc<KaspaRpcAdapter>,
}

impl AnalyzeDagUseCase {
    pub fn new(node: Arc<KaspaRpcAdapter>) -> Self {
        Self { node }
    }

    pub async fn get_pruning_block(&self, hash: &str) -> Option<BlockData> {
        self.node.get_block(hash).await.ok()
    }

    // 🚀 RESTORED ORIGINAL DETAILS ALGORITHM
    pub async fn execute(
        &self,
        f_tx: &str,
        w_cl: &str,
        daa_score: u64,
        is_coinbase: bool,
    ) -> Result<(String, Vec<String>, String, String, u64), AppError> {
        let mut acc_block_hash = String::new();
        let mut actual_mined_blocks: Vec<String> = Vec::new();
        let mut extracted_nonce = String::new();
        let mut extracted_worker = String::new();
        let mut block_time_ms: u64 = 0;
        let mut blue_block_fetch_errors: usize = 0;

        // Bypassing abstract traits to guarantee raw access to verbose_data for details
        let rpc_cl = self.node.client.clone();

        let mut visited = HashSet::new();
        let mut current_hashes = rpc_cl
            .get_block_dag_info()
            .await
            .map_err(|e| {
                AppError::NodeError(format!(
                    "DAG tip lookup failed while analyzing tx {}: {}",
                    f_tx, e
                ))
            })?
            .tip_hashes;

        for _attempt in 1..=800 {
            if current_hashes.is_empty() {
                break;
            }
            let mut next_hashes = vec![];
            for hash in &current_hashes {
                if !visited.insert(*hash) {
                    continue;
                }
                let block = match rpc_cl.get_block(*hash, true).await {
                    Ok(block) => block,
                    Err(e) => {
                        return Err(AppError::NodeError(format!(
                            "DAG block fetch failed while searching acceptance block. hash={} tx={}: {}",
                            hash, f_tx, e
                        )));
                    }
                };
                {
                    let mut found_tx = false;
                    for tx in &block.transactions {
                        if let Some(tx_verb) = &tx.verbose_data {
                            if tx_verb.transaction_id.to_string() == f_tx {
                                found_tx = true;
                                break;
                            }
                        }
                    }
                    if found_tx {
                        acc_block_hash = hash.to_string();
                        block_time_ms = block.header.timestamp;
                        break;
                    }
                    if block.header.daa_score >= daa_score.saturating_sub(60) {
                        for level in &block.header.parents_by_level {
                            for p_hash in level {
                                next_hashes.push(*p_hash);
                            }
                        }
                    }
                }
            }
            if !acc_block_hash.is_empty() {
                break;
            }
            current_hashes = next_hashes;
            sleep(Duration::from_millis(5)).await;
        }

        if is_coinbase && !acc_block_hash.is_empty() {
            let acc_hash_obj = acc_block_hash.parse::<Hash>().map_err(|e| {
                AppError::NodeError(format!(
                    "Acceptance block hash parse failed. acc_block_hash={} tx={}: {}",
                    acc_block_hash, f_tx, e
                ))
            })?;
            {
                let full_acc_block = match rpc_cl.get_block(acc_hash_obj, true).await {
                    Ok(block) => block,
                    Err(e) => {
                        return Err(AppError::NodeError(format!(
                            "Acceptance block fetch failed while extracting mined block details. acc_block_hash={} tx={}: {}",
                            acc_block_hash, f_tx, e
                        )));
                    }
                };
                {
                    let mut user_script_bytes: Vec<u8> = Vec::new();
                    if let Some(tx0) = full_acc_block.transactions.first() {
                        for out in &tx0.outputs {
                            if let Some(ov) = &out.verbose_data {
                                if ov.script_public_key_address.to_string() == w_cl {
                                    user_script_bytes = out.script_public_key.script().to_vec();
                                    break;
                                }
                            }
                        }
                    }
                    if !user_script_bytes.is_empty() {
                        if let Some(verbose) = &full_acc_block.verbose_data {
                            for blue_hash in &verbose.merge_set_blues_hashes {
                                let blue_block = match rpc_cl.get_block(*blue_hash, true).await {
                                    Ok(block) => block,
                                    Err(e) => {
                                        blue_block_fetch_errors += 1;

                                        tracing::warn!(
                                            "[DAG ANALYSIS] Failed to fetch blue block while extracting mined block details. blue_hash={} tx={}: {}",
                                            blue_hash,
                                            f_tx,
                                            e
                                        );

                                        continue;
                                    }
                                };
                                {
                                    if let Some(m_tx0) = blue_block.transactions.first() {
                                        // 🔍 THE REAL DETAILS: Searching for user bytes in the payload
                                        if let Some(pos) = m_tx0
                                            .payload
                                            .windows(user_script_bytes.len())
                                            .position(|w| w == user_script_bytes.as_slice())
                                        {
                                            actual_mined_blocks.push(blue_hash.to_string());
                                            block_time_ms = blue_block.header.timestamp;
                                            if extracted_nonce.is_empty() {
                                                extracted_nonce =
                                                    blue_block.header.nonce.to_string();
                                                let extra_data =
                                                    &m_tx0.payload[pos + user_script_bytes.len()..];
                                                // 🔍 Extracting Worker ASCII
                                                extracted_worker = extra_data
                                                    .iter()
                                                    .filter(|&&c| (32..=126).contains(&c))
                                                    .map(|&c| c as char)
                                                    .collect();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if is_coinbase
            && !acc_block_hash.is_empty()
            && actual_mined_blocks.is_empty()
            && blue_block_fetch_errors > 0
        {
            return Err(AppError::NodeError(format!(
                "Blue block fetch failed during mined block extraction. acc_block_hash={} tx={} failed_blue_blocks={}",
                acc_block_hash, f_tx, blue_block_fetch_errors
            )));
        }
        // Dependency Injection: Connection is shared, do not disconnect here.
        Ok((
            acc_block_hash,
            actual_mined_blocks,
            extracted_nonce,
            extracted_worker,
            block_time_ms,
        ))
    }
}
