//! Configuration management for the unified MCP server

use serde::Deserialize;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub defi: DefiConfig,
    #[serde(default)]
    pub prediction: PredictionConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            defi: DefiConfig::default(),
            prediction: PredictionConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

/// DeFi API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DefiConfig {
    /// Base URL for the Cortex DeFi API
    #[serde(default = "default_defi_api_url")]
    pub api_url: String,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

impl Default for DefiConfig {
    fn default() -> Self {
        Self {
            api_url: default_defi_api_url(),
            timeout_seconds: default_timeout(),
        }
    }
}

fn default_defi_api_url() -> String {
    std::env::var("CORTEX_API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

fn default_timeout() -> u64 {
    30
}

/// Prediction market configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PredictionConfig {
    /// Enable prediction market features
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Clickhouse URL
    #[serde(default = "default_clickhouse_url")]
    pub clickhouse_url: String,
    /// Clickhouse database name
    #[serde(default = "default_database")]
    pub database: String,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            clickhouse_url: default_clickhouse_url(),
            database: default_database(),
        }
    }
}

fn default_enabled() -> bool {
    std::env::var("CORTEX_PREDICTION_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true)
}

fn default_clickhouse_url() -> String {
    std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".to_string())
}

fn default_database() -> String {
    std::env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "cortex".to_string())
}

/// Cache configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache entries
    #[serde(default = "default_max_capacity")]
    pub max_capacity: u64,
    /// Cache TTL in seconds
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: default_max_capacity(),
            ttl_seconds: default_ttl(),
        }
    }
}

fn default_max_capacity() -> u64 {
    1000
}

fn default_ttl() -> u64 {
    300 // 5 minutes
}

impl AppConfig {
    /// Load configuration from file and environment
    pub async fn load() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            // Start with defaults
            .set_default("defi.api_url", default_defi_api_url())?
            .set_default("defi.timeout_seconds", default_timeout() as i64)?
            .set_default("prediction.enabled", default_enabled())?
            .set_default("prediction.clickhouse_url", default_clickhouse_url())?
            .set_default("prediction.database", default_database())?
            .set_default("cache.max_capacity", default_max_capacity() as i64)?
            .set_default("cache.ttl_seconds", default_ttl() as i64)?
            // Load from file if present
            .add_source(config::File::with_name("cortex-mcp").required(false))
            // Override with environment variables (CORTEX_ prefix)
            .add_source(
                config::Environment::with_prefix("CORTEX")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()?;

        Ok(settings.try_deserialize()?)
    }
}
