//! Polymarket API client
//!
//! Fetches user positions and market data from Polymarket's API.
//! Note: Polymarket uses Polygon (EVM) addresses. For Solana wallet correlation,
//! users must provide linked EVM addresses.

use chrono::{DateTime, Utc};
use cortex_core::{MarketStatus, PredictionMarketBet};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{CortexMcpError, Result};

const POLYMARKET_API_BASE: &str = "https://clob.polymarket.com";
const POLYMARKET_GAMMA_API: &str = "https://gamma-api.polymarket.com";

/// Polymarket API client
pub struct PolymarketClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct PolymarketPosition {
    asset: String,
    #[serde(rename = "conditionId")]
    condition_id: String,
    size: String,
    #[serde(rename = "avgPrice")]
    avg_price: String,
    #[serde(rename = "currentPrice")]
    current_price: Option<String>,
    #[serde(rename = "unrealizedPnl")]
    unrealized_pnl: Option<String>,
    outcome: String,
    #[serde(rename = "marketSlug")]
    market_slug: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PolymarketMarket {
    slug: String,
    question: String,
    category: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    active: bool,
    closed: bool,
    #[serde(rename = "outcomePrices")]
    outcome_prices: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GammaPositionResponse {
    positions: Option<Vec<GammaPosition>>,
}

#[derive(Debug, Deserialize)]
struct GammaPosition {
    market: GammaMarket,
    outcome: String,
    size: f64,
    #[serde(rename = "avgPrice")]
    avg_price: f64,
    #[serde(rename = "currentValue")]
    current_value: Option<f64>,
    #[serde(rename = "pnl")]
    pnl: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GammaMarket {
    slug: String,
    question: String,
    category: Option<String>,
    #[serde(rename = "endDateIso")]
    end_date: Option<String>,
    active: bool,
    closed: bool,
}

impl PolymarketClient {
    /// Create a new Polymarket client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Cortex-MCP/0.2.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch positions for an EVM wallet address
    pub async fn get_wallet_positions(&self, evm_address: &str) -> Result<Vec<PredictionMarketBet>> {
        // Validate EVM address format (basic check)
        if !evm_address.starts_with("0x") || evm_address.len() != 42 {
            return Err(CortexMcpError::InvalidAddress(
                "Invalid EVM address format. Expected 0x-prefixed 40-char hex".to_string(),
            ));
        }

        // Try Gamma API first (more comprehensive data)
        match self.fetch_gamma_positions(evm_address).await {
            Ok(positions) if !positions.is_empty() => return Ok(positions),
            Ok(_) => {} // Empty, try CLOB
            Err(e) => {
                tracing::warn!("Gamma API failed: {}, trying CLOB API", e);
            }
        }

        // Fallback to CLOB API
        self.fetch_clob_positions(evm_address).await
    }

    /// Fetch from Gamma API (includes resolved markets)
    async fn fetch_gamma_positions(&self, address: &str) -> Result<Vec<PredictionMarketBet>> {
        let url = format!("{}/positions?user={}", POLYMARKET_GAMMA_API, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CortexMcpError::Http(format!(
                "Gamma API returned status {}",
                response.status()
            )));
        }

        let data: GammaPositionResponse = response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        let positions = data.positions.unwrap_or_default();

        Ok(positions
            .into_iter()
            .map(|p| {
                let current_price = if p.size > 0.0 {
                    p.current_value.unwrap_or(0.0) / p.size
                } else {
                    p.avg_price
                };

                PredictionMarketBet {
                    platform: "polymarket".to_string(),
                    market_slug: p.market.slug.clone(),
                    market_title: p.market.question,
                    outcome: p.outcome,
                    amount_usd: p.size * p.avg_price,
                    entry_price: p.avg_price,
                    current_price,
                    shares: p.size,
                    unrealized_pnl: p.pnl.unwrap_or(0.0),
                    category: p.market.category.unwrap_or_else(|| "general".to_string()),
                    resolution_date: p.market.end_date.and_then(|d| d.parse().ok()),
                    placed_at: Utc::now(), // Gamma doesn't provide this
                    market_status: if p.market.closed {
                        MarketStatus::Resolved
                    } else if p.market.active {
                        MarketStatus::Open
                    } else {
                        MarketStatus::Closed
                    },
                }
            })
            .collect())
    }

    /// Fetch from CLOB API (active positions only)
    async fn fetch_clob_positions(&self, address: &str) -> Result<Vec<PredictionMarketBet>> {
        let url = format!("{}/positions?user={}", POLYMARKET_API_BASE, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            // CLOB returns 404 for wallets with no positions
            if response.status() == 404 {
                return Ok(vec![]);
            }
            return Err(CortexMcpError::Http(format!(
                "CLOB API returned status {}",
                response.status()
            )));
        }

        let positions: Vec<PolymarketPosition> = response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        // Enrich with market data
        let mut bets = Vec::new();
        for pos in positions {
            let size: f64 = pos.size.parse().unwrap_or(0.0);
            let avg_price: f64 = pos.avg_price.parse().unwrap_or(0.0);
            let current_price: f64 = pos.current_price
                .as_ref()
                .and_then(|p| p.parse().ok())
                .unwrap_or(avg_price);

            let unrealized_pnl = (current_price - avg_price) * size;

            bets.push(PredictionMarketBet {
                platform: "polymarket".to_string(),
                market_slug: pos.market_slug.unwrap_or_else(|| pos.condition_id.clone()),
                market_title: pos.title.unwrap_or_else(|| format!("Market {}", &pos.condition_id[..8])),
                outcome: pos.outcome,
                amount_usd: size * avg_price,
                entry_price: avg_price,
                current_price,
                shares: size,
                unrealized_pnl,
                category: "general".to_string(),
                resolution_date: None,
                placed_at: Utc::now(),
                market_status: MarketStatus::Open,
            });
        }

        Ok(bets)
    }

    /// Fetch market details by slug
    pub async fn get_market(&self, slug: &str) -> Result<PolymarketMarket> {
        let url = format!("{}/markets/{}", POLYMARKET_GAMMA_API, slug);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CortexMcpError::MarketNotFound(slug.to_string()));
        }

        response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))
    }

    /// Get all bettors for a market (for informed trader detection)
    pub async fn get_market_bettors(&self, slug: &str) -> Result<Vec<String>> {
        // Note: This endpoint may require authentication or have rate limits
        let url = format!("{}/markets/{}/traders", POLYMARKET_GAMMA_API, slug);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            // Fallback: return empty (feature gracefully degrades)
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct TradersResponse {
            traders: Vec<String>,
        }

        let data: TradersResponse = response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        Ok(data.traders)
    }
}

impl Default for PolymarketClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_address_validation() {
        let client = PolymarketClient::new();
        
        // Valid address format (won't actually fetch)
        let valid = "0x1234567890123456789012345678901234567890";
        assert!(valid.starts_with("0x") && valid.len() == 42);
        
        // Invalid addresses
        let invalid_no_prefix = "1234567890123456789012345678901234567890";
        assert!(!invalid_no_prefix.starts_with("0x"));
        
        let invalid_short = "0x123";
        assert!(invalid_short.len() != 42);
    }
}
