use anyhow::Context;
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::OnceLock;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardMarkup},
};

type SpamLimiter = RateLimiter<i64, DefaultKeyedStateStore<i64>, DefaultClock>;

pub fn f_num(n: f64) -> String {
    let s = format!("{:.0}", n);
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        result.push(c);
        if (len - i - 1) % 3 == 0 && i != len - 1 {
            result.push(',');
        }
    }
    result
}

// 🔄 Unified Community Function for Sending or In-Place Editing
pub async fn send_or_edit_log<T: AsRef<str>>(
    bot: &Bot,
    chat_id: ChatId,
    msg_id: Option<teloxide::types::MessageId>,
    text: T,
    markup: Option<InlineKeyboardMarkup>,
) -> anyhow::Result<()> {
    let text_ref = text.as_ref();
    crate::utils::log_multiline(
        &format!("📤 [BOT OUT] Chat: {}\n[RESPONSE]:", chat_id.0),
        text_ref,
        true,
    );

    let preview_opts = teloxide::types::LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_small_media: false,
        prefer_large_media: false,
        show_above_text: false,
    };

    if let Some(id) = msg_id {
        let mut req = bot
            .edit_message_text(chat_id, id, text_ref.to_string())
            .parse_mode(teloxide::types::ParseMode::Html)
            .link_preview_options(preview_opts);
        if let Some(ref m) = markup {
            req = req.reply_markup(m.clone());
        }

        match req.await {
            Ok(_) => Ok(()),
            Err(teloxide::RequestError::Api(teloxide::ApiError::MessageNotModified)) => Ok(()), // Gracefully ignore unchanged text
            Err(e) => Err(anyhow::anyhow!("API Error: {}", e)),
        }
    } else {
        let mut req = bot
            .send_message(chat_id, text_ref.to_string())
            .parse_mode(teloxide::types::ParseMode::Html)
            .link_preview_options(preview_opts);
        if let Some(ref m) = markup {
            req = req.reply_markup(m.clone());
        }
        req.await.context("API Error")?;
        Ok(())
    }
}

// 🔄 Helper to generate the Refresh Button
pub fn refresh_markup(cmd_callback: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![teloxide::types::InlineKeyboardButton::callback(
        "🔄 Refresh",
        cmd_callback,
    )]])
}

pub fn format_short_wallet(w: &str) -> String {
    let chars: Vec<char> = w.chars().collect();
    if chars.len() > 18 {
        let start: String = chars[0..12].iter().collect();
        let end: String = chars[chars.len() - 6..].iter().collect();
        format!("{}...{}", start, end)
    } else {
        w.to_string()
    }
}

pub fn format_hash(hash: &str, link_type: &str) -> String {
    format!(
        "<a href=\"https://kaspa.stream/{}/{}\">{}</a>",
        link_type,
        hash,
        format_short_wallet(hash)
    )
}

pub fn is_spam(chat_id: i64) -> bool {
    static LIMITER: OnceLock<SpamLimiter> = OnceLock::new();
    let limiter =
        LIMITER.get_or_init(|| RateLimiter::keyed(Quota::per_second(NonZeroU32::new(1).unwrap())));

    limiter.check_key(&chat_id).is_err()
}

pub fn clean_for_log(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

pub fn log_multiline(header: &str, body: &str, is_html: bool) {
    let safe_header = sanitize_for_log(header);

    for line in safe_header.lines() {
        if !line.trim().is_empty() {
            tracing::info!("{}", line);
        }
    }

    let body_to_print = if is_html {
        clean_for_log(body)
    } else {
        body.to_string()
    };

    let safe_body = sanitize_for_log(&body_to_print);

    for line in safe_body.lines() {
        if !line.trim().is_empty() {
            tracing::info!("   | {}", line);
        }
    }
}

pub async fn send_reply_or_edit_log(
    bot: &teloxide::Bot,
    chat_id: teloxide::types::ChatId,
    reply_to: teloxide::types::MessageId,
    edit_msg_id: Option<teloxide::types::MessageId>,
    text: String,
    markup: Option<teloxide::types::InlineKeyboardMarkup>,
) {
    crate::utils::log_multiline(&format!("📤 [BOT OUT] Chat: {}", chat_id.0), &text, true);

    if let Some(id) = edit_msg_id {
        let mut req = bot
            .edit_message_text(chat_id, id, text)
            .parse_mode(teloxide::types::ParseMode::Html);

        if let Some(m) = markup {
            req = req.reply_markup(m);
        }

        match req.await {
            Ok(_) => {}
            Err(teloxide::RequestError::Api(teloxide::ApiError::MessageNotModified)) => {}
            Err(e) => tracing::error!("[TELEGRAM ERROR] Failed to edit logged response: {}", e),
        }
    } else {
        let mut req = bot
            .send_message(chat_id, text)
            .reply_parameters(teloxide::types::ReplyParameters::new(reply_to))
            .parse_mode(teloxide::types::ParseMode::Html);

        if let Some(m) = markup {
            req = req.reply_markup(m);
        }

        if let Err(e) = req.await {
            tracing::error!("[TELEGRAM ERROR] Failed to send logged response: {}", e);
        }
    }
}

pub async fn send_logged_message(
    bot: &teloxide::Bot,
    chat_id: teloxide::types::ChatId,
    reply_to: Option<teloxide::types::MessageId>,
    text: String,
    markup: Option<teloxide::types::InlineKeyboardMarkup>,
) -> anyhow::Result<()> {
    crate::utils::log_multiline(&format!("📤 [BOT OUT] Chat: {}", chat_id.0), &text, true);

    let mut req = bot
        .send_message(chat_id, text)
        .parse_mode(teloxide::types::ParseMode::Html);

    if let Some(reply_id) = reply_to {
        req = req.reply_parameters(teloxide::types::ReplyParameters::new(reply_id));
    }

    if let Some(markup) = markup {
        req = req.reply_markup(markup);
    }

    if let Err(e) = req.await {
        tracing::error!("[TELEGRAM ERROR] Failed to send logged message: {}", e);
    }

    Ok(())
}

pub async fn edit_logged_message(
    bot: &teloxide::Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    text: String,
    markup: Option<teloxide::types::InlineKeyboardMarkup>,
) -> anyhow::Result<()> {
    crate::utils::log_multiline(
        &format!(
            "📤 [BOT OUT] Chat: {} | Edit Message: {}",
            chat_id.0, message_id.0
        ),
        &text,
        true,
    );

    let mut req = bot
        .edit_message_text(chat_id, message_id, text)
        .parse_mode(teloxide::types::ParseMode::Html);

    if let Some(markup) = markup {
        req = req.reply_markup(markup);
    }

    if let Err(e) = req.await {
        let msg = e.to_string();
        if !msg.to_lowercase().contains("message is not modified") {
            tracing::error!("[TELEGRAM ERROR] Failed to edit logged message: {}", msg);
        }
    }

    Ok(())
}

// === Telegram request protection helpers (Stage 3) ===

type TelegramLimiter = RateLimiter<i64, DefaultKeyedStateStore<i64>, DefaultClock>;

fn env_u32(key: &str, default_value: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(default_value)
}

fn safe_nonzero(value: u32, default_value: u32) -> NonZeroU32 {
    NonZeroU32::new(value)
        .or_else(|| NonZeroU32::new(default_value))
        .unwrap_or_else(|| NonZeroU32::new(1).unwrap())
}

fn per_second_quota(key: &str, default_value: u32) -> Quota {
    Quota::per_second(safe_nonzero(env_u32(key, default_value), default_value))
}

fn per_minute_quota(key: &str, default_value: u32) -> Quota {
    Quota::per_minute(safe_nonzero(env_u32(key, default_value), default_value))
}

pub fn is_command_rate_limited(chat_id: i64) -> bool {
    static LIMITER: OnceLock<TelegramLimiter> = OnceLock::new();

    let limiter = LIMITER
        .get_or_init(|| RateLimiter::keyed(per_second_quota("RATE_LIMIT_COMMANDS_PER_SECOND", 1)));

    limiter.check_key(&chat_id).is_err()
}

pub fn is_callback_rate_limited(chat_id: i64) -> bool {
    static LIMITER: OnceLock<TelegramLimiter> = OnceLock::new();

    let limiter = LIMITER
        .get_or_init(|| RateLimiter::keyed(per_second_quota("RATE_LIMIT_CALLBACKS_PER_SECOND", 3)));

    limiter.check_key(&chat_id).is_err()
}

pub fn is_add_wallet_rate_limited(chat_id: i64) -> bool {
    static LIMITER: OnceLock<TelegramLimiter> = OnceLock::new();

    let limiter = LIMITER.get_or_init(|| {
        RateLimiter::keyed(per_minute_quota("RATE_LIMIT_ADD_WALLET_PER_MINUTE", 5))
    });

    limiter.check_key(&chat_id).is_err()
}

pub fn max_wallets_per_user() -> i64 {
    std::env::var("MAX_WALLETS_PER_USER")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10)
}

pub fn rate_limit_message() -> &'static str {
    "⏳ <b>Too many requests.</b>\nPlease slow down and try again shortly."
}

// === End Telegram request protection helpers (Stage 3) ===

// === Logging privacy helpers (Stage 4) ===

pub fn env_bool(key: &str, default_value: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "true" || value == "1" || value == "yes" || value == "on"
        })
        .unwrap_or(default_value)
}

pub fn env_usize(key: &str, default_value: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

pub fn verbose_logs_enabled() -> bool {
    env_bool("ENABLE_VERBOSE_LOGS", false)
}

pub fn max_raw_message_chars() -> usize {
    env_usize("MAX_RAW_MESSAGE_CHARS", 512)
}

pub fn max_wallet_address_chars() -> usize {
    env_usize("MAX_WALLET_ADDRESS_CHARS", 120)
}

pub fn max_log_chars() -> usize {
    env_usize("LOG_MAX_CHARS", 5000)
}

pub fn sanitize_for_log(input: &str) -> String {
    let mut text = input.to_string();

    if !verbose_logs_enabled() {
        text = mask_kaspa_addresses(&text);
        text = mask_common_secret_values(&text);
    }

    truncate_for_log(&text)
}

pub fn sanitize_user_text(input: &str) -> String {
    input
        .replace('\u{0000}', "")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

fn truncate_for_log(input: &str) -> String {
    let max_chars = max_log_chars();

    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let short = input.chars().take(max_chars).collect::<String>();

    format!(
        "{}\n...[log truncated: original {} chars, limit {} chars]",
        short,
        input.chars().count(),
        max_chars
    )
}

fn mask_kaspa_addresses(input: &str) -> String {
    let mut output = String::new();

    for token in input.split_whitespace() {
        let cleaned = token
            .trim_matches(|c: char| {
                c == '<'
                    || c == '>'
                    || c == ','
                    || c == '.'
                    || c == ')'
                    || c == '('
                    || c == '['
                    || c == ']'
                    || c == '"'
                    || c == '\''
            })
            .to_string();

        if cleaned.starts_with("kaspa:") || cleaned.starts_with("kaspatest:") {
            output.push_str(&token.replace(&cleaned, &mask_identifier(&cleaned)));
        } else {
            output.push_str(token);
        }

        output.push(' ');
    }

    output.trim_end().to_string()
}

fn mask_identifier(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();

    if chars.len() <= 18 {
        return "***".to_string();
    }

    let start = chars.iter().take(12).collect::<String>();
    let end = chars
        .iter()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();

    format!("{}...{}", start, end)
}

fn mask_common_secret_values(input: &str) -> String {
    let sensitive_keys = [
        "BOT_TOKEN",
        "DATABASE_URL",
        "POSTGRES_URL",
        "DB_URL",
        "NODE_URL_01",
        "NODE_URL",
        "ADMIN_PIN",
        "WEBHOOK_SECRET_TOKEN",
        "COINGECKO_API_URL",
    ];

    let mut lines = Vec::new();

    for line in input.lines() {
        let mut masked = line.to_string();

        for key in sensitive_keys {
            let upper = masked.to_ascii_uppercase();

            if upper.contains(key) && (upper.contains('=') || upper.contains(':')) {
                if let Some(pos) = masked.find('=') {
                    masked = format!("{}=***REDACTED***", masked[..pos].trim_end());
                } else if let Some(pos) = masked.find(':') {
                    masked = format!("{}: ***REDACTED***", masked[..pos].trim_end());
                }
            }
        }

        lines.push(masked);
    }

    lines.join("\n")
}

pub fn validate_raw_message_size(text: &str) -> Result<(), String> {
    let max_chars = max_raw_message_chars();
    let actual_chars = text.chars().count();

    if actual_chars > max_chars {
        return Err(format!(
            "Message is too long. Limit: {} chars. Received: {} chars.",
            max_chars, actual_chars
        ));
    }

    Ok(())
}

pub fn validate_wallet_address_size(address: &str) -> Result<(), String> {
    let max_chars = max_wallet_address_chars();
    let actual_chars = address.chars().count();

    if actual_chars > max_chars {
        return Err(format!(
            "Wallet address is too long. Limit: {} chars. Received: {} chars.",
            max_chars, actual_chars
        ));
    }

    Ok(())
}

// === End logging privacy helpers (Stage 4) ===
