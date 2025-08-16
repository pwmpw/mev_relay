use crate::shared::types::{H160, H256};
use async_trait::async_trait;
use std::collections::HashMap;

/// Monitoring service trait for different Ethereum monitoring sources
#[async_trait]
pub trait MonitoringService: Send + Sync {
    /// Start monitoring
    async fn start(&mut self) -> crate::Result<()>;
    
    /// Stop monitoring
    async fn stop(&mut self) -> crate::Result<()>;
    
    /// Check if monitoring is active
    fn is_active(&self) -> bool;
    
    /// Get service status
    fn get_status(&self) -> ServiceStatus;
}

/// Service status information
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub is_active: bool,
    pub last_activity: Option<u64>,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub metrics: HashMap<String, u64>,
}

impl ServiceStatus {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_active: false,
            last_activity: None,
            error_count: 0,
            last_error: None,
            metrics: HashMap::new(),
        }
    }

    pub fn mark_active(&mut self) {
        self.is_active = true;
        self.last_activity = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }

    pub fn mark_inactive(&mut self) {
        self.is_active = false;
    }

    pub fn record_error(&mut self, error: String) {
        self.error_count += 1;
        self.last_error = Some(error);
    }

    pub fn update_metric(&mut self, key: String, value: u64) {
        self.metrics.insert(key, value);
    }
}

/// Block information from monitoring
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub number: u64,
    pub hash: H256,
    pub timestamp: u64,
    pub transactions: Vec<TransactionInfo>,
}

/// Transaction information from monitoring
#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub hash: H256,
    pub from: H160,
    pub to: Option<H160>,
    pub value: u128,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub block_number: Option<u64>,
    pub block_hash: Option<H256>,
    pub transaction_index: Option<u64>,
}

/// Mempool transaction pool
#[derive(Debug, Clone)]
pub struct MempoolPool {
    pub pending: HashMap<H256, TransactionInfo>,
    pub queued: HashMap<H256, TransactionInfo>,
}

impl MempoolPool {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            queued: HashMap::new(),
        }
    }

    pub fn add_pending(&mut self, tx: TransactionInfo) {
        self.pending.insert(tx.hash, tx);
    }

    pub fn add_queued(&mut self, tx: TransactionInfo) {
        self.queued.insert(tx.hash, tx);
    }

    pub fn remove_pending(&mut self, hash: &H256) {
        self.pending.remove(hash);
    }

    pub fn remove_queued(&mut self, hash: &H256) {
        self.queued.remove(hash);
    }

    pub fn get_pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn get_queued_count(&self) -> usize {
        self.queued.len()
    }

    pub fn get_all_pending(&self) -> Vec<&TransactionInfo> {
        self.pending.values().collect()
    }

    pub fn get_all_queued(&self) -> Vec<&TransactionInfo> {
        self.queued.values().collect()
    }
}

impl Default for MempoolPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Flashbots bundle information
#[derive(Debug, Clone)]
pub struct FlashbotsBundle {
    pub hash: H256,
    pub block_number: u64,
    pub transactions: Vec<TransactionInfo>,
    pub miner_reward: u128,
    pub coinbase_transaction: Option<TransactionInfo>,
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub poll_interval: u64,
    pub max_concurrent_requests: usize,
    pub request_timeout: u64,
    pub batch_size: usize,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval: 100, // milliseconds
            max_concurrent_requests: 50,
            request_timeout: 10000, // 10 seconds
            batch_size: 100,
        }
    }
}

/// Monitoring metrics
#[derive(Debug, Clone, Default)]
pub struct MonitoringMetrics {
    pub total_blocks_processed: u64,
    pub total_transactions_processed: u64,
    pub total_swap_events_detected: u64,
    pub last_block_number: Option<u64>,
    pub last_block_timestamp: Option<u64>,
    pub mempool_size: u64,
    pub flashbots_bundles_processed: u64,
}

impl MonitoringMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_blocks_processed(&mut self) {
        self.total_blocks_processed += 1;
    }

    pub fn increment_transactions_processed(&mut self, count: u64) {
        self.total_transactions_processed += count;
    }

    pub fn increment_swap_events(&mut self, count: u64) {
        self.total_swap_events_detected += count;
    }

    pub fn update_last_block(&mut self, number: u64, timestamp: u64) {
        self.last_block_number = Some(number);
        self.last_block_timestamp = Some(timestamp);
    }

    pub fn update_mempool_size(&mut self, size: u64) {
        self.mempool_size = size;
    }

    pub fn increment_flashbots_bundles(&mut self) {
        self.flashbots_bundles_processed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{H160, H256};

    #[test]
    fn test_service_status() {
        let mut status = ServiceStatus::new("test_service".to_string());
        
        assert!(!status.is_active);
        assert_eq!(status.error_count, 0);
        
        status.mark_active();
        assert!(status.is_active);
        assert!(status.last_activity.is_some());
        
        status.record_error("test error".to_string());
        assert_eq!(status.error_count, 1);
        assert_eq!(status.last_error, Some("test error".to_string()));
    }

    #[test]
    fn test_mempool_pool() {
        let mut pool = MempoolPool::new();
        
        let tx = TransactionInfo {
            hash: H256([1; 32]),
            from: H160([1; 20]),
            to: Some(H160([2; 20])),
            value: 0,
            gas_price: 20_000_000_000,
            gas_limit: 100000,
            nonce: 1,
            data: vec![],
            block_number: None,
            block_hash: None,
            transaction_index: None,
        };
        
        pool.add_pending(tx.clone());
        assert_eq!(pool.get_pending_count(), 1);
        assert_eq!(pool.get_queued_count(), 0);
        
        pool.add_queued(tx);
        assert_eq!(pool.get_queued_count(), 1);
    }

    #[test]
    fn test_monitoring_metrics() {
        let mut metrics = MonitoringMetrics::new();
        
        assert_eq!(metrics.total_blocks_processed, 0);
        assert_eq!(metrics.total_transactions_processed, 0);
        
        metrics.increment_blocks_processed();
        metrics.increment_transactions_processed(5);
        metrics.increment_swap_events(2);
        
        assert_eq!(metrics.total_blocks_processed, 1);
        assert_eq!(metrics.total_transactions_processed, 5);
        assert_eq!(metrics.total_swap_events_detected, 2);
        
        metrics.update_last_block(12345, 1234567890);
        assert_eq!(metrics.last_block_number, Some(12345));
        assert_eq!(metrics.last_block_timestamp, Some(1234567890));
    }
} 