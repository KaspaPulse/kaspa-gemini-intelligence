use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, std::fmt::Debug)]
#[command(rename_rule = "lowercase", description = "Kaspa Pulse Bot Commands:")]
pub enum Command {
    #[command(description = "Start the bot and show main menu.")]
    Start,
    #[command(description = "Show the guide and features.")]
    Help,
    #[command(description = "Add a wallet: /add <address>")]
    Add(String),
    #[command(description = "Remove a wallet: /remove <address>")]
    Remove(String),
    #[command(description = "List all tracked wallets.")]
    List,
    #[command(description = "Check live balance and UTXOs.")]
    Balance,
    #[command(description = "Estimate your solo-mining hashrate.")]
    Miner,
    #[command(description = "Count your unspent mined blocks.")]
    Blocks,
    #[command(description = "Support the developer.")]
    Donate,

    #[command(
        rename = "forget_wallets",
        description = "Delete all my tracked wallets."
    )]
    ForgetWallets,
    #[command(rename = "forget_all", description = "Erase all my data.")]
    ForgetAll,
    #[command(rename = "hidemenu", description = "إخفاء الكيبورد الثابت")]
    HideMenu,

    #[command(description = "Show full node and network health.")]
    Network,
    #[command(description = "Show BlockDAG consensus details.")]
    Dag,
    #[command(description = "Check KAS price and market cap.")]
    Price,
    #[command(description = "Check market cap details.")]
    Market,
    #[command(description = "Check circulating and max supply.")]
    Supply,
    #[command(description = "Check real-time mempool fees.")]
    Fees,

    #[command(description = "Admin: Community bot health report.")]
    Health,
    #[command(description = "Admin: Global analytics and user report.")]
    Stats,
    #[command(description = "Admin: System hardware diagnostics.")]
    Sys,
    #[command(description = "Admin: Pause UTXO monitoring.")]
    Pause,
    #[command(description = "Admin: Resume UTXO monitoring.")]
    Resume,
    #[command(description = "Admin: Safe restart of the bot binary.")]
    Restart,
    #[command(description = "Admin: Broadcast message to all users.")]
    Broadcast(String),
    #[command(description = "Admin: Tail last 25 lines of bot.log.")]
    Logs,
    #[command(description = "Admin: Show recent bot event log.")]
    Events,
    #[command(rename = "errors", description = "Admin: Show recent error events.")]
    Errors,
    #[command(
        rename = "delivery",
        description = "Admin: Show alert delivery summary."
    )]
    Delivery,
    #[command(
        rename = "subscribers",
        description = "Admin: Show wallet subscribers."
    )]
    Subscribers(String),
    #[command(
        rename = "wallet_events",
        description = "Admin: Show wallet event history."
    )]
    WalletEvents(String),
    #[command(
        rename = "cleanup_events",
        description = "Admin: Cleanup old bot events."
    )]
    CleanupEvents,
    #[command(
        rename = "db_diag",
        description = "Admin: Database health diagnostics."
    )]
    DbDiag,
    #[command(description = "Admin: Open settings panel.")]
    Settings,
    #[command(description = "Admin: Toggle a feature flag.")]
    Toggle(String),
    #[command(description = "Erase all my data and wallets.")]
    Forget,
}

pub fn public_bot_commands() -> Vec<teloxide::types::BotCommand> {
    vec![
        teloxide::types::BotCommand::new("start", "Open the main menu"),
        teloxide::types::BotCommand::new("help", "Show the guide and features"),
        teloxide::types::BotCommand::new("add", "Add a wallet: /add kaspa:..."),
        teloxide::types::BotCommand::new("remove", "Remove a wallet: /remove kaspa:..."),
        teloxide::types::BotCommand::new("list", "Show tracked wallets"),
        teloxide::types::BotCommand::new("balance", "Check live balance and UTXOs"),
        teloxide::types::BotCommand::new("miner", "Estimate solo-mining hashrate"),
        teloxide::types::BotCommand::new("blocks", "Show mined block stats"),
        teloxide::types::BotCommand::new("network", "Show node and network health"),
        teloxide::types::BotCommand::new("dag", "Show BlockDAG overview"),
        teloxide::types::BotCommand::new("price", "Check KAS price and market data"),
        teloxide::types::BotCommand::new("supply", "Check circulating and max supply"),
        teloxide::types::BotCommand::new("fees", "Check real-time network fees"),
        teloxide::types::BotCommand::new("donate", "Support development"),
    ]
}
