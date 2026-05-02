use crate::domain::models::AppContext;
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use chrono::Utc;
use kaspa_rpc_core::api::rpc::RpcApi;
use sqlx::Row;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use sysinfo::System;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

pub async fn handle_pause(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    app_context
        .live_sync_enabled
        .store(false, Ordering::Relaxed);
    crate::send_logged!(bot, msg, "⏸️ <b>Live monitoring paused.</b>");
    Ok(())
}

pub async fn handle_resume(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    app_context.live_sync_enabled.store(true, Ordering::Relaxed);
    crate::send_logged!(bot, msg, "▶️ <b>Live monitoring active.</b>");
    Ok(())
}

pub async fn handle_restart(bot: Bot, msg: Message) -> anyhow::Result<()> {
    crate::send_logged!(
        bot,
        msg,
        "🔄 <b>Restart requested.</b>\nPlease restart the service from the host process manager."
    );
    Ok(())
}

pub async fn handle_health(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1::BIGINT")
        .fetch_one(&app_context.pool)
        .await
        .map(|value| value == 1)
        .unwrap_or(false);

    let node_ok = app_context.rpc.get_server_info().await.is_ok();

    let tracked_wallets: i64 = if db_ok {
        sqlx::query_scalar("SELECT COUNT(*) FROM user_wallets")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0)
    } else {
        0
    };

    let last_alert: Option<chrono::NaiveDateTime> = if db_ok {
        sqlx::query_scalar("SELECT MAX(timestamp) FROM mined_blocks")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let webhook_enabled = std::env::var("USE_WEBHOOK")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    let uptime_seconds = current_process_uptime_seconds().unwrap_or_else(System::uptime);
    let uptime = format_uptime(uptime_seconds);

    let text = format!(
        "🩺 <b>Kaspa Pulse Health</b>\n\
         Community Mining Alerts\n\
         ━━━━━━━━━━━━━━━━━━\n\
         🤖 <b>Bot:</b> <code>Online</code>\n\
         🌐 <b>Node:</b> <code>{}</code>\n\
         🗄️ <b>DB:</b> <code>{}</code>\n\
         🔗 <b>Webhook:</b> <code>{}</code>\n\
         👛 <b>Tracked wallets:</b> <code>{}</code>\n\
         ⛏️ <b>Last alert:</b> <code>{}</code>\n\
         ⏱️ <b>Process uptime:</b> <code>{}</code>",
        if node_ok { "Online" } else { "Offline" },
        if db_ok { "OK" } else { "FAILED" },
        if webhook_enabled {
            "Enabled"
        } else {
            "Disabled"
        },
        tracked_wallets,
        last_alert
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "No alerts yet".to_string()),
        uptime
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}
pub async fn handle_stats(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT chat_id) FROM user_wallets")
        .fetch_one(&app_context.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

    let wallets_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_wallets")
        .fetch_one(&app_context.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

    let blocks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mined_blocks")
        .fetch_one(&app_context.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

    let text = format!(
        "📊 <b>System Stats</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         👥 Users: <code>{}</code>\n\
         👛 Wallets: <code>{}</code>\n\
         ⛏️ Mined Records: <code>{}</code>\n\
         🔄 Live Monitoring: <code>{}</code>\n\
         🧹 Memory Cleaner: <code>{}</code>\n\
         🚧 Maintenance: <code>{}</code>",
        users_count,
        wallets_count,
        blocks_count,
        app_context.live_sync_enabled.load(Ordering::Relaxed),
        app_context.memory_cleaner_enabled.load(Ordering::Relaxed),
        app_context.maintenance_mode.load(Ordering::Relaxed),
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_toggle(
    bot: Bot,
    msg: Message,
    flag: String,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let key = flag.trim().to_uppercase();
    let db = PostgresRepository::new(app_context.pool.clone());

    let new_state = match key.as_str() {
        "ENABLE_MEMORY_CLEANER" | "MEMORY" | "MEM" => {
            let current = app_context.memory_cleaner_enabled.load(Ordering::Relaxed);
            let next = !current;
            app_context
                .memory_cleaner_enabled
                .store(next, Ordering::Relaxed);
            db.update_setting("ENABLE_MEMORY_CLEANER", if next { "true" } else { "false" })
                .await?;
            Some(("ENABLE_MEMORY_CLEANER", next))
        }
        "ENABLE_LIVE_SYNC" | "LIVE" | "SYNC" => {
            let current = app_context.live_sync_enabled.load(Ordering::Relaxed);
            let next = !current;
            app_context.live_sync_enabled.store(next, Ordering::Relaxed);
            db.update_setting("ENABLE_LIVE_SYNC", if next { "true" } else { "false" })
                .await?;
            Some(("ENABLE_LIVE_SYNC", next))
        }
        "MAINTENANCE_MODE" | "MAINTENANCE" => {
            let current = app_context.maintenance_mode.load(Ordering::Relaxed);
            let next = !current;
            app_context.maintenance_mode.store(next, Ordering::Relaxed);
            db.update_setting("MAINTENANCE_MODE", if next { "true" } else { "false" })
                .await?;
            Some(("MAINTENANCE_MODE", next))
        }
        _ => None,
    };

    match new_state {
        Some((name, state)) => {
            crate::send_logged!(
                bot,
                msg,
                format!(
                    "✅ <b>Setting Updated</b>\n<code>{}</code> = <code>{}</code>",
                    name, state
                )
            );
        }
        None => {
            crate::send_logged!(
                bot,
                msg,
                "⚠️ <b>Unknown setting.</b>\nAvailable: MEMORY, SYNC, MAINTENANCE"
            );
        }
    }

    Ok(())
}

pub async fn handle_sys(bot: Bot, msg: Message, monitoring_status: bool) -> anyhow::Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory_mb = sys.total_memory() / 1024 / 1024;
    let used_memory_mb = sys.used_memory() / 1024 / 1024;
    let total_swap_mb = sys.total_swap() / 1024 / 1024;
    let used_swap_mb = sys.used_swap() / 1024 / 1024;

    let text = format!(
        "🖥️ <b>System Diagnostics</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         🔄 Monitoring: <code>{}</code>\n\
         🧠 RAM: <code>{} / {} MB</code>\n\
         💾 Swap: <code>{} / {} MB</code>\n\
         🕒 Time: <code>{}</code>",
        monitoring_status,
        used_memory_mb,
        total_memory_mb,
        used_swap_mb,
        total_swap_mb,
        Utc::now().to_rfc3339(),
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_logs(bot: Bot, msg: Message) -> anyhow::Result<()> {
    let candidates = ["bot.log", "logs/bot.log", "target/debug/bot.log"];
    let mut content = String::new();

    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = text.lines().rev().take(25).collect();
            content = lines.into_iter().rev().collect::<Vec<&str>>().join("\n");
            break;
        }
    }

    if content.trim().is_empty() {
        content = "No log file found or log file is empty.".to_string();
    }

    let safe = content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    crate::send_logged!(
        bot,
        msg,
        format!("📜 <b>Last Logs</b>\n<pre>{}</pre>", safe)
    );

    Ok(())
}

pub async fn handle_broadcast(
    bot: Bot,
    msg: Message,
    admin_id: i64,
    msg_text: String,
) -> anyhow::Result<()> {
    if msg.chat.id.0 != admin_id {
        crate::send_logged!(bot, msg, "⛔ Unauthorized.");
        return Ok(());
    }

    if msg_text.trim().is_empty() {
        crate::send_logged!(bot, msg, "⚠️ Usage: /broadcast message");
        return Ok(());
    }

    crate::send_logged!(
        bot,
        msg,
        format!(
            "📣 <b>Broadcast prepared.</b>\nThis safe build does not mass-send automatically.\n\nMessage:\n{}",
            msg_text
        )
    );

    Ok(())
}

pub async fn handle_db_diag(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let connection_ok = sqlx::query_scalar::<_, i64>("SELECT 1::BIGINT")
        .fetch_one(&app_context.pool)
        .await
        .map(|value| value == 1)
        .unwrap_or(false);

    let settings_count: i64 = if connection_ok {
        sqlx::query_scalar("SELECT COUNT(*) FROM system_settings")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0)
    } else {
        0
    };

    let wallets_count: i64 = if connection_ok {
        sqlx::query_scalar("SELECT COUNT(*) FROM user_wallets")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0)
    } else {
        0
    };

    let mined_count: i64 = if connection_ok {
        sqlx::query_scalar("SELECT COUNT(*) FROM mined_blocks")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0)
    } else {
        0
    };

    let text = format!(
        "🧪 <b>Database Diagnostics</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         Connection: <code>{}</code>\n\
         Ping Query: <code>SELECT 1::BIGINT</code>\n\
         Settings Rows: <code>{}</code>\n\
         Wallet Rows: <code>{}</code>\n\
         Mined Rows: <code>{}</code>",
        if connection_ok { "OK" } else { "FAILED" },
        settings_count,
        wallets_count,
        mined_count,
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}
pub async fn handle_interactive_settings(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    msg_id: Option<teloxide::types::MessageId>,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let mem = app_context.memory_cleaner_enabled.load(Ordering::Relaxed);
    let sync = app_context.live_sync_enabled.load(Ordering::Relaxed);
    let maint = app_context.maintenance_mode.load(Ordering::Relaxed);

    let text = format!(
        "⚙️ <b>Settings Panel</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         🧹 Memory Cleaner: <code>{}</code>\n\
         🔄 Live Monitoring: <code>{}</code>\n\
         🚧 Maintenance: <code>{}</code>",
        mem, sync, maint
    );

    let markup = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Toggle Memory", "btn_toggle_ENABLE_MEMORY_CLEANER"),
            InlineKeyboardButton::callback("Toggle Monitoring", "btn_toggle_ENABLE_LIVE_SYNC"),
        ],
        vec![InlineKeyboardButton::callback(
            "Toggle Maintenance",
            "btn_toggle_MAINTENANCE_MODE",
        )],
    ]);

    if let Some(id) = msg_id {
        let _ = bot
            .edit_message_text(chat_id, id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(markup)
            .await?;
    } else {
        let _ = bot
            .send_message(chat_id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(markup)
            .await?;
    }

    Ok(())
}

fn current_process_uptime_seconds() -> Option<u64> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let pid = sysinfo::get_current_pid().ok()?;
    let process = sys.process(pid)?;
    let process_start = process.start_time();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    now.checked_sub(process_start)
}

pub async fn handle_events(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            created_at,
            event_type,
            severity,
            chat_id,
            wallet_masked,
            status,
            error_message,
            metadata::text AS metadata_text
        FROM bot_event_log
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&app_context.pool)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => {
            crate::send_logged!(
                bot,
                msg,
                format!("❌ <b>Events unavailable.</b>\n<code>{}</code>", e)
            );
            return Ok(());
        }
    };

    if rows.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "📜 <b>Recent Bot Events</b>\n━━━━━━━━━━━━━━━━━━\nNo events found."
        );
        return Ok(());
    }

    let mut text = String::from("📜 <b>Recent Bot Events</b>\n━━━━━━━━━━━━━━━━━━\n");

    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let event_type: String = row.try_get("event_type")?;
        let severity: String = row.try_get("severity")?;
        let chat_id: Option<i64> = row.try_get("chat_id")?;
        let wallet_masked: Option<String> = row.try_get("wallet_masked")?;
        let status: Option<String> = row.try_get("status")?;
        let error_message: Option<String> = row.try_get("error_message")?;
        let metadata_text: Option<String> = row.try_get("metadata_text")?;

        let icon = match event_type.as_str() {
            "SYSTEM_START" => "🚀",
            "ALERT_DETECTED" => "🔎",
            "ALERT_DELIVERED" => "✅",
            "ALERT_DELIVERY_FAILED" => "❌",
            "DB_ERROR" => "🗄️",
            "RPC_ERROR" => "🌐",
            "TELEGRAM_ERROR" => "📨",
            _ => "•",
        };

        let severity_icon = if severity.eq_ignore_ascii_case("error") {
            "🔴"
        } else if severity.eq_ignore_ascii_case("warn") {
            "🟡"
        } else {
            "🟢"
        };

        text.push_str(&format!(
            "\n{} {} <b>{}</b>\n",
            icon, severity_icon, event_type
        ));

        text.push_str(&format!(
            "⏱️ <code>{}</code>\n",
            created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(status) = status {
            text.push_str(&format!("Status: <code>{}</code>\n", status));
        }

        if let Some(chat_id) = chat_id {
            text.push_str(&format!("Chat: <code>{}</code>\n", chat_id));
        }

        if let Some(wallet) = wallet_masked {
            if !wallet.trim().is_empty() {
                text.push_str(&format!("Wallet: <code>{}</code>\n", wallet));
            }
        }

        if let Some(error) = error_message {
            if !error.trim().is_empty() {
                let short_error = if error.chars().count() > 120 {
                    format!("{}...", error.chars().take(120).collect::<String>())
                } else {
                    error
                };
                text.push_str(&format!("Error: <code>{}</code>\n", short_error));
            }
        }

        if event_type == "ALERT_DETECTED" {
            if let Some(metadata) = metadata_text {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&metadata) {
                    if let Some(recipients) = value.get("recipients").and_then(|v| v.as_i64()) {
                        text.push_str(&format!("Recipients: <code>{}</code>\n", recipients));
                    }
                    if let Some(amount) = value.get("amount_kas").and_then(|v| v.as_f64()) {
                        text.push_str(&format!("Amount: <code>{:.4} KAS</code>\n", amount));
                    }
                }
            }
        }
    }

    if text.chars().count() > 3900 {
        text = text.chars().take(3900).collect::<String>();
        text.push_str("\n\n… truncated");
    }

    crate::send_logged!(bot, msg, text);
    Ok(())
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
