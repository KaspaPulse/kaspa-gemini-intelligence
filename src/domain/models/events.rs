#[derive(Debug, Clone)]
pub struct LiveBlockEvent {
    pub is_coinbase: bool,
    pub wallet_address: String,
    pub amount_kas: f64,
    pub live_balance_kas: f64,
    pub tx_id: String,
    pub block_time_ms: u64,
    pub acc_block_hash: String,
    pub mined_block_hash: Option<String>,
    pub extracted_worker: Option<String>,
    pub daa_score: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotEventType {
    SystemStart,
    WebhookStart,
    AlertDetected,
    AlertDelivered,
    AlertDeliveryFailed,
    AlertDuplicateSkipped,
    DbError,
    RpcError,
    RpcRecovered,
    TelegramError,
    PanicEvent,
    CommandIn,
    CallbackIn,
    RateLimited,
    AdminAction,
    EventLogPurged,
}

impl BotEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SystemStart => "SYSTEM_START",
            Self::WebhookStart => "WEBHOOK_START",
            Self::AlertDetected => "ALERT_DETECTED",
            Self::AlertDelivered => "ALERT_DELIVERED",
            Self::AlertDeliveryFailed => "ALERT_DELIVERY_FAILED",
            Self::AlertDuplicateSkipped => "ALERT_DUPLICATE_SKIPPED",
            Self::DbError => "DB_ERROR",
            Self::RpcError => "RPC_ERROR",
            Self::RpcRecovered => "RPC_RECOVERED",
            Self::TelegramError => "TELEGRAM_ERROR",
            Self::PanicEvent => "PANIC_EVENT",
            Self::CommandIn => "COMMAND_IN",
            Self::CallbackIn => "CALLBACK_IN",
            Self::RateLimited => "RATE_LIMITED",
            Self::AdminAction => "ADMIN_ACTION",
            Self::EventLogPurged => "EVENT_LOG_PURGED",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Info,
    Warn,
    Error,
}

impl EventSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}
