//! Cortex MCP Server
//!
//! MCP (Model Context Protocol) server that exposes Cortex APIs as tools
//! for AI assistants like Claude.

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

        _ => Err(format!("Unknown tool: {}", name)),
    }
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
