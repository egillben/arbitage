//! Gas Price Optimizer Module
//!
//! This module is responsible for calculating optimal gas prices.

use anyhow::Result;
use async_trait::async_trait;
use ethers::middleware::Middleware;
use ethers::providers::Provider;
use ethers::types::{BlockNumber, U256};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

use crate::config::{Config, GasStrategy};

/// Interface for gas price optimizers
#[async_trait]
pub trait GasOptimizer: Send + Sync {
    /// Get the optimal gas price
    async fn get_optimal_gas_price(&self) -> Result<U256>;

    /// Get the EIP-1559 fee data (base fee, priority fee)
    async fn get_eip1559_fee_data(&self) -> Result<(U256, U256)>;

    /// Update the gas price estimate
    async fn update_gas_price_estimate(&self) -> Result<()>;
}

/// Implementation of the gas price optimizer
pub struct GasOptimizerImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    current_gas_price: RwLock<U256>,
    current_base_fee: RwLock<U256>,
    current_priority_fee: RwLock<U256>,
    last_update: RwLock<Instant>,
}

/// Create a new gas price optimizer
pub async fn create_optimizer(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
) -> Result<Arc<dyn GasOptimizer>> {
    let optimizer = GasOptimizerImpl {
        config: config.clone(),
        blockchain_client,
        current_gas_price: RwLock::new(U256::from(config.gas.max_gas_price * 1_000_000_000)), // Convert gwei to wei
        current_base_fee: RwLock::new(U256::zero()),
        current_priority_fee: RwLock::new(U256::from(config.gas.priority_fee * 1_000_000_000)), // Convert gwei to wei
        last_update: RwLock::new(Instant::now() - Duration::from_secs(3600)), // Force an update on first call
    };

    // Initialize gas price estimates
    optimizer.update_gas_price_estimate().await?;

    Ok(Arc::new(optimizer))
}

#[async_trait]
impl GasOptimizer for GasOptimizerImpl {
    async fn get_optimal_gas_price(&self) -> Result<U256> {
        // Check if we need to update the gas price estimate
        let last_update = *self.last_update.read().await;
        if last_update.elapsed() > Duration::from_secs(15) {
            self.update_gas_price_estimate().await?;
        }

        // Get the current gas price based on the strategy
        match self.config.gas.strategy {
            GasStrategy::Fixed => {
                // Use the fixed gas price from the config
                let gas_price = U256::from(self.config.gas.max_gas_price * 1_000_000_000); // Convert gwei to wei
                Ok(gas_price)
            }
            GasStrategy::Eip1559 => {
                // Use EIP-1559 fee data
                let (base_fee, priority_fee) = self.get_eip1559_fee_data().await?;

                // Calculate the max fee per gas
                let max_fee_per_gas = base_fee
                    .saturating_mul(U256::from(
                        (self.config.gas.base_fee_multiplier * 100.0) as u64,
                    ))
                    .checked_div(U256::from(100))
                    .unwrap_or_default()
                    .saturating_add(priority_fee);

                // Ensure the max fee per gas doesn't exceed the max gas price
                let max_gas_price = U256::from(self.config.gas.max_gas_price * 1_000_000_000); // Convert gwei to wei
                let max_fee_per_gas = std::cmp::min(max_fee_per_gas, max_gas_price);

                Ok(max_fee_per_gas)
            }
            GasStrategy::Dynamic => {
                // Use the current gas price estimate
                let gas_price = *self.current_gas_price.read().await;

                // Ensure the gas price doesn't exceed the max gas price
                let max_gas_price = U256::from(self.config.gas.max_gas_price * 1_000_000_000); // Convert gwei to wei
                let gas_price = std::cmp::min(gas_price, max_gas_price);

                Ok(gas_price)
            }
        }
    }

    async fn get_eip1559_fee_data(&self) -> Result<(U256, U256)> {
        // Check if we need to update the gas price estimate
        let last_update = *self.last_update.read().await;
        if last_update.elapsed() > Duration::from_secs(15) {
            self.update_gas_price_estimate().await?;
        }

        // Get the current base fee and priority fee
        let base_fee = *self.current_base_fee.read().await;
        let priority_fee = *self.current_priority_fee.read().await;

        Ok((base_fee, priority_fee))
    }

    async fn update_gas_price_estimate(&self) -> Result<()> {
        // Get the latest block
        let latest_block = self
            .blockchain_client
            .get_block(BlockNumber::Latest)
            .await?;

        if let Some(block) = latest_block {
            // Update the base fee
            if let Some(base_fee) = block.base_fee_per_gas {
                let mut current_base_fee = self.current_base_fee.write().await;
                *current_base_fee = base_fee;
                debug!(
                    "Updated base fee: {} gwei",
                    base_fee.as_u64() / 1_000_000_000
                );
            }

            // Get the fee history to estimate the priority fee
            let fee_history = self
                .blockchain_client
                .fee_history(10, BlockNumber::Latest, &[10.0, 50.0, 90.0])
                .await?;

            // In ethers 2.0, fee_history.reward is a Vec<Vec<U256>>
            let rewards = &fee_history.reward;
            if !rewards.is_empty() && !rewards[0].is_empty() && rewards[0].len() > 1 {
                // Use the 50th percentile (median) priority fee
                let priority_fee = rewards[0][1];
                let mut current_priority_fee = self.current_priority_fee.write().await;
                *current_priority_fee = priority_fee;
                debug!(
                    "Updated priority fee: {} gwei",
                    priority_fee.as_u64() / 1_000_000_000
                );
            }

            // Get the gas price estimate
            let gas_price = self.blockchain_client.get_gas_price().await?;
            let mut current_gas_price = self.current_gas_price.write().await;
            *current_gas_price = gas_price;
            debug!(
                "Updated gas price: {} gwei",
                gas_price.as_u64() / 1_000_000_000
            );

            // Update the last update timestamp
            let mut last_update = self.last_update.write().await;
            *last_update = Instant::now();
        } else {
            warn!("Failed to get latest block for gas price estimation");
        }

        Ok(())
    }
}
