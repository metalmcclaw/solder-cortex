use std::collections::HashMap;

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::indexer::parser::ParsedTransaction;
use crate::types::TransactionType;

#[derive(Debug, Clone, Default)]
pub struct PnlMetrics {
    pub total_value: Decimal,
    pub realized_24h: Decimal,
    pub realized_7d: Decimal,
    pub realized_30d: Decimal,
    pub unrealized: Decimal,
}

pub fn compute_pnl(transactions: &[ParsedTransaction]) -> PnlMetrics {
    let now = Utc::now().timestamp_millis();
    let day_ago = now - Duration::days(1).num_milliseconds();
    let week_ago = now - Duration::days(7).num_milliseconds();
    let month_ago = now - Duration::days(30).num_milliseconds();

    let mut metrics = PnlMetrics::default();

    // Track token positions for unrealized PnL
    let mut token_positions: HashMap<String, TokenPosition> = HashMap::new();

    for tx in transactions {
        let is_realized = matches!(
            tx.tx_type,
            TransactionType::Swap
                | TransactionType::Withdraw
                | TransactionType::RemoveLiquidity
        );

        if is_realized {
            // For realized trades, we calculate PnL as output - input (simplified)
            let pnl = tx.usd_value; // USD value represents the trade value

            // Add to time-windowed realized PnL
            if tx.block_time >= day_ago {
                metrics.realized_24h += pnl;
            }
            if tx.block_time >= week_ago {
                metrics.realized_7d += pnl;
            }
            if tx.block_time >= month_ago {
                metrics.realized_30d += pnl;
            }
        }

        // Track positions for unrealized PnL
        match tx.tx_type {
            TransactionType::Deposit | TransactionType::AddLiquidity => {
                let position = token_positions
                    .entry(tx.token_in.clone())
                    .or_default();
                position.amount += tx.amount_in;
                position.cost_basis += tx.usd_value;
            }
            TransactionType::Withdraw | TransactionType::RemoveLiquidity => {
                let position = token_positions
                    .entry(tx.token_out.clone())
                    .or_default();
                position.amount -= tx.amount_out;
                // Proportionally reduce cost basis
                if !position.amount.is_zero() {
                    let ratio = tx.amount_out / (position.amount + tx.amount_out);
                    position.cost_basis *= dec!(1) - ratio;
                }
            }
            TransactionType::Swap => {
                // Swaps: reduce input position, increase output position
                if let Some(position) = token_positions.get_mut(&tx.token_in) {
                    position.amount -= tx.amount_in;
                }
                let output_position = token_positions
                    .entry(tx.token_out.clone())
                    .or_default();
                output_position.amount += tx.amount_out;
                output_position.cost_basis += tx.usd_value;
            }
            _ => {}
        }
    }

    // Calculate total value and unrealized PnL from remaining positions
    for (_, position) in &token_positions {
        if position.amount > Decimal::ZERO {
            // In production, we'd fetch current prices to calculate actual unrealized PnL
            // For now, we use cost basis as a proxy for value
            metrics.total_value += position.cost_basis;
        }
    }

    metrics
}

#[derive(Debug, Default)]
struct TokenPosition {
    amount: Decimal,
    cost_basis: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Protocol;

    fn make_tx(
        tx_type: TransactionType,
        token_in: &str,
        amount_in: Decimal,
        usd_value: Decimal,
        timestamp: i64,
    ) -> ParsedTransaction {
        ParsedTransaction {
            signature: "test".to_string(),
            wallet: "wallet".to_string(),
            protocol: Protocol::Jupiter,
            tx_type,
            token_in: token_in.to_string(),
            token_out: String::new(),
            amount_in,
            amount_out: Decimal::ZERO,
            usd_value,
            block_time: timestamp,
            slot: 0,
        }
    }

    #[test]
    fn test_empty_transactions() {
        let pnl = compute_pnl(&[]);
        assert_eq!(pnl.total_value, Decimal::ZERO);
        assert_eq!(pnl.realized_24h, Decimal::ZERO);
    }
}
