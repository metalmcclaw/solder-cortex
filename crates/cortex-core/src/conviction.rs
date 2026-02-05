//! Conviction calculation engine
//!
//! This module contains the core logic for calculating wallet conviction scores
//! by correlating DeFi positions with prediction market bets.

use crate::models::*;
use crate::error::{CortexError, CortexResult};
use chrono::Utc;

/// Calculate conviction score for a wallet
///
/// This is the core algorithm that bridges DeFi and prediction markets.
/// It analyzes a wallet's positions and bets to determine how "convicted"
/// they are in their market views.
pub fn calculate_conviction(wallet: &Wallet) -> CortexResult<WalletConviction> {
    // Validate we have enough data
    if wallet.defi_positions.is_empty() && wallet.prediction_bets.is_empty() {
        return Err(CortexError::InsufficientData(
            "Wallet has no DeFi positions or prediction bets".to_string(),
        ));
    }

    let mut signals: Vec<ConvictionSignal> = Vec::new();
    let mut total_signal_strength = 0.0;

    // Analyze each prediction bet against DeFi positions
    for bet in &wallet.prediction_bets {
        if let Some(signal) = analyze_bet_conviction(bet, &wallet.defi_positions) {
            total_signal_strength += signal.strength;
            signals.push(signal);
        }
    }

    // Calculate overall conviction score
    let conviction_score = if signals.is_empty() {
        0.0
    } else {
        (total_signal_strength / signals.len() as f64).min(1.0)
    };

    // Determine confidence based on data richness
    let confidence = calculate_confidence(wallet, &signals);

    // Generate interpretation
    let interpretation = generate_interpretation(&signals, conviction_score, &confidence);

    Ok(WalletConviction {
        wallet: wallet.address.clone(),
        conviction_score,
        confidence,
        signals,
        interpretation,
        analyzed_at: Utc::now(),
    })
}

/// Analyze a single prediction bet against DeFi positions
fn analyze_bet_conviction(
    bet: &PredictionMarketBet,
    positions: &[DeFiPosition],
) -> Option<ConvictionSignal> {
    // Extract the underlying asset from the market
    let market_asset = extract_market_asset(&bet.market_title, &bet.category);

    if market_asset.is_none() {
        return None;
    }

    let asset = market_asset.unwrap();

    // Find relevant DeFi positions for this asset
    let relevant_positions: Vec<&DeFiPosition> = positions
        .iter()
        .filter(|p| is_position_relevant(p, &asset))
        .collect();

    if relevant_positions.is_empty() {
        return None;
    }

    // Calculate total exposure to this asset
    let total_exposure: f64 = relevant_positions.iter().map(|p| p.usd_value).sum();
    let total_pnl: f64 = relevant_positions.iter().map(|p| p.unrealized_pnl).sum();

    // Determine if the bet aligns with the position
    let is_bullish_bet = bet.outcome.to_uppercase() == "YES"
        || bet.outcome.to_uppercase().contains("ABOVE")
        || bet.outcome.to_uppercase().contains("OVER")
        || bet.outcome.to_uppercase().contains("UP");

    let is_bullish_position = total_exposure > 0.0 && total_pnl >= 0.0;

    // Calculate signal strength based on:
    // 1. Size of bet relative to portfolio
    // 2. Size of position relative to portfolio
    // 3. Timing correlation (TODO: implement with transaction history)
    let bet_weight = (bet.amount_usd / 1000.0).min(1.0); // Normalize bet size
    let position_weight = (total_exposure / 10000.0).min(1.0); // Normalize position size

    let alignment_score = if is_bullish_bet == is_bullish_position {
        0.7 + (bet_weight * 0.15) + (position_weight * 0.15)
    } else {
        // Contradiction - could be hedging
        0.3
    };

    let (signal_type, description) = if is_bullish_bet == is_bullish_position {
        if is_bullish_bet {
            (
                SignalType::BullishAlignment,
                format!(
                    "Wallet is long ${:.0} in {} AND betting YES on \"{}\"",
                    total_exposure, asset, bet.market_title
                ),
            )
        } else {
            (
                SignalType::BearishAlignment,
                format!(
                    "Wallet has bearish {} exposure AND betting NO on \"{}\"",
                    asset, bet.market_title
                ),
            )
        }
    } else {
        (
            SignalType::Contradiction,
            format!(
                "Wallet's {} position contradicts their bet on \"{}\" - possible hedge",
                asset, bet.market_title
            ),
        )
    };

    Some(ConvictionSignal {
        signal_type,
        strength: alignment_score,
        defi_context: format!(
            "{} positions totaling ${:.2} ({:+.2} PnL)",
            relevant_positions.len(),
            total_exposure,
            total_pnl
        ),
        prediction_context: format!(
            "{} ${:.2} on \"{}\" @ {:.2}",
            bet.outcome, bet.amount_usd, bet.market_title, bet.entry_price
        ),
        description,
    })
}

/// Extract the underlying asset from a prediction market title
fn extract_market_asset(title: &str, category: &str) -> Option<String> {
    let title_lower = title.to_lowercase();

    // Crypto-specific patterns
    let crypto_assets = [
        ("bitcoin", "BTC"),
        ("btc", "BTC"),
        ("ethereum", "ETH"),
        ("eth", "ETH"),
        ("solana", "SOL"),
        ("sol ", "SOL"),
        ("$sol", "SOL"),
    ];

    for (pattern, asset) in crypto_assets {
        if title_lower.contains(pattern) {
            return Some(asset.to_string());
        }
    }

    // Category-based inference
    if category.to_lowercase() == "crypto" {
        // Try to extract from title patterns like "Will X reach Y"
        if title_lower.contains("price") || title_lower.contains("reach") {
            // Basic extraction - can be enhanced
            return Some("CRYPTO".to_string());
        }
    }

    None
}

/// Check if a DeFi position is relevant to an asset
fn is_position_relevant(position: &DeFiPosition, asset: &str) -> bool {
    let token_upper = position.token_symbol.to_uppercase();
    let asset_upper = asset.to_uppercase();

    // Direct match
    if token_upper == asset_upper {
        return true;
    }

    // Wrapped token match (e.g., WETH -> ETH)
    if token_upper.starts_with('W') && &token_upper[1..] == asset_upper {
        return true;
    }

    // LP token match (basic heuristic)
    if token_upper.contains(&asset_upper) && position.position_type == PositionType::LiquidityPool {
        return true;
    }

    false
}

/// Calculate confidence level based on data richness
fn calculate_confidence(wallet: &Wallet, signals: &[ConvictionSignal]) -> ConvictionConfidence {
    let position_count = wallet.defi_positions.len();
    let bet_count = wallet.prediction_bets.len();
    let signal_count = signals.len();

    // High confidence: multiple positions, multiple bets, multiple signals
    if position_count >= 3 && bet_count >= 2 && signal_count >= 2 {
        return ConvictionConfidence::High;
    }

    // Medium confidence: some data, at least one signal
    if (position_count >= 1 || bet_count >= 1) && signal_count >= 1 {
        return ConvictionConfidence::Medium;
    }

    ConvictionConfidence::Low
}

/// Generate human-readable interpretation
fn generate_interpretation(
    signals: &[ConvictionSignal],
    score: f64,
    confidence: &ConvictionConfidence,
) -> String {
    if signals.is_empty() {
        return "No cross-domain signals detected. Wallet has no correlatable activity between DeFi and prediction markets.".to_string();
    }

    let bullish_count = signals
        .iter()
        .filter(|s| s.signal_type == SignalType::BullishAlignment)
        .count();
    let bearish_count = signals
        .iter()
        .filter(|s| s.signal_type == SignalType::BearishAlignment)
        .count();
    let contradiction_count = signals
        .iter()
        .filter(|s| s.signal_type == SignalType::Contradiction)
        .count();

    let direction = if bullish_count > bearish_count {
        "bullish"
    } else if bearish_count > bullish_count {
        "bearish"
    } else {
        "mixed"
    };

    let confidence_str = match confidence {
        ConvictionConfidence::High => "High confidence",
        ConvictionConfidence::Medium => "Medium confidence",
        ConvictionConfidence::Low => "Low confidence",
    };

    let conviction_str = if score > 0.7 {
        "strong"
    } else if score > 0.4 {
        "moderate"
    } else {
        "weak"
    };

    let mut interpretation = format!(
        "{}: Wallet shows {} {} conviction (score: {:.2}). ",
        confidence_str, conviction_str, direction, score
    );

    if contradiction_count > 0 {
        interpretation.push_str(&format!(
            "Note: {} contradictory signal(s) detected - may indicate hedging strategy. ",
            contradiction_count
        ));
    }

    interpretation.push_str(&format!(
        "Analysis based on {} cross-domain signal(s).",
        signals.len()
    ));

    interpretation
}

/// Convert WalletConviction to API response format
pub fn conviction_to_response(
    conviction: &WalletConviction,
    wallet: &Wallet,
) -> WalletConvictionResponse {
    let defi_summary = DeFiSummary {
        total_value_usd: wallet.defi_positions.iter().map(|p| p.usd_value).sum(),
        position_count: wallet.defi_positions.len(),
        protocols: wallet.protocols.clone(),
        dominant_exposure: wallet
            .defi_positions
            .iter()
            .max_by(|a, b| a.usd_value.partial_cmp(&b.usd_value).unwrap())
            .map(|p| p.token_symbol.clone())
            .unwrap_or_else(|| "N/A".to_string()),
    };

    let prediction_summary = PredictionSummary {
        total_bet_usd: wallet.prediction_bets.iter().map(|b| b.amount_usd).sum(),
        bet_count: wallet.prediction_bets.len(),
        platforms: wallet
            .prediction_bets
            .iter()
            .map(|b| b.platform.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect(),
        categories: wallet
            .prediction_bets
            .iter()
            .map(|b| b.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect(),
    };

    WalletConvictionResponse {
        wallet: conviction.wallet.clone(),
        conviction_score: conviction.conviction_score,
        confidence: format!("{:?}", conviction.confidence).to_lowercase(),
        signals_count: conviction.signals.len(),
        signals: conviction
            .signals
            .iter()
            .map(|s| ConvictionSignalResponse {
                signal_type: format!("{:?}", s.signal_type).to_lowercase(),
                strength: s.strength,
                description: s.description.clone(),
            })
            .collect(),
        interpretation: conviction.interpretation.clone(),
        defi_summary,
        prediction_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_market_asset() {
        assert_eq!(
            extract_market_asset("Will Bitcoin reach $100k?", "crypto"),
            Some("BTC".to_string())
        );
        assert_eq!(
            extract_market_asset("ETH price above $5000 by March", "crypto"),
            Some("ETH".to_string())
        );
        assert_eq!(
            extract_market_asset("Will SOL flip ETH?", "crypto"),
            Some("SOL".to_string())
        );
    }

    #[test]
    fn test_is_position_relevant() {
        let position = DeFiPosition {
            protocol: "jupiter".to_string(),
            position_type: PositionType::Spot,
            token: "So11111111111111111111111111111111111111112".to_string(),
            token_symbol: "SOL".to_string(),
            amount: 100.0,
            usd_value: 15000.0,
            entry_price: Some(100.0),
            current_price: 150.0,
            unrealized_pnl: 5000.0,
            opened_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: None,
        };

        assert!(is_position_relevant(&position, "SOL"));
        assert!(!is_position_relevant(&position, "ETH"));
    }
}
