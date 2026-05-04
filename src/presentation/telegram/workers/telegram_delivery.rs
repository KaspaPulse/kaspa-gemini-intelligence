use sqlx::PgPool;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{ChatId, LinkPreviewOptions, ParseMode};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub fn start_telegram_delivery_worker(bot: Bot, pool: PgPool, token: CancellationToken) {
    crate::infrastructure::resilience::runtime::spawn_resilient(
        "telegram_delivery_worker",
        async move {
            info!("📬 [WORKER] Telegram delivery worker started.");

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        info!("[WORKER] Telegram delivery worker shutdown requested.");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                }

                if !crate::infrastructure::telegram_delivery_queue::delivery_queue_enabled() {
                    continue;
                }

                let batch =
                    match crate::infrastructure::telegram_delivery_queue::fetch_pending_batch(
                        &pool, 25,
                    )
                    .await
                    {
                        Ok(batch) => batch,
                        Err(e) => {
                            crate::infrastructure::metrics::inc_db_errors();
                            error!("[DELIVERY QUEUE] Failed to fetch pending messages: {}", e);
                            continue;
                        }
                    };

                if batch.is_empty() {
                    continue;
                }

                for item in batch {
                    crate::utils::log_multiline(
                        &format!("📤 [BOT QUEUE OUT] Chat: {}", item.chat_id),
                        &item.message_html,
                        true,
                    );

                    let send_result = bot
                        .send_message(ChatId(item.chat_id), &item.message_html)
                        .parse_mode(ParseMode::Html)
                        .link_preview_options(LinkPreviewOptions {
                            url: None,
                            is_disabled: true,
                            show_above_text: false,
                            prefer_small_media: false,
                            prefer_large_media: false,
                        })
                        .await;

                    match send_result {
                        Ok(_) => {
                            crate::infrastructure::metrics::inc_alerts_delivered();

                            if let Err(e) =
                                crate::infrastructure::telegram_delivery_queue::mark_sent(
                                    &pool, item.id,
                                )
                                .await
                            {
                                crate::infrastructure::metrics::inc_db_errors();
                                error!(
                                    "[DELIVERY QUEUE] Failed to mark sent id {}: {}",
                                    item.id, e
                                );
                            }

                            info!(
                                "✅ [QUEUED ALERT DELIVERED] id={} | chat={} | wallet={} | txid={} | block={} | amount_kas={} | daa_score={}",
                                item.id,
                                item.chat_id,
                                item.wallet_masked.as_deref().unwrap_or("unknown"),
                                item.txid_masked.as_deref().unwrap_or("unknown"),
                                item.block_hash_masked.as_deref().unwrap_or("unknown"),
                                item.amount_kas
                                    .map(|value| format!("{:.8}", value))
                                    .unwrap_or_else(|| "unknown".to_string()),
                                item.daa_score
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".to_string())
                            );
                        }
                        Err(e) => {
                            crate::infrastructure::metrics::inc_telegram_send_failures();

                            let err_text = e.to_string();

                            if let Err(db_error) =
                                crate::infrastructure::telegram_delivery_queue::mark_failed(
                                    &pool, item.id, &err_text,
                                )
                                .await
                            {
                                crate::infrastructure::metrics::inc_db_errors();
                                error!(
                                    "[DELIVERY QUEUE] Failed to mark failed id {}: {}",
                                    item.id, db_error
                                );
                            }

                            error!(
                                "[TELEGRAM ERROR] Queued alert send failed. id={} chat={} error={}",
                                item.id, item.chat_id, err_text
                            );
                        }
                    }
                }
            }
        },
    );
}
