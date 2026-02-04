use chrono::{Duration, Utc};
use clickhouse::Client;
use std::time::Instant;

use super::models::{PnlByProtocolRow, PositionRow, WalletSummaryRow};
use crate::error::AppResult;
use crate::types::TimeWindow;

pub async fn get_wallet_summary(client: &Client, wallet: &str) -> AppResult<Option<WalletSummaryRow>> {
    let start = Instant::now();
    tracing::debug!(wallet = %wallet, query = "get_wallet_summary", "Executing database query");

    let query = r#"
        SELECT
            wallet,
            total_value_usd,
            realized_pnl_24h,
            realized_pnl_7d,
            realized_pnl_30d,
            unrealized_pnl,
            largest_position_pct,
            protocol_count,
            position_count,
            risk_score,
            last_activity,
            protocols
        FROM wallet_summaries FINAL
        WHERE wallet = ?
    "#;

    let result: Vec<WalletSummaryRow> = client
        .query(query)
        .bind(wallet)
        .fetch_all()
        .await?;

    let found = result.len() > 0;
    tracing::debug!(
        wallet = %wallet,
        query = "get_wallet_summary",
        duration_ms = %start.elapsed().as_millis(),
        found = %found,
        "Database query completed"
    );

    Ok(result.into_iter().next())
}

pub async fn get_wallet_positions(client: &Client, wallet: &str) -> AppResult<Vec<PositionRow>> {
    let start = Instant::now();
    tracing::debug!(wallet = %wallet, query = "get_wallet_positions", "Executing database query");

    let query = r#"
        SELECT
            wallet,
            protocol,
            position_type,
            token,
            pool,
            amount,
            entry_price,
            current_price,
            usd_value,
            unrealized_pnl,
            apy
        FROM positions FINAL
        WHERE wallet = ?
        ORDER BY usd_value DESC
    "#;

    let positions: Vec<PositionRow> = client
        .query(query)
        .bind(wallet)
        .fetch_all()
        .await?;

    tracing::debug!(
        wallet = %wallet,
        query = "get_wallet_positions",
        duration_ms = %start.elapsed().as_millis(),
        row_count = %positions.len(),
        "Database query completed"
    );

    Ok(positions)
}

pub async fn get_wallet_pnl_by_protocol(
    client: &Client,
    wallet: &str,
    window: TimeWindow,
) -> AppResult<Vec<PnlByProtocolRow>> {
    let start = Instant::now();
    tracing::debug!(
        wallet = %wallet,
        window = ?window,
        query = "get_wallet_pnl_by_protocol",
        "Executing database query"
    );

    let time_filter = match window.to_days() {
        Some(days) => {
            let cutoff = Utc::now() - Duration::days(days);
            tracing::debug!(cutoff = %cutoff, days = %days, "Applying time filter");
            format!("AND block_time >= toDateTime64('{}', 3)", cutoff.format("%Y-%m-%d %H:%M:%S"))
        }
        None => {
            tracing::debug!("No time filter applied (all time)");
            String::new()
        }
    };

    let query = format!(
        r#"
        SELECT
            protocol,
            sum(
                CASE
                    WHEN tx_type IN ('swap', 'remove_liquidity', 'withdraw')
                    THEN usd_value
                    ELSE 0
                END
            ) - sum(
                CASE
                    WHEN tx_type IN ('add_liquidity', 'deposit', 'borrow')
                    THEN usd_value
                    ELSE 0
                END
            ) as realized,
            0 as unrealized,
            count(*) as trade_count
        FROM transactions
        WHERE wallet = ?
        {}
        GROUP BY protocol
        ORDER BY realized DESC
        "#,
        time_filter
    );

    let results: Vec<PnlByProtocolRow> = client
        .query(&query)
        .bind(wallet)
        .fetch_all()
        .await?;

    tracing::debug!(
        wallet = %wallet,
        query = "get_wallet_pnl_by_protocol",
        duration_ms = %start.elapsed().as_millis(),
        protocol_count = %results.len(),
        "Database query completed"
    );

    Ok(results)
}

pub async fn upsert_wallet_summary(client: &Client, summary: &WalletSummaryRow) -> AppResult<()> {
    let start = Instant::now();
    tracing::debug!(
        wallet = %summary.wallet,
        query = "upsert_wallet_summary",
        "Executing database insert"
    );

    let query = r#"
        INSERT INTO wallet_summaries (
            wallet, total_value_usd, realized_pnl_24h, realized_pnl_7d, realized_pnl_30d,
            unrealized_pnl, largest_position_pct, protocol_count, position_count,
            risk_score, last_activity, protocols, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, now64(3))
    "#;

    client
        .query(query)
        .bind(&summary.wallet)
        .bind(&summary.total_value_usd)
        .bind(&summary.realized_pnl_24h)
        .bind(&summary.realized_pnl_7d)
        .bind(&summary.realized_pnl_30d)
        .bind(&summary.unrealized_pnl)
        .bind(&summary.largest_position_pct)
        .bind(summary.protocol_count)
        .bind(summary.position_count)
        .bind(summary.risk_score)
        .bind(summary.last_activity)
        .bind(&summary.protocols)
        .execute()
        .await?;

    tracing::debug!(
        wallet = %summary.wallet,
        query = "upsert_wallet_summary",
        duration_ms = %start.elapsed().as_millis(),
        "Database insert completed"
    );

    Ok(())
}

pub async fn upsert_position(client: &Client, position: &PositionRow) -> AppResult<()> {
    let start = Instant::now();
    tracing::debug!(
        wallet = %position.wallet,
        protocol = %position.protocol,
        token = %position.token,
        query = "upsert_position",
        "Executing database insert"
    );

    let query = r#"
        INSERT INTO positions (
            wallet, protocol, position_type, token, pool, amount,
            entry_price, current_price, usd_value, unrealized_pnl, apy, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, now64(3))
    "#;

    client
        .query(query)
        .bind(&position.wallet)
        .bind(&position.protocol)
        .bind(&position.position_type)
        .bind(&position.token)
        .bind(&position.pool)
        .bind(&position.amount)
        .bind(&position.entry_price)
        .bind(&position.current_price)
        .bind(&position.usd_value)
        .bind(&position.unrealized_pnl)
        .bind(&position.apy)
        .execute()
        .await?;

    tracing::debug!(
        wallet = %position.wallet,
        query = "upsert_position",
        duration_ms = %start.elapsed().as_millis(),
        "Database insert completed"
    );

    Ok(())
}

pub async fn wallet_exists(client: &Client, wallet: &str) -> AppResult<bool> {
    let start = Instant::now();
    tracing::debug!(wallet = %wallet, query = "wallet_exists", "Executing database query");

    let query = r#"
        SELECT count(*) as cnt FROM wallet_summaries FINAL WHERE wallet = ?
    "#;

    #[derive(clickhouse::Row, serde::Deserialize)]
    struct CountResult {
        cnt: u64,
    }

    let result: Vec<CountResult> = client
        .query(query)
        .bind(wallet)
        .fetch_all()
        .await?;

    let exists = result.first().map(|r| r.cnt > 0).unwrap_or(false);

    tracing::debug!(
        wallet = %wallet,
        query = "wallet_exists",
        duration_ms = %start.elapsed().as_millis(),
        exists = %exists,
        "Database query completed"
    );

    Ok(exists)
}
