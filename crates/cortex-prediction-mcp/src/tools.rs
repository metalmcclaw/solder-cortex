use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cache::SharedCache;
use crate::db::models::*;
use crate::db::QueryEngine;
use crate::error::{validate_interval, validate_slug, PredictionError, Result};

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

// ============================================================================
// Tool Definitions
// ============================================================================

pub fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_market_trend".to_string(),
            description: "Query price movement of a prediction market over a specific timeframe. Returns OHLCV data, volume, and trend direction.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "Market slug identifier (e.g., 'will-trump-win-2024')"
                    },
                    "interval": {
                        "type": "string",
                        "enum": ["1m", "5m", "15m", "30m", "1h", "4h", "24h", "7d"],
                        "description": "Time interval for aggregation"
                    }
                },
                "required": ["slug", "interval"]
            }),
        },
        Tool {
            name: "get_volume_profile".to_string(),
            description: "Get trading volume summary and liquidity depth for a prediction market. Includes 24h/7d volume, trade counts, and order book metrics.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "Market slug identifier"
                    }
                },
                "required": ["slug"]
            }),
        },
        Tool {
            name: "search_market_memory".to_string(),
            description: "Search historical prediction markets by keyword. Finds markets matching the query in titles, descriptions, and categories.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query to find historical markets"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 10, max: 100)"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "detect_anomalies".to_string(),
            description: "Find price spikes that deviate significantly from the 1-hour moving average. Returns anomalies exceeding the standard deviation threshold.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "Market slug identifier"
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Standard deviation threshold (default: 3.0)"
                    }
                },
                "required": ["slug"]
            }),
        },
    ]
}

// ============================================================================
// Tool Business Logic
// ============================================================================

/// Core prediction market tool logic
pub struct PredictionTools {
    query_engine: Arc<QueryEngine>,
    cache: SharedCache,
}

impl PredictionTools {
    pub fn new(query_engine: Arc<QueryEngine>, cache: SharedCache) -> Self {
        Self {
            query_engine,
            cache,
        }
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        match name {
            "get_market_trend" => {
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;
                let interval = args["interval"].as_str().ok_or("Missing interval parameter")?;

                match self.get_market_trend(slug, interval).await {
                    Ok(result) => serde_json::to_value(result)
                        .map_err(|e| format!("Serialization error: {}", e)),
                    Err(e) => Err(e.to_string()),
                }
            }
            "get_volume_profile" => {
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;

                match self.get_volume_profile(slug).await {
                    Ok(result) => serde_json::to_value(result)
                        .map_err(|e| format!("Serialization error: {}", e)),
                    Err(e) => Err(e.to_string()),
                }
            }
            "search_market_memory" => {
                let query = args["query"].as_str().ok_or("Missing query parameter")?;
                let limit = args["limit"].as_u64().unwrap_or(10) as u32;

                match self.search_market_memory(query, limit).await {
                    Ok(result) => serde_json::to_value(result)
                        .map_err(|e| format!("Serialization error: {}", e)),
                    Err(e) => Err(e.to_string()),
                }
            }
            "detect_anomalies" => {
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;
                let threshold = args["threshold"].as_f64().unwrap_or(3.0);

                match self.detect_anomalies(slug, threshold).await {
                    Ok(result) => serde_json::to_value(result)
                        .map_err(|e| format!("Serialization error: {}", e)),
                    Err(e) => Err(e.to_string()),
                }
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    /// Get market price trend with OHLCV data
    async fn get_market_trend(
        &self,
        slug: &str,
        interval: &str,
    ) -> Result<MarketTrendResponse> {
        validate_slug(slug)?;
        validate_interval(interval)?;

        // Check cache first
        if let Some(cached) = self.cache.get_market_trend(slug, interval).await {
            tracing::debug!(slug = %slug, interval = %interval, "Cache hit for market trend");
            return self.format_trend_response(slug, interval, cached);
        }

        // Query database
        let ohlcv_data = self
            .query_engine
            .get_market_trend(slug, interval)
            .await?;

        if ohlcv_data.is_empty() {
            return Err(PredictionError::MarketNotFound(slug.to_string()));
        }

        // Cache result
        self.cache
            .set_market_trend(slug, interval, ohlcv_data.clone())
            .await;

        self.format_trend_response(slug, interval, ohlcv_data)
    }

    fn format_trend_response(
        &self,
        slug: &str,
        interval: &str,
        data: Vec<OhlcvRow>,
    ) -> Result<MarketTrendResponse> {
        if data.is_empty() {
            return Err(PredictionError::MarketNotFound(slug.to_string()));
        }

        let ohlcv: Vec<OhlcvData> = data
            .iter()
            .map(|row| {
                let timestamp = DateTime::<Utc>::from_timestamp_millis(row.interval_start)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default();

                OhlcvData {
                    timestamp,
                    open: row.open_price.parse().unwrap_or(0.0),
                    high: row.high_price.parse().unwrap_or(0.0),
                    low: row.low_price.parse().unwrap_or(0.0),
                    close: row.close_price.parse().unwrap_or(0.0),
                    volume_usd: row.volume_usd.parse().unwrap_or(0.0),
                    trades: row.trade_count,
                }
            })
            .collect();

        // Calculate summary
        let first = ohlcv.first().unwrap();
        let last = ohlcv.last().unwrap();
        let price_change = last.close - first.open;
        let price_change_pct = if first.open > 0.0 {
            (price_change / first.open) * 100.0
        } else {
            0.0
        };

        let high = ohlcv.iter().map(|o| o.high).fold(0.0_f64, f64::max);
        let low = ohlcv.iter().map(|o| o.low).fold(f64::MAX, f64::min);
        let total_volume: f64 = ohlcv.iter().map(|o| o.volume_usd).sum();

        let trend_direction = if price_change_pct > 1.0 {
            "up"
        } else if price_change_pct < -1.0 {
            "down"
        } else {
            "sideways"
        }
        .to_string();

        Ok(MarketTrendResponse {
            slug: slug.to_string(),
            platform: "polymarket".to_string(),
            interval: interval.to_string(),
            data_points: ohlcv.len(),
            ohlcv,
            summary: TrendSummary {
                price_change,
                price_change_pct,
                high,
                low,
                total_volume,
                trend_direction,
            },
        })
    }

    /// Get volume profile and liquidity data
    async fn get_volume_profile(&self, slug: &str) -> Result<VolumeProfileResponse> {
        validate_slug(slug)?;

        // Check cache
        if let Some(cached) = self.cache.get_volume_profile(slug).await {
            if let Some(data) = cached {
                tracing::debug!(slug = %slug, "Cache hit for volume profile");
                return self.format_volume_response(data);
            }
        }

        // Query database
        let profile = self
            .query_engine
            .get_volume_profile(slug)
            .await?
            .ok_or_else(|| PredictionError::MarketNotFound(slug.to_string()))?;

        // Cache result
        self.cache
            .set_volume_profile(slug, Some(profile.clone()))
            .await;

        self.format_volume_response(profile)
    }

    fn format_volume_response(&self, data: VolumeProfileRow) -> Result<VolumeProfileResponse> {
        let bid_depth: f64 = data.bid_depth_usd.parse().unwrap_or(0.0);
        let ask_depth: f64 = data.ask_depth_usd.parse().unwrap_or(0.0);
        let spread: f64 = data.spread.parse().unwrap_or(0.0);

        let spread_bps = spread * 10000.0;

        Ok(VolumeProfileResponse {
            slug: data.slug,
            platform: data.platform,
            volume_24h: data.total_volume_24h.parse().unwrap_or(0.0),
            volume_7d: data.total_volume_7d.parse().unwrap_or(0.0),
            trades_24h: data.total_trades_24h,
            unique_traders_24h: data.unique_traders_24h,
            avg_trade_size: data.avg_trade_size.parse().unwrap_or(0.0),
            liquidity: LiquidityInfo {
                bid_depth_usd: bid_depth,
                ask_depth_usd: ask_depth,
                spread,
                spread_bps,
            },
        })
    }

    /// Search historical markets by text query
    async fn search_market_memory(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<SearchMemoryResponse> {
        let query = query.trim();
        if query.is_empty() {
            return Err(PredictionError::Query("Search query cannot be empty".into()));
        }

        let limit = limit.min(100);

        // Check cache
        if let Some(cached) = self.cache.get_search_memory(query).await {
            tracing::debug!(query = %query, "Cache hit for search memory");
            return self.format_search_response(query, cached);
        }

        // Query database
        let results = self.query_engine.search_markets(query, limit).await?;

        // Cache result
        self.cache.set_search_memory(query, results.clone()).await;

        self.format_search_response(query, results)
    }

    fn format_search_response(
        &self,
        query: &str,
        results: Vec<MarketSearchResult>,
    ) -> Result<SearchMemoryResponse> {
        let markets: Vec<MarketSearchItem> = results
            .into_iter()
            .map(|r| MarketSearchItem {
                slug: r.slug,
                platform: r.platform,
                title: r.title,
                category: r.category,
                status: r.status,
                current_price: r.current_price.parse().unwrap_or(0.0),
                volume_24h: r.volume_24h.parse().unwrap_or(0.0),
                relevance_score: r.relevance_score,
            })
            .collect();

        Ok(SearchMemoryResponse {
            query: query.to_string(),
            results_count: markets.len(),
            markets,
        })
    }

    /// Detect price anomalies (spikes > threshold standard deviations)
    async fn detect_anomalies(
        &self,
        slug: &str,
        threshold: f64,
    ) -> Result<AnomalyResponse> {
        validate_slug(slug)?;

        if threshold <= 0.0 {
            return Err(PredictionError::Query(
                "Threshold must be greater than 0".into(),
            ));
        }

        // Check cache
        if let Some(cached) = self.cache.get_anomalies(slug).await {
            tracing::debug!(slug = %slug, "Cache hit for anomalies");
            return self.format_anomaly_response(slug, threshold, cached);
        }

        // Query database
        let anomalies = self
            .query_engine
            .detect_anomalies(slug, threshold)
            .await?;

        // Cache result
        self.cache
            .set_anomalies(slug, anomalies.clone())
            .await;

        self.format_anomaly_response(slug, threshold, anomalies)
    }

    fn format_anomaly_response(
        &self,
        slug: &str,
        threshold: f64,
        data: Vec<AnomalyRow>,
    ) -> Result<AnomalyResponse> {
        let anomalies: Vec<AnomalyItem> = data
            .into_iter()
            .map(|a| {
                let timestamp = DateTime::<Utc>::from_timestamp_millis(a.timestamp)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default();

                let direction = if a.z_score > 0.0 {
                    "spike_up"
                } else {
                    "spike_down"
                }
                .to_string();

                AnomalyItem {
                    timestamp,
                    price: a.price.parse().unwrap_or(0.0),
                    mean_price: a.mean_price.parse().unwrap_or(0.0),
                    z_score: a.z_score,
                    deviation_pct: a.deviation_pct.parse().unwrap_or(0.0),
                    direction,
                    outcome_token: a.outcome_token,
                }
            })
            .collect();

        Ok(AnomalyResponse {
            slug: slug.to_string(),
            threshold_std_dev: threshold,
            anomalies_found: anomalies.len(),
            anomalies,
        })
    }
}

// ============================================================================
// MCP Message Handlers
// ============================================================================

pub fn handle_initialize(_params: &Value) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "cortex-prediction-mcp",
            "version": "0.1.0"
        }
    })
}

pub fn handle_list_tools() -> Value {
    json!({
        "tools": get_tools()
    })
}

pub async fn handle_call_tool(tools: &PredictionTools, params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    match tools.execute(name, args).await {
        Ok(result) => {
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                }]
            })
        }
        Err(e) => {
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("Error: {}", e)
                }],
                "isError": true
            })
        }
    }
}

pub async fn handle_request(tools: &PredictionTools, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match request.method.as_str() {
        "initialize" => Some(handle_initialize(&request.params)),
        "initialized" => None, // Notification, no response needed
        "tools/list" => Some(handle_list_tools()),
        "tools/call" => Some(handle_call_tool(tools, &request.params).await),
        "notifications/cancelled" => None, // Notification
        _ => {
            return Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            });
        }
    };

    result.map(|r| JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id,
        result: Some(r),
        error: None,
    })
}
