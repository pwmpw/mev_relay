use crate::{
    events::domain::SwapEvent,
    monitoring::service::MonitoringOrchestrator,
    // messaging::publisher::EventPublisherService,  // Temporarily disabled
    infrastructure::{config::Config, health::HealthChecker, shutdown::ShutdownSignal},
};
use crate::Result;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, RwLock},
    time::{interval, Duration},
};
use tracing::{error, info, warn};

/// Main application orchestrator
pub struct MevRelay {
    config: Config,
    shutdown: ShutdownSignal,
    monitoring: MonitoringOrchestrator,
    // publisher: EventPublisherService,  // Temporarily disabled
    health_checker: HealthChecker,
    stats: Arc<RwLock<AppStats>>,
}

/// Application statistics
#[derive(Debug, Default)]
pub struct AppStats {
    pub total_events_processed: u64,
    pub events_by_source: std::collections::HashMap<String, u64>,
    pub events_by_protocol: std::collections::HashMap<String, u64>,
    pub last_event_timestamp: Option<u64>,
    pub uptime_seconds: u64,
    pub errors_count: u64,
    pub monitoring_services_active: u64,
    pub publisher_status: String,
}

impl AppStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_events_processed(&mut self, count: u64) {
        self.total_events_processed += count;
        self.last_event_timestamp = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }

    pub fn increment_source_count(&mut self, source: &str, count: u64) {
        *self.events_by_source.entry(source.to_string()).or_insert(0) += count;
    }

    pub fn increment_protocol_count(&mut self, protocol: &str, count: u64) {
        *self.events_by_protocol.entry(protocol.to_string()).or_insert(0) += count;
    }

    pub fn increment_errors(&mut self) {
        self.errors_count += 1;
    }

    pub fn update_uptime(&mut self, seconds: u64) {
        self.uptime_seconds = seconds;
    }

    pub fn update_monitoring_services(&mut self, count: u64) {
        self.monitoring_services_active = count;
    }

    pub fn update_publisher_status(&mut self, status: String) {
        self.publisher_status = status;
    }
}

impl MevRelay {
    /// Create a new MEV Relay application
    pub async fn new(config: Config, shutdown: ShutdownSignal) -> Result<Self> {
        info!("Initializing MEV Relay application...");

        // Validate configuration
        config.validate()
            .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;

        // Create event channel
        let (event_sender, event_receiver) = mpsc::channel::<SwapEvent>(10000);

        // Initialize monitoring orchestrator
        let monitoring = MonitoringOrchestrator::new(config.clone(), event_sender.clone())?;

        // Initialize event publisher
        // let publisher = EventPublisherService::new(config.clone(), event_receiver).await?;  // Temporarily disabled

        // Initialize health checker
        let health_checker = HealthChecker::new(config.clone())?;

        // Initialize statistics
        let stats = Arc::new(RwLock::new(AppStats::new()));

        info!("MEV Relay application initialized successfully");

        Ok(Self {
            config,
            shutdown,
            monitoring,
            // publisher,  // Temporarily disabled
            health_checker,
            stats,
        })
    }

    /// Run the application
    pub async fn run(&self) -> Result<()> {
        info!("Starting MEV Relay application...");

        // Start monitoring services
        let monitoring_handle = self.start_monitoring_services().await?;

        // Start event publisher
        let publisher_handle = self.start_event_publisher().await?;

        // Start health checker
        let health_handle = self.start_health_checker().await?;

        // Start statistics collection
        let stats_handle = self.start_stats_collection().await?;

        info!("MEV Relay application started successfully");

        // Wait for shutdown signal
        self.shutdown.wait().await;
        info!("Shutdown signal received, stopping MEV Relay...");

        // Stop all services gracefully
        self.stop_services(
            monitoring_handle,
            publisher_handle,
            health_handle,
            stats_handle,
        ).await?;

        info!("MEV Relay application stopped successfully");
        Ok(())
    }

    /// Start monitoring services
    async fn start_monitoring_services(&self) -> Result<tokio::task::JoinHandle<()>> {
        let monitoring = self.monitoring.clone();
        let stats = self.stats.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = monitoring.start_all().await {
                error!("Monitoring services failed: {}", e);
                let mut stats_guard = stats.write().await;
                stats_guard.increment_errors();
            }
        });

        Ok(handle)
    }

    /// Start event publisher
    async fn start_event_publisher(&self) -> Result<tokio::task::JoinHandle<()>> {
        // Publisher is temporarily disabled
        let stats = self.stats.clone();

        let handle = tokio::spawn(async move {
            // Publisher service is disabled for now
            let mut stats_guard = stats.write().await;
            stats_guard.update_publisher_status("disabled".to_string());
        });

        Ok(handle)
    }

    /// Start health checker
    async fn start_health_checker(&self) -> Result<tokio::task::JoinHandle<()>> {
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            // Health checker will be run in the main thread
        });

        Ok(handle)
    }

    /// Start statistics collection
    async fn start_stats_collection(&self) -> Result<tokio::task::JoinHandle<()>> {
        let stats = self.stats.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut stats_guard = stats.write().await;
                        let uptime = stats_guard.uptime_seconds + 60;
                        stats_guard.update_uptime(uptime);
                        
                        info!(
                            "App Stats - Total: {}, Uptime: {}s, Errors: {}, Services: {}",
                            stats_guard.total_events_processed,
                            stats_guard.uptime_seconds,
                            stats_guard.errors_count,
                            stats_guard.monitoring_services_active
                        );
                    }
                    _ = shutdown.wait() => {
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Stop all services gracefully
    async fn stop_services(
        &self,
        monitoring_handle: tokio::task::JoinHandle<()>,
        publisher_handle: tokio::task::JoinHandle<()>,
        health_handle: tokio::task::JoinHandle<()>,
        stats_handle: tokio::task::JoinHandle<()>,
    ) -> Result<()> {
        info!("Stopping all services...");

        // Stop monitoring services
        self.monitoring.stop_all().await?;

        // Cancel all tasks
        monitoring_handle.abort();
        publisher_handle.abort();
        health_handle.abort();
        stats_handle.abort();

        // Wait for tasks to finish
        let _ = monitoring_handle.await;
        let _ = publisher_handle.await;
        let _ = health_handle.await;
        let _ = stats_handle.await;

        info!("All services stopped");
        Ok(())
    }

    /// Get application statistics
    pub async fn get_stats(&self) -> AppStats {
        self.stats.read().await.clone()
    }

    /// Get application health status
    pub async fn get_health_status(&self) -> Result<crate::infrastructure::health::HealthStatus> {
        self.health_checker.get_status().await
    }

    /// Get monitoring services status
    pub async fn get_monitoring_status(&self) -> Result<Vec<crate::monitoring::domain::ServiceStatus>> {
        self.monitoring.get_all_service_statuses().await
    }

    /// Get publisher statistics
    pub async fn get_publisher_stats(&self) -> Result<crate::messaging::domain::PublisherStats> {
        // Publisher is temporarily disabled
        Err(anyhow::anyhow!("Publisher service is disabled"))
    }
}

impl Clone for AppStats {
    fn clone(&self) -> Self {
        Self {
            total_events_processed: self.total_events_processed,
            events_by_source: self.events_by_source.clone(),
            events_by_protocol: self.events_by_protocol.clone(),
            last_event_timestamp: self.last_event_timestamp,
            uptime_seconds: self.uptime_seconds,
            errors_count: self.errors_count,
            monitoring_services_active: self.monitoring_services_active,
            publisher_status: self.publisher_status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shutdown::ShutdownSignal;

    #[tokio::test]
    async fn test_mev_relay_creation() {
        let config = Config::default();
        let shutdown = ShutdownSignal::new();
        
        let relay = MevRelay::new(config, shutdown).await;
        assert!(relay.is_ok());
    }

    #[test]
    fn test_app_stats() {
        let mut stats = AppStats::new();
        
        assert_eq!(stats.total_events_processed, 0);
        assert_eq!(stats.errors_count, 0);
        
        stats.increment_events_processed(5);
        stats.increment_source_count("Mempool", 3);
        stats.increment_protocol_count("Uniswap V2", 2);
        stats.increment_errors();
        
        assert_eq!(stats.total_events_processed, 5);
        assert_eq!(stats.events_by_source["Mempool"], 3);
        assert_eq!(stats.events_by_protocol["Uniswap V2"], 2);
        assert_eq!(stats.errors_count, 1);
    }
} 