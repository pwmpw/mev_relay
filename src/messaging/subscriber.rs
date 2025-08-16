use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    messaging::domain::{EventSubscriber, EventReceiver, SubscriberStats},
    shared::Result,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Event subscriber service for consuming swap events
pub struct EventSubscriberService {
    config: Config,
    stats: Arc<RwLock<SubscriberStats>>,
}

/// Event receiver implementation
pub struct EventReceiverImpl {
    config: Config,
    stats: Arc<RwLock<SubscriberStats>>,
    is_active: Arc<RwLock<bool>>,
}

impl EventSubscriberService {
    pub fn new(config: Config) -> Result<Self> {
        let stats = Arc::new(RwLock::new(SubscriberStats::new()));

        Ok(Self {
            config,
            stats,
        })
    }

    /// Get subscriber statistics
    pub async fn get_stats(&self) -> Result<SubscriberStats> {
        let stats_guard = self.stats.read().await;
        Ok(stats_guard.clone())
    }

    /// Get service configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

impl Clone for EventSubscriberService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}

#[async_trait::async_trait]
impl EventSubscriber for EventSubscriberService {
    async fn subscribe(&mut self) -> Result<Box<dyn EventReceiver>> {
        let receiver = EventReceiverImpl::new(
            self.config.clone(),
            self.stats.clone(),
        );

        info!("Event subscriber created for channel: {}", self.config.redis.channel);
        Ok(Box::new(receiver))
    }

    async fn get_stats(&mut self) -> Result<SubscriberStats> {
        self.get_stats().await
    }
}

impl EventReceiverImpl {
    pub fn new(config: Config, stats: Arc<RwLock<SubscriberStats>>) -> Self {
        Self {
            config,
            stats,
            is_active: Arc::new(RwLock::new(true)),
        }
    }

    /// Update statistics for received event
    async fn update_stats_success(&self, event: &SwapEvent, duration: f64) {
        let mut stats_guard = self.stats.write().await;
        
        stats_guard.increment_events_received(1);
        stats_guard.increment_source_count(&event.source.to_string());
        stats_guard.increment_protocol_count(&event.protocol.name);
        stats_guard.update_average_processing_latency(duration);
    }

    /// Update statistics for error
    async fn update_stats_error(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_error_count();
    }
}

#[async_trait::async_trait]
impl EventReceiver for EventReceiverImpl {
    async fn receive_event(&mut self) -> Result<Option<SwapEvent>> {
        let is_active = self.is_active.read().await;
        if !*is_active {
            return Ok(None);
        }

        // This is a placeholder implementation
        // In production, you'd implement actual event receiving logic
        // from Redis, Kafka, or other message brokers
        
        // For now, return None to indicate no events available
        Ok(None)
    }

    fn is_active(&self) -> bool {
        // This would need to be implemented with proper async handling
        let is_active = self.is_active.try_read();
        match is_active {
            Ok(guard) => *guard,
            Err(_) => false,
        }
    }
}

impl Clone for EventReceiverImpl {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
            is_active: self.is_active.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_subscriber_creation() {
        let config = Config::default();
        let subscriber = EventSubscriberService::new(config);
        assert!(subscriber.is_ok());
    }

    #[test]
    fn test_subscriber_config() {
        let config = Config::default();
        let subscriber = EventSubscriberService::new(config).unwrap();
        assert_eq!(subscriber.get_config().redis.channel, "mev_swaps");
    }

    #[tokio::test]
    async fn test_subscriber_stats() {
        let config = Config::default();
        let subscriber = EventSubscriberService::new(config).unwrap();
        let stats = subscriber.get_stats().await.unwrap();
        assert_eq!(stats.total_events_received, 0);
    }
} 