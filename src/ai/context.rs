use kaspa_addresses::Address;
use kaspa_rpc_core::api::rpc::RpcApi;

use crate::context::AppContext;

pub async fn inject_live_wallet_context(chat_id: i64, ctx: &AppContext) -> String {
    let mut live_data = String::new();

    if let Ok(dag_info) = ctx.rpc.get_block_dag_info().await {
        live_data.push_str(&format!(
            "Network Difficulty: {}. \nDAA Score: {}. \n",
            crate::kaspa_features::format_difficulty(dag_info.difficulty),
            dag_info.virtual_daa_score
        ));
    }

    let price = ctx.price_cache.read().await.0;
    if price > 0.0 {
        live_data.push_str(&format!("KAS Price: ${:.4} USD. \n", price));
    }

    let wallets: Vec<String> = ctx
        .state
        .iter()
        .filter(|e| e.value().contains(&chat_id))
        .map(|e| e.key().clone())
        .collect();

    if !wallets.is_empty() {
        let mut total = 0.0;
        for w in &wallets {
            if let Ok(addr) = Address::try_from(w.as_str()) {
                if let Ok(utxos) = ctx.rpc.get_utxos_by_addresses(vec![addr]).await {
                    total += utxos
                        .iter()
                        .map(|u| u.utxo_entry.amount as f64)
                        .sum::<f64>()
                        / 1e8;
                }
            }
        }
        live_data.push_str(&format!("User Balance: {:.8} KAS.\n", total));
    } else {
        live_data.push_str("User Balance: 0 KAS (No wallet tracked).\n");
    }

    live_data
}
