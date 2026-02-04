use clickhouse::Client;

use super::models::*;
use crate::error::{PredictionError, Result};

/// Parameterized query executor for Clickhouse
/// Prevents SQL injection by using bind parameters
pub struct QueryEngine {
    client: Client,
}

impl QueryEngine {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get market metadata by slug
    pub async fn get_market(&self, slug: &str) -> Result<Option<MarketRow>> {
        let market = self
            .client
            .query(
                r#"
                SELECT
                    slug,
                    platform,
                    title,
                    description,
                    category,
                    status,
                    outcome_tokens,
                    outcome_labels
                FROM cortex.markets
                WHERE slug = ?
                LIMIT 1
                "#,
            )
            .bind(slug)
            .fetch_optional::<MarketRow>()
            .await?;

        Ok(market)
    }

    /// Get OHLCV data for a market within a time interval
    /// Uses Clickhouse's time bucket functions for efficient aggregation
    pub async fn get_market_trend(
        &self,
        slug: &str,
        interval: &str,
    ) -> Result<Vec<OhlcvRow>> {
        // Convert interval to Clickhouse interval function
        let (bucket_fn, lookback) = match interval {
            "1m" => ("toStartOfMinute", "1 HOUR"),
            "5m" => ("toStartOfFiveMinutes", "4 HOUR"),
            "15m" => ("toStartOfFifteenMinutes", "12 HOUR"),
            "30m" => ("toStartOfInterval(timestamp, INTERVAL 30 MINUTE)", "24 HOUR"),
            "1h" => ("toStartOfHour", "48 HOUR"),
            "4h" => ("toStartOfInterval(timestamp, INTERVAL 4 HOUR)", "7 DAY"),
            "24h" | "1d" => ("toStartOfDay", "30 DAY"),
            "7d" => ("toStartOfWeek", "90 DAY"),
            _ => return Err(PredictionError::InvalidInterval(interval.to_string())),
        };

        // Dynamic query with time bucketing
        // Note: bucket functions return DateTime, not DateTime64, so we cast to DateTime64
        // before calling toUnixTimestamp64Milli
        let query = format!(
            r#"
            SELECT
                toUnixTimestamp64Milli(toDateTime64({bucket_fn}(timestamp), 3)) AS interval_start,
                toString(argMin(price, timestamp)) AS open_price,
                toString(max(price)) AS high_price,
                toString(min(price)) AS low_price,
                toString(argMax(price, timestamp)) AS close_price,
                toString(sum(usd_value)) AS volume_usd,
                toUInt32(count()) AS trade_count
            FROM cortex.market_trades
            WHERE slug = ?
              AND timestamp >= now() - INTERVAL {lookback}
            GROUP BY {bucket_fn}(timestamp)
            ORDER BY interval_start ASC
            "#,
            bucket_fn = bucket_fn,
            lookback = lookback
        );

        let rows = self
            .client
            .query(&query)
            .bind(slug)
            .fetch_all::<OhlcvRow>()
            .await?;

        Ok(rows)
    }

    /// Get volume profile and liquidity data for a market
    pub async fn get_volume_profile(&self, slug: &str) -> Result<Option<VolumeProfileRow>> {
        let row = self
            .client
            .query(
                r#"
                WITH
                    volume_stats AS (
                        SELECT
                            sum(if(timestamp >= now() - INTERVAL 24 HOUR, usd_value, 0)) AS vol_24h,
                            sum(if(timestamp >= now() - INTERVAL 7 DAY, usd_value, 0)) AS vol_7d,
                            countIf(timestamp >= now() - INTERVAL 24 HOUR) AS trades_24h,
                            uniqExactIf(taker_address, timestamp >= now() - INTERVAL 24 HOUR) AS traders_24h,
                            avgIf(usd_value, timestamp >= now() - INTERVAL 24 HOUR) AS avg_trade
                        FROM cortex.market_trades
                        WHERE slug = ?
                    ),
                    orderbook_stats AS (
                        SELECT
                            bid_depth_usd,
                            ask_depth_usd,
                            spread
                        FROM cortex.market_orderbook
                        WHERE slug = ?
                        ORDER BY timestamp DESC
                        LIMIT 1
                    ),
                    market_info AS (
                        SELECT platform
                        FROM cortex.markets
                        WHERE slug = ?
                        LIMIT 1
                    )
                SELECT
                    ? AS slug,
                    market_info.platform AS platform,
                    toString(volume_stats.vol_24h) AS total_volume_24h,
                    toString(volume_stats.vol_7d) AS total_volume_7d,
                    toUInt32(volume_stats.trades_24h) AS total_trades_24h,
                    toUInt32(volume_stats.traders_24h) AS unique_traders_24h,
                    toString(volume_stats.avg_trade) AS avg_trade_size,
                    toString(coalesce(orderbook_stats.bid_depth_usd, 0)) AS bid_depth_usd,
                    toString(coalesce(orderbook_stats.ask_depth_usd, 0)) AS ask_depth_usd,
                    toString(coalesce(orderbook_stats.spread, 0)) AS spread
                FROM volume_stats
                CROSS JOIN market_info
                LEFT JOIN orderbook_stats ON 1=1
                "#,
            )
            .bind(slug)
            .bind(slug)
            .bind(slug)
            .bind(slug)
            .fetch_optional::<VolumeProfileRow>()
            .await?;

        Ok(row)
    }

    /// Search markets by text query using token bloom filter
    pub async fn search_markets(&self, query: &str, limit: u32) -> Result<Vec<MarketSearchResult>> {
        // Tokenize query for better matching
        let search_terms: Vec<&str> = query.split_whitespace().collect();

        if search_terms.is_empty() {
            return Ok(vec![]);
        }

        // Build OR conditions for each search term
        let conditions: Vec<String> = search_terms
            .iter()
            .map(|term| {
                format!(
                    "(hasToken(lower(title), lower('{term}')) OR hasToken(lower(description), lower('{term}')) OR lower(category) = lower('{term}'))",
                    term = term.replace('\'', "''") // Escape single quotes
                )
            })
            .collect();

        let where_clause = conditions.join(" OR ");

        let query_sql = format!(
            r#"
            WITH latest_prices AS (
                SELECT
                    slug,
                    argMax(price, timestamp) AS current_price,
                    sumIf(usd_value, timestamp >= now() - INTERVAL 24 HOUR) AS volume_24h
                FROM cortex.market_trades
                GROUP BY slug
            )
            SELECT
                m.slug AS slug,
                m.platform AS platform,
                m.title AS title,
                m.category AS category,
                m.status AS status,
                toString(coalesce(p.current_price, 0)) AS current_price,
                toString(coalesce(p.volume_24h, 0)) AS volume_24h,
                -- Simple relevance: count matching terms
                ({term_count}::Float64) AS relevance_score
            FROM cortex.markets m
            LEFT JOIN latest_prices p ON m.slug = p.slug
            WHERE {where_clause}
            ORDER BY relevance_score DESC, p.volume_24h DESC
            LIMIT ?
            "#,
            term_count = search_terms.len(),
            where_clause = where_clause
        );

        let rows = self
            .client
            .query(&query_sql)
            .bind(limit)
            .fetch_all::<MarketSearchResult>()
            .await?;

        Ok(rows)
    }

    /// Detect price anomalies (>3 standard deviations from 1-hour moving average)
    pub async fn detect_anomalies(
        &self,
        slug: &str,
        std_dev_threshold: f64,
    ) -> Result<Vec<AnomalyRow>> {
        let rows = self
            .client
            .query(
                r#"
                WITH
                    -- Calculate 1-hour rolling statistics
                    rolling_stats AS (
                        SELECT
                            timestamp,
                            price,
                            outcome_token,
                            avg(price) OVER (
                                PARTITION BY outcome_token
                                ORDER BY timestamp
                                RANGE BETWEEN INTERVAL 1 HOUR PRECEDING AND CURRENT ROW
                            ) AS mean_price,
                            stddevPop(price) OVER (
                                PARTITION BY outcome_token
                                ORDER BY timestamp
                                RANGE BETWEEN INTERVAL 1 HOUR PRECEDING AND CURRENT ROW
                            ) AS std_dev
                        FROM cortex.market_prices
                        WHERE slug = ?
                          AND timestamp >= now() - INTERVAL 24 HOUR
                    )
                SELECT
                    toUnixTimestamp64Milli(timestamp) AS timestamp,
                    toString(price) AS price,
                    toString(mean_price) AS mean_price,
                    toString(std_dev) AS std_dev,
                    if(std_dev > 0, (price - mean_price) / std_dev, 0) AS z_score,
                    toString(if(mean_price > 0, ((price - mean_price) / mean_price) * 100, 0)) AS deviation_pct,
                    outcome_token
                FROM rolling_stats
                WHERE std_dev > 0
                  AND abs((price - mean_price) / std_dev) > ?
                ORDER BY timestamp DESC
                "#,
            )
            .bind(slug)
            .bind(std_dev_threshold)
            .fetch_all::<AnomalyRow>()
            .await?;

        Ok(rows)
    }

    /// Get rolling statistics for a market
    pub async fn get_market_stats(
        &self,
        slug: &str,
        window: &str,
    ) -> Result<Vec<MarketStatsRow>> {
        let rows = self
            .client
            .query(
                r#"
                SELECT
                    slug,
                    outcome_token,
                    window,
                    toString(mean_price) AS mean_price,
                    toString(std_dev) AS std_dev,
                    toString(min_price) AS min_price,
                    toString(max_price) AS max_price,
                    toString(price_change) AS price_change,
                    toString(price_change_pct) AS price_change_pct,
                    toString(sma) AS sma,
                    toString(ema) AS ema
                FROM cortex.market_stats
                WHERE slug = ?
                  AND window = ?
                ORDER BY outcome_token
                "#,
            )
            .bind(slug)
            .bind(window)
            .fetch_all::<MarketStatsRow>()
            .await?;

        Ok(rows)
    }

    /// Health check - verify database connectivity
    pub async fn health_check(&self) -> Result<()> {
        self.client.query("SELECT 1").execute().await?;
        Ok(())
    }
}
