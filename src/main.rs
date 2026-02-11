mod api;
mod config;
mod db;
mod error;
mod indexer;
mod metrics;
mod types;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;
use crate::db::Database;
use crate::indexer::{Indexer, market_data::MarketDataIndexer};

pub use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub indexer: Indexer,  // Legacy wallet-based indexer
    pub market_indexer: MarketDataIndexer,  // New market-wide indexer
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with pretty format
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cortex=info,tower_http=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
        )
        .init();

    println!("================================================");
    println!("         SOLDER CORTEX - Starting Up            ");
    println!("================================================");

    // Load configuration
    let config = AppConfig::load().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    println!("[CONFIG] Server: {}:{}", config.server.host, config.server.port);
    println!("[CONFIG] Database: {}", config.database.url);
    println!("[CONFIG] LYS Labs WebSocket: {}", config.lyslabs.ws_url);
    if config.lyslabs.api_key.is_empty() {
        println!("[CONFIG] LYS Labs API Key: *** EMPTY - PLEASE SET CORTEX__LYSLABS__API_KEY ***");
    } else {
        println!("[CONFIG] LYS Labs API Key: {}...{} (length: {})",
            &config.lyslabs.api_key[..4],
            &config.lyslabs.api_key[config.lyslabs.api_key.len()-4..],
            config.lyslabs.api_key.len());
    }
    if config.helius.api_key.is_empty() {
        println!("[CONFIG] Helius API Key: *** EMPTY - PLEASE SET CORTEX__HELIUS__API_KEY ***");
    } else {
        println!("[CONFIG] Helius API Key: {}...{} (length: {})",
            &config.helius.api_key[..4],
            &config.helius.api_key[config.helius.api_key.len()-4..],
            config.helius.api_key.len());
    }

    tracing::info!(
        host = %config.server.host,
        port = %config.server.port,
        "Starting Solder Cortex"
    );

    // Initialize database
    println!("[DB] Initializing ClickHouse connection...");
    let db = Database::new(&config.database);

    // Check database connection
    match db.health_check().await {
        Ok(_) => {
            println!("[DB] ClickHouse connected successfully");
            tracing::info!("Connected to ClickHouse");
        }
        Err(e) => {
            println!("[DB] WARNING: ClickHouse not available - {}", e);
            tracing::warn!(error = %e, "ClickHouse not available, running in degraded mode");
        }
    }

    // Initialize legacy wallet indexer (keeping for backward compatibility)
    println!("[INDEXER] Initializing legacy wallet indexer (LYS Labs + Helius)...");
    let indexer = Indexer::new(&config.lyslabs, &config.helius, db.clone());
    println!("[INDEXER] Legacy indexer ready");

    // Initialize market-wide data indexer (NEW APPROACH)
    println!("[MARKET] Initializing market-wide data indexer...");
    let market_indexer = MarketDataIndexer::new(db.clone());
    
    // Start market data aggregation
    if let Err(e) = market_indexer.start_market_data_aggregation().await {
        println!("[MARKET] WARNING: Failed to start market data aggregation - {}", e);
        tracing::warn!(error = %e, "Market data aggregation failed to start");
    } else {
        println!("[MARKET] Market data aggregation started successfully");
        tracing::info!("Market-wide data aggregation active");
    }

    // === AUTO-START WALLET INDEXING ===
    // Index notable wallets immediately on startup for demo purposes
    println!("[AUTO-INDEX] Starting auto-indexing for notable wallets...");
    
    let notable_wallets = vec![
        // Jupiter aggregator (high volume DeFi)
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
        // Raydium authority
        "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
        // Example whale wallet (for demo)
        "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
    ];
    
    for wallet in &notable_wallets {
        match indexer.start_subscription(wallet).await {
            Ok(true) => println!("[AUTO-INDEX] ✓ Started indexing {}", &wallet[..8]),
            Ok(false) => println!("[AUTO-INDEX] - Already indexing {}", &wallet[..8]),
            Err(e) => println!("[AUTO-INDEX] ✗ Failed to index {}: {}", &wallet[..8], e),
        }
    }
    println!("[AUTO-INDEX] Auto-indexing initialization complete");

    // Create app state
    let state = AppState {
        db,
        indexer,
        market_indexer,
        config: Arc::new(config.clone()),
    };

    // Build router
    println!("[ROUTER] Setting up API routes...");
    let app = Router::new()
        .merge(api::create_router())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    println!("[ROUTER] Routes configured: /health, /api/v1/user/{{wallet}}/*, /api/v1/index");

    // Start server
    let addr: SocketAddr = config.server_addr().parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("================================================");
    println!("  Server listening on http://{}", addr);
    println!("  API Docs: http://localhost:8080 (Swagger UI)");
    println!("================================================");
    println!();

    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
