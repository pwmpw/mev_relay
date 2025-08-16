use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    monitoring::{domain::MonitoringService, mempool::MempoolMonitor, flashbots::FlashbotsMonitor},
    shared::Result,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Orchestrates all monitoring services
pub struct MonitoringOrchestrator {
    config: Config,
    event_sender: mpsc::Sender<SwapEvent>,
    services: Vec<Box<dyn MonitoringService>>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl MonitoringOrchestrator {
    pub fn new(config: Config, event_sender: mpsc::Sender<SwapEvent>) -> Result<Self> {
        let mut orchestrator = Self {
            config,
            event_sender,
            services: Vec::new(),
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        };

        orchestrator.initialize_services()?;

        Ok(orchestrator)
    }

    fn initialize_services(&mut self) -> Result<()> {
        // Initialize mempool monitoring
        if self.config.mempool.enabled {
            let mempool_monitor = MempoolMonitor::new(
                self.config.clone(),
                self.event_sender.clone(),
            )?;
            self.services.push(Box::new(mempool_monitor));
            info!("Mempool monitoring service initialized");
        }

        // Initialize Flashbots monitoring
        if self.config.flashbots.enabled {
            let flashbots_monitor = FlashbotsMonitor::new(
                self.config.clone(),
                self.event_sender.clone(),
            )?;
            self.services.push(Box::new(flashbots_monitor));
            info!("Flashbots monitoring service initialized");
        }

        info!("Initialized {} monitoring services", self.services.len());
        Ok(())
    }

    /// Start all monitoring services
    pub async fn start_all(&self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if *running_guard {
            warn!("Monitoring services are already running");
            return Ok(());
        }

        info!("Starting {} monitoring services", self.services.len());

        for service in &self.services {
            if let Err(e) = service.start().await {
                error!("Failed to start monitoring service: {}", e);
                return Err(anyhow::anyhow!("Service startup failed: {}", e));
            }
        }

        *running_guard = true;
        info!("All monitoring services started successfully");
        Ok(())
    }

    /// Stop all monitoring services
    pub async fn stop_all(&self) -> Result<()> {
        let mut running_guard = self.is_running.write().await;
        if !*running_guard {
            warn!("Monitoring services are not running");
            return Ok(());
        }

        info!("Stopping {} monitoring services", self.services.len());

        for service in &self.services {
            if let Err(e) = service.stop().await {
                error!("Failed to stop monitoring service: {}", e);
                // Continue stopping other services
            }
        }

        *running_guard = false;
        info!("All monitoring services stopped");
        Ok(())
    }

    /// Check if any monitoring services are active
    pub async fn has_active_services(&self) -> bool {
        let running_guard = self.is_running.read().await;
        *running_guard
    }

    /// Get status of all monitoring services
    pub async fn get_all_service_statuses(&self) -> Result<Vec<crate::monitoring::domain::ServiceStatus>> {
        let mut statuses = Vec::new();
        
        for service in &self.services {
            statuses.push(service.get_status());
        }

        Ok(statuses)
    }

    /// Get the number of active monitoring services
    pub fn get_service_count(&self) -> usize {
        self.services.len()
    }

    /// Check if a specific service type is enabled
    pub fn is_service_enabled(&self, service_name: &str) -> bool {
        match service_name {
            "mempool" => self.config.mempool.enabled,
            "flashbots" => self.config.flashbots.enabled,
            _ => false,
        }
    }

    /// Get monitoring configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

impl Clone for MonitoringOrchestrator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            event_sender: self.event_sender.clone(),
            services: self.services.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_monitoring_orchestrator_creation() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let orchestrator = MonitoringOrchestrator::new(config, sender);
        assert!(orchestrator.is_ok());
    }

    #[test]
    fn test_service_count() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let orchestrator = MonitoringOrchestrator::new(config, sender).unwrap();
        assert_eq!(orchestrator.get_service_count(), 2); // mempool + flashbots
    }

    #[test]
    fn test_service_enabled() {
        let config = Config::default();
        let (sender, _receiver) = mpsc::channel::<SwapEvent>(100);
        
        let orchestrator = MonitoringOrchestrator::new(config, sender).unwrap();
        assert!(orchestrator.is_service_enabled("mempool"));
        assert!(orchestrator.is_service_enabled("flashbots"));
        assert!(!orchestrator.is_service_enabled("unknown"));
    }
} 