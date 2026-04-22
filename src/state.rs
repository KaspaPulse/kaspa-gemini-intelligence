use dashmap::DashMap;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info};

/// Shared memory state for wallet tracking
pub type SharedState = Arc<DashMap<String, HashSet<i64>>>;
/// Shared memory state for UTXO tracking
pub type UtxoState = Arc<DashMap<String, HashSet<String>>>;

/// Initializes the database connection pool.
pub async fn init_db(db_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50)
        .connect(db_url)
        .await
}

// --- [FINANCIAL & STATS LOGIC] ---

/// Aggregates lifetime mining statistics for a specific wallet (Enterprise Sompi Precision).
pub async fn get_lifetime_stats(pool: &PgPool, wallet: &str) -> Result<(i64, i64), sqlx::Error> {
    let res = sqlx::query!(
        r#"SELECT COUNT(*) as "count!", (COALESCE(SUM(amount), 0))::BIGINT as "sum!" FROM mined_blocks WHERE wallet = $1"#,
        wallet
    )
    .fetch_one(pool)
    .await?;

    Ok((res.count, res.sum))
}

// --- [BLOCK RECORDING & SYNC] ---

/// Records a mined block using 'outpoint' to match the database schema.
pub async fn record_mined_block(
    pool: &PgPool,
    wallet: &str,
    outpoint: &str,
    amount: i64,
    daa_score: u64,
) {
    if let Err(e) = sqlx::query!(
        "INSERT INTO mined_blocks (wallet, outpoint, amount, daa_score) VALUES ($1, $2, $3, $4) ON CONFLICT (outpoint) DO NOTHING",
        wallet,
        outpoint,
        amount,
        daa_score as i64
    )
    .execute(pool)
    .await
    {
        error!("[DATABASE ERROR] Failed to record mined block: {}", e);
    }
}

/// Helper function required by sync.rs to record blocks during recovery.
pub async fn record_recovery_block(
    pool: &PgPool,
    wallet: &str,
    outpoint: &str,
    amount: i64,
    daa_score: u64,
) {
    record_mined_block(pool, wallet, outpoint, amount, daa_score).await;
}

/// Retrieves the latest DAA score for a wallet to determine the sync starting point.
pub async fn get_sync_checkpoint(pool: &PgPool, wallet: &str) -> u64 {
    sqlx::query_scalar!(
        "SELECT daa_score FROM mined_blocks WHERE wallet = $1 ORDER BY daa_score DESC LIMIT 1",
        wallet
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
    .map(|v| v as u64)
    .unwrap_or(0)
}

/// Placeholder for compatibility with sync.rs architecture.
pub async fn update_sync_checkpoint(_pool: &PgPool, _wallet: &str, _daa_score: u64) {
    // Checkpoint is naturally maintained by the latest daa_score in mined_blocks
}

// --- [SUBSCRIPTION MANAGEMENT] ---

/// Loads all active wallet subscriptions into the in-memory DashMap.
pub async fn load_state_from_db(pool: &PgPool, state: &SharedState) -> Result<(), sqlx::Error> {
    let rows = sqlx::query!("SELECT wallet, chat_id FROM user_wallets")
        .fetch_all(pool)
        .await?;

    for row in rows {
        state
            .entry(row.wallet)
            .or_insert_with(HashSet::new)
            .insert(row.chat_id);
    }
    info!(
        "[SYSTEM] Synchronized {} active wallets from database.",
        state.len()
    );
    Ok(())
}

/// Registers a new wallet tracking subscription.
pub async fn add_wallet_to_db(pool: &PgPool, wallet: &str, chat_id: i64) {
    if let Err(e) = sqlx::query!(
        "INSERT INTO user_wallets (wallet, chat_id) VALUES ($1, $2) 
         ON CONFLICT (wallet, chat_id) DO UPDATE SET last_active = CURRENT_TIMESTAMP",
        wallet,
        chat_id
    )
    .execute(pool)
    .await
    {
        error!("[DATABASE ERROR] Failed to add wallet subscription: {}", e);
    }
}

/// Removes a specific wallet subscription for a user.
pub async fn remove_wallet_from_db(pool: &PgPool, wallet: &str, chat_id: i64) {
    if let Err(e) = sqlx::query!(
        "DELETE FROM user_wallets WHERE wallet = $1 AND chat_id = $2",
        wallet,
        chat_id
    )
    .execute(pool)
    .await
    {
        error!(
            "[DATABASE ERROR] Failed to remove wallet subscription: {}",
            e
        );
    }
}

/// Completely purges all tracking data associated with a specific user.
pub async fn remove_all_user_data(pool: &PgPool, chat_id: i64) {
    if let Err(e) = sqlx::query!("DELETE FROM user_wallets WHERE chat_id = $1", chat_id)
        .execute(pool)
        .await
    {
        error!("[DATABASE ERROR] Failed to wipe user data: {}", e);
    }
}

// --- [AI KNOWLEDGE BASE EXTENSIONS] ---

pub async fn add_to_knowledge_base(
    pool: &PgPool,
    title: &str,
    link: &str,
    content: &str,
    source: &str,
) {
    if let Err(e) = sqlx::query!(
        "INSERT INTO knowledge_base (title, link, content, source) 
         VALUES ($1, $2, $3, $4) ON CONFLICT (link) DO NOTHING",
        title,
        link,
        content,
        source
    )
    .execute(pool)
    .await
    {
        error!(
            "[DATABASE ERROR] Failed to index knowledge base entry: {}",
            e
        );
    }
}

#[allow(dead_code)]
pub async fn get_knowledge_context(pool: &PgPool, keyword: &str) -> Option<String> {
    let search_term = format!("%{}%", keyword);
    sqlx::query_scalar!(
        "SELECT content FROM knowledge_base 
         WHERE title ILIKE $1 OR content ILIKE $1 
         ORDER BY published_at DESC LIMIT 1",
        search_term
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

// --- [CACHE LAYER FOR SETTINGS] ---

static SETTINGS_CACHE: std::sync::OnceLock<dashmap::DashMap<String, String>> =
    std::sync::OnceLock::new();

fn get_settings_cache() -> &'static dashmap::DashMap<String, String> {
    SETTINGS_CACHE.get_or_init(dashmap::DashMap::new)
}

pub async fn get_setting(pool: &PgPool, key: &str, default: &str) -> String {
    let cache = get_settings_cache();
    if let Some(val) = cache.get(key) {
        return val.clone();
    }

    let res: Option<String> =
        sqlx::query_scalar("SELECT value_data FROM system_settings WHERE key_name = $1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let final_val = match res {
        Some(val) => val,
        None => {
            let _ = sqlx::query("INSERT INTO system_settings (key_name, value_data) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(key)
                .bind(default)
                .execute(pool).await;
            default.to_string()
        }
    };

    cache.insert(key.to_string(), final_val.clone());
    final_val
}

pub async fn update_setting(pool: &PgPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO system_settings (key_name, value_data) VALUES ($1, $2) ON CONFLICT (key_name) DO UPDATE SET value_data = EXCLUDED.value_data, updated_at = CURRENT_TIMESTAMP")
        .bind(key)
        .bind(value)
        .execute(pool).await?;

    let cache = get_settings_cache();
    cache.insert(key.to_string(), value.to_string());

    Ok(())
}
