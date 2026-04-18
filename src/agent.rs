use reqwest::Client;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::env;
use tracing::{error, info};

pub async fn search_and_learn(pool: &PgPool, query: &str) -> Option<String> {
    let api_key = env::var("TAVILY_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        error!("[AI AGENT] CRITICAL: TAVILY_API_KEY is missing!");
        return None;
    }

    info!("[AI AGENT] Fetching live intelligence from internet for: {}", query);
    
    let client = Client::new();
    let res = client.post("https://api.tavily.com/search")
        .json(&json!({
            "api_key": api_key,
            "query": format!("Kaspa network official technical info: {}", query),
            "search_depth": "advanced",
            "include_answer": true,
            "days": 30
        }))
        .send()
        .await;

    match res {
        Ok(response) => {
            if let Ok(body) = response.json::<Value>().await {
                if let Some(answer) = body.get("answer").and_then(|a| a.as_str()) {
                    let answer_str = answer.to_string();
                    let safe_link = format!("tavily://search/{}", query.replace(" ", "_"));
                    
                    crate::state::add_to_knowledge_base(
                        pool, 
                        query, 
                        &safe_link, 
                        &answer_str, 
                        "Autonomous Internet Search"
                    ).await;

                    info!("[AI AGENT] Memory Synchronized: Learned new facts about '{}'", query);
                    return Some(answer_str);
                }
            }
        }
        Err(e) => error!("[AI AGENT] External network error: {}", e),
    }
    None
}
