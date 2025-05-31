//! MEV-Share Module
//!
//! This module is responsible for integrating with the MEV-Share network.
//! This is a custom implementation that doesn't rely on the mev-share-rs crate.

use anyhow::{Context, Result};
use ethers::types::{transaction::eip2718::TypedTransaction, Bytes, H256, U256};
use futures::stream::{StreamExt, TryStreamExt};
use log::{debug, error, info, warn};
use reqwest::{header, Client};
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;

/// MEV-Share API endpoints
const BUNDLE_STATS_ENDPOINT: &str = "/api/v1/bundle/stats";
const SEND_BUNDLE_ENDPOINT: &str = "/api/v1/bundle";
const BUNDLE_STATUS_ENDPOINT: &str = "/api/v1/bundle/status";
const SEND_TX_ENDPOINT: &str = "/api/v1/tx";
const SSE_TRANSACTIONS_ENDPOINT: &str = "/api/v1/events/transaction";

/// MEV-Share client
#[derive(Clone)]
pub struct MevShareClient {
    config: Arc<Config>,
    http_client: Client,
    api_url: String,
    api_key: Option<String>,
}

/// MEV-Share bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevShareBundle {
    /// Bundle version
    pub version: String,

    /// Bundle ID
    pub id: Option<String>,

    /// Bundle transactions
    pub transactions: Vec<String>,

    /// Block number
    pub block_number: String,

    /// Minimum timestamp
    pub min_timestamp: Option<u64>,

    /// Maximum timestamp
    pub max_timestamp: Option<u64>,

    /// Reverting transactions
    #[serde(rename = "revertingTxHashes")]
    pub reverting_tx_hashes: Option<Vec<String>>,
}

/// MEV-Share transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevShareTransaction {
    /// Transaction hash
    pub tx_hash: H256,

    /// Transaction data
    pub tx_data: Bytes,

    /// Hints
    pub hints: MevShareHints,
}

/// MEV-Share hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevShareHints {
    /// Transaction hash
    #[serde(rename = "txHash")]
    pub tx_hash: bool,

    /// Transaction calldata
    pub calldata: bool,

    /// Transaction contract address
    #[serde(rename = "contractAddress")]
    pub contract_address: bool,

    /// Transaction function selector
    #[serde(rename = "functionSelector")]
    pub function_selector: bool,

    /// Transaction logs
    pub logs: bool,
}

/// Bundle parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleParams {
    /// Block number
    pub block: Option<String>,

    /// Max block number
    #[serde(rename = "maxBlock")]
    pub max_block: Option<String>,
}

/// Bundle request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRequest {
    /// Bundle version
    pub version: String,

    /// Inclusion parameters
    pub inclusion: BundleParams,

    /// Bundle body (transactions)
    pub body: Vec<String>,

    /// Validity parameters
    pub validity: BundleParams,
}

/// Transaction with hint preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction data in hex format
    pub tx: String,

    /// Hint preferences
    pub preferences: Option<HintPreferences>,
}

/// Hint preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintPreferences {
    /// Transaction hint
    pub transaction: Option<bool>,

    /// Block hint
    pub block: Option<bool>,

    /// Calldata hint
    pub calldata: Option<bool>,

    /// Contract address hint
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<bool>,

    /// Logs hint
    pub logs: Option<bool>,

    /// Function selector hint
    #[serde(rename = "functionSelector")]
    pub function_selector: Option<bool>,

    /// Hash hint
    pub hash: Option<bool>,
}

/// Send bundle response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendBundleResponse {
    /// Bundle hash
    #[serde(rename = "bundleHash")]
    pub bundle_hash: String,
}

/// Send transaction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionResponse {
    /// Transaction hash
    #[serde(rename = "txHash")]
    pub tx_hash: String,
}

/// Bundle status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatusResponse {
    /// Bundle status
    pub status: String,
}

/// Bundle stats response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatsResponse {
    /// Total bundles
    #[serde(rename = "totalBundles")]
    pub total_bundles: u64,

    /// Total transactions
    #[serde(rename = "totalTransactions")]
    pub total_transactions: u64,
}

/// Create a new MEV-Share client
pub async fn create_client(config: &Arc<Config>) -> Result<Arc<MevShareClient>> {
    // Create the HTTP client with appropriate headers
    let mut headers = header::HeaderMap::new();

    // Add API key if available
    if let Some(api_key) = &config.mev_share.api_key {
        headers.insert(
            "X-Flashbots-Signature",
            header::HeaderValue::from_str(api_key).context("Invalid API key format")?,
        );
    }

    let http_client = Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .build()?;

    let client = MevShareClient {
        config: config.clone(),
        http_client,
        api_url: config.mev_share.api_url.clone(),
        api_key: config.mev_share.api_key.clone(),
    };

    let client = Arc::new(client);

    // Verify the connection
    client.ping().await?;

    Ok(client)
}

impl MevShareClient {
    /// Ping the MEV-Share API
    pub async fn ping(&self) -> Result<()> {
        if !self.config.mev_share.enabled {
            return Ok(());
        }

        // Make a simple request to verify the connection
        let _ = self.get_bundle_stats().await?;
        info!("Connected to MEV-Share network");

        Ok(())
    }

    /// Send a transaction via MEV-Share
    pub async fn send_transaction(&self, transaction: TypedTransaction) -> Result<H256> {
        if !self.config.mev_share.enabled {
            return Err(anyhow::anyhow!("MEV-Share is not enabled"));
        }

        // Serialize the transaction
        let tx_bytes = transaction.rlp();
        let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

        // Create a MEV-Share transaction
        let mev_tx = Transaction {
            tx: tx_hex,
            preferences: Some(HintPreferences {
                // Provide hints about the transaction
                transaction: Some(true),
                block: Some(true),
                // Don't reveal the calldata
                calldata: Some(false),
                contract_address: Some(true),
                logs: Some(true),
                function_selector: Some(true),
                hash: Some(true),
            }),
        };

        // Send the transaction
        let response = self
            .http_client
            .post(&format!("{}{}", self.api_url, SEND_TX_ENDPOINT))
            .json(&mev_tx)
            .send()
            .await?
            .error_for_status()?
            .json::<SendTransactionResponse>()
            .await?;

        // Parse the transaction hash
        let tx_hash = H256::from_slice(&hex::decode(&response.tx_hash[2..])?);

        info!("Sent transaction via MEV-Share: {}", tx_hash);

        Ok(tx_hash)
    }

    /// Send a bundle via MEV-Share
    pub async fn send_bundle(&self, bundle: MevShareBundle) -> Result<String> {
        if !self.config.mev_share.enabled {
            return Err(anyhow::anyhow!("MEV-Share is not enabled"));
        }

        // Create the bundle request
        let bundle_request = BundleRequest {
            version: bundle.version,
            inclusion: BundleParams {
                block: Some(bundle.block_number.clone()),
                max_block: None,
            },
            body: bundle.transactions,
            validity: BundleParams {
                block: None,
                max_block: None,
            },
        };

        // Send the bundle
        let response = self
            .http_client
            .post(&format!("{}{}", self.api_url, SEND_BUNDLE_ENDPOINT))
            .json(&bundle_request)
            .send()
            .await?
            .error_for_status()?
            .json::<SendBundleResponse>()
            .await?;

        info!("Sent bundle via MEV-Share: {}", response.bundle_hash);

        Ok(response.bundle_hash)
    }

    /// Get the status of a bundle
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<String> {
        if !self.config.mev_share.enabled {
            return Err(anyhow::anyhow!("MEV-Share is not enabled"));
        }

        // Get the bundle status
        let status = self
            .http_client
            .get(&format!(
                "{}{}/{}",
                self.api_url, BUNDLE_STATUS_ENDPOINT, bundle_id
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<BundleStatusResponse>()
            .await?;

        Ok(status.status)
    }

    /// Subscribe to MEV-Share events
    pub async fn subscribe(&self) -> Result<mpsc::Receiver<serde_json::Value>> {
        if !self.config.mev_share.enabled {
            return Err(anyhow::anyhow!("MEV-Share is not enabled"));
        }

        // Create a channel for events
        let (tx, rx) = mpsc::channel(100);

        // Create the event source URL
        let sse_url = format!("{}{}", self.api_url, SSE_TRANSACTIONS_ENDPOINT);

        // Clone necessary values for the async task
        let http_client = self.http_client.clone();
        let api_key = self.api_key.clone();

        // Spawn a task to listen for events
        tokio::spawn(async move {
            // Create a request with appropriate headers
            let mut request = http_client.get(&sse_url);

            // Add API key if available
            if let Some(key) = &api_key {
                request = request.header("X-Flashbots-Signature", key);
            }

            // Add Accept header for SSE
            request = request.header("Accept", "text/event-stream");

            // Send the request and get a streaming response
            match request.send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        error!(
                            "Failed to connect to MEV-Share event stream: {}",
                            response.status()
                        );
                        return;
                    }

                    // Get the response body as a byte stream
                    let mut stream = response.bytes_stream();

                    // Buffer for accumulating event data
                    let mut buffer = String::new();

                    // Process the stream
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                // Convert bytes to string and append to buffer
                                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                                    buffer.push_str(&text);

                                    // Process complete events in the buffer
                                    let mut processed = 0;
                                    while let Some(pos) = buffer[processed..].find("\n\n") {
                                        let real_pos = processed + pos;
                                        // Extract the event text
                                        let event_text =
                                            buffer[processed..real_pos].trim().to_string();

                                        // Update processed position
                                        processed = real_pos + 2;

                                        // Parse event data
                                        if event_text.starts_with("data: ") {
                                            let data = &event_text[6..];

                                            // Parse as JSON
                                            if let Ok(json) =
                                                serde_json::from_str::<serde_json::Value>(data)
                                            {
                                                // Send the event to the channel
                                                if let Err(e) = tx.send(json).await {
                                                    error!("Failed to send MEV-Share event: {}", e);
                                                    return;
                                                }
                                            } else {
                                                error!("Failed to parse MEV-Share event data as JSON: {}", data);
                                            }
                                        }
                                    }

                                    // Remove processed content from buffer if any was processed
                                    if processed > 0 {
                                        buffer = buffer[processed..].to_string();
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error receiving MEV-Share event chunk: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to MEV-Share event stream: {}", e);
                }
            }

            warn!("MEV-Share event stream ended");
        });

        info!("Subscribed to MEV-Share events");

        Ok(rx)
    }

    /// Create a MEV-Share transaction
    pub fn create_transaction(&self, tx_data: Bytes) -> MevShareTransaction {
        // Create a transaction hash from the data
        let tx_hash = ethers::utils::keccak256(tx_data.clone());

        MevShareTransaction {
            tx_hash: H256::from_slice(&tx_hash),
            tx_data,
            hints: MevShareHints {
                tx_hash: true,
                calldata: false, // Don't reveal calldata for privacy
                contract_address: true,
                function_selector: true,
                logs: true,
            },
        }
    }

    /// Create a MEV-Share bundle
    pub fn create_bundle(&self, transactions: Vec<Bytes>, block_number: u64) -> MevShareBundle {
        // Get the current block number
        let block_hex = format!("0x{:x}", block_number);

        // Convert transactions to hex strings
        let tx_hexes = transactions
            .iter()
            .map(|tx| format!("0x{}", hex::encode(tx)))
            .collect();

        MevShareBundle {
            version: "v0.1".to_string(),
            id: None,
            transactions: tx_hexes,
            block_number: block_hex,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: None,
        }
    }

    /// Get MEV-Share bundle statistics
    pub async fn get_bundle_stats(&self) -> Result<BundleStatsResponse> {
        if !self.config.mev_share.enabled {
            return Err(anyhow::anyhow!("MEV-Share is not enabled"));
        }

        // Get the bundle stats
        let stats = self
            .http_client
            .get(&format!("{}{}", self.api_url, BUNDLE_STATS_ENDPOINT))
            .send()
            .await?
            .error_for_status()?
            .json::<BundleStatsResponse>()
            .await?;

        Ok(stats)
    }
}
