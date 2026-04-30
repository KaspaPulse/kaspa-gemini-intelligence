#[derive(Debug, Clone)]
pub struct TrackedWallet {
    pub address: String,
    pub chat_id: i64,
}

#[derive(Debug, Clone)]
pub struct MinedBlock {
    pub wallet_address: String,
    pub outpoint: String,
    pub amount: i64,
    pub daa_score: u64,
}
