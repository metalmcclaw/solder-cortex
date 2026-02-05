//! Error types for Cortex Core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CortexError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Insufficient data for conviction calculation: {0}")]
    InsufficientData(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type CortexResult<T> = Result<T, CortexError>;
