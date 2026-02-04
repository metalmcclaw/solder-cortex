use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

/// Decimal values are stored as strings for precise representation in ClickHouse
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct TransactionRow {
    pub signature: String,
    pub wallet: String,
    pub protocol: String,
    pub tx_type: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub amount_out: String,
    pub usd_value: String,
    pub block_time: i64,
    pub slot: u64,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct PositionRow {
    pub wallet: String,
    pub protocol: String,
    pub position_type: String,
    pub token: String,
    pub pool: String,
    pub amount: String,
    pub entry_price: String,
    pub current_price: String,
    pub usd_value: String,
    pub unrealized_pnl: String,
    pub apy: String,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct WalletSummaryRow {
    pub wallet: String,
    pub total_value_usd: String,
    pub realized_pnl_24h: String,
    pub realized_pnl_7d: String,
    pub realized_pnl_30d: String,
    pub unrealized_pnl: String,
    pub largest_position_pct: String,
    pub protocol_count: u8,
    pub position_count: u16,
    pub risk_score: u8,
    pub last_activity: i64,
    pub protocols: Vec<String>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct PnlByProtocolRow {
    pub protocol: String,
    pub realized: String,
    pub unrealized: String,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct TokenPriceRow {
    pub token: String,
    pub price_usd: String,
}

impl WalletSummaryRow {
    pub fn last_activity_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.last_activity).unwrap_or_default()
    }
}
