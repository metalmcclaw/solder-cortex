//! AgentDEX Integration
//!
//! This module provides integration with AgentDEX for enriched DeFi activity data.
//! AgentDEX is an agent-first DEX API that provides swap execution and portfolio data.
//!
//! Integration points:
//! - Portfolio data: Get current token holdings via `/portfolio/:wallet`
//! - Swap history: Track trades executed through AgentDEX
//! - Real-time conviction: Cross-reference trades with prediction market bets
//!
//! API: https://agentdex.solana-clawd.dev (when deployed)
//! Repo: https://github.com/solana-clawd/agent-dex

use crate::models::*;
use crate::error::{CortexError, CortexResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// AgentDEX API base URL
pub const AGENTDEX_API_URL: &str = "https://agentdex.solana-clawd.dev";

/// AgentDEX client configuration
#[derive(Debug, Clone)]
pub struct AgentDexConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl Default for AgentDexConfig {
    fn default() -> Self {
        Self {
            base_url: AGENTDEX_API_URL.to_string(),
            api_key: None,
        }
    }
}

/// Portfolio response from AgentDEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDexPortfolio {
    pub wallet: String,
    pub tokens: Vec<AgentDexToken>,
    pub total_value_usd: f64,
    pub last_updated: DateTime<Utc>,
}

/// Token holding from AgentDEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDexToken {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub balance: f64,
    pub decimals: u8,
    pub price_usd: Option<f64>,
    pub value_usd: Option<f64>,
}

/// Swap record from AgentDEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDexSwap {
    pub signature: String,
    pub wallet: String,
    pub input_mint: String,
    pub input_symbol: String,
    pub input_amount: f64,
    pub output_mint: String,
    pub output_symbol: String,
    pub output_amount: f64,
    pub price_impact_pct: f64,
    pub fee_sol: f64,
    pub executed_at: DateTime<Utc>,
}

/// Convert AgentDEX portfolio to Cortex DeFi positions
pub fn portfolio_to_positions(portfolio: &AgentDexPortfolio) -> Vec<DeFiPosition> {
    portfolio
        .tokens
        .iter()
        .filter(|t| t.value_usd.unwrap_or(0.0) > 0.01) // Filter dust
        .map(|token| DeFiPosition {
            protocol: "agentdex".to_string(),
            position_type: PositionType::Spot,
            token: token.mint.clone(),
            token_symbol: token.symbol.clone(),
            amount: token.balance,
            usd_value: token.value_usd.unwrap_or(0.0),
            entry_price: None, // AgentDEX doesn't track entry price (yet)
            current_price: token.price_usd.unwrap_or(0.0),
            unrealized_pnl: 0.0, // Need historical data for PnL
            opened_at: Utc::now(),
            updated_at: portfolio.last_updated,
            metadata: Some(serde_json::json!({
                "source": "agentdex",
                "name": token.name,
                "decimals": token.decimals,
            })),
        })
        .collect()
}

/// Analyze swap timing relative to prediction market bets
///
/// Returns conviction signals based on swap-bet correlation:
/// - Bought asset AND betting bullish = high conviction
/// - Sold asset AND betting bearish = high conviction
/// - Opposite actions = possible hedge or contradiction
pub fn analyze_swap_conviction(
    swaps: &[AgentDexSwap],
    bets: &[PredictionMarketBet],
) -> Vec<ConvictionSignal> {
    let mut signals = Vec::new();

    for swap in swaps {
        // Find bets related to the swapped tokens
        for bet in bets {
            let bet_asset = extract_bet_asset(&bet.market_title);
            
            // Check if this swap is relevant to this bet
            let is_buy = swap.output_symbol.to_uppercase() == bet_asset;
            let is_sell = swap.input_symbol.to_uppercase() == bet_asset;

            if !is_buy && !is_sell {
                continue;
            }

            // Determine bet direction
            let is_bullish_bet = is_bullish_outcome(&bet.outcome);

            // Calculate time proximity (swaps close to bets are higher conviction)
            let time_diff = (swap.executed_at - bet.created_at).num_hours().abs();
            let timing_factor = if time_diff < 24 {
                1.0 // Same day = high correlation
            } else if time_diff < 168 {
                0.7 // Same week = medium correlation
            } else {
                0.4 // Older = low correlation
            };

            // Size factor (larger trades = higher conviction)
            let size_factor = (swap.output_amount * swap.output_symbol.len() as f64 / 10000.0)
                .min(1.0)
                .max(0.1);

            let (signal_type, description) = if is_buy && is_bullish_bet {
                (
                    SignalType::BullishAlignment,
                    format!(
                        "Bought {} {} via AgentDEX AND betting YES on \"{}\"",
                        swap.output_amount, swap.output_symbol, bet.market_title
                    ),
                )
            } else if is_sell && !is_bullish_bet {
                (
                    SignalType::BearishAlignment,
                    format!(
                        "Sold {} {} via AgentDEX AND betting NO on \"{}\"",
                        swap.input_amount, swap.input_symbol, bet.market_title
                    ),
                )
            } else {
                (
                    SignalType::Contradiction,
                    format!(
                        "AgentDEX trade contradicts bet on \"{}\" - possible hedge",
                        bet.market_title
                    ),
                )
            };

            signals.push(ConvictionSignal {
                signal_type,
                strength: timing_factor * size_factor * 0.8 + 0.2, // Base strength 0.2-1.0
                defi_context: format!(
                    "AgentDEX swap: {} {} -> {} {} (impact: {:.2}%)",
                    swap.input_amount,
                    swap.input_symbol,
                    swap.output_amount,
                    swap.output_symbol,
                    swap.price_impact_pct
                ),
                prediction_context: format!(
                    "{} ${:.2} on \"{}\"",
                    bet.outcome, bet.amount_usd, bet.market_title
                ),
                description,
            });
        }
    }

    signals
}

/// Extract the primary asset from a bet's market title
fn extract_bet_asset(title: &str) -> String {
    let title_upper = title.to_uppercase();
    
    // Common crypto assets
    let assets = [
        ("BITCOIN", "BTC"),
        ("BTC", "BTC"),
        ("ETHEREUM", "ETH"),
        ("ETH", "ETH"),
        ("SOLANA", "SOL"),
        ("SOL", "SOL"),
        ("BONK", "BONK"),
        ("JUP", "JUP"),
        ("USDC", "USDC"),
    ];

    for (pattern, asset) in assets {
        if title_upper.contains(pattern) {
            return asset.to_string();
        }
    }

    "UNKNOWN".to_string()
}

/// Determine if a bet outcome is bullish
fn is_bullish_outcome(outcome: &str) -> bool {
    let outcome_upper = outcome.to_uppercase();
    outcome_upper == "YES"
        || outcome_upper.contains("ABOVE")
        || outcome_upper.contains("OVER")
        || outcome_upper.contains("UP")
        || outcome_upper.contains("HIGHER")
}

/// Webhook payload for real-time swap notifications
///
/// AgentDEX can send webhooks when swaps execute, allowing
/// Solder Cortex to update conviction scores in real-time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDexWebhook {
    pub event_type: String, // "swap_executed"
    pub swap: AgentDexSwap,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bet_asset() {
        assert_eq!(extract_bet_asset("Will Bitcoin reach $100k?"), "BTC");
        assert_eq!(extract_bet_asset("ETH price above $5000"), "ETH");
        assert_eq!(extract_bet_asset("SOL to flip ETH?"), "SOL");
        assert_eq!(extract_bet_asset("Random market"), "UNKNOWN");
    }

    #[test]
    fn test_is_bullish_outcome() {
        assert!(is_bullish_outcome("YES"));
        assert!(is_bullish_outcome("Above $100"));
        assert!(!is_bullish_outcome("NO"));
        assert!(!is_bullish_outcome("Below $50"));
    }

    #[test]
    fn test_portfolio_to_positions() {
        let portfolio = AgentDexPortfolio {
            wallet: "test".to_string(),
            tokens: vec![
                AgentDexToken {
                    mint: "So11111111111111111111111111111111111111112".to_string(),
                    symbol: "SOL".to_string(),
                    name: "Wrapped SOL".to_string(),
                    balance: 100.0,
                    decimals: 9,
                    price_usd: Some(150.0),
                    value_usd: Some(15000.0),
                },
            ],
            total_value_usd: 15000.0,
            last_updated: Utc::now(),
        };

        let positions = portfolio_to_positions(&portfolio);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].token_symbol, "SOL");
        assert_eq!(positions[0].usd_value, 15000.0);
        assert_eq!(positions[0].protocol, "agentdex");
    }
}
