use crate::shared::types::{H160, H256};
use async_trait::async_trait;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub status: ServiceStatus,
    pub last_check: SystemTime,
    pub response_time: Duration,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Block information for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub number: u64,
    pub hash: String,
    pub timestamp: u64,
    pub parent_hash: String,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub miner: String,
    pub difficulty: u64,
    pub total_difficulty: u64,
    pub base_fee_per_gas: Option<u64>,
    pub extra_data: String,
}

/// Missed block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissedBlock {
    pub block_number: u64,
    pub expected_timestamp: u64,
    pub actual_timestamp: Option<u64>,
    pub missed_at: SystemTime,
    pub reason: MissedBlockReason,
    pub severity: MissedBlockSeverity,
    pub metadata: HashMap<String, String>,
    pub recovery_attempts: u32,
    pub recovered: bool,
}

/// Reasons why a block might be missed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MissedBlockReason {
    NetworkLatency,
    NodeUnresponsive,
    RpcError,
    InvalidBlock,
    ChainReorg,
    GasLimitExceeded,
    NonceMismatch,
    InsufficientFunds,
    ContractRevert,
    Timeout,
    ConnectionLost,
    RateLimitExceeded,
    Unknown,
}

/// Severity levels for missed blocks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MissedBlockSeverity {
    Low,      // Minor delays, no MEV impact
    Medium,   // Moderate delays, some MEV impact
    High,     // Significant delays, major MEV impact
    Critical, // Complete failure, severe MEV impact
}

/// Missed block statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissedBlockStats {
    pub total_missed: u64,
    pub total_recovered: u64,
    pub total_unrecovered: u64,
    pub average_recovery_time: Duration,
    pub longest_missed_streak: u32,
    pub current_missed_streak: u32,
    pub missed_by_reason: HashMap<MissedBlockReason, u64>,
    pub missed_by_severity: HashMap<MissedBlockSeverity, u64>,
    pub last_missed_block: Option<u64>,
    pub last_recovered_block: Option<u64>,
}

impl MissedBlockStats {
    pub fn increment_missed(&mut self, reason: MissedBlockReason, severity: MissedBlockSeverity) {
        self.total_missed += 1;
        self.last_missed_block = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as u64);
        
        *self.missed_by_reason.entry(reason).or_insert(0) += 1;
        *self.missed_by_severity.entry(severity).or_insert(0) += 1;
        
        self.current_missed_streak += 1;
        if self.current_missed_streak > self.longest_missed_streak {
            self.longest_missed_streak = self.current_missed_streak;
        }
    }
    
    pub fn increment_recovered(&mut self) {
        self.total_recovered += 1;
        self.last_recovered_block = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as u64);
        
        self.current_missed_streak = 0;
    }
    
    pub fn update_recovery_time(&mut self, recovery_time: Duration) {
        if self.total_recovered > 0 {
            let total_time = u64::from(self.average_recovery_time.as_secs()) * (self.total_recovered - 1) + recovery_time.as_secs();
            self.average_recovery_time = Duration::from_secs(total_time / self.total_recovered);
        } else {
            self.average_recovery_time = recovery_time;
        }
    }
    
    pub fn get_missed_rate(&self) -> f64 {
        if self.total_missed == 0 {
            0.0
        } else {
            (self.total_missed as f64) / (self.total_missed + self.total_recovered) as f64 * 100.0
        }
    }
    
    pub fn get_recovery_rate(&self) -> f64 {
        if self.total_missed == 0 {
            100.0
        } else {
            (self.total_recovered as f64) / self.total_missed as f64 * 100.0
        }
    }
}

/// Block monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMonitoringConfig {
    pub expected_block_time: Duration,
    pub max_block_delay: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub alert_threshold: u32,
    pub enable_recovery: bool,
    pub log_missed_blocks: bool,
    pub metrics_enabled: bool,
}

impl Default for BlockMonitoringConfig {
    fn default() -> Self {
        Self {
            expected_block_time: Duration::from_secs(12), // Ethereum mainnet
            max_block_delay: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(5),
            alert_threshold: 5,
            enable_recovery: true,
            log_missed_blocks: true,
            metrics_enabled: true,
        }
    }
}

/// Block monitoring service
pub struct BlockMonitor {
    config: BlockMonitoringConfig,
    stats: MissedBlockStats,
    missed_blocks: HashMap<u64, MissedBlock>,
    last_processed_block: Option<u64>,
    monitoring_started: SystemTime,
}

impl BlockMonitor {
    pub fn new(config: BlockMonitoringConfig) -> Self {
        Self {
            config,
            stats: MissedBlockStats::default(),
            missed_blocks: HashMap::new(),
            last_processed_block: None,
            monitoring_started: SystemTime::now(),
        }
    }
    
    /// Process a new block
    pub fn process_block(&mut self, block: &BlockInfo) -> Result<(), String> {
        let block_number = block.number;
        
        // Check if we missed any blocks
        if let Some(last_block) = self.last_processed_block {
            let expected_next = last_block + 1;
            if block_number > expected_next {
                // We missed some blocks
                for missed_number in expected_next..block_number {
                    self.record_missed_block(missed_number, block.timestamp, MissedBlockReason::Unknown, MissedBlockSeverity::Medium);
                }
            }
        }
        
        // Check if this block is delayed
        let expected_timestamp = self.get_expected_timestamp(block_number);
        if block.timestamp > expected_timestamp + self.config.max_block_delay.as_secs() {
            self.record_missed_block(block_number, block.timestamp, MissedBlockReason::NetworkLatency, MissedBlockSeverity::Low);
        }
        
        // Update last processed block
        self.last_processed_block = Some(block_number);
        
        // Check if we can recover any missed blocks
        if self.config.enable_recovery {
            self.attempt_recovery(block_number);
        }
        
        Ok(())
    }
    
    /// Record a missed block
    pub fn record_missed_block(&mut self, block_number: u64, actual_timestamp: u64, reason: MissedBlockReason, severity: MissedBlockSeverity) {
        let expected_timestamp = self.get_expected_timestamp(block_number);
        
        let missed_block = MissedBlock {
            block_number,
            expected_timestamp,
            actual_timestamp: Some(actual_timestamp),
            missed_at: SystemTime::now(),
            reason: reason.clone(),
            severity: severity.clone(),
            metadata: HashMap::new(),
            recovery_attempts: 0,
            recovered: false,
        };
        
        self.missed_blocks.insert(block_number, missed_block);
        self.stats.increment_missed(reason.clone(), severity.clone());
        
        if self.config.log_missed_blocks {
            tracing::warn!(
                "Missed block {}: reason={:?}, severity={:?}, expected={}, actual={}",
                block_number,
                reason,
                severity,
                expected_timestamp,
                actual_timestamp
            );
        }
    }
    
    /// Attempt to recover missed blocks
    pub fn attempt_recovery(&mut self, current_block: u64) {
        let mut recovered_blocks = Vec::new();
        let config = self.config.clone();
        
        // Collect all the data we need upfront to avoid borrow checker issues
        let block_numbers: Vec<u64> = self.missed_blocks.keys().copied().collect();
        let can_recover_blocks: Vec<(u64, bool)> = block_numbers
            .iter()
            .map(|&block_number| (block_number, self.can_recover_block(block_number, current_block)))
            .collect();
        let try_recover_results: Vec<(u64, bool)> = block_numbers
            .iter()
            .map(|&block_number| (block_number, self.try_recover_block(block_number)))
            .collect();
        
        for (block_number, missed_block) in &mut self.missed_blocks {
            if missed_block.recovered {
                continue;
            }
            
            if missed_block.recovery_attempts >= config.retry_attempts {
                continue;
            }
            
            // Check if we can recover this block
            let can_recover = can_recover_blocks.iter()
                .find(|(bn, _)| *bn == *block_number)
                .map(|(_, can)| *can)
                .unwrap_or(false);
                
            if can_recover {
                missed_block.recovery_attempts += 1;
                
                let try_recover = try_recover_results.iter()
                    .find(|(bn, _)| *bn == *block_number)
                    .map(|(_, result)| *result)
                    .unwrap_or(false);
                    
                if try_recover {
                    missed_block.recovered = true;
                    recovered_blocks.push(*block_number);
                    
                    let recovery_time = SystemTime::now()
                        .duration_since(missed_block.missed_at)
                        .unwrap_or(Duration::ZERO);
                    
                    self.stats.increment_recovered();
                    self.stats.update_recovery_time(recovery_time);
                    
                    tracing::info!(
                        "Recovered missed block {} after {} attempts in {:?}",
                        block_number,
                        missed_block.recovery_attempts,
                        recovery_time
                    );
                }
            }
        }
        
        // Remove recovered blocks
        for block_number in recovered_blocks {
            self.missed_blocks.remove(&block_number);
        }
    }
    
    /// Check if a block can be recovered
    fn can_recover_block(&self, missed_block: u64, current_block: u64) -> bool {
        // Only attempt recovery for recent blocks
        current_block.saturating_sub(missed_block) <= 100
    }
    
    /// Try to recover a specific block
    fn try_recover_block(&self, _block_number: u64) -> bool {
        // This would implement the actual recovery logic
        // For now, we'll simulate some recovery success with a probability
        // This makes testing more realistic
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        _block_number.hash(&mut hasher);
        let hash = hasher.finish();
        
        // 30% chance of successful recovery for testing purposes
        (hash % 10) < 3
    }
    
    /// Get expected timestamp for a block
    fn get_expected_timestamp(&self, block_number: u64) -> u64 {
        let genesis_timestamp = 1600000000; // Example genesis timestamp
        genesis_timestamp + (block_number * self.config.expected_block_time.as_secs())
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> &MissedBlockStats {
        &self.stats
    }
    
    /// Get missed blocks
    pub fn get_missed_blocks(&self) -> &HashMap<u64, MissedBlock> {
        &self.missed_blocks
    }
    
    /// Get monitoring uptime
    pub fn get_uptime(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.monitoring_started)
            .unwrap_or(Duration::ZERO)
    }
    
    /// Get the last processed block number
    pub fn get_last_processed_block(&self) -> Option<u64> {
        self.last_processed_block
    }
    
    /// Enable or disable recovery
    pub fn set_recovery_enabled(&mut self, enabled: bool) {
        self.config.enable_recovery = enabled;
    }
    
    /// Check if monitoring should alert
    pub fn should_alert(&self) -> bool {
        self.stats.current_missed_streak >= self.config.alert_threshold
    }
    
    /// Get alert message
    pub fn get_alert_message(&self) -> Option<String> {
        if self.should_alert() {
            Some(format!(
                "Critical: {} consecutive blocks missed. Current streak: {}. Total missed: {}",
                self.config.alert_threshold,
                self.stats.current_missed_streak,
                self.stats.total_missed
            ))
        } else {
            None
        }
    }
}

/// Block monitoring metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetrics {
    pub blocks_processed: u64,
    pub blocks_missed: u64,
    pub blocks_recovered: u64,
    pub current_missed_streak: u32,
    pub average_block_time: Duration,
    pub uptime: Duration,
    pub last_block_timestamp: Option<u64>,
    pub missed_block_rate: f64,
    pub recovery_rate: f64,
}

impl BlockMetrics {
    pub fn from_monitor(monitor: &BlockMonitor) -> Self {
        let stats = monitor.get_stats();
        
        Self {
            blocks_processed: monitor.last_processed_block.unwrap_or(0),
            blocks_missed: stats.total_missed,
            blocks_recovered: stats.total_recovered,
            current_missed_streak: stats.current_missed_streak,
            average_block_time: monitor.config.expected_block_time,
            uptime: monitor.get_uptime(),
            last_block_timestamp: monitor.last_processed_block,
            missed_block_rate: stats.get_missed_rate(),
            recovery_rate: stats.get_recovery_rate(),
        }
    }
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

/// Token metadata from subgraph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenMetadata {
    pub address: H160,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: Option<String>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub price_usd: Option<f64>,
    pub last_updated: u64,
}

/// Pool metadata from subgraph
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PoolMetadata {
    pub address: H160,
    pub token0: TokenMetadata,
    pub token1: TokenMetadata,
    pub fee_tier: u32,
    pub protocol: String,
    pub liquidity: f64,
    pub volume_24h: f64,
    pub tvl_usd: f64,
    pub last_updated: u64,
}

/// Subgraph query result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubgraphQueryResult<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<SubgraphError>>,
}

/// Subgraph error
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubgraphError {
    pub message: String,
    pub locations: Option<Vec<SubgraphLocation>>,
    pub path: Option<Vec<String>>,
}

/// Subgraph error location
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubgraphLocation {
    pub line: u32,
    pub column: u32,
}

/// Subgraph cache statistics
#[derive(Debug, Clone, Default)]
pub struct SubgraphCacheStats {
    pub tokens_cached: u64,
    pub pools_cached: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub last_refresh: Option<u64>,
    pub refresh_count: u64,
    pub errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{H160, H256};

    #[test]
    fn test_service_status() {
        let status = ServiceStatus::Healthy;
        
        // ServiceStatus is an enum, so we just test the variant
        assert!(matches!(status, ServiceStatus::Healthy));
        
        // Test other variants
        let degraded = ServiceStatus::Degraded;
        assert!(matches!(degraded, ServiceStatus::Degraded));
        
        let unhealthy = ServiceStatus::Unhealthy;
        assert!(matches!(unhealthy, ServiceStatus::Unhealthy));
        
        let unknown = ServiceStatus::Unknown;
        assert!(matches!(unknown, ServiceStatus::Unknown));
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