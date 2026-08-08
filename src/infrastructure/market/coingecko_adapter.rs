use crate::domain::errors::AppError;
use chrono::{TimeZone, Utc};
use reqwest::Client;
use std::collections::BTreeMap;
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
        if let Some((data, timestamp)) = *self.cache.read().await
            && timestamp.elapsed() < Duration::from_secs(60)
        {
            return Ok(data);
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
    async fn get_kaspa_usd_history(
        &self,
        from_unix: i64,
        to_unix: i64,
    ) -> Result<Vec<(String, f64)>, AppError> {
        if from_unix >= to_unix {
            return Ok(Vec::new());
        }

        if !self.circuit_breaker.is_allowed() {
            return Err(crate::domain::errors::AppError::Internal(
                "Service Unavailable (Circuit Open)".to_string(),
            ));
        }

        let url = std::env::var("COINGECKO_MARKET_CHART_RANGE_URL").unwrap_or_else(|_| {
            "https://api.coingecko.com/api/v3/coins/kaspa/market_chart/range".to_string()
        });

        let from = from_unix.to_string();
        let to = to_unix.to_string();

        let res = self
            .client
            .get(&url)
            .query(&[
                ("vs_currency", "usd"),
                ("from", from.as_str()),
                ("to", to.as_str()),
            ])
            .header("User-Agent", "KaspaPulse/1.0")
            .send()
            .await
            .map_err(|e| {
                self.circuit_breaker.record_failure();
                crate::domain::errors::AppError::Internal(e.to_string())
            })?;

        let status = res.status();
        if !status.is_success() {
            self.circuit_breaker.record_failure();
            return Err(crate::domain::errors::AppError::Internal(format!(
                "CoinGecko history request failed with status {status}"
            )));
        }

        let json: serde_json::Value = res.json().await.map_err(|e| {
            self.circuit_breaker.record_failure();
            crate::domain::errors::AppError::Internal(e.to_string())
        })?;

        let prices = json
            .get("prices")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                self.circuit_breaker.record_failure();
                crate::domain::errors::AppError::Internal(
                    "CoinGecko history response missing prices".to_string(),
                )
            })?;

        let mut by_day: BTreeMap<String, f64> = BTreeMap::new();

        for point in prices {
            let Some(values) = point.as_array() else {
                continue;
            };

            if values.len() < 2 {
                continue;
            }

            let Some(timestamp_ms) = values[0].as_f64() else {
                continue;
            };

            let Some(price) = values[1].as_f64() else {
                continue;
            };

            if !(price.is_finite() && price > 0.0) {
                continue;
            }

            if let Some(datetime) = Utc.timestamp_millis_opt(timestamp_ms as i64).single() {
                by_day.insert(datetime.date_naive().format("%Y-%m-%d").to_string(), price);
            }
        }

        if by_day.is_empty() {
            self.circuit_breaker.record_failure();
            return Err(crate::domain::errors::AppError::Internal(
                "CoinGecko history response did not contain usable daily prices".to_string(),
            ));
        }

        self.circuit_breaker.record_success();

        Ok(by_day.into_iter().collect())
    }
}

// --- Merged Trait (Formerly in ports) ---

use async_trait::async_trait;

#[async_trait]
pub trait MarketProvider: Send + Sync {
    /// Returns (Price in USD, Market Cap)
    async fn get_kaspa_market_data(&self) -> Result<(f64, f64), AppError>;

    /// Returns stored-history candidates as (YYYY-MM-DD, price_usd).
    async fn get_kaspa_usd_history(
        &self,
        from_unix: i64,
        to_unix: i64,
    ) -> Result<Vec<(String, f64)>, AppError>;
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
