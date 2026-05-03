use crate::domain::errors::AppError;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// Type aliases to satisfy clippy::type_complexity and improve readability
type MarketData = (f64, f64);
type CachedEntry = Option<(MarketData, Instant)>;
type SharedCache = Arc<RwLock<CachedEntry>>;

pub struct CoinGeckoAdapter {
    client: Client,
    cache: SharedCache,
    circuit_breaker: crate::infrastructure::resilience::circuit_breaker::CircuitBreaker,
}

impl Default for CoinGeckoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CoinGeckoAdapter {
    pub fn new() -> Self {
        Self {
            client: build_http_client(),
            cache: Arc::new(RwLock::new(None)),
            circuit_breaker:
                crate::infrastructure::resilience::circuit_breaker::CircuitBreaker::new(3, 300), // 3 failures = block for 5 minutes
        }
    }
}

#[async_trait]
impl MarketProvider for CoinGeckoAdapter {
    async fn get_kaspa_market_data(&self) -> Result<(f64, f64), AppError> {
        // 1. Check cache: Return data if it is younger than 60 seconds to prevent API rate limiting [cite: 1149]
        if let Some((data, timestamp)) = *self.cache.read().await {
            if timestamp.elapsed() < Duration::from_secs(60) {
                return Ok(data);
            }
        }

        // 2. Fetch API URL from environment or use production default [cite: 1150]
        let url = match std::env::var("COINGECKO_API_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => {
                tracing::warn!(
                    "[COINGECKO] COINGECKO_API_URL is missing. Serving stale cache if available."
                );

                if let Some((data, _)) = *self.cache.read().await {
                    return Ok(data);
                }

                self.circuit_breaker.record_failure();
                return Err(crate::domain::errors::AppError::Internal(
                    "COINGECKO_API_URL is missing and no stale market cache is available"
                        .to_string(),
                ));
            }
        };

        // 3. Execute request with proper User-Agent [cite: 1151]
        if !self.circuit_breaker.is_allowed() {
            tracing::warn!(
                "⚡ [API BLOCKED] Circuit Breaker is OPEN. Serving stale cache if available..."
            );
            if let Some((data, _)) = *self.cache.read().await {
                return Ok(data);
            } else {
                return Err(crate::domain::errors::AppError::Internal(
                    "Service Unavailable (Circuit Open)".to_string(),
                ));
            }
        }

        let res = self
            .client
            .get(&url)
            .header("User-Agent", "KaspaPulse/1.0")
            .send()
            .await
            .map_err(|e| {
                self.circuit_breaker.record_failure();
                crate::domain::errors::AppError::Internal(e.to_string())
            })?;

        // 4. Parse JSON response [cite: 1152]
        let json: serde_json::Value = res.json().await.map_err(|e| {
            self.circuit_breaker.record_failure();
            crate::domain::errors::AppError::Internal(e.to_string())
        })?;

        self.circuit_breaker.record_success();
        let price = json["kaspa"]["usd"].as_f64().unwrap_or(0.0);
        let mcap = json["kaspa"]["usd_market_cap"].as_f64().unwrap_or(0.0);

        // 5. Update shared cache with fresh data and current timestamp [cite: 1153]
        let mut cache_write = self.cache.write().await;
        *cache_write = Some(((price, mcap), Instant::now()));

        Ok((price, mcap))
    }
}

// --- Merged Trait (Formerly in ports) ---

use async_trait::async_trait;

#[async_trait]
pub trait MarketProvider: Send + Sync {
    /// Returns (Price in USD, Market Cap)
    async fn get_kaspa_market_data(&self) -> Result<(f64, f64), AppError>;
}
fn build_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(env_u64("HTTP_TIMEOUT_SECS", 10)))
        .connect_timeout(Duration::from_secs(env_u64("HTTP_CONNECT_TIMEOUT_SECS", 5)))
        .user_agent("KaspaPulse/1.2")
        .build()
        .expect("failed to build HTTP client")
}

fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default_value)
}
