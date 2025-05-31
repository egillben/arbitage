//! Blockchain Module
//!
//! This module is responsible for interacting with the Ethereum blockchain and listening for events.

mod listener;

pub use listener::{start_listener, BlockchainEventListener};

use anyhow::{Context, Result};
use ethers::providers::{Http, Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber, Filter, H256, U64};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::utils::validate_and_parse_address;

/// Alchemy-specific provider with enhanced capabilities
pub struct AlchemyProvider {
    /// The underlying HTTP provider
    pub http_provider: Arc<Provider<Http>>,

    /// The underlying WebSocket provider
    pub ws_provider: Option<Arc<Provider<Ws>>>,

    /// The Alchemy API key
    pub api_key: Option<String>,

    /// The chain ID
    pub chain_id: u64,
}

impl AlchemyProvider {
    /// Create a new Alchemy provider
    pub fn new(
        http_provider: Arc<Provider<Http>>,
        ws_provider: Option<Arc<Provider<Ws>>>,
        api_key: Option<String>,
        chain_id: u64,
    ) -> Self {
        Self {
            http_provider,
            ws_provider,
            api_key,
            chain_id,
        }
    }

    /// Get the HTTP provider
    pub fn http(&self) -> Arc<Provider<Http>> {
        self.http_provider.clone()
    }

    /// Get the WebSocket provider if available
    pub fn ws(&self) -> Option<Arc<Provider<Ws>>> {
        self.ws_provider.clone()
    }

    /// Get the gas price with Alchemy's enhanced gas API
    pub async fn get_gas_price(&self) -> Result<(u64, u64, u64)> {
        // If we have an Alchemy API key, use the enhanced gas API
        if let Some(api_key) = &self.api_key {
            let url = format!("https://eth-mainnet.g.alchemy.com/v2/{}/gas-price", api_key);

            let client = reqwest::Client::new();
            let response = client.get(&url).send().await?;

            if response.status().is_success() {
                let gas_data: serde_json::Value = response.json().await?;

                // Extract the gas prices
                let safe_gas_price = gas_data["result"]["safeLow"]["maxFee"]
                    .as_u64()
                    .unwrap_or(0);
                let average_gas_price = gas_data["result"]["standard"]["maxFee"]
                    .as_u64()
                    .unwrap_or(0);
                let fast_gas_price = gas_data["result"]["fast"]["maxFee"].as_u64().unwrap_or(0);

                return Ok((safe_gas_price, average_gas_price, fast_gas_price));
            }
        }

        // Fallback to standard gas price
        let gas_price = self.http_provider.get_gas_price().await?;
        let gas_price_gwei = gas_price.as_u64() / 1_000_000_000;

        Ok((gas_price_gwei, gas_price_gwei, gas_price_gwei))
    }

    /// Subscribe to pending transactions with Alchemy's enhanced API
    pub async fn subscribe_pending_transactions(&self) -> Result<H256> {
        if let Some(ws_provider) = &self.ws_provider {
            // Just return a dummy hash for now
            // In a real implementation, we would handle the subscription properly
            Ok(H256::zero())
        } else {
            // Return a dummy hash if WebSocket provider is not available
            Ok(H256::zero())
        }
    }

    /// Get token balances for an address using Alchemy's getTokenBalances API
    pub async fn get_token_balances(
        &self,
        address: Address,
        tokens: Vec<Address>,
    ) -> Result<HashMap<Address, ethers::types::U256>> {
        if let Some(api_key) = &self.api_key {
            let url = format!("https://eth-mainnet.g.alchemy.com/v2/{}/", api_key);

            let client = reqwest::Client::new();

            // Prepare the JSON-RPC request
            let params = serde_json::json!([
                address.to_string(),
                tokens
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
            ]);

            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "alchemy_getTokenBalances",
                "params": params
            });

            let response = client.post(&url).json(&request).send().await?;

            if response.status().is_success() {
                let balance_data: serde_json::Value = response.json().await?;

                // Extract the token balances
                let mut balances = HashMap::new();

                if let Some(token_balances) = balance_data["result"]["tokenBalances"].as_array() {
                    for token_balance in token_balances {
                        if let (Some(token_address), Some(balance)) = (
                            token_balance["contractAddress"].as_str(),
                            token_balance["tokenBalance"].as_str(),
                        ) {
                            // Parse the token address using our validation function
                            let address_result = validate_and_parse_address(token_address);

                            if let (Ok(address), Ok(balance)) = (
                                address_result,
                                U64::from_str_radix(balance.trim_start_matches("0x"), 16),
                            ) {
                                // Convert U64 to U256
                                let balance_u256 = ethers::types::U256::from(balance.as_u64());
                                balances.insert(address, balance_u256);
                            }
                        }
                    }
                }

                return Ok(balances);
            }
        }

        // Fallback to standard token balance queries
        let mut balances = HashMap::new();

        for token in tokens {
            // Create an ERC20 contract instance
            let abi_json = include_str!("../contract/abi/ERC20.json");
            let abi: ethers::abi::Abi = serde_json::from_str(abi_json)?;
            let contract = ethers::contract::Contract::new(token, abi, self.http_provider.clone());

            // Call the balanceOf function
            let balance: ethers::types::U256 = contract
                .method::<_, ethers::types::U256>("balanceOf", address)?
                .call()
                .await?;

            balances.insert(token, balance);
        }

        Ok(balances)
    }
}

/// Create a new blockchain client
pub async fn create_client(config: &Arc<Config>) -> Result<Arc<Provider<Http>>> {
    // Create the HTTP provider
    let provider = Provider::<Http>::try_from(&config.ethereum.rpc_url)
        .context("Failed to create HTTP provider")?;

    // Set the polling interval
    let provider = provider.interval(Duration::from_millis(2000));

    // Verify the connection
    let block_number = provider
        .get_block_number()
        .await
        .context("Failed to connect to Ethereum node")?;

    info!("Connected to Ethereum node at block {}", block_number);

    // Check if we're using Alchemy
    if config.ethereum.rpc_url.contains("alchemyapi.io") {
        info!("Using Alchemy as the Ethereum provider");
    }

    Ok(Arc::new(provider))
}

/// Create a new websocket client
pub async fn create_ws_client(config: &Arc<Config>) -> Result<Arc<Provider<Ws>>> {
    // Check if WebSocket is disabled in the config
    if let Some(use_websocket) = config.ethereum.use_websocket {
        if !use_websocket {
            return Err(anyhow::anyhow!(
                "WebSocket connections are disabled in the configuration"
            ));
        }
    }

    // Get the websocket URL
    let ws_url = if let Some(url) = &config.ethereum.ws_url {
        url.clone()
    } else {
        // Extract the websocket URL from the HTTP URL
        config.ethereum.rpc_url.replace("http", "ws")
    };

    // Create the websocket provider with timeout
    let timeout_duration = Duration::from_secs(config.ethereum.ws_timeout_seconds);

    let ws_future = Ws::connect(&ws_url);
    let ws = match tokio::time::timeout(timeout_duration, ws_future).await {
        Ok(result) => result.context("Failed to connect to websocket endpoint")?,
        Err(_) => {
            return Err(anyhow::anyhow!(
                "WebSocket connection timed out after {} seconds",
                config.ethereum.ws_timeout_seconds
            ))
        }
    };

    // Create the provider
    let provider = Provider::new(ws);

    // Verify the connection with timeout
    let block_number_future = provider.get_block_number();
    let block_number = match tokio::time::timeout(timeout_duration, block_number_future).await {
        Ok(result) => result.context("Failed to connect to Ethereum node via websocket")?,
        Err(_) => {
            return Err(anyhow::anyhow!(
                "WebSocket verification timed out after {} seconds",
                config.ethereum.ws_timeout_seconds
            ))
        }
    };

    info!(
        "Connected to Ethereum node via websocket at block {}",
        block_number
    );

    // Check if we're using Alchemy
    if ws_url.contains("alchemyapi.io") {
        info!("Using Alchemy WebSocket provider");
    }

    Ok(Arc::new(provider))
}

/// Create a new Alchemy provider
pub async fn create_alchemy_provider(config: &Arc<Config>) -> Result<Arc<AlchemyProvider>> {
    // Create the HTTP provider
    let http_provider = create_client(config).await?;

    // Create the WebSocket provider if enabled
    let ws_provider = if config.ethereum.use_websocket.unwrap_or(true) {
        match create_ws_client(config).await {
            Ok(provider) => {
                info!("Successfully created WebSocket provider for Alchemy");
                Some(provider)
            }
            Err(e) => {
                warn!("Failed to create WebSocket provider for Alchemy: {}", e);
                warn!("Alchemy provider will operate in HTTP-only mode");
                None
            }
        }
    } else {
        info!("WebSocket connections disabled. Alchemy provider will operate in HTTP-only mode");
        None
    };

    // Create the Alchemy provider
    let provider = AlchemyProvider::new(
        http_provider,
        ws_provider,
        config.ethereum.alchemy_api_key.clone(),
        config.ethereum.chain_id,
    );

    Ok(Arc::new(provider))
}

/// Get the contract ABI from a file or embedded resource
pub fn get_contract_abi(name: &str) -> Result<ethers::abi::Abi> {
    // This is a placeholder implementation
    // In a real implementation, we would load the ABI from a file or embedded resource

    // For now, just return a minimal ABI
    let json = match name {
        "uniswap_v2_factory" => {
            r#"[
            {
                "anonymous": false,
                "inputs": [
                    {
                        "indexed": true,
                        "internalType": "address",
                        "name": "token0",
                        "type": "address"
                    },
                    {
                        "indexed": true,
                        "internalType": "address",
                        "name": "token1",
                        "type": "address"
                    },
                    {
                        "indexed": false,
                        "internalType": "address",
                        "name": "pair",
                        "type": "address"
                    },
                    {
                        "indexed": false,
                        "internalType": "uint256",
                        "name": "",
                        "type": "uint256"
                    }
                ],
                "name": "PairCreated",
                "type": "event"
            }
        ]"#
        }
        "uniswap_v2_pair" => {
            r#"[
            {
                "anonymous": false,
                "inputs": [
                    {
                        "indexed": true,
                        "internalType": "address",
                        "name": "sender",
                        "type": "address"
                    },
                    {
                        "indexed": false,
                        "internalType": "uint256",
                        "name": "amount0In",
                        "type": "uint256"
                    },
                    {
                        "indexed": false,
                        "internalType": "uint256",
                        "name": "amount1In",
                        "type": "uint256"
                    },
                    {
                        "indexed": false,
                        "internalType": "uint256",
                        "name": "amount0Out",
                        "type": "uint256"
                    },
                    {
                        "indexed": false,
                        "internalType": "uint256",
                        "name": "amount1Out",
                        "type": "uint256"
                    },
                    {
                        "indexed": true,
                        "internalType": "address",
                        "name": "to",
                        "type": "address"
                    }
                ],
                "name": "Swap",
                "type": "event"
            }
        ]"#
        }
        _ => return Err(anyhow::anyhow!("Unknown contract ABI: {}", name)),
    };

    let abi = serde_json::from_str(json)?;
    Ok(abi)
}

/// Parse an Ethereum address
pub fn parse_address(address: &str) -> Result<Address> {
    validate_and_parse_address(address)
}
