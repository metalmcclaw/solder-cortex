pub mod dto;
pub mod handlers;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // User endpoints
        .route("/api/v1/user/{wallet}/summary", get(handlers::get_user_summary))
        .route("/api/v1/user/{wallet}/pnl", get(handlers::get_user_pnl))
        .route("/api/v1/user/{wallet}/positions", get(handlers::get_user_positions))
        // Indexing subscription endpoints
        .route("/api/v1/index", get(handlers::list_subscriptions))
        .route("/api/v1/index", post(handlers::index_wallet))
        .route("/api/v1/index/{wallet}", delete(handlers::stop_indexing))
}
