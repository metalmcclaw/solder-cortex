use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;
use std::time::Instant;

use super::dto::*;
use crate::db::queries;
use crate::error::{AppError, AppResult};
use crate::types::{validate_solana_address, TimeWindow};
use crate::AppState;

/// Helper to parse string to Decimal, defaulting to zero on error
fn parse_decimal(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_default()
}

pub async fn health_check(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /health");
    tracing::info!("Processing health check request");

    let db_status = match state.db.health_check().await {
        Ok(_) => {
            tracing::debug!("Database health check passed");
            "connected"
        }
        Err(e) => {
            tracing::warn!(error = %e, "Database health check failed");
            "disconnected"
        }
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status.to_string(),
    };

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /health -> 200 OK ({}ms) db={}", duration, db_status);
    tracing::info!(
        duration_ms = %duration,
        db_status = %db_status,
        "Health check completed"
    );

    Ok(Json(response))
}

pub async fn get_user_summary(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> AppResult<Json<UserSummaryResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v1/user/{}/summary", wallet);
    tracing::info!(wallet = %wallet, "Processing user summary request");

    // Validate wallet address
    if !validate_solana_address(&wallet) {
        println!("[RESPONSE] GET /api/v1/user/{}/summary -> 400 Bad Request (invalid wallet)", wallet);
        tracing::warn!(wallet = %wallet, "Invalid wallet address provided");
        return Err(AppError::InvalidWallet(wallet));
    }
    tracing::debug!(wallet = %wallet, "Wallet address validated");

    // Try to fetch from database
    println!("[DB] Querying wallet summary for {}", wallet);
    tracing::debug!(wallet = %wallet, "Querying database for wallet summary");
    if let Some(summary) = queries::get_wallet_summary(state.db.client(), &wallet).await? {
        let duration = start.elapsed().as_millis();
        println!("[RESPONSE] GET /api/v1/user/{}/summary -> 200 OK ({}ms) value=${}",
            wallet, duration, summary.total_value_usd);
        tracing::info!(
            wallet = %wallet,
            duration_ms = %duration,
            total_value_usd = %summary.total_value_usd,
            protocol_count = %summary.protocol_count,
            "User summary retrieved from database"
        );
        return Ok(Json(summary.into()));
    }

    // If not in DB, return placeholder data
    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v1/user/{}/summary -> 200 OK ({}ms) [not indexed yet]", wallet, duration);
    tracing::info!(
        wallet = %wallet,
        duration_ms = %duration,
        "Wallet not found in database, returning placeholder"
    );

    Ok(Json(UserSummaryResponse {
        wallet: wallet.clone(),
        total_value_usd: dec!(0),
        pnl: PnlSummary {
            realized_24h: dec!(0),
            realized_7d: dec!(0),
            realized_30d: dec!(0),
            unrealized: dec!(0),
        },
        risk: RiskSummary {
            score: 0,
            largest_position_pct: dec!(0),
            protocol_count: 0,
        },
        last_activity: chrono::Utc::now(),
        protocols: vec![],
    }))
}

pub async fn get_user_pnl(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
    Query(query): Query<PnlQuery>,
) -> AppResult<Json<UserPnlResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v1/user/{}/pnl?window={}", wallet, query.window);
    tracing::info!(wallet = %wallet, window = %query.window, "Processing user PnL request");

    // Validate wallet address
    if !validate_solana_address(&wallet) {
        println!("[RESPONSE] GET /api/v1/user/{}/pnl -> 400 Bad Request (invalid wallet)", wallet);
        tracing::warn!(wallet = %wallet, "Invalid wallet address provided");
        return Err(AppError::InvalidWallet(wallet));
    }
    tracing::debug!(wallet = %wallet, "Wallet address validated");

    // Parse time window
    let window = TimeWindow::from_str(&query.window)
        .ok_or_else(|| {
            println!("[RESPONSE] GET /api/v1/user/{}/pnl -> 400 Bad Request (invalid window)", wallet);
            tracing::warn!(wallet = %wallet, window = %query.window, "Invalid time window parameter");
            AppError::InvalidParam(format!("Invalid window: {}", query.window))
        })?;
    tracing::debug!(wallet = %wallet, window = ?window, "Time window parsed");

    // Query PnL data
    println!("[DB] Querying PnL for {} (window={})", wallet, query.window);
    tracing::debug!(wallet = %wallet, "Querying database for PnL by protocol");
    let pnl_data = queries::get_wallet_pnl_by_protocol(state.db.client(), &wallet, window).await?;

    let total_realized: Decimal = pnl_data.iter().map(|p| parse_decimal(&p.realized)).sum();
    let total_unrealized: Decimal = pnl_data.iter().map(|p| parse_decimal(&p.unrealized)).sum();
    let protocol_count = pnl_data.len();

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v1/user/{}/pnl -> 200 OK ({}ms) realized=${} unrealized=${}",
        wallet, duration, total_realized, total_unrealized);
    tracing::info!(
        wallet = %wallet,
        window = %query.window,
        duration_ms = %duration,
        protocol_count = %protocol_count,
        total_realized = %total_realized,
        total_unrealized = %total_unrealized,
        "User PnL retrieved successfully"
    );

    Ok(Json(UserPnlResponse {
        wallet,
        window: query.window,
        total_realized,
        total_unrealized,
        by_protocol: pnl_data.into_iter().map(Into::into).collect(),
    }))
}

pub async fn get_user_positions(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> AppResult<Json<UserPositionsResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v1/user/{}/positions", wallet);
    tracing::info!(wallet = %wallet, "Processing user positions request");

    // Validate wallet address
    if !validate_solana_address(&wallet) {
        println!("[RESPONSE] GET /api/v1/user/{}/positions -> 400 Bad Request (invalid wallet)", wallet);
        tracing::warn!(wallet = %wallet, "Invalid wallet address provided");
        return Err(AppError::InvalidWallet(wallet));
    }
    tracing::debug!(wallet = %wallet, "Wallet address validated");

    // Query positions
    println!("[DB] Querying positions for {}", wallet);
    tracing::debug!(wallet = %wallet, "Querying database for wallet positions");
    let positions = queries::get_wallet_positions(state.db.client(), &wallet).await?;

    let total_value: Decimal = positions.iter().map(|p| parse_decimal(&p.usd_value)).sum();
    let position_count = positions.len();

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v1/user/{}/positions -> 200 OK ({}ms) positions={} value=${}",
        wallet, duration, position_count, total_value);
    tracing::info!(
        wallet = %wallet,
        duration_ms = %duration,
        position_count = %position_count,
        total_value_usd = %total_value,
        "User positions retrieved successfully"
    );

    Ok(Json(UserPositionsResponse {
        wallet,
        positions: positions.into_iter().map(Into::into).collect(),
        total_value_usd: total_value,
    }))
}

/// Start continuous indexing for a wallet
pub async fn index_wallet(
    State(state): State<AppState>,
    Json(request): Json<IndexWalletRequest>,
) -> AppResult<Json<IndexWalletResponse>> {
    let start = Instant::now();
    println!("[REQUEST] POST /api/v1/index wallet={}", request.wallet);
    tracing::info!(wallet = %request.wallet, "Processing wallet indexing request");

    // Validate wallet address
    if !validate_solana_address(&request.wallet) {
        println!("[RESPONSE] POST /api/v1/index -> 400 Bad Request (invalid wallet)");
        tracing::warn!(wallet = %request.wallet, "Invalid wallet address provided for indexing");
        return Err(AppError::InvalidWallet(request.wallet));
    }
    tracing::debug!(wallet = %request.wallet, "Wallet address validated for indexing");

    // Start continuous subscription
    let wallet = request.wallet.clone();
    println!("[INDEXER] Starting continuous indexing for {}", wallet);
    tracing::info!(wallet = %wallet, "Starting continuous wallet subscription");

    let was_started = state.indexer.start_subscription(&wallet).await?;

    let duration = start.elapsed().as_millis();

    let (status, message) = if was_started {
        println!("[RESPONSE] POST /api/v1/index -> 200 OK ({}ms) status=started", duration);
        tracing::info!(wallet = %wallet, duration_ms = %duration, "Continuous indexing started");
        (
            "started".to_string(),
            "Continuous wallet indexing started. Transactions will be indexed in real-time.".to_string(),
        )
    } else {
        println!("[RESPONSE] POST /api/v1/index -> 200 OK ({}ms) status=already_running", duration);
        tracing::info!(wallet = %wallet, duration_ms = %duration, "Wallet already being indexed");
        (
            "already_running".to_string(),
            "Wallet is already being indexed continuously.".to_string(),
        )
    };

    Ok(Json(IndexWalletResponse {
        wallet: request.wallet,
        status,
        message,
    }))
}

/// Stop continuous indexing for a wallet
pub async fn stop_indexing(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> AppResult<Json<IndexWalletResponse>> {
    let start = Instant::now();
    println!("[REQUEST] DELETE /api/v1/index/{}", wallet);
    tracing::info!(wallet = %wallet, "Processing stop indexing request");

    // Validate wallet address
    if !validate_solana_address(&wallet) {
        println!("[RESPONSE] DELETE /api/v1/index/{} -> 400 Bad Request (invalid wallet)", wallet);
        tracing::warn!(wallet = %wallet, "Invalid wallet address provided");
        return Err(AppError::InvalidWallet(wallet));
    }

    let was_stopped = state.indexer.stop_subscription(&wallet).await;

    let duration = start.elapsed().as_millis();

    let (status, message) = if was_stopped {
        println!("[RESPONSE] DELETE /api/v1/index/{} -> 200 OK ({}ms) status=stopped", wallet, duration);
        tracing::info!(wallet = %wallet, duration_ms = %duration, "Continuous indexing stopped");
        (
            "stopped".to_string(),
            "Continuous wallet indexing stopped.".to_string(),
        )
    } else {
        println!("[RESPONSE] DELETE /api/v1/index/{} -> 200 OK ({}ms) status=not_running", wallet, duration);
        tracing::info!(wallet = %wallet, duration_ms = %duration, "No active subscription to stop");
        (
            "not_running".to_string(),
            "No active indexing subscription for this wallet.".to_string(),
        )
    };

    Ok(Json(IndexWalletResponse {
        wallet,
        status,
        message,
    }))
}

/// List all active indexing subscriptions
pub async fn list_subscriptions(
    State(state): State<AppState>,
) -> AppResult<Json<SubscriptionsResponse>> {
    let start = Instant::now();
    println!("[REQUEST] GET /api/v1/index");
    tracing::info!("Processing list subscriptions request");

    let subscriptions = state.indexer.list_subscriptions().await;

    let duration = start.elapsed().as_millis();
    println!("[RESPONSE] GET /api/v1/index -> 200 OK ({}ms) count={}", duration, subscriptions.len());
    tracing::info!(duration_ms = %duration, count = %subscriptions.len(), "Subscriptions listed");

    Ok(Json(SubscriptionsResponse {
        subscriptions: subscriptions.into_iter().map(|s| SubscriptionInfo {
            wallet: s.wallet,
            started_at: s.started_at,
            transactions_processed: s.transactions_processed,
            running: s.running,
        }).collect(),
    }))
}
