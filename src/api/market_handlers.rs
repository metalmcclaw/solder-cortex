use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use crate::error::AppResult;
use crate::indexer::market_data::{MarketDataCache, MarketDataIndexer};
use crate::AppState;

#[derive(Deserialize)]
pub struct MarketDataQuery {
    pub category: Option<String>,
    pub timeframe: Option<String>,  // "1h", "24h", "7d", "30d"
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct MarketOverviewResponse {
    pub defi: DeFiOverview,
    pub prediction_markets: PredictionMarketsOverview,
    pub sentiment: MarketSentimentResponse,
    pub last_updated: String,
}

#[derive(Serialize)]
pub struct DeFiOverview {
    pub total_tvl: f64,
    pub total_volume_24h: f64,
    pub protocol_breakdown: HashMap<String, ProtocolStats>,
    pub trending_tokens: Vec<TokenStats>,
}

#[derive(Serialize)]
pub struct ProtocolStats {
    pub tvl: f64,
    pub volume_24h: f64,
    pub market_share: f64,  // percentage of total
}

#[derive(Serialize)]
pub struct TokenStats {
    pub symbol: String,
    pub price_usd: f64,
    pub volume_24h: f64,
    pub price_change_24h: f64,
    pub defi_activity_score: f64,
}

#[derive(Serialize)]
pub struct PredictionMarketsOverview {
    pub total_markets: usize,
    pub total_volume_24h: f64,
    pub trending_topics: Vec<String>,
    pub market_categories: HashMap<String, CategoryStats>,
    pub featured_markets: Vec<FeaturedMarket>,
}

#[derive(Serialize)]
pub struct CategoryStats {
    pub market_count: usize,
    pub volume_24h: f64,
    pub avg_price_movement: f64,
}

#[derive(Serialize)]
pub struct FeaturedMarket {
    pub platform: String,
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub volume_24h: f64,
    pub end_date: String,
    pub category: String,
    pub conviction_score: f64,  // 0-1, how "sure" the market is
}

#[derive(Serialize)]
pub struct MarketSentimentResponse {
    pub overall_score: f64,      // -1 to 1
    pub confidence: f64,         // 0-1
    pub defi_sentiment: f64,
    pub prediction_sentiment: f64,
    pub fear_greed_index: f64,   // 0-100
    pub trending_topics: Vec<String>,
    pub sentiment_drivers: Vec<SentimentDriver>,
}

#[derive(Serialize)]
pub struct SentimentDriver {
    pub factor: String,
    pub impact: f64,     // -1 to 1
    pub description: String,
}

#[derive(Serialize)]
pub struct ConvictionSignalsResponse {
    pub signals: Vec<ConvictionSignal>,
    pub summary: ConvictionSummary,
}

#[derive(Serialize)]
pub struct ConvictionSignal {
    pub signal_type: String,  // "bullish_convergence", "bearish_divergence", etc.
    pub strength: f64,        // 0-1
    pub confidence: f64,      // 0-1
    pub description: String,
    pub supporting_data: HashMap<String, f64>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct ConvictionSummary {
    pub overall_conviction: f64,  // -1 to 1
    pub bull_signals: usize,
    pub bear_signals: usize,
    pub conflicting_signals: usize,
    pub data_completeness: f64,   // 0-1
}

/// Get comprehensive market overview
pub async fn get_market_overview(
    State(state): State<AppState>,
    Query(params): Query<MarketDataQuery>,
) -> AppResult<Json<MarketOverviewResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v2/market/overview");
    tracing::info!("Processing market overview request");

    let market_data = state.market_indexer.get_market_data().await;
    
    // Calculate DeFi overview
    let defi_overview = calculate_defi_overview(&market_data).await;
    
    // Calculate prediction markets overview
    let prediction_overview = calculate_prediction_overview(&market_data).await;
    
    // Format sentiment response
    let sentiment = MarketSentimentResponse {
        overall_score: market_data.market_sentiment.overall_score,
        confidence: calculate_sentiment_confidence(&market_data).await,
        defi_sentiment: market_data.market_sentiment.defi_sentiment,
        prediction_sentiment: market_data.market_sentiment.prediction_market_sentiment,
        fear_greed_index: market_data.market_sentiment.fear_greed_index,
        trending_topics: market_data.market_sentiment.trending_topics.clone(),
        sentiment_drivers: calculate_sentiment_drivers(&market_data).await,
    };

    let response = MarketOverviewResponse {
        defi: defi_overview,
        prediction_markets: prediction_overview,
        sentiment,
        last_updated: market_data.last_updated.to_rfc3339(),
    };

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v2/market/overview -> 200 OK ({}ms)", duration);
    tracing::info!(duration_ms = %duration, "Market overview completed");

    Ok(Json(response))
}

/// Get conviction signals analysis
pub async fn get_conviction_signals(
    State(state): State<AppState>,
    Query(params): Query<MarketDataQuery>,
) -> AppResult<Json<ConvictionSignalsResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v2/market/conviction");
    tracing::info!("Processing conviction signals request");

    let market_data = state.market_indexer.get_market_data().await;
    
    // Analyze conviction signals
    let signals = analyze_conviction_signals(&market_data).await;
    let summary = calculate_conviction_summary(&signals);

    let response = ConvictionSignalsResponse {
        signals,
        summary,
    };

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v2/market/conviction -> 200 OK ({}ms)", duration);
    tracing::info!(
        duration_ms = %duration,
        signal_count = %response.signals.len(),
        overall_conviction = %response.summary.overall_conviction,
        "Conviction signals analysis completed"
    );

    Ok(Json(response))
}

async fn calculate_defi_overview(market_data: &MarketDataCache) -> DeFiOverview {
    let total_tvl: f64 = market_data.defi_tvl.values().sum();
    let total_volume_24h: f64 = market_data.defi_volume_24h.values().sum();

    let mut protocol_breakdown = HashMap::new();
    for (protocol, tvl) in &market_data.defi_tvl {
        let volume = market_data.defi_volume_24h.get(protocol).unwrap_or(&0.0);
        let market_share = if total_tvl > 0.0 { tvl / total_tvl * 100.0 } else { 0.0 };
        
        protocol_breakdown.insert(protocol.clone(), ProtocolStats {
            tvl: *tvl,
            volume_24h: *volume,
            market_share,
        });
    }

    // Convert top tokens to simplified stats
    let trending_tokens = market_data.top_tokens.iter().map(|token| TokenStats {
        symbol: token.symbol.clone(),
        price_usd: token.price_usd,
        volume_24h: token.volume_24h,
        price_change_24h: token.price_change_24h,
        defi_activity_score: token.defi_usage.total_tvl / token.market_cap,
    }).collect();

    DeFiOverview {
        total_tvl,
        total_volume_24h,
        protocol_breakdown,
        trending_tokens,
    }
}

async fn calculate_prediction_overview(market_data: &MarketDataCache) -> PredictionMarketsOverview {
    let total_markets = market_data.prediction_markets.len();
    let total_volume_24h: f64 = market_data.prediction_markets.iter()
        .map(|m| m.volume_24h)
        .sum();

    // Group by category
    let mut market_categories = HashMap::new();
    for market in &market_data.prediction_markets {
        let stats = market_categories.entry(market.category.clone()).or_insert(CategoryStats {
            market_count: 0,
            volume_24h: 0.0,
            avg_price_movement: 0.0,
        });
        
        stats.market_count += 1;
        stats.volume_24h += market.volume_24h;
        
        // Simple price movement metric
        let price_movement = (market.yes_price - 0.5).abs() * 2.0; // 0-1 scale
        stats.avg_price_movement = (stats.avg_price_movement * (stats.market_count - 1) as f64 + price_movement) / stats.market_count as f64;
    }

    // Featured markets (top by volume + conviction)
    let mut featured_markets: Vec<FeaturedMarket> = market_data.prediction_markets.iter()
        .map(|market| {
            let conviction_score = calculate_conviction_for_market(market);
            FeaturedMarket {
                platform: market.platform.clone(),
                question: market.question.clone(),
                yes_price: market.yes_price,
                no_price: market.no_price,
                volume_24h: market.volume_24h,
                end_date: market.end_date.to_rfc3339(),
                category: market.category.clone(),
                conviction_score,
            }
        })
        .collect();

    // Sort by conviction * volume
    featured_markets.sort_by(|a, b| {
        let score_a = a.conviction_score * a.volume_24h;
        let score_b = b.conviction_score * b.volume_24h;
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    featured_markets.truncate(10); // Top 10

    // Extract trending topics
    let trending_topics = extract_trending_topics(&market_data.prediction_markets);

    PredictionMarketsOverview {
        total_markets,
        total_volume_24h,
        trending_topics,
        market_categories,
        featured_markets,
    }
}

async fn calculate_sentiment_confidence(market_data: &MarketDataCache) -> f64 {
    // Calculate confidence based on data completeness and consistency
    let defi_data_points = market_data.defi_tvl.len() + market_data.defi_volume_24h.len();
    let prediction_data_points = market_data.prediction_markets.len();
    
    // Simple confidence metric based on data availability
    let max_expected_defi_points = 6;  // 3 protocols * 2 metrics
    let max_expected_prediction_points = 50;  // reasonable number of markets
    
    let defi_completeness = (defi_data_points as f64 / max_expected_defi_points as f64).min(1.0);
    let prediction_completeness = (prediction_data_points as f64 / max_expected_prediction_points as f64).min(1.0);
    
    (defi_completeness + prediction_completeness) / 2.0
}

async fn calculate_sentiment_drivers(market_data: &MarketDataCache) -> Vec<SentimentDriver> {
    let mut drivers = Vec::new();
    
    // DeFi volume trends
    let total_volume: f64 = market_data.defi_volume_24h.values().sum();
    if total_volume > 0.0 {
        let volume_impact = if total_volume > 100_000_000.0 { 0.3 } else { -0.2 }; // $100M threshold
        drivers.push(SentimentDriver {
            factor: "DeFi Trading Volume".to_string(),
            impact: volume_impact,
            description: format!("24h DeFi volume: ${:.1}M", total_volume / 1_000_000.0),
        });
    }
    
    // Prediction market bullishness
    let crypto_markets = market_data.prediction_markets.iter()
        .filter(|m| m.tags.contains(&"crypto".to_string()));
    
    let avg_bullishness: f64 = crypto_markets.clone()
        .map(|m| m.yes_price)
        .sum::<f64>() / crypto_markets.count() as f64;
    
    if avg_bullishness > 0.0 {
        let prediction_impact = (avg_bullishness - 0.5) * 2.0;  // Convert to -1 to 1
        drivers.push(SentimentDriver {
            factor: "Crypto Prediction Markets".to_string(),
            impact: prediction_impact,
            description: format!("Average YES price on crypto markets: {:.1}%", avg_bullishness * 100.0),
        });
    }
    
    drivers
}

async fn analyze_conviction_signals(market_data: &MarketDataCache) -> Vec<ConvictionSignal> {
    let mut signals = Vec::new();
    
    // Signal 1: DeFi-Prediction Market Convergence
    if let Some(convergence_signal) = check_defi_prediction_convergence(market_data).await {
        signals.push(convergence_signal);
    }
    
    // Signal 2: Volume-TVL Ratio Anomalies
    if let Some(ratio_signal) = check_volume_tvl_ratios(market_data).await {
        signals.push(ratio_signal);
    }
    
    // Signal 3: Cross-Platform Prediction Consensus
    if let Some(consensus_signal) = check_prediction_consensus(market_data).await {
        signals.push(consensus_signal);
    }
    
    signals
}

async fn check_defi_prediction_convergence(market_data: &MarketDataCache) -> Option<ConvictionSignal> {
    // Look for convergence between DeFi activity and prediction market sentiment
    
    let defi_activity_score = calculate_defi_activity_score(market_data);
    let prediction_sentiment_score = market_data.market_sentiment.prediction_market_sentiment;
    
    let convergence = 1.0 - (defi_activity_score - prediction_sentiment_score).abs();
    
    if convergence > 0.7 {  // Strong convergence
        let mut supporting_data = HashMap::new();
        supporting_data.insert("defi_activity".to_string(), defi_activity_score);
        supporting_data.insert("prediction_sentiment".to_string(), prediction_sentiment_score);
        supporting_data.insert("convergence_score".to_string(), convergence);
        
        Some(ConvictionSignal {
            signal_type: if defi_activity_score > 0.0 { "bullish_convergence".to_string() } else { "bearish_convergence".to_string() },
            strength: convergence,
            confidence: 0.8,
            description: format!("DeFi activity and prediction market sentiment strongly aligned ({:.1}% convergence)", convergence * 100.0),
            supporting_data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    } else {
        None
    }
}

async fn check_volume_tvl_ratios(market_data: &MarketDataCache) -> Option<ConvictionSignal> {
    let mut ratios = Vec::new();
    
    for (protocol, tvl) in &market_data.defi_tvl {
        if let Some(volume) = market_data.defi_volume_24h.get(protocol) {
            if *tvl > 0.0 {
                ratios.push(volume / tvl);
            }
        }
    }
    
    if ratios.is_empty() {
        return None;
    }
    
    let avg_ratio: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
    
    // High volume/TVL ratio could indicate high activity/conviction
    if avg_ratio > 0.1 {  // 10% of TVL traded in 24h is significant
        let mut supporting_data = HashMap::new();
        supporting_data.insert("avg_volume_tvl_ratio".to_string(), avg_ratio);
        supporting_data.insert("protocols_analyzed".to_string(), ratios.len() as f64);
        
        Some(ConvictionSignal {
            signal_type: "high_activity_conviction".to_string(),
            strength: (avg_ratio * 10.0).min(1.0),  // Scale to 0-1
            confidence: 0.7,
            description: format!("High volume/TVL ratio ({:.1}%) indicates strong market activity", avg_ratio * 100.0),
            supporting_data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    } else {
        None
    }
}

async fn check_prediction_consensus(market_data: &MarketDataCache) -> Option<ConvictionSignal> {
    // Look for strong consensus in prediction markets (multiple markets pointing same direction)
    
    let crypto_markets: Vec<_> = market_data.prediction_markets.iter()
        .filter(|m| m.tags.contains(&"crypto".to_string()) || 
                   m.question.to_lowercase().contains("bitcoin") ||
                   m.question.to_lowercase().contains("ethereum") ||
                   m.question.to_lowercase().contains("solana"))
        .collect();
    
    if crypto_markets.len() < 3 {
        return None;  // Need at least 3 markets for consensus
    }
    
    let bullish_markets = crypto_markets.iter().filter(|m| m.yes_price > 0.6).count();
    let bearish_markets = crypto_markets.iter().filter(|m| m.yes_price < 0.4).count();
    
    let consensus_ratio = (bullish_markets.max(bearish_markets) as f64) / crypto_markets.len() as f64;
    
    if consensus_ratio > 0.7 {  // 70%+ markets agree
        let is_bullish = bullish_markets > bearish_markets;
        let mut supporting_data = HashMap::new();
        supporting_data.insert("total_markets".to_string(), crypto_markets.len() as f64);
        supporting_data.insert("consensus_markets".to_string(), bullish_markets.max(bearish_markets) as f64);
        supporting_data.insert("consensus_ratio".to_string(), consensus_ratio);
        
        Some(ConvictionSignal {
            signal_type: if is_bullish { "bullish_consensus".to_string() } else { "bearish_consensus".to_string() },
            strength: consensus_ratio,
            confidence: 0.85,
            description: format!("{:.0}% of crypto prediction markets show {} sentiment", 
                                consensus_ratio * 100.0, 
                                if is_bullish { "bullish" } else { "bearish" }),
            supporting_data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    } else {
        None
    }
}

fn calculate_conviction_summary(signals: &[ConvictionSignal]) -> ConvictionSummary {
    if signals.is_empty() {
        return ConvictionSummary {
            overall_conviction: 0.0,
            bull_signals: 0,
            bear_signals: 0,
            conflicting_signals: 0,
            data_completeness: 0.5,
        };
    }
    
    let bull_signals = signals.iter().filter(|s| s.signal_type.contains("bullish")).count();
    let bear_signals = signals.iter().filter(|s| s.signal_type.contains("bearish")).count();
    let conflicting_signals = if bull_signals > 0 && bear_signals > 0 { 1 } else { 0 };
    
    // Calculate overall conviction as weighted average
    let total_weight: f64 = signals.iter().map(|s| s.strength * s.confidence).sum();
    let weighted_conviction: f64 = signals.iter()
        .map(|s| {
            let direction = if s.signal_type.contains("bullish") { 1.0 } else { -1.0 };
            direction * s.strength * s.confidence
        })
        .sum();
    
    let overall_conviction = if total_weight > 0.0 {
        weighted_conviction / total_weight
    } else {
        0.0
    };
    
    let data_completeness = (signals.len() as f64 / 5.0).min(1.0);  // Assume 5 signals is complete
    
    ConvictionSummary {
        overall_conviction,
        bull_signals,
        bear_signals,
        conflicting_signals,
        data_completeness,
    }
}

fn calculate_defi_activity_score(market_data: &MarketDataCache) -> f64 {
    let total_volume: f64 = market_data.defi_volume_24h.values().sum();
    let total_tvl: f64 = market_data.defi_tvl.values().sum();
    
    if total_tvl > 0.0 {
        // Normalize the volume/TVL ratio to a -1 to 1 scale
        let ratio = total_volume / total_tvl;
        (ratio.min(0.5) - 0.25) * 4.0  // Map 0-0.5 to -1 to 1
    } else {
        0.0
    }
}

fn calculate_conviction_for_market(market: &crate::indexer::market_data::PredictionMarketData) -> f64 {
    // Conviction is higher when:
    // 1. Price is far from 0.5 (market is "sure")
    // 2. Volume is high (lots of activity)
    // 3. Market has time left (not expiring soon)
    
    let price_certainty = (market.yes_price - 0.5).abs() * 2.0;  // 0-1 scale
    let volume_score = (market.volume_24h.ln() / 20.0).min(1.0).max(0.0);  // Log scale, capped at 1
    
    // Time factor (assume markets with more time are more valuable for conviction)
    let days_left = (market.end_date - chrono::Utc::now()).num_days().max(0) as f64;
    let time_factor = (days_left / 30.0).min(1.0);  // 30 days = full score
    
    (price_certainty + volume_score + time_factor) / 3.0
}

fn extract_trending_topics(markets: &[crate::indexer::market_data::PredictionMarketData]) -> Vec<String> {
    use std::collections::HashMap;
    
    let mut topic_counts = HashMap::new();
    
    for market in markets {
        for tag in &market.tags {
            *topic_counts.entry(tag.clone()).or_insert(0) += 1;
        }
        
        // Extract topics from questions
        let question_words: Vec<&str> = market.question.to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 4)  // Only longer words
            .collect();
        
        for word in question_words {
            if !["will", "does", "would", "should", "could", "above", "below", "before", "after"].contains(&word) {
                *topic_counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    // Sort by count and take top topics
    let mut topics: Vec<(String, i32)> = topic_counts.into_iter().collect();
    topics.sort_by(|a, b| b.1.cmp(&a.1));
    
    topics.into_iter()
        .take(10)
        .map(|(topic, _)| topic)
        .collect()
}