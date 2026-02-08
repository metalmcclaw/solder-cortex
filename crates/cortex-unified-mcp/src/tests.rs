//! Integration tests for the Cortex Unified MCP Server
//!
//! Tests JSON-RPC protocol compliance, tool execution, error handling,
//! and configuration loading.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use std::sync::Arc;

    use crate::config::{AppConfig, CacheConfig, DefiConfig, PredictionConfig};
    use crate::defi::DefiClient;
    use crate::error::{detect_address_type, validate_interval, validate_slug, validate_wallet, AddressType};
    use crate::tools::{handle_request, CortexTools, JsonRpcRequest, Tool};

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn create_test_tools() -> Arc<CortexTools> {
        let defi_config = DefiConfig {
            api_url: "http://localhost:3000".to_string(),
            timeout_seconds: 5,
        };
        let defi_client = Arc::new(DefiClient::new(&defi_config));
        Arc::new(CortexTools::new(defi_client, None))
    }

    fn make_request(id: i32, method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(id),
            method: method.to_string(),
            params,
        }
    }

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();

        // DeFi config defaults
        assert!(!config.defi.api_url.is_empty());
        assert!(config.defi.timeout_seconds > 0);

        // Prediction config defaults
        assert!(!config.prediction.clickhouse_url.is_empty());
        assert!(!config.prediction.database.is_empty());

        // Cache config defaults
        assert!(config.cache.max_capacity > 0);
        assert!(config.cache.ttl_seconds > 0);
    }

    #[test]
    fn test_defi_config_defaults() {
        let config = DefiConfig::default();
        assert_eq!(config.timeout_seconds, 30);
        // API URL should default to localhost:3000 or env var
        assert!(config.api_url.contains("localhost") || config.api_url.contains("127.0.0.1") || !config.api_url.is_empty());
    }

    #[test]
    fn test_prediction_config_defaults() {
        let config = PredictionConfig::default();
        assert!(!config.clickhouse_url.is_empty());
        assert_eq!(config.database, "cortex");
    }

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert_eq!(config.max_capacity, 1000);
        assert_eq!(config.ttl_seconds, 300); // 5 minutes
    }

    // =========================================================================
    // Address Validation Tests
    // =========================================================================

    #[test]
    fn test_detect_address_type_solana() {
        // Valid Solana addresses (32-44 base58 chars)
        let valid_addresses = vec![
            "So11111111111111111111111111111111111111112",
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        ];

        for addr in valid_addresses {
            assert_eq!(
                detect_address_type(addr),
                AddressType::Solana,
                "Expected Solana for {}",
                addr
            );
        }
    }

    #[test]
    fn test_detect_address_type_evm() {
        // Valid EVM addresses (0x + 40 hex chars)
        let valid_addresses = vec![
            "0x1234567890123456789012345678901234567890",
            "0xdead000000000000000000000000000000000000",
            "0xABCDEF1234567890ABCDEF1234567890ABCDEF12",
        ];

        for addr in valid_addresses {
            assert_eq!(
                detect_address_type(addr),
                AddressType::Evm,
                "Expected EVM for {}",
                addr
            );
        }
    }

    #[test]
    fn test_detect_address_type_unknown() {
        let invalid_addresses = vec![
            "",
            "invalid",
            "0x123", // Too short for EVM
            "short", // Too short for Solana
        ];

        for addr in invalid_addresses {
            assert_eq!(
                detect_address_type(addr),
                AddressType::Unknown,
                "Expected Unknown for '{}'",
                addr
            );
        }
    }

    #[test]
    fn test_validate_wallet_valid() {
        let valid_wallets = vec![
            "So11111111111111111111111111111111111111112",
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        ];

        for wallet in valid_wallets {
            assert!(validate_wallet(wallet).is_ok(), "Should be valid: {}", wallet);
        }
    }

    #[test]
    fn test_validate_wallet_invalid() {
        // Empty
        assert!(validate_wallet("").is_err());

        // Too short
        assert!(validate_wallet("short").is_err());

        // Too long (> 44 chars)
        let too_long = "A".repeat(50);
        assert!(validate_wallet(&too_long).is_err());
    }

    #[test]
    fn test_validate_slug_valid() {
        assert!(validate_slug("will-trump-win-2024").is_ok());
        assert!(validate_slug("eth-price-above-5000").is_ok());
        assert!(validate_slug("a").is_ok());
    }

    #[test]
    fn test_validate_slug_invalid() {
        // Empty
        assert!(validate_slug("").is_err());

        // Too long
        let too_long = "a".repeat(300);
        assert!(validate_slug(&too_long).is_err());
    }

    #[test]
    fn test_validate_interval_valid() {
        let valid_intervals = vec!["1m", "5m", "15m", "30m", "1h", "4h", "24h", "7d"];
        for interval in valid_intervals {
            assert!(
                validate_interval(interval).is_ok(),
                "Should be valid: {}",
                interval
            );
        }
    }

    #[test]
    fn test_validate_interval_invalid() {
        let invalid_intervals = vec!["2m", "10m", "2h", "1d", "invalid", ""];
        for interval in invalid_intervals {
            assert!(
                validate_interval(interval).is_err(),
                "Should be invalid: {}",
                interval
            );
        }
    }

    // =========================================================================
    // JSON-RPC Protocol Tests
    // =========================================================================

    #[tokio::test]
    async fn test_jsonrpc_initialize() {
        let tools = create_test_tools();
        let request = make_request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(1));
        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "cortex-unified-mcp");
        assert_eq!(result["serverInfo"]["version"], "0.2.0");
    }

    #[tokio::test]
    async fn test_jsonrpc_initialized_notification() {
        let tools = create_test_tools();
        let request = make_request(2, "initialized", json!({}));

        let response = handle_request(&tools, request).await;
        // Notifications don't get responses
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_jsonrpc_tools_list() {
        let tools = create_test_tools();
        let request = make_request(3, "tools/list", json!({}));

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(3));
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let tools_array = result["tools"].as_array().expect("tools should be array");
        assert!(!tools_array.is_empty());

        // Verify tool structure
        for tool in tools_array {
            assert!(tool["name"].is_string());
            assert!(tool["description"].is_string());
            assert!(tool["inputSchema"].is_object());
        }
    }

    #[tokio::test]
    async fn test_jsonrpc_method_not_found() {
        let tools = create_test_tools();
        let request = make_request(4, "nonexistent/method", json!({}));

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601); // Method not found
        assert!(error.message.contains("Method not found"));
    }

    #[tokio::test]
    async fn test_jsonrpc_cancelled_notification() {
        let tools = create_test_tools();
        let request = make_request(5, "notifications/cancelled", json!({}));

        let response = handle_request(&tools, request).await;
        // Notifications don't get responses
        assert!(response.is_none());
    }

    // =========================================================================
    // Tool Execution Tests
    // =========================================================================

    #[tokio::test]
    async fn test_tool_cortex_health() {
        let tools = create_test_tools();
        let request = make_request(
            10,
            "tools/call",
            json!({
                "name": "cortex_health",
                "arguments": {}
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let content = result["content"].as_array().expect("content array");
        assert!(!content.is_empty());
        assert_eq!(content[0]["type"], "text");

        // Parse the text content to verify health response structure
        let text = content[0]["text"].as_str().unwrap();
        let health: Value = serde_json::from_str(text).expect("health should be valid JSON");
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["version"], "0.2.0");
        assert!(health["defi"].is_object());
        assert!(health["prediction"].is_object());
    }

    #[tokio::test]
    async fn test_tool_unknown_tool() {
        let tools = create_test_tools();
        let request = make_request(
            11,
            "tools/call",
            json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        let result = response.result.unwrap();
        assert!(result["isError"].as_bool().unwrap_or(false));
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_tool_missing_required_param() {
        let tools = create_test_tools();

        // cortex_get_wallet_summary requires "wallet" parameter
        let request = make_request(
            12,
            "tools/call",
            json!({
                "name": "cortex_get_wallet_summary",
                "arguments": {}
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        let result = response.result.unwrap();
        assert!(result["isError"].as_bool().unwrap_or(false));
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Missing wallet") || content.contains("Error"));
    }

    #[tokio::test]
    async fn test_tool_invalid_wallet_format() {
        let tools = create_test_tools();

        let request = make_request(
            13,
            "tools/call",
            json!({
                "name": "cortex_get_wallet_summary",
                "arguments": {
                    "wallet": "invalid"  // Too short
                }
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        let result = response.result.unwrap();
        assert!(result["isError"].as_bool().unwrap_or(false));
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Invalid") || content.contains("Error"));
    }

    // =========================================================================
    // Tool Definition Tests
    // =========================================================================

    #[test]
    fn test_tool_definitions_structure() {
        let tools = create_test_tools();
        let tool_list = tools.get_tools();

        assert!(!tool_list.is_empty(), "Should have at least some tools");

        // Core tools that should always be present
        let expected_tools = vec![
            "cortex_health",
            "cortex_get_wallet_summary",
            "cortex_get_wallet_pnl",
            "cortex_get_wallet_positions",
            "cortex_start_indexing",
            "cortex_stop_indexing",
            "cortex_list_subscriptions",
            "cortex_get_wallet_conviction",
            "cortex_detect_informed_traders",
        ];

        let tool_names: Vec<&str> = tool_list.iter().map(|t| t.name.as_str()).collect();

        for expected in expected_tools {
            assert!(
                tool_names.contains(&expected),
                "Missing expected tool: {}",
                expected
            );
        }
    }

    #[test]
    fn test_tool_input_schemas_valid() {
        let tools = create_test_tools();
        let tool_list = tools.get_tools();

        for tool in tool_list {
            // Every tool should have a valid JSON schema
            assert!(
                tool.input_schema["type"].is_string(),
                "Tool {} should have type in schema",
                tool.name
            );
            assert!(
                tool.input_schema["properties"].is_object(),
                "Tool {} should have properties in schema",
                tool.name
            );
            assert!(
                tool.input_schema["required"].is_array(),
                "Tool {} should have required array in schema",
                tool.name
            );
        }
    }

    #[test]
    fn test_tool_descriptions_not_empty() {
        let tools = create_test_tools();
        let tool_list = tools.get_tools();

        for tool in tool_list {
            assert!(
                !tool.description.is_empty(),
                "Tool {} should have a description",
                tool.name
            );
            // Descriptions should be meaningful (at least 20 chars)
            assert!(
                tool.description.len() >= 20,
                "Tool {} description too short: '{}'",
                tool.name,
                tool.description
            );
        }
    }

    // =========================================================================
    // Response Format Tests
    // =========================================================================

    #[tokio::test]
    async fn test_response_format_success() {
        let tools = create_test_tools();
        let request = make_request(20, "tools/list", json!({}));

        let response = handle_request(&tools, request).await.unwrap();

        // Verify JSON-RPC 2.0 structure
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(20));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_response_format_error() {
        let tools = create_test_tools();
        let request = make_request(21, "invalid/method", json!({}));

        let response = handle_request(&tools, request).await.unwrap();

        // Verify JSON-RPC 2.0 error structure
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(21));
        assert!(response.result.is_none() || response.error.is_some());

        if let Some(error) = response.error {
            assert!(error.code != 0);
            assert!(!error.message.is_empty());
        }
    }

    #[tokio::test]
    async fn test_tool_call_content_structure() {
        let tools = create_test_tools();
        let request = make_request(
            22,
            "tools/call",
            json!({
                "name": "cortex_health",
                "arguments": {}
            }),
        );

        let response = handle_request(&tools, request).await.unwrap();
        let result = response.result.unwrap();

        // MCP tools/call should return content array
        assert!(result["content"].is_array());
        let content = result["content"].as_array().unwrap();
        assert!(!content.is_empty());

        // Each content item should have type and text
        for item in content {
            assert!(item["type"].is_string());
            assert!(item["text"].is_string());
        }
    }

    // =========================================================================
    // Cross-Domain Intelligence Tests
    // =========================================================================

    #[tokio::test]
    async fn test_tool_conviction_with_evm_address() {
        let tools = create_test_tools();
        
        // Test with EVM address (Polymarket uses EVM)
        let request = make_request(
            30,
            "tools/call",
            json!({
                "name": "cortex_get_wallet_conviction",
                "arguments": {
                    "wallet": "0x1234567890123456789012345678901234567890"
                }
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        // Should handle EVM address without crashing
        let response = response.unwrap();
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_tool_conviction_with_linked_evm() {
        let tools = create_test_tools();
        
        // Test Solana wallet with linked EVM address
        let request = make_request(
            31,
            "tools/call",
            json!({
                "name": "cortex_get_wallet_conviction",
                "arguments": {
                    "wallet": "So11111111111111111111111111111111111111112",
                    "evm_address": "0x1234567890123456789012345678901234567890"
                }
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        let response = response.unwrap();
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_tool_detect_informed_traders() {
        let tools = create_test_tools();
        
        let request = make_request(
            32,
            "tools/call",
            json!({
                "name": "cortex_detect_informed_traders",
                "arguments": {
                    "market_slug": "test-market-slug",
                    "platform": "polymarket",
                    "min_conviction": 0.5
                }
            }),
        );

        let response = handle_request(&tools, request).await;
        assert!(response.is_some());

        // Should return valid response even if market doesn't exist
        let response = response.unwrap();
        assert!(response.error.is_none());
    }

    // =========================================================================
    // Concurrency Tests
    // =========================================================================

    #[tokio::test]
    async fn test_concurrent_requests() {
        let tools = create_test_tools();
        let tools = Arc::clone(&tools);

        let mut handles = vec![];

        for i in 0..10 {
            let tools_clone = Arc::clone(&tools);
            let handle = tokio::spawn(async move {
                let request = make_request(100 + i, "tools/list", json!({}));
                handle_request(&tools_clone, request).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let response = handle.await.expect("task should complete");
            assert!(response.is_some());
            let response = response.unwrap();
            assert!(response.error.is_none());
        }
    }
}
