-- Market Aggregation ClickHouse Schema
-- Run with: clickhouse-client --multiquery < migrations/003_market_aggregation.sql
-- Support for market-wide DeFi and prediction market data aggregation

-- Market metrics: time-series data for protocol-level and market-wide metrics
CREATE TABLE IF NOT EXISTS cortex.market_metrics (
    timestamp DateTime64(3),
    data_type LowCardinality(String),          -- 'defi', 'prediction', 'sentiment'
    protocol LowCardinality(String),           -- 'jupiter', 'raydium', 'polymarket', 'kalshi', 'overall'
    metric_name LowCardinality(String),        -- 'tvl', 'volume_24h', 'sentiment_score', etc.
    metric_value Decimal64(8),
    string_value String DEFAULT '',            -- For non-numeric data
    array_values Array(String) DEFAULT [],    -- For array data
    metadata String DEFAULT ''                 -- JSON metadata
) ENGINE = MergeTree()
ORDER BY (data_type, protocol, metric_name, timestamp)
PARTITION BY (data_type, toYYYYMM(timestamp))
TTL toDateTime(timestamp) + INTERVAL 1 YEAR;

-- DeFi protocol aggregates: pre-calculated protocol metrics
CREATE TABLE IF NOT EXISTS cortex.defi_aggregates (
    protocol LowCardinality(String),
    tvl_usd Decimal64(2),
    volume_24h_usd Decimal64(2),
    volume_7d_usd Decimal64(2),
    unique_users_24h UInt32,
    transaction_count_24h UInt32,
    avg_transaction_size_usd Decimal64(2),
    market_share_tvl Decimal32(4),           -- Percentage of total DeFi TVL
    market_share_volume Decimal32(4),        -- Percentage of total DeFi volume
    trend_score Decimal32(4),                -- -1 to 1, negative = declining, positive = growing
    calculated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(calculated_at)
ORDER BY protocol;

-- Token aggregates: market-wide token metrics across all protocols
CREATE TABLE IF NOT EXISTS cortex.token_aggregates (
    symbol String,
    mint_address String,
    price_usd Decimal64(8),
    market_cap_usd Decimal64(2),
    volume_24h_usd Decimal64(2),
    price_change_24h Decimal32(4),
    price_change_7d Decimal32(4),
    
    -- DeFi usage metrics
    defi_tvl_total Decimal64(2),             -- Total TVL across all protocols
    defi_protocols Array(String),            -- Protocols where this token is used
    swap_volume_24h Decimal64(2),
    lending_volume_24h Decimal64(2),
    
    -- Prediction market exposure
    prediction_markets_count UInt16,         -- Number of markets mentioning this token
    prediction_volume_24h Decimal64(2),      -- Volume in related prediction markets
    
    calculated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(calculated_at)
ORDER BY symbol;

-- Market sentiment aggregates: derived sentiment scores
CREATE TABLE IF NOT EXISTS cortex.sentiment_aggregates (
    scope LowCardinality(String),             -- 'overall', 'defi', 'prediction_markets', 'crypto'
    sentiment_score Decimal32(4),             -- -1 (bearish) to 1 (bullish)
    confidence_score Decimal32(4),            -- 0 to 1, quality of data
    
    -- Contributing factors
    defi_volume_factor Decimal32(4),
    defi_tvl_factor Decimal32(4),
    prediction_price_factor Decimal32(4),
    prediction_volume_factor Decimal32(4),
    
    -- Trending topics/keywords
    trending_topics Array(String),
    topic_weights Array(Float32),             -- Weights for trending topics
    
    -- Data completeness
    data_points_count UInt32,
    total_expected_data_points UInt32,
    
    calculated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(calculated_at)
ORDER BY (scope, calculated_at);

-- Conviction signals: detected patterns and correlations
CREATE TABLE IF NOT EXISTS cortex.conviction_signals (
    signal_id String,                         -- Unique identifier for this signal
    signal_type LowCardinality(String),       -- 'bullish_convergence', 'bearish_divergence', etc.
    strength Decimal32(4),                    -- 0 to 1
    confidence Decimal32(4),                  -- 0 to 1
    
    -- Signal sources
    defi_data_involved Boolean DEFAULT false,
    prediction_data_involved Boolean DEFAULT false,
    cross_platform Boolean DEFAULT false,
    
    -- Metadata
    description String,
    supporting_metrics String,               -- JSON with supporting data
    
    created_at DateTime64(3),
    expires_at DateTime64(3),               -- When this signal becomes stale
    
    -- Signal outcome tracking (for backtesting)
    outcome String DEFAULT '',              -- 'confirmed', 'failed', 'inconclusive'
    outcome_time DateTime64(3) DEFAULT toDateTime64(0, 3),
    outcome_notes String DEFAULT ''
) ENGINE = MergeTree()
ORDER BY (signal_type, created_at, signal_id)
PARTITION BY toYYYYMM(created_at)
TTL toDateTime(expires_at) + INTERVAL 30 DAY;

-- Cross-domain correlation tracking: DeFi vs Prediction Markets
CREATE TABLE IF NOT EXISTS cortex.cross_domain_correlations (
    entity_type LowCardinality(String),      -- 'token', 'protocol', 'topic'
    entity_name String,                      -- 'SOL', 'jupiter', 'bitcoin', etc.
    
    -- DeFi metrics
    defi_volume_change_24h Decimal32(4),     -- Percentage change
    defi_tvl_change_24h Decimal32(4),
    defi_activity_score Decimal32(4),        -- Normalized activity metric
    
    -- Prediction market metrics  
    prediction_price_avg Decimal32(4),       -- Average YES price for related markets
    prediction_volume_24h Decimal64(2),      -- Volume in related markets
    prediction_market_count UInt16,          -- Number of related markets
    
    -- Correlation metrics
    correlation_score Decimal32(4),          -- -1 to 1, how correlated DeFi and prediction data is
    correlation_strength LowCardinality(String), -- 'weak', 'moderate', 'strong'
    correlation_direction LowCardinality(String), -- 'positive', 'negative', 'none'
    
    sample_size UInt32,                      -- Number of data points used
    calculated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(calculated_at)
ORDER BY (entity_type, entity_name);

-- Market overview cache: pre-computed overview data for fast API responses
CREATE TABLE IF NOT EXISTS cortex.market_overview_cache (
    cache_type LowCardinality(String),       -- 'overview', 'conviction', 'sentiment'
    cache_key String,                        -- Additional key for sub-categories
    data_json String,                        -- JSON blob with computed data
    computed_at DateTime64(3),
    expires_at DateTime64(3)
) ENGINE = ReplacingMergeTree(computed_at)
ORDER BY (cache_type, cache_key);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_market_metrics_type_protocol ON cortex.market_metrics (data_type, protocol) TYPE set(100) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_market_metrics_metric_name ON cortex.market_metrics (metric_name) TYPE set(200) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_conviction_signals_type ON cortex.conviction_signals (signal_type) TYPE set(50) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_token_aggregates_symbol ON cortex.token_aggregates (symbol) TYPE bloom_filter GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_cross_domain_entity ON cortex.cross_domain_correlations (entity_type, entity_name) TYPE bloom_filter GRANULARITY 1;

-- Materialized views for real-time aggregation

-- Real-time DeFi protocol metrics (1-hour buckets)
CREATE MATERIALIZED VIEW IF NOT EXISTS cortex.mv_defi_hourly_metrics
TO cortex.market_metrics
AS SELECT
    toStartOfHour(block_time) AS timestamp,
    'defi' AS data_type,
    protocol,
    'volume_1h' AS metric_name,
    sum(usd_value) AS metric_value,
    '' AS string_value,
    [] AS array_values,
    '' AS metadata
FROM cortex.transactions
WHERE block_time >= now() - INTERVAL 2 HOUR  -- Only recent data
GROUP BY protocol, toStartOfHour(block_time);

-- Real-time prediction market metrics (1-hour buckets)  
CREATE MATERIALIZED VIEW IF NOT EXISTS cortex.mv_prediction_hourly_metrics
TO cortex.market_metrics  
AS SELECT
    toStartOfHour(timestamp) AS timestamp,
    'prediction' AS data_type,
    platform AS protocol,
    'volume_1h' AS metric_name,
    sum(usd_value) AS metric_value,
    '' AS string_value,
    [] AS array_values,
    '' AS metadata
FROM cortex.market_trades
WHERE timestamp >= now() - INTERVAL 2 HOUR
GROUP BY platform, toStartOfHour(timestamp);

-- Comments for documentation
COMMENT ON TABLE cortex.market_metrics IS 'Time-series metrics for DeFi protocols and prediction markets';
COMMENT ON TABLE cortex.defi_aggregates IS 'Pre-computed DeFi protocol statistics';
COMMENT ON TABLE cortex.token_aggregates IS 'Cross-protocol token metrics and DeFi usage';
COMMENT ON TABLE cortex.sentiment_aggregates IS 'Market sentiment derived from DeFi and prediction market data';
COMMENT ON TABLE cortex.conviction_signals IS 'Detected conviction patterns and cross-domain correlations';
COMMENT ON TABLE cortex.cross_domain_correlations IS 'Correlation tracking between DeFi activity and prediction market sentiment';
COMMENT ON TABLE cortex.market_overview_cache IS 'Cached API response data for fast market overview queries';