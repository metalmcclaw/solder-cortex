use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;

use crate::config::HeliusConfig;
use crate::error::{AppError, AppResult};

// ============================================================================
// Helius Client - For Historical Transaction Data
// ============================================================================

#[derive(Clone)]
pub struct HeliusClient {
    client: Client,
    api_key: String,
}

impl HeliusClient {
    pub fn new(config: &HeliusConfig) -> Self {
        let api_key_preview = if config.api_key.len() > 8 {
            format!(
                "{}...{}",
                &config.api_key[..4],
                &config.api_key[config.api_key.len() - 4..]
            )
        } else if config.api_key.is_empty() {
            "EMPTY".to_string()
        } else {
            "***".to_string()
        };
        println!("[HELIUS] Initializing client with API key: {}", api_key_preview);
        tracing::debug!("Creating Helius client");

        Self {
            client: Client::new(),
            api_key: config.api_key.clone(),
        }
    }

    fn api_url(&self) -> String {
        "https://api.helius.xyz/v0".to_string()
    }

    /// Fetch historical transactions for a wallet with pagination.
    /// Returns transactions in reverse chronological order (newest first).
    pub async fn get_transaction_history(
        &self,
        wallet: &str,
        before: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<EnhancedTransaction>> {
        let start = Instant::now();
        let url = format!(
            "{}/addresses/{}/transactions?api-key={}",
            self.api_url(),
            wallet,
            self.api_key
        );

        let mut query_params = vec![("limit", limit.to_string())];
        if let Some(b) = before {
            query_params.push(("before", b.to_string()));
        }

        println!(
            "[HELIUS] Fetching transactions for {} (limit={}, before={:?})",
            &wallet[..8],
            limit,
            before.map(|s| &s[..16.min(s.len())])
        );
        tracing::debug!(
            wallet = %wallet,
            limit = %limit,
            before = ?before,
            "Fetching historical transactions from Helius"
        );

        let response = self
            .client
            .get(&url)
            .query(&query_params)
            .send()
            .await
            .map_err(|e| {
                println!("[HELIUS] Request failed: {}", e);
                tracing::error!(error = %e, "Helius request failed");
                AppError::ExternalApi(format!("Helius request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("[HELIUS] API error {}: {}", status, &body[..200.min(body.len())]);
            tracing::error!(status = %status, body = %body, "Helius API error");
            return Err(AppError::ExternalApi(format!(
                "Helius API error {}: {}",
                status, body
            )));
        }

        let transactions: Vec<EnhancedTransaction> = response.json().await.map_err(|e| {
            println!("[HELIUS] Failed to parse response: {}", e);
            tracing::error!(error = %e, "Failed to parse Helius response");
            AppError::ExternalApi(format!("Failed to parse Helius response: {}", e))
        })?;

        println!(
            "[HELIUS] Received {} transactions ({}ms)",
            transactions.len(),
            start.elapsed().as_millis()
        );
        tracing::info!(
            wallet = %wallet,
            count = %transactions.len(),
            duration_ms = %start.elapsed().as_millis(),
            "Fetched historical transactions"
        );

        Ok(transactions)
    }

    /// Fetch all historical transactions for a wallet (with automatic pagination).
    /// Stops when reaching `max_transactions` or when no more transactions are available.
    pub async fn get_all_transaction_history(
        &self,
        wallet: &str,
        max_transactions: usize,
    ) -> AppResult<Vec<EnhancedTransaction>> {
        let start = Instant::now();
        let mut all_transactions = Vec::new();
        let mut before: Option<String> = None;
        let page_size = 100; // Helius max per request

        println!(
            "[HELIUS] Starting full history fetch for {} (max={})",
            &wallet[..8],
            max_transactions
        );
        tracing::info!(
            wallet = %wallet,
            max_transactions = %max_transactions,
            "Starting full history fetch from Helius"
        );

        loop {
            let transactions = self
                .get_transaction_history(wallet, before.as_deref(), page_size)
                .await?;

            if transactions.is_empty() {
                println!("[HELIUS] No more transactions available");
                break;
            }

            let last_sig = transactions.last().map(|t| t.signature.clone());
            all_transactions.extend(transactions);

            println!(
                "[HELIUS] Progress: {} transactions fetched",
                all_transactions.len()
            );

            if all_transactions.len() >= max_transactions {
                println!("[HELIUS] Reached max transactions limit");
                all_transactions.truncate(max_transactions);
                break;
            }

            before = last_sig;

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        println!(
            "[HELIUS] Completed history fetch: {} transactions in {}ms",
            all_transactions.len(),
            start.elapsed().as_millis()
        );
        tracing::info!(
            wallet = %wallet,
            total = %all_transactions.len(),
            duration_ms = %start.elapsed().as_millis(),
            "Completed full history fetch"
        );

        Ok(all_transactions)
    }
}

// ============================================================================
// Helius API Types - Enhanced Transaction Format
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedTransaction {
    pub signature: String,
    #[serde(default)]
    pub slot: u64,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub fee: u64,
    #[serde(default)]
    pub fee_payer: String,
    #[serde(rename = "type", default)]
    pub tx_type: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub token_transfers: Option<Vec<TokenTransfer>>,
    #[serde(default)]
    pub native_transfers: Option<Vec<NativeTransfer>>,
    #[serde(default)]
    pub account_data: Option<Vec<AccountData>>,
    #[serde(default)]
    pub instructions: Option<Vec<InstructionInfo>>,
    #[serde(default)]
    pub events: Option<TransactionEvents>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransfer {
    #[serde(default)]
    pub from_user_account: Option<String>,
    #[serde(default)]
    pub to_user_account: Option<String>,
    #[serde(default)]
    pub from_token_account: Option<String>,
    #[serde(default)]
    pub to_token_account: Option<String>,
    #[serde(default)]
    pub token_amount: f64,
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub token_standard: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTransfer {
    #[serde(default)]
    pub from_user_account: Option<String>,
    #[serde(default)]
    pub to_user_account: Option<String>,
    #[serde(default)]
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub native_balance_change: i64,
    #[serde(default)]
    pub token_balance_changes: Option<Vec<TokenBalanceChange>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalanceChange {
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub raw_token_amount: RawTokenAmount,
    #[serde(default)]
    pub user_account: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RawTokenAmount {
    #[serde(default)]
    pub token_amount: String,
    #[serde(default)]
    pub decimals: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionInfo {
    #[serde(default)]
    pub program_id: String,
    #[serde(default)]
    pub accounts: Vec<String>,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub inner_instructions: Option<Vec<InstructionInfo>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionEvents {
    #[serde(default)]
    pub swap: Option<SwapEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapEvent {
    #[serde(default)]
    pub native_input: Option<NativeAmount>,
    #[serde(default)]
    pub native_output: Option<NativeAmount>,
    #[serde(default)]
    pub token_inputs: Vec<HeliusTokenAmount>,
    #[serde(default)]
    pub token_outputs: Vec<HeliusTokenAmount>,
    #[serde(default)]
    pub token_fees: Vec<HeliusTokenAmount>,
    #[serde(default)]
    pub native_fees: Vec<NativeAmount>,
    #[serde(default)]
    pub inner_swaps: Vec<InnerSwap>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAmount {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusTokenAmount {
    #[serde(default)]
    pub user_account: Option<String>,
    #[serde(default)]
    pub token_account: Option<String>,
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub raw_token_amount: RawTokenAmount,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerSwap {
    #[serde(default)]
    pub native_input: Option<NativeAmount>,
    #[serde(default)]
    pub native_output: Option<NativeAmount>,
    #[serde(default)]
    pub token_inputs: Vec<HeliusTokenAmount>,
    #[serde(default)]
    pub token_outputs: Vec<HeliusTokenAmount>,
    #[serde(default)]
    pub program_info: Option<ProgramInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramInfo {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub program_name: String,
    #[serde(default)]
    pub instruction_name: String,
}
