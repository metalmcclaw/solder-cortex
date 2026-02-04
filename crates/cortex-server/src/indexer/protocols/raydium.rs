use rust_decimal::Decimal;

use crate::indexer::lyslabs::LysTransaction;
use crate::indexer::parser::ParsedTransaction;
use crate::types::{Protocol, TransactionType};

pub struct RaydiumParser;

impl RaydiumParser {
    pub const PROGRAM_IDS: &'static [&'static str] = &[
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM v4
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CLMM
        "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS",  // Raydium Router
    ];

    pub fn is_raydium_tx(tx: &LysTransaction) -> bool {
        // Check decoder type
        if tx.decoder_type.to_lowercase().contains("raydium") {
            return true;
        }

        // Check program ID
        if Self::PROGRAM_IDS.contains(&tx.program_id.as_str()) {
            return true;
        }

        false
    }

    pub fn parse(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        if !Self::is_raydium_tx(tx) {
            return None;
        }

        match tx.event_type.to_uppercase().as_str() {
            "SWAP" => Self::parse_swap(tx, wallet),
            "ADD_LIQUIDITY" => Self::parse_add_liquidity(tx, wallet),
            "REMOVE_LIQUIDITY" => Self::parse_remove_liquidity(tx, wallet),
            _ => None,
        }
    }

    fn parse_swap(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        // Extract input
        let (token_in, amount_in) = if let Some(ref ti) = tx.token_in {
            (ti.mint.clone(), Decimal::try_from(ti.ui_amount).unwrap_or_default())
        } else {
            return None;
        };

        // Extract output
        let (token_out, amount_out) = if let Some(ref to) = tx.token_out {
            (to.mint.clone(), Decimal::try_from(to.ui_amount).unwrap_or_default())
        } else {
            return None;
        };

        Some(ParsedTransaction {
            signature: tx.tx_signature.clone(),
            wallet: wallet.to_string(),
            protocol: Protocol::Raydium,
            tx_type: TransactionType::Swap,
            token_in,
            token_out,
            amount_in,
            amount_out,
            usd_value: Decimal::ZERO,
            block_time: tx.block_time * 1000,
            slot: tx.slot,
        })
    }

    fn parse_add_liquidity(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        // Use ui_amount if available, otherwise parse amount string
        let amount = if tx.ui_amount != 0.0 {
            tx.ui_amount
        } else {
            tx.amount.parse::<f64>().unwrap_or(0.0)
        };

        Some(ParsedTransaction {
            signature: tx.tx_signature.clone(),
            wallet: wallet.to_string(),
            protocol: Protocol::Raydium,
            tx_type: TransactionType::AddLiquidity,
            token_in: tx.mint.clone(),
            token_out: "LP".to_string(),
            amount_in: Decimal::try_from(amount).unwrap_or_default(),
            amount_out: Decimal::ZERO,
            usd_value: Decimal::ZERO,
            block_time: tx.block_time * 1000,
            slot: tx.slot,
        })
    }

    fn parse_remove_liquidity(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        // Use ui_amount if available, otherwise parse amount string
        let amount = if tx.ui_amount != 0.0 {
            tx.ui_amount
        } else {
            tx.amount.parse::<f64>().unwrap_or(0.0)
        };

        Some(ParsedTransaction {
            signature: tx.tx_signature.clone(),
            wallet: wallet.to_string(),
            protocol: Protocol::Raydium,
            tx_type: TransactionType::RemoveLiquidity,
            token_in: "LP".to_string(),
            token_out: tx.mint.clone(),
            amount_in: Decimal::ZERO,
            amount_out: Decimal::try_from(amount).unwrap_or_default(),
            usd_value: Decimal::ZERO,
            block_time: tx.block_time * 1000,
            slot: tx.slot,
        })
    }
}
