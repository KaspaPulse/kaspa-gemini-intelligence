use sqlx::{PgPool, Postgres};
use tracing::{info, warn};

/// Intent keywords for dynamic live search
const NEWS_INTENT: &[&str] = &[
    "news", "update", "latest", "recent", "announcement", "release", "whats new",
    "خبر", "اخبار", "جديد", "تحديث", "مستجدات"
];

const METRIC_INTENT: &[&str] = &[
    "hashrate", "price", "difficulty", "market", "daa", "tps", "bps",
    "سعر", "صعوبة", "هاشريت", "احصائيات"
];

/// Enterprise Autonomous RAG Engine
/// Logic: Live Search First -> Store -> Prune Old Data -> Respond
pub async fn get_rag_context(pool: &PgPool, user_query: &str) -> String {
    let lower_query = user_query.to_lowercase();
    let is_news = NEWS_INTENT.iter().any(|&k| lower_query.contains(k));
    let is_metric = METRIC_INTENT.iter().any(|&k| lower_query.contains(k));

    info!("[RAG] Processing Autonomous Query: '{}'", user_query);

    // 1. FORCED EXTERNAL PRIORITY: If user asks for news/metrics, bypass local DB
    if is_news || is_metric {
        info!("[RAG] News/Metric Intent: Bypassing local cache for fresh Tavily data.");
        return trigger_autonomous_agent(pool, user_query).await;
    }

    // 2. SMART LOCAL SEARCH: For technical architecture and manual instructions
    let search_anchor = user_query
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .max_by_key(|w| w.len())
        .unwrap_or("kaspa");

    let result = sqlx::query_as::<Postgres, (String, String)>(
        "SELECT title, content FROM knowledge_base 
         WHERE content ILIKE $1 OR title ILIKE $1 
         ORDER BY 
            CASE WHEN title LIKE 'Manual Input%' THEN 0 ELSE 1 END, 
            published_at DESC 
         LIMIT 5"
    )
    .bind(format!("%{}%", search_anchor))
    .fetch_all(pool)
    .await;

    match result {
        Ok(articles) if !articles.is_empty() => {
            info!("[RAG] Valid Local Data found. Using cache.");
            let mut context = String::from("\n[VERIFIED LOCAL KNOWLEDGE]:\n");
            for (title, content) in articles {
                let snippet = if content.len() > 700 { &content[..700] } else { &content };
                context.push_str(&format!("- Source: {}\n  Details: {}\n", title, snippet));
            }
            context
        }
        // 3. AUTO-FALLBACK: If local search yields nothing, force the agent
        _ => {
            info!("[RAG] Local DB silent. Engaging Agent as fallback.");
            trigger_autonomous_agent(pool, user_query).await
        }
    }
}

/// Invokes Tavily, Updates Database, and Prunes Obsolete Entries
async fn trigger_autonomous_agent(pool: &PgPool, query: &str) -> String {
    if let Some(agent_answer) = crate::agent::search_and_learn(pool, query).await {
        
        // --- DATA PRUNING PROTOCOL ---
        // Keeps only the most recent entry for each unique title to avoid "Data Clutter"
        let prune_result = sqlx::query(
            "DELETE FROM knowledge_base 
             WHERE id NOT IN (
                SELECT id FROM (
                    SELECT id, ROW_NUMBER() OVER (PARTITION BY title ORDER BY published_at DESC) as rn 
                    FROM knowledge_base
                ) t WHERE t.rn = 1
             )"
        ).execute(pool).await;

        match prune_result {
            Ok(_) => info!("[RAG] Database Cleaned. Obsolete duplicates removed."),
            Err(e) => warn!("[RAG] Pruning failed: {:?}", e),
        }

        format!("\n[AUTONOMOUS LIVE INTELLIGENCE]:\n{}\n", agent_answer)
    } else {
        warn!("[RAG] Autonomous Agent failed to fetch live data.");
        String::new()
    }
}