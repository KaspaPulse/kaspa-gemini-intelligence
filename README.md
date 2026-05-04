<div align="center">

# 🦀 Kaspa Pulse
### Community Mining Alerts for Kaspa Solo Miners

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Kaspa](https://img.shields.io/badge/Kaspa-Network-70D4CB.svg?style=for-the-badge)](https://kaspa.org/)
[![Database](https://img.shields.io/badge/Database-PostgreSQL-336791.svg?style=for-the-badge&logo=postgresql)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](LICENSE)

A production-oriented Telegram bot for tracking Kaspa wallets, confirmed solo-mining rewards, wallet balances, BlockDAG/network metrics, and live mining alerts.

</div>

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Production Architecture](#-production-architecture)
- [Features](#-features)
- [Commands](#-commands)
- [Environment Setup](#-environment-setup-env)
- [Database](#-database)
- [Security Model](#-security-model)
- [Running Locally](#-running-locally)
- [Systemd Deployment](#-systemd-deployment)
- [Webhook, Health, and Metrics](#-webhook-health-and-metrics)
- [Supply Chain Checks](#-supply-chain-checks)
- [Git History and Secret Rotation](#-git-history-and-secret-rotation)
- [Support](#-support)
- [License](#-license)

---

## 🚀 Overview

**Kaspa Pulse** is a Rust-based Telegram bot designed for community Kaspa miners and wallet monitoring.

It connects to a Kaspa node through wRPC/WebSocket and stores wallet, reward, deduplication, event, and delivery state in PostgreSQL.

The current design goal is:

```text
Simple.
Private.
Fast.
Auditable.
Production-ready.
Community-focused.
````

Kaspa Pulse is intentionally lightweight. It does not require Redis, AI services, RSS ingestion, or unnecessary enterprise components.

---

## 🧱 Production Architecture

Current high-level alert flow:

```text
Telegram User
    ↓
Bot Command / Wallet Input
    ↓
Input Validation + Rate Limits
    ↓
PostgreSQL wallet state
    ↓
UTXO Monitor Worker
    ↓
Reward Confirmation Gate
    ↓
DAG Analysis
    ↓
Event Logs + Dedup State
    ↓
telegram_delivery_queue
    ↓
Telegram Delivery Worker
    ↓
Telegram Alert
```

Important separation:

```text
/pause          = pauses live monitoring
/mute_alerts    = mutes Telegram alert delivery only
```

When alert delivery is muted, the bot still:

* detects rewards
* performs confirmation checks
* analyzes DAG data
* records events
* updates deduplication state
* stores suppressed alert counts

It only stops sending Telegram mining alerts.

---

## ✨ Features

### 👛 Wallet Tracking

* Add and remove Kaspa wallets.
* List tracked wallets.
* Open each wallet from inline buttons.
* View wallet-specific balance, UTXOs, blocks, and miner estimates.
* Limit maximum wallets per user.

### ⛏️ Mining Alerts

* Monitors wallet UTXOs.
* Detects coinbase mining rewards.
* Waits for the configured reward confirmation threshold.
* Performs DAG analysis to identify accepting block and real mined block.
* Stores mined block records in PostgreSQL.
* Uses wallet-scoped deduplication to avoid duplicate alerts.
* Sends confirmed mining alerts through a database-backed Telegram delivery queue.

### 🔕 Alert Delivery Toggle

Admin can stop or resume alert delivery without stopping the bot:

```text
/mute_alerts
/unmute_alerts
/alerts_status
```

When muted, alerts are recorded as suppressed and counted in the status summary.

### 📬 Telegram Delivery Queue

Kaspa Pulse uses a delivery queue table:

```text
telegram_delivery_queue
```

The UTXO monitor enqueues alert messages, and a separate Telegram delivery worker sends them.

This provides:

* safer separation between detection and delivery
* retry tracking
* sent/failed status
* failure logging
* metrics support
* future backoff improvements

Direct Telegram send is kept only as a fallback when the queue is disabled or enqueue fails.

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

### 🩺 Health and Metrics

Local-only endpoints can expose:

```text
/healthz
/readyz
/metrics
```

Metrics are rendered in Prometheus-style text format and include counters such as:

```text
kaspa_pulse_alerts_delivered_total
kaspa_pulse_alerts_suppressed_total
kaspa_pulse_admin_actions_confirmed_total
kaspa_pulse_telegram_send_failures_total
kaspa_pulse_rpc_timeouts_total
kaspa_pulse_db_errors_total
```

### 🛡️ Safety & Reliability

* Rate limiting for commands and callbacks.
* Wallet count limit per user.
* Maximum raw message length.
* Maximum wallet address length.
* Privacy-safe logs with masked wallet addresses.
* Telegram HTML output escaping for user-controlled text.
* Admin-only diagnostics.
* Admin confirmation tokens for sensitive actions.
* Admin audit log.
* Health and database diagnostics.
* Panic/restart marker support.
* Graceful shutdown drain before database pool close.
* SQLx offline mode support for reproducible CI/build checks.

---

## 📱 Commands

### Public Commands

| Command             | Description                               |
| ------------------- | ----------------------------------------- |
| `/start`            | Open the main menu                        |
| `/help`             | Show help guide                           |
| `/add kaspa:...`    | Track a wallet                            |
| `/remove kaspa:...` | Stop tracking a wallet                    |
| `/list`             | View tracked wallets                      |
| `/balance`          | Show wallet balance summary               |
| `/blocks`           | Show mined block summary                  |
| `/miner`            | Show solo-miner hashrate estimate         |
| `/network`          | Show node/network health                  |
| `/dag`              | Show BlockDAG metrics                     |
| `/price`            | Show KAS market data                      |
| `/market`           | Show market data                          |
| `/supply`           | Show supply metrics                       |
| `/fees`             | Show fee estimate                         |
| `/donate`           | Show support address                      |
| `/forget_wallets`   | Delete tracked wallets after confirmation |
| `/forget_all`       | Delete user data after confirmation       |
| `/hidemenu`         | Hide persistent Telegram keyboard         |

### Admin Commands

| Command           | Description                              |
| ----------------- | ---------------------------------------- |
| `/health`         | Production health report                 |
| `/db_diag`        | Database diagnostics                     |
| `/stats`          | Bot/system stats                         |
| `/sys`            | System diagnostics                       |
| `/logs`           | View recent logs if enabled              |
| `/settings`       | Settings panel                           |
| `/pause`          | Pause live monitoring                    |
| `/resume`         | Resume live monitoring                   |
| `/restart`        | Restart service request                  |
| `/events`         | Show recent bot events                   |
| `/errors`         | Show recent error events                 |
| `/delivery`       | Alert delivery summary                   |
| `/mute_alerts`    | Stop sending Telegram mining alerts only |
| `/unmute_alerts`  | Resume sending Telegram mining alerts    |
| `/alerts_status`  | Show alert delivery enabled/muted status |
| `/subscribers`    | Show wallet subscribers                  |
| `/wallet_events`  | Show wallet event history                |
| `/cleanup_events` | Cleanup old bot events                   |
| `/broadcast`      | Broadcast admin message                  |
| `/toggle`         | Toggle selected feature flag             |

Admin-sensitive actions should use confirmation flow where applicable.

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

# Runtime must use the least-privilege app user, not postgres.
DATABASE_URL=postgres://kaspa_pulse_app:PUT_APP_PASSWORD_HERE@127.0.0.1:5433/kaspa_dev?sslmode=disable

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
SHUTDOWN_DRAIN_SECS=3
SQLX_OFFLINE=true

# ==============================================================================
# TELEGRAM PROTECTION
# ==============================================================================
MAX_WALLETS_PER_USER=10
RATE_LIMIT_COMMANDS_PER_SECOND=1
RATE_LIMIT_CALLBACKS_PER_SECOND=3
RATE_LIMIT_ADD_WALLET_PER_MINUTE=5
MAX_RAW_MESSAGE_CHARS=512
MAX_WALLET_ADDRESS_CHARS=120
LOG_MAX_CHARS=3000

# ==============================================================================
# ALERT DELIVERY
# ==============================================================================
ENABLE_TELEGRAM_DELIVERY_QUEUE=true
ENABLE_ALERT_DELIVERY=true
MIN_REWARD_CONFIRMATIONS=10

# ==============================================================================
# WEBHOOK SETTINGS
# Use polling unless webhook is fully configured behind a reverse proxy.
# ==============================================================================
USE_WEBHOOK=false
WEBHOOK_DOMAIN=your-domain.com
WEBHOOK_PORT=8443
WEBHOOK_BIND=127.0.0.1
WEBHOOK_SECRET_TOKEN=PUT_RANDOM_32_PLUS_CHAR_SECRET_HERE
WEBHOOK_MAX_CONNECTIONS=20
WEBHOOK_ALLOW_PUBLIC_BIND=false

# ==============================================================================
# HEALTH AND METRICS
# ==============================================================================
HEALTH_ENDPOINT_ENABLED=true
HEALTH_BIND=127.0.0.1
HEALTH_PORT=18080
HEALTH_ALLOW_PUBLIC_BIND=false

# ==============================================================================
# RETENTION
# ==============================================================================
LOG_RETENTION_DAYS=30
EVENT_LOG_RETENTION_DAYS=30
WALLET_DEDUP_RETENTION_DAYS=30
SEEN_UTXO_RETENTION_DAYS=30
```

---

## 🗄️ Database

Kaspa Pulse uses PostgreSQL.

Runtime should use the limited role:

```text
kaspa_pulse_app
```

The `postgres` superuser should be used only for administrative tasks and migrations.

Important tables include:

```text
user_wallets
mined_blocks
actual_mined_blocks
pending_rewards
wallet_seen_utxos
wallet_alert_dedup
bot_event_log
system_settings
admin_audit_log
telegram_delivery_queue
_sqlx_migrations
```

Recommended database name:

```text
kaspa_dev
```

Runtime connection example:

```env
DATABASE_URL=postgres://kaspa_pulse_app:password@127.0.0.1:5433/kaspa_dev?sslmode=disable
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

### Migrations

Schema changes must be applied through migrations, not runtime table creation.

Important migrations:

```text
0003_alert_delivery_setting.sql
0004_admin_audit_and_delivery_queue.sql
0005_delivery_queue_payload.sql
```

Run migrations with an admin database user when DDL permissions are required.

---

## 🛡️ Security Model

### Database Least Privilege

Production runtime must not use the PostgreSQL `postgres` superuser.

Runtime should use:

```text
kaspa_pulse_app
```

Allowed runtime privileges:

* CONNECT on the target database
* USAGE on schema `public`
* SELECT / INSERT / UPDATE / DELETE on application tables
* USAGE / SELECT / UPDATE on application sequences
* EXECUTE on the safe retention helper, when available

### Input and Output Safety

* Wallet input is length-limited.
* Raw Telegram messages are length-limited.
* Wallet parsing uses Kaspa address validation where applicable.
* Invisible/control characters should be rejected or normalized.
* Telegram HTML parse mode requires escaping user-controlled text before insertion into HTML responses.
* Logs should mask wallet, TXID, and block hash values.

### Admin Safety

Sensitive admin actions should require confirmation with short TTL tokens.

Examples:

```text
/pause
/resume
/restart
/mute_alerts
/unmute_alerts
/cleanup_events
```

Admin actions are recorded in:

```text
admin_audit_log
```

### Alert Delivery Safety

Alert delivery has two independent controls:

```text
ENABLE_ALERT_DELIVERY=true
ENABLE_TELEGRAM_DELIVERY_QUEUE=true
```

`ENABLE_ALERT_DELIVERY=false` suppresses outgoing alert messages but keeps detection and database recording active.

`ENABLE_TELEGRAM_DELIVERY_QUEUE=false` disables queue mode and allows direct-send fallback.

---

## ▶️ Running Locally

```bash
cargo check
cargo test
cargo run --release
```

For Windows development with a remote database through SSH tunnel:

```powershell
ssh -o ServerAliveInterval=60 -L 15433:127.0.0.1:5433 kaspa@YOUR_SERVER_IP
```

Then use:

```env
DATABASE_URL=postgres://kaspa_pulse_app:password@127.0.0.1:15433/kaspa_dev?sslmode=disable
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
Environment=SQLX_OFFLINE=true

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

After history rewrite, use reset instead of normal pull on existing clones:

```bash
git fetch origin
git checkout dev
git reset --hard origin/dev
git clean -fd
```

---

## 🔗 Webhook, Health, and Metrics

Polling mode is the simplest default:

```env
USE_WEBHOOK=false
```

If webhook mode is enabled:

```env
USE_WEBHOOK=true
WEBHOOK_BIND=127.0.0.1
WEBHOOK_SECRET_TOKEN=PUT_RANDOM_32_PLUS_CHAR_SECRET_HERE
```

Run the bot behind Nginx or Caddy.

Do not expose the bot port directly to the public internet.

Telegram webhook secret header:

```text
X-Telegram-Bot-Api-Secret-Token
```

Local health and metrics endpoints:

```bash
curl http://127.0.0.1:18080/healthz
curl http://127.0.0.1:18080/readyz
curl http://127.0.0.1:18080/metrics
```

---

## 🔎 Supply Chain Checks

Recommended local checks:

```bash
cargo fmt
cargo audit
cargo deny check
cargo tree -d
cargo machete
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Or run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\security-check.ps1
```

Ignored RustSec advisories must be documented in:

```text
SECURITY_ADVISORIES.md
```

---

## 🧹 Git History and Secret Rotation

If `.env`, `.backup`, database dump files, or secrets were ever committed:

1. Remove them from the current repository state.
2. Add `.gitignore` protections.
3. Rewrite Git history.
4. Force-push updated `dev` and `main`.
5. Delete local `refs/original`.
6. Run aggressive garbage collection.
7. Rotate affected passwords and tokens.
8. Reset server clones with `git reset --hard origin/dev`.

Sensitive paths that must never be committed:

```text
.env
.env.*
.backup/
backups/
*.dump
*.sql.dump
*.bak
repo-before-history-clean-*.bundle
project_code_export_*.txt
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
