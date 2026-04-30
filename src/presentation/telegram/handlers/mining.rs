use crate::domain::models::AppContext;
use crate::network::stats_use_cases::GetMinerStatsUseCase;
use crate::wallet::wallet_use_cases::WalletQueriesUseCase;
use std::sync::Arc;
use teloxide::prelude::*;

pub async fn handle_blocks(
    bot: Bot,
    msg: Message,
    cid: i64,
    wallet_query: Arc<WalletQueriesUseCase>,
    _app_context: Arc<AppContext>,
) -> anyhow::Result<()> {
    let details = match wallet_query.get_wallet_blocks_details(cid).await {
        Ok(details) => details,
        Err(e) => {
            crate::send_logged!(bot, msg, format!("❌ Error: {}", e));
            return Ok(());
        }
    };

    if details.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "📭 <b>No tracked wallets.</b>\nUse /add or press Add Wallet from the menu."
        );
        return Ok(());
    }

    let total_1h: i64 = details.iter().map(|w| w.blocks_1h).sum();
    let total_24h: i64 = details.iter().map(|w| w.blocks_24h).sum();
    let total_7d: i64 = details.iter().map(|w| w.blocks_7d).sum();
    let total_lifetime: i64 = details.iter().map(|w| w.lifetime_blocks).sum();

    let status = if total_1h > 0 {
        "Active 🟢"
    } else {
        "Idle 🟡"
    };

    let wallets: Vec<String> = details.iter().map(|w| w.address.clone()).collect();

    let text = format!(
        "🧱 <b>Mined Blocks</b>\n\
         Community Mining Alerts\n\
         ━━━━━━━━━━━━━━━━━━\n\
         👛 <b>Tracked Wallets:</b> <code>{}</code>\n\
         ⏱️ <b>Total Last 1 Hour:</b> <code>{}</code>\n\
         ⏳ <b>Total Last 24 Hours:</b> <code>{}</code>\n\
         📆 <b>Total Last 7 Days:</b> <code>{}</code>\n\
         🏆 <b>Total Lifetime Blocks:</b> <code>{}</code>\n\
         📈 <b>Mining Status:</b> {}\n\n\
         Select a wallet below to view detailed block stats.\n\n\
         ⏱️ <code>{}</code>",
        details.len(),
        total_1h,
        total_24h,
        total_7d,
        total_lifetime,
        status,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    let markup = crate::presentation::telegram::handlers::wallet::wallet_buttons_markup(
        &wallets,
        "wallet_blocks",
        true,
    );

    let _ = crate::utils::send_reply_or_edit_log(
        &bot,
        msg.chat.id,
        msg.id,
        msg.from.as_ref().filter(|u| u.is_bot).map(|_| msg.id),
        text,
        Some(markup),
    )
    .await;

    Ok(())
}

pub async fn handle_miner(
    bot: Bot,
    msg: Message,
    cid: i64,
    app_context: Arc<AppContext>,
    miner_stats: Arc<GetMinerStatsUseCase>,
) -> anyhow::Result<()> {
    let tracked: Vec<String> =
        sqlx::query_scalar("SELECT wallet FROM user_wallets WHERE chat_id = $1")
            .bind(cid)
            .fetch_all(&app_context.pool)
            .await
            .unwrap_or_default();

    if tracked.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "📭 <b>No tracked wallets.</b>\nUse /add or press Add Wallet from the menu."
        );
        return Ok(());
    }

    let mut global_hashrate = "Unknown".to_string();

    if let Some(first_wallet) = tracked.first() {
        if let Ok(stats) = miner_stats.execute(first_wallet).await {
            global_hashrate = stats.global_network_hashrate;
        }
    }

    let text = format!(
        "⛏️ <b>Solo-Miner Hashrate</b>\n\
         Community Mining Alerts\n\
         ━━━━━━━━━━━━━━━━━━\n\
         👛 <b>Tracked Wallets:</b> <code>{}</code>\n\
         🌐 <b>Global Hashrate:</b> <code>{}</code>\n\n\
         Select a wallet below to view detailed miner hashrate.\n\n\
         ⏱️ <code>{}</code>",
        tracked.len(),
        global_hashrate,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    let markup = crate::presentation::telegram::handlers::wallet::wallet_buttons_markup(
        &tracked,
        "wallet_miner",
        true,
    );

    let _ = crate::utils::send_reply_or_edit_log(
        &bot,
        msg.chat.id,
        msg.id,
        msg.from.as_ref().filter(|u| u.is_bot).map(|_| msg.id),
        text,
        Some(markup),
    )
    .await;

    Ok(())
}

pub async fn handle_wallet_blocks_detail(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    wallet_query: Arc<WalletQueriesUseCase>,
) -> anyhow::Result<()> {
    let details = wallet_query
        .get_wallet_blocks_details(cid)
        .await
        .unwrap_or_default();

    let Some(detail) = details.get(index) else {
        edit_text(
            &bot,
            chat_id,
            message_id,
            "⚠️ Wallet not found.".to_string(),
            crate::presentation::telegram::menus::TelegramMenus::main_menu_markup(),
        )
        .await;
        return Ok(());
    };

    let status = if detail.blocks_1h > 0 {
        "Active 🟢"
    } else {
        "Idle 🟡"
    };

    let mut daily_text = String::new();

    if !detail.daily_blocks.is_empty() {
        daily_text.push_str("📅 <b>Last 7 Days:</b>\n");
        for (day, count) in detail.daily_blocks.iter().take(7) {
            daily_text.push_str(&format!("├ <code>{}</code>: {} blocks\n", day, count));
        }
    } else {
        daily_text.push_str("📅 <b>Last 7 Days:</b> <code>No blocks</code>\n");
    }

    let text = format!(
        "🧱 <b>Wallet {} Blocks</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         <code>{}</code>\n\n\
         ⏱️ <b>Last 1 Hour:</b> <code>{}</code>\n\
         ⏳ <b>Last 24 Hours:</b> <code>{}</code>\n\
         📆 <b>Last 7 Days:</b> <code>{}</code>\n\
         🏆 <b>Lifetime Blocks:</b> <code>{}</code>\n\
         📈 <b>Status:</b> {}\n\
         {}\n\
         ⏱️ <code>{}</code>",
        index + 1,
        detail.address,
        detail.blocks_1h,
        detail.blocks_24h,
        detail.blocks_7d,
        detail.lifetime_blocks,
        status,
        daily_text,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    edit_text(
        &bot,
        chat_id,
        message_id,
        text,
        crate::presentation::telegram::handlers::wallet::wallet_panel_markup(index),
    )
    .await;

    Ok(())
}

pub async fn handle_wallet_miner_detail(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    app_context: Arc<AppContext>,
    miner_stats: Arc<GetMinerStatsUseCase>,
) -> anyhow::Result<()> {
    let tracked: Vec<String> =
        sqlx::query_scalar("SELECT wallet FROM user_wallets WHERE chat_id = $1")
            .bind(cid)
            .fetch_all(&app_context.pool)
            .await
            .unwrap_or_default();

    let Some(wallet) = tracked.get(index) else {
        edit_text(
            &bot,
            chat_id,
            message_id,
            "⚠️ Wallet not found.".to_string(),
            crate::presentation::telegram::menus::TelegramMenus::main_menu_markup(),
        )
        .await;
        return Ok(());
    };

    let text = match miner_stats.execute(wallet).await {
        Ok(stats) => format!(
            "⛏️ <b>Wallet {} Miner</b>\n\
             ━━━━━━━━━━━━━━━━━━\n\
             <code>{}</code>\n\n\
             🌐 <b>Global Hashrate:</b> <code>{}</code>\n\
             📊 <b>Actual Hashrate:</b>\n\
             ├ 1H: <code>{}</code>\n\
             ├ 24H: <code>{}</code>\n\
             └ 7D: <code>{}</code>\n\n\
             ⚡ <b>Unspent Hashrate:</b>\n\
             ├ 1H: <code>{}</code>\n\
             ├ 24H: <code>{}</code>\n\
             └ 7D: <code>{}</code>\n\n\
             ⏱️ <code>{}</code>",
            index + 1,
            wallet,
            stats.global_network_hashrate,
            stats.actual_hashrate_1h,
            stats.actual_hashrate_24h,
            stats.actual_hashrate_7d,
            stats.unspent_hashrate_1h,
            stats.unspent_hashrate_24h,
            stats.unspent_hashrate_7d,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
        Err(e) => format!("❌ <b>Error fetching miner stats:</b> {}", e),
    };

    edit_text(
        &bot,
        chat_id,
        message_id,
        text,
        crate::presentation::telegram::handlers::wallet::wallet_panel_markup(index),
    )
    .await;

    Ok(())
}

async fn edit_text(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    text: String,
    markup: teloxide::types::InlineKeyboardMarkup,
) {
    let _ = crate::utils::edit_logged_message(bot, chat_id, message_id, text, Some(markup)).await;
}
