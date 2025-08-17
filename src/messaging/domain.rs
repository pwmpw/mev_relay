use crate::events::domain::SwapEvent;
use async_trait::async_trait;
use std::collections::HashMap;

/// Message broker trait for different messaging systems
#[async_trait]
pub trait MessageBroker: Send + Sync {
    /// Publish a message to a channel
    async fn publish(&mut self, channel: &str, message: &str) -> crate::Result<()>;
    
    /// Publish multiple messages to a channel
    async fn publish_batch(&mut self, channel: &str, messages: &[String]) -> crate::Result<()>;
    
    /// Subscribe to a channel
    async fn subscribe(&mut self, channel: &str) -> crate::Result<Box<dyn MessageReceiver>>;
    
    /// Check connection health
    async fn health_check(&mut self) -> crate::Result<bool>;
    
    /// Get broker statistics
    async fn get_stats(&mut self) -> crate::Result<BrokerStats>;
}

/// Message receiver for consuming messages
#[async_trait]
pub trait MessageReceiver: Send + Sync {
    /// Receive a message
    async fn receive(&mut self) -> crate::Result<Option<String>>;
    
    /// Check if receiver is still active
    fn is_active(&self) -> bool;
}

/// Message broker statistics
#[derive(Debug, Clone, Default)]
pub struct BrokerStats {
    pub total_messages_published: u64,
    pub total_messages_received: u64,
    pub active_channels: u64,
    pub connection_status: ConnectionStatus,
    pub last_activity: Option<u64>,
    pub error_count: u64,
    pub performance_metrics: HashMap<String, f64>,
}

/// Connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

impl BrokerStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_published(&mut self) {
        self.total_messages_published += 1;
        self.update_last_activity();
    }

    pub fn increment_received(&mut self) {
        self.total_messages_received += 1;
        self.update_last_activity();
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
        self.update_last_activity();
    }

    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
    }

    pub fn update_performance_metric(&mut self, key: String, value: f64) {
        self.performance_metrics.insert(key, value);
    }

    fn update_last_activity(&mut self) {
        self.last_activity = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }
}

/// Event publisher trait for publishing swap events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single swap event
    async fn publish_event(&mut self, event: &SwapEvent) -> crate::Result<()>;
    
    /// Publish multiple swap events
    async fn publish_events(&mut self, events: &[SwapEvent]) -> crate::Result<()>;
    
    /// Get publisher statistics
    async fn get_stats(&mut self) -> crate::Result<PublisherStats>;
}

/// Event subscriber trait for consuming swap events
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Subscribe to swap events
    async fn subscribe(&mut self) -> crate::Result<Box<dyn EventReceiver>>;
    
    /// Get subscriber statistics
    async fn get_stats(&mut self) -> crate::Result<SubscriberStats>;
}

/// Event receiver for consuming swap events
#[async_trait]
pub trait EventReceiver: Send + Sync {
    /// Receive a swap event
    async fn receive_event(&mut self) -> crate::Result<Option<SwapEvent>>;
    
    /// Check if receiver is still active
    fn is_active(&self) -> bool;
}

/// Publisher statistics
#[derive(Debug, Clone, Default)]
pub struct PublisherStats {
    pub total_events_published: u64,
    pub events_by_source: HashMap<String, u64>,
    pub events_by_protocol: HashMap<String, u64>,
    pub last_published_at: Option<u64>,
    pub average_publish_latency: f64,
    pub error_count: u64,
}

impl PublisherStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_events_published(&mut self, count: u64) {
        self.total_events_published += count;
        self.last_published_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }

    pub fn increment_source_count(&mut self, source: &str) {
        *self.events_by_source.entry(source.to_string()).or_insert(0) += 1;
    }

    pub fn increment_protocol_count(&mut self, protocol: &str) {
        *self.events_by_protocol.entry(protocol.to_string()).or_insert(0) += 1;
    }

    pub fn update_average_latency(&mut self, latency: f64) {
        if self.total_events_published > 0 {
            let current_avg = self.average_publish_latency;
            let total = self.total_events_published as f64;
            self.average_publish_latency = (current_avg * (total - 1.0) + latency) / total;
        } else {
            self.average_publish_latency = latency;
        }
    }

    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
    }
}

/// Subscriber statistics
#[derive(Debug, Clone, Default)]
pub struct SubscriberStats {
    pub total_events_received: u64,
    pub events_by_source: HashMap<String, u64>,
    pub events_by_protocol: HashMap<String, u64>,
    pub last_received_at: Option<u64>,
    pub average_processing_latency: f64,
    pub error_count: u64,
}

impl SubscriberStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_events_received(&mut self, count: u64) {
        self.total_events_received += count;
        self.last_received_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }

    pub fn increment_source_count(&mut self, source: &str) {
        *self.events_by_source.entry(source.to_string()).or_insert(0) += 1;
    }

    pub fn increment_protocol_count(&mut self, protocol: &str) {
        *self.events_by_protocol.entry(protocol.to_string()).or_insert(0) += 1;
    }

    pub fn update_average_processing_latency(&mut self, latency: f64) {
        if self.total_events_received > 0 {
            let current_avg = self.average_processing_latency;
            let total = self.total_events_received as f64;
            self.average_processing_latency = (current_avg * (total - 1.0) + latency) / total;
        } else {
            self.average_processing_latency = latency;
        }
    }

    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
    }
}

/// Message channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub name: String,
    pub max_message_size: usize,
    pub retention_policy: RetentionPolicy,
    pub compression_enabled: bool,
}

/// Message retention policy
#[derive(Debug, Clone)]
pub enum RetentionPolicy {
    KeepAll,
    KeepLast(usize),
    KeepForDuration(std::time::Duration),
    DeleteAfterRead,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            name: "mev_swaps".to_string(),
            max_message_size: 1024 * 1024, // 1MB
            retention_policy: RetentionPolicy::KeepLast(1000),
            compression_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::domain::{SwapEvent, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo};

    #[test]
    fn test_broker_stats() {
        let mut stats = BrokerStats::new();
        
        assert_eq!(stats.total_messages_published, 0);
        assert_eq!(stats.connection_status, ConnectionStatus::Disconnected);
        
        stats.increment_published();
        stats.set_connection_status(ConnectionStatus::Connected);
        
        assert_eq!(stats.total_messages_published, 1);
        assert_eq!(stats.connection_status, ConnectionStatus::Connected);
        assert!(stats.last_activity.is_some());
    }

    #[test]
    fn test_publisher_stats() {
        let mut stats = PublisherStats::new();
        
        assert_eq!(stats.total_events_published, 0);
        
        stats.increment_events_published(5);
        stats.increment_source_count("Mempool");
        stats.increment_protocol_count("Uniswap V2");
        
        assert_eq!(stats.total_events_published, 5);
        assert_eq!(stats.events_by_source["Mempool"], 1);
        assert_eq!(stats.events_by_protocol["Uniswap V2"], 1);
    }

    #[test]
    fn test_subscriber_stats() {
        let mut stats = SubscriberStats::new();
        
        assert_eq!(stats.total_events_received, 0);
        
        stats.increment_events_received(3);
        stats.increment_source_count("Flashbots");
        
        assert_eq!(stats.total_events_received, 3);
        assert_eq!(stats.events_by_source["Flashbots"], 1);
    }

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::default();
        
        assert_eq!(config.name, "mev_swaps");
        assert_eq!(config.max_message_size, 1024 * 1024);
        assert!(matches!(config.retention_policy, RetentionPolicy::KeepLast(1000)));
        assert!(!config.compression_enabled);
    }
} 