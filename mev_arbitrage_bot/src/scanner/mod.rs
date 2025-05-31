//! Opportunity Scanner Module
//!
//! This module is responsible for monitoring DEX prices and identifying arbitrage opportunities.

use anyhow::Result;
use async_trait::async_trait;
use ethers::providers::Provider;
use ethers::types::{Address, U256};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::dex::{DexInterfaces, DexType, TradeQuote};
use crate::price::{PriceOracle, PriceOracleInterface};
use crate::utils::validate_and_parse_address;

/// Represents an arbitrage opportunity between different DEXes
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    /// Unique identifier for the opportunity
    pub id: String,

    /// Timestamp when the opportunity was identified
    pub timestamp: u64,

    /// Source DEX for the arbitrage
    pub source_dex: String,

    /// Target DEX for the arbitrage
    pub target_dex: String,

    /// Token path for the arbitrage (e.g., [WETH, USDC, DAI, WETH])
    pub token_path: Vec<Address>,

    /// Estimated profit in USD
    pub estimated_profit: f64,

    /// Required flash loan amount in USD
    pub required_loan_amount: f64,

    /// Estimated gas cost in USD
    pub estimated_gas_cost: f64,

    /// Net profit after gas costs
    pub net_profit: f64,

    /// Confidence score (0-100)
    pub confidence_score: u8,
}

/// Interface for opportunity scanners
#[async_trait]
pub trait OpportunityScanner: Send + Sync {
    /// Scan for arbitrage opportunities
    async fn scan(&self) -> Result<Vec<ArbitrageOpportunity>>;

    /// Start continuous scanning
    async fn start_continuous_scanning(&self) -> Result<()>;

    /// Stop continuous scanning
    async fn stop_continuous_scanning(&self) -> Result<()>;
}

/// Implementation of the opportunity scanner
#[derive(Clone)]
pub struct OpportunityScannerImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    dex_interfaces: Arc<DexInterfaces>,
    price_oracle: Arc<PriceOracle>,
    is_scanning: Arc<RwLock<bool>>,
}

/// Create a new opportunity scanner
pub async fn create_scanner(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    dex_interfaces: Arc<DexInterfaces>,
    price_oracle: Arc<PriceOracle>,
) -> Result<Arc<dyn OpportunityScanner>> {
    let scanner = OpportunityScannerImpl {
        config: config.clone(),
        blockchain_client,
        dex_interfaces,
        price_oracle,
        is_scanning: Arc::new(RwLock::new(false)),
    };

    Ok(Arc::new(scanner))
}

#[async_trait]
impl OpportunityScanner for OpportunityScannerImpl {
    async fn scan(&self) -> Result<Vec<ArbitrageOpportunity>> {
        info!("Scanning for arbitrage opportunities...");
        let mut opportunities = Vec::new();

        // Get the list of tokens we're interested in
        let tokens = &self.config.flash_loan.tokens;
        if tokens.is_empty() {
            warn!("No tokens configured for scanning");
            return Ok(Vec::new());
        }

        // For each pair of tokens, check for arbitrage opportunities
        for i in 0..tokens.len() {
            for j in 0..tokens.len() {
                if i == j {
                    continue; // Skip same token pairs
                }

                let token_a = match validate_and_parse_address(&tokens[i].address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        warn!("Invalid token address {}: {}", tokens[i].address, e);
                        continue;
                    }
                };

                let token_b = match validate_and_parse_address(&tokens[j].address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        warn!("Invalid token address {}: {}", tokens[j].address, e);
                        continue;
                    }
                };

                // Get quotes from all DEXes for this token pair
                let input_amount = U256::from(10).pow(U256::from(tokens[i].decimals));
                match self
                    .dex_interfaces
                    .get_quotes(token_a, token_b, input_amount)
                    .await
                {
                    Ok(quotes) => {
                        if quotes.len() < 2 {
                            // Need at least 2 DEXes to compare
                            continue;
                        }

                        // Find the best buy and sell prices
                        let mut best_buy_quote: Option<TradeQuote> = None;
                        let mut best_sell_quote: Option<TradeQuote> = None;

                        for quote in &quotes {
                            if best_buy_quote.is_none()
                                || quote.output_amount
                                    > best_buy_quote.as_ref().unwrap().output_amount
                            {
                                best_buy_quote = Some(quote.clone());
                            }

                            if best_sell_quote.is_none()
                                || quote.output_amount
                                    < best_sell_quote.as_ref().unwrap().output_amount
                            {
                                best_sell_quote = Some(quote.clone());
                            }
                        }

                        // If we have both quotes, check for arbitrage opportunity
                        if let (Some(buy_quote), Some(sell_quote)) =
                            (best_buy_quote, best_sell_quote)
                        {
                            if buy_quote.output_amount > sell_quote.output_amount {
                                // There's a potential arbitrage opportunity

                                // Calculate profit in token B
                                let profit_in_token_b = buy_quote
                                    .output_amount
                                    .saturating_sub(sell_quote.output_amount);

                                // Convert profit to USD
                                let token_b_price_usd = match PriceOracleInterface::get_price_usd(
                                    &*self.price_oracle,
                                    token_b,
                                )
                                .await
                                {
                                    Ok(price) => price,
                                    Err(e) => {
                                        warn!(
                                            "Failed to get USD price for token {:?}: {}",
                                            token_b, e
                                        );
                                        continue;
                                    }
                                };

                                // Calculate profit in USD
                                let decimals = tokens[j].decimals as u32;
                                let profit_usd = (profit_in_token_b.as_u128() as f64
                                    / 10f64.powi(decimals as i32))
                                    * token_b_price_usd;

                                // Calculate required loan amount
                                let token_a_price_usd = match PriceOracleInterface::get_price_usd(
                                    &*self.price_oracle,
                                    token_a,
                                )
                                .await
                                {
                                    Ok(price) => price,
                                    Err(e) => {
                                        warn!(
                                            "Failed to get USD price for token {:?}: {}",
                                            token_a, e
                                        );
                                        continue;
                                    }
                                };

                                let loan_amount_usd = (input_amount.as_u128() as f64
                                    / 10f64.powi(tokens[i].decimals as i32))
                                    * token_a_price_usd;

                                // Estimate gas cost (this would be more accurate in a real implementation)
                                let estimated_gas_cost = 0.01; // $0.01 for simplicity

                                // Calculate net profit
                                let net_profit = profit_usd - estimated_gas_cost;

                                // Only consider opportunities with positive net profit
                                if net_profit > 0.0 {
                                    // Create a unique ID for this opportunity
                                    let id = format!(
                                        "{}_{}_{}_{}",
                                        tokens[i].symbol,
                                        tokens[j].symbol,
                                        buy_quote.dex_type as u8,
                                        sell_quote.dex_type as u8
                                    );

                                    // Get DEX names
                                    let source_dex = format!("{:?}", buy_quote.dex_type);
                                    let target_dex = format!("{:?}", sell_quote.dex_type);

                                    // Create token path
                                    let token_path = vec![token_a, token_b, token_a];

                                    // Create the opportunity
                                    let opportunity = ArbitrageOpportunity {
                                        id,
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                        source_dex,
                                        target_dex,
                                        token_path,
                                        estimated_profit: profit_usd,
                                        required_loan_amount: loan_amount_usd,
                                        estimated_gas_cost,
                                        net_profit,
                                        confidence_score: 80, // Arbitrary confidence score
                                    };

                                    info!(
                                        "Found arbitrage opportunity: {} -> {} with profit: ${:.2}",
                                        opportunity.source_dex,
                                        opportunity.target_dex,
                                        opportunity.net_profit
                                    );

                                    opportunities.push(opportunity);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to get quotes for token pair {:?} -> {:?}: {}",
                            token_a, token_b, e
                        );
                        continue;
                    }
                }
            }
        }

        debug!("Scan complete. Found {} opportunities", opportunities.len());
        Ok(opportunities)
    }

    async fn start_continuous_scanning(&self) -> Result<()> {
        let mut is_scanning = self.is_scanning.write().await;
        if *is_scanning {
            info!("Continuous scanning already running");
            return Ok(());
        }

        *is_scanning = true;
        info!("Starting continuous scanning for arbitrage opportunities");

        // Clone necessary references for the background task
        let scanner = Arc::new(self.clone());

        // Start a background task to continuously scan for opportunities
        tokio::spawn(async move {
            let scanner = scanner;

            while *scanner.is_scanning.read().await {
                // Scan for opportunities
                match scanner.scan().await {
                    Ok(opportunities) => {
                        if !opportunities.is_empty() {
                            info!(
                                "Continuous scan found {} arbitrage opportunities",
                                opportunities.len()
                            );

                            // In a real implementation, we would process these opportunities
                            // For now, just log them
                            for opportunity in &opportunities {
                                info!(
                                    "Opportunity: {} -> {} with profit: ${:.2}",
                                    opportunity.source_dex,
                                    opportunity.target_dex,
                                    opportunity.net_profit
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error during continuous scanning: {}", e);
                    }
                }

                // Sleep for a configurable interval
                // In test mode, scan less frequently (every 10 seconds) to reduce log spam
                let sleep_duration = if scanner.config.test_mode {
                    10000 // 10 seconds in test mode
                } else {
                    1000 // 1 second in normal mode
                };

                if scanner.config.test_mode {
                    debug!("Test mode: Sleeping for 10 seconds between scans");
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_duration)).await;
            }

            info!("Continuous scanning stopped");
        });

        Ok(())
    }

    async fn stop_continuous_scanning(&self) -> Result<()> {
        let mut is_scanning = self.is_scanning.write().await;
        if !*is_scanning {
            info!("Continuous scanning is not running");
            return Ok(());
        }

        *is_scanning = false;
        info!("Stopping continuous scanning for arbitrage opportunities");

        Ok(())
    }
}
