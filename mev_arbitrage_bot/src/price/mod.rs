//! Price Oracle Module
//!
//! This module is responsible for maintaining price data from various sources.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::providers::Provider;
use ethers::types::{Address, U256};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

use crate::config::{Config, TokenConfig};
use crate::utils::validate_and_parse_address;

/// Price source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriceSource {
    /// On-chain DEX price
    Dex(DexSource),

    /// Off-chain price API
    Api(ApiSource),
}

/// DEX price source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexSource {
    /// Uniswap V2
    UniswapV2,

    /// Sushiswap
    Sushiswap,

    /// Curve
    Curve,
}

/// API price source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiSource {
    /// CoinGecko
    CoinGecko,

    /// CoinMarketCap
    CoinMarketCap,
}

/// Token price data
#[derive(Debug, Clone)]
pub struct TokenPrice {
    /// Token address
    pub token: Address,

    /// Token symbol
    pub symbol: String,

    /// Price in USD
    pub price_usd: f64,

    /// Price in ETH
    pub price_eth: f64,

    /// Price sources
    pub sources: HashMap<PriceSource, f64>,

    /// Last update timestamp
    pub last_update: Instant,
}

/// Interface for price oracles
#[async_trait]
pub trait PriceOracleInterface: Send + Sync {
    /// Get the price of a token in USD
    async fn get_price_usd(&self, token: Address) -> Result<f64>;

    /// Get the price of a token in ETH
    async fn get_price_eth(&self, token: Address) -> Result<f64>;

    /// Get the price of a token in terms of another token
    async fn get_price_in_token(&self, base_token: Address, quote_token: Address) -> Result<f64>;

    /// Update all prices
    async fn update_prices(&self) -> Result<()>;

    /// Add a price source
    async fn add_price_source(&self, source: PriceSource) -> Result<()>;

    /// Remove a price source
    async fn remove_price_source(&self, source: PriceSource) -> Result<()>;
}

/// Implementation of the price oracle
pub struct PriceOracle {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    prices: RwLock<HashMap<Address, TokenPrice>>,
    sources: RwLock<Vec<PriceSource>>,
    last_update: RwLock<Instant>,
}

/// Create a new price oracle
pub async fn create_oracle(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<PriceOracle>> {
    let oracle = PriceOracle {
        config: config.clone(),
        blockchain_client,
        prices: RwLock::new(HashMap::new()),
        sources: RwLock::new(vec![
            PriceSource::Dex(DexSource::UniswapV2),
            PriceSource::Dex(DexSource::Sushiswap),
            PriceSource::Api(ApiSource::CoinGecko),
        ]),
        last_update: RwLock::new(Instant::now() - Duration::from_secs(3600)), // Force an update on first call
    };

    // Initialize prices for configured tokens
    let oracle = Arc::new(oracle);
    oracle.initialize_prices().await?;

    Ok(oracle)
}

impl PriceOracle {
    /// Initialize prices for configured tokens
    async fn initialize_prices(&self) -> Result<()> {
        // Get the list of tokens from the config
        let tokens = &self.config.flash_loan.tokens;

        // Initialize prices for each token
        for token in tokens {
            self.initialize_token_price(token).await?;
        }

        // Update all prices
        self.update_prices().await?;

        Ok(())
    }

    /// Initialize price data for a token
    async fn initialize_token_price(&self, token_config: &TokenConfig) -> Result<()> {
        let token_address = match validate_and_parse_address(&token_config.address) {
            Ok(address) => address,
            Err(e) => {
                log::warn!(
                    "Failed to parse token address for {}: {}",
                    token_config.symbol,
                    e
                );
                // Provide a fallback address for testing
                Address::from_low_u64_be(20 + token_config.symbol.len() as u64)
            }
        };

        let token_price = TokenPrice {
            token: token_address,
            symbol: token_config.symbol.clone(),
            price_usd: 0.0,
            price_eth: 0.0,
            sources: HashMap::new(),
            last_update: Instant::now(),
        };

        let mut prices = self.prices.write().await;
        prices.insert(token_address, token_price);

        Ok(())
    }

    /// Get price from a specific source
    async fn get_price_from_source(&self, token: Address, source: PriceSource) -> Result<f64> {
        match source {
            PriceSource::Dex(dex_source) => self.get_price_from_dex(token, dex_source).await,
            PriceSource::Api(api_source) => self.get_price_from_api(token, api_source).await,
        }
    }

    /// Get price from a DEX
    async fn get_price_from_dex(&self, token: Address, dex_source: DexSource) -> Result<f64> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Get the DEX contract
        // 2. Get the token pair
        // 3. Get the reserves
        // 4. Calculate the price

        // For now, just return a dummy price
        match dex_source {
            DexSource::UniswapV2 => Ok(1000.0), // Assume 1 ETH = $1000
            DexSource::Sushiswap => Ok(1010.0), // Slight variation
            DexSource::Curve => Ok(990.0),      // Slight variation
        }
    }

    /// Get price from an API
    async fn get_price_from_api(&self, token: Address, api_source: ApiSource) -> Result<f64> {
        // This is a placeholder implementation
        // In a real implementation, we would:
        // 1. Make an HTTP request to the API
        // 2. Parse the response
        // 3. Extract the price

        // For now, just return a dummy price
        match api_source {
            ApiSource::CoinGecko => Ok(1005.0),    // Assume 1 ETH = $1005
            ApiSource::CoinMarketCap => Ok(995.0), // Slight variation
        }
    }

    /// Calculate the median price from multiple sources
    fn calculate_median_price(&self, prices: &[f64]) -> Option<f64> {
        if prices.is_empty() {
            return None;
        }

        let mut sorted_prices = prices.to_vec();
        sorted_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mid = sorted_prices.len() / 2;
        if sorted_prices.len() % 2 == 0 {
            Some((sorted_prices[mid - 1] + sorted_prices[mid]) / 2.0)
        } else {
            Some(sorted_prices[mid])
        }
    }

    /// Check if a price is within acceptable deviation
    fn is_price_within_deviation(&self, price: f64, median: f64) -> bool {
        let deviation = (price - median).abs() / median * 100.0;
        deviation <= self.config.security.max_price_deviation
    }
}

#[async_trait]
impl PriceOracleInterface for PriceOracle {
    async fn get_price_usd(&self, token: Address) -> Result<f64> {
        // Check if we need to update prices
        let last_update = *self.last_update.read().await;
        if last_update.elapsed() > Duration::from_secs(60) {
            self.update_prices().await?;
        }

        // Get the price from the cache
        let prices = self.prices.read().await;
        let token_price = prices
            .get(&token)
            .context(format!("Price not found for token: {:?}", token))?;

        Ok(token_price.price_usd)
    }

    async fn get_price_eth(&self, token: Address) -> Result<f64> {
        // Check if we need to update prices
        let last_update = *self.last_update.read().await;
        if last_update.elapsed() > Duration::from_secs(60) {
            self.update_prices().await?;
        }

        // Get the price from the cache
        let prices = self.prices.read().await;
        let token_price = prices
            .get(&token)
            .context(format!("Price not found for token: {:?}", token))?;

        Ok(token_price.price_eth)
    }

    async fn get_price_in_token(&self, base_token: Address, quote_token: Address) -> Result<f64> {
        // Get the prices in USD
        let base_price_usd = self.get_price_usd(base_token).await?;
        let quote_price_usd = self.get_price_usd(quote_token).await?;

        // Calculate the price in terms of the quote token
        if quote_price_usd == 0.0 {
            return Err(anyhow::anyhow!("Quote token price is zero"));
        }

        Ok(base_price_usd / quote_price_usd)
    }

    async fn update_prices(&self) -> Result<()> {
        // Get the list of tokens
        let tokens = {
            let prices = self.prices.read().await;
            prices.keys().cloned().collect::<Vec<_>>()
        };

        // Get the list of sources
        let sources = {
            let sources = self.sources.read().await;
            sources.clone()
        };

        // Update prices for each token
        for token in tokens {
            // Get prices from all sources
            let mut token_prices = HashMap::new();
            for source in &sources {
                match self.get_price_from_source(token, *source).await {
                    Ok(price) => {
                        token_prices.insert(*source, price);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to get price for token {:?} from source {:?}: {}",
                            token, source, e
                        );
                    }
                }
            }

            // Calculate the median price
            let prices_vec = token_prices.values().cloned().collect::<Vec<_>>();
            if let Some(median_price) = self.calculate_median_price(&prices_vec) {
                // Filter out prices that deviate too much
                let filtered_prices = token_prices
                    .iter()
                    .filter(|(_, &price)| self.is_price_within_deviation(price, median_price))
                    .map(|(&source, &price)| (source, price))
                    .collect::<HashMap<_, _>>();

                // Calculate the final price as the average of filtered prices
                let final_price = if filtered_prices.is_empty() {
                    median_price
                } else {
                    filtered_prices.values().sum::<f64>() / filtered_prices.len() as f64
                };

                // Update the price in the cache
                let mut prices = self.prices.write().await;
                if let Some(token_price) = prices.get_mut(&token) {
                    token_price.price_usd = final_price;
                    token_price.sources = filtered_prices;
                    token_price.last_update = Instant::now();

                    // For ETH, price in ETH is always 1.0
                    if token == Address::from_low_u64_be(0) {
                        token_price.price_eth = 1.0;
                    }
                }
                drop(prices);

                // Calculate the price in ETH for non-ETH tokens in a separate step
                if token != Address::from_low_u64_be(0) {
                    // Get the ETH price
                    let eth_price_usd = {
                        let prices = self.prices.read().await;
                        if let Some(eth_price) = prices.get(&Address::from_low_u64_be(0)) {
                            eth_price.price_usd
                        } else {
                            0.0
                        }
                    };

                    // Update the token price in ETH if we have a valid ETH price
                    if eth_price_usd > 0.0 {
                        let token_price_usd = {
                            let prices = self.prices.read().await;
                            if let Some(token_price) = prices.get(&token) {
                                token_price.price_usd
                            } else {
                                0.0
                            }
                        };

                        let mut prices = self.prices.write().await;
                        if let Some(token_price) = prices.get_mut(&token) {
                            token_price.price_eth = token_price_usd / eth_price_usd;
                        }
                    }
                }

                // Log the updated price
                {
                    let prices = self.prices.read().await;
                    if let Some(token_price) = prices.get(&token) {
                        debug!(
                            "Updated price for token {}: ${:.2} (${:.2} ETH)",
                            token_price.symbol, token_price.price_usd, token_price.price_eth
                        );
                    }
                }
            } else {
                warn!(
                    "Failed to calculate median price for token {:?}: no valid prices",
                    token
                );
            }
        }

        // Update the last update timestamp
        let mut last_update = self.last_update.write().await;
        *last_update = Instant::now();

        Ok(())
    }

    async fn add_price_source(&self, source: PriceSource) -> Result<()> {
        let mut sources = self.sources.write().await;
        if !sources.contains(&source) {
            sources.push(source);
            debug!("Added price source: {:?}", source);
        }

        Ok(())
    }

    async fn remove_price_source(&self, source: PriceSource) -> Result<()> {
        let mut sources = self.sources.write().await;
        if let Some(index) = sources.iter().position(|&s| s == source) {
            sources.remove(index);
            debug!("Removed price source: {:?}", source);
        }

        Ok(())
    }
}
