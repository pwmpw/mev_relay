use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    messaging::{domain::{EventPublisher, PublisherStats}, redis::RedisPublisher},
};
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Event publisher service that handles publishing swap events
pub struct EventPublisherService {
    config: Config,
    event_receiver: mpsc::Receiver<SwapEvent>,
    redis_publisher: RedisPublisher,
    stats: Arc<tokio::sync::RwLock<PublisherStats>>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl EventPublisherService {
    pub async fn new(config: Config, event_receiver: mpsc::Receiver<SwapEvent>) -> Result<Self> {
        let redis_publisher = RedisPublisher::new(config.clone()).await?;
        let stats = Arc::new(tokio::sync::RwLock::new(PublisherStats::new()));

        Ok(Self {
            config,
            event_receiver,
            redis_publisher,
            stats,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }

    /// Start the event publisher service
    pub async fn start(&mut self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if *running_guard {
            warn!("Event publisher service is already running");
            return Ok(());
        }

        info!("Starting event publisher service");

        *running_guard = true;
        drop(running_guard);

        self.run_event_loop().await?;

        Ok(())
    }

    /// Main event processing loop
    async fn run_event_loop(&mut self) -> Result<()> {
        info!("Event publisher service started, waiting for events...");

        while let Some(event) = self.event_receiver.recv().await {
            let start_time = std::time::Instant::now();

            // Process the event
            match self.process_event(&event).await {
                Ok(_) => {
                    let duration = start_time.elapsed().as_millis() as f64 / 1000.0;
                    self.update_stats_success(&event, duration).await;
                    
                    info!(
                        "Event published successfully - ID: {}, Source: {}, Protocol: {}",
                        event.id.as_str(),
                        event.source,
                        event.protocol.name
                    );
                }
                Err(e) => {
                    self.update_stats_error(&event).await;
                    error!("Failed to publish event {}: {}", event.id.as_str(), e);
                }
            }
        }

        info!("Event receiver closed, stopping publisher service");
        Ok(())
    }

    /// Process a single event
    async fn process_event(&mut self, event: &SwapEvent) -> Result<()> {
        // Validate the event
        event.validate()
            .map_err(|e| anyhow::anyhow!("Event validation failed: {}", e))?;

        // Publish to Redis
        self.redis_publisher.publish_event(event).await?;

        Ok(())
    }

    /// Update statistics for successful event processing
    async fn update_stats_success(&self, event: &SwapEvent, duration: f64) {
        let mut stats_guard = self.stats.write().await;
        
        stats_guard.increment_events_published(1);
        stats_guard.increment_source_count(&event.source.to_string());
        stats_guard.increment_protocol_count(&event.protocol.name);
        stats_guard.update_average_latency(duration);
    }

    /// Update statistics for failed event processing
    async fn update_stats_error(&self, _event: &SwapEvent) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_error_count();
    }

    /// Get publisher statistics
    pub async fn get_stats(&self) -> Result<PublisherStats> {
        let stats_guard = self.stats.read().await;
        Ok(stats_guard.clone())
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        let running_guard = self.is_running.read().await;
        *running_guard
    }

    /// Get Redis publisher reference
    pub fn get_redis_publisher(&self) -> &RedisPublisher {
        &self.redis_publisher
    }

    /// Get service configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::domain::{EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_event_publisher_creation() {
        let config = Config::default();
        let (_sender, receiver) = mpsc::channel::<SwapEvent>(100);
        
        let publisher = EventPublisherService::new(config, receiver);
        assert!(publisher.await.is_ok());
    }

    #[tokio::test]
    async fn test_publisher_config() {
        let config = Config::default();
        let (_sender, receiver) = mpsc::channel::<SwapEvent>(100);
        
        let publisher = EventPublisherService::new(config, receiver).await.unwrap();
        assert_eq!(publisher.get_config().redis.channel, "mev_swaps");
    }

    #[tokio::test]
    async fn test_publisher_running_state() {
        let config = Config::default();
        let (_sender, receiver) = mpsc::channel::<SwapEvent>(100);
        
        let publisher = EventPublisherService::new(config, receiver).await.unwrap();
        assert!(!publisher.is_running().await);
    }
} 