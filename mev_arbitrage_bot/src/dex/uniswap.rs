//! Uniswap Interface Module
//!
//! This module is responsible for interfacing with Uniswap V2.

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

/// Uniswap V2 interface
pub struct UniswapInterface {
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

/// Create a new Uniswap interface
pub async fn create_interface(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<dyn DexInterface>> {
    // Parse addresses
    let factory_address = match validate_and_parse_address(&config.dex.uniswap.factory_address) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Failed to parse uniswap factory address: {}", e);
            // Provide a fallback address for testing
            Address::from_low_u64_be(0)
        }
    };

    let router_address = match validate_and_parse_address(&config.dex.uniswap.router_address) {
        Ok(address) => address,
        Err(e) => {
            log::warn!("Failed to parse uniswap router address: {}", e);
            // Provide a fallback address for testing
            Address::from_low_u64_be(3)
        }
    };

    // Load ABIs
    let factory_abi = include_str!("./abi/uniswap_v2_factory.json");
    let factory_abi: Abi = serde_json::from_str(factory_abi).unwrap_or_else(|_| {
        // If the ABI file is not available, use a minimal ABI
        let json = r#"[
            {
                "constant": true,
                "inputs": [
                    {
                        "internalType": "address",
                        "name": "tokenA",
                        "type": "address"
                    },
                    {
                        "internalType": "address",
                        "name": "tokenB",
                        "type": "address"
                    }
                ],
                "name": "getPair",
                "outputs": [
                    {
                        "internalType": "address",
                        "name": "pair",
                        "type": "address"
                    }
                ],
                "payable": false,
                "stateMutability": "view",
                "type": "function"
            }
        ]"#;
        serde_json::from_str(json).expect("Failed to parse fallback ABI")
    });

    let router_abi = include_str!("./abi/uniswap_v2_router.json");
    let router_abi: Abi = serde_json::from_str(router_abi).unwrap_or_else(|_| {
        // If the ABI file is not available, use a minimal ABI
        let json = r#"[
            {
                "inputs": [
                    {
                        "internalType": "uint256",
                        "name": "amountIn",
                        "type": "uint256"
                    },
                    {
                        "internalType": "address[]",
                        "name": "path",
                        "type": "address[]"
                    }
                ],
                "name": "getAmountsOut",
                "outputs": [
                    {
                        "internalType": "uint256[]",
                        "name": "amounts",
                        "type": "uint256[]"
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

    let interface = UniswapInterface {
        name: "Uniswap V2".to_string(),
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
        warn!("Failed to initialize Uniswap pools: {}", e);
    }

    Ok(interface)
}

impl UniswapInterface {
    /// Initialize pools
    async fn initialize_pools(&self) -> Result<()> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Query the factory for all pair creation events
        // 2. Get the pool addresses
        // 3. Get the token addresses and reserves for each pool

        // For now, just create a dummy pool
        let weth_address =
            match validate_and_parse_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") {
                Ok(address) => address,
                Err(e) => {
                    log::warn!("Failed to parse WETH address: {}", e);
                    // Provide a fallback address for testing
                    Address::from_low_u64_be(6)
                }
            };

        let usdc_address =
            match validate_and_parse_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") {
                Ok(address) => address,
                Err(e) => {
                    log::warn!("Failed to parse USDC address: {}", e);
                    // Provide a fallback address for testing
                    Address::from_low_u64_be(7)
                }
            };

        let pool_address = self
            .factory_contract
            .method::<_, Address>("getPair", (weth_address, usdc_address))?
            .call()
            .await?;

        if pool_address != Address::zero() {
            let reserves = self.get_reserves(pool_address).await?;

            let pool_info = PoolInfo {
                address: pool_address,
                dex_type: DexType::UniswapV2,
                tokens: vec![weth_address, usdc_address],
                reserves,
                fee: 30, // 0.3%
            };

            // Add the pool to the list
            if let Ok(mut pools) = self.pools.lock() {
                pools.push(pool_info);
            }

            info!("Initialized Uniswap V2 WETH-USDC pool: {:?}", pool_address);
        }

        Ok(())
    }
}

#[async_trait]
impl DexInterface for UniswapInterface {
    fn name(&self) -> &str {
        &self.name
    }

    fn dex_type(&self) -> DexType {
        DexType::UniswapV2
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
                if (pool.tokens[0] == token_a && pool.tokens[1] == token_b)
                    || (pool.tokens[0] == token_b && pool.tokens[1] == token_a)
                {
                    return Ok(Some(pool.clone()));
                }
            }
        }

        // If not, query the factory
        let pool_address = self
            .factory_contract
            .method::<_, Address>("getPair", (token_a, token_b))?
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
            dex_type: DexType::UniswapV2,
            tokens: vec![token_a, token_b],
            reserves,
            fee: 30, // 0.3%
        };

        // Add the pool to the list
        if let Ok(mut pools) = self.pools.lock() {
            pools.push(pool_info.clone());
            return Ok(Some(pool_info));
        }

        Err(anyhow::anyhow!("Failed to lock pools mutex"))
    }

    async fn get_reserves(&self, pool: Address) -> Result<Vec<U256>> {
        // Create a minimal ABI for the pool contract
        let pool_abi = r#"[
            {
                "constant": true,
                "inputs": [],
                "name": "getReserves",
                "outputs": [
                    {
                        "internalType": "uint112",
                        "name": "_reserve0",
                        "type": "uint112"
                    },
                    {
                        "internalType": "uint112",
                        "name": "_reserve1",
                        "type": "uint112"
                    },
                    {
                        "internalType": "uint32",
                        "name": "_blockTimestampLast",
                        "type": "uint32"
                    }
                ],
                "payable": false,
                "stateMutability": "view",
                "type": "function"
            }
        ]"#;

        let pool_abi: ethers::abi::Abi = serde_json::from_str(pool_abi)
            .map_err(|e| anyhow::anyhow!("Failed to parse pool ABI: {}", e))?;

        // Create the pool contract
        let pool_contract =
            ethers::contract::Contract::new(pool, pool_abi, self.blockchain_client.clone());

        // Call getReserves
        let result: (U256, U256, u32) = pool_contract
            .method::<_, (U256, U256, u32)>("getReserves", ())?
            .call()
            .await?;

        // Return the reserves
        Ok(vec![result.0, result.1])
    }

    async fn get_quote(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<TradeQuote> {
        // Create the path
        let path = vec![input_token, output_token];

        // Call the getAmountsOut function on the router
        let amounts: Vec<U256> = self
            .router_contract
            .method::<_, Vec<U256>>("getAmountsOut", (input_amount, path.clone()))?
            .call()
            .await?;

        // Get the output amount
        let output_amount = amounts[1];

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
            path,
            pools: vec![pool.address],
            dex_type: DexType::UniswapV2,
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

        // For now, just return a direct path
        Ok(vec![input_token, output_token])
    }
}
