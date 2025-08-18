use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient, RedisError};
use redis::streams::StreamMaxlen;
use crate::events::domain::SwapEvent;
use crate::Result;

/// Configuration for Redis buffer
#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub max_queue_size: usize,
    pub batch_size: usize,
    pub stream_max_len: usize,
    pub cache_ttl_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            batch_size: 100,
            stream_max_len: 1000,
            cache_ttl_seconds: 3600,
            retry_attempts: 3,
            retry_delay_ms: 100,
        }
    }
}

/// Buffer statistics
#[derive(Debug, Clone, Default)]
pub struct BufferStats {
    pub events_buffered: u64,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub events_cached: u64,
    pub events_retrieved: u64,
    pub errors: u64,
    pub total_latency: Duration,
    pub batch_count: u64,
}

impl BufferStats {
    pub fn increment_events_buffered(&mut self, count: u64) {
        self.events_buffered += count;
    }

    pub fn increment_events_processed(&mut self, count: u64) {
        self.events_processed += count;
    }

    pub fn increment_events_dropped(&mut self, count: u64) {
        self.events_dropped += count;
    }

    pub fn increment_events_cached(&mut self, count: u64) {
        self.events_cached += count;
    }

    pub fn increment_events_retrieved(&mut self, count: u64) {
        self.events_retrieved += count;
    }

    pub fn increment_errors(&mut self) {
        self.errors += 1;
    }

    pub fn update_latency(&mut self, latency: Duration) {
        self.total_latency += latency;
    }

    pub fn increment_batch_count(&mut self) {
        self.batch_count += 1;
    }

    pub fn get_average_latency(&self) -> Duration {
        if self.events_buffered > 0 {
            Duration::from_nanos(self.total_latency.as_nanos() as u64 / self.events_buffered)
        } else {
            Duration::ZERO
        }
    }
}

/// Redis buffer implementation
#[derive(Clone)]
pub struct RedisBuffer {
    config: BufferConfig,
    connection_manager: ConnectionManager,
    stats: Arc<RwLock<BufferStats>>,
    queue_name: String,
    stream_name: String,
    cache_prefix: String,
}

impl RedisBuffer {
    /// Create a new Redis buffer
    pub async fn new(
        redis_url: String,
        queue_name: String,
        config: Option<BufferConfig>,
    ) -> Result<Self> {
        let redis_client = RedisClient::open(redis_url)
            .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;

        let connection_manager = redis::aio::ConnectionManager::new(redis_client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Redis connection manager: {}", e))?;

        let config = config.unwrap_or_default();
        let stats = Arc::new(RwLock::new(BufferStats::default()));
        let stream_name = format!("{}:stream", queue_name);
        let cache_prefix = format!("{}:cache", queue_name);

        Ok(Self {
            config,
            connection_manager,
            stats,
            queue_name,
            stream_name,
            cache_prefix,
        })
    }

    /// Buffer a single event
    pub async fn buffer_event(&mut self, event: &SwapEvent) -> Result<()> {
        let start_time = Instant::now();
        
        // Check queue size limit
        let queue_size = self.get_queue_size().await?;
        if queue_size >= self.config.max_queue_size {
            self.drop_event(event).await;
            return Ok(());
        }

        // Serialize event
        let event_json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;

        // Add to Redis list (queue)
        let result: std::result::Result<(), redis::RedisError> = self.connection_manager
            .lpush(&self.queue_name, event_json)
            .await;

        match result {
            Ok(_) => {
                self.update_stats_success(start_time.elapsed()).await;
                debug!("Event buffered in queue: {}", self.queue_name);
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                error!("Failed to buffer event: {}", e);
                Err(anyhow::anyhow!("Redis buffer failed: {}", e))
            }
        }
    }

    /// Buffer multiple events in a batch
    pub async fn buffer_events(&mut self, events: &[SwapEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let start_time = Instant::now();
        let mut pipeline = redis::pipe();

        // Check queue size limit
        let queue_size = self.get_queue_size().await?;
        let available_slots = self.config.max_queue_size.saturating_sub(queue_size);
        let events_to_buffer = events.len().min(available_slots);

        if events_to_buffer < events.len() {
            warn!("Queue full, dropping {} events", events.len() - events_to_buffer);
            self.update_stats_dropped(events.len() - events_to_buffer).await;
        }

        // Add events to pipeline
        for event in &events[..events_to_buffer] {
            let event_json = serde_json::to_string(event)
                .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;
            pipeline.lpush(&self.queue_name, event_json);
        }

        // Execute pipeline
        let result: std::result::Result<Vec<()>, redis::RedisError> = pipeline
            .query_async(&mut self.connection_manager)
            .await;

        match result {
            Ok(_) => {
                self.update_stats_batch_success(events_to_buffer, start_time.elapsed()).await;
                info!("Buffered {} events in queue: {}", events_to_buffer, self.queue_name);
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                error!("Failed to buffer events batch: {}", e);
                Err(anyhow::anyhow!("Redis batch buffer failed: {}", e))
            }
        }
    }

    /// Process events from the queue
    pub async fn process_events(&mut self, batch_size: Option<usize>) -> Result<Vec<SwapEvent>> {
        let batch_size = batch_size.unwrap_or(self.config.batch_size);
        let mut events = Vec::new();

        for _ in 0..batch_size {
            // Pop event from queue (right side - FIFO)
            let result: std::result::Result<Option<String>, redis::RedisError> = self.connection_manager
                .rpop(&self.queue_name, None)
                .await;

            match result {
                Ok(Some(event_json)) => {
                    match serde_json::from_str::<SwapEvent>(&event_json) {
                        Ok(event) => {
                            events.push(event);
                            self.update_stats_processed().await;
                        }
                        Err(e) => {
                            error!("Failed to deserialize event: {}", e);
                            self.update_stats_error().await;
                        }
                    }
                }
                Ok(None) => break, // Queue is empty
                Err(e) => {
                    error!("Failed to pop event from queue: {}", e);
                    self.update_stats_error().await;
                    break;
                }
            }
        }

        Ok(events)
    }

    /// Add event to Redis stream for persistence
    pub async fn add_to_stream(&mut self, event: &SwapEvent) -> Result<()> {
        let event_json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis();

        let stream_data = vec![
            ("event", event_json),
            ("timestamp", timestamp.to_string()),
        ];

        let result: std::result::Result<String, redis::RedisError> = self.connection_manager
            .xadd_maxlen(&self.stream_name, StreamMaxlen::Equals(self.config.stream_max_len), "*", &stream_data)
            .await;

        match result {
            Ok(_) => {
                debug!("Event added to stream: {}", self.stream_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to add event to stream: {}", e);
                Err(anyhow::anyhow!("Redis stream failed: {}", e))
            }
        }
    }

    /// Cache event with TTL
    pub async fn cache_event(&mut self, key: &str, event: &SwapEvent, ttl: Duration) -> Result<()> {
        let event_json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;

        let cache_key = format!("{}:{}", self.cache_prefix, key);
        let mut pipeline = redis::pipe();
        pipeline.set(&cache_key, event_json);
        pipeline.expire(&cache_key, ttl.as_secs() as i64);

        let result: std::result::Result<Vec<()>, redis::RedisError> = pipeline
            .query_async(&mut self.connection_manager)
            .await;

        match result {
            Ok(_) => {
                self.update_stats_cached().await;
                debug!("Event cached: {}", cache_key);
                Ok(())
            }
            Err(e) => {
                error!("Failed to cache event: {}", e);
                Err(anyhow::anyhow!("Redis cache failed: {}", e))
            }
        }
    }

    /// Retrieve cached event
    pub async fn get_cached_event(&mut self, key: &str) -> Result<Option<SwapEvent>> {
        let cache_key = format!("{}:{}", self.cache_prefix, key);

        let result: std::result::Result<Option<String>, redis::RedisError> = self.connection_manager
            .get(&cache_key)
            .await;

        match result {
            Ok(Some(event_json)) => {
                match serde_json::from_str::<SwapEvent>(&event_json) {
                    Ok(event) => {
                        self.update_stats_retrieved().await;
                        Ok(Some(event))
                    }
                    Err(e) => {
                        error!("Failed to deserialize cached event: {}", e);
                        Err(anyhow::anyhow!("Failed to deserialize cached event: {}", e))
                    }
                }
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!("Failed to retrieve cached event: {}", e);
                Err(anyhow::anyhow!("Redis cache retrieval failed: {}", e))
            }
        }
    }

    /// Get current queue size
    pub async fn get_queue_size(&mut self) -> Result<usize> {
        let result: std::result::Result<usize, redis::RedisError> = self.connection_manager
            .llen(&self.queue_name)
            .await;

        match result {
            Ok(size) => Ok(size),
            Err(e) => {
                error!("Failed to get queue size: {}", e);
                Err(anyhow::anyhow!("Redis queue size failed: {}", e))
            }
        }
    }

    /// Clear the queue
    pub async fn clear_queue(&mut self) -> Result<()> {
        let result: std::result::Result<(), redis::RedisError> = self.connection_manager
            .del(&self.queue_name)
            .await;

        match result {
            Ok(_) => {
                info!("Queue cleared: {}", self.queue_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to clear queue: {}", e);
                Err(anyhow::anyhow!("Redis queue clear failed: {}", e))
            }
        }
    }

    /// Health check
    pub async fn health_check(&mut self) -> Result<()> {
        // Simple health check - try to get queue size
        let _size = self.get_queue_size().await?;
        Ok(())
    }

    /// Get buffer statistics
    pub async fn get_stats(&self) -> BufferStats {
        let stats_guard = self.stats.read().await;
        stats_guard.clone()
    }

    // Private helper methods
    async fn drop_event(&self, _event: &SwapEvent) {
        // In a real implementation, you might want to log or store dropped events
        debug!("Event dropped due to queue size limit");
    }

    async fn update_stats_success(&self, latency: Duration) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_buffered(1);
        stats_guard.update_latency(latency);
    }

    async fn update_stats_batch_success(&self, count: usize, latency: Duration) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_buffered(count as u64);
        stats_guard.update_latency(latency);
        stats_guard.increment_batch_count();
    }

    async fn update_stats_processed(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_processed(1);
    }

    async fn update_stats_dropped(&self, count: usize) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_dropped(count as u64);
    }

    async fn update_stats_cached(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_cached(1);
    }

    async fn update_stats_retrieved(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_events_retrieved(1);
    }

    async fn update_stats_error(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_errors();
    }
}

/// Buffer manager for coordinating multiple buffers
#[derive(Clone)]
pub struct BufferManager {
    pub buffers: HashMap<String, RedisBuffer>,
    pub config: BufferConfig,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new(config: BufferConfig) -> Self {
        Self {
            buffers: HashMap::new(),
            config,
        }
    }

    /// Add a buffer
    pub async fn add_buffer(&mut self, name: String, redis_url: String) -> Result<()> {
        let buffer = RedisBuffer::new(redis_url, name.clone(), Some(self.config.clone())).await?;
        self.buffers.insert(name, buffer);
        Ok(())
    }

    /// Get a buffer by name
    pub fn get_buffer(&mut self, name: &str) -> Option<&mut RedisBuffer> {
        self.buffers.get_mut(name)
    }

    /// Remove a buffer
    pub fn remove_buffer(&mut self, name: &str) -> Option<RedisBuffer> {
        self.buffers.remove(name)
    }

    /// Get all buffer names
    pub fn get_buffer_names(&self) -> Vec<String> {
        self.buffers.keys().cloned().collect()
    }

    /// Health check all buffers
    pub async fn health_check_all(&mut self) -> Result<()> {
        for (name, buffer) in &mut self.buffers {
            if let Err(e) = buffer.health_check().await {
                error!("Buffer '{}' health check failed: {}", name, e);
                return Err(e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::domain::{SwapEvent, EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo};
use crate::shared::types::{H160, H256};

    fn create_test_event() -> SwapEvent {
        SwapEvent {
            id: EventId::new(),
            source: EventSource::Mempool,
            protocol: ProtocolInfo {
                name: "Uniswap V2".to_string(),
                version: "2.0".to_string(),
                address: H160::from([1u8; 20]),
            },
            transaction: TransactionInfo {
                hash: H256::from([2u8; 32]),
                from: H160::from([3u8; 20]),
                to: Some(H160::from([4u8; 20])),
                gas_price: 20000000000u128,
                gas_limit: 300000u64,
                gas_used: 150000u64,
                nonce: 5u64,
                value: 0u128,
            },
            swap_details: SwapDetails {
                token_in: H160::from([6u8; 20]),
                token_out: H160::from([7u8; 20]),
                amount_in: 1000000000000000000u128,
                amount_out: 2000000000000000000u128,
                pool_address: Some(H160::from([8u8; 20])),
                fee_tier: Some(3000u32),
            },
            block_info: BlockInfo {
                number: 12345u64,
                timestamp: 1640995200u64,
                hash: H256::from([9u8; 32]),
            },
            metadata: crate::events::domain::EventMetadata::new(),
        }
    }

    #[tokio::test]
    async fn test_buffer_config_default() {
        let config = BufferConfig::default();
        assert_eq!(config.max_queue_size, 10000);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.stream_max_len, 1000);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_buffer_stats() {
        let mut stats = BufferStats::default();
        stats.increment_events_buffered(5);
        stats.increment_events_processed(3);
        stats.increment_errors();
        
        assert_eq!(stats.events_buffered, 5);
        assert_eq!(stats.events_processed, 3);
        assert_eq!(stats.errors, 1);
    }

    #[tokio::test]
    async fn test_buffer_manager() {
        let config = BufferConfig::default();
        let mut manager = BufferManager::new(config);
        
        assert_eq!(manager.get_buffer_names().len(), 0);
        assert!(manager.get_buffer("nonexistent").is_none());
    }
} 