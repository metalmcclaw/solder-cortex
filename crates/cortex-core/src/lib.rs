//! # Cortex Core
//!
//! Unified data models and types for Solder Cortex - The Memory Layer for AI Agents.
//!
//! This crate provides the canonical data structures that bridge DeFi analytics
//! and prediction market intelligence, enabling cross-domain insights.

pub mod models;
pub mod conviction;
pub mod error;

pub use models::*;
pub use conviction::*;
pub use error::*;
