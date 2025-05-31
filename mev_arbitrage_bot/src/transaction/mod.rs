//! Transaction Module
//!
//! This module is responsible for constructing and executing transaction payloads.

mod builder;
mod executor;

pub use builder::{create_builder, TransactionBuilder};
pub use executor::{create_executor, TransactionExecutor};

use crate::contract::ContractManager;

use anyhow::Result;
use ethers::types::{Address, Bytes, TransactionRequest, H256, U256};
use std::sync::Arc;

/// Represents an arbitrage transaction
#[derive(Debug, Clone)]
pub struct ArbitrageTransaction {
    /// The transaction request
    pub request: TransactionRequest,

    /// The estimated gas cost
    pub estimated_gas: U256,

    /// The estimated gas price
    pub estimated_gas_price: U256,

    /// The estimated total cost (gas * gas price)
    pub estimated_cost: U256,

    /// The estimated profit
    pub estimated_profit: f64,

    /// The token path
    pub token_path: Vec<Address>,

    /// The DEX path
    pub dex_path: Vec<String>,

    /// The calldata
    pub calldata: Bytes,

    /// Whether to use MEV-Share
    pub use_mev_share: bool,
}

/// Represents the result of a transaction execution
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// The transaction hash
    pub tx_hash: H256,

    /// The block number where the transaction was included
    pub block_number: Option<u64>,

    /// The gas used
    pub gas_used: Option<U256>,

    /// The actual cost (gas used * gas price)
    pub actual_cost: Option<U256>,

    /// Whether the transaction was successful
    pub success: bool,

    /// The error message, if any
    pub error: Option<String>,
}

/// Validate a transaction before sending it
pub async fn validate_transaction(tx: &ArbitrageTransaction) -> Result<()> {
    // This is a placeholder implementation
    // In a real implementation, we would:
    // 1. Check that the transaction has a valid gas limit
    // 2. Check that the transaction has a valid gas price
    // 3. Check that the transaction has a valid nonce
    // 4. Check that the transaction has a valid to address
    // 5. Check that the transaction has valid calldata

    // For now, just return Ok
    Ok(())
}
