use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub lyslabs: LysLabsConfig,
    pub helius: HeliusConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
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
pub struct LysLabsConfig {
    pub api_key: String,
    pub ws_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HeliusConfig {
    pub api_key: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("database.url", "http://localhost:8123")?
            .set_default("database.database", "cortex")?
            .set_default("lyslabs.api_key", "")?
            .set_default("lyslabs.ws_url", "wss://solana-mainnet-api-vip.lyslabs.ai/v1/")?
            .set_default("helius.api_key", "")?
            // Load from config file if it exists
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            // Override with environment variables (CORTEX__SERVER__HOST, etc.)
            // Using double underscore as separator to handle nested keys with underscores
            .add_source(
                Environment::with_prefix("CORTEX")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
