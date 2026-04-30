use crate::wallet::wallet_use_cases::*;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn handle_add(
    bot: Bot,
    msg: Message,
    cid: i64,
    wallet: String,
    wallet_mgt: Arc<WalletManagementUseCase>,
) -> anyhow::Result<()> {
    let clean_wallet = wallet.trim();

    if crate::utils::is_add_wallet_rate_limited(cid) {
        crate::send_logged!(bot, msg, crate::utils::rate_limit_message());
        return Ok(());
    }

    if let Err(reason) = crate::utils::validate_wallet_address_size(clean_wallet) {
        crate::send_logged!(bot, msg, format!("🚫 <b>Wallet rejected.</b>\n{}", reason));
        return Ok(());
    }
    if clean_wallet.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "⚠️ <b>Usage:</b> /add <code>kaspa:your_wallet_address</code>"
        );
        return Ok(());
    }

    if !is_valid_kaspa_address(clean_wallet) {
        crate::send_logged!(
            bot,
            msg,
            "🚫 <b>Invalid wallet address.</b>\nPlease send a valid <code>kaspa:...</code> address."
        );
        return Ok(());
    }

    match wallet_mgt.add_wallet(clean_wallet, cid).await {
        Ok(_) => {
            crate::send_logged!(
                bot,
                msg,
                format!(
                    "✅ <b>Wallet Added</b>\nNow tracking:\n<code>{}</code>",
                    clean_wallet
                )
            );
        }
        Err(e) => {
            crate::send_logged!(bot, msg, format!("❌ <b>Error:</b> {}", e));
        }
    }

    Ok(())
}

pub async fn handle_remove(
    bot: Bot,
    msg: Message,
    cid: i64,
    wallet: String,
    wallet_mgt: Arc<WalletManagementUseCase>,
) -> anyhow::Result<()> {
    let clean_wallet = wallet.trim();

    if let Err(reason) = crate::utils::validate_wallet_address_size(clean_wallet) {
        crate::send_logged!(bot, msg, format!("🚫 <b>Wallet rejected.</b>\n{}", reason));
        return Ok(());
    }
    if !is_valid_kaspa_address(clean_wallet) {
        crate::send_logged!(
            bot,
            msg,
            "🚫 <b>Invalid wallet address.</b>\nPlease send a valid <code>kaspa:...</code> address."
        );
        return Ok(());
    }

    match wallet_mgt.remove_wallet(clean_wallet, cid).await {
        Ok(_) => {
            crate::send_logged!(bot, msg, "🗑️ <b>Wallet Removed.</b>");
        }
        Err(e) => {
            crate::send_logged!(bot, msg, format!("❌ <b>Error:</b> {}", e));
        }
    }

    Ok(())
}

pub async fn handle_list(
    bot: Bot,
    msg: Message,
    cid: i64,
    wallet_query: Arc<WalletQueriesUseCase>,
) -> anyhow::Result<()> {
    let wallets = match wallet_query.get_list(cid).await {
        Ok(wallets) => wallets,
        Err(e) => {
            crate::send_logged!(bot, msg, format!("❌ {}", e));
            return Ok(());
        }
    };

    if wallets.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "📭 <b>No tracked wallets.</b>\nUse /add or press Add Wallet from the menu."
        );
        return Ok(());
    }

    let text = wallet_list_text(&wallets);
    let markup = wallet_buttons_markup(&wallets, "wallet_panel", true);

    let _ = crate::utils::send_logged_message(&bot, msg.chat.id, Some(msg.id), text, Some(markup))
        .await;

    Ok(())
}

pub async fn handle_balance(
    bot: Bot,
    msg: Message,
    cid: i64,
    wallet_query: Arc<WalletQueriesUseCase>,
    _app_context: Arc<crate::domain::models::AppContext>,
) -> anyhow::Result<()> {
    let wallet_details = match wallet_query.get_wallet_balances(cid).await {
        Ok(details) => details,
        Err(e) => {
            crate::send_logged!(bot, msg, format!("❌ Error: {}", e));
            return Ok(());
        }
    };

    if wallet_details.is_empty() {
        crate::send_logged!(
            bot,
            msg,
            "📭 <b>No tracked wallets.</b>\nUse /add or press Add Wallet from the menu."
        );
        return Ok(());
    }

    let mut kas_price = 0.0;
    if let Ok(response) = reqwest::get("https://api.kaspa.org/info/price").await {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            kas_price = json["price"].as_f64().unwrap_or(0.0);
        }
    }

    let total_sompi: u64 = wallet_details.iter().map(|w| w.balance_sompi).sum();
    let total_utxos: usize = wallet_details.iter().map(|w| w.utxos).sum();
    let total_kas = total_sompi as f64 / 1e8;
    let total_value = total_kas * kas_price;
    let avg_utxo = if total_utxos > 0 {
        total_kas / total_utxos as f64
    } else {
        0.0
    };

    let wallets: Vec<String> = wallet_details.iter().map(|w| w.address.clone()).collect();

    let text = format!(
        "💰 <b>Wallet Analytics</b>\n\
         Community Mining Alerts\n\
         ━━━━━━━━━━━━━━━━━━\n\
         👛 <b>Tracked Wallets:</b> <code>{}</code>\n\
         💵 <b>Total Balance:</b> <code>{:.2} KAS</code>\n\
         💲 <b>Total Value:</b> <code>${:.2} USD</code>\n\
         🔄 <b>Total UTXOs:</b> <code>{}</code>\n\
         📊 <b>Average UTXO:</b> <code>{:.2} KAS</code>\n\n\
         Select a wallet below to view detailed balance.\n\n\
         ⏱️ <code>{}</code>",
        wallet_details.len(),
        total_kas,
        total_value,
        total_utxos,
        avg_utxo,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    let markup = wallet_buttons_markup(&wallets, "wallet_balance", true);

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

pub async fn handle_wallet_panel(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    wallet_query: Arc<WalletQueriesUseCase>,
) -> anyhow::Result<()> {
    let wallets = wallet_query.get_list(cid).await.unwrap_or_default();

    let Some(address) = wallets.get(index) else {
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

    let text = format!(
        "{} <b>Wallet {}</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         <code>{}</code>\n\n\
         Choose what you want to view.",
        wallet_number_emoji(index + 1),
        index + 1,
        address
    );

    edit_text(&bot, chat_id, message_id, text, wallet_panel_markup(index)).await;
    Ok(())
}

pub async fn handle_wallet_balance_detail(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    wallet_query: Arc<WalletQueriesUseCase>,
) -> anyhow::Result<()> {
    let details = wallet_query
        .get_wallet_balances(cid)
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

    let mut kas_price = 0.0;
    if let Ok(response) = reqwest::get("https://api.kaspa.org/info/price").await {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            kas_price = json["price"].as_f64().unwrap_or(0.0);
        }
    }

    let balance_kas = detail.balance_sompi as f64 / 1e8;
    let fiat_value = balance_kas * kas_price;
    let avg_utxo = if detail.utxos > 0 {
        balance_kas / detail.utxos as f64
    } else {
        0.0
    };

    let status = if detail.is_online {
        "Online 🟢"
    } else {
        "Unavailable 🔴"
    };

    let text = format!(
        "💰 <b>Wallet {} Balance</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         <code>{}</code>\n\n\
         💵 <b>Balance:</b> <code>{:.2} KAS</code>\n\
         💲 <b>Value:</b> <code>${:.2} USD</code>\n\
         🔄 <b>UTXOs:</b> <code>{}</code>\n\
         📊 <b>Average UTXO:</b> <code>{:.2} KAS</code>\n\
         🩺 <b>Status:</b> <code>{}</code>\n\n\
         ⏱️ <code>{}</code>",
        index + 1,
        detail.address,
        balance_kas,
        fiat_value,
        detail.utxos,
        avg_utxo,
        status,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    edit_text(&bot, chat_id, message_id, text, wallet_panel_markup(index)).await;
    Ok(())
}

pub async fn handle_wallet_remove_confirm(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    wallet_query: Arc<WalletQueriesUseCase>,
) -> anyhow::Result<()> {
    let wallets = wallet_query.get_list(cid).await.unwrap_or_default();

    let Some(address) = wallets.get(index) else {
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

    let text = format!(
        "⚠️ <b>Confirm Remove Wallet {}</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         <code>{}</code>\n\n\
         Are you sure you want to remove this wallet?",
        index + 1,
        address
    );

    edit_text(
        &bot,
        chat_id,
        message_id,
        text,
        confirm_remove_markup(index),
    )
    .await;
    Ok(())
}

pub async fn handle_wallet_remove_do(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    cid: i64,
    index: usize,
    wallet_query: Arc<WalletQueriesUseCase>,
    wallet_mgt: Arc<WalletManagementUseCase>,
) -> anyhow::Result<()> {
    let wallets = wallet_query.get_list(cid).await.unwrap_or_default();

    let Some(address) = wallets.get(index) else {
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

    if let Err(e) = wallet_mgt.remove_wallet(address, cid).await {
        tracing::error!("[DATABASE ERROR] Failed to remove wallet: {}", e);
    }

    let text = format!(
        "🗑️ <b>Wallet Removed</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         Wallet {} was removed.",
        index + 1
    );

    edit_text(
        &bot,
        chat_id,
        message_id,
        text,
        crate::presentation::telegram::menus::TelegramMenus::main_menu_markup(),
    )
    .await;

    Ok(())
}

pub fn wallet_buttons_markup(
    wallets: &[String],
    callback_prefix: &str,
    include_main_menu: bool,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for (index, wallet) in wallets.iter().enumerate() {
        rows.push(vec![InlineKeyboardButton::callback(
            format!(
                "{} Wallet {} - {}",
                wallet_number_emoji(index + 1),
                index + 1,
                crate::utils::format_short_wallet(wallet)
            ),
            format!("{}_{}", callback_prefix, index),
        )]);
    }

    if include_main_menu {
        rows.push(vec![InlineKeyboardButton::callback(
            "🔙 Main Menu",
            "cmd_start",
        )]);
    }

    InlineKeyboardMarkup::new(rows)
}

pub fn wallet_panel_markup(index: usize) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("💰 Balance", format!("wallet_balance_{}", index)),
            InlineKeyboardButton::callback("🧱 Blocks", format!("wallet_blocks_{}", index)),
        ],
        vec![
            InlineKeyboardButton::callback("⛏️ Miner", format!("wallet_miner_{}", index)),
            InlineKeyboardButton::callback("➖ Remove", format!("wallet_remove_confirm_{}", index)),
        ],
        vec![
            InlineKeyboardButton::callback("👛 All Wallets", "cmd_wallets"),
            InlineKeyboardButton::callback("🔙 Main Menu", "cmd_start"),
        ],
    ])
}

fn confirm_remove_markup(index: usize) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                "✅ Yes, remove wallet",
                format!("wallet_remove_do_{}", index),
            ),
            InlineKeyboardButton::callback("❌ Cancel", format!("wallet_panel_{}", index)),
        ],
        vec![InlineKeyboardButton::callback("🔙 Main Menu", "cmd_start")],
    ])
}

fn wallet_list_text(wallets: &[String]) -> String {
    let list = wallets
        .iter()
        .enumerate()
        .map(|(index, wallet)| {
            format!(
                "{} <b>Wallet {}:</b> <code>{}</code>",
                wallet_number_emoji(index + 1),
                index + 1,
                crate::utils::format_short_wallet(wallet)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "👛 <b>Tracked Wallets</b>\n\
         ━━━━━━━━━━━━━━━━━━\n\
         {}\n\n\
         Select a wallet below to open its panel.",
        list
    )
}

fn wallet_number_emoji(number: usize) -> &'static str {
    match number {
        1 => "1️⃣",
        2 => "2️⃣",
        3 => "3️⃣",
        4 => "4️⃣",
        5 => "5️⃣",
        6 => "6️⃣",
        7 => "7️⃣",
        8 => "8️⃣",
        9 => "9️⃣",
        10 => "🔟",
        _ => "🔹",
    }
}

async fn edit_text(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    text: String,
    markup: InlineKeyboardMarkup,
) {
    let _ = crate::utils::edit_logged_message(bot, chat_id, message_id, text, Some(markup)).await;
}

fn is_valid_kaspa_address(address: &str) -> bool {
    address.starts_with("kaspa:")
        && address.len() > 20
        && address.chars().skip(6).all(|c| c.is_ascii_alphanumeric())
}
