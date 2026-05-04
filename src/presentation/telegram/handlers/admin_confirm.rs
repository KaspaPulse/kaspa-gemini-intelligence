use crate::domain::models::AppContext;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

const ADMIN_CONFIRM_TTL_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveAdminAction {
    Pause,
    Resume,
    Restart,
    CleanupEvents,
    MuteAlerts,
    UnmuteAlerts,
    ClearWallets,
    ForgetAll,
    ToggleMemoryCleaner,
    ToggleLiveSync,
    ToggleMaintenance,
}

impl SensitiveAdminAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Restart => "restart",
            Self::CleanupEvents => "cleanup_events",
            Self::MuteAlerts => "mute_alerts",
            Self::UnmuteAlerts => "unmute_alerts",
            Self::ClearWallets => "clear_wallets",
            Self::ForgetAll => "forget_all",
            Self::ToggleMemoryCleaner => "toggle_memory",
            Self::ToggleLiveSync => "toggle_live_sync",
            Self::ToggleMaintenance => "toggle_maintenance",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pause => "Pause live monitoring",
            Self::Resume => "Resume live monitoring",
            Self::Restart => "Restart service request",
            Self::CleanupEvents => "Purge old event logs",
            Self::MuteAlerts => "Stop mining alert delivery",
            Self::UnmuteAlerts => "Resume mining alert delivery",
            Self::ClearWallets => "Clear all tracked wallets",
            Self::ForgetAll => "Delete all user data",
            Self::ToggleMemoryCleaner => "Toggle memory cleaner",
            Self::ToggleLiveSync => "Toggle live monitoring setting",
            Self::ToggleMaintenance => "Toggle maintenance mode",
        }
    }

    pub const fn execute_callback(self) -> &'static str {
        match self {
            Self::Pause => "cmd_pause",
            Self::Resume => "cmd_resume",
            Self::Restart => "cmd_restart",
            Self::CleanupEvents => "cmd_cleanup_events",
            Self::MuteAlerts => "do_mute_alerts",
            Self::UnmuteAlerts => "do_unmute_alerts",
            Self::ClearWallets => "do_forget_wallets",
            Self::ForgetAll => "do_forget_all",
            Self::ToggleMemoryCleaner => "btn_toggle_ENABLE_MEMORY_CLEANER",
            Self::ToggleLiveSync => "btn_toggle_ENABLE_LIVE_SYNC",
            Self::ToggleMaintenance => "btn_toggle_MAINTENANCE_MODE",
        }
    }

    pub const fn risk_text(self) -> &'static str {
        match self {
            Self::Pause => "This will stop live monitoring until it is resumed.",
            Self::Resume => "This will enable live monitoring again.",
            Self::Restart => "This will request a service restart action.",
            Self::CleanupEvents => {
                "This will purge old event records according to the configured cleanup policy."
            }
            Self::ClearWallets => "This will remove all tracked wallets for this chat.",
            Self::ForgetAll => "This will remove all wallets and user data linked to this chat.",
            Self::ToggleMemoryCleaner => "This will change the memory cleaner runtime state.",
            Self::ToggleLiveSync => "This will change live monitoring runtime state.",
            Self::ToggleMaintenance => "This will change maintenance mode.",
            Self::MuteAlerts => "This will stop Telegram mining alert delivery only. Block detection, DAG analysis, and database logging will continue.",
            Self::UnmuteAlerts => "This will resume Telegram mining alert delivery for new alerts.",
        }
    }
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn token_seed(chat_id: i64, action: SensitiveAdminAction, expires_at: u64) -> u128 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);

    let chat_component = (chat_id.unsigned_abs() as u128) << 32;
    let action_component = action.as_str().bytes().fold(0u128, |acc, b| {
        acc.wrapping_mul(131).wrapping_add(b as u128)
    });

    nanos
        ^ chat_component
        ^ action_component
        ^ ((expires_at as u128) << 16)
        ^ (std::process::id() as u128)
}

pub fn issue_token(ctx: &Arc<AppContext>, chat_id: i64, action: SensitiveAdminAction) -> String {
    cleanup_expired(ctx);

    let expires_at = now_unix_secs().saturating_add(ADMIN_CONFIRM_TTL_SECS);
    let token = format!("{:016x}", token_seed(chat_id, action, expires_at));
    let stored = format!("{}|{}|{}", action.as_str(), token, expires_at);

    ctx.admin_sessions.insert(chat_id, stored);

    token
}

pub fn confirmation_callback(action: SensitiveAdminAction, token: &str) -> String {
    format!("admin_do:{}:{}", action.as_str(), token)
}

pub fn confirmation_markup(action: SensitiveAdminAction, token: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                format!("✅ Confirm {}", action.label()),
                confirmation_callback(action, token),
            ),
            InlineKeyboardButton::callback("❌ Cancel", "cancel_action"),
        ],
        vec![InlineKeyboardButton::callback("🔙 Main Menu", "cmd_start")],
    ])
}

pub fn confirmation_text(action: SensitiveAdminAction) -> String {
    format!(
        "⚠️ <b>Admin Confirmation Required</b>\n━━━━━━━━━━━━━━━━━━\n<b>Action:</b> <code>{}</code>\n<b>Risk:</b> {}\n\nThis confirmation expires in <code>{}</code> seconds.",
        action.label(),
        action.risk_text(),
        ADMIN_CONFIRM_TTL_SECS
    )
}

pub async fn send_command_confirmation(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    ctx: &Arc<AppContext>,
    action: SensitiveAdminAction,
) -> anyhow::Result<()> {
    let token = issue_token(ctx, chat_id.0, action);

    bot.send_message(chat_id, confirmation_text(action))
        .parse_mode(ParseMode::Html)
        .reply_markup(confirmation_markup(action, &token))
        .await?;

    Ok(())
}

pub async fn edit_callback_confirmation(
    bot: &Bot,
    msg: &teloxide::types::MaybeInaccessibleMessage,
    ctx: &Arc<AppContext>,
    action: SensitiveAdminAction,
) -> anyhow::Result<()> {
    let chat_id = msg.chat().id;
    let token = issue_token(ctx, chat_id.0, action);

    bot.edit_message_text(chat_id, msg.id(), confirmation_text(action))
        .parse_mode(ParseMode::Html)
        .reply_markup(confirmation_markup(action, &token))
        .await?;

    Ok(())
}

pub fn cleanup_expired(ctx: &Arc<AppContext>) {
    let now = now_unix_secs();

    ctx.admin_sessions.retain(|_, value| {
        let mut parts = value.split('|');
        let _action = parts.next();
        let _token = parts.next();
        let expires_at = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);

        expires_at > now
    });
}

pub fn sensitive_action_from_toggle_flag(flag: &str) -> Option<SensitiveAdminAction> {
    match flag.trim().to_uppercase().as_str() {
        "ENABLE_MEMORY_CLEANER" | "MEMORY" | "MEM" => {
            Some(SensitiveAdminAction::ToggleMemoryCleaner)
        }
        "ENABLE_LIVE_SYNC" | "LIVE" | "SYNC" => Some(SensitiveAdminAction::ToggleLiveSync),
        "MAINTENANCE_MODE" | "MAINTENANCE" => Some(SensitiveAdminAction::ToggleMaintenance),
        _ => None,
    }
}

pub fn sensitive_action_from_callback(data: &str) -> Option<SensitiveAdminAction> {
    match data {
        "cmd_pause" => Some(SensitiveAdminAction::Pause),
        "cmd_resume" => Some(SensitiveAdminAction::Resume),
        "cmd_restart" => Some(SensitiveAdminAction::Restart),
        "cmd_cleanup_events" => Some(SensitiveAdminAction::CleanupEvents),
        "cmd_mute_alerts" => Some(SensitiveAdminAction::MuteAlerts),
        "cmd_unmute_alerts" => Some(SensitiveAdminAction::UnmuteAlerts),
        "confirm_forget_wallets" => Some(SensitiveAdminAction::ClearWallets),
        "confirm_forget_all" => Some(SensitiveAdminAction::ForgetAll),
        "btn_toggle_ENABLE_MEMORY_CLEANER" => Some(SensitiveAdminAction::ToggleMemoryCleaner),
        "btn_toggle_ENABLE_LIVE_SYNC" => Some(SensitiveAdminAction::ToggleLiveSync),
        "btn_toggle_MAINTENANCE_MODE" => Some(SensitiveAdminAction::ToggleMaintenance),
        _ => None,
    }
}

pub fn action_from_admin_do_callback(data: &str) -> Result<SensitiveAdminAction, String> {
    let parts = data.split(':').collect::<Vec<_>>();

    if parts.len() != 3 || parts[0] != "admin_do" {
        return Err("Invalid confirmation callback.".to_string());
    }

    match parts[1] {
        "pause" => Ok(SensitiveAdminAction::Pause),
        "resume" => Ok(SensitiveAdminAction::Resume),
        "restart" => Ok(SensitiveAdminAction::Restart),
        "cleanup_events" => Ok(SensitiveAdminAction::CleanupEvents),
        "mute_alerts" => Ok(SensitiveAdminAction::MuteAlerts),
        "unmute_alerts" => Ok(SensitiveAdminAction::UnmuteAlerts),
        "clear_wallets" => Ok(SensitiveAdminAction::ClearWallets),
        "forget_all" => Ok(SensitiveAdminAction::ForgetAll),
        "toggle_memory" => Ok(SensitiveAdminAction::ToggleMemoryCleaner),
        "toggle_live_sync" => Ok(SensitiveAdminAction::ToggleLiveSync),
        "toggle_maintenance" => Ok(SensitiveAdminAction::ToggleMaintenance),
        _ => Err("Unknown sensitive action.".to_string()),
    }
}

pub fn validate_admin_do_callback(
    ctx: &Arc<AppContext>,
    chat_id: i64,
    data: &str,
) -> Result<SensitiveAdminAction, String> {
    cleanup_expired(ctx);

    let parts = data.split(':').collect::<Vec<_>>();

    if parts.len() != 3 || parts[0] != "admin_do" {
        return Err("Invalid confirmation callback.".to_string());
    }

    let requested_action = action_from_admin_do_callback(data)?;
    let requested_token = parts[2];

    let Some((_, stored)) = ctx.admin_sessions.remove(&chat_id) else {
        return Err("Confirmation expired or missing. Please try again.".to_string());
    };

    let stored_parts = stored.split('|').collect::<Vec<_>>();

    if stored_parts.len() != 3 {
        return Err("Invalid confirmation session. Please try again.".to_string());
    }

    let stored_action = stored_parts[0];
    let stored_token = stored_parts[1];
    let expires_at = stored_parts[2]
        .parse::<u64>()
        .map_err(|_| "Invalid confirmation expiry. Please try again.".to_string())?;

    if expires_at <= now_unix_secs() {
        return Err("Confirmation expired. Please try again.".to_string());
    }

    if stored_action != requested_action.as_str() || stored_token != requested_token {
        return Err("Confirmation token mismatch. Please try again.".to_string());
    }

    Ok(requested_action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_mapping_for_sensitive_actions_is_stable() {
        assert_eq!(
            sensitive_action_from_callback("cmd_pause"),
            Some(SensitiveAdminAction::Pause)
        );
        assert_eq!(
            sensitive_action_from_callback("btn_toggle_MAINTENANCE_MODE"),
            Some(SensitiveAdminAction::ToggleMaintenance)
        );
        assert_eq!(
            SensitiveAdminAction::ForgetAll.execute_callback(),
            "do_forget_all"
        );
    }

    #[test]
    fn invalid_admin_do_callback_is_rejected() {
        assert!(action_from_admin_do_callback("bad").is_err());
        assert!(action_from_admin_do_callback("admin_do:unknown:token").is_err());
    }
}
