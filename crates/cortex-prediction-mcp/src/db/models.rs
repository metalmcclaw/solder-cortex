use clickhouse::Row;
use serde::{Deserialize, Serialize};

/// Market metadata from the markets table
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct MarketRow {
    pub slug: String,
    pub platform: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub status: String,
    pub outcome_tokens: Vec<String>,
    pub outcome_labels: Vec<String>,
}

/// Price point for trend analysis
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: i64, // Milliseconds since epoch
    pub price: String,  // Decimal as string for precision
    pub outcome_token: String,
}

/// Aggregated OHLCV data for a time bucket
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct OhlcvRow {
    pub interval_start: i64,
    pub open_price: String,
    pub high_price: String,
    pub low_price: String,
    pub close_price: String,
    pub volume_usd: String,
    pub trade_count: u32,
}

/// Volume profile data
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct VolumeProfileRow {
    pub slug: String,
    pub platform: String,
    pub total_volume_24h: String,
    pub total_volume_7d: String,
    pub total_trades_24h: u32,
    pub unique_traders_24h: u32,
    pub avg_trade_size: String,
    pub bid_depth_usd: String,
    pub ask_depth_usd: String,
    pub spread: String,
}

/// Search result for market memory queries
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct MarketSearchResult {
    pub slug: String,
    pub platform: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub current_price: String,
    pub volume_24h: String,
    pub relevance_score: f64,
}

/// Anomaly detection result
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct AnomalyRow {
    pub timestamp: i64,
    pub price: String,
    pub mean_price: String,
    pub std_dev: String,
    pub z_score: f64,
    pub deviation_pct: String,
    pub outcome_token: String,
}

/// Rolling statistics for a market
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct MarketStatsRow {
    pub slug: String,
    pub outcome_token: String,
    pub window: String,
    pub mean_price: String,
    pub std_dev: String,
    pub min_price: String,
    pub max_price: String,
    pub price_change: String,
    pub price_change_pct: String,
    pub sma: String,
    pub ema: String,
}

/// Response types for MCP tools (agent-readable JSON)

#[derive(Debug, Serialize)]
pub struct MarketTrendResponse {
    pub slug: String,
    pub platform: String,
    pub interval: String,
    pub data_points: usize,
    pub ohlcv: Vec<OhlcvData>,
    pub summary: TrendSummary,
}

#[derive(Debug, Serialize)]
pub struct OhlcvData {
    pub timestamp: String, // ISO8601
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume_usd: f64,
    pub trades: u32,
}

#[derive(Debug, Serialize)]
pub struct TrendSummary {
    pub price_change: f64,
    pub price_change_pct: f64,
    pub high: f64,
    pub low: f64,
    pub total_volume: f64,
    pub trend_direction: String, // "up", "down", "sideways"
}

#[derive(Debug, Serialize)]
pub struct VolumeProfileResponse {
    pub slug: String,
    pub platform: String,
    pub volume_24h: f64,
    pub volume_7d: f64,
    pub trades_24h: u32,
    pub unique_traders_24h: u32,
    pub avg_trade_size: f64,
    pub liquidity: LiquidityInfo,
}

#[derive(Debug, Serialize)]
pub struct LiquidityInfo {
    pub bid_depth_usd: f64,
    pub ask_depth_usd: f64,
    pub spread: f64,
    pub spread_bps: f64, // Spread in basis points
}

#[derive(Debug, Serialize)]
pub struct SearchMemoryResponse {
    pub query: String,
    pub results_count: usize,
    pub markets: Vec<MarketSearchItem>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct AnomalyResponse {
    pub slug: String,
    pub threshold_std_dev: f64,
    pub anomalies_found: usize,
    pub anomalies: Vec<AnomalyItem>,
}

#[derive(Debug, Serialize)]
pub struct AnomalyItem {
    pub timestamp: String,
    pub price: f64,
    pub mean_price: f64,
    pub z_score: f64,
    pub deviation_pct: f64,
    pub direction: String, // "spike_up", "spike_down"
    pub outcome_token: String,
}
