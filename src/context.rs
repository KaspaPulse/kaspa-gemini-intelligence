use dashmap::DashMap;
use kaspa_rpc_core::api::rpc::RpcApi;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub pool: SqlitePool,
    pub state: Arc<DashMap<String, HashSet<i64>>>,
    pub rpc: Arc<dyn RpcApi>,
    pub price_cache: Arc<RwLock<(f64, f64)>>,
    pub monitoring: Arc<AtomicBool>,
    pub admin_id: i64,
}
pub type AppContext = Arc<AppState>;
