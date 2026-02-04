use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Jupiter,
    Raydium,
    Kamino,
    Meteora,
    Orca,
    PumpFun,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Jupiter => write!(f, "jupiter"),
            Protocol::Raydium => write!(f, "raydium"),
            Protocol::Kamino => write!(f, "kamino"),
            Protocol::Meteora => write!(f, "meteora"),
            Protocol::Orca => write!(f, "orca"),
            Protocol::PumpFun => write!(f, "pumpfun"),
        }
    }
}

impl Protocol {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "jupiter" => Some(Protocol::Jupiter),
            "raydium" => Some(Protocol::Raydium),
            "kamino" => Some(Protocol::Kamino),
            "meteora" => Some(Protocol::Meteora),
            "orca" => Some(Protocol::Orca),
            "pumpfun" | "pump_fun" | "pump.fun" => Some(Protocol::PumpFun),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Swap,
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    AddLiquidity,
    RemoveLiquidity,
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionType::Swap => write!(f, "swap"),
            TransactionType::Deposit => write!(f, "deposit"),
            TransactionType::Withdraw => write!(f, "withdraw"),
            TransactionType::Borrow => write!(f, "borrow"),
            TransactionType::Repay => write!(f, "repay"),
            TransactionType::AddLiquidity => write!(f, "add_liquidity"),
            TransactionType::RemoveLiquidity => write!(f, "remove_liquidity"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    LendingSupply,
    LendingBorrow,
    Lp,
}

impl fmt::Display for PositionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionType::LendingSupply => write!(f, "lending_supply"),
            PositionType::LendingBorrow => write!(f, "lending_borrow"),
            PositionType::Lp => write!(f, "lp"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeWindow {
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
    #[serde(rename = "all")]
    All,
}

impl TimeWindow {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "24h" | "1d" => Some(TimeWindow::Day),
            "7d" | "1w" => Some(TimeWindow::Week),
            "30d" | "1m" => Some(TimeWindow::Month),
            "all" => Some(TimeWindow::All),
            _ => None,
        }
    }

    pub fn to_days(&self) -> Option<i64> {
        match self {
            TimeWindow::Day => Some(1),
            TimeWindow::Week => Some(7),
            TimeWindow::Month => Some(30),
            TimeWindow::All => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAmount {
    pub token: String,
    pub amount: Decimal,
    pub usd_value: Decimal,
}

pub fn validate_solana_address(address: &str) -> bool {
    if address.len() < 32 || address.len() > 44 {
        return false;
    }
    bs58::decode(address).into_vec().is_ok()
}
