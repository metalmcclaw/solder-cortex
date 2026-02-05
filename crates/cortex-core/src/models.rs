//! Unified data models for cross-domain intelligence
//!
//! These models represent the canonical structures that bridge DeFi analytics
//! and prediction market data, enabling conviction scoring and informed trader detection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// =============================================================================
// Core Wallet Entity
// =============================================================================

/// The unified wallet entity - the primary key that links all cross-domain data.
///
/// This is the heart of Solder Cortex. A wallet is not just an address; it's
/// an entity with DeFi positions, prediction market bets, and behavioral patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Solana wallet address (base58 encoded)
    pub address: String,

    /// Total portfolio value in USD across all protocols
    pub total_value_usd: f64,

    /// DeFi positions across all indexed protocols
    pub defi_positions: Vec<DeFiPosition>,

    /// Prediction market bets across all indexed platforms
    pub prediction_bets: Vec<PredictionMarketBet>,

    /// Wallet classification (whale, trader, bot, etc.)
    pub classification: Option<WalletClassification>,

    /// Risk score (0-100, higher = riskier)
    pub risk_score: u8,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Protocols this wallet has interacted with
    pub protocols: Vec<String>,
}

/// Wallet classification based on behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WalletClassification {
    Whale,
    Trader,
    Bot,
    Fund,
    Exchange,
    Retail,
    New,
    Unknown,
}

// =============================================================================
// DeFi Domain Models
// =============================================================================

/// A normalized DeFi position across any protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeFiPosition {
    /// Protocol name (e.g., "jupiter", "raydium", "kamino")
    pub protocol: String,

    /// Position type (e.g., "swap", "lp", "lending", "staking")
    pub position_type: PositionType,

    /// Primary token involved
    pub token: String,

    /// Token symbol (e.g., "SOL", "ETH", "USDC")
    pub token_symbol: String,

    /// Amount of token held
    pub amount: f64,

    /// Current USD value
    pub usd_value: f64,

    /// Entry price (if applicable)
    pub entry_price: Option<f64>,

    /// Current token price
    pub current_price: f64,

    /// Unrealized PnL in USD
    pub unrealized_pnl: f64,

    /// Position opened timestamp
    pub opened_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Additional metadata (pool address, etc.)
    pub metadata: Option<serde_json::Value>,
}

/// Type of DeFi position
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    Spot,
    Swap,
    LiquidityPool,
    Lending,
    Borrowing,
    Staking,
    Farming,
    Perpetual,
    Options,
    Other,
}

// =============================================================================
// Prediction Market Domain Models
// =============================================================================

/// A normalized prediction market bet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionMarketBet {
    /// Platform name (e.g., "polymarket", "kalshi")
    pub platform: String,

    /// Market identifier/slug
    pub market_slug: String,

    /// Market title/question
    pub market_title: String,

    /// Outcome the wallet bet on (e.g., "YES", "NO", or specific outcome)
    pub outcome: String,

    /// Amount wagered in USD
    pub amount_usd: f64,

    /// Average entry price for outcome tokens
    pub entry_price: f64,

    /// Current price of outcome tokens
    pub current_price: f64,

    /// Number of outcome tokens held
    pub shares: f64,

    /// Unrealized PnL
    pub unrealized_pnl: f64,

    /// Market category (e.g., "crypto", "politics", "sports")
    pub category: String,

    /// Market resolution date (if known)
    pub resolution_date: Option<DateTime<Utc>>,

    /// Bet placed timestamp
    pub placed_at: DateTime<Utc>,

    /// Market status
    pub market_status: MarketStatus,
}

/// Status of a prediction market
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketStatus {
    Open,
    Closed,
    Resolved,
    Disputed,
}

// =============================================================================
// Cross-Domain Intelligence Models
// =============================================================================

/// The result of analyzing a wallet's cross-domain conviction
///
/// This is the key insight Solder Cortex provides: connecting what a wallet
/// does in DeFi with what it bets on in prediction markets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConviction {
    /// Wallet address
    pub wallet: String,

    /// Overall conviction score (0.0 - 1.0)
    /// Higher = stronger alignment between actions and bets
    pub conviction_score: f64,

    /// Confidence level in the score (based on data availability)
    pub confidence: ConvictionConfidence,

    /// Individual conviction signals detected
    pub signals: Vec<ConvictionSignal>,

    /// Summary interpretation for agents
    pub interpretation: String,

    /// Timestamp of analysis
    pub analyzed_at: DateTime<Utc>,
}

/// Confidence level in conviction calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConvictionConfidence {
    /// High confidence - multiple correlated signals
    High,
    /// Medium confidence - some signals, limited data
    Medium,
    /// Low confidence - minimal data, speculative
    Low,
}

/// A single conviction signal - a detected correlation between domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvictionSignal {
    /// Type of signal detected
    pub signal_type: SignalType,

    /// Strength of signal (0.0 - 1.0)
    pub strength: f64,

    /// DeFi position(s) involved
    pub defi_context: String,

    /// Prediction market bet(s) involved
    pub prediction_context: String,

    /// Human-readable description
    pub description: String,
}

/// Types of conviction signals we can detect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Wallet is long a token AND betting on price increase
    BullishAlignment,
    /// Wallet is short/sold a token AND betting on price decrease
    BearishAlignment,
    /// Wallet actions contradict their bets (potential hedge or misdirection)
    Contradiction,
    /// Wallet accumulated before placing bet (informed trading)
    FrontRunning,
    /// Wallet size in market is significant relative to portfolio
    HighConviction,
    /// Wallet has history of accurate predictions
    TrackRecord,
}

/// Result of detecting informed traders in a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformedTraderAnalysis {
    /// Market being analyzed
    pub market_slug: String,

    /// Platform
    pub platform: String,

    /// Number of informed traders detected
    pub informed_count: usize,

    /// Traders with correlated on-chain activity
    pub informed_traders: Vec<InformedTrader>,

    /// Aggregate signal: which way are informed traders leaning?
    pub aggregate_signal: AggregateSignal,

    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

/// An individual informed trader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformedTrader {
    /// Wallet address
    pub wallet: String,

    /// Classification
    pub classification: WalletClassification,

    /// Their bet in this market
    pub bet_outcome: String,

    /// Size of their bet in USD
    pub bet_size_usd: f64,

    /// Their relevant on-chain activity
    pub onchain_activity: String,

    /// Conviction score for this trader
    pub conviction_score: f64,
}

/// Aggregate signal from informed traders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSignal {
    /// Dominant direction ("bullish", "bearish", "mixed")
    pub direction: String,

    /// Percentage of informed traders aligned with dominant direction
    pub alignment_pct: f64,

    /// Total USD committed by informed traders
    pub total_informed_usd: f64,

    /// Confidence in aggregate signal
    pub confidence: ConvictionConfidence,
}

// =============================================================================
// API Response Models (Agent-Readable)
// =============================================================================

/// Response for the `cortex_get_wallet_conviction` MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConvictionResponse {
    pub wallet: String,
    pub conviction_score: f64,
    pub confidence: String,
    pub signals_count: usize,
    pub signals: Vec<ConvictionSignalResponse>,
    pub interpretation: String,
    pub defi_summary: DeFiSummary,
    pub prediction_summary: PredictionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvictionSignalResponse {
    pub signal_type: String,
    pub strength: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeFiSummary {
    pub total_value_usd: f64,
    pub position_count: usize,
    pub protocols: Vec<String>,
    pub dominant_exposure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionSummary {
    pub total_bet_usd: f64,
    pub bet_count: usize,
    pub platforms: Vec<String>,
    pub categories: Vec<String>,
}

/// Response for the `cortex_detect_informed_traders` MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformedTradersResponse {
    pub market_slug: String,
    pub platform: String,
    pub informed_traders_count: usize,
    pub aggregate_signal: AggregateSignalResponse,
    pub traders: Vec<InformedTraderResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSignalResponse {
    pub direction: String,
    pub alignment_pct: f64,
    pub total_informed_usd: f64,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformedTraderResponse {
    pub wallet: String,
    pub classification: String,
    pub bet_outcome: String,
    pub bet_size_usd: f64,
    pub conviction_score: f64,
    pub onchain_activity: String,
}
