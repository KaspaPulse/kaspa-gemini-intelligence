use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// 🟢 PUBLIC USER MENU: Complete features with BlockDAG & Support
pub fn main_menu_markup() -> InlineKeyboardMarkup {
    let rows = vec![
        vec![
            InlineKeyboardButton::callback("💰 My Balances", "cmd_balance"),
            InlineKeyboardButton::callback("⛏️ My Hashrate", "cmd_miner"),
        ],
        vec![
            InlineKeyboardButton::callback("🧱 Mined Blocks", "cmd_blocks"),
            InlineKeyboardButton::callback("💼 Tracked Wallets", "cmd_list"),
        ],
        vec![
            InlineKeyboardButton::callback("🌐 Network Stats", "cmd_network"),
            InlineKeyboardButton::callback("💵 KAS Price", "cmd_price"),
        ],
        vec![
            InlineKeyboardButton::callback("🪙 Coin Supply", "cmd_supply"),
            InlineKeyboardButton::callback("⛽ Mempool Fees", "cmd_fees"),
        ],
        vec![
            InlineKeyboardButton::callback("📦 BlockDAG Details", "cmd_dag"),
            InlineKeyboardButton::callback("❤️ Support", "cmd_donate"),
        ],
    ];

    InlineKeyboardMarkup::new(rows)
}

/// 🔴 ADMIN TERMINAL: Hybrid View including ALL features + Management
pub fn admin_menu_markup() -> InlineKeyboardMarkup {
    let rows = vec![
        // Section 1: User Features (Compact)
        vec![
            InlineKeyboardButton::callback("💰 Balances", "cmd_balance"),
            InlineKeyboardButton::callback("⛏️ Hashrate", "cmd_miner"),
            InlineKeyboardButton::callback("🧱 Blocks", "cmd_blocks"),
        ],
        vec![
            InlineKeyboardButton::callback("🌐 Network", "cmd_network"),
            InlineKeyboardButton::callback("💵 Price", "cmd_price"),
            InlineKeyboardButton::callback("💼 Wallets", "cmd_list"),
        ],
        vec![
            InlineKeyboardButton::callback("🪙 Supply", "cmd_supply"),
            InlineKeyboardButton::callback("⛽ Fees", "cmd_fees"),
            InlineKeyboardButton::callback("📦 DAG Info", "cmd_dag"), // Re-added here
        ],
        vec![InlineKeyboardButton::callback(
            "❤️ Support Developer",
            "cmd_donate",
        )], // Re-added here
        // Section 2: Visual Separator
        vec![InlineKeyboardButton::callback(
            "─── ⚙️ ADMIN TERMINAL ⚙️ ───",
            "none",
        )],
        // Section 3: Professional Management Tools
        vec![
            InlineKeyboardButton::callback("📊 Admin Stats", "cmd_stats"),
            InlineKeyboardButton::callback("🔄 Sync All Blocks", "admin_sync_blocks"),
        ],
        vec![
            InlineKeyboardButton::callback("🖥️ System Status", "cmd_sys"),
            InlineKeyboardButton::callback("📜 View Logs", "cmd_logs"),
        ],
        vec![
            InlineKeyboardButton::callback("⏸️ Pause", "cmd_pause"),
            InlineKeyboardButton::callback("▶️ Resume", "cmd_resume"),
            InlineKeyboardButton::callback("🔄 Restart", "cmd_restart"),
        ],
        vec![InlineKeyboardButton::callback(
            "📢 Broadcast Message",
            "cmd_broadcast",
        )],
    ];

    InlineKeyboardMarkup::new(rows)
}

pub fn format_difficulty(val: f64) -> String {
    if val <= 0.0 {
        return "0.00".to_string();
    }
    if val >= 1e15 {
        format!("{:.2} P", val / 1e15)
    } else if val >= 1e12 {
        format!("{:.2} T", val / 1e12)
    } else if val >= 1e9 {
        format!("{:.2} G", val / 1e9)
    } else {
        format!("{:.2}", val)
    }
}

pub fn format_hashrate(h: f64) -> String {
    if h >= 1e15 {
        format!("{:.2} PH/s", h / 1e15)
    } else if h >= 1e12 {
        format!("{:.2} TH/s", h / 1e12)
    } else if h >= 1e9 {
        format!("{:.2} GH/s", h / 1e9)
    } else if h >= 1e6 {
        format!("{:.2} MH/s", h / 1e6)
    } else {
        format!("{:.2} H/s", h)
    }
}
