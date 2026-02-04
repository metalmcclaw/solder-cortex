pub mod models;
pub mod queries;

use clickhouse::Client;

use crate::config::DatabaseConfig;
pub use queries::QueryEngine;

/// Initialize Clickhouse client with configuration
/// The clickhouse crate handles connection pooling internally
pub fn create_client(config: &DatabaseConfig) -> Client {
    let mut client = Client::default()
        .with_url(&config.url)
        .with_database(&config.database);

    if let Some(ref user) = config.user {
        client = client.with_user(user);
    }
    if let Some(ref password) = config.password {
        client = client.with_password(password);
    }

    tracing::info!(
        url = %config.url,
        database = %config.database,
        "Connecting to Clickhouse"
    );

    client
}

/// Create a QueryEngine with the configured client
pub fn create_query_engine(config: &DatabaseConfig) -> QueryEngine {
    let client = create_client(config);
    QueryEngine::new(client)
}
