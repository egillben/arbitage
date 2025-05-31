//! Configuration module for the MEV arbitrage bot
//!
//! This module handles loading and parsing configuration from files and environment variables.

use anyhow::{Context, Result};
use dotenv::dotenv;
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Main configuration structure for the MEV arbitrage bot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Ethereum network configuration
    pub ethereum: EthereumConfig,

    /// MEV-Share configuration
    pub mev_share: MevShareConfig,

    /// Flash loan configuration
    pub flash_loan: FlashLoanConfig,

    /// DEX configuration
    pub dex: DexConfig,

    /// Arbitrage configuration
    pub arbitrage: ArbitrageConfig,

    /// Gas price configuration
    pub gas: GasConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Test mode configuration
    /// When enabled, reduces log verbosity and slows down scanning frequency
    #[serde(default)]
    pub test_mode: bool,
}

/// Ethereum network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    /// RPC URL for the Ethereum node (e.g., Alchemy)
    pub rpc_url: String,

    /// Websocket URL for the Ethereum node (e.g., Alchemy)
    pub ws_url: Option<String>,

    /// Whether to use WebSocket connections (defaults to true)
    pub use_websocket: Option<bool>,

    /// Polling interval in milliseconds for HTTP fallback (defaults to 2000)
    pub polling_interval_ms: Option<u64>,

    /// Chain ID of the Ethereum network
    pub chain_id: u64,

    /// Private key for the bot's wallet (encrypted in storage, decrypted at runtime)
    #[serde(skip_serializing)]
    pub private_key: Option<String>,

    /// Public address of the bot's wallet
    pub wallet_address: String,

    /// Maximum number of blocks to look back for events
    pub max_block_lookback: u64,

    /// Websocket connection timeout in seconds
    pub ws_timeout_seconds: u64,

    /// Alchemy API key
    #[serde(skip_serializing)]
    pub alchemy_api_key: Option<String>,
}

/// MEV-Share configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevShareConfig {
    /// MEV-Share API URL
    pub api_url: String,

    /// MEV-Share API key
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// Whether to use MEV-Share for transaction protection
    pub enabled: bool,

    /// Maximum tip to pay to validators (in gwei)
    pub max_validator_tip: u64,
}

/// Flash loan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanConfig {
    /// Address of the Aave lending pool
    pub aave_lending_pool: String,

    /// Maximum amount to borrow (in ETH)
    pub max_borrow_amount: f64,

    /// List of tokens to consider for flash loans
    pub tokens: Vec<TokenConfig>,
}

/// Token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Token symbol (e.g., "WETH", "USDC")
    pub symbol: String,

    /// Token address
    pub address: String,

    /// Token decimals
    pub decimals: u8,
}

/// DEX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    /// Uniswap configuration
    pub uniswap: DexInstanceConfig,

    /// Sushiswap configuration
    pub sushiswap: DexInstanceConfig,

    /// Curve configuration
    pub curve: DexInstanceConfig,
}

/// Configuration for a specific DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexInstanceConfig {
    /// Whether this DEX is enabled
    pub enabled: bool,

    /// Factory address
    pub factory_address: String,

    /// Router address
    pub router_address: String,

    /// List of pool addresses to monitor
    pub pools: Vec<String>,
}

/// Arbitrage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Minimum profit threshold (in USD)
    pub min_profit_threshold: f64,

    /// Maximum number of hops in a trade path
    pub max_hops: u8,

    /// Slippage tolerance percentage
    pub slippage_tolerance: f64,

    /// Timeout for opportunity evaluation (in milliseconds)
    pub evaluation_timeout_ms: u64,

    /// Maximum number of concurrent evaluations
    pub max_concurrent_evaluations: u8,

    /// Smart contract configuration
    pub contract: ContractConfig,
}

/// Smart contract configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// Address of the deployed ArbitrageExecutor contract
    pub contract_address: Option<String>,

    /// Whether to deploy a new contract if one is not provided
    pub deploy_if_missing: bool,

    /// Gas limit for contract deployment
    pub deployment_gas_limit: u64,
}

/// Gas price configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConfig {
    /// Strategy for gas price calculation
    pub strategy: GasStrategy,

    /// Maximum gas price willing to pay (in gwei)
    pub max_gas_price: u64,

    /// Base fee multiplier for EIP-1559 transactions
    pub base_fee_multiplier: f64,

    /// Priority fee for EIP-1559 transactions (in gwei)
    pub priority_fee: u64,

    /// Gas limit for arbitrage transactions
    pub gas_limit: u64,
}

/// Gas price calculation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GasStrategy {
    /// Fixed gas price
    #[serde(rename = "fixed")]
    Fixed,

    /// EIP-1559 transaction type
    #[serde(rename = "eip1559")]
    Eip1559,

    /// Dynamic gas price based on network conditions
    #[serde(rename = "dynamic")]
    Dynamic,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Transaction timeout (in seconds)
    pub transaction_timeout: u64,

    /// Number of price sources required for validation
    pub min_price_sources: u8,

    /// Maximum price deviation percentage
    pub max_price_deviation: f64,

    /// Whether to simulate transactions before sending
    pub simulate_transactions: bool,

    /// Maximum slippage allowed during execution (percentage)
    pub max_execution_slippage: f64,
}

/// Load configuration from file and environment variables
pub fn load_config() -> Result<Arc<Config>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Load configuration from file
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let mut config: Config = config::Config::builder()
        .add_source(config::File::with_name(&config_path).required(false))
        .add_source(config::Environment::with_prefix("MEV_BOT"))
        .build()
        .context("Failed to build configuration")?
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    // Load sensitive information from environment variables
    config.ethereum.private_key = std::env::var("ETHEREUM_PRIVATE_KEY").ok();
    config.ethereum.alchemy_api_key = std::env::var("ALCHEMY_API_KEY").ok();
    config.mev_share.api_key = std::env::var("MEV_SHARE_API_KEY").ok();

    // Set the websocket URL based on the RPC URL and Alchemy API key if not provided
    if config.ethereum.ws_url.is_none() {
        if let Some(api_key) = &config.ethereum.alchemy_api_key {
            if config.ethereum.rpc_url.contains("alchemyapi.io") {
                config.ethereum.ws_url =
                    Some(format!("wss://eth-mainnet.ws.alchemyapi.io/v2/{}", api_key));
            }
        } else {
            // Default to converting http to ws
            config.ethereum.ws_url = Some(config.ethereum.rpc_url.replace("http", "ws"));
        }
    }

    // Validate configuration
    validate_config(&config)?;

    Ok(Arc::new(config))
}

/// Validate the configuration
fn validate_config(config: &Config) -> Result<()> {
    // Validate Ethereum configuration
    if config.ethereum.rpc_url.is_empty() {
        anyhow::bail!("Ethereum RPC URL is required");
    }

    if config.ethereum.chain_id == 0 {
        anyhow::bail!("Ethereum chain ID is required");
    }

    // Validate wallet configuration
    if config.ethereum.private_key.is_none() && config.ethereum.wallet_address.is_empty() {
        anyhow::bail!("Either private key or wallet address is required");
    }

    // Validate MEV-Share configuration
    if config.mev_share.enabled && config.mev_share.api_url.is_empty() {
        anyhow::bail!("MEV-Share API URL is required when MEV-Share is enabled");
    }

    // Validate arbitrage configuration
    if config.arbitrage.min_profit_threshold <= 0.0 {
        anyhow::bail!("Minimum profit threshold must be greater than zero");
    }

    if config.arbitrage.max_hops == 0 {
        anyhow::bail!("Maximum hops must be greater than zero");
    }

    // Validate gas configuration
    if config.gas.max_gas_price == 0 {
        anyhow::bail!("Maximum gas price must be greater than zero");
    }

    if config.gas.gas_limit == 0 {
        anyhow::bail!("Gas limit must be greater than zero");
    }

    Ok(())
}

/// Create a default configuration
pub fn create_default_config() -> Config {
    Config {
        ethereum: EthereumConfig {
            rpc_url: "https://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string(),
            ws_url: Some("wss://eth-mainnet.ws.alchemyapi.io/v2/your-api-key".to_string()),
            use_websocket: Some(true),
            polling_interval_ms: Some(2000),
            chain_id: 1, // Mainnet
            private_key: None,
            wallet_address: "".to_string(),
            max_block_lookback: 10,
            ws_timeout_seconds: 30,
            alchemy_api_key: None,
        },
        test_mode: false,
        mev_share: MevShareConfig {
            api_url: "https://mev-share.flashbots.net".to_string(),
            api_key: None,
            enabled: true,
            max_validator_tip: 2, // 2 gwei
        },
        flash_loan: FlashLoanConfig {
            aave_lending_pool: "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9".to_string(), // Aave V2 lending pool
            max_borrow_amount: 100.0,                                                    // 100 ETH
            tokens: vec![
                TokenConfig {
                    symbol: "WETH".to_string(),
                    address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                    decimals: 18,
                },
                TokenConfig {
                    symbol: "USDC".to_string(),
                    address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                    decimals: 6,
                },
                TokenConfig {
                    symbol: "DAI".to_string(),
                    address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
                    decimals: 18,
                },
            ],
        },
        dex: DexConfig {
            uniswap: DexInstanceConfig {
                enabled: true,
                factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(), // Uniswap V2 factory
                router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(), // Uniswap V2 router
                pools: vec![],
            },
            sushiswap: DexInstanceConfig {
                enabled: true,
                factory_address: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string(), // Sushiswap factory
                router_address: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string(), // Sushiswap router
                pools: vec![],
            },
            curve: DexInstanceConfig {
                enabled: true,
                factory_address: "0x0959158b6040D32d04c301A72CBFD6b39E21c9AE".to_string(), // Curve factory
                router_address: "0x8e764bE4288B842791989DB5b8ec067279829809".to_string(), // Curve router
                pools: vec![],
            },
        },
        arbitrage: ArbitrageConfig {
            min_profit_threshold: 50.0, // $50
            max_hops: 3,
            slippage_tolerance: 0.5, // 0.5%
            evaluation_timeout_ms: 500,
            max_concurrent_evaluations: 5,
            contract: ContractConfig {
                contract_address: None,
                deploy_if_missing: true,
                deployment_gas_limit: 5000000,
            },
        },
        gas: GasConfig {
            strategy: GasStrategy::Eip1559,
            max_gas_price: 100, // 100 gwei
            base_fee_multiplier: 1.2,
            priority_fee: 2, // 2 gwei
            gas_limit: 500000,
        },
        security: SecurityConfig {
            transaction_timeout: 60, // 60 seconds
            min_price_sources: 2,
            max_price_deviation: 1.0, // 1%
            simulate_transactions: true,
            max_execution_slippage: 1.0, // 1%
        },
    }
}
