//! Curve Interface Module
//!
//! This module is responsible for interfacing with Curve Finance.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::abi::{Abi, Token};
use ethers::contract::{Contract, ContractCall, ContractInstance};
use ethers::providers::Provider;
use ethers::types::{Address, Bytes, U256};
use log::{debug, info, warn};
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::dex::{DexInterface, DexType, PoolInfo, TradeQuote};
use crate::utils::validate_and_parse_address;

/// Curve interface
pub struct CurveInterface {
    name: String,
    factory_address: Address,
    router_address: Address,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    factory_contract:
        ContractInstance<Arc<Provider<ethers::providers::Http>>, Provider<ethers::providers::Http>>,
    router_contract:
        ContractInstance<Arc<Provider<ethers::providers::Http>>, Provider<ethers::providers::Http>>,
    pools: Mutex<Vec<PoolInfo>>,
}

/// Create a new Curve interface
pub async fn create_interface(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<dyn DexInterface>> {
    // Parse addresses
    let factory_address = match validate_and_parse_address(&config.dex.curve.factory_address) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Failed to parse curve factory address: {}", e);
            // Provide a fallback address for testing
            Address::from_low_u64_be(1)
        }
    };

    let router_address = match validate_and_parse_address(&config.dex.curve.router_address) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Failed to parse curve router address: {}", e);
            // Provide a fallback address for testing
            Address::from_low_u64_be(5)
        }
    };

    // Load ABIs
    let factory_abi = include_str!("./abi/curve_factory.json");
    let factory_abi: Abi = serde_json::from_str(factory_abi).unwrap_or_else(|_| {
        // If the ABI file is not available, use a minimal ABI
        let json = r#"[
            {
                "name": "find_pool_for_coins",
                "outputs": [
                    {
                        "type": "address",
                        "name": ""
                    }
                ],
                "inputs": [
                    {
                        "type": "address",
                        "name": "_from"
                    },
                    {
                        "type": "address",
                        "name": "_to"
                    }
                ],
                "stateMutability": "view",
                "type": "function"
            }
        ]"#;
        serde_json::from_str(json).expect("Failed to parse fallback ABI")
    });

    let router_abi = include_str!("./abi/curve_router.json");
    let router_abi: Abi = serde_json::from_str(router_abi).unwrap_or_else(|_| {
        // If the ABI file is not available, use a minimal ABI
        let json = r#"[
            {
                "name": "get_best_rate",
                "outputs": [
                    {
                        "type": "address",
                        "name": ""
                    },
                    {
                        "type": "uint256",
                        "name": ""
                    }
                ],
                "inputs": [
                    {
                        "type": "address",
                        "name": "_from"
                    },
                    {
                        "type": "address",
                        "name": "_to"
                    },
                    {
                        "type": "uint256",
                        "name": "_amount"
                    }
                ],
                "stateMutability": "view",
                "type": "function"
            }
        ]"#;
        serde_json::from_str(json).expect("Failed to parse fallback ABI")
    });

    // Create contracts
    let factory_contract = Contract::new(factory_address, factory_abi, blockchain_client.clone());
    let router_contract = Contract::new(router_address, router_abi, blockchain_client.clone());

    let interface = CurveInterface {
        name: "Curve".to_string(),
        factory_address,
        router_address,
        blockchain_client: blockchain_client.clone(),
        factory_contract,
        router_contract,
        pools: Mutex::new(Vec::new()),
    };

    let interface = Arc::new(interface);

    // Initialize pools
    if let Err(e) = interface.initialize_pools().await {
        warn!("Failed to initialize Curve pools: {}", e);
    }

    Ok(interface)
}

impl CurveInterface {
    /// Initialize pools
    async fn initialize_pools(&self) -> Result<()> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Query the factory for all pool creation events
        // 2. Get the pool addresses
        // 3. Get the token addresses and reserves for each pool

        // For now, just create a dummy pool for stablecoins
        let usdc_address =
            match validate_and_parse_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") {
                Ok(address) => address,
                Err(e) => {
                    log::warn!("Failed to parse USDC address: {}", e);
                    // Provide a fallback address for testing
                    Address::from_low_u64_be(10)
                }
            };

        let dai_address =
            match validate_and_parse_address("0x6B175474E89094C44Da98b954EedeAC495271d0F") {
                Ok(address) => address,
                Err(e) => {
                    log::warn!("Failed to parse DAI address: {}", e);
                    // Provide a fallback address for testing
                    Address::from_low_u64_be(11)
                }
            };

        let pool_address = self
            .factory_contract
            .method::<_, Address>("find_pool_for_coins", (usdc_address, dai_address))?
            .call()
            .await?;

        if pool_address != Address::zero() {
            // Create dummy reserves for now
            let reserves = vec![
                U256::from(1000000000u128),             // 1000 USDC (6 decimals)
                U256::from(1000000000000000000000u128), // 1000 DAI (18 decimals)
            ];

            let pool_info = PoolInfo {
                address: pool_address,
                dex_type: DexType::Curve,
                tokens: vec![usdc_address, dai_address],
                reserves,
                fee: 4, // 0.04%
            };

            // Add the pool to the list
            if let Ok(mut pools) = self.pools.lock() {
                pools.push(pool_info);
            }

            info!("Initialized Curve USDC-DAI pool: {:?}", pool_address);
        }

        Ok(())
    }

    /// Get the index of a token in a pool
    async fn get_token_index(&self, pool: Address, token: Address) -> Result<usize> {
        // This is a placeholder implementation
        // In a real implementation, we would call the coins function on the pool contract

        // For now, just return a dummy index
        if let Ok(pools) = self.pools.lock() {
            for pool_info in &*pools {
                if pool_info.address == pool {
                    for (i, &pool_token) in pool_info.tokens.iter().enumerate() {
                        if pool_token == token {
                            return Ok(i);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Token not found in pool"))
    }
}

#[async_trait]
impl DexInterface for CurveInterface {
    fn name(&self) -> &str {
        &self.name
    }

    fn dex_type(&self) -> DexType {
        DexType::Curve
    }

    fn factory_address(&self) -> Address {
        self.factory_address
    }

    fn router_address(&self) -> Address {
        self.router_address
    }

    async fn get_pools(&self) -> Result<Vec<PoolInfo>> {
        if let Ok(pools) = self.pools.lock() {
            Ok(pools.clone())
        } else {
            Err(anyhow::anyhow!("Failed to lock pools mutex"))
        }
    }

    async fn get_pool(&self, token_a: Address, token_b: Address) -> Result<Option<PoolInfo>> {
        // Check if the pool is already in the list
        if let Ok(pools) = self.pools.lock() {
            for pool in &*pools {
                if pool.tokens.contains(&token_a) && pool.tokens.contains(&token_b) {
                    return Ok(Some(pool.clone()));
                }
            }
        }

        // If not, query the factory
        let pool_address = self
            .factory_contract
            .method::<_, Address>("find_pool_for_coins", (token_a, token_b))?
            .call()
            .await?;

        if pool_address == Address::zero() {
            return Ok(None);
        }

        // Get the reserves
        let reserves = self.get_reserves(pool_address).await?;

        // Create the pool info
        let pool_info = PoolInfo {
            address: pool_address,
            dex_type: DexType::Curve,
            tokens: vec![token_a, token_b],
            reserves,
            fee: 4, // 0.04%
        };

        // Add the pool to the list
        if let Ok(mut pools) = self.pools.lock() {
            pools.push(pool_info.clone());
            return Ok(Some(pool_info));
        }

        Err(anyhow::anyhow!("Failed to lock pools mutex"))
    }

    async fn get_reserves(&self, pool: Address) -> Result<Vec<U256>> {
        // This is a placeholder implementation
        // In a real implementation, we would call the balances function on the pool contract

        // For now, just return dummy reserves for a stablecoin pool
        Ok(vec![
            U256::from(1000000000u128),             // 1000 USDC (6 decimals)
            U256::from(1000000000000000000000u128), // 1000 DAI (18 decimals)
        ])
    }

    async fn get_quote(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<TradeQuote> {
        // Call the get_best_rate function on the router
        let (pool_address, output_amount): (Address, U256) = self
            .router_contract
            .method::<_, (Address, U256)>(
                "get_best_rate",
                (input_token, output_token, input_amount),
            )?
            .call()
            .await?;

        // Get the pool
        let pool = self
            .get_pool(input_token, output_token)
            .await?
            .context("Pool not found")?;

        // Calculate the price impact
        let price_impact = 0; // Placeholder

        // Create the trade quote
        let quote = TradeQuote {
            input_token,
            output_token,
            input_amount,
            output_amount,
            price_impact,
            path: vec![input_token, output_token],
            pools: vec![pool_address],
            dex_type: DexType::Curve,
        };

        Ok(quote)
    }

    async fn find_best_path(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<Vec<Address>> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Find all possible paths between the tokens
        // 2. Get quotes for each path
        // 3. Return the path with the highest output amount

        // For Curve, we would also consider paths through stablecoins

        // For now, just return a direct path
        Ok(vec![input_token, output_token])
    }
}
