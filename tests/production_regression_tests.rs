use std::fs;

fn read_source(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}

fn extract_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("start marker not found: {}", start));

    let after_start = &source[start_index..];

    let end_index = after_start
        .find(end)
        .unwrap_or_else(|| panic!("end marker not found: {}", end));

    &after_start[..end_index]
}

#[test]
fn dag_candidate_missing_block_must_not_abort_search() {
    let source = read_source("src/network/analyze_dag.rs");

    let candidate_search = extract_between(
        &source,
        "for hash in &current_hashes",
        "if !acc_block_hash.is_empty()",
    );

    assert!(
        candidate_search.contains("rpc_cl.get_block(*hash, true).await"),
        "DAG candidate search must fetch candidate blocks"
    );

    assert!(
        candidate_search.contains("continue;"),
        "missing/unavailable DAG candidate blocks must be skipped so the search can continue"
    );

    assert!(
        !candidate_search.contains("DAG block fetch failed while searching acceptance block"),
        "candidate block fetch failures must not abort DAG search; they should warn and continue"
    );

    assert!(
        !candidate_search.contains("return Err(AppError::NodeError"),
        "candidate block fetch failures must not return Err inside the candidate-search loop"
    );
}

#[test]
fn dag_tip_lookup_must_not_silently_fallback_to_empty_tips() {
    let source = read_source("src/network/analyze_dag.rs");

    assert!(
        !source.contains("Err(_) => vec![]"),
        "DAG tip lookup must propagate or log errors, not silently fallback to empty tips"
    );

    assert!(
        source.contains("DAG tip lookup failed while analyzing tx"),
        "DAG tip lookup must have an explicit error message"
    );
}

#[test]
fn dag_execute_must_not_use_silent_rpc_ok_or_unwrap_fallbacks() {
    let source = read_source("src/network/analyze_dag.rs");

    let execute_body = extract_between(
        &source,
        "pub async fn execute",
        "// Dependency Injection: Connection is shared",
    );

    assert!(
        !execute_body.contains(".await.ok()"),
        "execute() must not hide RPC errors with .ok()"
    );

    assert!(
        !execute_body.contains("unwrap_or_default()"),
        "execute() must not hide sensitive DAG data with unwrap_or_default()"
    );

    assert!(
        !execute_body.contains("unwrap_or(0)"),
        "execute() must not hide sensitive DAG data with unwrap_or(0)"
    );

    assert!(
        !execute_body.contains("if let Ok(block) = rpc_cl.get_block(*hash, true).await"),
        "candidate block fetch must not use silent if-let Ok"
    );
}

#[test]
fn blue_block_fetch_errors_are_not_silent_when_no_actual_block_is_found() {
    let source = read_source("src/network/analyze_dag.rs");

    assert!(
        source.contains("blue_block_fetch_errors"),
        "blue block fetch failures must be counted"
    );

    assert!(
        source.contains("Blue block fetch failed during mined block extraction"),
        "blue block fetch failures must become explicit when no actual mined block is found"
    );
}

#[test]
fn wallet_utxo_seen_state_must_not_silently_fallback_to_empty_db_state() {
    let source = read_source("src/wallet/wallet_use_cases.rs");

    assert!(
        !source.contains(
            ".get_seen_utxos(wallet_address)\n            .await\n            .unwrap_or_default()"
        ),
        "seen UTXO DB load must not silently fallback to empty state"
    );

    assert!(
        source.contains("seen_utxo_load_failed"),
        "seen UTXO load failures must be logged as DB errors"
    );

    assert!(
        source.contains(r#""action":"abort_wallet_scan""#),
        "seen UTXO load failure must abort wallet scan rather than continue with incomplete state"
    );
}

#[test]
fn live_balance_fallback_must_be_logged() {
    let source = read_source("src/wallet/wallet_use_cases.rs");

    assert!(
        !source.contains("node.get_balance(&wallet).await.map(|(b, _)| b).unwrap_or(0)"),
        "live balance RPC fallback must not be silent"
    );

    assert!(
        source.contains("live_balance_failed"),
        "live balance fallback must be logged"
    );
}

#[test]
fn kaspa_adapter_must_not_unwrap_user_supplied_addresses() {
    let source = read_source("src/infrastructure/node/kaspa_adapter.rs");

    let function_body = extract_between(
        &source,
        "pub async fn get_utxos_by_addresses",
        "pub async fn connect",
    );

    assert!(
        !function_body.contains(".unwrap()"),
        "get_utxos_by_addresses must not unwrap address parsing"
    );

    assert!(
        function_body.contains("Invalid Kaspa address passed to get_utxos_by_addresses"),
        "invalid address parsing must return an explicit AppError"
    );
}

#[test]
fn reward_confirmation_gate_must_run_before_dag_analysis() {
    let source = read_source("src/wallet/wallet_use_cases.rs");

    assert!(
        source.contains("MIN_REWARD_CONFIRMATIONS"),
        "reward confirmation threshold must be configurable"
    );

    assert!(
        source.contains("get_virtual_daa_score"),
        "reward confirmation gate must use virtual DAA score"
    );

    assert!(
        source.contains("virtual_daa_score.saturating_sub(utxo.block_daa_score)"),
        "confirmations must be calculated from virtual DAA minus reward DAA"
    );

    assert!(
        source.contains("reward_confirmations >= min_reward_confirmations"),
        "DAG analysis must wait until the reward reaches the configured confirmations"
    );

    let before_join_set = extract_between(
        &source,
        "let utxos = self.node.get_utxos(wallet_address).await?",
        "let mut join_set = tokio::task::JoinSet::new();",
    );

    assert!(
        before_join_set.contains("continue;"),
        "unconfirmed rewards must stay unprocessed until they reach the confirmation threshold"
    );

    assert!(
        before_join_set.contains("new_rewards.push(utxo.clone())"),
        "confirmed rewards must still enter the DAG analysis path"
    );
}

#[test]
fn unconfirmed_rewards_must_not_be_marked_seen_before_processing() {
    let source = read_source("src/wallet/wallet_use_cases.rs");

    let loop_body = extract_between(&source, "for utxo in utxos", "known_mem.retain");

    assert!(
        loop_body.contains("if !reward_is_confirmed"),
        "the monitor must explicitly handle unconfirmed rewards"
    );

    assert!(
        loop_body.contains("continue;"),
        "unconfirmed rewards must not fall through into seen UTXO persistence"
    );

    assert!(
        loop_body.contains("current_outpoints_vec.push(utxo.outpoint.clone())"),
        "confirmed or already-seen UTXOs must still be persisted"
    );
}

#[test]
fn help_guide_must_include_current_commands_buttons_and_safety_policy() {
    let source = read_source("src/presentation/telegram/handlers/mod.rs");

    assert!(
        source.contains("Reward Confirmation Policy"),
        "/help must explain reward confirmation policy"
    );

    assert!(
        source.contains("10 DAA confirmations"),
        "/help must mention the default confirmation threshold"
    );

    assert!(
        source.contains("Wallet Buttons"),
        "/help must include wallet button guide"
    );

    assert!(
        source.contains("Owner Buttons"),
        "/help must include owner/admin button guide"
    );

    assert!(
        source.contains("/events") && source.contains("/errors") && source.contains("/delivery"),
        "/help must include observability commands"
    );

    assert!(
        source.contains("DAG analysis does not stop when a candidate block is unavailable"),
        "/help must explain the DAG safety behavior"
    );

    assert!(
        source.contains("help_text_2"),
        "/help should be split into multiple Telegram-safe messages"
    );
}
