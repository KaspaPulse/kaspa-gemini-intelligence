use kaspa_pulse::wallet::reward_confirmation::reward_confirmation_status;

#[test]
fn coinbase_reward_below_required_confirmations_waits() {
    let status = reward_confirmation_status(true, 100, 109, 10);

    assert_eq!(status.confirmations, 9);
    assert!(!status.is_confirmed);
}

#[test]
fn coinbase_reward_at_required_confirmations_is_confirmed() {
    let status = reward_confirmation_status(true, 100, 110, 10);

    assert_eq!(status.confirmations, 10);
    assert!(status.is_confirmed);
}

#[test]
fn coinbase_reward_above_required_confirmations_is_confirmed() {
    let status = reward_confirmation_status(true, 100, 115, 10);

    assert_eq!(status.confirmations, 15);
    assert!(status.is_confirmed);
}

#[test]
fn non_coinbase_utxo_is_not_blocked_by_reward_confirmation_gate() {
    let status = reward_confirmation_status(false, 100, 100, 10);

    assert_eq!(status.confirmations, 0);
    assert!(status.is_confirmed);
}

#[test]
fn virtual_daa_behind_reward_daa_saturates_to_zero() {
    let status = reward_confirmation_status(true, 200, 100, 10);

    assert_eq!(status.confirmations, 0);
    assert!(!status.is_confirmed);
}

#[test]
fn required_confirmations_are_clamped_to_safe_range() {
    let zero_required = reward_confirmation_status(true, 100, 100, 0);
    assert_eq!(zero_required.confirmations, 0);
    assert!(!zero_required.is_confirmed);

    let huge_required = reward_confirmation_status(true, 100, 10_101, u64::MAX);
    assert_eq!(huge_required.confirmations, 10_001);
    assert!(huge_required.is_confirmed);
}
