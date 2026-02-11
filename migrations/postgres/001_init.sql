-- Solder Cortex PostgreSQL Schema
-- Relational data: users, wallets, subscriptions, sessions

-- Tracked wallets
CREATE TABLE IF NOT EXISTS wallets (
    id SERIAL PRIMARY KEY,
    address VARCHAR(64) NOT NULL UNIQUE,
    label VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_wallets_address ON wallets(address);

-- Wallet subscriptions (real-time indexing)
CREATE TABLE IF NOT EXISTS subscriptions (
    id SERIAL PRIMARY KEY,
    wallet_id INTEGER REFERENCES wallets(id) ON DELETE CASCADE,
    status VARCHAR(32) DEFAULT 'active', -- active, paused, stopped
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_sync_at TIMESTAMP WITH TIME ZONE,
    tx_count INTEGER DEFAULT 0,
    UNIQUE(wallet_id)
);

-- User sessions (for web dashboard)
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_ip VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_active_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Conviction signals cache
CREATE TABLE IF NOT EXISTS conviction_signals (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(64) NOT NULL,
    evm_address VARCHAR(42),
    score DECIMAL(5,4),
    signal VARCHAR(32), -- STRONG_BULLISH, BULLISH, NEUTRAL, BEARISH, STRONG_BEARISH
    confidence DECIMAL(5,4),
    cross_domain BOOLEAN DEFAULT FALSE,
    informed_trader BOOLEAN DEFAULT FALSE,
    computed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() + INTERVAL '1 hour'
);

CREATE INDEX idx_conviction_wallet ON conviction_signals(wallet_address);
CREATE INDEX idx_conviction_score ON conviction_signals(score DESC);

-- Prediction market positions
CREATE TABLE IF NOT EXISTS prediction_positions (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(64) NOT NULL,
    platform VARCHAR(32) NOT NULL, -- polymarket, kalshi, metaculus
    market_slug VARCHAR(255) NOT NULL,
    market_title TEXT,
    side VARCHAR(8), -- YES, NO
    amount DECIMAL(18,8),
    entry_price DECIMAL(10,4),
    current_price DECIMAL(10,4),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_prediction_wallet ON prediction_positions(wallet_address);
CREATE INDEX idx_prediction_platform ON prediction_positions(platform, market_slug);

-- API usage tracking
CREATE TABLE IF NOT EXISTS api_calls (
    id SERIAL PRIMARY KEY,
    session_id UUID REFERENCES sessions(id),
    method VARCHAR(64) NOT NULL,
    params JSONB,
    response_time_ms INTEGER,
    error TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_api_calls_session ON api_calls(session_id);
CREATE INDEX idx_api_calls_method ON api_calls(method);
CREATE INDEX idx_api_calls_time ON api_calls(created_at DESC);
