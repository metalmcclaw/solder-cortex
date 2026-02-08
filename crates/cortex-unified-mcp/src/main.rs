//! Cortex Unified MCP Server
//!
//! The unified Model Context Protocol server for Solder Cortex, providing
//! cross-domain intelligence by combining DeFi analytics and prediction
//! market data into a single interface for AI agents.
//!
//! # Features
//!
//! - **DeFi Analytics**: Wallet summaries, PnL tracking, position monitoring
//! - **Prediction Markets**: Market trends, volume profiles, anomaly detection
//! - **Cross-Domain Intelligence**: Conviction scoring, informed trader detection
//!
//! # Architecture
//!
//! This server unifies the previously separate `cortex-mcp` and
//! `cortex-prediction-mcp` servers into a single entry point.

mod config;
mod defi;
mod error;
mod polymarket;
mod prediction;
mod tools;

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use tokio::runtime::Runtime;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::AppConfig;
use crate::defi::DefiClient;
use crate::prediction::PredictionEngine;
use crate::tools::{handle_request, CortexTools, JsonRpcRequest};

fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is for MCP protocol)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("cortex_unified_mcp=info")
        }))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Starting Cortex Unified MCP Server v0.2.0");

    // Create Tokio runtime
    let rt = Runtime::new()?;

    // Load configuration
    let config = rt.block_on(async {
        AppConfig::load().await.map_err(|e| {
            tracing::warn!(error = %e, "Using default configuration");
            e
        })
    }).unwrap_or_default();

    tracing::info!(
        defi_api_url = %config.defi.api_url,
        prediction_enabled = config.prediction.enabled,
        "Configuration loaded"
    );

    // Initialize DeFi client
    let defi_client = Arc::new(DefiClient::new(&config.defi));

    // Initialize Prediction Engine (optional - may not have Clickhouse)
    let prediction_engine = if config.prediction.enabled {
        match rt.block_on(async { PredictionEngine::new(&config.prediction).await }) {
            Ok(engine) => {
                tracing::info!("Prediction market engine initialized");
                Some(Arc::new(engine))
            }
            Err(e) => {
                tracing::warn!(error = %e, "Prediction engine unavailable - running in DeFi-only mode");
                None
            }
        }
    } else {
        tracing::info!("Prediction engine disabled by configuration");
        None
    };

    // Create unified tools instance
    let tools = Arc::new(CortexTools::new(defi_client, prediction_engine));

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
                tracing::error!(error = %e, line = %line, "Error parsing request");
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
