//! DeFi API client for wallet analytics
//!
//! Connects to the Cortex DeFi API (cortex-server) for wallet data.
//! Integrates with Polymarket for cross-domain conviction analysis.

use chrono::Utc;
use cortex_core::{
    calculate_conviction, conviction_to_response, ConvictionConfidence, DeFiPosition,
    MarketStatus, PositionType, PredictionMarketBet, Wallet, WalletClassification,
    WalletConvictionResponse,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::config::DefiConfig;
use crate::error::{detect_address_type, AddressType, CortexMcpError, Result};
use crate::polymarket::PolymarketClient;

/// HTTP client for the Cortex DeFi API
pub struct DefiClient {
    client: Client,
    api_url: String,
    polymarket: Arc<PolymarketClient>,
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
            polymarket: Arc::new(PolymarketClient::new()),
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
    ///
    /// Supports both Solana and EVM addresses:
    /// - Solana: Fetches DeFi data from Cortex API
    /// - EVM: Fetches prediction market positions from Polymarket
    ///
    /// For cross-domain analysis, provide `evm_address` parameter to correlate
    /// a Solana wallet's DeFi activity with Polymarket positions.
    pub async fn get_wallet_conviction(&self, wallet_addr: &str) -> Result<WalletConvictionResponse> {
        self.get_wallet_conviction_with_evm(wallet_addr, None).await
    }

    /// Calculate wallet conviction with optional linked EVM address
    pub async fn get_wallet_conviction_with_evm(
        &self,
        wallet_addr: &str,
        evm_address: Option<&str>,
    ) -> Result<WalletConvictionResponse> {
        let addr_type = detect_address_type(wallet_addr);

        // Fetch DeFi data (for Solana addresses)
        let (summary, defi_positions) = if addr_type == AddressType::Solana {
            let summary = self.get_wallet_summary(wallet_addr).await.unwrap_or(json!({}));
            let positions_data = self.get_wallet_positions(wallet_addr).await.unwrap_or(json!({"positions": []}));
            (summary, parse_defi_positions(&positions_data))
        } else {
            // EVM address - no DeFi data from Cortex (Solana-focused)
            (json!({}), vec![])
        };

        // Fetch prediction bets
        let prediction_bets = self.fetch_prediction_bets_cross_chain(wallet_addr, evm_address).await;

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

    /// Fetch prediction bets with cross-chain support
    async fn fetch_prediction_bets_cross_chain(
        &self,
        wallet_addr: &str,
        evm_address: Option<&str>,
    ) -> Vec<PredictionMarketBet> {
        let mut bets = Vec::new();
        let addr_type = detect_address_type(wallet_addr);

        // If wallet is EVM, fetch directly from Polymarket
        if addr_type == AddressType::Evm {
            match self.polymarket.get_wallet_positions(wallet_addr).await {
                Ok(positions) => {
                    tracing::info!(
                        wallet = %wallet_addr,
                        positions = positions.len(),
                        "Fetched Polymarket positions"
                    );
                    bets.extend(positions);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to fetch Polymarket positions");
                }
            }
        }

        // If linked EVM address provided, also fetch from Polymarket
        if let Some(evm) = evm_address {
            match self.polymarket.get_wallet_positions(evm).await {
                Ok(positions) => {
                    tracing::info!(
                        evm_address = %evm,
                        positions = positions.len(),
                        "Fetched linked Polymarket positions"
                    );
                    bets.extend(positions);
                }
                Err(e) => {
                    tracing::warn!(error = %e, evm = %evm, "Failed to fetch linked Polymarket positions");
                }
            }
        }

        // Demo mode fallback
        if bets.is_empty() && std::env::var("CORTEX_DEMO_MODE").is_ok() {
            bets = demo_prediction_bets();
        }

        bets
    }

    /// Detect informed traders by correlating market bettors with on-chain activity
    pub async fn detect_informed_traders(
        &self,
        market_slug: &str,
        platform: &str,
        min_conviction: f64,
    ) -> Result<Value> {
        // Get bettors for this market
        let bettors = match self.polymarket.get_market_bettors(market_slug).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, market = %market_slug, "Failed to fetch market bettors");
                return Ok(json!({
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
                        "Could not fetch bettors for market '{}': {}. Ensure the market slug is correct.",
                        market_slug, e
                    )
                }));
            }
        };

        if bettors.is_empty() {
            return Ok(json!({
                "market_slug": market_slug,
                "platform": platform,
                "informed_traders_count": 0,
                "aggregate_signal": {
                    "direction": "no_bettors",
                    "alignment_pct": 0.0,
                    "total_informed_usd": 0.0,
                    "confidence": "low"
                },
                "traders": [],
                "note": "No bettors found for this market"
            }));
        }

        // Analyze each bettor for conviction signals
        let mut informed_traders = Vec::new();
        let mut total_bullish_usd = 0.0;
        let mut total_bearish_usd = 0.0;

        for bettor_addr in bettors.iter().take(50) {
            // Limit to 50 to avoid rate limits
            let conviction = self.get_wallet_conviction(bettor_addr).await;
            
            if let Ok(c) = conviction {
                if c.conviction_score >= min_conviction {
                    // Determine direction from signals
                    let is_bullish = c.signals.iter()
                        .any(|s| s.signal_type.contains("bullish"));
                    
                    if is_bullish {
                        total_bullish_usd += c.defi_summary.total_value_usd;
                    } else {
                        total_bearish_usd += c.defi_summary.total_value_usd;
                    }

                    informed_traders.push(json!({
                        "address": bettor_addr,
                        "conviction_score": c.conviction_score,
                        "confidence": c.confidence,
                        "defi_exposure_usd": c.defi_summary.total_value_usd,
                        "bet_exposure_usd": c.prediction_summary.total_bet_usd,
                        "direction": if is_bullish { "bullish" } else { "bearish" },
                        "interpretation": c.interpretation
                    }));
                }
            }
        }

        // Calculate aggregate signal
        let total_informed_usd = total_bullish_usd + total_bearish_usd;
        let (direction, alignment_pct) = if total_informed_usd > 0.0 {
            if total_bullish_usd > total_bearish_usd {
                ("bullish".to_string(), total_bullish_usd / total_informed_usd * 100.0)
            } else {
                ("bearish".to_string(), total_bearish_usd / total_informed_usd * 100.0)
            }
        } else {
            ("neutral".to_string(), 0.0)
        };

        let confidence = if informed_traders.len() >= 5 {
            "high"
        } else if informed_traders.len() >= 2 {
            "medium"
        } else {
            "low"
        };

        Ok(json!({
            "market_slug": market_slug,
            "platform": platform,
            "bettors_analyzed": bettors.len().min(50),
            "informed_traders_count": informed_traders.len(),
            "aggregate_signal": {
                "direction": direction,
                "alignment_pct": alignment_pct,
                "total_informed_usd": total_informed_usd,
                "confidence": confidence
            },
            "traders": informed_traders,
            "min_conviction_threshold": min_conviction
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

/// Demo prediction bets for testing
fn demo_prediction_bets() -> Vec<PredictionMarketBet> {
    vec![
        PredictionMarketBet {
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
        },
        PredictionMarketBet {
            platform: "polymarket".to_string(),
            market_slug: "sol-above-200-feb-2026".to_string(),
            market_title: "Will Solana be above $200 by end of February 2026?".to_string(),
            outcome: "YES".to_string(),
            amount_usd: 3000.0,
            entry_price: 0.55,
            current_price: 0.62,
            shares: 5454.55,
            unrealized_pnl: 381.82,
            category: "crypto".to_string(),
            resolution_date: None,
            placed_at: Utc::now(),
            market_status: MarketStatus::Open,
        },
    ]
}
