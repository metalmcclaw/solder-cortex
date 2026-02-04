use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub database: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached entries
    pub max_capacity: u64,
    /// TTL for cached entries in seconds
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl_seconds: 60,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Database defaults
            .set_default("database.url", "http://localhost:8123")?
            .set_default("database.database", "cortex")?
            // Cache defaults
            .set_default("cache.max_capacity", 1000)?
            .set_default("cache.ttl_seconds", 60)?
            // Load from config files if they exist
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            // Override with environment variables
            // CORTEX_PREDICTION__DATABASE__URL, etc.
            .add_source(
                Environment::with_prefix("CORTEX_PREDICTION")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }
}
