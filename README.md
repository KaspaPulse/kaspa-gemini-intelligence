# 🦀 Kaspa Pulse: The Ultimate Enterprise Miner's Companion

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=flat-square)
![Kaspa](https://img.shields.io/badge/Kaspa-Network-70D4CB.svg?style=flat-square)
![Database](https://img.shields.io/badge/Database-PostgreSQL-336791.svg?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)
![AI-Powered](https://img.shields.io/badge/AI-Llama3%20%7C%20OpenAI-purple.svg?style=flat-square)
![Architecture](https://img.shields.io/badge/Architecture-Parallel%20Streaming-success.svg?style=flat-square)
![State Management](https://img.shields.io/badge/State-DashMap%20RAM-blue.svg?style=flat-square)
![Standard](https://img.shields.io/badge/Standard-Zero%20Warnings-success.svg?style=flat-square)

---

## 🚀 Overview

**Kaspa Pulse** (formerly Kaspa Solo) is an ultra-high-performance, enterprise-grade Telegram bot engineered entirely in Rust.

Built for **Kaspa Solo Miners** and **Full Node Operators**, it delivers:

- ⚡ Zero-latency notifications via direct wRPC  
- 🔐 Maximum privacy (no public APIs or external explorers)  
- 🧠 RAG AI-powered intelligence (Vector Search & Voice-to-Text)  
- 🔬 Deep GHOSTDAG blockchain forensics (Nonce & Worker extraction)  
- 🐘 High-Performance PostgreSQL State Management  
- 🛡️ Anti-Flood, Rate Limited & Prompt-Injection Hardened  
- ⚙️ **Dynamic Enterprise Control Panel (Zero-Downtime Configuration)**  

It connects directly to your node via **wRPC WebSocket**, ensuring raw, unindexed, real-time blockchain data.

---

## ✨ Core Features

### 🧠 Advanced AI Intelligence (Universal Standard API)

- **Voice-to-Text Analytics:** Send a voice note (OGG) directly to the bot. It transcribes the audio and feeds it to the AI (Whisper V3).
- **Context-Aware RAG:** Uses live wallet balance, DAA score, price + `pgvector` database.
- **Universal API Support:** Compatible with OpenAI / Groq APIs.
- **Enterprise Retry Logic:** Handles 429 rate limits with exponential backoff.

---

### ⚙️ Dynamic Enterprise Control Panel

Manage everything live from Telegram:

- Toggle **Maintenance Mode** or **Private Access**
- Enable/Disable workers (RSS, Memory Cleaner, AI)
- Switch AI providers instantly

---

### 🛡️ Smart Node Safety Protocol (Anti-Ban)

- Detects Local vs Public Node
- Prevents heavy operations on public nodes
- Keeps real-time tracking safe

---

### 🎯 Deterministic Block Detection & Forensics

- Reverse DAG traversal
- Byte-level payload scanning
- 100% accurate reward attribution
- Extracts **Nonce** & **Worker ID**

---

### 🕒 Smart UTXO Processing

- Parallel processing (`tokio::task::JoinSet`)
- Sorted by `block_time_ms`
- Zero message desync

---

### ⛏️ Hashrate Estimation

- 1H / 24H / 7D analysis  
- Based on real mined rewards  

---

### 🗄️ Storage & Architecture

- PostgreSQL (`sqlx`) + `pgvector`
- DashMap (RAM state)
- Modular structure:  
  `ai/`, `handlers/`, `workers/`, `services/`

---

## 📖 How to Use

1. Send `/start`
2. Paste your Kaspa address
3. Use buttons or commands
4. Chat with AI or send voice

---

## 📱 Commands

### 📌 Public

| Command | Description |
|--------|------------|
| `/start` | Initialize bot |
| `/help` | Help guide |
| `/add` | Add wallet |
| `/remove` | Remove wallet |
| `/list` | List wallets |
| `/balance` | Show balance |
| `/blocks` | Mined blocks |
| `/miner` | Hashrate |
| `/network` | Node status |
| `/dag` | DAG overview |
| `/price` | KAS price |
| `/market` | Market cap |
| `/supply` | Supply stats |
| `/fees` | Fee estimation |
| `/donate` | Support |

---

### 👑 Admin

| Command | Description |
|--------|------------|
| `/settings` | Control panel |
| `/toggle` | Toggle flags |
| `/stats` | Bot stats |
| `/sys` | System info |
| `/logs` | Logs |
| `/broadcast` | Message all |
| `/pause` | Pause workers |
| `/resume` | Resume |
| `/restart` | Restart |
| `/learn` | Add AI data |
| `/autolearn` | RSS sync |
| `/sync` | DAG rescan |

---

## ⚙️ Prerequisites

- Rust `1.70+`
- PostgreSQL `15+`
- Linux / Windows

### Ubuntu

```bash
sudo apt update
sudo apt install -y cmake build-essential pkg-config libssl-dev postgresql postgresql-contrib
````

---

## 🔐 Environment Setup

Create `.env`:

```env
BOT_TOKEN=your_telegram_bot_token_here
ADMIN_ID=your_telegram_user_id

WS_URL=ws://127.0.0.1:18110
DATABASE_URL=postgres://user:password@127.0.0.1:5432/kaspa_db
RUST_LOG=info

AI_API_KEY=your_key
AI_BASE_URL=https://api.groq.com/openai/v1
AI_CHAT_MODEL=llama-3.3-70b-versatile
AI_AUDIO_MODEL=whisper-large-v3

MAINTENANCE_MODE=false
ALLOW_PUBLIC_USERS=true
ENABLE_RSS_WORKER=true
ENABLE_MEMORY_CLEANER=true
ENABLE_LIVE_SYNC=true
ENABLE_AI_VECTORIZER=false
```

---

## 🛠️ Deployment

### 1. PostgreSQL

```bash
sudo -u postgres psql

CREATE DATABASE kaspa_db;
CREATE USER kaspa_admin WITH PASSWORD 'password';
GRANT ALL PRIVILEGES ON DATABASE kaspa_db TO kaspa_admin;

\c kaspa_db
CREATE EXTENSION vector;
\q
```

---

### 2. Build

```bash
git clone https://github.com/KaspaPulse/kaspa-gemini-intelligence.git
cd kaspa-gemini-intelligence
cargo build --release
```

---

### 3. Systemd Service

```bash
sudo nano /etc/systemd/system/kaspa-pulse.service
```

```ini
[Unit]
Description=Kaspa Pulse Bot
After=network.target postgresql.service

[Service]
User=your_username
WorkingDirectory=/home/your_username/kaspa-gemini-intelligence
ExecStart=/home/your_username/kaspa-gemini-intelligence/target/release/kaspa-solo
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable kaspa-pulse
sudo systemctl start kaspa-pulse
```

---

## 💖 Support

```
kaspa:qz0yqq8z3twwgg7lq2mjzg6w4edqys45w2wslz7tym2tc6s84580vvx9zr44g
```

---

## 📜 License

MIT License

---

## 🧠 Final Note

Built with precision, engineered with Rust, and designed for serious Kaspa miners.

⛏️ Happy Solo Mining

