use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::error::AppResult;
use crate::db::Database;
use serde::{Deserialize, Serialize};

/// Market-wide data aggregator for DeFi and Prediction Markets
#[derive(Clone)]
pub struct MarketDataIndexer {
    db: Database,
    polymarket: PolymarketClient,
    kalshi: KalshiClient,
    defi_protocols: DeFiProtocolsClient,
    /// Cached market data
    market_cache: Arc<RwLock<MarketDataCache>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketDataCache {
    pub defi_tvl: HashMap<String, f64>,  // protocol -> TVL
    pub defi_volume_24h: HashMap<String, f64>,  // protocol -> 24h volume
    pub prediction_markets: Vec<PredictionMarketData>,
    pub top_tokens: Vec<TokenMetrics>,
    pub market_sentiment: MarketSentiment,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredictionMarketData {
    pub platform: String,  // "polymarket", "kalshi"
    pub market_id: String,
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub volume_24h: f64,
    pub total_volume: f64,
    pub end_date: chrono::DateTime<chrono::Utc>,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub symbol: String,
    pub mint: String,
    pub price_usd: f64,
    pub volume_24h: f64,
    pub market_cap: f64,
    pub price_change_24h: f64,
    pub defi_usage: DeFiUsageStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeFiUsageStats {
    pub total_tvl: f64,
    pub protocols: HashMap<String, f64>,  // protocol -> TVL for this token
    pub swap_volume_24h: f64,
    pub lending_volume_24h: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketSentiment {
    pub overall_score: f64,  // -1 to 1
    pub defi_sentiment: f64,
    pub prediction_market_sentiment: f64,
    pub trending_topics: Vec<String>,
    pub fear_greed_index: f64,  // 0-100
}

impl MarketDataIndexer {
    pub fn new(db: Database) -> Self {
        tracing::info!("Initializing MarketDataIndexer for market-wide data aggregation");
        
        Self {
            db,
            polymarket: PolymarketClient::new(),
            kalshi: KalshiClient::new(),
            defi_protocols: DeFiProtocolsClient::new(),
            market_cache: Arc::new(RwLock::new(MarketDataCache {
                defi_tvl: HashMap::new(),
                defi_volume_24h: HashMap::new(),
                prediction_markets: Vec::new(),
                top_tokens: Vec::new(),
                market_sentiment: MarketSentiment {
                    overall_score: 0.0,
                    defi_sentiment: 0.0,
                    prediction_market_sentiment: 0.0,
                    trending_topics: Vec::new(),
                    fear_greed_index: 50.0,
                },
                last_updated: chrono::Utc::now(),
            })),
        }
    }

    /// Start continuous market data aggregation
    pub async fn start_market_data_aggregation(&self) -> AppResult<()> {
        tracing::info!("Starting continuous market data aggregation");
        println!("[MARKET] Starting market-wide data aggregation...");

        // === RUN INITIAL AGGREGATION IMMEDIATELY ===
        println!("[STARTUP] Running initial data aggregation cycle...");
        
        // Initial DeFi aggregation
        match self.aggregate_defi_data().await {
            Ok(_) => println!("[STARTUP] ✓ DeFi data loaded"),
            Err(e) => println!("[STARTUP] ✗ DeFi data failed: {}", e),
        }
        
        // Initial prediction markets aggregation  
        match self.aggregate_prediction_markets().await {
            Ok(_) => println!("[STARTUP] ✓ Prediction markets loaded"),
            Err(e) => println!("[STARTUP] ✗ Prediction markets failed: {}", e),
        }
        
        // Initial sentiment calculation
        match self.calculate_market_sentiment().await {
            Ok(_) => println!("[STARTUP] ✓ Market sentiment calculated"),
            Err(e) => println!("[STARTUP] ✗ Market sentiment failed: {}", e),
        }
        
        println!("[STARTUP] Initial data aggregation complete");

        // Start background tasks for continuous data source updates
        let indexer = self.clone();
        tokio::spawn(async move {
            indexer.defi_aggregation_loop().await;
        });

        let indexer = self.clone();
        tokio::spawn(async move {
            indexer.prediction_markets_aggregation_loop().await;
        });

        let indexer = self.clone();
        tokio::spawn(async move {
            indexer.market_sentiment_loop().await;
        });

        Ok(())
    }

    /// Aggregate DeFi data across all protocols
    async fn defi_aggregation_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;
            
            tracing::info!("Starting DeFi data aggregation cycle");
            let start = Instant::now();

            match self.aggregate_defi_data().await {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    tracing::info!(duration_ms = %duration, "DeFi data aggregation completed");
                    println!("[DEFI] Aggregation cycle completed ({}ms)", duration);
                }
                Err(e) => {
                    tracing::error!(error = %e, "DeFi data aggregation failed");
                    println!("[DEFI] Aggregation failed: {}", e);
                }
            }
        }
    }

    /// Aggregate prediction market data
    async fn prediction_markets_aggregation_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(180)); // 3 minutes

        loop {
            interval.tick().await;
            
            tracing::info!("Starting prediction markets data aggregation cycle");
            let start = Instant::now();

            match self.aggregate_prediction_markets().await {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    tracing::info!(duration_ms = %duration, "Prediction markets aggregation completed");
                    println!("[PREDICTION] Aggregation cycle completed ({}ms)", duration);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Prediction markets aggregation failed");
                    println!("[PREDICTION] Aggregation failed: {}", e);
                }
            }
        }
    }

    /// Calculate and update market sentiment
    async fn market_sentiment_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(600)); // 10 minutes

        loop {
            interval.tick().await;
            
            tracing::info!("Starting market sentiment calculation");
            let start = Instant::now();

            match self.calculate_market_sentiment().await {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    tracing::info!(duration_ms = %duration, "Market sentiment calculation completed");
                    println!("[SENTIMENT] Calculation completed ({}ms)", duration);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Market sentiment calculation failed");
                    println!("[SENTIMENT] Calculation failed: {}", e);
                }
            }
        }
    }

    async fn aggregate_defi_data(&self) -> AppResult<()> {
        println!("[DEFI] Aggregating data from Jupiter, Raydium, Kamino...");
        
        // Fetch data from major DeFi protocols
        let jupiter_data = self.defi_protocols.get_jupiter_metrics().await?;
        let raydium_data = self.defi_protocols.get_raydium_metrics().await?;
        let kamino_data = self.defi_protocols.get_kamino_metrics().await?;

        // Update cache
        {
            let mut cache = self.market_cache.write().await;
            cache.defi_tvl.insert("jupiter".to_string(), jupiter_data.tvl);
            cache.defi_tvl.insert("raydium".to_string(), raydium_data.tvl);
            cache.defi_tvl.insert("kamino".to_string(), kamino_data.tvl);
            
            cache.defi_volume_24h.insert("jupiter".to_string(), jupiter_data.volume_24h);
            cache.defi_volume_24h.insert("raydium".to_string(), raydium_data.volume_24h);
            cache.defi_volume_24h.insert("kamino".to_string(), kamino_data.volume_24h);
        }

        // Store in database
        self.store_defi_metrics(&jupiter_data, "jupiter").await?;
        self.store_defi_metrics(&raydium_data, "raydium").await?;
        self.store_defi_metrics(&kamino_data, "kamino").await?;

        tracing::info!("DeFi data aggregation completed successfully");
        Ok(())
    }

    async fn aggregate_prediction_markets(&self) -> AppResult<()> {
        println!("[PREDICTION] Aggregating data from Polymarket and Kalshi...");
        
        // Fetch from Polymarket
        let polymarket_markets = self.polymarket.get_trending_markets().await?;
        let kalshi_markets = self.kalshi.get_active_markets().await?;

        // Update cache
        {
            let mut cache = self.market_cache.write().await;
            cache.prediction_markets.clear();
            cache.prediction_markets.extend(polymarket_markets);
            cache.prediction_markets.extend(kalshi_markets);
        }

        tracing::info!(
            polymarket_count = %polymarket_markets.len(),
            kalshi_count = %kalshi_markets.len(),
            "Prediction markets data aggregated"
        );
        
        Ok(())
    }

    async fn calculate_market_sentiment(&self) -> AppResult<()> {
        let cache = self.market_cache.read().await;
        
        // Calculate sentiment from prediction markets
        let prediction_sentiment = self.calculate_prediction_sentiment(&cache.prediction_markets).await?;
        
        // Calculate DeFi sentiment from volume/TVL trends
        let defi_sentiment = self.calculate_defi_sentiment(&cache.defi_tvl, &cache.defi_volume_24h).await?;
        
        // Overall sentiment
        let overall_sentiment = (prediction_sentiment + defi_sentiment) / 2.0;
        
        drop(cache);

        // Update sentiment in cache
        {
            let mut cache = self.market_cache.write().await;
            cache.market_sentiment.overall_score = overall_sentiment;
            cache.market_sentiment.defi_sentiment = defi_sentiment;
            cache.market_sentiment.prediction_market_sentiment = prediction_sentiment;
            cache.last_updated = chrono::Utc::now();
        }

        tracing::info!(
            overall_sentiment = %overall_sentiment,
            defi_sentiment = %defi_sentiment,
            prediction_sentiment = %prediction_sentiment,
            "Market sentiment calculated"
        );

        Ok(())
    }

    async fn calculate_prediction_sentiment(&self, markets: &[PredictionMarketData]) -> AppResult<f64> {
        // Analyze prediction markets for overall sentiment
        // Higher "YES" prices on positive markets = bullish sentiment
        // This is a simplified sentiment calculation
        
        let mut total_sentiment = 0.0;
        let mut count = 0;

        for market in markets {
            if market.tags.contains(&"crypto".to_string()) || 
               market.tags.contains(&"defi".to_string()) ||
               market.question.to_lowercase().contains("bitcoin") ||
               market.question.to_lowercase().contains("ethereum") ||
               market.question.to_lowercase().contains("solana") {
                
                // Simple heuristic: if it's a "positive" question and YES price > 0.5, it's bullish
                let sentiment_score = if is_positive_question(&market.question) {
                    (market.yes_price - 0.5) * 2.0  // Convert 0-1 to -1-1
                } else {
                    (market.no_price - 0.5) * 2.0   // Invert for negative questions
                };
                
                total_sentiment += sentiment_score;
                count += 1;
            }
        }

        if count > 0 {
            Ok(total_sentiment / count as f64)
        } else {
            Ok(0.0)
        }
    }

    async fn calculate_defi_sentiment(&self, tvl: &HashMap<String, f64>, volume: &HashMap<String, f64>) -> AppResult<f64> {
        // Simple DeFi sentiment based on volume/TVL ratios
        // Higher volume relative to TVL suggests more activity/confidence
        
        let mut total_ratio = 0.0;
        let mut count = 0;

        for (protocol, tvl_value) in tvl {
            if let Some(volume_value) = volume.get(protocol) {
                if *tvl_value > 0.0 {
                    let ratio = volume_value / tvl_value;
                    total_ratio += ratio;
                    count += 1;
                }
            }
        }

        if count > 0 {
            let avg_ratio = total_ratio / count as f64;
            // Convert to -1 to 1 scale (this is a simplified heuristic)
            Ok((avg_ratio.min(1.0) - 0.5) * 2.0)
        } else {
            Ok(0.0)
        }
    }

    async fn store_defi_metrics(&self, data: &DeFiProtocolMetrics, protocol: &str) -> AppResult<()> {
        let query = r#"
            INSERT INTO market_metrics (
                timestamp, data_type, protocol, metric_name, metric_value
            ) VALUES (now(), 'defi', ?, 'tvl', ?)
        "#;

        self.db
            .client()
            .query(query)
            .bind(protocol)
            .bind(data.tvl)
            .execute()
            .await?;

        Ok(())
    }

    /// Get current market data cache
    pub async fn get_market_data(&self) -> MarketDataCache {
        self.market_cache.read().await.clone()
    }
}

fn is_positive_question(question: &str) -> bool {
    let positive_keywords = ["increase", "rise", "above", "higher", "more than", "exceed", "bullish", "grow"];
    let question_lower = question.to_lowercase();
    
    positive_keywords.iter().any(|keyword| question_lower.contains(keyword))
}

// Placeholder client structures - these would be implemented with actual API calls
#[derive(Clone)]
struct PolymarketClient;

impl PolymarketClient {
    fn new() -> Self { Self }
    
    async fn get_trending_markets(&self) -> AppResult<Vec<PredictionMarketData>> {
        // TODO: Implement real Polymarket API integration
        // For now, return empty vec until API credentials are configured
        tracing::warn!("Polymarket API not yet implemented - returning empty results");
        Ok(vec![])
    }
}

#[derive(Clone)]
struct KalshiClient;

impl KalshiClient {
    fn new() -> Self { Self }
    
    async fn get_active_markets(&self) -> AppResult<Vec<PredictionMarketData>> {
        // TODO: Implement real Kalshi API integration  
        // For now, return empty vec until API credentials are configured
        tracing::warn!("Kalshi API not yet implemented - returning empty results");
        Ok(vec![])
    }
}

#[derive(Clone)]
struct DeFiProtocolsClient;

#[derive(Clone, Debug)]
struct DeFiProtocolMetrics {
    tvl: f64,
    volume_24h: f64,
}

impl DeFiProtocolsClient {
    fn new() -> Self { Self }
    
    async fn get_jupiter_metrics(&self) -> AppResult<DeFiProtocolMetrics> {
        // TODO: Implement real Jupiter API integration
        // For now, return minimal data until API endpoints are integrated
        tracing::warn!("Jupiter API not yet implemented - using placeholder data");
        Ok(DeFiProtocolMetrics { 
            tvl: 0.0,
            volume_24h: 0.0
        })
    }
    
    async fn get_raydium_metrics(&self) -> AppResult<DeFiProtocolMetrics> {
        // TODO: Implement real Raydium API integration
        // For now, return minimal data until API endpoints are integrated
        tracing::warn!("Raydium API not yet implemented - using placeholder data");
        Ok(DeFiProtocolMetrics { 
            tvl: 0.0,
            volume_24h: 0.0
        })
    }
    
    async fn get_kamino_metrics(&self) -> AppResult<DeFiProtocolMetrics> {
        // TODO: Implement real Kamino API integration
        // For now, return minimal data until API endpoints are integrated
        tracing::warn!("Kamino API not yet implemented - using placeholder data");
        Ok(DeFiProtocolMetrics { 
            tvl: 0.0,
            volume_24h: 0.0
        })
    }
}