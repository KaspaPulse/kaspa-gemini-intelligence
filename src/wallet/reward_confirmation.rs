#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardConfirmationStatus {
    pub confirmations: u64,
    pub is_confirmed: bool,
}

pub fn reward_confirmation_status(
    is_coinbase: bool,
    reward_daa_score: u64,
    virtual_daa_score: u64,
    required_confirmations: u64,
) -> RewardConfirmationStatus {
    let required_confirmations = required_confirmations.clamp(1, 10_000);
    let confirmations = virtual_daa_score.saturating_sub(reward_daa_score);

    RewardConfirmationStatus {
        confirmations,
        is_confirmed: !is_coinbase || confirmations >= required_confirmations,
    }
}
