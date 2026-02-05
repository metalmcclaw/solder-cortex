//! DeFi API client for wallet analytics
//!
//! Connects to the Cortex DeFi API (cortex-server) for wallet data.

use chrono::Utc;
use cortex_core::{
    calculate_conviction, conviction_to_response, ConvictionConfidence, DeFiPosition,
    MarketStatus, PositionType, PredictionMarketBet, Wallet, WalletClassification,
    WalletConvictionResponse,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::DefiConfig;
use crate::error::{CortexMcpError, Result};

/// HTTP client for the Cortex DeFi API
pub struct DefiClient {
    client: Client,
    api_url: String,
}

impl DefiClient {
    /// Create a new DeFi client
    pub fn new(config: &DefiConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_url: config.api_url.clone(),
        }
    }

    /// Health check
    pub async fn health(&self) -> Result<Value> {
        let url = format!("{}/health", self.api_url);
        self.get(&url).await
    }

    /// Get wallet summary
    pub async fn get_wallet_summary(&self, wallet: &str) -> Result<Value> {
        let url = format!("{}/api/v1/user/{}/summary", self.api_url, wallet);
        self.get(&url).await
    }

    /// Get wallet PnL
    pub async fn get_wallet_pnl(&self, wallet: &str, window: &str) -> Result<Value> {
        let url = format!("{}/api/v1/user/{}/pnl?window={}", self.api_url, wallet, window);
        self.get(&url).await
    }

    /// Get wallet positions
    pub async fn get_wallet_positions(&self, wallet: &str) -> Result<Value> {
        let url = format!("{}/api/v1/user/{}/positions", self.api_url, wallet);
        self.get(&url).await
    }

    /// Start indexing a wallet
    pub async fn start_indexing(&self, wallet: &str) -> Result<Value> {
        let url = format!("{}/api/v1/index", self.api_url);
        self.post(&url, json!({ "wallet": wallet })).await
    }

    /// Stop indexing a wallet
    pub async fn stop_indexing(&self, wallet: &str) -> Result<Value> {
        let url = format!("{}/api/v1/index/{}", self.api_url, wallet);
        self.delete(&url).await
    }

    /// List wallet subscriptions
    pub async fn list_subscriptions(&self) -> Result<Value> {
        let url = format!("{}/api/v1/index", self.api_url);
        self.get(&url).await
    }

    /// Calculate wallet conviction
    pub async fn get_wallet_conviction(&self, wallet_addr: &str) -> Result<WalletConvictionResponse> {
        // Fetch DeFi data
        let summary = self.get_wallet_summary(wallet_addr).await.unwrap_or(json!({}));
        let positions_data = self.get_wallet_positions(wallet_addr).await.unwrap_or(json!({"positions": []}));

        // Parse DeFi positions
        let defi_positions = parse_defi_positions(&positions_data);

        // Fetch prediction bets (placeholder for now)
        let prediction_bets = fetch_prediction_bets(wallet_addr);

        // Build wallet entity
        let wallet = Wallet {
            address: wallet_addr.to_string(),
            total_value_usd: summary["total_value_usd"].as_f64().unwrap_or(0.0),
            defi_positions,
            prediction_bets,
            classification: Some(WalletClassification::Unknown),
            risk_score: summary["risk_score"].as_u64().unwrap_or(50) as u8,
            last_activity: Utc::now(),
            protocols: summary["protocols"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        };

        // Calculate conviction
        match calculate_conviction(&wallet) {
            Ok(conviction) => Ok(conviction_to_response(&conviction, &wallet)),
            Err(e) => {
                // Return meaningful response even with limited data
                Ok(WalletConvictionResponse {
                    wallet: wallet_addr.to_string(),
                    conviction_score: 0.0,
                    confidence: "low".to_string(),
                    signals_count: 0,
                    signals: vec![],
                    interpretation: format!(
                        "Unable to calculate conviction: {}. This wallet may not have correlated DeFi and prediction market activity.",
                        e
                    ),
                    defi_summary: cortex_core::DeFiSummary {
                        total_value_usd: wallet.total_value_usd,
                        position_count: wallet.defi_positions.len(),
                        protocols: wallet.protocols,
                        dominant_exposure: "N/A".to_string(),
                    },
                    prediction_summary: cortex_core::PredictionSummary {
                        total_bet_usd: 0.0,
                        bet_count: 0,
                        platforms: vec![],
                        categories: vec![],
                    },
                })
            }
        }
    }

    /// Detect informed traders (placeholder implementation)
    pub async fn detect_informed_traders(
        &self,
        market_slug: &str,
        platform: &str,
        min_conviction: f64,
    ) -> Result<Value> {
        // TODO: Full implementation requires prediction market bettor data
        Ok(json!({
            "market_slug": market_slug,
            "platform": platform,
            "informed_traders_count": 0,
            "aggregate_signal": {
                "direction": "insufficient_data",
                "alignment_pct": 0.0,
                "total_informed_usd": 0.0,
                "confidence": "low"
            },
            "traders": [],
            "note": format!(
                "Informed trader detection for '{}' on {} requires prediction market data integration. \
                This feature correlates bettors' on-chain DeFi activity with their market positions. \
                Minimum conviction threshold: {:.2}",
                market_slug, platform, min_conviction
            )
        }))
    }

    // HTTP helper methods

    async fn get(&self, url: &str) -> Result<Value> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))
    }

    async fn post(&self, url: &str, body: Value) -> Result<Value> {
        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))
    }

    async fn delete(&self, url: &str) -> Result<Value> {
        let response = self
            .client
            .delete(url)
            .send()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| CortexMcpError::Http(e.to_string()))
    }
}

/// Parse DeFi positions from API response
fn parse_defi_positions(data: &Value) -> Vec<DeFiPosition> {
    let positions = match data["positions"].as_array() {
        Some(arr) => arr,
        None => return vec![],
    };

    positions
        .iter()
        .filter_map(|p| {
            Some(DeFiPosition {
                protocol: p["protocol"].as_str()?.to_string(),
                position_type: match p["position_type"].as_str()? {
                    "lending" | "supply" => PositionType::Lending,
                    "borrowing" | "borrow" => PositionType::Borrowing,
                    "lp" | "liquidity" => PositionType::LiquidityPool,
                    "staking" | "stake" => PositionType::Staking,
                    "swap" => PositionType::Swap,
                    _ => PositionType::Other,
                },
                token: p["token"].as_str().unwrap_or("").to_string(),
                token_symbol: p["token_symbol"]
                    .as_str()
                    .or_else(|| p["token"].as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                amount: p["amount"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| p["amount"].as_f64())
                    .unwrap_or(0.0),
                usd_value: p["usd_value"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| p["usd_value"].as_f64())
                    .unwrap_or(0.0),
                entry_price: p["entry_price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| p["entry_price"].as_f64()),
                current_price: p["current_price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| p["current_price"].as_f64())
                    .unwrap_or(0.0),
                unrealized_pnl: p["unrealized_pnl"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| p["unrealized_pnl"].as_f64())
                    .unwrap_or(0.0),
                opened_at: Utc::now(),
                updated_at: Utc::now(),
                metadata: None,
            })
        })
        .collect()
}

/// Fetch prediction market bets for a wallet (placeholder)
fn fetch_prediction_bets(wallet_addr: &str) -> Vec<PredictionMarketBet> {
    // Demo mode returns sample data
    if std::env::var("CORTEX_DEMO_MODE").is_ok() {
        return vec![PredictionMarketBet {
            platform: "polymarket".to_string(),
            market_slug: "eth-above-5000-march-2026".to_string(),
            market_title: "Will ETH be above $5,000 by March 2026?".to_string(),
            outcome: "YES".to_string(),
            amount_usd: 5000.0,
            entry_price: 0.67,
            current_price: 0.72,
            shares: 7462.69,
            unrealized_pnl: 373.13,
            category: "crypto".to_string(),
            resolution_date: None,
            placed_at: Utc::now(),
            market_status: MarketStatus::Open,
        }];
    }

    // TODO: Query actual prediction market data
    let _ = wallet_addr;
    vec![]
}
