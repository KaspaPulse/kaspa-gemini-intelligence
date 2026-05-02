use kaspa_pulse::domain::models::{BotEventType, EventSeverity};

#[test]
fn bot_event_type_strings_are_stable() {
    assert_eq!(BotEventType::SystemStart.as_str(), "SYSTEM_START");
    assert_eq!(BotEventType::WebhookStart.as_str(), "WEBHOOK_START");
    assert_eq!(BotEventType::AlertDetected.as_str(), "ALERT_DETECTED");
    assert_eq!(BotEventType::AlertDelivered.as_str(), "ALERT_DELIVERED");
    assert_eq!(
        BotEventType::AlertDeliveryFailed.as_str(),
        "ALERT_DELIVERY_FAILED"
    );
    assert_eq!(
        BotEventType::AlertDuplicateSkipped.as_str(),
        "ALERT_DUPLICATE_SKIPPED"
    );
    assert_eq!(BotEventType::DbError.as_str(), "DB_ERROR");
    assert_eq!(BotEventType::RpcError.as_str(), "RPC_ERROR");
    assert_eq!(BotEventType::RpcRecovered.as_str(), "RPC_RECOVERED");
    assert_eq!(BotEventType::TelegramError.as_str(), "TELEGRAM_ERROR");
    assert_eq!(BotEventType::PanicEvent.as_str(), "PANIC_EVENT");
    assert_eq!(BotEventType::CommandIn.as_str(), "COMMAND_IN");
    assert_eq!(BotEventType::CallbackIn.as_str(), "CALLBACK_IN");
    assert_eq!(BotEventType::RateLimited.as_str(), "RATE_LIMITED");
    assert_eq!(BotEventType::AdminAction.as_str(), "ADMIN_ACTION");
    assert_eq!(BotEventType::EventLogPurged.as_str(), "EVENT_LOG_PURGED");
}

#[test]
fn event_severity_strings_are_stable() {
    assert_eq!(EventSeverity::Info.as_str(), "info");
    assert_eq!(EventSeverity::Warn.as_str(), "warn");
    assert_eq!(EventSeverity::Error.as_str(), "error");
}
