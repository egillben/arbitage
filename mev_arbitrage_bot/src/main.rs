//! MEV Arbitrage Bot
//!
//! This bot identifies and executes arbitrage opportunities on Ethereum using flash loans
//! and MEV-Share for protection against front-running.

mod blockchain;
mod config;
mod contract;
mod dex;
mod flash_loan;
mod gas;
mod mev_share;
mod price;
mod scanner;
mod strategy;
mod transaction;
mod utils;

use anyhow::Result;
use log::{error, info};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load configuration
    let config = config::load_config()?;
    info!("Configuration loaded successfully");

    // Initialize blockchain connection
    let blockchain_client = blockchain::create_client(&config).await?;
    info!("Connected to blockchain provider");

    // Initialize MEV-Share client
    let mev_share_client = mev_share::create_client(&config).await?;
    info!("Connected to MEV-Share network");

    // Initialize contract manager
    let contract_manager = contract::create_manager(&config, blockchain_client.clone()).await?;
    info!("Contract manager initialized");

    // Initialize price oracle
    let price_oracle = price::create_oracle(&config, blockchain_client.clone()).await?;
    info!("Price oracle initialized");

    // Initialize DEX interfaces
    let dex_interfaces = dex::create_interfaces(&config, blockchain_client.clone()).await?;
    info!("DEX interfaces initialized");

    // Initialize flash loan manager
    let flash_loan_manager = flash_loan::create_manager(&config, blockchain_client.clone()).await?;
    info!("Flash loan manager initialized");

    // Initialize gas price optimizer
    let gas_optimizer = gas::create_optimizer(&config, blockchain_client.clone()).await?;
    info!("Gas price optimizer initialized");

    // Initialize transaction builder and executor
    let tx_builder = transaction::create_builder(
        &config,
        blockchain_client.clone(),
        Some(contract_manager.clone()),
    )
    .await?;
    let tx_executor = transaction::create_executor(
        &config,
        blockchain_client.clone(),
        mev_share_client.clone(),
        gas_optimizer.clone(),
    )
    .await?;
    info!("Transaction components initialized");

    // Initialize opportunity scanner
    let scanner = scanner::create_scanner(
        &config,
        blockchain_client.clone(),
        dex_interfaces.clone(),
        price_oracle.clone(),
    )
    .await?;
    info!("Opportunity scanner initialized");

    // Initialize arbitrage strategy engine
    let strategy_engine = strategy::create_engine(
        &config,
        price_oracle.clone(),
        dex_interfaces.clone(),
        flash_loan_manager.clone(),
    )
    .await?;
    info!("Strategy engine initialized");

    // Start the blockchain event listener
    let event_listener = blockchain::start_listener(
        &config,
        blockchain_client.clone(),
        scanner.clone(),
        price_oracle.clone(),
    )
    .await?;
    info!("Blockchain event listener started");

    // Start the main arbitrage loop
    info!("Starting main arbitrage loop");
    let arbitrage_loop = tokio::spawn(async move {
        loop {
            // Scan for opportunities
            match scanner.scan().await {
                Ok(opportunities) => {
                    if !opportunities.is_empty() {
                        info!(
                            "Found {} potential arbitrage opportunities",
                            opportunities.len()
                        );

                        // Evaluate opportunities and find the best one
                        if let Some(best_opportunity) =
                            strategy_engine.evaluate_opportunities(opportunities).await
                        {
                            info!(
                                "Selected best arbitrage opportunity with estimated profit: {}",
                                best_opportunity.estimated_profit
                            );

                            // Build the transaction
                            match tx_builder
                                .build_arbitrage_transaction(&best_opportunity)
                                .await
                            {
                                Ok(transaction) => {
                                    // Execute the transaction
                                    match tx_executor.execute_transaction(transaction).await {
                                        Ok(tx_hash) => {
                                            info!(
                                                "Arbitrage transaction executed successfully: {}",
                                                tx_hash
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to execute arbitrage transaction: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to build arbitrage transaction: {}", e);
                                }
                            }
                        } else {
                            info!("No profitable arbitrage opportunities found after evaluation");
                        }
                    }
                }
                Err(e) => {
                    error!("Error scanning for arbitrage opportunities: {}", e);
                }
            }

            // Small delay to prevent excessive CPU usage
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Wait for Ctrl+C signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, stopping bot...");
            arbitrage_loop.abort();
            event_listener.stop().await?;
            info!("Bot stopped successfully");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {}", e);
        }
    }

    Ok(())
}
