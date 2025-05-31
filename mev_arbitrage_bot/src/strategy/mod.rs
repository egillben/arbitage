//! Arbitrage Strategy Engine Module
//!
//! This module is responsible for evaluating arbitrage opportunities and determining optimal trade paths.

use anyhow::Result;
use async_trait::async_trait;
use ethers::types::{Address, U256};
use std::sync::Arc;

use crate::config::Config;
use crate::dex::{DexInterfaces, DexType};
use crate::flash_loan::FlashLoanManager;
use crate::price::{PriceOracle, PriceOracleInterface};
use crate::scanner::ArbitrageOpportunity;

/// Interface for arbitrage strategy engines
#[async_trait]
pub trait StrategyEngine: Send + Sync {
    /// Evaluate a list of arbitrage opportunities and select the best one
    async fn evaluate_opportunities(
        &self,
        opportunities: Vec<ArbitrageOpportunity>,
    ) -> Option<ArbitrageOpportunity>;

    /// Find the optimal trade path for a given token pair
    async fn find_optimal_path(
        &self,
        from_token: Address,
        to_token: Address,
    ) -> Result<Vec<Address>>;

    /// Calculate the expected profit for a given trade path
    async fn calculate_expected_profit(&self, path: &[Address], amount: f64) -> Result<f64>;
}

/// Implementation of the arbitrage strategy engine
pub struct StrategyEngineImpl {
    config: Arc<Config>,
    price_oracle: Arc<PriceOracle>,
    dex_interfaces: Arc<DexInterfaces>,
    flash_loan_manager: Arc<dyn FlashLoanManager>,
}

/// Create a new arbitrage strategy engine
pub async fn create_engine(
    config: &Arc<Config>,
    price_oracle: Arc<PriceOracle>,
    dex_interfaces: Arc<DexInterfaces>,
    flash_loan_manager: Arc<dyn FlashLoanManager>,
) -> Result<Arc<dyn StrategyEngine>> {
    let engine = StrategyEngineImpl {
        config: config.clone(),
        price_oracle,
        dex_interfaces,
        flash_loan_manager,
    };

    Ok(Arc::new(engine))
}

impl StrategyEngineImpl {
    /// Get the decimals for a token
    async fn get_token_decimals(&self, token: Address) -> Result<u8> {
        // In a real implementation, we would query the token contract
        // For now, use a default value or look up in config

        for token_config in &self.config.flash_loan.tokens {
            if let Ok(token_address) =
                crate::utils::validate_and_parse_address(&token_config.address)
            {
                if token_address == token {
                    return Ok(token_config.decimals);
                }
            }
        }

        // Default to 18 decimals if not found
        Ok(18)
    }

    /// Estimate gas cost for a trade path
    async fn estimate_gas_cost(
        &self,
        path_length: usize,
        dex_types: Vec<crate::dex::DexType>,
    ) -> Result<f64> {
        // Base gas cost for a flash loan
        let mut gas_cost = 0.005; // $0.005 base cost

        // Add cost based on path length
        gas_cost += match path_length {
            2 => 0.001, // Direct path
            3 => 0.002, // One intermediate token
            _ => 0.004, // Multiple intermediate tokens
        };

        // Add cost based on DEX types (some DEXes are more gas-intensive)
        for dex_type in dex_types {
            gas_cost += match dex_type {
                crate::dex::DexType::UniswapV2 => 0.001,
                crate::dex::DexType::Sushiswap => 0.001,
                crate::dex::DexType::Curve => 0.002, // Curve is typically more gas-intensive
            };
        }

        // Apply a fixed multiplier for gas cost estimation
        gas_cost *= 1.2; // Use a reasonable default multiplier

        Ok(gas_cost)
    }
}

#[async_trait]
impl StrategyEngine for StrategyEngineImpl {
    async fn evaluate_opportunities(
        &self,
        opportunities: Vec<ArbitrageOpportunity>,
    ) -> Option<ArbitrageOpportunity> {
        if opportunities.is_empty() {
            return None;
        }

        // Filter out opportunities below the profit threshold
        let profitable_opportunities: Vec<ArbitrageOpportunity> = opportunities
            .into_iter()
            .filter(|op| op.net_profit > self.config.arbitrage.min_profit_threshold)
            .collect();

        if profitable_opportunities.is_empty() {
            log::info!("No profitable arbitrage opportunities found after filtering");
            return None;
        }

        // Calculate gas costs and adjust net profit
        let mut evaluated_opportunities = Vec::new();
        for mut opportunity in profitable_opportunities {
            // Estimate gas cost based on the token path length
            let estimated_gas = match opportunity.token_path.len() {
                3 => 0.005, // Simple path
                4 => 0.008, // Medium complexity
                _ => 0.012, // Complex path
            };

            // Update gas cost and net profit
            opportunity.estimated_gas_cost = estimated_gas;
            opportunity.net_profit = opportunity.estimated_profit - estimated_gas;

            // Only include if still profitable after gas costs
            if opportunity.net_profit > self.config.arbitrage.min_profit_threshold {
                evaluated_opportunities.push(opportunity);
            }
        }

        if evaluated_opportunities.is_empty() {
            log::info!("No profitable arbitrage opportunities found after gas cost evaluation");
            return None;
        }

        // Sort by net profit (descending)
        evaluated_opportunities.sort_by(|a, b| {
            b.net_profit
                .partial_cmp(&a.net_profit)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return the opportunity with the highest net profit
        let best_opportunity = evaluated_opportunities.remove(0);
        log::info!(
            "Selected best arbitrage opportunity: {} -> {} via {} with net profit: ${:.2}",
            best_opportunity.source_dex,
            best_opportunity.target_dex,
            best_opportunity.token_path.len() - 1,
            best_opportunity.net_profit
        );

        Some(best_opportunity)
    }

    async fn find_optimal_path(
        &self,
        from_token: Address,
        to_token: Address,
    ) -> Result<Vec<Address>> {
        log::info!(
            "Finding optimal path from {:?} to {:?}",
            from_token,
            to_token
        );

        // Get all available DEX interfaces
        let dex_interfaces = self.dex_interfaces.get_all_interfaces();
        if dex_interfaces.is_empty() {
            return Err(anyhow::anyhow!("No DEX interfaces available"));
        }

        // Define possible intermediate tokens
        let mut intermediate_tokens = Vec::new();

        // Add common tokens from config
        for token_config in &self.config.flash_loan.tokens {
            if let Ok(token_address) =
                crate::utils::validate_and_parse_address(&token_config.address)
            {
                if token_address != from_token && token_address != to_token {
                    intermediate_tokens.push(token_address);
                }
            }
        }

        // Define possible paths to check
        let mut paths = Vec::new();

        // Direct path
        paths.push(vec![from_token, to_token]);

        // Single-hop paths through intermediate tokens
        for &intermediate in &intermediate_tokens {
            paths.push(vec![from_token, intermediate, to_token]);
        }

        // Two-hop paths through pairs of intermediate tokens
        for i in 0..intermediate_tokens.len() {
            for j in i + 1..intermediate_tokens.len() {
                paths.push(vec![
                    from_token,
                    intermediate_tokens[i],
                    intermediate_tokens[j],
                    to_token,
                ]);
            }
        }

        // Calculate expected profit for each path
        let mut best_path = None;
        let mut best_profit = 0.0;

        // Use a standard amount for comparison
        let amount = 1.0; // 1 unit of from_token

        for path in paths {
            match self.calculate_expected_profit(&path, amount).await {
                Ok(profit) => {
                    log::debug!(
                        "Path {:?} has expected profit: ${:.2}",
                        path.iter()
                            .map(|&addr| format!("{:?}", addr))
                            .collect::<Vec<_>>()
                            .join(" -> "),
                        profit
                    );

                    if profit > best_profit {
                        best_profit = profit;
                        best_path = Some(path);
                    }
                }
                Err(e) => {
                    log::debug!("Failed to calculate profit for path: {:?}", e);
                }
            }
        }

        if let Some(path) = best_path {
            log::info!(
                "Found optimal path with expected profit: ${:.2}",
                best_profit
            );
            Ok(path)
        } else {
            Err(anyhow::anyhow!("No profitable path found"))
        }
    }

    async fn calculate_expected_profit(&self, path: &[Address], amount: f64) -> Result<f64> {
        if path.len() < 2 {
            return Err(anyhow::anyhow!("Path must contain at least 2 tokens"));
        }

        // Convert amount to U256
        let from_token = path[0];
        let from_token_decimals = self.get_token_decimals(from_token).await?;
        let input_amount =
            ethers::utils::parse_units(amount.to_string(), from_token_decimals as usize)?.into();

        // Simulate the trades along the path
        let mut current_amount = input_amount;
        let mut dex_used = Vec::new();

        for i in 0..path.len() - 1 {
            let token_in = path[i];
            let token_out = path[i + 1];

            // Get the best quote for this token pair
            let best_quote = match self
                .dex_interfaces
                .find_best_quote(token_in, token_out, current_amount)
                .await?
            {
                Some(quote) => quote,
                None => {
                    return Err(anyhow::anyhow!(
                        "No quote available for {} -> {}",
                        token_in,
                        token_out
                    ))
                }
            };

            // Update current amount and record the DEX used
            current_amount = best_quote.output_amount;
            dex_used.push(best_quote.dex_type);

            log::debug!(
                "Step {}: {} -> {} on {:?}, amount: {} -> {}",
                i + 1,
                token_in,
                token_out,
                best_quote.dex_type,
                input_amount,
                current_amount
            );
        }

        // Calculate profit in the original token
        let profit_in_token = if path[0] == path[path.len() - 1] {
            // If it's a circular path, we can directly compare
            if current_amount > input_amount {
                current_amount.saturating_sub(input_amount)
            } else {
                return Ok(0.0); // No profit
            }
        } else {
            // If it's not circular, we need to convert back to the original token
            // This is a simplified approach
            let final_token = path[path.len() - 1];
            let final_token_price =
                PriceOracleInterface::get_price_usd(&*self.price_oracle, final_token).await?;
            let from_token_price =
                PriceOracleInterface::get_price_usd(&*self.price_oracle, from_token).await?;

            if from_token_price <= 0.0 {
                return Err(anyhow::anyhow!("Invalid price for from_token"));
            }

            let final_token_decimals = self.get_token_decimals(final_token).await?;
            let final_amount_f64 = ethers::utils::format_units(
                current_amount.as_u128(),
                final_token_decimals as usize,
            )?
            .parse::<f64>()?;

            let final_value_usd = final_amount_f64 * final_token_price;
            let initial_value_usd = amount * from_token_price;

            if final_value_usd > initial_value_usd {
                let profit_usd = final_value_usd - initial_value_usd;
                let profit_in_from_token = profit_usd / from_token_price;

                ethers::utils::parse_units(
                    profit_in_from_token.to_string(),
                    from_token_decimals as usize,
                )?
                .into()
            } else {
                return Ok(0.0); // No profit
            }
        };

        // Convert profit to USD
        let from_token_price =
            PriceOracleInterface::get_price_usd(&*self.price_oracle, from_token).await?;
        let profit_f64 =
            ethers::utils::format_units(profit_in_token.as_u128(), from_token_decimals as usize)?
                .parse::<f64>()?;

        let profit_usd = profit_f64 * from_token_price;

        // Estimate gas costs
        let gas_cost = self.estimate_gas_cost(path.len(), dex_used).await?;

        // Calculate net profit
        let net_profit = profit_usd - gas_cost;

        if net_profit > 0.0 {
            Ok(net_profit)
        } else {
            Ok(0.0) // No profit after gas costs
        }
    }
}
