use crate::domain::models::{BotEventRecord, BotEventType, EventSeverity};
use crate::infrastructure::database::postgres_adapter::PostgresRepository;
use crate::infrastructure::node::kaspa_adapter::KaspaRpcAdapter;

mod application;
mod config;
mod domain;
mod infrastructure;
mod network;
mod presentation;
mod wallet;

pub mod utils;

use dotenvy::dotenv;
use std::env;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::dptree;
use teloxide::prelude::*;
use teloxide::types::BotCommandScope;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};

use crate::infrastructure::market::coingecko_adapter::CoinGeckoAdapter;
use crate::network::analyze_dag::AnalyzeDagUseCase;
use crate::network::stats_use_cases::GetMinerStatsUseCase;
use crate::network::stats_use_cases::NetworkStatsUseCase;
use crate::presentation::telegram::commands::Command;
use crate::wallet::wallet_use_cases::WalletManagementUseCase;
use crate::wallet::wallet_use_cases::WalletQueriesUseCase;

fn panic_event_marker_path() -> PathBuf {
    env::var("PANIC_EVENT_MARKER_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("panic_event_pending.json"))
}

fn truncate_panic_text(value: &str, max_chars: usize) -> String {
    let mut text = value
        .replace('\u{0000}', "")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string();

    if text.chars().count() > max_chars {
        text = text.chars().take(max_chars).collect::<String>();
        text.push_str("...[truncated]");
    }

    text
}

fn write_pending_panic_marker(location: &str, message: &str) {
    let marker_path = panic_event_marker_path();

    let payload = serde_json::json!({
        "event_type": "PANIC_EVENT",
        "status": "pending_recovery",
        "location": truncate_panic_text(location, 300),
        "message": truncate_panic_text(message, 1000),
        "created_at": chrono::Utc::now().to_rfc3339(),
        "pid": std::process::id()
    });

    if let Err(e) = fs::write(&marker_path, payload.to_string()) {
        tracing::error!(
            "[PANIC_EVENT] Failed to write pending panic marker at {:?}: {}",
            marker_path,
            e
        );
    }
}

async fn record_pending_panic_marker(
    db_repo: &Arc<PostgresRepository>,
) -> Result<(), crate::domain::errors::AppError> {
    let marker_path = panic_event_marker_path();

    if !marker_path.exists() {
        return Ok(());
    }

    let marker_content = match fs::read_to_string(&marker_path) {
        Ok(content) => content,
        Err(e) => {
            tracing::error!(
                "[PANIC_EVENT] Failed to read pending panic marker at {:?}: {}",
                marker_path,
                e
            );
            return Ok(());
        }
    };

    let marker_json: serde_json::Value =
        serde_json::from_str(&marker_content).unwrap_or_else(|_| {
            serde_json::json!({
                "event_type": "PANIC_EVENT",
                "status": "pending_recovery",
                "message": truncate_panic_text(&marker_content, 1000)
            })
        });

    let message = marker_json
        .get("message")
        .and_then(|value| value.as_str())
        .unwrap_or("panic marker recovered");

    let metadata_json = marker_json.to_string();

    db_repo
        .record_bot_event_typed(
            BotEventType::PanicEvent,
            EventSeverity::Error,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("recovered_after_restart"),
            Some(message),
            None,
            &metadata_json,
        )
        .await?;

    if let Err(e) = fs::remove_file(&marker_path) {
        tracing::warn!(
            "[PANIC_EVENT] Failed to remove pending panic marker at {:?}: {}",
            marker_path,
            e
        );
    } else {
        tracing::info!("[PANIC_EVENT] Recovered pending panic marker into bot_event_log.");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    registry().with(fmt::layer()).with(filter).init();

    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown".to_string());

        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<String>()
                    .map(|value| value.to_string())
            })
            .unwrap_or_else(|| "unknown panic payload".to_string());

        write_pending_panic_marker(&location, &message);

        tracing::error!(
            event_type = "PANIC_EVENT",
            location = %location,
            message = %message,
            "panic captured by global panic hook"
        );
    }));

    info!("Kaspa Pulse starting.");

    let db_url = env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set in .env"))?;
    let rpc_url =
        env::var("NODE_URL_01").map_err(|_| anyhow::anyhow!("NODE_URL_01 must be set in .env"))?;

    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    let db_max_connections: u32 = env::var("DB_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10)
        .clamp(2, 50);

    let verbose_logs = env::var("ENABLE_VERBOSE_LOGS")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    tracing::info!(
        "[SYSTEM] Environment: {} | DB max connections: {} | Verbose logs: {}",
        app_env,
        db_max_connections,
        verbose_logs
    );

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(db_max_connections)
        .connect(&db_url)
        .await?;

    let db_repo = Arc::new(PostgresRepository::new(pool.clone()));

    let allow_runtime_schema_ensure = env::var("ALLOW_RUNTIME_SCHEMA_ENSURE")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    if allow_runtime_schema_ensure {
        tracing::warn!(
            "[DATABASE] Runtime schema ensure is enabled. Prefer applying migrations before startup."
        );

        db_repo
            .ensure_pending_rewards_table()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to ensure pending_rewards table: {}", e))?;
    } else {
        tracing::info!(
            "[DATABASE] Runtime schema ensure disabled. Database schema is expected to be managed by migrations."
        );
    }
    if let Err(e) = record_pending_panic_marker(&db_repo).await {
        tracing::error!("[PANIC_EVENT] Failed to record pending panic marker: {}", e);
    }
    let mut system_start_event =
        BotEventRecord::new(BotEventType::SystemStart, EventSeverity::Info);
    system_start_event.status = Some("ok");

    let _ = db_repo.record_bot_event_record(system_start_event).await;

    let network_id = match kaspa_consensus_core::network::NetworkId::from_str("mainnet") {
        Ok(network_id) => network_id,
        Err(mainnet_error) => {
            match kaspa_consensus_core::network::NetworkId::from_str("testnet-12") {
                Ok(network_id) => network_id,
                Err(testnet_error) => {
                    return Err(anyhow::anyhow!(
                        "failed to parse fallback network ids: mainnet={mainnet_error}; testnet-12={testnet_error}"
                    ));
                }
            }
        }
    };

    let rpc_client = kaspa_wrpc_client::KaspaRpcClient::new(
        kaspa_wrpc_client::WrpcEncoding::SerdeJson,
        Some(&rpc_url),
        None,
        Some(network_id),
        None,
    )
    .map_err(|e| anyhow::anyhow!("RPC Connection Failed: {}", e))?;

    let rpc_client_arc = Arc::new(rpc_client);
    let node_provider = Arc::new(KaspaRpcAdapter::new(rpc_client_arc.clone()));

    tracing::info!("[SYSTEM] Running node pre-flight diagnostic with safe timeouts.");
    let preflight = tokio::time::timeout(
        crate::infrastructure::resilience::runtime::rpc_timeout_duration(),
        async {
            let _ = node_provider.get_server_info().await;
            let _ = node_provider.get_sync_status().await;
            let _ = node_provider.get_block_dag_info().await;
            let _ = node_provider.get_coin_supply().await;
            let _ = node_provider.get_utxos_by_addresses(vec![]).await;
            let _ = node_provider.connect(false).await;
        },
    )
    .await;

    if preflight.is_err() {
        tracing::warn!(
            "[SYSTEM] Node pre-flight timed out. Bot will start in degraded mode and node monitor will keep retrying."
        );
    }

    let market_provider: Arc<dyn crate::infrastructure::market::coingecko_adapter::MarketProvider> =
        Arc::new(CoinGeckoAdapter::new());

    let wallet_management_uc = Arc::new(WalletManagementUseCase::new(db_repo.clone()));

    let wallet_queries_uc = Arc::new(WalletQueriesUseCase::new(
        db_repo.clone(),
        node_provider.clone(),
    ));

    let network_stats_uc = Arc::new(NetworkStatsUseCase::new(node_provider.clone()));
    let dag_uc = Arc::new(AnalyzeDagUseCase::new(node_provider.clone()));

    let get_miner_stats_uc = Arc::new(GetMinerStatsUseCase::new(
        db_repo.clone(),
        node_provider.clone(),
    ));

    let market_stats_uc = Arc::new(crate::network::stats_use_cases::GetMarketStatsUseCase::new(
        node_provider.clone(),
        market_provider.clone(),
    ));

    let bot_token =
        env::var("BOT_TOKEN").map_err(|_| anyhow::anyhow!("BOT_TOKEN must be set in .env"))?;
    let bot = Bot::new(bot_token);
    // Telegram command scopes are synchronized after ADMIN_ID validation.

    let admin_id_raw = env::var("ADMIN_ID")
        .map_err(|_| anyhow::anyhow!("ADMIN_ID must be set in .env for production safety"))?;

    let admin_id: i64 = admin_id_raw
        .parse()
        .map_err(|_| anyhow::anyhow!("ADMIN_ID must be a valid numeric Telegram chat ID"))?;

    if admin_id <= 0 {
        return Err(anyhow::anyhow!(
            "ADMIN_ID must be greater than zero for production safety"
        ));
    }

    // Clear stale Telegram command scopes so deleted legacy commands disappear.
    let _ = bot.delete_my_commands().await;
    let _ = bot
        .delete_my_commands()
        .scope(BotCommandScope::AllPrivateChats)
        .await;
    let _ = bot
        .delete_my_commands()
        .scope(BotCommandScope::AllGroupChats)
        .await;
    let _ = bot
        .delete_my_commands()
        .scope(BotCommandScope::AllChatAdministrators)
        .await;
    let _ = bot
        .delete_my_commands()
        .scope(BotCommandScope::Chat {
            chat_id: teloxide::types::Recipient::Id(ChatId(admin_id)),
        })
        .await;

    // Public commands for all users.
    let _ = bot
        .set_my_commands(crate::presentation::telegram::commands::public_bot_commands())
        .await;

    // Admin commands only in the admin chat.
    let _ = bot
        .set_my_commands(crate::presentation::telegram::commands::admin_bot_commands())
        .scope(BotCommandScope::Chat {
            chat_id: teloxide::types::Recipient::Id(ChatId(admin_id)),
        })
        .await;

    tracing::info!("[SYSTEM] Telegram commands synced.");

    let cancel_token = tokio_util::sync::CancellationToken::new();

    let app_context = std::sync::Arc::new(crate::domain::models::AppContext::new(
        rpc_client_arc.clone(),
        pool.clone(),
        admin_id,
    ));

    {
        let is_mem = db_repo
            .get_setting("ENABLE_MEMORY_CLEANER", "false")
            .await
            .unwrap_or_else(|_| "false".to_string())
            == "true";
        app_context
            .memory_cleaner_enabled
            .store(is_mem, std::sync::atomic::Ordering::Relaxed);

        let is_sync = db_repo
            .get_setting("ENABLE_LIVE_SYNC", "true")
            .await
            .unwrap_or_else(|_| "true".to_string())
            == "true";
        app_context
            .live_sync_enabled
            .store(is_sync, std::sync::atomic::Ordering::Relaxed);

        let is_maint = db_repo
            .get_setting("MAINTENANCE_MODE", "false")
            .await
            .unwrap_or_else(|_| "false".to_string())
            == "true";
        app_context
            .maintenance_mode
            .store(is_maint, std::sync::atomic::Ordering::Relaxed);
    }

    let pool_shutdown = pool.clone();
    let db_shutdown = db_repo.clone();
    let ct_shutdown = cancel_token.clone();

    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(e) => {
                    tracing::error!("[SYSTEM] Failed to install SIGTERM handler: {}", e);
                    let _ = tokio::signal::ctrl_c().await;
                    tracing::warn!("[SYSTEM] SIGINT received. Starting graceful shutdown.");
                    ct_shutdown.cancel();

                    let mut shutdown_event =
                        BotEventRecord::new(BotEventType::SystemShutdown, EventSeverity::Info);
                    shutdown_event.status = Some("ok");
                    shutdown_event.metadata_json = r#"{"reason":"signal"}"#;

                    let _ = db_shutdown.record_bot_event_record(shutdown_event).await;

                    {
                        let shutdown_drain_secs = std::env::var("SHUTDOWN_DRAIN_SECS")
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .filter(|v| *v <= 30)
                            .unwrap_or(3);

                        tracing::info!(
            "[SYSTEM] Waiting {} seconds for background workers to drain before closing database pool.",
            shutdown_drain_secs
        );

                        tokio::time::sleep(std::time::Duration::from_secs(shutdown_drain_secs))
                            .await;
                    }

                    pool_shutdown.close().await;
                    tracing::info!("[SYSTEM] Database connections closed safely.");
                    return;
                }
            };

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::warn!("[SYSTEM] SIGINT received. Starting graceful shutdown.");
                }
                _ = sigterm.recv() => {
                    tracing::warn!("[SYSTEM] SIGTERM received. Starting graceful shutdown.");
                }
            }
        }

        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
            tracing::warn!("[SYSTEM] SIGINT received. Starting graceful shutdown.");
        }

        ct_shutdown.cancel();

        let mut shutdown_event =
            BotEventRecord::new(BotEventType::SystemShutdown, EventSeverity::Info);
        shutdown_event.status = Some("ok");
        shutdown_event.metadata_json = r#"{"reason":"signal"}"#;

        let _ = db_shutdown.record_bot_event_record(shutdown_event).await;

        {
            let shutdown_drain_secs = std::env::var("SHUTDOWN_DRAIN_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v <= 30)
                .unwrap_or(3);

            tracing::info!(
            "[SYSTEM] Waiting {} seconds for background workers to drain before closing database pool.",
            shutdown_drain_secs
        );

            tokio::time::sleep(std::time::Duration::from_secs(shutdown_drain_secs)).await;
        }

        pool_shutdown.close().await;
        tracing::info!("[SYSTEM] Database connections closed safely.");
    });

    crate::presentation::telegram::workers::utxo_monitor::start_utxo_monitor(
        bot.clone(),
        node_provider.clone(),
        db_repo.clone(),
        cancel_token.clone(),
    );

    crate::presentation::telegram::workers::telegram_delivery::start_telegram_delivery_worker(
        bot.clone(),
        app_context.pool.clone(),
        cancel_token.clone(),
    );
    crate::infrastructure::external_services::system::spawn_node_monitor(
        (*app_context).clone(),
        bot.clone(),
        cancel_token.clone(),
    );

    crate::infrastructure::external_services::system::spawn_price_monitor(
        (*app_context).clone(),
        cancel_token.clone(),
    );

    crate::infrastructure::external_services::system::spawn_memory_cleaner(
        (*app_context).clone(),
        cancel_token.clone(),
    );

    let system_tasks_uc =
        Arc::new(crate::application::background_jobs::SystemTasksUseCase::new(db_repo.clone()));

    crate::presentation::telegram::workers::periodic_tasks::start_system_monitors(
        system_tasks_uc.clone(),
        cancel_token.clone(),
    );

    use crate::presentation::telegram::handlers;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handlers::handle_command),
        )
        .branch(Update::filter_callback_query().endpoint(handlers::handle_callback))
        .branch(Update::filter_my_chat_member().endpoint(handlers::handle_block_user))
        .branch(Update::filter_message().endpoint(handlers::handle_raw_message));

    let bot_use_cases = crate::presentation::telegram::handlers::BotUseCases {
        wallet_mgt: wallet_management_uc.clone(),
        wallet_query: wallet_queries_uc.clone(),
        network_stats: network_stats_uc.clone(),
        market_stats: market_stats_uc.clone(),
        miner_stats: get_miner_stats_uc.clone(),
        dag_uc: dag_uc.clone(),
    };

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![
            db_repo.clone(),
            node_provider,
            app_context,
            dag_uc,
            bot_use_cases
        ])
        .enable_ctrlc_handler()
        .build();

    if env::var("USE_WEBHOOK").unwrap_or_else(|_| "false".to_string()) == "true" {
        info!("Running in WEBHOOK mode");

        let domain = env::var("WEBHOOK_DOMAIN")
            .map_err(|_| anyhow::anyhow!("WEBHOOK_DOMAIN must be set when USE_WEBHOOK=true"))?;

        let port: u16 = env::var("WEBHOOK_PORT")
            .unwrap_or_else(|_| "8443".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("WEBHOOK_PORT must be a valid u16 port"))?;

        let bind_ip_raw = env::var("WEBHOOK_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
        let bind_ip: IpAddr = bind_ip_raw
            .parse()
            .map_err(|_| anyhow::anyhow!("WEBHOOK_BIND must be a valid IP address"))?;

        let secret_token = env::var("WEBHOOK_SECRET_TOKEN")
            .map(|value| value.trim().to_string())
            .map_err(|_| {
                anyhow::anyhow!("WEBHOOK_SECRET_TOKEN must be set when USE_WEBHOOK=true")
            })?;

        crate::infrastructure::webhook_security::validate_webhook_runtime_settings(
            &app_env,
            bind_ip,
            &domain,
            &secret_token,
        )?;

        let addr = SocketAddr::new(bind_ip, port);
        let url = format!("https://{}/webhook", domain).parse()?;
        let webhook_metadata = format!(
            r#"{{"domain":"{}","bind":"{}","port":{}}}"#,
            domain, bind_ip, port
        );

        let mut webhook_start_event =
            BotEventRecord::new(BotEventType::WebhookStart, EventSeverity::Info);
        webhook_start_event.status = Some("listening");
        webhook_start_event.metadata_json = &webhook_metadata;

        let _ = db_repo.record_bot_event_record(webhook_start_event).await;

        tracing::info!(
            "[WEBHOOK] Listening on {}:{} for domain {}",
            bind_ip,
            port,
            domain
        );

        crate::infrastructure::webhook_security::spawn_health_endpoint(cancel_token.clone());

        let options = teloxide::update_listeners::webhooks::Options::new(addr, url)
            .secret_token(secret_token)
            .max_connections(crate::infrastructure::webhook_security::webhook_max_connections());

        let listener = teloxide::update_listeners::webhooks::axum(bot, options).await?;

        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("[SYSTEM] Webhook dispatcher shutdown requested.");
            }
            _ = dispatcher.dispatch_with_listener(
                listener,
                LoggingErrorHandler::with_custom_text("Webhook Error"),
            ) => {}
        }
    } else {
        info!("Running in POLLING mode");
        bot.delete_webhook().await?;
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::info!("[SYSTEM] Polling dispatcher shutdown requested.");
            }
            _ = dispatcher.dispatch() => {}
        }
    }

    Ok(())
}
