use std::env;

/// Validates a secondary PIN for destructive or sensitive Admin actions
#[allow(dead_code)]
#[deprecated(note="Vulnerable plaintext PIN check removed in v1.2.0")]
pub fn verify_admin_pin(provided_pin: &str) -> bool {
    let expected_pin = env::var("ADMIN_PIN")
        .expect("CRITICAL SECURITY FATAL: ADMIN_PIN is completely missing from .env file!");
    provided_pin.trim() == expected_pin.trim()
}
