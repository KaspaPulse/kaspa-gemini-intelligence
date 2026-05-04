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

fn format_uptime(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
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
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_wallets")
            .fetch_one(&app_context.pool)
            .await
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
        .unwrap_or(0);

    let wallets_count: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_wallets")
        .fetch_one(&app_context.pool)
        .await
        .unwrap_or(0);

    let blocks_count: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mined_blocks")
        .fetch_one(&app_context.pool)
        .await
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
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM system_settings")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    let wallets_count: i64 = if connection_ok {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_wallets")
            .fetch_one(&app_context.pool)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    let mined_count: i64 = if connection_ok {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mined_blocks")
            .fetch_one(&app_context.pool)
            .await
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
    fn html_escape(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());

        for ch in value.chars() {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                _ => escaped.push(ch),
            }
        }

        escaped
    }

    fn compact_text(value: &str, max_chars: usize) -> String {
        let cleaned = value
            .replace(['\r', '\n', '\t'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if cleaned.chars().count() <= max_chars {
            return cleaned;
        }

        let mut short = cleaned.chars().take(max_chars).collect::<String>();
        short.push('…');
        short
    }

    fn event_icon(event_type: &str) -> &'static str {
        match event_type {
            "SYSTEM_START" => "🚀",
            "SYSTEM_SHUTDOWN" => "🛑",
            "WEBHOOK_START" => "🌐",
            "PANIC_EVENT" => "💥",
            "ALERT_DETECTED" => "🔎",
            "ALERT_DELIVERED" => "✅",
            "ALERT_DELIVERY_FAILED" => "❌",
            "ALERT_DUPLICATE_SKIPPED" => "♻️",
            "DB_ERROR" => "🗄️",
            "RPC_ERROR" => "🌐",
            "RPC_RECOVERED" => "✅",
            "TELEGRAM_ERROR" => "📨",
            "EVENT_LOG_PURGED" => "🧹",
            "ADMIN_ACTION" => "🛡️",
            "RATE_LIMITED" => "⏳",
            _ => "•",
        }
    }

    fn severity_icon(severity: &str) -> &'static str {
        if severity.eq_ignore_ascii_case("error") {
            "🔴"
        } else if severity.eq_ignore_ascii_case("warn") {
            "🟡"
        } else {
            "🟢"
        }
    }

    let rows = sqlx::query(
        r#"
        SELECT
            created_at,
            event_type,
            severity,
            chat_id,
            wallet_masked,
            status,
            error_message
        FROM bot_event_log
        ORDER BY created_at DESC
        LIMIT 10
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
                format!(
                    "❌ <b>Events unavailable.</b>\n<code>{}</code>",
                    html_escape(&e.to_string())
                )
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

    let mut text = String::from(
        "📜 <b>Recent Bot Events</b>\n━━━━━━━━━━━━━━━━━━\n<code>Latest 10 compact events</code>\n",
    );

    for (index, row) in rows.iter().enumerate() {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let event_type: String = row.try_get("event_type")?;
        let severity: String = row.try_get("severity")?;
        let chat_id: Option<i64> = row.try_get("chat_id")?;
        let wallet_masked: Option<String> = row.try_get("wallet_masked")?;
        let status: Option<String> = row.try_get("status")?;
        let error_message: Option<String> = row.try_get("error_message")?;

        let status_text = status
            .as_deref()
            .map(|value| compact_text(value, 28))
            .unwrap_or_else(|| "-".to_string());

        text.push_str(&format!(
            "\n<code>{:02}</code> {} {} <b>{}</b>\n",
            index + 1,
            severity_icon(&severity),
            event_icon(&event_type),
            html_escape(&event_type)
        ));

        text.push_str(&format!(
            "⏱ <code>{}</code> | <code>{}</code>\n",
            created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            html_escape(&status_text)
        ));

        let mut details = Vec::new();

        if let Some(chat_id) = chat_id {
            details.push(format!("Chat: <code>{}</code>", chat_id));
        }

        if let Some(wallet) = wallet_masked {
            details.push(format!(
                "Wallet: <code>{}</code>",
                html_escape(&compact_text(&wallet, 32))
            ));
        }

        if !details.is_empty() {
            text.push_str(&details.join(" | "));
            text.push('\n');
        }

        if let Some(error) = error_message {
            let error = compact_text(&error, 90);
            if !error.is_empty() {
                text.push_str(&format!("⚠️ <code>{}</code>\n", html_escape(&error)));
            }
        }
    }

    if text.chars().count() > 3900 {
        text = text.chars().take(3900).collect::<String>();
        text.push_str("\n… truncated");
    }

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_errors(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT created_at, event_type, severity, chat_id, wallet_masked, status, error_message
        FROM bot_event_log
        WHERE severity = 'error'
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&app_context.pool)
    .await?;

    let mut text = String::from("🚨 <b>Recent Error Events</b>\n━━━━━━━━━━━━━━━━━━\n");

    if rows.is_empty() {
        text.push_str("No recent errors.");
    }

    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let event_type: String = row.try_get("event_type")?;
        let chat_id: Option<i64> = row.try_get("chat_id")?;
        let wallet: Option<String> = row.try_get("wallet_masked")?;
        let status: Option<String> = row.try_get("status")?;
        let err: Option<String> = row.try_get("error_message")?;

        text.push_str(&format!(
            "\n🔴 <b>{}</b>\n⏱️ <code>{}</code>\n",
            event_type,
            created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(status) = status {
            text.push_str(&format!("Status: <code>{}</code>\n", status));
        }
        if let Some(chat_id) = chat_id {
            text.push_str(&format!("Chat: <code>{}</code>\n", chat_id));
        }
        if let Some(wallet) = wallet {
            if !wallet.is_empty() {
                text.push_str(&format!("Wallet: <code>{}</code>\n", wallet));
            }
        }
        if let Some(err) = err {
            let short = if err.chars().count() > 120 {
                format!("{}...", err.chars().take(120).collect::<String>())
            } else {
                err
            };
            text.push_str(&format!("Error: <code>{}</code>\n", short));
        }
    }

    if text.chars().count() > 3400 {
        text = text.chars().take(3400).collect::<String>();
        text.push_str("\n… truncated");
    }

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_delivery(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let detected: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bot_event_log WHERE event_type = 'ALERT_DETECTED' AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&app_context.pool)
    .await?;

    let delivered: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bot_event_log WHERE event_type = 'ALERT_DELIVERED' AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&app_context.pool)
    .await?;

    let failed: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bot_event_log WHERE event_type = 'ALERT_DELIVERY_FAILED' AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&app_context.pool)
    .await?;

    let unique_wallets: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT wallet_masked) FROM bot_event_log WHERE event_type LIKE 'ALERT_%' AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&app_context.pool)
    .await?;

    let text = format!(
        "📬 <b>Alert Delivery Summary</b>\n━━━━━━━━━━━━━━━━━━\n⏱️ Window: <code>Last 24 hours</code>\n🔎 Detected: <code>{}</code>\n✅ Delivered: <code>{}</code>\n❌ Failed: <code>{}</code>\n👛 Wallets: <code>{}</code>",
        detected, delivered, failed, unique_wallets
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_subscribers(
    bot: Bot,
    msg: Message,
    wallet: String,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let clean_wallet = wallet.trim();

    if clean_wallet.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "⚠️ <b>Usage:</b> /subscribers <code>kaspa:wallet</code>"
        );
        return Ok(());
    }

    let rows: Vec<(i64,)> =
        sqlx::query_as("SELECT chat_id FROM user_wallets WHERE wallet = $1 ORDER BY chat_id")
            .bind(clean_wallet)
            .fetch_all(&app_context.pool)
            .await?;

    let mut text = format!(
        "👥 <b>Wallet Subscribers</b>\n━━━━━━━━━━━━━━━━━━\nWallet: <code>{}</code>\nSubscribers: <code>{}</code>\n",
        crate::utils::format_short_wallet(clean_wallet),
        rows.len()
    );

    for (idx, row) in rows.iter().enumerate() {
        text.push_str(&format!("\n{}. <code>{}</code>", idx + 1, row.0));
    }

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_wallet_events(
    bot: Bot,
    msg: Message,
    wallet: String,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let clean_wallet = wallet.trim();

    if clean_wallet.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "⚠️ <b>Usage:</b> /wallet_events <code>kaspa:wallet</code>"
        );
        return Ok(());
    }

    let wallet_masked = crate::utils::format_short_wallet(clean_wallet);

    let rows = sqlx::query(
        r#"
        SELECT created_at, event_type, severity, chat_id, status
        FROM bot_event_log
        WHERE wallet_masked = $1
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .bind(&wallet_masked)
    .fetch_all(&app_context.pool)
    .await?;

    let mut text = format!(
        "👛 <b>Wallet Events</b>\n━━━━━━━━━━━━━━━━━━\nShowing latest <code>10</code> events\nWallet: <code>{}</code>\n",
        wallet_masked
    );

    if rows.is_empty() {
        text.push_str("\nNo events found.");
    }

    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let event_type: String = row.try_get("event_type")?;
        let severity: String = row.try_get("severity")?;
        let chat_id: Option<i64> = row.try_get("chat_id")?;
        let status: Option<String> = row.try_get("status")?;

        text.push_str(&format!(
            "\n{} <b>{}</b>\n⏱️ <code>{}</code>\n",
            if severity == "error" { "🔴" } else { "🟢" },
            event_type,
            created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(status) = status {
            text.push_str(&format!("Status: <code>{}</code>\n", status));
        }
        if let Some(chat_id) = chat_id {
            text.push_str(&format!("Chat: <code>{}</code>\n", chat_id));
        }
    }

    if text.chars().count() > 3400 {
        text = text.chars().take(3400).collect::<String>();
        text.push_str("\n… truncated");
    }

    crate::send_logged!(bot, msg, text);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_cleanup_events(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let retention_days: i64 = std::env::var("BOT_EVENT_LOG_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(60)
        .clamp(1, 365);

    let result = sqlx::query(
        "DELETE FROM bot_event_log
         WHERE created_at < NOW() - ($1::text || ' days')::interval",
    )
    .bind(retention_days.to_string())
    .execute(&app_context.pool)
    .await?;

    let text = format!(
        "🧹 <b>Events Cleanup Complete</b>\n━━━━━━━━━━━━━━━━━━\nRetention: <code>{} days</code>\nDeleted rows: <code>{}</code>",
        retention_days,
        result.rows_affected()
    );

    crate::send_logged!(bot, msg, text);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_mute_alerts(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    crate::wallet::alert_delivery_gate::set_alert_delivery_enabled(&app_context.pool, false)
        .await?;

    let text = "🔕 <b>Alert Delivery Muted</b>\n━━━━━━━━━━━━━━━━━━\nTelegram mining alerts are now <code>DISABLED</code>.\n\nThe bot will continue detecting blocks, analyzing DAG data, updating dedup state, and recording events in the database.";

    crate::send_logged!(bot, msg, text);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_unmute_alerts(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    crate::wallet::alert_delivery_gate::set_alert_delivery_enabled(&app_context.pool, true).await?;

    let text = "🔔 <b>Alert Delivery Resumed</b>\n━━━━━━━━━━━━━━━━━━\nTelegram mining alerts are now <code>ENABLED</code>.\n\nOnly new alerts after this point will be sent.";

    crate::send_logged!(bot, msg, text);
    Ok(())
}

pub async fn handle_alerts_status(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let text =
        crate::wallet::alert_delivery_gate::alert_delivery_status_text(&app_context.pool).await;

    crate::send_logged!(bot, msg, text);
    Ok(())
}
