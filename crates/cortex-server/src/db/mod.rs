pub mod models;
pub mod queries;

use clickhouse::Client;

use crate::config::DatabaseConfig;

#[derive(Clone)]
pub struct Database {
    client: Client,
}

impl Database {
    pub fn new(config: &DatabaseConfig) -> Self {
        let mut client = Client::default()
            .with_url(&config.url)
            .with_database(&config.database);

        // Add user/password if provided
        if let Some(ref user) = config.user {
            client = client.with_user(user);
        }
        if let Some(ref password) = config.password {
            client = client.with_password(password);
        }

        println!("[DB] Connecting to {} database '{}'", config.url, config.database);

        Self { client }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub async fn health_check(&self) -> Result<(), clickhouse::error::Error> {
        self.client.query("SELECT 1").execute().await?;
        Ok(())
    }
}
