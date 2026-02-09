//! Kalshi API client
//!
//! Fetches user positions and market data from Kalshi's API.
//! Kalshi is a regulated US exchange (CFTC).

use chrono::{DateTime, Utc};
use cortex_core::{MarketStatus, PredictionMarketBet};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{CortexMcpError, Result};

const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";

/// Kalshi API client
pub struct KalshiClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct KalshiMarket {
    ticker: String,
    title: String,
    subtitle: Option<String>,
    category: String,
    #[serde(rename = "expiration_time")]
    expiration_time: String,
    status: String,
    yes_bid: i64,
    yes_ask: i64,
}

#[derive(Debug, Deserialize)]
struct KalshiPosition {
    ticker: String,
    #[serde(rename = "market_ticker")]
    market_ticker: String,
    side: String, // "yes" or "no"
    count: i32,
    #[serde(rename = "avg_price")]
    avg_price: i32, // In cents
    #[serde(rename = "realized_pnl")]
    realized_pnl: i32,
}

#[derive(Debug, Deserialize)]
struct KalshiPortfolioResponse {
    positions: Vec<KalshiPosition>,
}

impl KalshiClient {
    /// Create a new Kalshi client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Cortex-MCP/0.2.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch positions for a Kalshi user (by wallet/ID - tricky since Kalshi is centralized)
    /// For hackathon purposes, we might simulate or allow passing a Kalshi API key/ID
    /// Currently, we'll assume we can fetch public portfolio if enabled, or this is a placeholder
    /// for authenticated integration.
    pub async fn get_user_positions(&self, user_id: &str) -> Result<Vec<PredictionMarketBet>> {
        // NOTE: Kalshi requires authentication (Bearer token) for portfolio.
        // We'll assume the user_id passed here is a placeholder for now,
        // or we'd need to inject API credentials.
        // For this implementation, we'll return an empty list or mock data if demo mode.
        
        if std::env::var("CORTEX_DEMO_MODE").is_ok() {
            return Ok(self.mock_positions());
        }

        // Real implementation would require auth headers
        // let url = format!("{}/portfolio/positions", KALSHI_API_BASE);
        // ... auth logic ...
        
        Ok(vec![])
    }

    /// Fetch market details
    pub async fn get_market(&self, ticker: &str) -> Result<KalshiMarket> {
        let url = format!("{}/markets/{}", KALSHI_API_BASE, ticker);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CortexMcpError::MarketNotFound(ticker.to_string()));
        }

        #[derive(Deserialize)]
        struct MarketResponse {
            market: KalshiMarket,
        }

        let data: MarketResponse = response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        Ok(data.market)
    }

    /// Get all bettors for a market (for informed trader detection)
    pub async fn get_market_bettors(&self, ticker: &str) -> Result<Vec<String>> {
        // Kalshi doesn't expose a public list of bettors per market via API.
        // This would typically require proprietary data or a different endpoint.
        // For demonstration, we return an empty list or mock data.
        
        if std::env::var("CORTEX_DEMO_MODE").is_ok() {
            // Mock bettors (some random hashes/IDs)
            return Ok(vec![
                "kalshi_user_12345".to_string(),
                "kalshi_user_67890".to_string()
            ]);
        }

        Ok(vec![])
    }

    fn mock_positions(&self) -> Vec<PredictionMarketBet> {
        vec![
            PredictionMarketBet {
                platform: "kalshi".to_string(),
                market_slug: "KX-FED-RATE-DEC24".to_string(),
                market_title: "Fed Interest Rate Decision Dec 2024".to_string(),
                outcome: "Unchanged".to_string(),
                amount_usd: 1500.0,
                entry_price: 0.45,
                current_price: 0.52,
                shares: 3333.33,
                unrealized_pnl: 233.33,
                category: "economics".to_string(),
                resolution_date: None,
                placed_at: Utc::now(),
                market_status: MarketStatus::Open,
            }
        ]
    }
}

impl Default for KalshiClient {
    fn default() -> Self {
        Self::new()
    }
}
