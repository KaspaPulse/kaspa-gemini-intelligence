# 🦀 Kaspa Pulse: The Ultimate Enterprise Miner's Companion

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?style=flat-square&logo=rust)
![Docker](https://img.shields.io/badge/Docker-Supported-2496ED.svg?style=flat-square&logo=docker)
![Kaspa](https://img.shields.io/badge/Kaspa-Network-70D4CB.svg?style=flat-square&logo=kaspa)
![Database](https://img.shields.io/badge/Database-PostgreSQL%20%2B%20pgvector-336791.svg?style=flat-square&logo=postgresql)
![AI-Powered](https://img.shields.io/badge/AI-Multi--LLM%20Support-8A2BE2.svg?style=flat-square)
![Architecture](https://img.shields.io/badge/Architecture-Parallel%20Streaming-success.svg?style=flat-square)
![State Management](https://img.shields.io/badge/State-DashMap%20RAM-blue.svg?style=flat-square)
![Standard](https://img.shields.io/badge/Standard-Zero%20Warnings-success.svg?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)

---

## 🚀 Overview

**Kaspa Pulse** (formerly Kaspa Solo) is an ultra-high-performance, enterprise-grade Telegram bot engineered entirely in Rust.

Built for **Kaspa Solo Miners** and **Full Node Operators**, it delivers:

- ⚡ Zero-latency notifications via direct wRPC  
- 🔐 Maximum privacy (no public APIs or external explorers)  
- 🧠 RAG AI-powered intelligence (Vector Search & Voice-to-Text)  
- 🤖 Multi-LLM Support (Switch between Groq, DeepSeek, OpenAI, Claude, Gemini live)
- 🔬 Deep GHOSTDAG blockchain forensics (Nonce & Worker extraction)  
- 🐘 High-Performance PostgreSQL State Management  
- 🛡️ Anti-Flood, Rate Limited & Prompt-Injection Hardened  
- ⚙️ **Dynamic Enterprise Control Panel (Zero-Downtime Configuration)** It connects directly to your node via **wRPC WebSocket**, ensuring raw, unindexed, real-time blockchain data.

---

## ✨ Core & Enterprise Features

### 🧠 Advanced AI Intelligence & Autonomous Agents
- **Multi-Engine Switching:** Change the active conversational model instantly via `/models` (Llama 3, DeepSeek V2, GPT-4o, Claude 3.5, Gemini Pro).
- **Voice-to-Text Analytics:** Send a voice note (OGG) directly to the bot. It transcribes the audio using Whisper-Large-V3.
- **Context-Aware RAG:** Uses live wallet balance, DAA score, price + `pgvector` database.
- **Autonomous Web Research:** Integrated Tavily agent for Kaspa news aggregation.

### ⚙️ Dynamic Enterprise Control Panel
Manage everything live from Telegram via the `/settings` UI:
- Toggle **Maintenance Mode** or **Webhooks** natively.
- Enable/Disable workers (RSS, Memory Cleaner, AI Chat, AI Voice).
- Visual UI with 3-column inline buttons and active status indicators (🟢/🔴).

### 🛡️ Smart Node Safety Protocol (Anti-Ban)
- Detects Local vs Public Node.
- Prevents heavy operations on public nodes.
- Keeps real-time tracking safe.

### 🎯 Deterministic Block Detection & Forensics
- Reverse DAG traversal and Byte-level payload scanning.
- 100% accurate reward attribution.
- Extracts **Nonce** & **Worker ID**.

### 🕒 Smart UTXO Processing
- Parallel processing (`tokio::task::JoinSet`).
- Sorted by `block_time_ms` with zero message desync.

### ⛏️ Hashrate Estimation
- 1H / 24H / 7D analysis based on real mined rewards and network difficulty.

---

## 📖 How to Use

1. Send `/start`
2. Paste your Kaspa address
3. Use buttons or commands
4. Chat with AI or send voice

---

## 📱 Commands Architecture

### 📌 Public
| Command | Description |
|---|---|
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
| `/hidemenu` | Hide inline keyboards |

### 👑 Admin
| Command | Description |
|---|---|
| `/settings` | Enterprise Control Panel |
| `/models` | AI Model Switcher UI |
| `/stats` | Bot & System stats |
| `/sys` | Hardware Diagnostics |
| `/db_diag` | Database Health Check |
| `/logs` | View internal logs |
| `/broadcast` | Message all users |
| `/pause` | Suspend specific workers |
| `/resume` | Resume operations |
| `/restart` | Safely Reboot Engine |
| `/sync` | Force DAG rescan |
| `/forget_all` | GDPR Database Wipe |

---

## 🔐 Environment Setup (`.env`)

Create `.env` in the root directory. 

```env
# ==============================================================================
# 🤖 TELEGRAM BOT CONFIGURATION
# ==============================================================================
BOT_TOKEN=your_telegram_bot_token_here
ADMIN_ID=your_telegram_user_id
ADMIN_PIN=778899

# ==============================================================================
# ⚙️ NODE, DATABASE & CACHE
# ==============================================================================
NODE_URL_01=wss://your_node_[url.com/json](https://url.com/json)
NODE_URL_02=ws://127.0.0.1:18110
DATABASE_URL=postgres://user:password@127.0.0.1:5432/kaspa_db?sslmode=disable
REDIS_URL=redis://127.0.0.1:6379

# ==============================================================================
# 🧠 AI ENGINES & AUTONOMOUS AGENT
# ==============================================================================
AI_CHAT_API_KEY=your_groq_key
AI_CHAT_BASE_URL=[https://api.groq.com/openai/v1](https://api.groq.com/openai/v1)
AI_CHAT_MODEL=llama-3.3-70b-versatile

DEEPSEEK_API_KEY=your_deepseek_key
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key
GEMINI_API_KEY=your_gemini_key

AI_AUDIO_API_KEY=your_groq_key
AI_AUDIO_BASE_URL=[https://api.groq.com/openai/v1](https://api.groq.com/openai/v1)
AI_AUDIO_MODEL=whisper-large-v3

TAVILY_API_KEY=your_tavily_key

# ==============================================================================
# 🔗 EXTERNAL APIs & KNOWLEDGE BASE
# ==============================================================================
COINGECKO_API_URL=[https://api.coingecko.com/api/v3/simple/price?ids=kaspa&vs_currencies=usd&include_market_cap=true](https://api.coingecko.com/api/v3/simple/price?ids=kaspa&vs_currencies=usd&include_market_cap=true)
RSS_FEEDS=[https://medium.com/feed/kaspa-currency,https://github.com/kaspanet/rusty-kaspa/releases.atom](https://medium.com/feed/kaspa-currency,https://github.com/kaspanet/rusty-kaspa/releases.atom)

# ==============================================================================
# 🛡️ SYSTEM & ENTERPRISE SECURITY
# ==============================================================================
RUST_LOG=info,kaspa_solo=debug
ENCRYPTION_MASTER_KEY=your_secure_hex_key

# --- Webhook Config ---
WEBHOOK_DOMAIN=api.yourdomain.com
WEBHOOK_PORT=8443

🛠️ Deployment Options
Option A: Docker Compose (Recommended for Enterprise)
The repository already includes a fully configured docker-compose.yml. Just ensure your .env is set up, then run:

Bash
docker-compose up -d --build

Option B: Manual Linux Deployment

1. Prerequisites (Ubuntu)

Bash
sudo apt update
sudo apt install -y cmake build-essential pkg-config libssl-dev postgresql postgresql-contrib redis-server

2. PostgreSQL Setup

Bash
sudo -u postgres psql
CREATE DATABASE kaspa_db;
CREATE USER user WITH PASSWORD 'password';
GRANT ALL PRIVILEGES ON DATABASE kaspa_db TO user;
\c kaspa_db
CREATE EXTENSION vector;
\q

3. Build from Source

Bash
git clone https://github.com/KaspaPulse/kaspa-telegram-notify.git
cd kaspa-telegram-notify
git checkout dev
cargo build --release

4. Systemd Service

Bash
sudo nano /etc/systemd/system/kaspa-pulse.service
Ini, TOML
[Unit]
Description=Kaspa Pulse Bot
After=network.target postgresql.service redis.service

[Service]
User=root
WorkingDirectory=/path/to/kaspa-telegram-notify
ExecStart=/path/to/kaspa-telegram-notify/target/release/kaspa-pulse
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
Bash
sudo systemctl daemon-reload
sudo systemctl enable kaspa-pulse
sudo systemctl start kaspa-pulse

Option C: Manual Windows Deployment

1. Prerequisites (Windows)
Install Rust (rustup).

Install Build Tools for Visual Studio (Ensure "Desktop development with C++" is selected during installation).

Install Git for Windows.

Install PostgreSQL for Windows (and compile/install the pgvector extension).

Install Redis (via WSL2, Docker Desktop, or Memurai).

2. Database Setup
Open psql (or pgAdmin) and run:

SQL
CREATE DATABASE kaspa_db;
CREATE USER user WITH PASSWORD 'password';
GRANT ALL PRIVILEGES ON DATABASE kaspa_db TO user;
\c kaspa_db
CREATE EXTENSION vector;

3. Build and Run (PowerShell)
Open PowerShell and execute:

PowerShell
git clone https://github.com/KaspaPulse/kaspa-telegram-notify.git
cd kaspa-telegram-notify
git checkout dev

# Build the enterprise release
cargo build --release

# Run the bot engine
.\target\release\kaspa-pulse.exe

💖 Support
kaspa:qz0yqq8z3twwgg7lq2mjzg6w4edqys45w2wslz7tym2tc6s84580vvx9zr44g

📜 License
MIT License

🧠 Final Note
Built with precision, engineered with Rust, and designed for serious Kaspa miners.

⛏️ Happy Solo Mining