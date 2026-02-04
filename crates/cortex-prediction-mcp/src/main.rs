//! Cortex Prediction Market MCP Server
//!
//! MCP server that provides prediction market data tools for AI agents.
//! Connects directly to Clickhouse for efficient querying of market data.
//! Implements the MCP protocol using pure JSON-RPC over stdio.

mod cache;
mod config;
mod db;
mod error;
mod tools;

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use tokio::runtime::Runtime;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use cache::create_cache;
use config::AppConfig;
use db::create_query_engine;
use tools::{handle_request, JsonRpcRequest, PredictionTools};

fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is for MCP protocol)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("cortex_prediction_mcp=info")
        }))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Starting Cortex Prediction Market MCP Server");

    // Create Tokio runtime
    let rt = Runtime::new()?;

    // Load configuration
    let config = rt.block_on(async {
        AppConfig::load().map_err(|e| {
            tracing::error!(error = %e, "Failed to load configuration");
            e
        })
    })?;

    tracing::info!(
        database_url = %config.database.url,
        database = %config.database.database,
        cache_capacity = %config.cache.max_capacity,
        cache_ttl = %config.cache.ttl_seconds,
        "Configuration loaded"
    );

    // Initialize components
    let query_engine = Arc::new(create_query_engine(&config.database));
    let cache = create_cache(&config.cache);

    // Verify database connection
    rt.block_on(async {
        if let Err(e) = query_engine.health_check().await {
            tracing::error!(error = %e, "Failed to connect to Clickhouse");
            return Err(anyhow::anyhow!("Database connection failed: {}", e));
        }
        Ok(())
    })?;
    tracing::info!("Clickhouse connection verified");

    // Create tools instance
    let tools = Arc::new(PredictionTools::new(query_engine, cache));

    tracing::info!("MCP server ready, listening on stdio");

    // Main loop: read JSON-RPC requests from stdin, write responses to stdout
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(error = %e, "Error reading stdin");
                continue;
            }
        };

        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "Error parsing request");
                continue;
            }
        };

        tracing::debug!(method = %request.method, "Received request");

        // Handle the request asynchronously
        let tools_clone = Arc::clone(&tools);
        let response = rt.block_on(async move {
            handle_request(&tools_clone, request).await
        });

        // Only send response if one was produced (notifications don't get responses)
        if let Some(response) = response {
            let response_str = serde_json::to_string(&response).unwrap_or_default();
            if let Err(e) = writeln!(stdout, "{}", response_str) {
                tracing::error!(error = %e, "Error writing response");
            }
            if let Err(e) = stdout.flush() {
                tracing::error!(error = %e, "Error flushing stdout");
            }
        }
    }

    tracing::info!("MCP server shutting down");
    Ok(())
}
