use kaspa_addresses::Address;
use std::collections::HashSet;
use teloxide::{prelude::*, types::ChatId};

use crate::context::AppContext;
use super::is_local_node;

pub async fn handle_block_user(update: teloxide::types::ChatMemberUpdated, ctx: AppContext) -> anyhow::Result<()> {
    if update.new_chat_member.is_banned() || update.new_chat_member.is_left() {
        crate::state::remove_all_user_data(&ctx.pool, &ctx.state, update.chat.id.0).await;
    }
    Ok(())
}

pub async fn fallback_heuristic_text(bot: Bot, chat_id: ChatId, raw_text: &str, ctx: AppContext) -> anyhow::Result<()> {
    let lower_text = raw_text.to_lowercase();
    if lower_text.starts_with("kaspa:") || (lower_text.starts_with('q') && lower_text.len() >= 60) {
        let clean_address = if lower_text.starts_with("kaspa:") { raw_text.to_string() } else { format!("kaspa:{}", raw_text) };
        if Address::try_from(clean_address.as_str()).is_ok() {
            ctx.state.entry(clean_address.clone()).or_insert_with(HashSet::new).insert(chat_id.0);
            crate::state::add_wallet_to_db(&ctx.pool, &clean_address, chat_id.0).await;
            
            let is_local = is_local_node();
            let sync_status_msg = if is_local { "\n\n🔄 <i>Syncing missed blocks...</i>" } else { "\n\n⚠️ <i>Live tracking active. (Historical sync disabled on public nodes)</i>" };
            
            let _ = bot.send_message(chat_id, format!("⚡ <b>Smart Auto-Add Activated!</b>\n✅ Now tracking:\n<code>{}</code>{}", clean_address, sync_status_msg)).parse_mode(teloxide::types::ParseMode::Html).await;
            if is_local {
                let ctx_c = ctx.clone();
                let wallet_c = clean_address.clone();
                tokio::spawn(async move { let _ = crate::workers::sync_single_wallet(ctx_c, wallet_c).await; });
            }
        }
        return Ok(());
    }

    if raw_text.starts_with('/') {
        let known_commands = vec!["/start", "/help", "/add", "/remove", "/list", "/balance", "/blocks", "/miner", "/network", "/dag", "/price", "/market", "/supply", "/fees", "/donate"];
        for cmd in known_commands {
            if strsim::levenshtein(&lower_text, cmd) <= 2 && lower_text.len() > 2 {
                let _ = bot.send_message(chat_id, format!("🤖 <b>Command not found.</b>\nDid you mean {} ?", cmd)).parse_mode(teloxide::types::ParseMode::Html).await;
                return Ok(());
            }
        }
    }

    let response = if lower_text.contains("balance") || lower_text.contains("funds") { "💰 Tap /balance to view your live node data." }
    else if lower_text.contains("hashrate") || lower_text.contains("speed") { "⛏️ Tap /miner to estimate your solo hashrate." }
    else if lower_text.contains("block") || lower_text.contains("mined") { "🧱 Tap /blocks to view mined blocks." }
    else { "🤖 <b>Unrecognized Input.</b> Press /start for the menu." };
    
    let _ = bot.send_message(chat_id, response).parse_mode(teloxide::types::ParseMode::Html).await;
    Ok(())
}

pub async fn handle_raw_message_v2(bot: Bot, msg: Message, ctx: AppContext) -> anyhow::Result<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    if user_id != ctx.admin_id && ctx.rate_limiter.check_key(&user_id).is_err() {
        tracing::warn!("[SECURITY] Spam blocked for User: {}", user_id);
        let _ = bot.send_message(msg.chat.id, "🛑 <b>Rate Limit Exceeded!</b>").parse_mode(teloxide::types::ParseMode::Html).await;
        return Ok(());
    }

    if msg.voice().is_some() { return crate::ai::process_voice_message(bot, msg, ctx).await; }

    if let Some(text) = msg.text() {
        let raw_text = text.trim();
        let lower_text = raw_text.to_lowercase();
        if raw_text.starts_with('/') || lower_text.starts_with("kaspa:") || (lower_text.starts_with('q') && lower_text.len() >= 60) {
            return fallback_heuristic_text(bot, msg.chat.id, raw_text, ctx).await;
        }
        return crate::ai::process_conversational_intent(bot, msg.chat.id, msg.id, user_id, raw_text.to_string(), ctx).await;
    }
    Ok(())
}
