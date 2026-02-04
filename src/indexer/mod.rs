pub mod helius;
pub mod lyslabs;
pub mod parser;
pub mod protocols;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use self::helius::{EnhancedTransaction, HeliusClient};
use self::lyslabs::{LysLabsClient, LysTransaction};
use self::parser::parse_transaction;
use crate::config::{HeliusConfig, LysLabsConfig};
use crate::db::models::WalletSummaryRow;
use crate::db::{queries, Database};
use crate::error::AppResult;
use crate::metrics;

/// Maximum historical transactions to fetch from Helius
const MAX_HISTORICAL_TRANSACTIONS: usize = 1000;

/// Info about an active wallet subscription
#[derive(Clone)]
pub struct WalletSubscription {
    pub wallet: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub transactions_processed: Arc<RwLock<u64>>,
    cancel_token: CancellationToken,
}

impl WalletSubscription {
    fn new(wallet: String, cancel_token: CancellationToken) -> Self {
        Self {
            wallet,
            started_at: chrono::Utc::now(),
            transactions_processed: Arc::new(RwLock::new(0)),
            cancel_token,
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub async fn tx_count(&self) -> u64 {
        *self.transactions_processed.read().await
    }
}

/// Subscription status for API responses
#[derive(Clone, serde::Serialize)]
pub struct SubscriptionStatus {
    pub wallet: String,
    pub started_at: String,
    pub transactions_processed: u64,
    pub running: bool,
}

#[derive(Clone)]
pub struct Indexer {
    lyslabs: LysLabsClient,
    helius: HeliusClient,
    db: Database,
    /// Active wallet subscriptions
    subscriptions: Arc<RwLock<HashMap<String, WalletSubscription>>>,
}

impl Indexer {
    pub fn new(lyslabs_config: &LysLabsConfig, helius_config: &HeliusConfig, db: Database) -> Self {
        tracing::debug!("Initializing Indexer with LYS Labs and Helius clients");
        Self {
            lyslabs: LysLabsClient::new(lyslabs_config),
            helius: HeliusClient::new(helius_config),
            db,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start continuous indexing for a wallet.
    /// First fetches historical data from Helius, then starts LYS Labs real-time stream.
    /// Returns true if subscription was started, false if already subscribed.
    pub async fn start_subscription(&self, wallet: &str) -> AppResult<bool> {
        let mut subs = self.subscriptions.write().await;

        // Check if already subscribed
        if subs.contains_key(wallet) {
            println!("[INDEXER] Wallet {} is already subscribed", wallet);
            tracing::info!(wallet = %wallet, "Wallet already has active subscription");
            return Ok(false);
        }

        println!("[INDEXER] Starting subscription for wallet: {}", wallet);
        println!("[INDEXER] Phase 1: Fetching historical data from Helius...");
        tracing::info!(wallet = %wallet, "Starting wallet subscription (Helius historical + LYS Labs real-time)");

        // Create cancellation token and channel for transactions
        let cancel_token = CancellationToken::new();
        let (tx_sender, tx_receiver) = mpsc::channel::<LysTransaction>(1000);

        // Create subscription record
        let subscription = WalletSubscription::new(wallet.to_string(), cancel_token.clone());
        let tx_counter = subscription.transactions_processed.clone();

        // Store subscription
        subs.insert(wallet.to_string(), subscription);
        drop(subs); // Release lock before spawning

        // Spawn transaction processor first so it's ready to receive
        let db = self.db.clone();
        let wallet_owned = wallet.to_string();
        let tx_counter_clone = tx_counter.clone();
        tokio::spawn(async move {
            Self::process_transaction_stream(wallet_owned, tx_receiver, db, tx_counter_clone).await;
        });

        // Fetch historical transactions from Helius and send to processor
        let helius = self.helius.clone();
        let wallet_for_helius = wallet.to_string();
        let tx_sender_for_helius = tx_sender.clone();

        tokio::spawn(async move {
            println!("[HELIUS] Starting historical data fetch for {}", &wallet_for_helius[..8]);

            match helius.get_all_transaction_history(&wallet_for_helius, MAX_HISTORICAL_TRANSACTIONS).await {
                Ok(transactions) => {
                    println!("[HELIUS] Fetched {} historical transactions, processing...", transactions.len());

                    let mut sent = 0;
                    for helius_tx in transactions {
                        // Convert Helius transaction to LysTransaction format
                        let lys_tx = Self::convert_helius_to_lys(&helius_tx, &wallet_for_helius);
                        if tx_sender_for_helius.send(lys_tx).await.is_err() {
                            println!("[HELIUS] Channel closed, stopping historical data send");
                            break;
                        }
                        sent += 1;
                    }

                    println!("[HELIUS] Sent {} historical transactions to processor", sent);
                }
                Err(e) => {
                    println!("[HELIUS] Failed to fetch historical data: {}", e);
                    tracing::error!(wallet = %wallet_for_helius, error = %e, "Failed to fetch historical data");
                }
            }
        });

        // Start the LYS Labs continuous stream for real-time data
        println!("[INDEXER] Phase 2: Starting LYS Labs real-time stream...");
        self.lyslabs
            .start_continuous_stream(wallet.to_string(), tx_sender, cancel_token)
            .await?;

        Ok(true)
    }

    /// Convert a Helius EnhancedTransaction to LysTransaction format for unified processing
    fn convert_helius_to_lys(helius_tx: &EnhancedTransaction, wallet: &str) -> LysTransaction {
        // Determine decoder type from source/type
        let decoder_type = helius_tx
            .source
            .as_ref()
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| helius_tx.tx_type.to_uppercase());

        // Map Helius tx_type to event_type
        let event_type = match helius_tx.tx_type.to_uppercase().as_str() {
            "SWAP" => "SWAP".to_string(),
            "TRANSFER" => "TRANSFER".to_string(),
            "UNKNOWN" => {
                // Check if it's a swap from events
                if helius_tx.events.as_ref().and_then(|e| e.swap.as_ref()).is_some() {
                    "SWAP".to_string()
                } else {
                    "UNKNOWN".to_string()
                }
            }
            other => other.to_string(),
        };

        // Extract token_in and token_out from swap events if available
        let (token_in, token_out) = if let Some(events) = &helius_tx.events {
            if let Some(swap) = &events.swap {
                let ti = swap.token_inputs.first().map(|t| {
                    lyslabs::LysTokenAmount {
                        mint: t.mint.clone(),
                        amount: t.raw_token_amount.token_amount.clone(),
                        ui_amount: t.raw_token_amount.token_amount.parse::<f64>().unwrap_or(0.0)
                            / 10_f64.powi(t.raw_token_amount.decimals as i32),
                        decimals: t.raw_token_amount.decimals,
                        owner: t.user_account.clone().unwrap_or_default(),
                    }
                });
                let to = swap.token_outputs.first().map(|t| {
                    lyslabs::LysTokenAmount {
                        mint: t.mint.clone(),
                        amount: t.raw_token_amount.token_amount.clone(),
                        ui_amount: t.raw_token_amount.token_amount.parse::<f64>().unwrap_or(0.0)
                            / 10_f64.powi(t.raw_token_amount.decimals as i32),
                        decimals: t.raw_token_amount.decimals,
                        owner: t.user_account.clone().unwrap_or_default(),
                    }
                });
                (ti, to)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Extract source/destination from token transfers
        let (source, destination, mint) = if let Some(transfers) = &helius_tx.token_transfers {
            if let Some(transfer) = transfers.first() {
                (
                    transfer.from_user_account.clone().unwrap_or_default(),
                    transfer.to_user_account.clone().unwrap_or_default(),
                    transfer.mint.clone(),
                )
            } else {
                (String::new(), String::new(), String::new())
            }
        } else {
            (String::new(), String::new(), String::new())
        };

        // Collect accounts involved
        let mut accounts = vec![wallet.to_string(), helius_tx.fee_payer.clone()];
        if let Some(transfers) = &helius_tx.token_transfers {
            for t in transfers {
                if let Some(from) = &t.from_user_account {
                    accounts.push(from.clone());
                }
                if let Some(to) = &t.to_user_account {
                    accounts.push(to.clone());
                }
            }
        }

        LysTransaction {
            tx_signature: helius_tx.signature.clone(),
            slot: helius_tx.slot,
            block_time: helius_tx.timestamp,
            decoder_type,
            event_type,
            mint,
            source,
            destination,
            fee_payer: helius_tx.fee_payer.clone(),
            program_id: String::new(), // Helius doesn't provide this directly
            pool: String::new(),
            token_in,
            token_out,
            accounts,
            ui_amount: 0.0,
            amount: String::new(),
        }
    }

    /// Stop continuous indexing for a wallet.
    /// Returns true if subscription was stopped, false if not subscribed.
    pub async fn stop_subscription(&self, wallet: &str) -> bool {
        let mut subs = self.subscriptions.write().await;

        if let Some(subscription) = subs.remove(wallet) {
            println!("[INDEXER] Stopping subscription for wallet: {}", wallet);
            tracing::info!(wallet = %wallet, "Stopping wallet subscription");
            subscription.cancel();
            true
        } else {
            println!("[INDEXER] No active subscription for wallet: {}", wallet);
            tracing::info!(wallet = %wallet, "No active subscription to stop");
            false
        }
    }

    /// Check if a wallet has an active subscription.
    pub async fn is_subscribed(&self, wallet: &str) -> bool {
        let subs = self.subscriptions.read().await;
        subs.contains_key(wallet)
    }

    /// List all active subscriptions.
    pub async fn list_subscriptions(&self) -> Vec<SubscriptionStatus> {
        let subs = self.subscriptions.read().await;
        let mut result = Vec::new();

        for (wallet, sub) in subs.iter() {
            let tx_count = *sub.transactions_processed.read().await;
            result.push(SubscriptionStatus {
                wallet: wallet.clone(),
                started_at: sub.started_at.to_rfc3339(),
                transactions_processed: tx_count,
                running: !sub.cancel_token.is_cancelled(),
            });
        }

        result
    }

    /// Process incoming transactions from the stream
    async fn process_transaction_stream(
        wallet: String,
        mut rx: mpsc::Receiver<LysTransaction>,
        db: Database,
        tx_counter: Arc<RwLock<u64>>,
    ) {
        println!("[INDEXER] Transaction processor started for wallet: {}", wallet);
        tracing::info!(wallet = %wallet, "Transaction processor started");

        while let Some(lys_tx) = rx.recv().await {
            // Parse the transaction
            if let Some(parsed) = parse_transaction(&lys_tx, &wallet) {
                let row = parsed.to_row();

                // Insert into database
                let query = r#"
                    INSERT INTO transactions (
                        signature, wallet, protocol, tx_type, token_in, token_out,
                        amount_in, amount_out, usd_value, block_time, slot
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, fromUnixTimestamp64Milli(?), ?)
                "#;

                match db
                    .client()
                    .query(query)
                    .bind(&row.signature)
                    .bind(&row.wallet)
                    .bind(&row.protocol)
                    .bind(&row.tx_type)
                    .bind(&row.token_in)
                    .bind(&row.token_out)
                    .bind(&row.amount_in)
                    .bind(&row.amount_out)
                    .bind(&row.usd_value)
                    .bind(row.block_time)
                    .bind(row.slot)
                    .execute()
                    .await
                {
                    Ok(_) => {
                        let mut count = tx_counter.write().await;
                        *count += 1;
                        println!("[INDEXER] Stored tx {} for wallet {} (total: {})",
                            &row.signature[..16], &wallet[..8], *count);
                        tracing::debug!(
                            wallet = %wallet,
                            signature = %row.signature,
                            protocol = %row.protocol,
                            tx_type = %row.tx_type,
                            "Transaction stored"
                        );
                    }
                    Err(e) => {
                        println!("[INDEXER] Failed to store tx: {}", e);
                        tracing::error!(
                            wallet = %wallet,
                            signature = %row.signature,
                            error = %e,
                            "Failed to store transaction"
                        );
                    }
                }
            }
        }

        println!("[INDEXER] Transaction processor ended for wallet: {}", wallet);
        tracing::info!(wallet = %wallet, "Transaction processor ended");
    }

    /// Legacy: Index a wallet with a one-time snapshot (original behavior).
    /// Kept for backward compatibility.
    pub async fn index_wallet_snapshot(&self, wallet: &str) -> AppResult<()> {
        const STREAM_TIMEOUT_SECS: u64 = 15;
        const MAX_TRANSACTIONS: usize = 1000;
        let start = Instant::now();
        println!("[INDEXER] Starting indexing for {} (timeout={}s, max={})",
            wallet, STREAM_TIMEOUT_SECS, MAX_TRANSACTIONS);
        tracing::info!(
            wallet = %wallet,
            timeout_secs = %STREAM_TIMEOUT_SECS,
            max_transactions = %MAX_TRANSACTIONS,
            "Starting wallet indexing via LYS Labs stream"
        );

        // Stream transactions from LYS Labs WebSocket
        println!("[INDEXER] Connecting to LYS Labs WebSocket...");
        tracing::debug!(wallet = %wallet, "Connecting to LYS Labs WebSocket for streaming");
        let stream_start = Instant::now();
        let transactions = self
            .lyslabs
            .stream_wallet_transactions(wallet, MAX_TRANSACTIONS, STREAM_TIMEOUT_SECS)
            .await?;
        println!("[INDEXER] Received {} raw transactions ({}ms)",
            transactions.len(), stream_start.elapsed().as_millis());
        tracing::info!(
            wallet = %wallet,
            raw_count = %transactions.len(),
            stream_duration_ms = %stream_start.elapsed().as_millis(),
            "Completed streaming from LYS Labs"
        );

        // Parse and filter DeFi transactions
        println!("[INDEXER] Parsing transactions...");
        tracing::debug!(wallet = %wallet, "Parsing raw transactions");
        let parse_start = Instant::now();
        let mut all_transactions = Vec::new();
        let mut parse_failures = 0;
        for tx in &transactions {
            if let Some(parsed) = parse_transaction(tx, wallet) {
                all_transactions.push(parsed);
            } else {
                parse_failures += 1;
            }
        }

        println!("[INDEXER] Parsed {} DeFi transactions ({} skipped, {}ms)",
            all_transactions.len(), parse_failures, parse_start.elapsed().as_millis());
        tracing::info!(
            wallet = %wallet,
            raw_count = %transactions.len(),
            parsed_count = %all_transactions.len(),
            parse_failures = %parse_failures,
            parse_duration_ms = %parse_start.elapsed().as_millis(),
            "Parsed transactions"
        );

        // Insert transactions into ClickHouse
        println!("[INDEXER] Inserting {} transactions into database...", all_transactions.len());
        tracing::debug!(wallet = %wallet, tx_count = %all_transactions.len(), "Inserting transactions into database");
        let insert_start = Instant::now();
        let mut insert_errors = 0;
        for (i, tx) in all_transactions.iter().enumerate() {
            let row = tx.to_row();
            if let Err(e) = self.insert_transaction(&row).await {
                tracing::warn!(
                    wallet = %wallet,
                    signature = %tx.signature,
                    error = %e,
                    "Failed to insert transaction"
                );
                insert_errors += 1;
            }
            if (i + 1) % 100 == 0 {
                println!("[INDEXER] Insert progress: {}/{}", i + 1, all_transactions.len());
                tracing::debug!(wallet = %wallet, progress = %i + 1, total = %all_transactions.len(), "Transaction insert progress");
            }
        }
        println!("[INDEXER] Inserted {} transactions ({} errors, {}ms)",
            all_transactions.len() - insert_errors, insert_errors, insert_start.elapsed().as_millis());
        tracing::info!(
            wallet = %wallet,
            inserted = %(all_transactions.len() - insert_errors),
            errors = %insert_errors,
            insert_duration_ms = %insert_start.elapsed().as_millis(),
            "Completed transaction inserts"
        );

        // Compute and store summary metrics
        println!("[INDEXER] Computing wallet metrics...");
        tracing::debug!(wallet = %wallet, "Computing wallet summary metrics");
        let metrics_start = Instant::now();
        self.compute_wallet_summary(wallet, &all_transactions).await?;
        println!("[INDEXER] Metrics computed ({}ms)", metrics_start.elapsed().as_millis());
        tracing::debug!(
            wallet = %wallet,
            metrics_duration_ms = %metrics_start.elapsed().as_millis(),
            "Completed metrics computation"
        );

        println!("[INDEXER] Indexing complete for {} - {} transactions in {}ms",
            wallet, all_transactions.len(), start.elapsed().as_millis());
        tracing::info!(
            wallet = %wallet,
            total_duration_ms = %start.elapsed().as_millis(),
            transactions_processed = %all_transactions.len(),
            "Wallet indexing complete"
        );

        Ok(())
    }

    async fn insert_transaction(&self, tx: &crate::db::models::TransactionRow) -> AppResult<()> {
        tracing::trace!(
            signature = %tx.signature,
            wallet = %tx.wallet,
            protocol = %tx.protocol,
            tx_type = %tx.tx_type,
            "Inserting transaction into database"
        );

        let query = r#"
            INSERT INTO transactions (
                signature, wallet, protocol, tx_type, token_in, token_out,
                amount_in, amount_out, usd_value, block_time, slot
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, fromUnixTimestamp64Milli(?), ?)
        "#;

        self.db
            .client()
            .query(query)
            .bind(&tx.signature)
            .bind(&tx.wallet)
            .bind(&tx.protocol)
            .bind(&tx.tx_type)
            .bind(&tx.token_in)
            .bind(&tx.token_out)
            .bind(&tx.amount_in)
            .bind(&tx.amount_out)
            .bind(&tx.usd_value)
            .bind(tx.block_time)
            .bind(tx.slot)
            .execute()
            .await?;

        tracing::trace!(signature = %tx.signature, "Transaction inserted successfully");

        Ok(())
    }

    async fn compute_wallet_summary(
        &self,
        wallet: &str,
        transactions: &[parser::ParsedTransaction],
    ) -> AppResult<()> {
        tracing::debug!(wallet = %wallet, tx_count = %transactions.len(), "Computing wallet summary");

        // Compute PnL metrics
        tracing::trace!(wallet = %wallet, "Computing PnL metrics");
        let pnl = metrics::compute_pnl(transactions);
        tracing::debug!(
            wallet = %wallet,
            total_value = %pnl.total_value,
            realized_24h = %pnl.realized_24h,
            unrealized = %pnl.unrealized,
            "PnL metrics computed"
        );

        // Compute risk metrics
        tracing::trace!(wallet = %wallet, "Computing risk metrics");
        let risk = metrics::compute_risk(transactions);
        tracing::debug!(
            wallet = %wallet,
            risk_score = %risk.score,
            position_count = %risk.position_count,
            "Risk metrics computed"
        );

        // Gather protocol list
        let mut protocols: Vec<String> = transactions
            .iter()
            .map(|t| t.protocol.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        protocols.sort();
        tracing::debug!(wallet = %wallet, protocols = ?protocols, "Protocols identified");

        // Get last activity time
        let last_activity = transactions
            .iter()
            .map(|t| t.block_time)
            .max()
            .unwrap_or(0);

        let summary = WalletSummaryRow {
            wallet: wallet.to_string(),
            total_value_usd: pnl.total_value.to_string(),
            realized_pnl_24h: pnl.realized_24h.to_string(),
            realized_pnl_7d: pnl.realized_7d.to_string(),
            realized_pnl_30d: pnl.realized_30d.to_string(),
            unrealized_pnl: pnl.unrealized.to_string(),
            largest_position_pct: risk.largest_position_pct.to_string(),
            protocol_count: protocols.len() as u8,
            position_count: risk.position_count,
            risk_score: risk.score,
            last_activity,
            protocols,
        };

        tracing::debug!(wallet = %wallet, "Upserting wallet summary to database");
        queries::upsert_wallet_summary(self.db.client(), &summary).await?;
        tracing::info!(
            wallet = %wallet,
            total_value_usd = %summary.total_value_usd,
            protocol_count = %summary.protocol_count,
            risk_score = %summary.risk_score,
            "Wallet summary stored"
        );

        Ok(())
    }
}
