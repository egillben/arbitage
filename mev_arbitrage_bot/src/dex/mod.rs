//! DEX Interface Module
//!
//! This module is responsible for interfacing with decentralized exchanges.

mod curve;
mod sushiswap;
mod uniswap;

use anyhow::Result;
use async_trait::async_trait;
use ethers::providers::Provider;
use ethers::types::{Address, U256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;

/// DEX type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexType {
    /// Uniswap V2
    UniswapV2,

    /// Sushiswap
    Sushiswap,

    /// Curve
    Curve,
}

/// Pool information
#[derive(Debug, Clone)]
pub struct PoolInfo {
    /// Pool address
    pub address: Address,

    /// DEX type
    pub dex_type: DexType,

    /// Token addresses
    pub tokens: Vec<Address>,

    /// Token reserves
    pub reserves: Vec<U256>,

    /// Pool fee (in basis points)
    pub fee: u32,
}

/// Trade quote
#[derive(Debug, Clone)]
pub struct TradeQuote {
    /// Input token
    pub input_token: Address,

    /// Output token
    pub output_token: Address,

    /// Input amount
    pub input_amount: U256,

    /// Expected output amount
    pub output_amount: U256,

    /// Price impact (in basis points)
    pub price_impact: u32,

    /// Path of tokens
    pub path: Vec<Address>,

    /// Path of pools
    pub pools: Vec<Address>,

    /// DEX type
    pub dex_type: DexType,
}

/// Interface for DEX interactions
#[async_trait]
pub trait DexInterface: Send + Sync {
    /// Get the name of the DEX
    fn name(&self) -> &str;

    /// Get the type of the DEX
    fn dex_type(&self) -> DexType;

    /// Get the factory address
    fn factory_address(&self) -> Address;

    /// Get the router address
    fn router_address(&self) -> Address;

    /// Get all pools
    async fn get_pools(&self) -> Result<Vec<PoolInfo>>;

    /// Get a specific pool
    async fn get_pool(&self, token_a: Address, token_b: Address) -> Result<Option<PoolInfo>>;

    /// Get the reserves for a pool
    async fn get_reserves(&self, pool: Address) -> Result<Vec<U256>>;

    /// Get a quote for a trade
    async fn get_quote(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<TradeQuote>;

    /// Find the best path for a trade
    async fn find_best_path(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<Vec<Address>>;
}

/// Collection of DEX interfaces
pub struct DexInterfaces {
    interfaces: HashMap<DexType, Arc<dyn DexInterface>>,
    test_mode: bool,
}

impl DexInterfaces {
    /// Create a new collection of DEX interfaces
    pub fn new(test_mode: bool) -> Self {
        Self {
            interfaces: HashMap::new(),
            test_mode,
        }
    }

    /// Add a DEX interface
    pub fn add_interface(&mut self, interface: Arc<dyn DexInterface>) {
        self.interfaces.insert(interface.dex_type(), interface);
    }

    /// Get a DEX interface by type
    pub fn get_interface(&self, dex_type: DexType) -> Option<Arc<dyn DexInterface>> {
        self.interfaces.get(&dex_type).cloned()
    }

    /// Get all DEX interfaces
    pub fn get_all_interfaces(&self) -> Vec<Arc<dyn DexInterface>> {
        self.interfaces.values().cloned().collect()
    }

    /// Get a quote from all DEXes
    pub async fn get_quotes(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<Vec<TradeQuote>> {
        let mut quotes = Vec::new();

        for interface in self.interfaces.values() {
            match interface
                .get_quote(input_token, output_token, input_amount)
                .await
            {
                Ok(quote) => {
                    quotes.push(quote);
                }
                Err(e) => {
                    // In test mode, log expected errors at debug level instead of warn
                    if self.test_mode
                        && (e.to_string().contains("Invalid data")
                            || e.to_string().contains("Invalid name"))
                    {
                        log::debug!("Failed to get quote from {}: {}", interface.name(), e);
                    } else {
                        log::warn!("Failed to get quote from {}: {}", interface.name(), e);
                    }
                }
            }
        }

        Ok(quotes)
    }

    /// Find the best quote across all DEXes
    pub async fn find_best_quote(
        &self,
        input_token: Address,
        output_token: Address,
        input_amount: U256,
    ) -> Result<Option<TradeQuote>> {
        let quotes = self
            .get_quotes(input_token, output_token, input_amount)
            .await?;

        if quotes.is_empty() {
            return Ok(None);
        }

        // Find the quote with the highest output amount
        let best_quote = quotes
            .into_iter()
            .max_by(|a, b| a.output_amount.cmp(&b.output_amount))
            .unwrap();

        Ok(Some(best_quote))
    }
}

/// Create DEX interfaces
pub async fn create_interfaces(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<DexInterfaces>> {
    let mut interfaces = DexInterfaces::new(config.test_mode);

    // Create Uniswap interface if enabled
    if config.dex.uniswap.enabled {
        let uniswap_interface =
            uniswap::create_interface(config, blockchain_client.clone()).await?;
        interfaces.add_interface(uniswap_interface);
    }

    // Create Sushiswap interface if enabled
    if config.dex.sushiswap.enabled {
        let sushiswap_interface =
            sushiswap::create_interface(config, blockchain_client.clone()).await?;
        interfaces.add_interface(sushiswap_interface);
    }

    // Create Curve interface if enabled
    if config.dex.curve.enabled {
        let curve_interface = curve::create_interface(config, blockchain_client.clone()).await?;
        interfaces.add_interface(curve_interface);
    }

    Ok(Arc::new(interfaces))
}
