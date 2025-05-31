//! Transaction Builder Module
//!
//! This module is responsible for constructing transaction payloads.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::abi::{AbiEncode, Token};
use ethers::providers::Provider;
use ethers::types::{Address, Bytes, TransactionRequest, U256};
use log::{debug, info, warn};
use std::sync::Arc;

use crate::config::Config;
use crate::contract::ContractManager;
use crate::scanner::ArbitrageOpportunity;
use crate::transaction::ArbitrageTransaction;
use crate::utils::validate_and_parse_address;

/// Interface for transaction builders
#[async_trait]
pub trait TransactionBuilder: Send + Sync {
    /// Build an arbitrage transaction from an opportunity
    async fn build_arbitrage_transaction(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ArbitrageTransaction>;

    /// Estimate the gas cost for a transaction
    async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<U256>;

    /// Build the calldata for a transaction
    fn build_calldata(
        &self,
        token_path: &[Address],
        amounts: &[U256],
        dex_path: &[String],
    ) -> Result<Bytes>;
}

/// Implementation of the transaction builder
pub struct TransactionBuilderImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    wallet_address: Address,
    contract_manager: Option<Arc<dyn ContractManager>>,
}

/// Create a new transaction builder
pub async fn create_builder(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    contract_manager: Option<Arc<dyn ContractManager>>,
) -> Result<Arc<dyn TransactionBuilder>> {
    // Parse the wallet address
    let wallet_address = match validate_and_parse_address(&config.ethereum.wallet_address) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Failed to parse wallet address: {}", e);
            // Provide a fallback address for testing
            Address::from_low_u64_be(9)
        }
    };

    let builder = TransactionBuilderImpl {
        config: config.clone(),
        blockchain_client,
        wallet_address,
        contract_manager,
    };

    Ok(Arc::new(builder))
}

#[async_trait]
impl TransactionBuilder for TransactionBuilderImpl {
    async fn build_arbitrage_transaction(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<ArbitrageTransaction> {
        // Determine the optimal token path
        let token_path = opportunity.token_path.clone();

        // Determine the DEX path
        let dex_path = vec![
            opportunity.source_dex.clone(),
            opportunity.target_dex.clone(),
        ];

        // Calculate the optimal amounts based on the opportunity
        let flash_loan_amount =
            U256::from((opportunity.estimated_profit * 2.0) as u128 * 10u128.pow(18));
        let amounts = vec![flash_loan_amount];

        // Create the modes for the flash loan (0 = no debt)
        let modes = vec![U256::from(0)];

        // Calculate the slippage tolerance in basis points (0.5% = 50 basis points)
        let slippage = U256::from((self.config.arbitrage.slippage_tolerance * 100.0) as u64);

        // Build the calldata for the transaction
        let calldata = self.build_calldata(&token_path, &amounts, &dex_path)?;

        // Create the transaction request
        let request = if let Some(contract_manager) = &self.contract_manager {
            // Get the contract address
            if let Some(_contract_address) = contract_manager.get_contract_address() {
                // Build the transaction using the contract manager
                // Clone all vectors to avoid ownership issues
                let token_path_first = vec![token_path[0]];
                let amounts_clone = amounts.clone();
                let token_path_clone = token_path.clone();
                let dex_path_clone = dex_path.clone();

                contract_manager
                    .execute_arbitrage(
                        token_path_first, // Use the first token in the path as the flash loan asset
                        amounts_clone,
                        modes,
                        token_path_clone,
                        dex_path_clone,
                        slippage,
                    )
                    .await?
            } else {
                // Contract address not set, use a placeholder transaction
                warn!("Contract address not set, using placeholder transaction");

                // Create a placeholder transaction request
                TransactionRequest::new()
                    .from(self.wallet_address)
                    .to(self.wallet_address) // This would be the arbitrage contract
                    .data(calldata.clone())
                    .gas(U256::from(self.config.gas.gas_limit))
            }
        } else {
            // Contract manager not available, use a placeholder transaction
            warn!("Contract manager not available, using placeholder transaction");

            // Create a placeholder transaction request
            TransactionRequest::new()
                .from(self.wallet_address)
                .to(self.wallet_address) // This would be the arbitrage contract
                .data(calldata.clone())
                .gas(U256::from(self.config.gas.gas_limit))
        };

        // Estimate the gas cost
        let estimated_gas = self.estimate_gas(&request).await?;

        // Estimate the gas price
        let estimated_gas_price = U256::from(self.config.gas.max_gas_price * 1_000_000_000); // Convert gwei to wei

        // Estimate the total cost
        let estimated_cost = estimated_gas.saturating_mul(estimated_gas_price);

        Ok(ArbitrageTransaction {
            request,
            estimated_gas,
            estimated_gas_price,
            estimated_cost,
            estimated_profit: opportunity.estimated_profit,
            token_path,
            dex_path,
            calldata,
            use_mev_share: self.config.mev_share.enabled,
        })
    }

    async fn estimate_gas(&self, _tx: &TransactionRequest) -> Result<U256> {
        // This is a placeholder implementation
        // In a real implementation, we would call the eth_estimateGas RPC method

        // For now, just return the gas limit from the config
        Ok(U256::from(self.config.gas.gas_limit))
    }

    fn build_calldata(
        &self,
        token_path: &[Address],
        amounts: &[U256],
        dex_path: &[String],
    ) -> Result<Bytes> {
        if let Some(contract_manager) = &self.contract_manager {
            // Get the contract ABI
            let contract_abi = contract_manager.get_contract_abi();

            // Find the executeArbitrage function
            let function = contract_abi
                .function("executeArbitrage")
                .context("Failed to find executeArbitrage function")?;

            // Create the modes for the flash loan (0 = no debt)
            let modes = vec![U256::from(0); amounts.len()];

            // Calculate the slippage tolerance in basis points (0.5% = 50 basis points)
            let slippage = U256::from((self.config.arbitrage.slippage_tolerance * 100.0) as u64);

            // Encode the function call
            let data = function
                .encode_input(&[
                    Token::Array(
                        token_path
                            .iter()
                            .map(|&addr| Token::Address(addr))
                            .collect(),
                    ),
                    Token::Array(amounts.iter().map(|&amount| Token::Uint(amount)).collect()),
                    Token::Array(modes.iter().map(|&mode| Token::Uint(mode)).collect()),
                    Token::Array(
                        token_path
                            .iter()
                            .map(|&addr| Token::Address(addr))
                            .collect(),
                    ),
                    Token::Array(
                        dex_path
                            .iter()
                            .map(|dex| Token::String(dex.clone()))
                            .collect(),
                    ),
                    Token::Uint(slippage),
                ])
                .context("Failed to encode executeArbitrage function call")?;

            Ok(Bytes::from(data))
        } else {
            // Contract manager not available, use a placeholder implementation
            warn!("Contract manager not available, using placeholder calldata");

            // Create a placeholder calldata
            let tokens = vec![
                Token::Array(
                    token_path
                        .iter()
                        .map(|&addr| Token::Address(addr))
                        .collect(),
                ),
                Token::Array(amounts.iter().map(|&amount| Token::Uint(amount)).collect()),
                Token::Array(
                    dex_path
                        .iter()
                        .map(|dex| Token::String(dex.clone()))
                        .collect(),
                ),
            ];

            // Encode the function selector and parameters
            let selector = [0x12, 0x34, 0x56, 0x78]; // Dummy selector

            // Manually encode the tokens
            let mut encoded_params = Vec::new();
            for token in &tokens {
                match token {
                    Token::Array(arr) => {
                        for item in arr {
                            if let Token::Address(addr) = item {
                                encoded_params.extend_from_slice(&addr.0);
                            } else if let Token::Uint(num) = item {
                                // Convert U256 to bytes manually
                                let bytes = num.to_string().into_bytes();
                                encoded_params.extend_from_slice(&bytes);
                            } else if let Token::String(s) = item {
                                encoded_params.extend_from_slice(s.as_bytes());
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Combine the selector and parameters
            let mut calldata = Vec::with_capacity(selector.len() + encoded_params.len());
            calldata.extend_from_slice(&selector);
            calldata.extend_from_slice(&encoded_params);

            Ok(Bytes::from(calldata))
        }
    }
}
