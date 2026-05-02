use kaspa_pulse::utils::{
    format_short_wallet, is_add_wallet_rate_limited, validate_raw_message_size,
    validate_wallet_address_size,
};

#[test]
fn masks_long_wallets() {
    let wallet = "kaspa:qq2avyvncscg5dtsk8u4uwjhlr3799dhaqj8k9y6q5y9hpwfxjy6u00pep7vg";
    let masked = format_short_wallet(wallet);
    assert!(masked.starts_with("kaspa:qq2avy"));
    assert!(masked.ends_with("pep7vg"));
    assert!(masked.contains("..."));
}

#[test]
fn validates_message_size() {
    std::env::set_var("MAX_RAW_MESSAGE_CHARS", "10");
    assert!(validate_raw_message_size("short").is_ok());
    assert!(validate_raw_message_size("this message is too long").is_err());
}

#[test]
fn validates_wallet_size() {
    std::env::set_var("MAX_WALLET_ADDRESS_CHARS", "20");
    assert!(validate_wallet_address_size("kaspa:short").is_ok());
    assert!(validate_wallet_address_size("kaspa:this_is_a_very_long_wallet_value").is_err());
}

#[test]
fn add_wallet_rate_limit_blocks_burst() {
    std::env::set_var("RATE_LIMIT_ADD_WALLET_PER_MINUTE", "1");
    let chat_id = 9988776655_i64;
    let first = is_add_wallet_rate_limited(chat_id);
    let second = is_add_wallet_rate_limited(chat_id);
    assert!(!first);
    assert!(second);
}
