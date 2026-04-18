use dashmap::DashMap;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info};

pub type SharedState = Arc<DashMap<String, HashSet<i64>>>;
pub type UtxoState = Arc<DashMap<String, HashSet<String>>>;

pub async fn init_db(db_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(db_url)
        .await?;

    // Added last_active for Data Retention Policy
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_wallets (
            wallet VARCHAR(255) NOT NULL,
            chat_id BIGINT NOT NULL,
            last_active TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (wallet, chat_id)
        )",
    )
    .execute(&pool)
    .await?;

    // Safe migration if the column doesn't exist
    let _ = sqlx::query("ALTER TABLE user_wallets ADD COLUMN IF NOT EXISTS last_active TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP").execute(&pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mined_blocks (
            outpoint VARCHAR(255) PRIMARY KEY,
            wallet VARCHAR(255) NOT NULL,
            amount DOUBLE PRECISION NOT NULL,
            daa_score BIGINT NOT NULL,
            timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            sync_source VARCHAR(50) DEFAULT 'LIVE'
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_wallet_stats ON mined_blocks(wallet, timestamp)")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS sync_checkpoint (wallet VARCHAR(255) PRIMARY KEY, last_daa_score BIGINT NOT NULL)").execute(&pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS knowledge_base (id SERIAL PRIMARY KEY, title TEXT NOT NULL, link TEXT UNIQUE NOT NULL, content TEXT NOT NULL, source TEXT NOT NULL, published_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP)").execute(&pool).await?;

    Ok(pool)
}

#[allow(dead_code)]
pub async fn update_user_activity(pool: &PgPool, chat_id: i64) {
    let _ =
        sqlx::query("UPDATE user_wallets SET last_active = CURRENT_TIMESTAMP WHERE chat_id = $1")
            .bind(chat_id)
            .execute(pool)
            .await;
}

#[allow(dead_code)]
pub async fn enforce_retention_policy(pool: &PgPool) {
    // GDPR Standard: Delete PII (wallet linkage) after 90 days of inactivity
    match sqlx::query(
        "DELETE FROM user_wallets WHERE last_active < CURRENT_TIMESTAMP - INTERVAL '90 days'",
    )
    .execute(pool)
    .await
    {
        Ok(res) => {
            if res.rows_affected() > 0 {
                info!(
                    "🛡️ [PRIVACY] Retention Policy Enforced: Deleted {} inactive user linkages.",
                    res.rows_affected()
                );
            }
        }
        Err(e) => error!("⚠️ [PRIVACY] Failed to enforce retention policy: {}", e),
    }
}

pub async fn record_mined_block(
    pool: &PgPool,
    outpoint: &str,
    wallet: &str,
    amount: f64,
    daa: u64,
) {
    if let Err(e) = sqlx::query("INSERT INTO mined_blocks (outpoint, wallet, amount, daa_score, sync_source) VALUES ($1, $2, $3, $4, 'LIVE') ON CONFLICT (outpoint) DO NOTHING")
        .bind(outpoint).bind(wallet).bind(amount).bind(daa as i64).execute(pool).await { error!("[DB ERROR] {}", e); }
}

pub async fn record_recovery_block(
    pool: &PgPool,
    outpoint: &str,
    wallet: &str,
    amount: f64,
    daa: u64,
) {
    if let Err(e) = sqlx::query("INSERT INTO mined_blocks (outpoint, wallet, amount, daa_score, sync_source) VALUES ($1, $2, $3, $4, 'RECOVERY') ON CONFLICT (outpoint) DO NOTHING")
        .bind(outpoint).bind(wallet).bind(amount).bind(daa as i64).execute(pool).await { error!("[DB ERROR] {}", e); }
}

pub async fn get_sync_checkpoint(pool: &PgPool, wallet: &str) -> u64 {
    sqlx::query_scalar("SELECT last_daa_score FROM sync_checkpoint WHERE wallet = $1")
        .bind(wallet)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0) as u64
}

pub async fn update_sync_checkpoint(pool: &PgPool, wallet: &str, daa_score: u64) {
    let _ = sqlx::query("INSERT INTO sync_checkpoint (wallet, last_daa_score) VALUES ($1, $2) ON CONFLICT (wallet) DO UPDATE SET last_daa_score = EXCLUDED.last_daa_score")
        .bind(wallet).bind(daa_score as i64).execute(pool).await;
}

pub async fn get_lifetime_stats(pool: &PgPool, wallet: &str) -> Result<(i64, f64), sqlx::Error> {
    sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(amount), 0.0) FROM mined_blocks WHERE wallet = $1",
    )
    .bind(wallet)
    .fetch_one(pool)
    .await
}

pub async fn load_state_from_db(pool: &PgPool, state: &SharedState) -> Result<(), sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT wallet, chat_id FROM user_wallets")
        .fetch_all(pool)
        .await?;
    for (wallet, chat_id) in rows {
        state
            .entry(wallet)
            .or_insert_with(HashSet::new)
            .insert(chat_id);
    }
    info!("[DB] Synchronized {} active wallets.", state.len());
    Ok(())
}

pub async fn add_wallet_to_db(pool: &PgPool, wallet: &str, chat_id: i64) {
    let _ = sqlx::query("INSERT INTO user_wallets (wallet, chat_id) VALUES ($1, $2) ON CONFLICT (wallet, chat_id) DO UPDATE SET last_active = CURRENT_TIMESTAMP")
        .bind(wallet).bind(chat_id).execute(pool).await;
}

pub async fn remove_wallet_from_db(pool: &PgPool, wallet: &str, chat_id: i64) {
    let _ = sqlx::query("DELETE FROM user_wallets WHERE wallet = $1 AND chat_id = $2")
        .bind(wallet)
        .bind(chat_id)
        .execute(pool)
        .await;
}

pub async fn remove_all_user_data(pool: &PgPool, _state: &SharedState, chat_id: i64) {
    let _ = sqlx::query("DELETE FROM user_wallets WHERE chat_id = $1")
        .bind(chat_id)
        .execute(pool)
        .await;
}

// --- START OF AI KNOWLEDGE BASE EXTENSIONS ---
pub async fn add_to_knowledge_base(
    pool: &PgPool,
    title: &str,
    link: &str,
    content: &str,
    source: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO knowledge_base (title, link, content, source) VALUES ($1, $2, $3, $4) ON CONFLICT (link) DO NOTHING"
    )
    .bind(title)
    .bind(link)
    .bind(content)
    .bind(source)
    .execute(pool)
    .await;
}

#[allow(dead_code)]
pub async fn get_knowledge_context(pool: &PgPool, keyword: &str) -> Option<String> {
    let search_term = format!("%{}%", keyword);
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM knowledge_base WHERE title ILIKE $1 OR content ILIKE $1 ORDER BY published_at DESC LIMIT 1"
    )
    .bind(search_term)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    result.map(|r| r.0)
}
// --- END OF AI KNOWLEDGE BASE EXTENSIONS ---
