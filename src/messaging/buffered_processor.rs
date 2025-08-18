use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::events::domain::SwapEvent;
use crate::events::filter::PoolFilter;
use crate::Result;
use crate::shared::config::FilteringConfig;

/// Configuration for buffered event processor
#[derive(Debug, Clone)]
pub struct BufferedProcessorConfig {
    pub processing_interval: Duration,
    pub batch_size: usize,
    pub max_workers: usize,
    pub backpressure_threshold: usize,
    pub enable_filtering: bool,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
}

impl Default for BufferedProcessorConfig {
    fn default() -> Self {
        Self {
            processing_interval: Duration::from_millis(100),
            batch_size: 100,
            max_workers: 4,
            backpressure_threshold: 1000,
            enable_filtering: true,
            enable_caching: true,
            cache_ttl_seconds: 3600,
        }
    }
}

/// Processor statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    pub events_received: u64,
    pub events_processed: u64,
    pub events_filtered: u64,
    pub events_cached: u64,
    pub events_dropped: u64,
    pub errors: u64,
    pub processing_time: Duration,
    pub worker_count: usize,
}

impl ProcessorStats {
    pub fn increment_events_received(&mut self, count: u64) {
        self.events_received += count;
    }

    pub fn increment_events_processed(&mut self, count: u64) {
        self.events_processed += count;
    }

    pub fn increment_events_filtered(&mut self, count: u64) {
        self.events_filtered += count;
    }

    pub fn increment_events_cached(&mut self, count: u64) {
        self.events_cached += count;
    }

    pub fn increment_events_dropped(&mut self, count: u64) {
        self.events_dropped += count;
    }

    pub fn increment_errors(&mut self) {
        self.errors += 1;
    }

    pub fn update_processing_time(&mut self, time: Duration) {
        self.processing_time += time;
    }

    pub fn set_worker_count(&mut self, count: usize) {
        self.worker_count = count;
    }
}

/// Buffered event processor that handles event buffering, filtering, and processing
pub struct BufferedEventProcessor {
    config: BufferedProcessorConfig,
    buffer_manager: crate::messaging::buffer::BufferManager,
    filter: Arc<RwLock<PoolFilter>>,
    stats: Arc<RwLock<ProcessorStats>>,
    is_running: Arc<RwLock<bool>>,
    workers: HashMap<String, tokio::task::JoinHandle<()>>,
}

impl BufferedEventProcessor {
    /// Create a new buffered event processor
    pub fn new(
        config: BufferedProcessorConfig,
        buffer_manager: crate::messaging::buffer::BufferManager,
        filter: PoolFilter,
    ) -> Self {
        Self {
            config,
            buffer_manager,
            filter: Arc::new(RwLock::new(filter)),
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            workers: HashMap::new(),
        }
    }

    /// Start the processor
    pub async fn start(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if *running_guard {
            warn!("Processor is already running");
            return Ok(());
        }

        *running_guard = true;
        drop(running_guard);

        info!("Starting buffered event processor");
        
        // Start processing loop
        let processing_loop = self.start_processing_loop().await?;
        self.workers.insert("processing".to_string(), processing_loop);

        // Start health monitoring
        let health_monitor = self.start_health_monitor().await?;
        self.workers.insert("health".to_string(), health_monitor);

        info!("Buffered event processor started with {} workers", self.workers.len());
        Ok(())
    }

    /// Stop the processor
    pub async fn stop(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if !*running_guard {
            warn!("Processor is not running");
            return Ok(());
        }

        *running_guard = false;
        drop(running_guard);

        info!("Stopping buffered event processor");

        // Cancel all workers
        for (name, worker) in &mut self.workers {
            info!("Cancelling worker: {}", name);
            worker.abort();
        }

        // Wait for workers to finish
        for (name, worker) in self.workers.drain() {
            match worker.await {
                Ok(_) => debug!("Worker {} finished gracefully", name),
                Err(e) if e.is_cancelled() => debug!("Worker {} was cancelled", name),
                Err(e) => error!("Worker {} failed: {}", name, e),
            }
        }

        info!("Buffered event processor stopped");
        Ok(())
    }

    /// Process a single event
    pub async fn process_event(&mut self, event: &SwapEvent) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        // Update stats
        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_received(1);
        }

        // Apply filtering if enabled
        if self.config.enable_filtering {
            let filter_guard = self.filter.read().await;
            let events = vec![event.clone()];
            let filtered_events = filter_guard.filter_events(&events);
            
            if filtered_events.is_empty() {
                let mut stats_guard = self.stats.write().await;
                stats_guard.increment_events_filtered(1);
                return Ok(());
            }
        }

        // Check backpressure
        if self.should_apply_backpressure().await? {
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_dropped(1);
            warn!("Backpressure applied, dropping event");
            return Ok(());
        }

        // Get default buffer
        let enable_caching = self.config.enable_caching;
        let cache_ttl = self.config.cache_ttl_seconds;
        
        {
            let buffer_manager = self.get_buffer_manager_mut();
            if let Some(buffer) = buffer_manager.get_buffer("default") {
                // Buffer the event
                buffer.buffer_event(event).await?;

                // Cache the event if enabled
                if enable_caching {
                    let cache_key = format!("event:{}", event.transaction.hash);
                    buffer.cache_event(&cache_key, event, Duration::from_secs(cache_ttl)).await?;
                }

                // Add to stream for persistence
                buffer.add_to_stream(event).await?;

                // Update stats
                {
                    let mut stats_guard = self.stats.write().await;
                    stats_guard.increment_events_processed(1);
                    stats_guard.update_processing_time(start_time.elapsed());
                }

                debug!("Event processed successfully");
            } else {
                error!("Default buffer not found");
                return Err(anyhow::anyhow!("Default buffer not found"));
            }
        }

        Ok(())
    }

    /// Process multiple events
    pub async fn process_events(&mut self, events: &[SwapEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let start_time = std::time::Instant::now();
        
        // Update stats
        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_received(events.len() as u64);
        }

        // Apply filtering if enabled
        let events_to_process = if self.config.enable_filtering {
            let filter_guard = self.filter.read().await;
            let filtered_events = filter_guard.filter_events(events);
            
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_filtered((events.len() - filtered_events.len()) as u64);
            
            filtered_events
        } else {
            events.to_vec()
        };

        if events_to_process.is_empty() {
            return Ok(());
        }

        // Check backpressure
        if self.should_apply_backpressure().await? {
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_dropped(events_to_process.len() as u64);
            warn!("Backpressure applied, dropping {} events", events_to_process.len());
            return Ok(());
        }

        // Get default buffer
        let enable_caching = self.config.enable_caching;
        let cache_ttl = self.config.cache_ttl_seconds;
        
        {
            let buffer_manager = self.get_buffer_manager_mut();
            if let Some(buffer) = buffer_manager.get_buffer("default") {
                // Buffer the events
                buffer.buffer_events(&events_to_process).await?;

                // Cache the events if enabled
                if enable_caching {
                    for event in &events_to_process {
                        let cache_key = format!("event:{}", event.transaction.hash);
                        buffer.cache_event(&cache_key, event, Duration::from_secs(cache_ttl)).await?;
                    }
                }

                // Add to stream for persistence
                for event in &events_to_process {
                    buffer.add_to_stream(event).await?;
                }
            } else {
                error!("Default buffer not found");
                return Err(anyhow::anyhow!("Default buffer not found"));
            }
        }

        // Update stats
        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.increment_events_processed(events_to_process.len() as u64);
            stats_guard.update_processing_time(start_time.elapsed());
        }

        info!("Processed {} events successfully", events_to_process.len());

        Ok(())
    }

    /// Get processor statistics
    pub async fn get_stats(&self) -> ProcessorStats {
        let stats_guard = self.stats.read().await;
        let mut stats = stats_guard.clone();
        stats.set_worker_count(self.workers.len());
        stats
    }

    /// Check if the processor is running
    pub async fn is_running(&self) -> bool {
        let running_guard = self.is_running.read().await;
        *running_guard
    }

    /// Get buffer manager reference
    pub fn get_buffer_manager(&self) -> &crate::messaging::buffer::BufferManager {
        &self.buffer_manager
    }

    /// Get buffer manager mutable reference
    pub fn get_buffer_manager_mut(&mut self) -> &mut crate::messaging::buffer::BufferManager {
        &mut self.buffer_manager
    }

    // Private methods

    /// Start the main processing loop
    async fn start_processing_loop(&self) -> Result<tokio::task::JoinHandle<()>> {
        let config = self.config.clone();
        let mut buffer_manager = self.buffer_manager.clone();
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.processing_interval);
            
            while {
                let running_guard = is_running.read().await;
                *running_guard
            } {
                interval.tick().await;

                // Process events from buffer
                if let Some(buffer) = buffer_manager.get_buffer("default") {
                    match buffer.process_events(Some(config.batch_size)).await {
                        Ok(events) => {
                            if !events.is_empty() {
                                debug!("Processed {} events from buffer", events.len());
                                let mut stats_guard = stats.write().await;
                                stats_guard.increment_events_processed(events.len() as u64);
                            }
                        }
                        Err(e) => {
                            error!("Failed to process events from buffer: {}", e);
                            let mut stats_guard = stats.write().await;
                            stats_guard.increment_errors();
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Start health monitoring
    async fn start_health_monitor(&self) -> Result<tokio::task::JoinHandle<()>> {
        let mut buffer_manager = self.buffer_manager.clone();
        let is_running = self.is_running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            while {
                let running_guard = is_running.read().await;
                *running_guard
            } {
                interval.tick().await;

                // Health check all buffers
                if let Err(e) = buffer_manager.health_check_all().await {
                    error!("Buffer health check failed: {}", e);
                }
            }
        });

        Ok(handle)
    }

    /// Check if backpressure should be applied
    async fn should_apply_backpressure(&mut self) -> Result<bool> {
        if let Some(buffer) = self.buffer_manager.get_buffer("default") {
            let queue_size = buffer.get_queue_size().await?;
            Ok(queue_size >= self.config.backpressure_threshold)
        } else {
            Ok(false)
        }
    }
}

impl Clone for BufferedEventProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            buffer_manager: self.buffer_manager.clone(),
            filter: self.filter.clone(),
            stats: self.stats.clone(),
            is_running: self.is_running.clone(),
            workers: HashMap::new(), // Workers can't be cloned
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::FilteringConfig;

    #[tokio::test]
    async fn test_processor_config_default() {
        let config = BufferedProcessorConfig::default();
        assert_eq!(config.processing_interval, Duration::from_millis(100));
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.backpressure_threshold, 1000);
        assert!(config.enable_filtering);
        assert!(config.enable_caching);
        assert_eq!(config.cache_ttl_seconds, 3600);
    }

    #[tokio::test]
    async fn test_processor_stats() {
        let mut stats = ProcessorStats::default();
        stats.increment_events_received(10);
        stats.increment_events_processed(8);
        stats.increment_events_filtered(2);
        stats.increment_errors();
        
        assert_eq!(stats.events_received, 10);
        assert_eq!(stats.events_processed, 8);
        assert_eq!(stats.events_filtered, 2);
        assert_eq!(stats.errors, 1);
    }

    #[tokio::test]
    async fn test_processor_creation() {
        let config = BufferedProcessorConfig::default();
        let buffer_manager = crate::messaging::buffer::BufferManager::new(crate::messaging::buffer::BufferConfig::default());
        let filter_config = FilteringConfig::default();
        let filter = PoolFilter::new(filter_config);
        
        let processor = BufferedEventProcessor::new(config, buffer_manager, filter);
        assert!(!processor.is_running().await);
    }
} 