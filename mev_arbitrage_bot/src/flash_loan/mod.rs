//! Flash Loan Manager Module
//!
//! This module is responsible for interfacing with Aave flash loan contracts.

use anyhow::Result;
use async_trait::async_trait;
use ethers::abi::{Abi, Token};
use ethers::contract::{Contract, ContractInstance};
use ethers::providers::Provider;
use ethers::types::{Address, Bytes, TransactionRequest, U256};
use std::sync::Arc;

use crate::config::Config;
use crate::utils::validate_and_parse_address;

/// Flash loan parameters
#[derive(Debug, Clone)]
pub struct FlashLoanParams {
    /// Tokens to borrow
    pub tokens: Vec<Address>,

    /// Amounts to borrow for each token
    pub amounts: Vec<U256>,

    /// Whether to use the borrowed amount as collateral
    pub modes: Vec<u8>,

    /// Address that will receive the funds
    pub receiver_address: Address,

    /// Arbitrary data to pass to the receiver
    pub params: Bytes,
}

/// Interface for flash loan managers
#[async_trait]
pub trait FlashLoanManager: Send + Sync {
    /// Create a flash loan transaction
    async fn create_flash_loan_transaction(
        &self,
        params: FlashLoanParams,
    ) -> Result<TransactionRequest>;

    /// Calculate the flash loan fee
    async fn calculate_fee(&self, token: Address, amount: U256) -> Result<U256>;

    /// Get the maximum borrowable amount for a token
    async fn get_max_borrowable_amount(&self, token: Address) -> Result<U256>;
}

/// Implementation of the flash loan manager
pub struct FlashLoanManagerImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    lending_pool_contract:
        ContractInstance<Arc<Provider<ethers::providers::Http>>, Provider<ethers::providers::Http>>,
}

/// Create a new flash loan manager
pub async fn create_manager(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<dyn FlashLoanManager>> {
    // This is a placeholder implementation
    // In a real implementation, we would initialize the flash loan manager with the provided parameters

    // Load the Aave lending pool ABI
    let lending_pool_abi = include_str!("./abi/aave_lending_pool.json");
    let lending_pool_abi: Abi = serde_json::from_str(lending_pool_abi).unwrap_or_else(|_| {
        // If the ABI file is not available, use a minimal ABI with just the flashLoan function
        let json = r#"[
            {
                "inputs": [
                    {
                        "internalType": "address",
                        "name": "receiverAddress",
                        "type": "address"
                    },
                    {
                        "internalType": "address[]",
                        "name": "assets",
                        "type": "address[]"
                    },
                    {
                        "internalType": "uint256[]",
                        "name": "amounts",
                        "type": "uint256[]"
                    },
                    {
                        "internalType": "uint256[]",
                        "name": "modes",
                        "type": "uint256[]"
                    },
                    {
                        "internalType": "address",
                        "name": "onBehalfOf",
                        "type": "address"
                    },
                    {
                        "internalType": "bytes",
                        "name": "params",
                        "type": "bytes"
                    },
                    {
                        "internalType": "uint16",
                        "name": "referralCode",
                        "type": "uint16"
                    }
                ],
                "name": "flashLoan",
                "outputs": [],
                "stateMutability": "nonpayable",
                "type": "function"
            }
        ]"#;
        serde_json::from_str(json).expect("Failed to parse fallback ABI")
    });

    // Create the lending pool contract
    let lending_pool_address =
        match validate_and_parse_address(&config.flash_loan.aave_lending_pool) {
            Ok(address) => address,
            Err(e) => {
                log::error!("Failed to parse aave_lending_pool address: {}", e);
                // Provide a fallback address for testing
                Address::from_low_u64_be(2)
            }
        };
    let lending_pool_contract = Contract::new(
        lending_pool_address,
        lending_pool_abi,
        blockchain_client.clone(),
    );

    let manager = FlashLoanManagerImpl {
        config: config.clone(),
        blockchain_client,
        lending_pool_contract,
    };

    Ok(Arc::new(manager))
}

#[async_trait]
impl FlashLoanManager for FlashLoanManagerImpl {
    async fn create_flash_loan_transaction(
        &self,
        params: FlashLoanParams,
    ) -> Result<TransactionRequest> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Validate the flash loan parameters
        // 2. Create a transaction to call the flashLoan function on the Aave lending pool

        // For now, just return a dummy transaction
        let tx = TransactionRequest::new()
            .to(self.lending_pool_contract.address())
            .data(Bytes::from(vec![0u8]));

        Ok(tx)
    }

    async fn calculate_fee(&self, _token: Address, amount: U256) -> Result<U256> {
        // Aave charges a 0.09% fee on flash loans
        let fee_percentage = U256::from(9)
            .saturating_mul(amount)
            .checked_div(U256::from(10000))
            .unwrap_or_default();
        Ok(fee_percentage)
    }

    async fn get_max_borrowable_amount(&self, _token: Address) -> Result<U256> {
        // This is a placeholder implementation
        // In a real implementation, we would query the Aave lending pool for the available liquidity

        // For now, just return a dummy amount
        Ok(U256::from(1000000000000000000u128)) // 1 ETH
    }
}

// Create a directory for ABI files
#[cfg(not(test))]
#[path = "abi/mod.rs"]
pub mod abi {
    // This module will contain the ABI files for the flash loan contracts
}
