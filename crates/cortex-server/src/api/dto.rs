use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::str::FromStr;

use crate::db::models::{PositionRow, PnlByProtocolRow, WalletSummaryRow};

/// Helper to parse string to Decimal, defaulting to zero on error
fn parse_decimal(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_default()
}

// ============================================================================
// GET /api/v1/user/{wallet}/summary
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserSummaryResponse {
    pub wallet: String,
    pub total_value_usd: Decimal,
    pub pnl: PnlSummary,
    pub risk: RiskSummary,
    pub last_activity: DateTime<Utc>,
    pub protocols: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PnlSummary {
    pub realized_24h: Decimal,
    pub realized_7d: Decimal,
    pub realized_30d: Decimal,
    pub unrealized: Decimal,
}

#[derive(Debug, Serialize)]
pub struct RiskSummary {
    pub score: u8,
    pub largest_position_pct: Decimal,
    pub protocol_count: u8,
}

impl From<WalletSummaryRow> for UserSummaryResponse {
    fn from(row: WalletSummaryRow) -> Self {
        Self {
            wallet: row.wallet.clone(),
            total_value_usd: parse_decimal(&row.total_value_usd),
            pnl: PnlSummary {
                realized_24h: parse_decimal(&row.realized_pnl_24h),
                realized_7d: parse_decimal(&row.realized_pnl_7d),
                realized_30d: parse_decimal(&row.realized_pnl_30d),
                unrealized: parse_decimal(&row.unrealized_pnl),
            },
            risk: RiskSummary {
                score: row.risk_score,
                largest_position_pct: parse_decimal(&row.largest_position_pct),
                protocol_count: row.protocol_count,
            },
            last_activity: row.last_activity_datetime(),
            protocols: row.protocols,
        }
    }
}

// ============================================================================
// GET /api/v1/user/{wallet}/pnl
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PnlQuery {
    #[serde(default = "default_window")]
    pub window: String,
}

fn default_window() -> String {
    "7d".to_string()
}

#[derive(Debug, Serialize)]
pub struct UserPnlResponse {
    pub wallet: String,
    pub window: String,
    pub total_realized: Decimal,
    pub total_unrealized: Decimal,
    pub by_protocol: Vec<ProtocolPnl>,
}

#[derive(Debug, Serialize)]
pub struct ProtocolPnl {
    pub protocol: String,
    pub realized: Decimal,
    pub unrealized: Decimal,
    pub trade_count: u64,
}

impl From<PnlByProtocolRow> for ProtocolPnl {
    fn from(row: PnlByProtocolRow) -> Self {
        Self {
            protocol: row.protocol,
            realized: parse_decimal(&row.realized),
            unrealized: parse_decimal(&row.unrealized),
            trade_count: row.trade_count,
        }
    }
}

// ============================================================================
// GET /api/v1/user/{wallet}/positions
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserPositionsResponse {
    pub wallet: String,
    pub positions: Vec<Position>,
    pub total_value_usd: Decimal,
}

#[derive(Debug, Serialize)]
pub struct Position {
    pub protocol: String,
    #[serde(rename = "type")]
    pub position_type: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
    pub amount: Decimal,
    pub usd_value: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apy: Option<Decimal>,
    pub unrealized_pnl: Decimal,
}

impl From<PositionRow> for Position {
    fn from(row: PositionRow) -> Self {
        let apy = parse_decimal(&row.apy);
        Self {
            protocol: row.protocol,
            position_type: row.position_type,
            token: row.token,
            pool: if row.pool.is_empty() { None } else { Some(row.pool) },
            amount: parse_decimal(&row.amount),
            usd_value: parse_decimal(&row.usd_value),
            apy: if apy.is_zero() { None } else { Some(apy) },
            unrealized_pnl: parse_decimal(&row.unrealized_pnl),
        }
    }
}

// ============================================================================
// Health check
// ============================================================================

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
}

// ============================================================================
// Index wallet request (for triggering indexing)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct IndexWalletRequest {
    pub wallet: String,
}

#[derive(Debug, Serialize)]
pub struct IndexWalletResponse {
    pub wallet: String,
    pub status: String,
    pub message: String,
}

// ============================================================================
// List subscriptions
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<SubscriptionInfo>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionInfo {
    pub wallet: String,
    pub started_at: String,
    pub transactions_processed: u64,
    pub running: bool,
}
