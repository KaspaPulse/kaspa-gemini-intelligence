use crate::domain::models::AppContext;
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::wallet::wallet_use_cases::WalletManagementUseCase;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use teloxide::prelude::*;

pub async fn handle_raw_message(
    bot: Bot,
    msg: Message,
    app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let cid = msg.chat.id.0;

    if app_context.maintenance_mode.load(Ordering::Relaxed) && cid != app_context.admin_id {
        return Ok(());
    }

    if let Some((_, pending_cmd)) = app_context.admin_sessions.remove(&cid) {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;

        if cid == app_context.admin_id {
            if pending_cmd.starts_with("TOGGLE:") {
                let flag = pending_cmd.split(':').nth(1).unwrap_or("").to_string();
                let _ = crate::presentation::telegram::handlers::admin::handle_toggle(
                    bot,
                    msg,
                    flag,
                    app_context,
                )
                .await;
            } else {
                match pending_cmd.as_str() {
                    "PAUSE" => {
                        let _ = crate::presentation::telegram::handlers::admin::handle_pause(
                            bot,
                            msg,
                            app_context,
                        )
                        .await;
                    }
                    "RESUME" => {
                        let _ = crate::presentation::telegram::handlers::admin::handle_resume(
                            bot,
                            msg,
                            app_context,
                        )
                        .await;
                    }
                    "RESTART" => {
                        let _ = crate::presentation::telegram::handlers::admin::handle_restart(
                            bot, msg,
                        )
                        .await;
                    }
                    _ => {}
                }
            }
        } else {
            let _ = bot
                .send_message(msg.chat.id, "Access denied. Admin session terminated.")
                .await;
        }

        return Ok(());
    }

    let raw_text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    if let Err(reason) = crate::utils::validate_raw_message_size(raw_text) {
        crate::send_logged!(bot, msg, format!("🚫 <b>Message rejected.</b>\n{}", reason));
        return Ok(());
    }

    let clean_text = crate::utils::sanitize_user_text(raw_text);

    if clean_text.starts_with('/') {
        return Ok(());
    }

    let wallet_address = clean_text
        .split_whitespace()
        .find(|part| part.starts_with("kaspa:") || part.starts_with("kaspatest:"))
        .map(|s| s.to_string());

    if let Some(addr) = wallet_address {
        let db = Arc::new(PostgresRepository::new(app_context.pool.clone()));
        let wallet_mgt = Arc::new(WalletManagementUseCase::new(db));

        crate::presentation::telegram::handlers::wallet::handle_add(
            bot, msg, cid, addr, wallet_mgt,
        )
        .await?;
    }

    Ok(())
}
