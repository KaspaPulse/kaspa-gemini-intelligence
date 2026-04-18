use sqlx::{PgPool, Postgres};
use tracing::info;

/// Enterprise-Standard Keyword Clusters for Intent Detection
const NEWS_INTENT: &[&str] = &[
    "news",
    "update",
    "latest",
    "recent",
    "announcement",
    "release",
    "roadmap",
    "whats new",
    "خبر",
    "اخبار",
    "جديد",
    "اخر",
    "مستجدات",
    "تحديث",
    "تطورات",
    "اعلان",
];

const TECH_INTENT: &[&str] = &[
    "protocol",
    "algorithm",
    "mining",
    "consensus",
    "dagknight",
    "smart",
    "pow",
    "kheavyhash",
    "تقني",
    "بروتوكول",
    "خوارزمية",
    "تعدين",
    "اجماع",
    "بلوك",
    "داج",
];

const METRIC_INTENT: &[&str] = &[
    "hashrate",
    "price",
    "difficulty",
    "supply",
    "market",
    "daa",
    "tps",
    "bps",
    "سعر",
    "هاشريت",
    "صعوبة",
    "امداد",
    "سوق",
    "اداء",
    "احصائيات",
];

/// Smart RAG Engine: Retrieves relevant Kaspa knowledge based on user intent
pub async fn get_rag_context(pool: &PgPool, user_query: &str) -> String {
    let lower_query = user_query.to_lowercase();

    // Determine if the user is looking for time-sensitive news or metrics
    let is_news = NEWS_INTENT.iter().any(|&k| lower_query.contains(k));
    let is_metric = METRIC_INTENT.iter().any(|&k| lower_query.contains(k));
    let _is_tech = TECH_INTENT.iter().any(|&k| lower_query.contains(k));

    // Retrieval Strategy
    let result: Result<Vec<(String, String)>, sqlx::Error> = if is_news || is_metric {
        info!("[RAG] High-priority intent detected. Fetching latest global context...");
        sqlx::query_as::<Postgres, (String, String)>(
            "SELECT title, content FROM knowledge_base ORDER BY published_at DESC LIMIT 5",
        )
        .fetch_all(pool)
        .await
    } else {
        // Find the most significant word in the query
        let search_anchor = user_query
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .max_by_key(|w| w.len())
            .unwrap_or("kaspa");

        info!("[RAG] Specific search intent: '{}'", search_anchor);
        sqlx::query_as::<Postgres, (String, String)>(
            "SELECT title, content FROM knowledge_base WHERE content LIKE `$1 OR title LIKE `$1 ORDER BY published_at DESC LIMIT 3"
        )
        .bind(format!("%{}%", search_anchor))
        .fetch_all(pool).await
    };

    match result {
        Ok(articles) if !articles.is_empty() => {
            let mut context_buffer = String::from("\n[OFFICIAL KASPA KNOWLEDGE & UPDATES]:\n");
            for (title, content) in articles {
                let snippet = if content.len() > 300 {
                    &content[..300]
                } else {
                    &content
                };
                context_buffer.push_str(&format!("- Source: {}\n  Details: {}\n", title, snippet));
            }
            context_buffer
        }
        _ => {
            info!("[RAG] No relevant context found in database.");
            String::new()
        }
    }
}
