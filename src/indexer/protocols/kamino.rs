use rust_decimal::Decimal;

use crate::indexer::lyslabs::LysTransaction;
use crate::indexer::parser::ParsedTransaction;
use crate::types::{Protocol, TransactionType};

pub struct KaminoParser;

impl KaminoParser {
    pub const PROGRAM_IDS: &'static [&'static str] = &[
        "KLend2g3cP87ber41L3rfCMYbkK3YqPjSSahS1E3HVK",  // Kamino Lending
        "6LtLpnUFNByNXLyCoK9wA2MykKAmQNZKBdY8s47dehDc", // Kamino Liquidity
        "kvauTFR8qm1dhniz6pYuBZkuene3Hfrs1VQhVRgCNrr",  // Kamino Vaults
    ];

    pub fn is_kamino_tx(tx: &LysTransaction) -> bool {
        // Check decoder type
        if tx.decoder_type.to_lowercase().contains("kamino") {
            return true;
        }

        // Check program ID
        if Self::PROGRAM_IDS.contains(&tx.program_id.as_str()) {
            return true;
        }

        false
    }

    pub fn parse(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
        if !Self::is_kamino_tx(tx) {
            return None;
        }

        // Determine transaction type from event type
        let tx_type = match tx.event_type.to_uppercase().as_str() {
            "DEPOSIT" | "SUPPLY" => TransactionType::Deposit,
            "WITHDRAW" | "REDEEM" => TransactionType::Withdraw,
            "BORROW" => TransactionType::Borrow,
            "REPAY" => TransactionType::Repay,
            _ => return None,
        };

        // Use ui_amount if available, otherwise parse amount string
        let amount = if tx.ui_amount != 0.0 {
            tx.ui_amount
        } else {
            tx.amount.parse::<f64>().unwrap_or(0.0)
        };

        match tx_type {
            TransactionType::Deposit | TransactionType::Repay => {
                Some(ParsedTransaction {
                    signature: tx.tx_signature.clone(),
                    wallet: wallet.to_string(),
                    protocol: Protocol::Kamino,
                    tx_type,
                    token_in: tx.mint.clone(),
                    token_out: String::new(),
                    amount_in: Decimal::try_from(amount).unwrap_or_default(),
                    amount_out: Decimal::ZERO,
                    usd_value: Decimal::ZERO,
                    block_time: tx.block_time * 1000,
                    slot: tx.slot,
                })
            }
            TransactionType::Withdraw | TransactionType::Borrow => {
                Some(ParsedTransaction {
                    signature: tx.tx_signature.clone(),
                    wallet: wallet.to_string(),
                    protocol: Protocol::Kamino,
                    tx_type,
                    token_in: String::new(),
                    token_out: tx.mint.clone(),
                    amount_in: Decimal::ZERO,
                    amount_out: Decimal::try_from(amount).unwrap_or_default(),
                    usd_value: Decimal::ZERO,
                    block_time: tx.block_time * 1000,
                    slot: tx.slot,
                })
            }
            _ => None,
        }
    }
}
