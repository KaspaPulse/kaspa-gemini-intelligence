use crate::context::AppContext;

use serde_json::json;
use std::process::Command;
use teloxide::net::Download;
use teloxide::prelude::*;

/// 🧠 Conversational Intent Processor
/// Handles text messages, restores SQLite history, and injects live node context.
pub async fn process_conversational_intent(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    msg_id: teloxide::types::MessageId,
    user_text: String,
    ctx: AppContext,
) -> anyhow::Result<()> {
    // 1. DATABASE MEMORY: Retrieve last 10 interactions for deep context
    let records: Vec<(String, String)> = sqlx::query_as(
        "SELECT role, message FROM (SELECT role, message, id FROM chat_history WHERE chat_id = ?1 ORDER BY id DESC LIMIT 10) ORDER BY id ASC"
    ).bind(chat_id.0).fetch_all(&ctx.pool).await.unwrap_or_default();

    let mut conversation_history = String::new();
    for (role, msg) in records {
        conversation_history.push_str(&format!("{}: {}\n", role.to_uppercase(), msg));
    }

    // 2. LIVE NODE INJECTION: Feed real-time DAG data into Gemini's "Brain"
    let mut live_context = String::from("[REAL-TIME NETWORK DATA]\n");
    if let Ok(dag) = ctx.rpc.get_block_dag_info().await {
        live_context.push_str(&format!(
            "Difficulty: {}, Block Count: {}. ",
            dag.difficulty, dag.block_count
        ));
    }
    let price = ctx.price_cache.read().await.0;
    live_context.push_str(&format!("KAS Price: ${:.4} USD.\n", price));

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}", api_key);

    let initial_msg = bot
        .send_message(
            chat_id,
            "⏳ <b>Kaspa Intelligence:</b> Recalling memory & scanning node...",
        )
        .reply_parameters(teloxide::types::ReplyParameters::new(msg_id))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    // 3. ENTERPRISE PROMPT: Strict rules similar to your old Qwen implementation
    let system_instruction = "You are the Kaspa Enterprise AI. 
    Rules: 
    - Use provided [REAL-TIME NETWORK DATA].
    - Refer to [CONVERSATION HISTORY] for continuity.
    - Kaspa is PoW BlockDAG; no PoS, no CEO.
    - Be factual, professional, and brief.";

    let full_prompt = format!(
        "{}\n\n{}\n\n[CONVERSATION HISTORY]\n{}\n\n[USER QUESTION]\n{}",
        system_instruction, live_context, conversation_history, user_text
    );

    let client = reqwest::Client::new();
    let payload = json!({ "contents": [{ "parts": [{ "text": full_prompt }] }] });

    if let Ok(res) = client.post(&url).json(&payload).send().await {
        if let Ok(res_json) = res.json::<serde_json::Value>().await {
            let ai_reply = res_json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("⚠️ Gemini API Error: No candidate returned.")
                .to_string();

            // 4. PERSISTENCE: Save interaction to SQLite for future recall
            let _ = sqlx::query(
                "INSERT INTO chat_history (chat_id, role, message) VALUES (?1, 'user', ?2)",
            )
            .bind(chat_id.0)
            .bind(&user_text)
            .execute(&ctx.pool)
            .await;
            let _ = sqlx::query(
                "INSERT INTO chat_history (chat_id, role, message) VALUES (?1, 'assistant', ?2)",
            )
            .bind(chat_id.0)
            .bind(&ai_reply)
            .execute(&ctx.pool)
            .await;

            bot.edit_message_text(
                chat_id,
                initial_msg.id,
                format!("🤖 <b>Kaspa Intelligence:</b>\n{}", ai_reply),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
    }
    Ok(())
}

/// 🎙️ Voice Intent Processor (Whisper Pipeline)
/// Downloads .ogg, converts to .wav via FFmpeg, and prepares for transcription.
pub async fn process_voice_intent(bot: Bot, msg: Message, _ctx: AppContext) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let voice = match msg.voice() {
        Some(v) => v,
        None => return Ok(()),
    };

    let initial_msg = bot
        .send_message(
            chat_id,
            "🎙️ <b>Processing Voice...</b>\n<i>Downloading audio...</i>",
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    let file = bot.get_file(voice.file.id.clone()).await?;
    let ogg_path = format!("temp_{}.ogg", voice.file.id);
    let wav_path = format!("temp_{}.wav", voice.file.id);

    let mut dst = tokio::fs::File::create(&ogg_path).await?;
    bot.download_file(&file.path, &mut dst).await?;

    // 🔄 AUDIO CONVERSION: FFmpeg transformation (Enterprise Standard)
    bot.edit_message_text(
        chat_id,
        initial_msg.id,
        "🎙️ <b>Processing Voice...</b>\n<i>Converting to high-fidelity WAV...</i>",
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;

    let status = Command::new("ffmpeg")
        .args(["-y", "-i", &ogg_path, "-ar", "16000", "-ac", "1", &wav_path])
        .status();

    match status {
        Ok(s) if s.success() => {
            bot.edit_message_text(
                chat_id,
                initial_msg.id,
                "✅ <b>Transcription Ready:</b> Audio processed successfully.",
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
        _ => {
            bot.edit_message_text(
                chat_id,
                initial_msg.id,
                "❌ <b>FFmpeg Error:</b> Audio conversion failed.",
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
    }

    // Cleanup temporary files
    let _ = tokio::fs::remove_file(&ogg_path).await;
    let _ = tokio::fs::remove_file(&wav_path).await;

    Ok(())
}
