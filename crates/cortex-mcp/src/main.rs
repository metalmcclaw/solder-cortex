//! Cortex MCP Server
//!
//! MCP (Model Context Protocol) server that exposes Cortex APIs as tools
//! for AI assistants like Claude.
//!
//! This is the unified MCP server for Solder Cortex, providing both DeFi
//! analytics and cross-domain intelligence features.

use chrono::Utc;
use cortex_core::{
    ConvictionConfidence, DeFiPosition, MarketStatus, PositionType,
    PredictionMarketBet, Wallet, WalletClassification, WalletConvictionResponse,
    calculate_conviction, conviction_to_response,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const CORTEX_API_URL: &str = "http://localhost:3000";

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct TextContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ============================================================================
// Tool Definitions
// ============================================================================

fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "cortex_health".to_string(),
            description: "Check the health status of the Cortex service and database connection".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "cortex_get_wallet_summary".to_string(),
            description: "Get a comprehensive summary of a Solana wallet including total value, PnL (profit/loss), risk metrics, and protocol exposure. Use this to understand a wallet's overall DeFi position.".to_string(),
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
            description: "Get detailed profit and loss breakdown by protocol for a wallet within a time window. Shows realized and unrealized PnL for each protocol (Jupiter, Raydium, Kamino, etc.)".to_string(),
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
            description: "Get all current open positions for a wallet across DeFi protocols. Includes lending positions (supply/borrow) and liquidity pool positions with their USD values.".to_string(),
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
            description: "Start continuous indexing for a wallet. This fetches historical transactions from Helius and then streams real-time transactions from LYS Labs. The wallet will be continuously monitored for new DeFi activity.".to_string(),
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
            description: "Stop continuous indexing for a wallet. This stops the real-time transaction monitoring for the specified wallet.".to_string(),
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
            description: "List all wallets currently being indexed with their status, start time, and number of transactions processed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Cross-Domain Intelligence Tools
        Tool {
            name: "cortex_get_wallet_conviction".to_string(),
            description: "Analyze a wallet's cross-domain conviction by correlating their DeFi positions with prediction market bets. Returns a conviction score (0-1) indicating how aligned their on-chain actions are with their market predictions. High conviction = putting money where their mouth is.".to_string(),
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
            description: "Detect informed traders in a prediction market by finding wallets that have both placed bets AND have relevant on-chain DeFi activity. Returns wallets with high conviction scores and their aggregate signal direction.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "market_slug": {
                        "type": "string",
                        "description": "Prediction market identifier/slug (e.g., 'eth-above-5000-march-2026')"
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
    ]
}

// ============================================================================
// HTTP Client
// ============================================================================

fn http_get(url: &str) -> Result<Value, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let body: Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(body)
}

fn http_post(url: &str, body: Value) -> Result<Value, String> {
    let response = ureq::post(url)
        .set("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let result: Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result)
}

fn http_delete(url: &str) -> Result<Value, String> {
    let response = ureq::delete(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let body: Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(body)
}

// ============================================================================
// Tool Execution
// ============================================================================

fn execute_tool(name: &str, args: &Value) -> Result<Value, String> {
    let api_url = std::env::var("CORTEX_API_URL").unwrap_or_else(|_| CORTEX_API_URL.to_string());

    match name {
        "cortex_health" => {
            let url = format!("{}/health", api_url);
            http_get(&url)
        }

        "cortex_get_wallet_summary" => {
            let wallet = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            let url = format!("{}/api/v1/user/{}/summary", api_url, wallet);
            http_get(&url)
        }

        "cortex_get_wallet_pnl" => {
            let wallet = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            let window = args["window"].as_str().unwrap_or("7d");
            let url = format!("{}/api/v1/user/{}/pnl?window={}", api_url, wallet, window);
            http_get(&url)
        }

        "cortex_get_wallet_positions" => {
            let wallet = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            let url = format!("{}/api/v1/user/{}/positions", api_url, wallet);
            http_get(&url)
        }

        "cortex_start_indexing" => {
            let wallet = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            let url = format!("{}/api/v1/index", api_url);
            http_post(&url, json!({ "wallet": wallet }))
        }

        "cortex_stop_indexing" => {
            let wallet = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            let url = format!("{}/api/v1/index/{}", api_url, wallet);
            http_delete(&url)
        }

        "cortex_list_subscriptions" => {
            let url = format!("{}/api/v1/index", api_url);
            http_get(&url)
        }

        // Cross-Domain Intelligence Tools
        "cortex_get_wallet_conviction" => {
            let wallet_addr = args["wallet"]
                .as_str()
                .ok_or("Missing wallet parameter")?;
            
            execute_wallet_conviction(&api_url, wallet_addr)
        }

        "cortex_detect_informed_traders" => {
            let market_slug = args["market_slug"]
                .as_str()
                .ok_or("Missing market_slug parameter")?;
            let platform = args["platform"].as_str().unwrap_or("polymarket");
            let min_conviction = args["min_conviction"].as_f64().unwrap_or(0.5);
            
            execute_informed_traders(&api_url, market_slug, platform, min_conviction)
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// ============================================================================
// Cross-Domain Intelligence Execution
// ============================================================================

/// Execute wallet conviction analysis
fn execute_wallet_conviction(api_url: &str, wallet_addr: &str) -> Result<Value, String> {
    // Fetch DeFi data from the existing API
    let summary_url = format!("{}/api/v1/user/{}/summary", api_url, wallet_addr);
    let positions_url = format!("{}/api/v1/user/{}/positions", api_url, wallet_addr);
    
    let summary = http_get(&summary_url).unwrap_or(json!({}));
    let positions_data = http_get(&positions_url).unwrap_or(json!({"positions": []}));
    
    // Parse DeFi positions from API response
    let defi_positions = parse_defi_positions(&positions_data);
    
    // Fetch prediction market bets (from prediction MCP or placeholder)
    // TODO: Integrate with cortex-prediction-mcp for real data
    let prediction_bets = fetch_prediction_bets(wallet_addr);
    
    // Build unified wallet entity
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
        Ok(conviction) => {
            let response = conviction_to_response(&conviction, &wallet);
            serde_json::to_value(response).map_err(|e| e.to_string())
        }
        Err(e) => {
            // Return a meaningful response even with limited data
            Ok(json!({
                "wallet": wallet_addr,
                "conviction_score": 0.0,
                "confidence": "low",
                "signals_count": 0,
                "signals": [],
                "interpretation": format!("Unable to calculate conviction: {}. This wallet may not have correlated DeFi and prediction market activity.", e),
                "defi_summary": {
                    "total_value_usd": wallet.total_value_usd,
                    "position_count": wallet.defi_positions.len(),
                    "protocols": wallet.protocols
                },
                "prediction_summary": {
                    "total_bet_usd": 0.0,
                    "bet_count": 0,
                    "platforms": [],
                    "categories": []
                }
            }))
        }
    }
}

/// Parse DeFi positions from API response
fn parse_defi_positions(data: &Value) -> Vec<DeFiPosition> {
    let positions = match data["positions"].as_array() {
        Some(arr) => arr,
        None => return vec![],
    };
    
    positions.iter().filter_map(|p| {
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
            token_symbol: p["token_symbol"].as_str()
                .or_else(|| p["token"].as_str())
                .unwrap_or("UNKNOWN").to_string(),
            amount: p["amount"].as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| p["amount"].as_f64())
                .unwrap_or(0.0),
            usd_value: p["usd_value"].as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| p["usd_value"].as_f64())
                .unwrap_or(0.0),
            entry_price: p["entry_price"].as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| p["entry_price"].as_f64()),
            current_price: p["current_price"].as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| p["current_price"].as_f64())
                .unwrap_or(0.0),
            unrealized_pnl: p["unrealized_pnl"].as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| p["unrealized_pnl"].as_f64())
                .unwrap_or(0.0),
            opened_at: Utc::now(), // TODO: Parse from data
            updated_at: Utc::now(),
            metadata: None,
        })
    }).collect()
}

/// Fetch prediction market bets for a wallet
/// TODO: Replace with actual integration to cortex-prediction-mcp
fn fetch_prediction_bets(wallet_addr: &str) -> Vec<PredictionMarketBet> {
    // For demo purposes, return empty unless we have actual data
    // In production, this would query the prediction market database
    
    // Check environment for demo mode
    if std::env::var("CORTEX_DEMO_MODE").is_ok() {
        // Return sample data for demonstration
        return vec![
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
        ];
    }
    
    vec![]
}

/// Execute informed traders detection
fn execute_informed_traders(
    _api_url: &str,
    market_slug: &str,
    platform: &str,
    min_conviction: f64,
) -> Result<Value, String> {
    // TODO: Implement full informed trader detection
    // This requires:
    // 1. Query prediction market for all bettors on this market
    // 2. For each bettor, fetch their DeFi positions
    // 3. Calculate conviction scores
    // 4. Filter by min_conviction and aggregate
    
    // For now, return a placeholder that explains the feature
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

// ============================================================================
// MCP Message Handlers
// ============================================================================

fn handle_initialize(_params: &Value) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "cortex-mcp",
            "version": "0.1.0"
        }
    })
}

fn handle_list_tools() -> Value {
    json!({
        "tools": get_tools()
    })
}

fn handle_call_tool(params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    match execute_tool(name, args) {
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

fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => Some(handle_initialize(&request.params)),
        "initialized" => None, // Notification, no response needed
        "tools/list" => Some(handle_list_tools()),
        "tools/call" => Some(handle_call_tool(&request.params)),
        _ => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            };
        }
    };

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id,
        result,
        error: None,
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    eprintln!("Cortex MCP server starting...");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                continue;
            }
        };

        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error parsing request: {}", e);
                continue;
            }
        };

        // Skip notifications (methods that don't need a response)
        if request.method == "initialized" || request.method == "notifications/cancelled" {
            continue;
        }

        let response = handle_request(request);

        let response_str = serde_json::to_string(&response).unwrap_or_default();
        if let Err(e) = writeln!(stdout, "{}", response_str) {
            eprintln!("Error writing response: {}", e);
        }
        if let Err(e) = stdout.flush() {
            eprintln!("Error flushing stdout: {}", e);
        }
    }
}
