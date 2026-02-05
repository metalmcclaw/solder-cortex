//! MCP tool definitions and request handling
//!
//! This module defines all available tools and handles MCP protocol messages.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::defi::DefiClient;
use crate::error::{validate_wallet, CortexMcpError};
use crate::prediction::PredictionEngine;

// =============================================================================
// MCP Protocol Types
// =============================================================================

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

// =============================================================================
// Unified Tools
// =============================================================================

/// Unified tool handler for all Cortex capabilities
pub struct CortexTools {
    defi: Arc<DefiClient>,
    prediction: Option<Arc<PredictionEngine>>,
}

impl CortexTools {
    pub fn new(defi: Arc<DefiClient>, prediction: Option<Arc<PredictionEngine>>) -> Self {
        Self { defi, prediction }
    }

    /// Get all available tools
    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            // Health check
            Tool {
                name: "cortex_health".to_string(),
                description: "Check the health status of the Cortex service and all connected backends".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            // DeFi Tools
            Tool {
                name: "cortex_get_wallet_summary".to_string(),
                description: "Get a comprehensive summary of a Solana wallet including total value, PnL (profit/loss), risk metrics, and protocol exposure.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address (base58 encoded)"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_get_wallet_pnl".to_string(),
                description: "Get detailed profit and loss breakdown by protocol for a wallet. Shows realized and unrealized PnL.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address (base58 encoded)"
                        },
                        "window": {
                            "type": "string",
                            "enum": ["24h", "7d", "30d", "all"],
                            "description": "Time window for PnL calculation (default: 7d)"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_get_wallet_positions".to_string(),
                description: "Get all current open positions for a wallet across DeFi protocols.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address (base58 encoded)"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_start_indexing".to_string(),
                description: "Start continuous indexing for a wallet. Fetches historical transactions and monitors real-time activity.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address to start indexing"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_stop_indexing".to_string(),
                description: "Stop continuous indexing for a wallet.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address to stop indexing"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_list_subscriptions".to_string(),
                description: "List all wallets currently being indexed with their status.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            // Cross-Domain Intelligence Tools
            Tool {
                name: "cortex_get_wallet_conviction".to_string(),
                description: "Analyze a wallet's cross-domain conviction by correlating DeFi positions with prediction market bets. Returns a conviction score (0-1) indicating alignment between on-chain actions and market predictions.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "wallet": {
                            "type": "string",
                            "description": "Solana wallet address (base58 encoded)"
                        }
                    },
                    "required": ["wallet"]
                }),
            },
            Tool {
                name: "cortex_detect_informed_traders".to_string(),
                description: "Detect informed traders in a prediction market by finding wallets with both bets AND relevant on-chain DeFi activity.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "market_slug": {
                            "type": "string",
                            "description": "Prediction market identifier/slug"
                        },
                        "platform": {
                            "type": "string",
                            "enum": ["polymarket", "kalshi"],
                            "description": "Prediction market platform (default: polymarket)"
                        },
                        "min_conviction": {
                            "type": "number",
                            "description": "Minimum conviction score to include (0-1, default: 0.5)"
                        }
                    },
                    "required": ["market_slug"]
                }),
            },
        ];

        // Add prediction market tools if engine is available
        if self.prediction.is_some() {
            tools.extend(vec![
                Tool {
                    name: "cortex_get_market_trend".to_string(),
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
                    name: "cortex_get_volume_profile".to_string(),
                    description: "Get trading volume summary and liquidity depth for a prediction market.".to_string(),
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
                    name: "cortex_search_market_memory".to_string(),
                    description: "Search historical prediction markets by keyword.".to_string(),
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
                    name: "cortex_detect_anomalies".to_string(),
                    description: "Find price spikes that deviate significantly from the moving average.".to_string(),
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
            ]);
        }

        tools
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            // Health check
            "cortex_health" => {
                let defi_health = self.defi.health().await.ok();
                let prediction_status = self.prediction.is_some();

                Ok(json!({
                    "status": "healthy",
                    "version": "0.2.0",
                    "defi": {
                        "available": defi_health.is_some(),
                        "response": defi_health
                    },
                    "prediction": {
                        "available": prediction_status
                    }
                }))
            }

            // DeFi tools
            "cortex_get_wallet_summary" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                self.defi.get_wallet_summary(wallet).await.map_err(|e| e.to_string())
            }

            "cortex_get_wallet_pnl" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                let window = args["window"].as_str().unwrap_or("7d");
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                self.defi.get_wallet_pnl(wallet, window).await.map_err(|e| e.to_string())
            }

            "cortex_get_wallet_positions" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                self.defi.get_wallet_positions(wallet).await.map_err(|e| e.to_string())
            }

            "cortex_start_indexing" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                self.defi.start_indexing(wallet).await.map_err(|e| e.to_string())
            }

            "cortex_stop_indexing" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                self.defi.stop_indexing(wallet).await.map_err(|e| e.to_string())
            }

            "cortex_list_subscriptions" => {
                self.defi.list_subscriptions().await.map_err(|e| e.to_string())
            }

            // Cross-domain intelligence
            "cortex_get_wallet_conviction" => {
                let wallet = args["wallet"].as_str().ok_or("Missing wallet parameter")?;
                validate_wallet(wallet).map_err(|e| e.to_string())?;
                let response = self.defi.get_wallet_conviction(wallet).await.map_err(|e| e.to_string())?;
                serde_json::to_value(response).map_err(|e| e.to_string())
            }

            "cortex_detect_informed_traders" => {
                let market_slug = args["market_slug"].as_str().ok_or("Missing market_slug parameter")?;
                let platform = args["platform"].as_str().unwrap_or("polymarket");
                let min_conviction = args["min_conviction"].as_f64().unwrap_or(0.5);
                self.defi.detect_informed_traders(market_slug, platform, min_conviction).await.map_err(|e| e.to_string())
            }

            // Prediction market tools
            "cortex_get_market_trend" => {
                let prediction = self.prediction.as_ref()
                    .ok_or("Prediction market features not available")?;
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;
                let interval = args["interval"].as_str().ok_or("Missing interval parameter")?;
                let response = prediction.get_market_trend(slug, interval).await.map_err(|e| e.to_string())?;
                serde_json::to_value(response).map_err(|e| e.to_string())
            }

            "cortex_get_volume_profile" => {
                let prediction = self.prediction.as_ref()
                    .ok_or("Prediction market features not available")?;
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;
                let response = prediction.get_volume_profile(slug).await.map_err(|e| e.to_string())?;
                serde_json::to_value(response).map_err(|e| e.to_string())
            }

            "cortex_search_market_memory" => {
                let prediction = self.prediction.as_ref()
                    .ok_or("Prediction market features not available")?;
                let query = args["query"].as_str().ok_or("Missing query parameter")?;
                let limit = args["limit"].as_u64().unwrap_or(10) as u32;
                let response = prediction.search_market_memory(query, limit).await.map_err(|e| e.to_string())?;
                serde_json::to_value(response).map_err(|e| e.to_string())
            }

            "cortex_detect_anomalies" => {
                let prediction = self.prediction.as_ref()
                    .ok_or("Prediction market features not available")?;
                let slug = args["slug"].as_str().ok_or("Missing slug parameter")?;
                let threshold = args["threshold"].as_f64().unwrap_or(3.0);
                let response = prediction.detect_anomalies(slug, threshold).await.map_err(|e| e.to_string())?;
                serde_json::to_value(response).map_err(|e| e.to_string())
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

// =============================================================================
// MCP Protocol Handlers
// =============================================================================

fn handle_initialize(_params: &Value) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "cortex-unified-mcp",
            "version": "0.2.0"
        }
    })
}

fn handle_list_tools(tools: &CortexTools) -> Value {
    json!({
        "tools": tools.get_tools()
    })
}

async fn handle_call_tool(tools: &CortexTools, params: &Value) -> Value {
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

/// Handle an incoming MCP request
pub async fn handle_request(
    tools: &CortexTools,
    request: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let result = match request.method.as_str() {
        "initialize" => Some(handle_initialize(&request.params)),
        "initialized" => None,
        "tools/list" => Some(handle_list_tools(tools)),
        "tools/call" => Some(handle_call_tool(tools, &request.params).await),
        "notifications/cancelled" => None,
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
