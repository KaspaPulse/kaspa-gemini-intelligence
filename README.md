# 🦀 Kaspa Pulse: The Ultimate Enterprise Miner's Companion

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=flat-square)
![Kaspa](https://img.shields.io/badge/Kaspa-Network-70D4CB.svg?style=flat-square)
![Database](https://img.shields.io/badge/Database-SQLite-blue.svg?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)
![AI-Powered](https://img.shields.io/badge/AI-Gemini%202.5%20Flash-purple.svg?style=flat-square)
![Zero-Warnings](https://img.shields.io/badge/Standard-Zero%20Warnings-success.svg?style=flat-square)

---

## 🚀 Overview

**Kaspa Pulse** (formerly Kaspa Solo) is an ultra-high-performance, enterprise-grade Telegram bot engineered entirely in Rust.

Built for **Kaspa Solo Miners** and **Full Node Operators**, it delivers:

* ⚡ Zero-latency notifications
* 🔐 Maximum privacy (no public APIs)
* 🧠 AI-powered intelligence
* 🔬 Deep GHOSTDAG blockchain forensics

It connects directly to your node via **wRPC WebSocket**, ensuring raw, unindexed, real-time blockchain data.

---

## ✨ Features

### 🧠 AI Intelligence (Gemini 2.5 Flash)

* Natural conversational interaction
* Voice message understanding (OGG → Base64 → AI)
* Smart contextual responses (wallets, difficulty, DAA score)
* Built-in retry system (handles 429 / 503 errors)

### 🎯 Deterministic Block Detection

* Reverse DAG traversal
* Byte-level payload scanning
* 100% accurate reward attribution
* No explorer dependency

### 🔬 Mining Forensics

* Extracts block **Nonce**
* Decodes **Worker ID**
* Identifies exact mining source

### 🕒 Smart UTXO Processing

* Parallel processing via `tokio::task::JoinSet`
* Chronological sorting using `block_time_ms`
* Perfect notification ordering

### ⛏️ Hashrate Estimation

* 1H / 24H / 7D analysis
* Based on real mined rewards

### 🗄️ Storage Engine

* SQLite (ACID compliant)
* Crash-safe
* Fast startup

---

## 📱 Commands

### 📌 Public

| Command             | Description        |
| ------------------- | ------------------ |
| `/start`            | Initialize bot     |
| `/help`             | Show guide         |
| `/add <address>`    | Track wallet       |
| `/remove <address>` | Remove wallet      |
| `/list`             | Show wallets       |
| `/balance`          | Show balance       |
| `/blocks`           | Count mined blocks |
| `/miner`            | Estimate hashrate  |
| `/network`          | Node stats         |
| `/dag`              | DAG info           |
| `/price`            | KAS price          |
| `/market`           | Market cap         |
| `/supply`           | Supply             |
| `/fees`             | Fees               |
| `/donate`           | Support            |

---

### 👑 Admin

| Command              | Description    |
| -------------------- | -------------- |
| `/stats`             | Bot analytics  |
| `/sys`               | System stats   |
| `/logs`              | Get logs       |
| `/broadcast <msg>`   | Send message   |
| `/pause` / `/resume` | Control engine |
| `/restart`           | Restart bot    |
| `/learn <text>`      | Teach AI       |
| `/autolearn`         | Auto news      |

---

## ⚙️ Prerequisites

### 🔧 Build Tools

* **Windows**

```bash
winget install cmake
```

* **Linux**

```bash
sudo apt update && sudo apt install cmake build-essential
```

---

## 🔐 Environment Setup

Create `.env` file:

```env
BOT_TOKEN=your_telegram_bot_token_here
ADMIN_ID=your_telegram_user_id
WS_URL=ws://127.0.0.1:18110
GEMINI_API_KEY=your_google_ai_studio_key_here
RUST_LOG=info,kaspa_solo=debug
```

---

## 🛠️ Deployment

### 📦 Clone

```bash
git clone https://github.com/KaspaPulse/kaspa-gemini-intelligence.git
cd kaspa-gemini-intelligence
```

---

### 🐧 Linux

#### Install

```bash
sudo apt update && sudo apt install -y curl build-essential pkg-config libssl-dev cmake
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Build

```bash
cargo build --release
```

#### Service

```bash
sudo nano /etc/systemd/system/kaspa-pulse.service
```

```ini
[Unit]
Description=Kaspa Pulse Enterprise Bot
After=network.target

[Service]
User=your_username
WorkingDirectory=/home/your_username/kaspa-gemini-intelligence
ExecStart=/home/your_username/kaspa-gemini-intelligence/target/release/kaspa-solo
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable kaspa-pulse
sudo systemctl start kaspa-pulse
```

---

### 🪟 Windows

#### Build

```powershell
cargo build --release
```

#### NSSM Service

```cmd
nssm install KaspaPulseBot
```

ثم:

```cmd
nssm start KaspaPulseBot
```

---

## 🤝 Contributing

```bash
git checkout -b feature/new-feature
git commit -m "feat: add feature"
git push origin feature/new-feature
```

---

## 💖 Support

Kaspa (KAS):

```
kaspa:qz0yqq8z3twwgg7lq2mjzg6w4edqys45w2wslz7tym2tc6s84580vvx9zr44g
```

---

## 📜 License

MIT License

---

## 🧠 Final Note

Built with precision for the Kaspa ecosystem.
