use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    monitoring::domain::{MonitoringService, ServiceStatus, TransactionInfo as MonitoringTransactionInfo},
    shared::{Result, types::H160, utils::time},
};
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
}

impl MempoolMonitor {
    pub fn new(config: Config, event_sender: mpsc::Sender<SwapEvent>) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(config.mempool.request_timeout))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let status = ServiceStatus::new("Mempool Monitor".to_string());

        Ok(Self {
            config,
            event_sender,
            http_client,
            status,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
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

                    if let Err(e) = self.poll_mempool(&http_client, &config, &event_sender).await {
                        error!("Mempool polling error: {}", e);
                        self.status.record_error(e.to_string());
                    } else {
                        self.status.mark_active();
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
            
            for tx in pending_txs {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

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
} 