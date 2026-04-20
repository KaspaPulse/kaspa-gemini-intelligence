use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// 🟢 PUBLIC USER MENU: Complete features with BlockDAG & Support
pub fn main_menu_markup() -> InlineKeyboardMarkup {
    let rows = vec![
        // --- 💰 Wallet & Mining ---
        vec![
            InlineKeyboardButton::callback("💰 Balances", "cmd_balance"),
            InlineKeyboardButton::callback("💼 Wallets", "cmd_list"),
            InlineKeyboardButton::callback("⛏️ Hashrate", "cmd_miner"),
        ],
        vec![
            InlineKeyboardButton::callback("🧱 Mined Blocks", "cmd_blocks"),
            InlineKeyboardButton::callback("💵 KAS Price", "cmd_price"),
            InlineKeyboardButton::callback("📈 Market", "cmd_market"),
        ],
        // --- 🌐 Network & Blockchain ---
        vec![
            InlineKeyboardButton::callback("🌐 Network", "cmd_network"),
            InlineKeyboardButton::callback("📦 BlockDAG", "cmd_dag"),
        ],
        vec![
            InlineKeyboardButton::callback("🪙 Supply", "cmd_supply"),
            InlineKeyboardButton::callback("⛽ Fees", "cmd_fees"),
        ],
        // --- 🆘 Help & Support ---
        vec![InlineKeyboardButton::callback(
            "❤️ Support Developer",
            "cmd_donate",
        )],
    ];

    InlineKeyboardMarkup::new(rows)
}

/// 🔴 ADMIN TERMINAL: Enterprise Command Center (ALL FEATURES)
pub fn admin_menu_markup() -> InlineKeyboardMarkup {
    let rows = vec![
        // --- ⚙️ SYSTEM & CONTROL ---
        vec![InlineKeyboardButton::callback(
            "─── ⚙️ SYSTEM CONTROL ⚙️ ───",
            "none",
        )],
        vec![
            InlineKeyboardButton::callback("⚙️ Settings", "cmd_settings"),
            InlineKeyboardButton::callback("📊 Analytics", "cmd_stats"),
            InlineKeyboardButton::callback("🖥️ Hardware", "cmd_sys"),
        ],
        vec![
            InlineKeyboardButton::callback("📜 View Logs", "cmd_logs"),
            InlineKeyboardButton::callback("🔄 Node Sync", "admin_sync_blocks"),
        ],
        // --- 🛠️ OPERATIONS ---
        vec![InlineKeyboardButton::callback(
            "─── 🛠️ OPERATIONS ───",
            "none",
        )],
        vec![
            InlineKeyboardButton::callback("⏸️ Pause Engine", "cmd_pause"),
            InlineKeyboardButton::callback("▶️ Resume Engine", "cmd_resume"),
            InlineKeyboardButton::callback("⚠️ Restart System", "cmd_restart"),
        ],
        // --- 👤 ALL PUBLIC FEATURES ---
        vec![InlineKeyboardButton::callback(
            "─── 👤 ALL FEATURES ───",
            "none",
        )],
        vec![
            InlineKeyboardButton::callback("💰 Balances", "cmd_balance"),
            InlineKeyboardButton::callback("💼 Wallets", "cmd_list"),
            InlineKeyboardButton::callback("⛏️ Hashrate", "cmd_miner"),
        ],
        vec![
            InlineKeyboardButton::callback("🧱 Mined Blocks", "cmd_blocks"),
            InlineKeyboardButton::callback("💵 KAS Price", "cmd_price"),
            InlineKeyboardButton::callback("📈 Market Data", "cmd_market"),
        ],
        vec![
            InlineKeyboardButton::callback("🌐 Network", "cmd_network"),
            InlineKeyboardButton::callback("📦 BlockDAG", "cmd_dag"),
        ],
        vec![
            InlineKeyboardButton::callback("🪙 Coin Supply", "cmd_supply"),
            InlineKeyboardButton::callback("⛽ Mempool Fees", "cmd_fees"),
        ],
        vec![InlineKeyboardButton::callback(
            "❤️ Support Developer",
            "cmd_donate",
        )],
    ];

    InlineKeyboardMarkup::new(rows)
}

// ==============================================================================
// FORMATTING UTILITIES
// ==============================================================================

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
