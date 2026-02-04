use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_util::sync::CancellationToken;

use crate::config::LysLabsConfig;
use crate::error::{AppError, AppResult};

// ============================================================================
// LYS Labs WebSocket Client
// ============================================================================

#[derive(Clone)]
pub struct LysLabsClient {
    api_key: String,
    ws_url: String,
    /// Cached transactions per wallet for the current session
    wallet_transactions: Arc<RwLock<HashMap<String, Vec<LysTransaction>>>>,
}

impl LysLabsClient {
    pub fn new(config: &LysLabsConfig) -> Self {
        let api_key_preview = if config.api_key.len() > 8 {
            format!("{}...{}", &config.api_key[..4], &config.api_key[config.api_key.len()-4..])
        } else if config.api_key.is_empty() {
            "EMPTY".to_string()
        } else {
            "***".to_string()
        };
        println!("[LYSLABS] Initializing client with API key: {}", api_key_preview);
        println!("[LYSLABS] WebSocket URL: {}", config.ws_url);
        tracing::debug!(ws_url = %config.ws_url, "Creating LYS Labs client");
        Self {
            api_key: config.api_key.clone(),
            ws_url: config.ws_url.clone(),
            wallet_transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn ws_url_with_key(&self) -> String {
        format!("{}?apiKey={}", self.ws_url, self.api_key)
    }

    /// Subscribe to transaction stream and collect transactions for a specific wallet.
    /// This streams real-time transactions and filters by wallet address.
    /// Returns after collecting the specified limit or timeout.
    pub async fn stream_wallet_transactions(
        &self,
        wallet: &str,
        limit: usize,
        timeout_secs: u64,
    ) -> AppResult<Vec<LysTransaction>> {
        let start = Instant::now();
        let url = self.ws_url_with_key();

        // Log URL with masked API key
        if self.api_key.is_empty() {
            println!("[LYSLABS] ERROR: API key is EMPTY! Check CORTEX_LYSLABS_API_KEY env var");
        } else {
            let masked_key = format!("{}...{}", &self.api_key[..4], &self.api_key[self.api_key.len()-4..]);
            println!("[LYSLABS] Connecting with API key: {}", masked_key);
        }

        tracing::info!(
            wallet = %wallet,
            limit = %limit,
            timeout_secs = %timeout_secs,
            "Connecting to LYS Labs WebSocket"
        );

        let connect_start = Instant::now();
        let (ws_stream, response) = connect_async(&url)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "WebSocket connection failed");
                AppError::ExternalApi(format!("WebSocket connection failed: {}", e))
            })?;
        tracing::debug!(
            connect_duration_ms = %connect_start.elapsed().as_millis(),
            status = ?response.status(),
            "WebSocket connection established"
        );

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to transaction stream
        // Try different subscription formats for LYS Labs
        let subscribe_msg = serde_json::json!({ "action": "subscribe" });
        println!("[WEBSOCKET] Sending subscribe message: {}", subscribe_msg);
        tracing::debug!("Sending subscribe message to LYS Labs");
        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| {
                println!("[WEBSOCKET] Failed to send subscribe: {}", e);
                tracing::error!(error = %e, "Failed to send subscribe message");
                AppError::ExternalApi(format!("Failed to subscribe: {}", e))
            })?;

        println!("[WEBSOCKET] Subscribed successfully, waiting for transactions for wallet: {}", wallet);
        tracing::info!(wallet = %wallet, "Subscribed to LYS Labs transaction stream, waiting for transactions");

        let (tx, mut rx) = mpsc::channel::<LysTransaction>(1000);
        let collected_limit = limit;
        let wallet_for_spawn = wallet.to_string();

        // Spawn task to process incoming messages
        let process_handle = tokio::spawn(async move {
            let mut messages_received: u64 = 0;
            let mut transactions_matched: u64 = 0;
            let mut parse_errors: u64 = 0;

            println!("[WEBSOCKET] Started listening for transactions...");

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        messages_received += 1;

                        // Log first few raw messages for debugging
                        if messages_received <= 3 {
                            println!("[WEBSOCKET] Raw message #{}: {}...",
                                messages_received,
                                text.chars().take(200).collect::<String>());
                        }

                        if messages_received % 100 == 0 {
                            println!("[WEBSOCKET] Progress: {} messages received, {} matched, {} parse errors",
                                messages_received, transactions_matched, parse_errors);
                        }

                        match serde_json::from_str::<LysWebSocketMessage>(&text) {
                            Ok(ws_msg) => {
                                // Log message types we're seeing
                                if messages_received <= 10 {
                                    println!("[WEBSOCKET] Message type: '{}'", ws_msg.msg_type);
                                }

                                // Extract transactions from "transaction" or "transactions" messages
                                let transactions = match ws_msg.msg_type.as_str() {
                                    "transaction" | "transactions" => ws_msg.extract_transactions(),
                                    _ => {
                                        if messages_received <= 10 {
                                            println!("[WEBSOCKET] Skipping message type: '{}'", ws_msg.msg_type);
                                        }
                                        vec![]
                                    }
                                };

                                if messages_received <= 5 && !transactions.is_empty() {
                                    println!("[WEBSOCKET] Extracted {} transactions from message", transactions.len());
                                }

                                for transaction in transactions {
                                    // Filter by wallet involvement
                                    if transaction.involves_wallet(&wallet_for_spawn) {
                                        transactions_matched += 1;
                                        let sig_preview = if transaction.tx_signature.len() > 16 {
                                            &transaction.tx_signature[..16]
                                        } else {
                                            &transaction.tx_signature
                                        };
                                        println!("[WEBSOCKET] MATCHED transaction for wallet: sig={}, type={}, decoder={}",
                                            sig_preview,
                                            transaction.event_type,
                                            transaction.decoder_type);
                                        tracing::debug!(
                                            wallet = %wallet_for_spawn,
                                            signature = %transaction.tx_signature,
                                            decoder_type = %transaction.decoder_type,
                                            event_type = %transaction.event_type,
                                            "Transaction matched for wallet"
                                        );
                                        if tx.send(transaction).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                parse_errors += 1;
                                if parse_errors <= 3 {
                                    println!("[WEBSOCKET] Parse error: {} - text: {}...",
                                        e, text.chars().take(100).collect::<String>());
                                }
                            }
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        println!("[WEBSOCKET] Connection closed by server: {:?}", frame);
                        tracing::info!(frame = ?frame, "WebSocket connection closed by server");
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        tracing::trace!("Received ping from WebSocket");
                    }
                    Err(e) => {
                        println!("[WEBSOCKET] Error: {}", e);
                        tracing::warn!(error = %e, "WebSocket error occurred");
                        break;
                    }
                    _ => {}
                }
            }
            println!("[WEBSOCKET] Task ending - {} messages, {} matched, {} errors",
                messages_received, transactions_matched, parse_errors);
            tracing::debug!(
                wallet = %wallet_for_spawn,
                total_messages = %messages_received,
                total_matched = %transactions_matched,
                "WebSocket processing task ending"
            );
        });

        // Collect transactions with timeout
        let mut transactions = Vec::new();
        let timeout = tokio::time::Duration::from_secs(timeout_secs);
        let deadline = tokio::time::Instant::now() + timeout;

        tracing::debug!(wallet = %wallet, "Starting transaction collection loop");

        loop {
            tokio::select! {
                Some(tx) = rx.recv() => {
                    transactions.push(tx);
                    if transactions.len() % 10 == 0 {
                        tracing::debug!(
                            wallet = %wallet,
                            collected = %transactions.len(),
                            limit = %collected_limit,
                            "Transaction collection progress"
                        );
                    }
                    if transactions.len() >= collected_limit {
                        tracing::info!(
                            wallet = %wallet,
                            collected = %transactions.len(),
                            "Transaction limit reached, stopping collection"
                        );
                        break;
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    tracing::info!(
                        wallet = %wallet,
                        collected = %transactions.len(),
                        timeout_secs = %timeout_secs,
                        "Timeout reached, stopping collection"
                    );
                    break;
                }
            }
        }

        // Cleanup
        tracing::debug!(wallet = %wallet, "Aborting WebSocket processing task");
        process_handle.abort();

        tracing::info!(
            wallet = %wallet,
            tx_count = %transactions.len(),
            total_duration_ms = %start.elapsed().as_millis(),
            "Collected transactions from LYS Labs stream"
        );

        Ok(transactions)
    }

    /// Start continuous streaming for a wallet. Returns a channel receiver for transactions.
    /// The streaming continues until the cancellation token is triggered.
    pub async fn start_continuous_stream(
        &self,
        wallet: String,
        tx_sender: mpsc::Sender<LysTransaction>,
        cancel_token: CancellationToken,
    ) -> AppResult<()> {
        let api_key = self.api_key.clone();
        let ws_url = self.ws_url.clone();

        println!("[LYSLABS] Starting continuous stream for wallet: {}", wallet);
        tracing::info!(wallet = %wallet, "Starting continuous WebSocket stream");

        // Spawn the streaming task
        tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            const MAX_RECONNECT_ATTEMPTS: u32 = 10;
            const RECONNECT_DELAY_BASE_MS: u64 = 1000;

            loop {
                if cancel_token.is_cancelled() {
                    println!("[LYSLABS] Continuous stream cancelled for wallet: {}", wallet);
                    tracing::info!(wallet = %wallet, "Continuous stream cancelled");
                    break;
                }

                // Connect to WebSocket
                let url = format!("{}?apiKey={}", ws_url, api_key);
                println!("[LYSLABS] Connecting to WebSocket for continuous stream (attempt {})", reconnect_attempts + 1);

                match connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        reconnect_attempts = 0; // Reset on successful connection
                        println!("[LYSLABS] WebSocket connected for wallet: {}", wallet);
                        tracing::info!(wallet = %wallet, "WebSocket connected for continuous streaming");

                        let (mut write, mut read) = ws_stream.split();

                        // Subscribe to transaction stream
                        let subscribe_msg = serde_json::json!({ "action": "subscribe" });
                        if let Err(e) = write.send(Message::Text(subscribe_msg.to_string())).await {
                            println!("[LYSLABS] Failed to subscribe: {}", e);
                            tracing::error!(error = %e, "Failed to send subscribe message");
                            continue;
                        }

                        println!("[LYSLABS] Subscribed, listening for transactions for wallet: {}", wallet);
                        tracing::info!(wallet = %wallet, "Listening for transactions");

                        let mut messages_received: u64 = 0;
                        let mut transactions_matched: u64 = 0;

                        // Process messages until cancelled or disconnected
                        loop {
                            tokio::select! {
                                _ = cancel_token.cancelled() => {
                                    println!("[LYSLABS] Stream cancelled for wallet: {}", wallet);
                                    tracing::info!(wallet = %wallet, "Stream cancelled by user");
                                    break;
                                }
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(text))) => {
                                            messages_received += 1;

                                            if let Ok(ws_msg) = serde_json::from_str::<LysWebSocketMessage>(&text) {
                                                let transactions = match ws_msg.msg_type.as_str() {
                                                    "transaction" | "transactions" => ws_msg.extract_transactions(),
                                                    _ => vec![],
                                                };

                                                for transaction in transactions {
                                                    if transaction.involves_wallet(&wallet) {
                                                        transactions_matched += 1;
                                                        let sig_preview = if transaction.tx_signature.len() > 16 {
                                                            &transaction.tx_signature[..16]
                                                        } else {
                                                            &transaction.tx_signature
                                                        };
                                                        println!("[LYSLABS] MATCHED tx for {}: sig={}, type={}, decoder={}",
                                                            &wallet[..8], sig_preview, transaction.event_type, transaction.decoder_type);
                                                        tracing::debug!(
                                                            wallet = %wallet,
                                                            signature = %transaction.tx_signature,
                                                            "Transaction matched"
                                                        );

                                                        // Send to processing channel
                                                        if tx_sender.send(transaction).await.is_err() {
                                                            println!("[LYSLABS] Channel closed, stopping stream for wallet: {}", wallet);
                                                            tracing::warn!(wallet = %wallet, "Transaction channel closed");
                                                            return;
                                                        }
                                                    }
                                                }
                                            }

                                            // Log progress periodically
                                            if messages_received % 1000 == 0 {
                                                println!("[LYSLABS] Wallet {} - {} messages received, {} matched",
                                                    &wallet[..8], messages_received, transactions_matched);
                                                tracing::info!(
                                                    wallet = %wallet,
                                                    messages = %messages_received,
                                                    matched = %transactions_matched,
                                                    "Streaming progress"
                                                );
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) => {
                                            println!("[LYSLABS] WebSocket closed for wallet: {}", wallet);
                                            tracing::warn!(wallet = %wallet, "WebSocket closed by server");
                                            break;
                                        }
                                        Some(Ok(Message::Ping(data))) => {
                                            let _ = write.send(Message::Pong(data)).await;
                                        }
                                        Some(Err(e)) => {
                                            println!("[LYSLABS] WebSocket error for wallet {}: {}", wallet, e);
                                            tracing::error!(wallet = %wallet, error = %e, "WebSocket error");
                                            break;
                                        }
                                        None => {
                                            println!("[LYSLABS] WebSocket stream ended for wallet: {}", wallet);
                                            tracing::warn!(wallet = %wallet, "WebSocket stream ended");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("[LYSLABS] Failed to connect WebSocket for wallet {}: {}", wallet, e);
                        tracing::error!(wallet = %wallet, error = %e, "WebSocket connection failed");
                    }
                }

                // Check if we should reconnect
                if cancel_token.is_cancelled() {
                    break;
                }

                reconnect_attempts += 1;
                if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                    println!("[LYSLABS] Max reconnect attempts reached for wallet: {}", wallet);
                    tracing::error!(wallet = %wallet, "Max reconnect attempts reached, stopping");
                    break;
                }

                // Exponential backoff
                let delay = RECONNECT_DELAY_BASE_MS * (2_u64.pow(reconnect_attempts.min(6)));
                println!("[LYSLABS] Reconnecting in {}ms for wallet: {}", delay, wallet);
                tracing::info!(wallet = %wallet, delay_ms = %delay, "Reconnecting after delay");
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }

            println!("[LYSLABS] Continuous stream ended for wallet: {}", wallet);
            tracing::info!(wallet = %wallet, "Continuous stream task completed");
        });

        Ok(())
    }

    /// Get token price using Jupiter API (same as before, LYS Labs doesn't provide prices)
    pub async fn get_token_price(&self, token_mint: &str) -> AppResult<Option<f64>> {
        let start = Instant::now();
        tracing::debug!(token_mint = %token_mint, "Fetching token price from Jupiter API");

        let client = reqwest::Client::new();
        let url = format!("https://price.jup.ag/v6/price?ids={}", token_mint);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(token_mint = %token_mint, error = %e, "Jupiter price API request failed");
                AppError::ExternalApi(format!("Jupiter price API failed: {}", e))
            })?;

        if !response.status().is_success() {
            tracing::debug!(
                token_mint = %token_mint,
                status = %response.status(),
                "Jupiter API returned non-success status"
            );
            return Ok(None);
        }

        let price_response: JupiterPriceResponse = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!(token_mint = %token_mint, error = %e, "Failed to parse Jupiter price response");
                AppError::ExternalApi(format!("Failed to parse price response: {}", e))
            })?;

        let price = price_response.data.get(token_mint).map(|p| p.price);

        tracing::debug!(
            token_mint = %token_mint,
            price = ?price,
            duration_ms = %start.elapsed().as_millis(),
            "Token price fetched"
        );

        Ok(price)
    }
}

// ============================================================================
// LYS Labs WebSocket Message Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct LysWebSocketMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Raw data - we parse this manually for flexibility
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl LysWebSocketMessage {
    /// Extract transactions from the data field, handling various formats
    pub fn extract_transactions(&self) -> Vec<LysTransaction> {
        let Some(data) = &self.data else {
            return vec![];
        };

        // Handle array of transactions
        if let Some(arr) = data.as_array() {
            return arr
                .iter()
                .filter_map(|v| LysTransaction::from_value(v))
                .collect();
        }

        // Handle single transaction
        if let Some(tx) = LysTransaction::from_value(data) {
            return vec![tx];
        }

        vec![]
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LysTransaction {
    /// Transaction signature
    pub tx_signature: String,
    /// Slot number
    pub slot: u64,
    /// Block time (unix timestamp)
    pub block_time: i64,
    /// Decoder type: SPL_TOKEN, PUMP_FUN, RAYDIUM, JUPITER, etc.
    pub decoder_type: String,
    /// Event type: SWAP, MINT, TRANSFER, MIGRATE, CREATE_POOL, etc.
    pub event_type: String,
    /// Token mint address
    pub mint: String,
    /// Source wallet/account
    pub source: String,
    /// Destination wallet/account
    pub destination: String,
    /// Fee payer
    pub fee_payer: String,
    /// Program ID that processed this transaction
    pub program_id: String,
    /// Pool address for DEX operations
    pub pool: String,
    /// Swap token in
    pub token_in: Option<LysTokenAmount>,
    /// Swap token out
    pub token_out: Option<LysTokenAmount>,
    /// Additional accounts involved (from various fields)
    pub accounts: Vec<String>,
    /// UI amount value
    pub ui_amount: f64,
    /// Raw amount
    pub amount: String,
}

impl LysTransaction {
    /// Parse a LysTransaction from a serde_json::Value, extracting fields flexibly
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let obj = v.as_object()?;

        // Extract tx_signature from various possible field names
        let tx_signature = obj
            .get("txSignature")
            .or_else(|| obj.get("signature"))
            .or_else(|| obj.get("tx_signature"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if tx_signature.is_empty() {
            return None;
        }

        let slot = obj
            .get("slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let block_time = obj
            .get("blockTime")
            .or_else(|| obj.get("block_time"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let decoder_type = obj
            .get("decoderType")
            .or_else(|| obj.get("decoder_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let event_type = obj
            .get("eventType")
            .or_else(|| obj.get("event_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mint = obj
            .get("mint")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let source = obj
            .get("source")
            .or_else(|| obj.get("from"))
            .or_else(|| obj.get("authority"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let destination = obj
            .get("destination")
            .or_else(|| obj.get("to"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let fee_payer = obj
            .get("feePayer")
            .or_else(|| obj.get("fee_payer"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let program_id = obj
            .get("programId")
            .or_else(|| obj.get("program_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pool = obj
            .get("pool")
            .or_else(|| obj.get("ammId"))
            .or_else(|| obj.get("poolAddress"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract token amounts for swaps
        let token_in = obj.get("tokenIn").or_else(|| obj.get("token_in")).and_then(|v| LysTokenAmount::from_value(v));
        let token_out = obj.get("tokenOut").or_else(|| obj.get("token_out")).and_then(|v| LysTokenAmount::from_value(v));

        // Collect all accounts from various fields
        let mut accounts: Vec<String> = vec![];

        // Add source/dest/feePayer if present
        if !source.is_empty() { accounts.push(source.clone()); }
        if !destination.is_empty() { accounts.push(destination.clone()); }
        if !fee_payer.is_empty() { accounts.push(fee_payer.clone()); }

        // Add token owners
        if let Some(ref ti) = token_in {
            if !ti.owner.is_empty() { accounts.push(ti.owner.clone()); }
        }
        if let Some(ref to) = token_out {
            if !to.owner.is_empty() { accounts.push(to.owner.clone()); }
        }

        // Add from accounts array if present
        if let Some(accs) = obj.get("accounts").and_then(|v| v.as_array()) {
            for acc in accs {
                if let Some(s) = acc.as_str() {
                    accounts.push(s.to_string());
                }
            }
        }

        // Parse ui_amount from nested object or direct value
        let ui_amount = obj
            .get("uiAmount")
            .or_else(|| obj.get("ui_amount"))
            .map(|v| {
                if let Some(n) = v.as_f64() {
                    n
                } else if let Some(obj) = v.as_object() {
                    obj.get("value").and_then(|n| n.as_f64()).unwrap_or(0.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);

        let amount = obj
            .get("amount")
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else if let Some(n) = v.as_u64() {
                    n.to_string()
                } else if let Some(n) = v.as_i64() {
                    n.to_string()
                } else {
                    "0".to_string()
                }
            })
            .unwrap_or_default();

        Some(Self {
            tx_signature,
            slot,
            block_time,
            decoder_type,
            event_type,
            mint,
            source,
            destination,
            fee_payer,
            program_id,
            pool,
            token_in,
            token_out,
            accounts,
            ui_amount,
            amount,
        })
    }
}

impl LysTransaction {
    /// Check if this transaction involves the given wallet address
    pub fn involves_wallet(&self, wallet: &str) -> bool {
        self.source == wallet
            || self.destination == wallet
            || self.fee_payer == wallet
            || self.accounts.iter().any(|a| a == wallet)
            || self
                .token_in
                .as_ref()
                .map(|t| t.owner == wallet)
                .unwrap_or(false)
            || self
                .token_out
                .as_ref()
                .map(|t| t.owner == wallet)
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LysTokenAmount {
    pub mint: String,
    pub amount: String,
    pub ui_amount: f64,
    pub decimals: u8,
    pub owner: String,
}

impl LysTokenAmount {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let obj = v.as_object()?;

        let mint = obj
            .get("mint")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let amount = obj
            .get("amount")
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else if let Some(n) = v.as_u64() {
                    n.to_string()
                } else {
                    "0".to_string()
                }
            })
            .unwrap_or_default();

        let ui_amount = obj
            .get("uiAmount")
            .or_else(|| obj.get("ui_amount"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let decimals = obj
            .get("decimals")
            .and_then(|v| v.as_u64())
            .map(|n| n as u8)
            .unwrap_or(0);

        let owner = obj
            .get("owner")
            .or_else(|| obj.get("authority"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(Self {
            mint,
            amount,
            ui_amount,
            decimals,
            owner,
        })
    }
}

// ============================================================================
// Jupiter Price API Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: HashMap<String, JupiterPrice>,
}

#[derive(Debug, Deserialize)]
pub struct JupiterPrice {
    pub id: String,
    #[serde(rename = "mintSymbol")]
    pub mint_symbol: Option<String>,
    pub price: f64,
}
