use kaspa_pulse::infrastructure::telegram_delivery_queue::{
    delivery_queue_enabled, retry_after_seconds, retry_delay_seconds,
};
use std::sync::Mutex;

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn telegram_delivery_queue_enabled_by_default() {
    let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");

    std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
    assert!(delivery_queue_enabled());
}

#[test]
fn telegram_delivery_queue_can_be_disabled() {
    let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");

    std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
    std::env::set_var("ENABLE_TELEGRAM_DELIVERY_QUEUE", "false");

    assert!(!delivery_queue_enabled());

    std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
}

#[test]
fn telegram_delivery_queue_accepts_enabled_values() {
    let _guard = ENV_TEST_LOCK.lock().expect("env test lock poisoned");

    std::env::set_var("ENABLE_TELEGRAM_DELIVERY_QUEUE", "enabled");
    assert!(delivery_queue_enabled());

    std::env::set_var("ENABLE_TELEGRAM_DELIVERY_QUEUE", "1");
    assert!(delivery_queue_enabled());

    std::env::remove_var("ENABLE_TELEGRAM_DELIVERY_QUEUE");
}

#[test]
fn retry_after_is_extracted_from_telegram_error() {
    assert_eq!(
        retry_after_seconds("Too Many Requests: retry_after 42"),
        Some(42)
    );
}

#[test]
fn retry_delay_uses_backoff_when_no_retry_after() {
    assert_eq!(retry_delay_seconds(0, "network error"), 5);
    assert_eq!(retry_delay_seconds(1, "network error"), 15);
    assert_eq!(retry_delay_seconds(2, "network error"), 60);
    assert_eq!(retry_delay_seconds(3, "network error"), 300);
}
