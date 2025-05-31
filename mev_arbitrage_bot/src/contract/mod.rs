//! Smart Contract Module
//!
//! This module is responsible for interacting with the ArbitrageExecutor smart contract.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::abi::{Abi, Token};
use ethers::contract::{Contract, ContractFactory};
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, Bytes, TransactionRequest, H256, U256};
use log::{debug, error, info, warn};
use std::sync::Arc;

use crate::config::Config;
use crate::utils::validate_and_parse_address;

/// Interface for smart contract managers
#[async_trait]
pub trait ContractManager: Send + Sync {
    /// Deploy the ArbitrageExecutor contract
    async fn deploy_contract(
        &self,
        lending_pool_address: Address,
        uniswap_router_address: Address,
        sushiswap_router_address: Address,
        curve_router_address: Address,
    ) -> Result<Address>;

    /// Execute an arbitrage opportunity
    async fn execute_arbitrage(
        &self,
        assets: Vec<Address>,
        amounts: Vec<U256>,
        modes: Vec<U256>,
        token_path: Vec<Address>,
        dex_path: Vec<String>,
        slippage: U256,
    ) -> Result<TransactionRequest>;

    /// Authorize a caller
    async fn authorize_caller(&self, caller: Address) -> Result<TransactionRequest>;

    /// Unauthorize a caller
    async fn unauthorize_caller(&self, caller: Address) -> Result<TransactionRequest>;

    /// Activate emergency stop
    async fn activate_emergency_stop(&self) -> Result<TransactionRequest>;

    /// Deactivate emergency stop
    async fn deactivate_emergency_stop(&self) -> Result<TransactionRequest>;

    /// Recover ERC20 tokens
    async fn recover_erc20(&self, token: Address, amount: U256) -> Result<TransactionRequest>;

    /// Recover ETH
    async fn recover_eth(&self) -> Result<TransactionRequest>;

    /// Get the contract address
    fn get_contract_address(&self) -> Option<Address>;

    /// Set the contract address
    fn set_contract_address(&mut self, address: Address);

    /// Get the contract ABI
    fn get_contract_abi(&self) -> Abi;
}

/// Implementation of the smart contract manager
pub struct ContractManagerImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<Http>>,
    wallet: Option<LocalWallet>,
    contract_address: Option<Address>,
    contract_abi: Abi,
}

/// Create a new smart contract manager
pub async fn create_manager(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<Http>>,
) -> Result<Arc<ContractManagerImpl>> {
    // Initialize the wallet if a private key is provided
    let wallet = if let Some(private_key) = &config.ethereum.private_key {
        Some(private_key.parse::<LocalWallet>()?)
    } else {
        None
    };

    // Load the contract ABI
    let contract_abi = load_contract_abi()?;

    // Create the contract manager
    let manager = ContractManagerImpl {
        config: config.clone(),
        blockchain_client,
        wallet,
        contract_address: None,
        contract_abi,
    };

    Ok(Arc::new(manager))
}

/// Load the contract ABI from the embedded JSON file
fn load_contract_abi() -> Result<Abi> {
    // Load the ABI from the embedded JSON file
    let abi_json = include_str!("./abi/ArbitrageExecutor.json");
    let abi: Abi =
        serde_json::from_str(abi_json).context("Failed to parse ArbitrageExecutor ABI")?;

    Ok(abi)
}

#[async_trait]
impl ContractManager for ContractManagerImpl {
    async fn deploy_contract(
        &self,
        lending_pool_address: Address,
        uniswap_router_address: Address,
        sushiswap_router_address: Address,
        curve_router_address: Address,
    ) -> Result<Address> {
        // Check if we have a wallet
        let wallet = self
            .wallet
            .as_ref()
            .context("No wallet available for deploying contract")?;

        // Create a client with signer
        let client_with_signer = SignerMiddleware::new(
            self.blockchain_client.clone(),
            wallet.clone().with_chain_id(self.config.ethereum.chain_id),
        );

        // Load the contract bytecode
        let bytecode = include_str!("./bytecode/ArbitrageExecutor.bin");
        let bytecode =
            hex::decode(bytecode.trim()).context("Failed to decode ArbitrageExecutor bytecode")?;

        // Create the contract factory
        let factory = ContractFactory::new(
            self.contract_abi.clone(),
            Bytes::from(bytecode),
            Arc::new(client_with_signer),
        );

        // Deploy the contract
        info!("Deploying ArbitrageExecutor contract...");
        let constructor_args = (
            lending_pool_address,
            uniswap_router_address,
            sushiswap_router_address,
            curve_router_address,
        );

        let contract = factory
            .deploy(constructor_args)
            .context("Failed to deploy contract")?
            .send()
            .await
            .context("Failed to send contract deployment transaction")?;

        let contract_address = contract.address();
        info!(
            "ArbitrageExecutor contract deployed at: {}",
            contract_address
        );

        Ok(contract_address)
    }

    async fn execute_arbitrage(
        &self,
        assets: Vec<Address>,
        amounts: Vec<U256>,
        modes: Vec<U256>,
        token_path: Vec<Address>,
        dex_path: Vec<String>,
        slippage: U256,
    ) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("executeArbitrage")
            .context("Failed to find executeArbitrage function")?;

        let params = (assets, amounts, modes, token_path, dex_path, slippage);
        let data = function
            .encode_input(&[
                Token::Array(params.0.iter().map(|&addr| Token::Address(addr)).collect()),
                Token::Array(params.1.iter().map(|&amount| Token::Uint(amount)).collect()),
                Token::Array(params.2.iter().map(|&mode| Token::Uint(mode)).collect()),
                Token::Array(params.3.iter().map(|&addr| Token::Address(addr)).collect()),
                Token::Array(
                    params
                        .4
                        .iter()
                        .map(|dex| Token::String(dex.clone()))
                        .collect(),
                ),
                Token::Uint(params.5),
            ])
            .context("Failed to encode executeArbitrage function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn authorize_caller(&self, caller: Address) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("authorizeCaller")
            .context("Failed to find authorizeCaller function")?;

        let data = function
            .encode_input(&[Token::Address(caller)])
            .context("Failed to encode authorizeCaller function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn unauthorize_caller(&self, caller: Address) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("unauthorizeCaller")
            .context("Failed to find unauthorizeCaller function")?;

        let data = function
            .encode_input(&[Token::Address(caller)])
            .context("Failed to encode unauthorizeCaller function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn activate_emergency_stop(&self) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("activateEmergencyStop")
            .context("Failed to find activateEmergencyStop function")?;

        let data = function
            .encode_input(&[])
            .context("Failed to encode activateEmergencyStop function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn deactivate_emergency_stop(&self) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("deactivateEmergencyStop")
            .context("Failed to find deactivateEmergencyStop function")?;

        let data = function
            .encode_input(&[])
            .context("Failed to encode deactivateEmergencyStop function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn recover_erc20(&self, token: Address, amount: U256) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("recoverERC20")
            .context("Failed to find recoverERC20 function")?;

        let data = function
            .encode_input(&[Token::Address(token), Token::Uint(amount)])
            .context("Failed to encode recoverERC20 function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    async fn recover_eth(&self) -> Result<TransactionRequest> {
        // Check if we have a contract address
        let contract_address = self.contract_address.context("Contract address not set")?;

        // Create the contract instance
        let contract = Contract::new(
            contract_address,
            self.contract_abi.clone(),
            self.blockchain_client.clone(),
        );

        // Encode the function call
        let function = self
            .contract_abi
            .function("recoverETH")
            .context("Failed to find recoverETH function")?;

        let data = function
            .encode_input(&[])
            .context("Failed to encode recoverETH function call")?;

        // Create the transaction request
        let tx = TransactionRequest::new()
            .to(contract_address)
            .data(Bytes::from(data));

        Ok(tx)
    }

    fn get_contract_address(&self) -> Option<Address> {
        self.contract_address
    }

    fn set_contract_address(&mut self, address: Address) {
        self.contract_address = Some(address);
    }

    fn get_contract_abi(&self) -> Abi {
        self.contract_abi.clone()
    }
}
