//! Transaction Executor Module
//!
//! This module is responsible for executing transactions on the Ethereum network.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::middleware::{Middleware, SignerMiddleware};
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{transaction::eip2718::TypedTransaction, Address, H256, U256};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::gas::GasOptimizer;
use crate::mev_share::MevShareClient;
use crate::transaction::{validate_transaction, ArbitrageTransaction, TransactionResult};

/// Interface for transaction executors
#[async_trait]
pub trait TransactionExecutor: Send + Sync {
    /// Execute a transaction
    async fn execute_transaction(&self, tx: ArbitrageTransaction) -> Result<H256>;

    /// Get the status of a transaction
    async fn get_transaction_status(&self, tx_hash: H256) -> Result<TransactionResult>;

    /// Wait for a transaction to be confirmed
    async fn wait_for_transaction(
        &self,
        tx_hash: H256,
        timeout: Duration,
    ) -> Result<TransactionResult>;

    /// Cancel a pending transaction
    async fn cancel_transaction(&self, tx_hash: H256) -> Result<H256>;
}

/// Implementation of the transaction executor
pub struct TransactionExecutorImpl {
    config: Arc<Config>,
    blockchain_client: Arc<Provider<Http>>,
    mev_share_client: Arc<MevShareClient>,
    gas_optimizer: Arc<dyn GasOptimizer>,
    wallet: Option<LocalWallet>,
}

/// Create a new transaction executor
pub async fn create_executor(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<Http>>,
    mev_share_client: Arc<MevShareClient>,
    gas_optimizer: Arc<dyn GasOptimizer>,
) -> Result<Arc<dyn TransactionExecutor>> {
    // Initialize the wallet if a private key is provided
    let wallet = if let Some(private_key) = &config.ethereum.private_key {
        Some(private_key.parse::<LocalWallet>()?)
    } else {
        None
    };

    let executor = TransactionExecutorImpl {
        config: config.clone(),
        blockchain_client,
        mev_share_client,
        gas_optimizer,
        wallet,
    };

    Ok(Arc::new(executor))
}

#[async_trait]
impl TransactionExecutor for TransactionExecutorImpl {
    async fn execute_transaction(&self, tx: ArbitrageTransaction) -> Result<H256> {
        // Validate the transaction
        validate_transaction(&tx).await?;

        // Check if we have a wallet
        let wallet = self
            .wallet
            .as_ref()
            .context("No wallet available for signing transactions")?;

        // Optimize gas price
        let gas_price = self.gas_optimizer.get_optimal_gas_price().await?;

        // Create a typed transaction
        let mut typed_tx: TypedTransaction = tx.request.clone().into();
        typed_tx.set_gas_price(gas_price);

        // Sign the transaction
        let client_with_signer =
            SignerMiddleware::new(self.blockchain_client.clone(), wallet.clone());

        let tx_hash = if tx.use_mev_share {
            // Send the transaction via MEV-Share
            debug!("Sending transaction via MEV-Share");
            self.mev_share_client.send_transaction(typed_tx).await?
        } else {
            // Send the transaction directly
            debug!("Sending transaction directly");
            let pending_tx = client_with_signer.send_transaction(typed_tx, None).await?;
            pending_tx.tx_hash()
        };

        info!("Transaction sent: {}", tx_hash);

        Ok(tx_hash)
    }

    async fn get_transaction_status(&self, tx_hash: H256) -> Result<TransactionResult> {
        // Get the transaction receipt
        let receipt = self
            .blockchain_client
            .get_transaction_receipt(tx_hash)
            .await?;

        // Get the transaction
        let tx = self.blockchain_client.get_transaction(tx_hash).await?;

        // Create the transaction result
        let result = match receipt {
            Some(receipt) => {
                let success = receipt.status.unwrap_or_default().as_u64() == 1;
                let gas_used = receipt.gas_used;
                let gas_price = tx.and_then(|tx| tx.gas_price);
                let actual_cost =
                    gas_used.and_then(|gas| gas_price.map(|price| gas.saturating_mul(price)));

                TransactionResult {
                    tx_hash,
                    block_number: receipt.block_number.map(|bn| bn.as_u64()),
                    gas_used,
                    actual_cost,
                    success,
                    error: if !success {
                        Some("Transaction reverted".to_string())
                    } else {
                        None
                    },
                }
            }
            None => {
                // Transaction is still pending
                TransactionResult {
                    tx_hash,
                    block_number: None,
                    gas_used: None,
                    actual_cost: None,
                    success: false,
                    error: Some("Transaction pending".to_string()),
                }
            }
        };

        Ok(result)
    }

    async fn wait_for_transaction(
        &self,
        tx_hash: H256,
        timeout: Duration,
    ) -> Result<TransactionResult> {
        let start_time = Instant::now();

        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("Transaction timed out after {:?}", timeout));
            }

            // Get the transaction status
            let status = self.get_transaction_status(tx_hash).await?;

            // If the transaction is confirmed, return the status
            if status.block_number.is_some() {
                return Ok(status);
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn cancel_transaction(&self, tx_hash: H256) -> Result<H256> {
        // Get the transaction
        let tx = self
            .blockchain_client
            .get_transaction(tx_hash)
            .await?
            .context("Transaction not found")?;

        // Check if we have a wallet
        let wallet = self
            .wallet
            .as_ref()
            .context("No wallet available for signing transactions")?;

        // Create a cancellation transaction (same nonce, higher gas price, zero value, to self)
        let from_address = tx.from;
        let nonce = tx.nonce;
        let gas_price = tx
            .gas_price
            .unwrap_or_default()
            .saturating_mul(U256::from(120))
            .checked_div(U256::from(100))
            .unwrap_or_default(); // 20% higher

        // Create a legacy transaction
        let mut cancel_tx = TypedTransaction::Legacy(Default::default());
        cancel_tx.set_nonce(nonce);
        cancel_tx.set_gas_price(gas_price);
        cancel_tx.set_gas(U256::from(21000)); // Minimum gas for a simple transfer
        cancel_tx.set_to(ethers::types::NameOrAddress::Address(from_address));
        cancel_tx.set_value(U256::zero());
        cancel_tx.set_data(Default::default());
        cancel_tx.set_chain_id(self.config.ethereum.chain_id);

        // Sign and send the cancellation transaction
        let client_with_signer =
            SignerMiddleware::new(self.blockchain_client.clone(), wallet.clone());
        let pending_tx = client_with_signer.send_transaction(cancel_tx, None).await?;
        let cancel_tx_hash = pending_tx.tx_hash();

        info!("Cancellation transaction sent: {:?}", cancel_tx_hash);

        Ok(cancel_tx_hash)
    }
}
