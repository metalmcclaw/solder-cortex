-- Solder Cortex ClickHouse Schema
-- Run with: clickhouse-client --multiquery < migrations/001_init.sql

CREATE DATABASE IF NOT EXISTS cortex;

-- Transactions table: stores parsed DeFi transactions
CREATE TABLE IF NOT EXISTS cortex.transactions (
    signature String,
    wallet String,
    protocol LowCardinality(String),
    tx_type LowCardinality(String),
    token_in String,
    token_out String,
    amount_in Decimal128(18),
    amount_out Decimal128(18),
    usd_value Decimal64(2),
    block_time DateTime64(3),
    slot UInt64,
    created_at DateTime64(3) DEFAULT now64(3)
) ENGINE = MergeTree()
ORDER BY (wallet, block_time, signature)
PARTITION BY toYYYYMM(block_time);

-- Positions table: current open positions (uses ReplacingMergeTree for upserts)
CREATE TABLE IF NOT EXISTS cortex.positions (
    wallet String,
    protocol LowCardinality(String),
    position_type LowCardinality(String),
    token String,
    pool String DEFAULT '',
    amount Decimal128(18),
    entry_price Decimal64(8),
    current_price Decimal64(8),
    usd_value Decimal64(2),
    unrealized_pnl Decimal64(2),
    apy Decimal32(6) DEFAULT 0,
    updated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (wallet, protocol, position_type, token);

-- Wallet summaries: pre-aggregated metrics for fast API responses
CREATE TABLE IF NOT EXISTS cortex.wallet_summaries (
    wallet String,
    total_value_usd Decimal64(2),
    realized_pnl_24h Decimal64(2),
    realized_pnl_7d Decimal64(2),
    realized_pnl_30d Decimal64(2),
    unrealized_pnl Decimal64(2),
    largest_position_pct Decimal32(4),
    protocol_count UInt8,
    position_count UInt16,
    risk_score UInt8,
    last_activity DateTime64(3),
    protocols Array(String),
    updated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY wallet;

-- Token prices cache (for PnL calculations)
CREATE TABLE IF NOT EXISTS cortex.token_prices (
    token String,
    price_usd Decimal64(8),
    updated_at DateTime64(3)
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY token;

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_transactions_wallet ON cortex.transactions (wallet) TYPE bloom_filter GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_transactions_protocol ON cortex.transactions (protocol) TYPE set(10) GRANULARITY 1;
CREATE INDEX IF NOT EXISTS idx_positions_wallet ON cortex.positions (wallet) TYPE bloom_filter GRANULARITY 1;
