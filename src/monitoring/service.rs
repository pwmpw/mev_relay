use crate::{
    events::domain::SwapEvent,
    infrastructure::config::Config,
    monitoring::{domain::MonitoringService, mempool::MempoolMonitor, flashbots::FlashbotsMonitor, subgraph::SubgraphServiceImpl},
};
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Orchestrates all monitoring services
pub struct MonitoringOrchestrator {
    config: Config,
    event_sender: mpsc::Sender<SwapEvent>,
    services: Vec<Arc<tokio::sync::Mutex<Box<dyn MonitoringService>>>>,
    subgraph_service: Option<Arc<tokio::sync::Mutex<SubgraphServiceImpl>>>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl MonitoringOrchestrator {
    pub fn new(config: Config, event_sender: mpsc::Sender<SwapEvent>) -> Result<Self> {
        let mut orchestrator = Self {
            config,
            event_sender,
            services: Vec::new(),
            subgraph_service: None,
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
            self.services.push(Arc::new(tokio::sync::Mutex::new(Box::new(mempool_monitor))));
            info!("Mempool monitoring service initialized");
        }

        // Initialize Flashbots monitoring
        if self.config.flashbots.enabled {
            let flashbots_monitor = FlashbotsMonitor::new(
                self.config.clone(),
                self.event_sender.clone(),
            )?;
            self.services.push(Arc::new(tokio::sync::Mutex::new(Box::new(flashbots_monitor))));
            info!("Flashbots monitoring service initialized");
        }

        // Initialize subgraph service
        if self.config.subgraph.enabled {
            let subgraph_service = SubgraphServiceImpl::new(self.config.subgraph.clone());
            self.subgraph_service = Some(Arc::new(tokio::sync::Mutex::new(subgraph_service)));
            info!("Subgraph service initialized");
        }

        info!("Initialized {} monitoring services", self.services.len());
        Ok(())
    }

    /// Get subgraph service reference
    pub fn get_subgraph_service(&self) -> Option<Arc<tokio::sync::Mutex<SubgraphServiceImpl>>> {
        self.subgraph_service.clone()
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
            let mut service_guard = service.lock().await;
            if let Err(e) = service_guard.start().await {
                error!("Failed to start monitoring service: {}", e);
                return Err(anyhow::anyhow!("Service startup failed: {}", e));
            }
        }

        // Start subgraph service if enabled
        if let Some(subgraph_service) = &self.subgraph_service {
            let mut service_guard = subgraph_service.lock().await;
            if let Err(e) = service_guard.start().await {
                error!("Failed to start subgraph service: {}", e);
                return Err(anyhow::anyhow!("Subgraph service startup failed: {}", e));
            }
            info!("Subgraph service started successfully");
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
            let mut service_guard = service.lock().await;
            if let Err(e) = service_guard.stop().await {
                error!("Failed to stop monitoring service: {}", e);
                // Continue stopping other services
            }
        }

        // Stop subgraph service if enabled
        if let Some(subgraph_service) = &self.subgraph_service {
            let mut service_guard = subgraph_service.lock().await;
            if let Err(e) = service_guard.stop().await {
                error!("Failed to stop subgraph service: {}", e);
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
            let service_guard = service.lock().await;
            statuses.push(service_guard.get_status());
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
            services: Vec::new(), // Cannot clone Box<dyn Trait>
            subgraph_service: self.subgraph_service.clone(),
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