use rust_decimal::Decimal;

use crate::indexer::lyslabs::LysTransaction;
use crate::indexer::parser::ParsedTransaction;
use crate::types::{Protocol, TransactionType};

pub struct JupiterParser;

impl JupiterParser {
    pub const PROGRAM_IDS: &'static [&'static str] = &[
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", // Jupiter v6
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB", // Jupiter v4
        "JUP3c2Uh3WA4Ng34tw6kPd2G4C5BB21Xo36Je1s32Ph", // Jupiter v3
    ];

    pub fn is_jupiter_tx(tx: &LysTransaction) -> bool {
        // Check decoder type
        if tx.decoder_type.to_lowercase().contains("jupiter") {
            return true;
        }

        // Check program ID
        if Self::PROGRAM_IDS.contains(&tx.program_id.as_str()) {
            return true;
        }

        false
    }

    pub fn parse(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        if !Self::is_jupiter_tx(tx) {
            return None;
        }

        // Jupiter transactions are primarily swaps
        if tx.event_type.to_uppercase() != "SWAP" {
            return None;
        }

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
            protocol: Protocol::Jupiter,
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
}
