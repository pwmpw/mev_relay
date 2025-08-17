use crate::{
    events::{domain::SwapEvent, filter::PoolFilter},
    infrastructure::config::Config,
    monitoring::domain::{MonitoringService, ServiceStatus, TransactionInfo as MonitoringTransactionInfo},
    shared::{types::H160, utils::time},
};
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Mempool monitoring service
pub struct MempoolMonitor {
    config: Config,
    event_sender: mpsc::Sender<SwapEvent>,
    http_client: Client,
    status: ServiceStatus,
    is_running: Arc<tokio::sync::RwLock<bool>>,
    pool_filter: PoolFilter,
}

impl MempoolMonitor {
    pub fn new(config: Config, event_sender: mpsc::Sender<SwapEvent>) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(config.mempool.request_timeout))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let status = ServiceStatus::new("Mempool Monitor".to_string());

        let pool_filter = PoolFilter::new(config.filtering.clone());
        Ok(Self {
            config,
            event_sender,
            http_client,
            status,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
            pool_filter,
        })
    }

    /// Start monitoring the mempool
    async fn start_monitoring(&self) -> Result<()> {
        let mut interval = interval(Duration::from_millis(self.config.mempool.poll_interval));
        let is_running = self.is_running.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        let http_client = self.http_client.clone();

        info!("Starting mempool monitoring on {}", config.mempool.rpc_url);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !*is_running.read().await {
                        break;
                    }

                    match self.poll_mempool(&http_client, &config, &event_sender).await {
                        Ok(_) => {
                            // Status will be updated in the start/stop methods
                        }
                        Err(e) => {
                            error!("Mempool polling error: {}", e);
                            // Status will be updated in the start/stop methods
                        }
                    }
                }
            }
        }

        info!("Mempool monitoring stopped");
        Ok(())
    }

    /// Poll the mempool for new transactions
    async fn poll_mempool(
        &self,
        client: &Client,
        config: &Config,
        event_sender: &mpsc::Sender<SwapEvent>,
    ) -> Result<()> {
        let pending_txs = self.get_pending_transactions(client, config).await?;
        
        if !pending_txs.is_empty() {
            info!("Found {} pending transactions in mempool", pending_txs.len());
            
            // Apply pool filtering
            let filtered_txs = self.pool_filter.filter_events(&pending_txs);
            
            if filtered_txs.len() != pending_txs.len() {
                info!(
                    "Pool filter: {} transactions filtered out, {} transactions kept",
                    pending_txs.len() - filtered_txs.len(),
                    filtered_txs.len()
                );
            }
            
            for tx in filtered_txs {
                if let Err(e) = event_sender.send(tx).await {
                    error!("Failed to send mempool transaction: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get pending transactions from the mempool
    async fn get_pending_transactions(
        &self,
        client: &Client,
        config: &Config,
    ) -> Result<Vec<SwapEvent>> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "txpool_content",
            "params": [],
            "id": 1
        });

        let response = client
            .post(&config.mempool.rpc_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "RPC request failed with status: {}",
                response.status()
            ));
        }

        let response_data: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON response: {}", e))?;

        self.parse_txpool_response(response_data)
    }

    /// Parse the txpool response and extract transactions
    fn parse_txpool_response(&self, response: Value) -> Result<Vec<SwapEvent>> {
        let mut events = Vec::new();

        if let Some(result) = response.get("result") {
            if let Some(pending) = result.get("pending") {
                if let Some(pending_obj) = pending.as_object() {
                    for (_address, txs) in pending_obj {
                        if let Some(tx_array) = txs.as_array() {
                            for tx in tx_array {
                                if let Some(event) = self.parse_transaction(tx)? {
                                    events.push(event);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    /// Parse a single transaction and convert to SwapEvent
    fn parse_transaction(&self, tx: &Value) -> Result<Option<SwapEvent>> {
        // This is a simplified parser - in production you'd want more sophisticated parsing
        // to detect actual swap transactions
        
        if let (Some(hash), Some(from), Some(to), Some(value), Some(gas_price)) = (
            tx.get("hash").and_then(|h| h.as_str()),
            tx.get("from").and_then(|f| f.as_str()),
            tx.get("to").and_then(|t| t.as_str()),
            tx.get("value").and_then(|v| v.as_str()),
            tx.get("gasPrice").and_then(|g| g.as_str()),
        ) {
            // Basic validation
            if hash.len() != 66 || from.len() != 42 || to.len() != 42 {
                return Ok(None);
            }

            // For now, return None - in production you'd implement actual swap detection
            // This is just a placeholder for the domain-driven structure
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Get the latest block information
    async fn get_latest_block(&self) -> Result<u64> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response = self.http_client
            .post(&self.config.mempool.rpc_url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get latest block"));
        }

        let response_data: Value = response.json().await?;
        
        if let Some(result) = response_data.get("result") {
            if let Some(block_hex) = result.as_str() {
                let block_number = u64::from_str_radix(
                    block_hex.strip_prefix("0x").unwrap_or(block_hex),
                    16,
                )?;
                return Ok(block_number);
            }
        }

        Err(anyhow::anyhow!("Invalid block number response"))
    }
}

#[async_trait]
impl MonitoringService for MempoolMonitor {
    async fn start(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if *running_guard {
            warn!("Mempool monitor is already running");
            return Ok(());
        }

        *running_guard = true;
        drop(running_guard);

        self.start_monitoring().await
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if !*running_guard {
            warn!("Mempool monitor is not running");
            return Ok(());
        }

        *running_guard = false;
        self.status.mark_inactive();
        info!("Mempool monitor stopped");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.status.is_active
    }

    fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }
}

impl Clone for MempoolMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            event_sender: self.event_sender.clone(),
            http_client: self.http_client.clone(),
            status: self.status.clone(),
            is_running: self.is_running.clone(),
            pool_filter: self.pool_filter.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::events::domain::{EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo, EventMetadata};
    use crate::shared::types::H256;

    fn create_test_swap_event(
        pool_address: Option<H160>,
        token_in: H160,
        token_out: H160,
        protocol_name: &str,
        from: H160,
        to: Option<H160>,
    ) -> SwapEvent {
        SwapEvent {
            id: EventId::new(),
            transaction: TransactionInfo {
                hash: H256([1; 32]),
                from,
                to,
                value: 0,
                gas_price: 20_000_000_000,
                gas_limit: 100000,
                gas_used: 50000,
                nonce: 1,
            },
            swap_details: SwapDetails {
                token_in,
                token_out,
                amount_in: 1_000_000_000_000_000_000, // 1 ETH
                amount_out: 950_000_000_000_000_000,
                pool_address,
                fee_tier: Some(3000),
            },
            block_info: BlockInfo {
                number: 12345,
                hash: H256([2; 32]),
                timestamp: 1234567890,
            },
            source: EventSource::Mempool,
            protocol: ProtocolInfo {
                name: protocol_name.to_string(),
                version: "2.0".to_string(),
                address: H160([3; 20]),
            },
            metadata: EventMetadata::new(),
        }
    }

    #[tokio::test]
    async fn test_mempool_monitor_creation() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = MempoolMonitor::new(config, sender);
        assert!(monitor.is_ok());
    }

    #[test]
    fn test_monitor_status() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = MempoolMonitor::new(config, sender).unwrap();
        assert!(!monitor.is_active());
    }

    #[test]
    fn test_monitor_clone() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = MempoolMonitor::new(config, sender).unwrap();
        let cloned = monitor.clone();
        
        // Test that clone works
        assert_eq!(monitor.pool_filter.get_filter_stats().total_pools, 
                   cloned.pool_filter.get_filter_stats().total_pools);
    }

    #[test]
    fn test_pool_filter_integration() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = MempoolMonitor::new(config, sender).unwrap();
        
        // Test that pool filter is properly initialized
        let filter_stats = monitor.pool_filter.get_filter_stats();
        assert!(filter_stats.total_pools > 0);
        assert!(filter_stats.total_tokens > 0);
        assert!(filter_stats.included_protocols.contains(&"Uniswap V2".to_string()));
    }

    #[test]
    fn test_filtering_config_integration() {
        let mut config = Config::default();
        
        // Test that filtering config is properly set
        assert!(config.filtering.enabled);
        assert!(!config.filtering.pool_addresses.is_empty());
        assert!(!config.filtering.token_addresses.is_empty());
        assert_eq!(config.filtering.min_liquidity_eth, 100.0);
        assert_eq!(config.filtering.min_volume_24h_eth, 1000.0);
        
        // Test custom filtering configuration
        config.filtering.pool_addresses = vec![
            "0x1234567890123456789012345678901234567890".to_string(),
        ];
        config.filtering.min_liquidity_eth = 500.0;
        
        assert_eq!(config.filtering.pool_addresses.len(), 1);
        assert_eq!(config.filtering.min_liquidity_eth, 500.0);
    }

    #[test]
    fn test_mempool_config_poll_interval() {
        let config = Config::default();
        
        // Test that poll_interval is properly set
        assert_eq!(config.mempool.poll_interval, 100);
        
        // Test that flashbots poll_interval is properly set
        assert_eq!(config.flashbots.poll_interval, 1000);
    }
} 