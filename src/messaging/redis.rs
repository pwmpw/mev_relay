use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    messaging::domain::{MessageBroker, MessageReceiver, BrokerStats, ConnectionStatus},
    shared::Result,
};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};
use serde_json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Redis implementation of the message broker
pub struct RedisPublisher {
    config: Config,
    connection_manager: ConnectionManager,
    stats: Arc<RwLock<BrokerStats>>,
}

/// Redis message receiver
pub struct RedisMessageReceiver {
    connection_manager: ConnectionManager,
    channel: String,
    is_active: Arc<RwLock<bool>>,
}

impl RedisPublisher {
    pub fn new(config: Config) -> Result<Self> {
        let redis_client = RedisClient::open(config.redis.url.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;

        let connection_manager = redis::aio::ConnectionManager::new(redis_client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Redis connection manager: {}", e))?;

        let stats = Arc::new(RwLock::new(BrokerStats::new()));

        Ok(Self {
            config,
            connection_manager,
            stats,
        })
    }

    /// Publish a single swap event
    pub async fn publish_event(&mut self, event: &SwapEvent) -> Result<()> {
        let start_time = std::time::Instant::now();

        let event_json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;

        let result: Result<(), redis::RedisError> = self.connection_manager
            .publish(&self.config.redis.channel, event_json)
            .await;

        match result {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis() as f64 / 1000.0;
                self.update_stats_success(duration).await;
                info!("Event published to Redis channel: {}", self.config.redis.channel);
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                error!("Failed to publish event to Redis: {}", e);
                Err(anyhow::anyhow!("Redis publish failed: {}", e))
            }
        }
    }

    /// Publish multiple events in a batch
    pub async fn publish_events(&mut self, events: &[SwapEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let start_time = std::time::Instant::now();
        let mut pipeline = redis::pipe();

        for event in events {
            let event_json = serde_json::to_string(event)
                .map_err(|e| anyhow::anyhow!("Failed to serialize event: {}", e))?;
            pipeline.publish(&self.config.redis.channel, event_json);
        }

        let result: Result<Vec<()>, redis::RedisError> = pipeline
            .query_async(&mut self.connection_manager)
            .await;

        match result {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis() as f64 / 1000.0;
                self.update_stats_batch_success(events.len(), duration).await;
                info!("Published {} events to Redis channel: {}", events.len(), self.config.redis.channel);
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                error!("Failed to publish batch to Redis: {}", e);
                Err(anyhow::anyhow!("Redis batch publish failed: {}", e))
            }
        }
    }

    /// Check Redis connection health
    pub async fn health_check(&mut self) -> Result<bool> {
        let result: Result<String, redis::RedisError> = self.connection_manager
            .ping()
            .await;

        match result {
            Ok(_) => {
                self.update_stats_health(true).await;
                Ok(true)
            }
            Err(e) => {
                self.update_stats_health(false).await;
                error!("Redis health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get Redis statistics
    pub async fn get_stats(&self) -> Result<BrokerStats> {
        let stats_guard = self.stats.read().await;
        Ok(stats_guard.clone())
    }

    /// Update statistics for successful publish
    async fn update_stats_success(&self, duration: f64) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_published();
        stats_guard.update_performance_metric("last_publish_duration".to_string(), duration);
        stats_guard.set_connection_status(ConnectionStatus::Connected);
    }

    /// Update statistics for batch publish
    async fn update_stats_batch_success(&self, count: usize, duration: f64) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_published();
        stats_guard.update_performance_metric("last_batch_publish_duration".to_string(), duration);
        stats_guard.update_performance_metric("last_batch_size".to_string(), count as f64);
        stats_guard.set_connection_status(ConnectionStatus::Connected);
    }

    /// Update statistics for error
    async fn update_stats_error(&self) {
        let mut stats_guard = self.stats.write().await;
        stats_guard.increment_error_count();
        stats_guard.set_connection_status(ConnectionStatus::Error("Publish failed".to_string()));
    }

    /// Update health check statistics
    async fn update_stats_health(&self, is_healthy: bool) {
        let mut stats_guard = self.stats.write().await;
        if is_healthy {
            stats_guard.set_connection_status(ConnectionStatus::Connected);
        } else {
            stats_guard.set_connection_status(ConnectionStatus::Error("Health check failed".to_string()));
        }
    }

    /// Get Redis configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

impl Clone for RedisPublisher {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connection_manager: self.connection_manager.clone(),
            stats: self.stats.clone(),
        }
    }
}

#[async_trait]
impl MessageBroker for RedisPublisher {
    async fn publish(&mut self, channel: &str, message: &str) -> Result<()> {
        let result: Result<(), redis::RedisError> = self.connection_manager
            .publish(channel, message)
            .await;

        match result {
            Ok(_) => {
                self.update_stats_success(0.0).await;
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                Err(anyhow::anyhow!("Redis publish failed: {}", e))
            }
        }
    }

    async fn publish_batch(&mut self, channel: &str, messages: &[String]) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let mut pipeline = redis::pipe();
        for message in messages {
            pipeline.publish(channel, message);
        }

        let result: Result<Vec<()>, redis::RedisError> = pipeline
            .query_async(&mut self.connection_manager)
            .await;

        match result {
            Ok(_) => {
                self.update_stats_batch_success(messages.len(), 0.0).await;
                Ok(())
            }
            Err(e) => {
                self.update_stats_error().await;
                Err(anyhow::anyhow!("Redis batch publish failed: {}", e))
            }
        }
    }

    async fn subscribe(&mut self, channel: &str) -> Result<Box<dyn MessageReceiver>> {
        let receiver = RedisMessageReceiver::new(
            self.connection_manager.clone(),
            channel.to_string(),
        );

        Ok(Box::new(receiver))
    }

    async fn health_check(&mut self) -> Result<bool> {
        self.health_check().await
    }

    async fn get_stats(&mut self) -> Result<BrokerStats> {
        self.get_stats().await
    }
}

impl RedisMessageReceiver {
    pub fn new(connection_manager: ConnectionManager, channel: String) -> Self {
        Self {
            connection_manager,
            channel,
            is_active: Arc::new(RwLock::new(true)),
        }
    }
}

#[async_trait]
impl MessageReceiver for RedisMessageReceiver {
    async fn receive(&mut self) -> Result<Option<String>> {
        // This is a simplified implementation
        // In production, you'd implement proper Redis pub/sub subscription
        let is_active = self.is_active.read().await;
        if !*is_active {
            return Ok(None);
        }

        // For now, return None as this is a placeholder
        // In production, you'd implement actual message receiving logic
        Ok(None)
    }

    fn is_active(&self) -> bool {
        // This would need to be implemented with proper async handling
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_publisher_creation() {
        let config = Config::default();
        let publisher = RedisPublisher::new(config);
        // This will fail in tests without Redis, but we can test the structure
        assert!(publisher.is_err()); // Expected to fail without Redis
    }

    #[test]
    fn test_config_access() {
        let config = Config::default();
        // Test that we can access config even if Redis connection fails
        assert_eq!(config.redis.channel, "mev_swaps");
    }
} 