use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    monitoring::domain::{MonitoringService, ServiceStatus},
};
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Flashbots monitoring service
pub struct FlashbotsMonitor {
    config: Config,
    event_sender: mpsc::Sender<SwapEvent>,
    http_client: Client,
    status: ServiceStatus,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl FlashbotsMonitor {
    pub fn new(config: Config, event_sender: mpsc::Sender<SwapEvent>) -> Result<Self> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_millis(config.flashbots.request_timeout));

        // Add auth header if provided
        if !config.flashbots.auth_header.is_empty() {
            client_builder = client_builder.default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    reqwest::header::HeaderValue::from_str(&config.flashbots.auth_header)
                        .map_err(|e| anyhow::anyhow!("Invalid auth header: {}", e))?,
                );
                headers
            });
        }

        let http_client = client_builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let status = ServiceStatus::new("Flashbots Monitor".to_string());

        Ok(Self {
            config,
            event_sender,
            http_client,
            status,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }

    /// Start monitoring Flashbots bundles
    async fn start_monitoring(&self) -> Result<()> {
        let mut interval = interval(Duration::from_millis(self.config.flashbots.poll_interval));
        let is_running = self.is_running.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();
        let http_client = self.http_client.clone();

        info!("Starting Flashbots monitoring on {}", config.flashbots.rpc_url);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !*is_running.read().await {
                        break;
                    }

                    match self.poll_flashbots(&http_client, &config, &event_sender).await {
                        Ok(_) => {
                            // Status will be updated in the start/stop methods
                        }
                        Err(e) => {
                            error!("Flashbots polling error: {}", e);
                            // Status will be updated in the start/stop methods
                        }
                    }
                }
            }
        }

        info!("Flashbots monitoring stopped");
        Ok(())
    }

    /// Poll Flashbots for new bundles
    async fn poll_flashbots(
        &self,
        client: &Client,
        config: &Config,
        event_sender: &mpsc::Sender<SwapEvent>,
    ) -> Result<()> {
        let bundles = self.get_flashbots_bundles(client, config).await?;
        
        if !bundles.is_empty() {
            info!("Found {} Flashbots bundles", bundles.len());
            
            for bundle in bundles {
                if let Err(e) = event_sender.send(bundle).await {
                    error!("Failed to send Flashbots bundle: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get Flashbots bundles
    async fn get_flashbots_bundles(
        &self,
        client: &Client,
        config: &Config,
    ) -> Result<Vec<SwapEvent>> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "flashbots_getBundleStats",
            "params": [],
            "id": 1
        });

        let response = client
            .post(&config.flashbots.rpc_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Flashbots RPC request failed with status: {}",
                response.status()
            ));
        }

        let response_data: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON response: {}", e))?;

        self.parse_flashbots_response(response_data)
    }

    /// Parse Flashbots response and extract bundles
    fn parse_flashbots_response(&self, response: Value) -> Result<Vec<SwapEvent>> {
        let mut events = Vec::new();

        if let Some(result) = response.get("result") {
            if let Some(bundles) = result.as_array() {
                for bundle in bundles {
                    if let Some(event) = self.parse_bundle(bundle)? {
                        events.push(event);
                    }
                }
            }
        }

        Ok(events)
    }

    /// Parse a single bundle and convert to SwapEvent
    fn parse_bundle(&self, bundle: &Value) -> Result<Option<SwapEvent>> {
        // This is a simplified parser - in production you'd want more sophisticated parsing
        // to detect actual swap transactions within bundles
        
        if let (Some(bundle_hash), Some(block_number)) = (
            bundle.get("bundleHash").and_then(|h| h.as_str()),
            bundle.get("blockNumber").and_then(|b| b.as_u64()),
        ) {
            // Basic validation
            if bundle_hash.len() != 66 {
                return Ok(None);
            }

            // For now, return None - in production you'd implement actual swap detection
            // This is just a placeholder for the domain-driven structure
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Get bundle by hash
    async fn get_bundle_by_hash(&self, bundle_hash: &str) -> Result<Value> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "flashbots_getBundleByHash",
            "params": [bundle_hash],
            "id": 1
        });

        let response = self.http_client
            .post(&self.config.flashbots.rpc_url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get bundle by hash"));
        }

        let response_data: Value = response.json().await?;
        Ok(response_data)
    }

    /// Get bundle statistics
    async fn get_bundle_stats(&self) -> Result<Value> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "flashbots_getBundleStats",
            "params": [],
            "id": 1
        });

        let response = self.http_client
            .post(&self.config.flashbots.rpc_url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get bundle stats"));
        }

        let response_data: Value = response.json().await?;
        Ok(response_data)
    }

    /// Simulate a bundle
    async fn simulate_bundle(&self, bundle: Value) -> Result<Value> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "flashbots_simulateBundle",
            "params": [bundle],
            "id": 1
        });

        let response = self.http_client
            .post(&self.config.flashbots.rpc_url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to simulate bundle"));
        }

        let response_data: Value = response.json().await?;
        Ok(response_data)
    }
}

#[async_trait]
impl MonitoringService for FlashbotsMonitor {
    async fn start(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if *running_guard {
            warn!("Flashbots monitor is already running");
            return Ok(());
        }

        *running_guard = true;
        drop(running_guard);

        self.start_monitoring().await
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if !*running_guard {
            warn!("Flashbots monitor is not running");
            return Ok(());
        }

        *running_guard = false;
        self.status.mark_inactive();
        info!("Flashbots monitor stopped");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.status.is_active
    }

    fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }
}

impl Clone for FlashbotsMonitor {
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
    async fn test_flashbots_monitor_creation() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = FlashbotsMonitor::new(config, sender);
        assert!(monitor.is_ok());
    }

    #[test]
    fn test_monitor_status() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let monitor = FlashbotsMonitor::new(config, sender).unwrap();
        assert!(!monitor.is_active());
    }
} 