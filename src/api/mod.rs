pub mod dto;
pub mod handlers;
pub mod market_handlers;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Legacy v1 endpoints (wallet-focused - DEPRECATED)
        .route("/api/v1/user/{wallet}/summary", get(handlers::get_user_summary))
        .route("/api/v1/user/{wallet}/pnl", get(handlers::get_user_pnl))
        .route("/api/v1/user/{wallet}/positions", get(handlers::get_user_positions))
        // Legacy indexing subscription endpoints
        .route("/api/v1/index", get(handlers::list_subscriptions))
        .route("/api/v1/index", post(handlers::index_wallet))
        .route("/api/v1/index/{wallet}", delete(handlers::stop_indexing))
        // NEW v2 endpoints (market-wide focus)
        .route("/api/v2/market/overview", get(market_handlers::get_market_overview))
        .route("/api/v2/market/conviction", get(market_handlers::get_conviction_signals))
}
