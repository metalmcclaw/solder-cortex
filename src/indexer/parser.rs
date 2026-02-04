use rust_decimal::Decimal;
use tracing;

use super::lyslabs::LysTransaction;
use crate::db::models::TransactionRow;
use crate::types::{Protocol, TransactionType};

#[derive(Clone)]
pub struct ParsedTransaction {
    pub signature: String,
    pub wallet: String,
    pub protocol: Protocol,
    pub tx_type: TransactionType,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub usd_value: Decimal,
    pub block_time: i64,
    pub slot: u64,
}

impl ParsedTransaction {
    pub fn to_row(&self) -> TransactionRow {
        TransactionRow {
            signature: self.signature.clone(),
            wallet: self.wallet.clone(),
            protocol: self.protocol.to_string(),
            tx_type: self.tx_type.to_string(),
            token_in: self.token_in.clone(),
            token_out: self.token_out.clone(),
            amount_in: self.amount_in.to_string(),
            amount_out: self.amount_out.to_string(),
            usd_value: self.usd_value.to_string(),
            block_time: self.block_time,
            slot: self.slot,
        }
    }
}

/// Parse a LYS Labs transaction into a ParsedTransaction
pub fn parse_transaction(tx: &LysTransaction, wallet: &str) -> Option<ParsedTransaction> {
    tracing::trace!(
        signature = %tx.tx_signature,
        decoder_type = %tx.decoder_type,
        event_type = %tx.event_type,
        "Parsing transaction"
    );

    // Try to identify the protocol from decoder type and program ID
    let protocol = match identify_protocol(tx) {
        Some(p) => {
            tracing::trace!(signature = %tx.tx_signature, protocol = ?p, "Protocol identified");
            p
        }
        None => {
            tracing::trace!(
                signature = %tx.tx_signature,
                decoder_type = %tx.decoder_type,
                program_id = %tx.program_id,
                "Could not identify protocol, skipping transaction"
            );
            return None;
        }
    };

    // Parse based on event type
    let result = match tx.event_type.to_uppercase().as_str() {
        "SWAP" => {
            tracing::trace!(signature = %tx.tx_signature, "Parsing as SWAP");
            parse_swap(tx, wallet, protocol)
        }
        "TRANSFER" => {
            tracing::trace!(signature = %tx.tx_signature, "Skipping TRANSFER transaction");
            None // We don't track simple transfers
        }
        "DEPOSIT" | "SUPPLY" => {
            tracing::trace!(signature = %tx.tx_signature, "Parsing as DEPOSIT/SUPPLY");
            parse_lending_operation(tx, wallet, protocol, TransactionType::Deposit)
        }
        "WITHDRAW" | "REDEEM" => {
            tracing::trace!(signature = %tx.tx_signature, "Parsing as WITHDRAW/REDEEM");
            parse_lending_operation(tx, wallet, protocol, TransactionType::Withdraw)
        }
        "BORROW" => {
            tracing::trace!(signature = %tx.tx_signature, "Parsing as BORROW");
            parse_lending_operation(tx, wallet, protocol, TransactionType::Borrow)
        }
        "REPAY" => {
            tracing::trace!(signature = %tx.tx_signature, "Parsing as REPAY");
            parse_lending_operation(tx, wallet, protocol, TransactionType::Repay)
        }
        _ => {
            // Check decoder type for additional context
            if is_swap_decoder(&tx.decoder_type) {
                tracing::trace!(
                    signature = %tx.tx_signature,
                    event_type = %tx.event_type,
                    "Unknown event type but swap decoder detected, parsing as swap"
                );
                parse_swap(tx, wallet, protocol)
            } else {
                tracing::trace!(
                    signature = %tx.tx_signature,
                    event_type = %tx.event_type,
                    "Unknown event type and not a swap decoder, skipping"
                );
                None
            }
        }
    };

    if let Some(ref parsed) = result {
        tracing::debug!(
            signature = %parsed.signature,
            protocol = ?parsed.protocol,
            tx_type = ?parsed.tx_type,
            amount_in = %parsed.amount_in,
            amount_out = %parsed.amount_out,
            "Transaction parsed successfully"
        );
    }

    result
}

fn identify_protocol(tx: &LysTransaction) -> Option<Protocol> {
    let decoder_lower = tx.decoder_type.to_lowercase();

    // Check decoder type first
    if decoder_lower.contains("jupiter") {
        return Some(Protocol::Jupiter);
    }
    if decoder_lower.contains("raydium") {
        return Some(Protocol::Raydium);
    }
    if decoder_lower.contains("kamino") {
        return Some(Protocol::Kamino);
    }
    if decoder_lower.contains("meteora") {
        return Some(Protocol::Meteora);
    }
    if decoder_lower.contains("orca") {
        return Some(Protocol::Orca);
    }
    if decoder_lower.contains("pump") {
        return Some(Protocol::PumpFun);
    }

    // Check program ID
    identify_protocol_by_program_id(&tx.program_id)
}

fn identify_protocol_by_program_id(program_id: &str) -> Option<Protocol> {
    match program_id {
        // Jupiter v6
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => Some(Protocol::Jupiter),
        // Jupiter v4
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => Some(Protocol::Jupiter),
        // Raydium AMM
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(Protocol::Raydium),
        // Raydium CLMM
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some(Protocol::Raydium),
        // Kamino Lending
        "KLend2g3cP87ber41L3rfCMYbkK3YqPjSSahS1E3HVK" => Some(Protocol::Kamino),
        // Kamino Liquidity
        "6LtLpnUFNByNXLyCoK9wA2MykKAmQNZKBdY8s47dehDc" => Some(Protocol::Kamino),
        // Meteora DLMM
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => Some(Protocol::Meteora),
        // Orca Whirlpool
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(Protocol::Orca),
        // Pump.fun
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(Protocol::PumpFun),
        _ => None,
    }
}

fn is_swap_decoder(decoder_type: &str) -> bool {
    let decoder_lower = decoder_type.to_lowercase();
    decoder_lower.contains("swap")
        || decoder_lower.contains("raydium")
        || decoder_lower.contains("jupiter")
        || decoder_lower.contains("meteora")
        || decoder_lower.contains("orca")
        || decoder_lower.contains("pump")
}

fn parse_swap(tx: &LysTransaction, wallet: &str, protocol: Protocol) -> Option<ParsedTransaction> {
    // Try to get token in/out from dedicated fields
    let (token_in, amount_in) = if let Some(ref ti) = tx.token_in {
        (
            ti.mint.clone(),
            Decimal::try_from(ti.ui_amount).unwrap_or_default(),
        )
    } else if !tx.mint.is_empty() && tx.source == wallet {
        // Token being sent from wallet - use ui_amount or parse amount string
        let amount = if tx.ui_amount != 0.0 {
            tx.ui_amount
        } else {
            tx.amount.parse::<f64>().unwrap_or(0.0)
        };
        (tx.mint.clone(), Decimal::try_from(amount).unwrap_or_default())
    } else {
        (String::new(), Decimal::ZERO)
    };

    let (token_out, amount_out) = if let Some(ref to) = tx.token_out {
        (
            to.mint.clone(),
            Decimal::try_from(to.ui_amount).unwrap_or_default(),
        )
    } else if !tx.mint.is_empty() && tx.destination == wallet {
        // Token being received by wallet - use ui_amount or parse amount string
        let amount = if tx.ui_amount != 0.0 {
            tx.ui_amount
        } else {
            tx.amount.parse::<f64>().unwrap_or(0.0)
        };
        (tx.mint.clone(), Decimal::try_from(amount).unwrap_or_default())
    } else {
        (String::new(), Decimal::ZERO)
    };

    // Skip if we couldn't identify tokens
    if token_in.is_empty() && token_out.is_empty() {
        return None;
    }

    Some(ParsedTransaction {
        signature: tx.tx_signature.clone(),
        wallet: wallet.to_string(),
        protocol,
        tx_type: TransactionType::Swap,
        token_in,
        token_out,
        amount_in,
        amount_out,
        usd_value: Decimal::ZERO, // Will be computed later with price data
        block_time: tx.block_time * 1000, // Convert to milliseconds
        slot: tx.slot,
    })
}

fn parse_lending_operation(
    tx: &LysTransaction,
    wallet: &str,
    protocol: Protocol,
    tx_type: TransactionType,
) -> Option<ParsedTransaction> {
    // Use ui_amount if available, otherwise parse amount string
    let amount = if tx.ui_amount != 0.0 {
        tx.ui_amount
    } else {
        tx.amount.parse::<f64>().unwrap_or(0.0)
    };

    Some(ParsedTransaction {
        signature: tx.tx_signature.clone(),
        wallet: wallet.to_string(),
        protocol,
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
