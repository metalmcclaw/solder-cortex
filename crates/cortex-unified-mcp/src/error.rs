//! Error types for the unified MCP server

use thiserror::Error;

/// Unified error type for the MCP server
#[derive(Error, Debug)]
pub enum CortexMcpError {
    #[error("DeFi API error: {0}")]
    DefiApi(String),

    #[error("Prediction engine error: {0}")]
    Prediction(String),

    #[error("Prediction features not available")]
    PredictionNotAvailable,

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, CortexMcpError>;

/// Validate a market slug
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(CortexMcpError::InvalidParameter("Market slug cannot be empty".into()));
    }
    if slug.len() > 256 {
        return Err(CortexMcpError::InvalidParameter("Market slug too long".into()));
    }
    Ok(())
}

/// Validate a time interval
pub fn validate_interval(interval: &str) -> Result<()> {
    let valid = ["1m", "5m", "15m", "30m", "1h", "4h", "24h", "7d"];
    if !valid.contains(&interval) {
        return Err(CortexMcpError::InvalidParameter(format!(
            "Invalid interval '{}'. Valid: {:?}",
            interval, valid
        )));
    }
    Ok(())
}

/// Validate a Solana wallet address
pub fn validate_wallet(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(CortexMcpError::InvalidParameter("Wallet address cannot be empty".into()));
    }
    // Basic base58 check (32-44 characters)
    if address.len() < 32 || address.len() > 44 {
        return Err(CortexMcpError::InvalidParameter(
            "Invalid Solana wallet address length".into(),
        ));
    }
    Ok(())
}
