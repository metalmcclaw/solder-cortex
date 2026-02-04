use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::indexer::parser::ParsedTransaction;
use crate::types::{Protocol, TransactionType};

#[derive(Debug, Clone, Default)]
pub struct RiskMetrics {
    pub score: u8,
    pub largest_position_pct: Decimal,
    pub position_count: u16,
    pub protocol_concentration: Decimal,
}

pub fn compute_risk(transactions: &[ParsedTransaction]) -> RiskMetrics {
    if transactions.is_empty() {
        return RiskMetrics::default();
    }

    // Track positions by token/protocol
    let mut positions: HashMap<(String, Protocol), Decimal> = HashMap::new();
    let mut protocols: HashSet<Protocol> = HashSet::new();

    for tx in transactions {
        protocols.insert(tx.protocol);

        match tx.tx_type {
            TransactionType::Deposit | TransactionType::AddLiquidity | TransactionType::Borrow => {
                let key = (tx.token_in.clone(), tx.protocol);
                *positions.entry(key).or_default() += tx.usd_value;
            }
            TransactionType::Withdraw | TransactionType::RemoveLiquidity | TransactionType::Repay => {
                let key = (tx.token_out.clone(), tx.protocol);
                let pos = positions.entry(key).or_default();
                *pos = (*pos - tx.usd_value).max(Decimal::ZERO);
            }
            TransactionType::Swap => {
                // Swaps don't directly create positions, but indicate activity
            }
        }
    }

    // Calculate metrics
    let total_value: Decimal = positions.values().copied().sum();
    let position_count = positions.len() as u16;

    // Largest position percentage
    let largest_position = positions.values().copied().max().unwrap_or_default();
    let largest_position_pct = if total_value > Decimal::ZERO {
        largest_position / total_value
    } else {
        Decimal::ZERO
    };

    // Protocol concentration (inverse of diversification)
    let mut protocol_values: HashMap<Protocol, Decimal> = HashMap::new();
    for ((_, protocol), value) in &positions {
        *protocol_values.entry(*protocol).or_default() += value;
    }

    let largest_protocol_value = protocol_values.values().copied().max().unwrap_or_default();
    let protocol_concentration = if total_value > Decimal::ZERO {
        largest_protocol_value / total_value
    } else {
        Decimal::ZERO
    };

    // Calculate risk score (0-100)
    let score = calculate_risk_score(
        largest_position_pct,
        protocol_concentration,
        protocols.len(),
        position_count,
    );

    RiskMetrics {
        score,
        largest_position_pct,
        position_count,
        protocol_concentration,
    }
}

fn calculate_risk_score(
    largest_position_pct: Decimal,
    protocol_concentration: Decimal,
    protocol_count: usize,
    position_count: u16,
) -> u8 {
    let mut score: u8 = 0;

    // Concentration risk (0-40 points)
    // Higher concentration = higher risk
    let concentration_score = (largest_position_pct * dec!(40))
        .to_string()
        .parse::<f64>()
        .unwrap_or(0.0) as u8;
    score += concentration_score.min(40);

    // Protocol concentration risk (0-30 points)
    let protocol_risk = (protocol_concentration * dec!(30))
        .to_string()
        .parse::<f64>()
        .unwrap_or(0.0) as u8;
    score += protocol_risk.min(30);

    // Diversification bonus (reduces risk)
    // More protocols = lower risk
    let diversification_bonus = match protocol_count {
        0 | 1 => 20,
        2 => 10,
        3 => 5,
        _ => 0,
    };
    score += diversification_bonus;

    // Position count factor
    // Too few or too many positions can be risky
    let position_factor = match position_count {
        0 => 10,
        1..=3 => 5,
        4..=10 => 0,
        _ => 5, // Many positions = complexity risk
    };
    score += position_factor;

    score.min(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_transactions() {
        let risk = compute_risk(&[]);
        assert_eq!(risk.score, 0);
        assert_eq!(risk.position_count, 0);
    }

    #[test]
    fn test_single_protocol_high_concentration() {
        // Single large position should have high risk
        let risk = calculate_risk_score(
            dec!(0.8),  // 80% in one position
            dec!(1.0),  // 100% in one protocol
            1,          // 1 protocol
            1,          // 1 position
        );
        assert!(risk > 50, "High concentration should result in high risk score");
    }

    #[test]
    fn test_diversified_portfolio() {
        let risk = calculate_risk_score(
            dec!(0.2),  // 20% max position
            dec!(0.33), // 33% per protocol
            3,          // 3 protocols
            6,          // 6 positions
        );
        assert!(risk < 30, "Diversified portfolio should have lower risk");
    }
}
