-- Prediction Markets ClickHouse Schema
-- Run with: clickhouse-client --multiquery < migrations/002_prediction_markets.sql
-- Supports: Polymarket, Kalshi, and other prediction market platforms

-- Markets table: stores market metadata and current state
CREATE TABLE IF NOT EXISTS cortex.markets (
    slug String,                              -- Unique market identifier (e.g., "will-trump-win-2024")
    platform LowCardinality(String),          -- polymarket, kalshi, etc.
    title String,                             -- Human-readable market title
    description String DEFAULT '',            -- Full market description
    category LowCardinality(String) DEFAULT '',-- politics, sports, crypto, etc.
    resolution_source String DEFAULT '',      -- Where resolution comes from

    -- Outcome tokens (for binary markets, typically YES/NO)
    outcome_tokens Array(String),             -- Token identifiers
    outcome_labels Array(String),             -- Human-readable labels

    -- Status
    status LowCardinality(String) DEFAULT 'active', -- active, closed, resolved
    resolved_outcome String DEFAULT '',       -- Winning outcome if resolved

    -- Timestamps
    created_at DateTime64(3),
    end_date DateTime64(3),                   -- When market closes for trading
    resolution_date DateTime64(3) DEFAULT toDateTime64(0, 3), -- When resolved
    updated_at DateTime64(3) DEFAULT now64(3)
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (platform, slug)
PARTITION BY platform;

-- Market prices: time-series price data for market outcomes
CREATE TABLE IF NOT EXISTS cortex.market_prices (
    slug String,
    platform LowCardinality(String),
    outcome_token String,                     -- Which outcome this price is for
    price Decimal64(6),                       -- Price between 0 and 1 (probability)
    bid_price Decimal64(6) DEFAULT 0,         -- Best bid
    ask_price Decimal64(6) DEFAULT 0,         -- Best ask
    mid_price Decimal64(6) DEFAULT 0,         -- Mid-market price
    timestamp DateTime64(3)
) ENGINE = MergeTree()
ORDER BY (slug, outcome_token, timestamp)
PARTITION BY (platform, toYYYYMM(timestamp))
TTL toDateTime(timestamp) + INTERVAL 2 YEAR;

-- Market trades: individual trade events
CREATE TABLE IF NOT EXISTS cortex.market_trades (
    trade_id String,
    slug String,
    platform LowCardinality(String),
    outcome_token String,
    side LowCardinality(String),              -- buy, sell
    price Decimal64(6),
    quantity Decimal64(8),
    usd_value Decimal64(2),
    maker_address String DEFAULT '',          -- Anonymized or actual address
    taker_address String DEFAULT '',
    timestamp DateTime64(3)
) ENGINE = MergeTree()
ORDER BY (slug, timestamp, trade_id)
PARTITION BY (platform, toYYYYMM(timestamp))
TTL toDateTime(timestamp) + INTERVAL 2 YEAR;

-- Market volume aggregates: pre-computed volume data by time intervals
CREATE TABLE IF NOT EXISTS cortex.market_volume (
    slug String,
    platform LowCardinality(String),
    interval LowCardinality(String),          -- 1m, 5m, 1h, 4h, 1d
    volume_usd Decimal64(2),
    trade_count UInt32,
    unique_traders UInt32,
    open_price Decimal64(6),
    high_price Decimal64(6),
    low_price Decimal64(6),
    close_price Decimal64(6),
    vwap Decimal64(6),                        -- Volume-weighted average price
    interval_start DateTime64(3),
    interval_end DateTime64(3)
) ENGINE = ReplacingMergeTree(interval_end)
ORDER BY (slug, interval, interval_start)
PARTITION BY (platform, toYYYYMM(interval_start));

-- Order book snapshots: for liquidity analysis
CREATE TABLE IF NOT EXISTS cortex.market_orderbook (
    slug String,
    platform LowCardinality(String),
    outcome_token String,
    bids Array(Tuple(price Decimal64(6), quantity Decimal64(8))),
    asks Array(Tuple(price Decimal64(6), quantity Decimal64(8))),
    bid_depth_usd Decimal64(2),               -- Total USD in bids
    ask_depth_usd Decimal64(2),               -- Total USD in asks
    spread Decimal64(6),                      -- Bid-ask spread
    timestamp DateTime64(3)
) ENGINE = MergeTree()
ORDER BY (slug, outcome_token, timestamp)
PARTITION BY (platform, toYYYYMM(timestamp))
TTL toDateTime(timestamp) + INTERVAL 30 DAY;

-- Price statistics: rolling statistics for anomaly detection
CREATE TABLE IF NOT EXISTS cortex.market_stats (
    slug String,
    platform LowCardinality(String),
    outcome_token String,
    window LowCardinality(String),            -- 1h, 4h, 24h

    -- Rolling statistics
    mean_price Decimal64(6),
    std_dev Decimal64(6),
    min_price Decimal64(6),
    max_price Decimal64(6),
    price_change Decimal64(6),                -- Absolute change
    price_change_pct Decimal64(4),            -- Percentage change

    -- Moving averages
    sma Decimal64(6),                         -- Simple moving average
    ema Decimal64(6),                         -- Exponential moving average

    calculated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(calculated_at)
ORDER BY (slug, outcome_token, window)
PARTITION BY platform;

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_markets_category ON cortex.markets (category) TYPE set(50) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_markets_status ON cortex.markets (status) TYPE set(5) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_markets_title ON cortex.markets (title) TYPE tokenbf_v1(10240, 3, 0) GRANULARITY 4;
CREATE INDEX IF NOT EXISTS idx_markets_description ON cortex.markets (description) TYPE tokenbf_v1(10240, 3, 0) GRANULARITY 4;
CREATE INDEX IF NOT EXISTS idx_market_prices_slug ON cortex.market_prices (slug) TYPE bloom_filter GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_market_trades_slug ON cortex.market_trades (slug) TYPE bloom_filter GRANULARITY 1;

-- Materialized view for real-time volume aggregation (1-hour buckets)
CREATE MATERIALIZED VIEW IF NOT EXISTS cortex.mv_market_volume_1h
TO cortex.market_volume
AS SELECT
    slug,
    platform,
    '1h' AS interval,
    sum(usd_value) AS volume_usd,
    count() AS trade_count,
    uniqExact(taker_address) AS unique_traders,
    argMin(price, timestamp) AS open_price,
    max(price) AS high_price,
    min(price) AS low_price,
    argMax(price, timestamp) AS close_price,
    sum(price * quantity) / sum(quantity) AS vwap,
    toStartOfHour(timestamp) AS interval_start,
    toStartOfHour(timestamp) + INTERVAL 1 HOUR AS interval_end
FROM cortex.market_trades
GROUP BY slug, platform, toStartOfHour(timestamp);
