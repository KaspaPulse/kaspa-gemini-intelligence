<div align="center">

# 🦀 Kaspa Pulse
### Community Mining Alerts for Kaspa Solo Miners

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Kaspa](https://img.shields.io/badge/Kaspa-Network-70D4CB.svg?style=for-the-badge)](https://kaspa.org/)
[![Database](https://img.shields.io/badge/Database-PostgreSQL-336791.svg?style=for-the-badge&logo=postgresql)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](LICENSE)

A lightweight Telegram bot for tracking Kaspa wallets, solo-mining rewards, wallet balances, BlockDAG/network metrics, and live mining alerts.

</div>

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Features](#-features)
- [Commands](#-commands)
- [Environment Setup](#-environment-setup-env)
- [Database](#-database)
- [Running Locally](#-running-locally)
- [Systemd Deployment](#-systemd-deployment)
- [Security Notes](#-security-notes)
- [Support](#-support)
- [License](#-license)

---

## 🚀 Overview

**Kaspa Pulse** is a Rust-based Telegram bot designed for community Kaspa miners and wallet monitoring.

It connects to a Kaspa node through wRPC/WebSocket and stores wallet/mining state in PostgreSQL. The bot focuses on simple, fast, and useful mining alerts without AI, RSS, Redis, or unnecessary enterprise components.

The current design goal is:

```text
Simple.
Private.
Fast.
Community-focused.
````

---

## ✨ Features

### 👛 Wallet Tracking

* Add and remove Kaspa wallets.
* List tracked wallets.
* Open each wallet from inline buttons.
* View wallet-specific balance, UTXOs, blocks, and miner estimates.

### ⛏️ Mining Alerts

* Monitors wallet UTXOs.
* Detects mined rewards.
* Stores mined block records in PostgreSQL.
* Sends Telegram alerts when new rewards are detected.

### 📊 Wallet Analytics

* Total balance across tracked wallets.
* Per-wallet balance.
* Total UTXOs and average UTXO size.
* USD value using market price APIs.

### 🧱 Mined Blocks

* Blocks in the last 1 hour.
* Blocks in the last 24 hours.
* Blocks in the last 7 days.
* Lifetime mined block records.
* Per-wallet block breakdown.

### ⛏️ Miner Hashrate Estimate

* 1H / 24H / 7D actual hashrate estimate.
* Unspent UTXO-based estimate.
* Per-wallet miner view.

### 🌐 Network & DAG Metrics

* Node status.
* Network name.
* Connected peers.
* Global hashrate.
* Sync status.
* Active tips.
* Live BPS and expected BPS.
* BlockDAG metrics.
* Pruning point and pruning time.

### 🛡️ Safety & Reliability

* Rate limiting for commands and callbacks.
* Wallet count limit per user.
* Maximum raw message length.
* Maximum wallet address length.
* Privacy-safe logs with masked wallet addresses.
* Admin-only diagnostics.
* Health and database diagnostics.

---

## 📱 Commands

### Public Commands

| Command             | Description                       |
| ------------------- | --------------------------------- |
| `/start`            | Open the main menu                |
| `/help`             | Show help guide                   |
| `/add kaspa:...`    | Track a wallet                    |
| `/remove kaspa:...` | Stop tracking a wallet            |
| `/list`             | View tracked wallets              |
| `/balance`          | Show wallet balance summary       |
| `/blocks`           | Show mined block summary          |
| `/miner`            | Show solo-miner hashrate estimate |
| `/network`          | Show node/network health          |
| `/dag`              | Show BlockDAG metrics             |
| `/price`            | Show KAS market data              |
| `/market`           | Show market data                  |
| `/supply`           | Show supply metrics               |
| `/fees`             | Show fee estimate                 |
| `/donate`           | Show support address              |

### Admin Commands

| Command       | Description                         |
| ------------- | ----------------------------------- |
| `/health`     | Production health report            |
| `/db_diag`    | Database diagnostics                |
| `/stats`      | Bot/system stats                    |
| `/sys`        | System diagnostics                  |
| `/logs`       | View logs if enabled                |
| `/settings`   | Settings panel                      |
| `/pause`      | Pause monitoring                    |
| `/resume`     | Resume monitoring                   |
| `/restart`    | Restart service if configured       |
| `/forget_all` | Delete user data after confirmation |

---

## 🔐 Environment Setup (`.env`)

Create a `.env` file in the project root.

Do **not** commit `.env` to Git.

```env
# ==============================================================================
# TELEGRAM BOT CONFIGURATION
# ==============================================================================
BOT_TOKEN=PUT_YOUR_TELEGRAM_BOT_TOKEN_HERE
ADMIN_ID=PUT_YOUR_TELEGRAM_ADMIN_ID_HERE

# ==============================================================================
# KASPA NODE & DATABASE
# ==============================================================================
NODE_URL_01=wss://your-kaspa-node.example.com/json
DATABASE_URL=postgres://postgres:PUT_PASSWORD_HERE@127.0.0.1:5433/kaspa_dev?sslmode=disable

# ==============================================================================
# EXTERNAL APIs
# ==============================================================================
COINGECKO_API_URL=https://api.coingecko.com/api/v3/simple/price?ids=kaspa&vs_currencies=usd&include_market_cap=true
KASPA_API_PRICE_URL=https://api.kaspa.org/info/price
KASPA_API_MCAP_URL=https://api.kaspa.org/info/marketcap

# ==============================================================================
# PRODUCTION SETTINGS
# ==============================================================================
APP_ENV=production
RUST_LOG=info
ENABLE_VERBOSE_LOGS=false
DB_MAX_CONNECTIONS=10
HTTP_TIMEOUT_SECS=10
HTTP_CONNECT_TIMEOUT_SECS=5

# ==============================================================================
# TELEGRAM PROTECTION
# ==============================================================================
MAX_WALLETS_PER_USER=10
RATE_LIMIT_COMMANDS_PER_SECOND=1
RATE_LIMIT_CALLBACKS_PER_SECOND=3
RATE_LIMIT_ADD_WALLET_PER_MINUTE=5
MAX_RAW_MESSAGE_CHARS=512
MAX_WALLET_ADDRESS_CHARS=120
LOG_MAX_CHARS=5000

# ==============================================================================
# WEBHOOK SETTINGS
# Use polling unless webhook is fully configured behind a reverse proxy.
# ==============================================================================
USE_WEBHOOK=false
WEBHOOK_DOMAIN=your-domain.com
WEBHOOK_PORT=8443
WEBHOOK_BIND=127.0.0.1
WEBHOOK_SECRET_TOKEN=PUT_RANDOM_SECRET_HERE
```

---

## 🗄️ Database

Kaspa Pulse uses PostgreSQL.

The active tables are:

```text
user_wallets
mined_blocks
system_settings
_sqlx_migrations
```

Legacy AI/RSS tables are not required in the current version.

Recommended database name:

```text
kaspa_dev
```

Example connection:

```env
DATABASE_URL=postgres://postgres:password@127.0.0.1:5433/kaspa_dev?sslmode=disable
```

If the password contains special characters, URL-encode it.

Examples:

```text
$  -> %24
@  -> %40
!  -> %21
#  -> %23
%  -> %25
```

---

## ▶️ Running Locally

```bash
cargo check
cargo run --release
```

For Windows development with a remote database through SSH tunnel:

```powershell
ssh -o ServerAliveInterval=60 -L 15433:127.0.0.1:5433 kaspa@YOUR_SERVER_IP
```

Then use:

```env
DATABASE_URL=postgres://postgres:password@127.0.0.1:15433/kaspa_dev?sslmode=disable
```

---

## 🛠️ Systemd Deployment

Example service file:

```ini
[Unit]
Description=Kaspa Pulse Community Mining Alerts
After=network.target

[Service]
User=kaspa
WorkingDirectory=/home/kaspa/kaspa-telegram-notify
ExecStart=/home/kaspa/kaspa-telegram-notify/target/release/kaspa-pulse
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable kaspa-pulse.service
sudo systemctl restart kaspa-pulse.service
sudo systemctl status kaspa-pulse.service --no-pager
```

Follow logs:

```bash
sudo journalctl -u kaspa-pulse.service -f
```

---

## 🔗 Webhook Notes

Polling mode is the simplest and recommended default:

```env
USE_WEBHOOK=false
```

If webhook mode is enabled:

```env
USE_WEBHOOK=true
WEBHOOK_BIND=127.0.0.1
WEBHOOK_SECRET_TOKEN=PUT_RANDOM_SECRET_HERE
```

Run the bot behind Nginx or Caddy and validate:

```text
X-Telegram-Bot-Api-Secret-Token
```

Do not expose the bot port directly to the public internet.

---

## 🛡️ Security Notes

* Never commit `.env`.
* Rotate any token that was exposed.
* Keep wallet addresses masked in logs unless `ENABLE_VERBOSE_LOGS=true`.
* Keep `MAX_WALLETS_PER_USER` enabled.
* Keep rate limits enabled.
* Prefer polling unless webhook reverse proxy is configured properly.
* Use `cargo deny check`, `cargo machete`, and `cargo check` before deployment.

Recommended checks:

```bash
cargo fmt
cargo check
cargo deny check
cargo machete
```

---

## 💖 Support

If you find this tool useful, you can support development:

```text
kaspa:qz0yqq8z3twwgg7lq2mjzg6w4edqys45w2wslz7tym2tc6s84580vvx9zr44g
```

---

## 📜 License

This project is licensed under the MIT License.

<div align="center">
  <i>⛏️ Happy Kaspa Mining!</i>
</div>
