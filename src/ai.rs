#![allow(dead_code)]
use crate::context::AppContext;
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_qwen2::ModelWeights;
use hf_hub::{api::sync::Api, Repo, RepoType};
use kaspa_addresses::Address;
use kaspa_rpc_core::api::rpc::RpcApi;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use teloxide::net::Download;
use teloxide::prelude::*;
use tokenizers::Tokenizer;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct LocalAiEngine {
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

impl LocalAiEngine {
    pub fn new() -> anyhow::Result<Self> {
        tracing::info!("[AI ENGINE] Initializing Local Qwen Engine...");
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
        tracing::info!("[AI ENGINE] Using device: {:?}", device);

        let api = Api::new()?;
        let model_repo = api.repo(Repo::with_revision(
            "Qwen/Qwen2.5-0.5B-Instruct-GGUF".to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        let model_path = model_repo.get("qwen2.5-0.5b-instruct-q4_k_m.gguf")?;

        let tokenizer_repo = api.repo(Repo::with_revision(
            "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        let tokenizer_path = tokenizer_repo.get("tokenizer.json")?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let mut file = std::fs::File::open(&model_path)?;
        let content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(anyhow::Error::msg)?;
        let model =
            ModelWeights::from_gguf(content, &mut file, &device).map_err(anyhow::Error::msg)?;

        tracing::info!("[AI ENGINE] Model loaded successfully.");
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn generate(&mut self, prompt: &str, live_context: &str) -> anyhow::Result<String> {
        let rag_context = crate::rag::search_kaspa_docs(prompt);
                let system_message = format!("You are an uncompromisingly accurate Kaspa AI Assistant.
[ABSOLUTE RULES]
1. DO NOT invent, hallucinate, or assume facts. If the exact answer is not in the [KNOWLEDGE BASE] or [LIVE DATA], reply ONLY with: 'I don't have enough information to answer that.'
2. Kaspa is a PoW BlockDAG. It has NO CEO, NO Proof of Stake (PoS), and NO Ethereum-style Smart Contracts. Reject any claims otherwise.
3. NEVER write code unless explicitly asked to write Rust scripts for Kaspa nodes.
4. DO NOT repeat sentences. If you find yourself looping, STOP generating immediately.
5. Keep answers highly professional, factual, and brief.

[LIVE DATA]
{}

[KNOWLEDGE BASE]
{}", live_context, rag_context);
        let formatted_prompt = format!("<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", system_message, prompt);

        let tokens = self
            .tokenizer
            .encode(formatted_prompt, true)
            .map_err(anyhow::Error::msg)?;
        let mut tokens_vec = tokens.get_ids().to_vec();
        let mut generated_text = String::new();
        let mut logits_processor = LogitsProcessor::new(299792458, Some(0.2), None);
        let eos_token = self.tokenizer.token_to_id("<|im_end|>").unwrap_or(151645);

        tracing::info!("⚙️ [AI ENGINE] Starting token generation loop...");

        for index in 0..512 {
            let context_size = if index > 0 { 1 } else { tokens_vec.len() };
            let start_pos = tokens_vec.len().saturating_sub(context_size);
            let context = &tokens_vec[start_pos..];

            let input_tensor = Tensor::new(context, &self.device)?.unsqueeze(0)?;
            let logits = self
                .model
                .forward(&input_tensor, tokens_vec.len() - context.len())?
                .squeeze(0)?;
            let next_token = logits_processor.sample(&logits)?;
            tokens_vec.push(next_token);

            if (index + 1) % 10 == 0 {
                tracing::info!("⏳ [AI ENGINE] Generated {} tokens so far...", index + 1);
            }
            if next_token == eos_token {
                break;
            }
            if let Ok(text) = self.tokenizer.decode(&[next_token], true) {
                generated_text.push_str(&text);
            }
        }
        Ok(generated_text.trim().to_string())
    }
}

pub async fn inject_live_wallet_context(chat_id: i64, ctx: &crate::context::AppContext) -> String {
    let mut live_data = String::new();

    // 1. Fetch Network Stats (Always available)
    if let Ok(dag_info) = ctx.rpc.get_block_dag_info().await {
        live_data.push_str(&format!(
            "Network Difficulty: {}. \n",
            crate::kaspa_features::format_difficulty(dag_info.difficulty)
        ));
        live_data.push_str(&format!("DAA Score: {}. \n", dag_info.virtual_daa_score));
    }
    if let Ok(hashrate) = ctx.rpc.estimate_network_hashes_per_second(1000, None).await {
        live_data.push_str(&format!(
            "Network Hashrate: {}. \n",
            crate::kaspa_features::format_hashrate(hashrate as f64)
        ));
    }

    // 2. Fetch KAS Price
    let price = ctx.price_cache.read().await.0;
    if price > 0.0 {
        live_data.push_str(&format!("KAS Price: ${:.4} USD. \n", price));
    }

    // 3. Fetch User Wallet Balance
    let wallets: Vec<String> = ctx
        .state
        .iter()
        .filter(|e| e.value().contains(&chat_id))
        .map(|e| e.key().clone())
        .collect();
    if wallets.is_empty() {
        live_data.push_str("User Balance: 0 KAS (No wallet tracked).\n");
    } else {
        let mut total = 0.0;
        for w in &wallets {
            if let Ok(addr) = Address::try_from(w.as_str()) {
                if let Ok(utxos) = ctx.rpc.get_utxos_by_addresses(vec![addr]).await {
                    let bal = utxos
                        .iter()
                        .map(|u| u.utxo_entry.amount as f64)
                        .sum::<f64>()
                        / 1e8;
                    total += bal;
                }
            }
        }
        live_data.push_str(&format!("User Balance: {:.8} KAS.\n", total));
    }

    live_data
}

pub type SharedAiEngine = Arc<Mutex<LocalAiEngine>>;

fn is_english_only(text: &str) -> bool {
    !text
        .chars()
        .any(|c| c.is_alphabetic() && !c.is_ascii_alphabetic())
}

pub async fn process_conversational_intent(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    msg_id: teloxide::types::MessageId,
    _user_id: i64,
    user_text: String,
    ctx: AppContext,
) -> anyhow::Result<()> {
    tracing::info!(
        "📥 [AI IN] Chat: {} | Text received: {}",
        chat_id,
        user_text
    );

    if !is_english_only(&user_text) {
        tracing::warn!("⚠️ [FILTER] Blocked non-English input. Rejecting immediately.");
        let _ = bot
            .send_message(
                chat_id,
                "⚠️ <b>Notice:</b> Please ask your question in English only.",
            )
            .reply_parameters(teloxide::types::ReplyParameters::new(msg_id))
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    let initial_msg = bot
        .send_message(
            chat_id,
            "⏳ <b>Kaspa AI:</b> Analyzing knowledge base... (0s)",
        )
        .reply_parameters(teloxide::types::ReplyParameters::new(msg_id))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    let progress_msg_id = initial_msg.id;
    let bot_clone = bot.clone();

    let progress_task = tokio::spawn(async move {
        let mut seconds = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
            seconds += 4;
            let _ = bot_clone
                .edit_message_text(
                    chat_id,
                    progress_msg_id,
                    format!(
                        "⏳ <b>Kaspa AI is thinking...</b>\n<i>Generating response... ({}s)</i>",
                        seconds
                    ),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await;
            let _ = bot_clone
                .send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
                .await;
        }
    });

        let engine_arc = ctx.ai_engine.clone();
    tracing::info!("🧠 [AI ENGINE] Retrieving context and thinking...");

    // 1. Initialize SQLite Memory Table & Fetch History
    let _ = sqlx::query("CREATE TABLE IF NOT EXISTS chat_history (id INTEGER PRIMARY KEY AUTOINCREMENT, chat_id INTEGER, role TEXT, message TEXT, timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)").execute(&ctx.pool).await;
    let records: Result<Vec<(String, String)>, _> = sqlx::query_as("SELECT role, message FROM (SELECT role, message, timestamp FROM chat_history WHERE chat_id = ?1 ORDER BY id DESC LIMIT 6) ORDER BY id ASC").bind(chat_id.0).fetch_all(&ctx.pool).await;
    
    let mut history_str = String::new();
    if let Ok(rows) = records {
        for (role, msg) in rows {
            history_str.push_str(&format!("{}: {}\n", role.to_uppercase(), msg));
        }
    }
    
    let enriched_prompt = if history_str.is_empty() {
        user_text.clone()
    } else {
        format!("[CONVERSATION HISTORY]\n{}\n\n[CURRENT QUESTION]\n{}", history_str, user_text)
    };

    let live_ctx_data = inject_live_wallet_context(chat_id.0, &ctx).await;
    let user_text_for_db = user_text.clone();
    
    // 2. Generate Context-Aware Response
    let response = match tokio::task::spawn_blocking(move || {
        let mut engine = engine_arc.lock().unwrap();
        engine.generate(&enriched_prompt, &live_ctx_data)
    })
    .await?
    {
        Ok(text) => {
            tracing::info!("✅ [AI OUT] Generated Context-Aware Reply");
            // 3. Save Interactions to SQLite
            let _ = sqlx::query("INSERT INTO chat_history (chat_id, role, message) VALUES (?1, 'user', ?2)").bind(chat_id.0).bind(&user_text_for_db).execute(&ctx.pool).await;
            let _ = sqlx::query("INSERT INTO chat_history (chat_id, role, message) VALUES (?1, 'assistant', ?2)").bind(chat_id.0).bind(&text).execute(&ctx.pool).await;
            format!("🤖 <b>Kaspa AI:</b>\n{}", text)
        }
        Err(e) => format!("⚠️ <b>AI Error:</b>\n{}", e),
    };

    progress_task.abort();
    let _ = bot
        .edit_message_text(chat_id, progress_msg_id, response)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await;
    Ok(())
}

// --- NEW: OFFLINE WHISPER INTEGRATION ---
pub async fn process_voice_message(bot: Bot, msg: Message, ctx: AppContext) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    tracing::info!(
        "🎙️ [VOICE IN] Received voice message from chat: {}",
        chat_id
    );

    let voice = match msg.voice() {
        Some(v) => v,
        None => return Ok(()),
    };

    let initial_msg = bot
        .send_message(chat_id, "⏳ <b>Kaspa AI:</b> Downloading audio... (0s)")
        .reply_parameters(teloxide::types::ReplyParameters::new(msg.id))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    let progress_msg_id = initial_msg.id;
    let bot_clone = bot.clone();

    let progress_task = tokio::spawn(async move {
        let mut seconds = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
            seconds += 4;
            let _ = bot_clone
                .edit_message_text(
                    chat_id,
                    progress_msg_id,
                    format!(
                        "⏳ <b>Kaspa AI is processing voice...</b>\n<i>Analyzing... ({}s)</i>",
                        seconds
                    ),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await;
            let _ = bot_clone
                .send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
                .await;
        }
    });

    let file = bot.get_file(voice.file.id.clone()).await?;
    let ogg_path = format!("temp_{}.ogg", voice.file.id);
    let wav_path = format!("temp_{}.wav", voice.file.id);

    let mut dst = tokio::fs::File::create(&ogg_path).await?;
    bot.download_file(&file.path, &mut dst).await?;

    tracing::info!("🔄 [VOICE] Converting audio using ffmpeg...");
    let ffmpeg_status = Command::new("ffmpeg")
        .args([
            "-y", "-i", &ogg_path, "-ar", "16000", "-ac", "1", "-f", "wav", &wav_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !ffmpeg_status.success() {
        progress_task.abort();
        let _ = bot
            .edit_message_text(
                chat_id,
                progress_msg_id,
                "⚠️ <b>Error:</b> Audio conversion failed.",
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await;
        let _ = tokio::fs::remove_file(&ogg_path).await;
        return Ok(());
    }

    tracing::info!("🎵 [VOICE] Executing Offline Whisper Transcription...");

    // Run Heavy Whisper & Qwen operations in a blocking thread
    let engine_arc = ctx.ai_engine.clone();
    let wav_path_clone = wav_path.clone();
    let live_ctx_data_clone = inject_live_wallet_context(chat_id.0, &ctx).await;

    let response = match tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        // 1. Ensure Whisper Model is downloaded (tiny.en is very fast and strict English)
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            "ggerganov/whisper.cpp".to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        let model_path = repo.get("ggml-tiny.en.bin")?;

        // 2. Load Audio Samples and Normalize to f32 (-1.0 to 1.0)
        let mut reader = hound::WavReader::open(&wav_path_clone)?;
        let audio_data: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();

        // 3. Initialize Whisper Context
        let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        // 4. Set Whisper to strict English
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // 5. Run transcription
        state
            .full(params, &audio_data)
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        let mut transcribed_text = String::new();
        let num_segments = state.full_n_segments();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment
                    .to_str_lossy()
                    .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                transcribed_text.push_str(&text);
            }
        }

        let clean_text = transcribed_text.trim().to_string();
        tracing::info!("📝 [VOICE TRANSCRIBED] Text: {}", clean_text);

        // If Whisper heard silence or noise, abort
        if clean_text.is_empty() {
            return Ok("🎙️ <i>Audio was unclear or silent.</i>".to_string());
        }

        // 6. Pass transcribed text to Qwen!
        tracing::info!("🧠 [AI ENGINE] Feeding voice transcription to Qwen...");
        let mut engine = engine_arc.lock().unwrap();
        let qwen_reply = engine.generate(&clean_text, &live_ctx_data_clone)?;
        tracing::info!("✅ [AI OUT] Generated Reply: {}", qwen_reply);

        Ok(format!(
            "🎙️ <b>Transcribed:</b> <i>\"{}\"</i>\n\n🤖 <b>Kaspa AI:</b>\n{}",
            clean_text, qwen_reply
        ))
    })
    .await?
    {
        Ok(final_text) => final_text,
        Err(e) => {
            tracing::error!("❌ [VOICE/AI ERROR] {}", e);
            format!("⚠️ <b>Voice Error:</b>\n{}", e)
        }
    };

    // Clean up
    let _ = tokio::fs::remove_file(&ogg_path).await;
    let _ = tokio::fs::remove_file(&wav_path).await;

    progress_task.abort();
    let _ = bot
        .edit_message_text(chat_id, progress_msg_id, response)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await;

    Ok(())
}
