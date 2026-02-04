use thiserror::Error;

#[derive(Error, Debug)]
pub enum PredictionError {
    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Invalid interval: {0}. Valid intervals: 1m, 5m, 15m, 1h, 4h, 24h")]
    InvalidInterval(String),

    #[error("Invalid slug format: {0}")]
    InvalidSlug(String),

    #[error("Database error: {0}")]
    Database(#[from] clickhouse::error::Error),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PredictionError>;

/// Validate a market slug format
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(PredictionError::InvalidSlug("Slug cannot be empty".into()));
    }

    // Slugs should be lowercase alphanumeric with hyphens
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(PredictionError::InvalidSlug(format!(
            "Slug must contain only lowercase letters, numbers, and hyphens: {}",
            slug
        )));
    }

    Ok(())
}

/// Validate interval format
pub fn validate_interval(interval: &str) -> Result<()> {
    match interval {
        "1m" | "5m" | "15m" | "30m" | "1h" | "4h" | "24h" | "1d" | "7d" => Ok(()),
        _ => Err(PredictionError::InvalidInterval(interval.to_string())),
    }
}
