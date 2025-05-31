//! Blockchain Event Listener Module
//!
//! This module is responsible for listening to blockchain events and processing them.

use anyhow::{Context, Result};
use async_trait::async_trait;
use ethers::abi::RawLog;
use ethers::contract::{Contract, Event};
use ethers::providers::{Http, Middleware, Provider, StreamExt, Ws};
use ethers::types::{Address, BlockNumber, Filter, Log, H256, U64};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use crate::blockchain::AlchemyProvider;
use crate::config::Config;
use crate::price::{PriceOracle, PriceOracleInterface};
use crate::scanner::OpportunityScanner;

/// Event handler function type
type EventHandlerFn = Box<dyn Fn(Log) -> Result<()> + Send + Sync>;

/// Interface for blockchain event listeners
#[async_trait]
pub trait BlockchainEventListener: Send + Sync {
    /// Start listening for events
    async fn start(&self) -> Result<()>;

    /// Stop listening for events
    async fn stop(&self) -> Result<()>;

    /// Register an event handler
    async fn register_event_handler(&self, event_name: &str, handler: EventHandlerFn)
        -> Result<()>;

    /// Process a new block
    async fn process_block(&self, block_number: u64) -> Result<()>;
}

/// Implementation of the blockchain event listener
pub struct BlockchainEventListenerImpl {
    config: Arc<Config>,
    blockchain_client_http: Arc<Provider<Http>>,
    blockchain_client_ws: Option<Arc<Provider<Ws>>>,
    alchemy_provider: Option<Arc<AlchemyProvider>>,
    scanner: Arc<dyn OpportunityScanner>,
    price_oracle: Arc<PriceOracle>,
    event_handlers: RwLock<HashMap<String, Vec<EventHandlerFn>>>,
    is_running: RwLock<bool>,
    task_handle: RwLock<Option<JoinHandle<()>>>,
    polling_interval: Duration,
}

/// Start a new blockchain event listener
pub async fn start_listener(
    config: &Arc<Config>,
    blockchain_client: Arc<Provider<ethers::providers::Http>>,
    scanner: Arc<dyn OpportunityScanner>,
    price_oracle: Arc<PriceOracle>,
) -> Result<Arc<dyn BlockchainEventListener>> {
    // Check if WebSocket connections are enabled in the config
    let use_websocket = config.ethereum.use_websocket.unwrap_or(true);

    // Create a websocket client for event listening if enabled
    let ws_client = if use_websocket {
        match crate::blockchain::create_ws_client(config).await {
            Ok(client) => {
                info!("WebSocket connection established successfully");
                Some(client)
            }
            Err(e) => {
                warn!(
                    "Failed to create WebSocket client: {}. Falling back to HTTP polling",
                    e
                );
                None
            }
        }
    } else {
        info!("WebSocket connections disabled in configuration. Using HTTP polling instead");
        None
    };

    // Create an Alchemy provider if possible
    let alchemy_provider = if config.ethereum.rpc_url.contains("alchemyapi.io") {
        match crate::blockchain::create_alchemy_provider(config).await {
            Ok(provider) => {
                info!("Using Alchemy provider for enhanced blockchain event listening");
                Some(provider)
            }
            Err(e) => {
                warn!("Failed to create Alchemy provider: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Set polling interval for HTTP fallback (default to 2 seconds)
    let polling_interval =
        Duration::from_millis(config.ethereum.polling_interval_ms.unwrap_or(2000));

    let listener = BlockchainEventListenerImpl {
        config: config.clone(),
        blockchain_client_http: blockchain_client,
        blockchain_client_ws: ws_client,
        alchemy_provider,
        scanner,
        price_oracle,
        event_handlers: RwLock::new(HashMap::new()),
        is_running: RwLock::new(false),
        task_handle: RwLock::new(None),
        polling_interval,
    };

    let listener = Arc::new(listener);

    // Start listening for events
    listener.start().await?;

    Ok(listener)
}

#[async_trait]
impl BlockchainEventListener for BlockchainEventListenerImpl {
    async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        *is_running = true;

        // Create a channel for new block notifications
        let (tx, mut rx) = mpsc::channel(100);

        // Clone the Arc for the task
        let self_clone = Arc::new(self.clone());

        // Start a task to listen for new blocks
        let task_handle = tokio::spawn(async move {
            // Use Alchemy's enhanced WebSocket API if available
            if let Some(alchemy_provider) = &self_clone.alchemy_provider {
                if let Some(ws_provider) = alchemy_provider.ws() {
                    info!("Using Alchemy WebSocket provider for block subscription");
                    let mut stream = ws_provider.subscribe_blocks().await.unwrap();

                    while let Some(block) = stream.next().await {
                        let block_number = block.number.unwrap_or_default().as_u64();
                        debug!("New block from Alchemy: {}", block_number);

                        // Send the block number to the processing task
                        if let Err(e) = tx.send(block_number).await {
                            error!("Failed to send block number to processing task: {}", e);
                            break;
                        }
                    }

                    warn!("Alchemy block subscription stream ended");
                    return;
                }
            }

            // Check if we have a WebSocket client
            if let Some(ws_client) = &self_clone.blockchain_client_ws {
                info!("Using WebSocket provider for block subscription");
                match ws_client.subscribe_blocks().await {
                    Ok(mut stream) => {
                        while let Some(block) = stream.next().await {
                            let block_number = block.number.unwrap_or_default().as_u64();
                            debug!("New block: {}", block_number);

                            // Send the block number to the processing task
                            if let Err(e) = tx.send(block_number).await {
                                error!("Failed to send block number to processing task: {}", e);
                                break;
                            }
                        }
                        warn!("WebSocket block subscription stream ended");
                    }
                    Err(e) => {
                        error!("Failed to subscribe to blocks via WebSocket: {}", e);
                    }
                }
            } else {
                // Fallback to HTTP polling
                info!(
                    "Using HTTP polling for block updates (interval: {} ms)",
                    self_clone.polling_interval.as_millis()
                );

                let mut last_block_number = 0u64;

                loop {
                    match self_clone.blockchain_client_http.get_block_number().await {
                        Ok(block_number) => {
                            let block_number = block_number.as_u64();

                            // Only process if it's a new block
                            if block_number > last_block_number {
                                debug!("New block from HTTP polling: {}", block_number);
                                last_block_number = block_number;

                                // Send the block number to the processing task
                                if let Err(e) = tx.send(block_number).await {
                                    error!("Failed to send block number to processing task: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to get block number via HTTP: {}", e);
                        }
                    }

                    // Sleep for the polling interval
                    tokio::time::sleep(self_clone.polling_interval).await;
                }
            }
        });

        // Clone the Arc for the processing task
        let self_clone = Arc::new(self.clone());

        // Start a task to process new blocks
        let processing_handle = tokio::spawn(async move {
            while let Some(block_number) = rx.recv().await {
                if let Err(e) = self_clone.process_block(block_number).await {
                    error!("Failed to process block {}: {}", block_number, e);
                }
            }

            warn!("Block processing task ended");
        });

        // Store the task handle
        let mut task_handle_lock = self.task_handle.write().await;
        *task_handle_lock = Some(task_handle);

        info!("Blockchain event listener started");

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        *is_running = false;

        // Abort the task
        let mut task_handle = self.task_handle.write().await;
        if let Some(handle) = task_handle.take() {
            handle.abort();
        }

        info!("Blockchain event listener stopped");

        Ok(())
    }

    async fn register_event_handler(
        &self,
        event_name: &str,
        handler: EventHandlerFn,
    ) -> Result<()> {
        let mut event_handlers = self.event_handlers.write().await;

        // Get or create the handler list for this event
        let handlers = event_handlers
            .entry(event_name.to_string())
            .or_insert_with(Vec::new);

        // Add the handler
        handlers.push(handler);

        Ok(())
    }

    async fn process_block(&self, block_number: u64) -> Result<()> {
        // Get the block details
        let block = if let Some(alchemy_provider) = &self.alchemy_provider {
            alchemy_provider
                .http()
                .get_block_with_txs(block_number)
                .await?
        } else {
            self.blockchain_client_http
                .get_block_with_txs(block_number)
                .await?
        };

        if let Some(block) = block {
            debug!(
                "Processing block {} with {} transactions",
                block_number,
                block.transactions.len()
            );

            // Process any relevant events
            // In a real implementation, we would process events from the block

            // Update the price oracle
            self.price_oracle.update_prices().await?;

            // Scan for arbitrage opportunities
            let opportunities = self.scanner.scan().await?;

            if !opportunities.is_empty() {
                info!(
                    "Found {} potential arbitrage opportunities in block {}",
                    opportunities.len(),
                    block_number
                );
            }
        } else {
            warn!("Block {} not found", block_number);
        }

        Ok(())
    }
}

impl Clone for BlockchainEventListenerImpl {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            blockchain_client_http: self.blockchain_client_http.clone(),
            blockchain_client_ws: self.blockchain_client_ws.clone(),
            alchemy_provider: self.alchemy_provider.clone(),
            scanner: self.scanner.clone(),
            price_oracle: self.price_oracle.clone(),
            event_handlers: RwLock::new(HashMap::new()),
            is_running: RwLock::new(false),
            task_handle: RwLock::new(None),
            polling_interval: self.polling_interval,
        }
    }
}
