//! Prediction market engine
//!
//! Connects to Clickhouse for historical prediction market data.

use chrono::{DateTime, Utc};
use clickhouse::Client;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::PredictionConfig;
use crate::error::{validate_interval, validate_slug, CortexMcpError, Result};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MarketTrendResponse {
    pub slug: String,
    pub platform: String,
    pub interval: String,
    pub data_points: usize,
    pub ohlcv: Vec<OhlcvData>,
    pub summary: TrendSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct OhlcvData {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume_usd: f64,
    pub trades: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendSummary {
    pub price_change: f64,
    pub price_change_pct: f64,
    pub high: f64,
    pub low: f64,
    pub total_volume: f64,
    pub trend_direction: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeProfileResponse {
    pub slug: String,
    pub platform: String,
    pub volume_24h: f64,
    pub volume_7d: f64,
    pub trades_24h: u64,
    pub unique_traders_24h: u64,
    pub avg_trade_size: f64,
    pub liquidity: LiquidityInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidityInfo {
    pub bid_depth_usd: f64,
    pub ask_depth_usd: f64,
    pub spread: f64,
    pub spread_bps: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchMemoryResponse {
    pub query: String,
    pub results_count: usize,
    pub markets: Vec<MarketSearchItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketSearchItem {
    pub slug: String,
    pub platform: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub current_price: f64,
    pub volume_24h: f64,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalyResponse {
    pub slug: String,
    pub threshold_std_dev: f64,
    pub anomalies_found: usize,
    pub anomalies: Vec<AnomalyItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalyItem {
    pub timestamp: String,
    pub price: f64,
    pub mean_price: f64,
    pub z_score: f64,
    pub deviation_pct: f64,
    pub direction: String,
    pub outcome_token: String,
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
pub struct OhlcvRow {
    pub interval_start: i64,
    pub open_price: String,
    pub high_price: String,
    pub low_price: String,
    pub close_price: String,
    pub volume_usd: String,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
pub struct VolumeProfileRow {
    pub slug: String,
    pub platform: String,
    pub total_volume_24h: String,
    pub total_volume_7d: String,
    pub total_trades_24h: u64,
    pub unique_traders_24h: u64,
    pub avg_trade_size: String,
    pub bid_depth_usd: String,
    pub ask_depth_usd: String,
    pub spread: String,
}

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
pub struct MarketSearchRow {
    pub slug: String,
    pub platform: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub current_price: String,
    pub volume_24h: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
pub struct AnomalyRow {
    pub timestamp: i64,
    pub price: String,
    pub mean_price: String,
    pub z_score: f64,
    pub deviation_pct: String,
    pub outcome_token: String,
}

// =============================================================================
// Prediction Engine
// =============================================================================

/// Prediction market data engine
pub struct PredictionEngine {
    client: Client,
    cache: Cache<String, serde_json::Value>,
}

impl PredictionEngine {
    /// Create a new prediction engine
    pub async fn new(config: &PredictionConfig) -> Result<Self> {
        let client = Client::default()
            .with_url(&config.clickhouse_url)
            .with_database(&config.database);

        // Verify connection
        client
            .query("SELECT 1")
            .fetch_one::<u8>()
            .await
            .map_err(|e| CortexMcpError::Database(e.to_string()))?;

        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        Ok(Self { client, cache })
    }

    /// Get market trend with OHLCV data
    pub async fn get_market_trend(
        &self,
        slug: &str,
        interval: &str,
    ) -> Result<MarketTrendResponse> {
        validate_slug(slug)?;
        validate_interval(interval)?;

        let cache_key = format!("trend:{}:{}", slug, interval);
        if let Some(cached) = self.cache.get(&cache_key).await {
            if let Ok(response) = serde_json::from_value(cached) {
                return Ok(response);
            }
        }

        let interval_seconds = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "30m" => 1800,
            "1h" => 3600,
            "4h" => 14400,
            "24h" => 86400,
            "7d" => 604800,
            _ => 3600,
        };

        let query = format!(
            r#"
            SELECT
                toInt64(toUnixTimestamp(toStartOfInterval(timestamp, INTERVAL {} SECOND)) * 1000) as interval_start,
                toString(argMin(price, timestamp)) as open_price,
                toString(max(price)) as high_price,
                toString(min(price)) as low_price,
                toString(argMax(price, timestamp)) as close_price,
                toString(sum(size_usd)) as volume_usd,
                count() as trade_count
            FROM trades
            WHERE slug = ?
              AND timestamp > now() - INTERVAL 7 DAY
            GROUP BY interval_start
            ORDER BY interval_start
            LIMIT 500
            "#,
            interval_seconds
        );

        let rows: Vec<OhlcvRow> = self
            .client
            .query(&query)
            .bind(slug)
            .fetch_all()
            .await
            .map_err(|e| CortexMcpError::Database(e.to_string()))?;

        if rows.is_empty() {
            return Err(CortexMcpError::MarketNotFound(slug.to_string()));
        }

        let response = self.format_trend_response(slug, interval, rows)?;

        // Cache result
        if let Ok(value) = serde_json::to_value(&response) {
            self.cache.insert(cache_key, value).await;
        }

        Ok(response)
    }

    fn format_trend_response(
        &self,
        slug: &str,
        interval: &str,
        data: Vec<OhlcvRow>,
    ) -> Result<MarketTrendResponse> {
        let ohlcv: Vec<OhlcvData> = data
            .iter()
            .map(|row| {
                let timestamp = DateTime::<Utc>::from_timestamp_millis(row.interval_start)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default();

                OhlcvData {
                    timestamp,
                    open: row.open_price.parse().unwrap_or(0.0),
                    high: row.high_price.parse().unwrap_or(0.0),
                    low: row.low_price.parse().unwrap_or(0.0),
                    close: row.close_price.parse().unwrap_or(0.0),
                    volume_usd: row.volume_usd.parse().unwrap_or(0.0),
                    trades: row.trade_count,
                }
            })
            .collect();

        let first = ohlcv.first().ok_or(CortexMcpError::MarketNotFound(slug.to_string()))?;
        let last = ohlcv.last().unwrap();
        let price_change = last.close - first.open;
        let price_change_pct = if first.open > 0.0 {
            (price_change / first.open) * 100.0
        } else {
            0.0
        };

        let high = ohlcv.iter().map(|o| o.high).fold(0.0_f64, f64::max);
        let low = ohlcv.iter().map(|o| o.low).fold(f64::MAX, f64::min);
        let total_volume: f64 = ohlcv.iter().map(|o| o.volume_usd).sum();

        let trend_direction = if price_change_pct > 1.0 {
            "up"
        } else if price_change_pct < -1.0 {
            "down"
        } else {
            "sideways"
        }
        .to_string();

        Ok(MarketTrendResponse {
            slug: slug.to_string(),
            platform: "polymarket".to_string(),
            interval: interval.to_string(),
            data_points: ohlcv.len(),
            ohlcv,
            summary: TrendSummary {
                price_change,
                price_change_pct,
                high,
                low,
                total_volume,
                trend_direction,
            },
        })
    }

    /// Get volume profile for a market
    pub async fn get_volume_profile(&self, slug: &str) -> Result<VolumeProfileResponse> {
        validate_slug(slug)?;

        let cache_key = format!("volume:{}", slug);
        if let Some(cached) = self.cache.get(&cache_key).await {
            if let Ok(response) = serde_json::from_value(cached) {
                return Ok(response);
            }
        }

        let query = r#"
            SELECT
                slug,
                'polymarket' as platform,
                toString(sum(CASE WHEN timestamp > now() - INTERVAL 24 HOUR THEN size_usd ELSE 0 END)) as total_volume_24h,
                toString(sum(CASE WHEN timestamp > now() - INTERVAL 7 DAY THEN size_usd ELSE 0 END)) as total_volume_7d,
                countIf(timestamp > now() - INTERVAL 24 HOUR) as total_trades_24h,
                uniqIf(trader, timestamp > now() - INTERVAL 24 HOUR) as unique_traders_24h,
                toString(avgIf(size_usd, timestamp > now() - INTERVAL 24 HOUR)) as avg_trade_size,
                '0' as bid_depth_usd,
                '0' as ask_depth_usd,
                '0.02' as spread
            FROM trades
            WHERE slug = ?
            GROUP BY slug
            "#;

        let row: Option<VolumeProfileRow> = self
            .client
            .query(query)
            .bind(slug)
            .fetch_optional()
            .await
            .map_err(|e| CortexMcpError::Database(e.to_string()))?;

        let row = row.ok_or(CortexMcpError::MarketNotFound(slug.to_string()))?;

        let spread: f64 = row.spread.parse().unwrap_or(0.02);
        let response = VolumeProfileResponse {
            slug: row.slug,
            platform: row.platform,
            volume_24h: row.total_volume_24h.parse().unwrap_or(0.0),
            volume_7d: row.total_volume_7d.parse().unwrap_or(0.0),
            trades_24h: row.total_trades_24h,
            unique_traders_24h: row.unique_traders_24h,
            avg_trade_size: row.avg_trade_size.parse().unwrap_or(0.0),
            liquidity: LiquidityInfo {
                bid_depth_usd: row.bid_depth_usd.parse().unwrap_or(0.0),
                ask_depth_usd: row.ask_depth_usd.parse().unwrap_or(0.0),
                spread,
                spread_bps: spread * 10000.0,
            },
        };

        if let Ok(value) = serde_json::to_value(&response) {
            self.cache.insert(cache_key, value).await;
        }

        Ok(response)
    }

    /// Search historical markets
    pub async fn search_market_memory(
        &self,
        query_text: &str,
        limit: u32,
    ) -> Result<SearchMemoryResponse> {
        if query_text.trim().is_empty() {
            return Err(CortexMcpError::InvalidParameter(
                "Search query cannot be empty".into(),
            ));
        }

        let limit = limit.min(100);

        let query = r#"
            SELECT
                slug,
                'polymarket' as platform,
                title,
                category,
                status,
                toString(current_price) as current_price,
                toString(volume_24h) as volume_24h,
                1.0 as relevance_score
            FROM markets
            WHERE title ILIKE ?
               OR category ILIKE ?
            ORDER BY volume_24h DESC
            LIMIT ?
            "#;

        let pattern = format!("%{}%", query_text);
        let rows: Vec<MarketSearchRow> = self
            .client
            .query(query)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all()
            .await
            .map_err(|e| CortexMcpError::Database(e.to_string()))?;

        let markets: Vec<MarketSearchItem> = rows
            .into_iter()
            .map(|r| MarketSearchItem {
                slug: r.slug,
                platform: r.platform,
                title: r.title,
                category: r.category,
                status: r.status,
                current_price: r.current_price.parse().unwrap_or(0.0),
                volume_24h: r.volume_24h.parse().unwrap_or(0.0),
                relevance_score: r.relevance_score,
            })
            .collect();

        Ok(SearchMemoryResponse {
            query: query_text.to_string(),
            results_count: markets.len(),
            markets,
        })
    }

    /// Detect price anomalies
    pub async fn detect_anomalies(
        &self,
        slug: &str,
        threshold: f64,
    ) -> Result<AnomalyResponse> {
        validate_slug(slug)?;

        if threshold <= 0.0 {
            return Err(CortexMcpError::InvalidParameter(
                "Threshold must be greater than 0".into(),
            ));
        }

        let query = r#"
            WITH stats AS (
                SELECT
                    avg(price) as mean_price,
                    stddevPop(price) as std_dev
                FROM trades
                WHERE slug = ?
                  AND timestamp > now() - INTERVAL 7 DAY
            )
            SELECT
                toInt64(toUnixTimestamp(t.timestamp) * 1000) as timestamp,
                toString(t.price) as price,
                toString(s.mean_price) as mean_price,
                (t.price - s.mean_price) / nullIf(s.std_dev, 0) as z_score,
                toString(abs(t.price - s.mean_price) / nullIf(s.mean_price, 0) * 100) as deviation_pct,
                t.outcome_token as outcome_token
            FROM trades t
            CROSS JOIN stats s
            WHERE t.slug = ?
              AND t.timestamp > now() - INTERVAL 7 DAY
              AND abs((t.price - s.mean_price) / nullIf(s.std_dev, 0)) > ?
            ORDER BY t.timestamp DESC
            LIMIT 100
            "#;

        let rows: Vec<AnomalyRow> = self
            .client
            .query(query)
            .bind(slug)
            .bind(slug)
            .bind(threshold)
            .fetch_all()
            .await
            .map_err(|e| CortexMcpError::Database(e.to_string()))?;

        let anomalies: Vec<AnomalyItem> = rows
            .into_iter()
            .map(|a| {
                let timestamp = DateTime::<Utc>::from_timestamp_millis(a.timestamp)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default();

                let direction = if a.z_score > 0.0 {
                    "spike_up"
                } else {
                    "spike_down"
                }
                .to_string();

                AnomalyItem {
                    timestamp,
                    price: a.price.parse().unwrap_or(0.0),
                    mean_price: a.mean_price.parse().unwrap_or(0.0),
                    z_score: a.z_score,
                    deviation_pct: a.deviation_pct.parse().unwrap_or(0.0),
                    direction,
                    outcome_token: a.outcome_token,
                }
            })
            .collect();

        Ok(AnomalyResponse {
            slug: slug.to_string(),
            threshold_std_dev: threshold,
            anomalies_found: anomalies.len(),
            anomalies,
        })
    }
}
