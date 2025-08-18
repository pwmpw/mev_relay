use crate::infrastructure::config::Config;
use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: ServiceStatus,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub version: String,
    pub checks: HashMap<String, CheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub message: String,
    pub timestamp: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check(&self) -> CheckResult;
    fn name(&self) -> &str;
}

pub struct HealthChecker {
    config: Config,
    checks: Vec<Box<dyn HealthCheck>>,
    status: Arc<RwLock<HealthStatus>>,
    start_time: std::time::Instant,
}

impl HealthChecker {
    pub fn new(config: Config) -> Result<Self> {
        let mut checker = Self {
            config,
            checks: Vec::new(),
            status: Arc::new(RwLock::new(HealthStatus {
                status: ServiceStatus::Healthy,
                timestamp: crate::shared::utils::time::now(),
                uptime_seconds: 0,
                version: env!("CARGO_PKG_VERSION").to_string(),
                checks: HashMap::new(),
            })),
            start_time: std::time::Instant::now(),
        };

        // Register health checks
        checker.register_checks()?;

        Ok(checker)
    }

    fn register_checks(&mut self) -> Result<()> {
        // Redis health check
        self.checks.push(Box::new(RedisHealthCheck::new(self.config.clone())));
        
        // Ethereum RPC health check
        self.checks.push(Box::new(EthereumHealthCheck::new(self.config.clone())));
        
        // System health check
        self.checks.push(Box::new(SystemHealthCheck::new()));

        info!("Registered {} health checks", self.checks.len());
        Ok(())
    }

    pub async fn run(&self, shutdown: crate::infrastructure::shutdown::ShutdownSignal) {
        let mut interval = interval(Duration::from_secs(30));
        let status = self.status.clone();

        info!("Health checker started");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.perform_health_checks().await;
                }
                _ = shutdown.wait() => {
                    info!("Health checker shutdown signal received");
                    break;
                }
            }
        }

        info!("Health checker stopped");
    }

    async fn perform_health_checks(&self) {
        let start_time = std::time::Instant::now();
        let mut all_checks = HashMap::new();
        let mut overall_status = ServiceStatus::Healthy;

        for check in &self.checks {
            let check_start = std::time::Instant::now();
            let result = check.check().await;
            let duration = check_start.elapsed().as_millis() as u64;

            let check_result = CheckResult {
                status: result.status.clone(),
                message: result.message,
                timestamp: crate::shared::utils::time::now(),
                duration_ms: duration,
            };

            all_checks.insert(check.name().to_string(), check_result.clone());

            // Update overall status
            match result.status {
                CheckStatus::Fail => {
                    overall_status = ServiceStatus::Unhealthy;
                }
                CheckStatus::Warn => {
                    if overall_status == ServiceStatus::Healthy {
                        overall_status = ServiceStatus::Degraded;
                    }
                }
                CheckStatus::Pass => {}
            }
        }

        let uptime = self.start_time.elapsed().as_secs();
        
        let health_status = HealthStatus {
            status: overall_status.clone(),
            timestamp: crate::shared::utils::time::now(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: all_checks,
        };

        {
            let mut status_guard = self.status.write().await;
            *status_guard = health_status;
        }

        let total_duration = start_time.elapsed().as_millis();
        info!(
            "Health check completed in {}ms - Status: {:?}",
            total_duration, overall_status
        );
    }

    pub async fn get_status(&self) -> Result<HealthStatus> {
        let status = self.status.read().await;
        Ok(status.clone())
    }

    pub async fn force_check(&self) -> Result<HealthStatus> {
        self.perform_health_checks().await;
        self.get_status().await
    }
}

// Redis Health Check
struct RedisHealthCheck {
    config: Config,
}

impl RedisHealthCheck {
    fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl HealthCheck for RedisHealthCheck {
    fn name(&self) -> &str {
        "redis"
    }

    async fn check(&self) -> CheckResult {
        let start_time = std::time::Instant::now();
        
        match redis::Client::open(self.config.redis.url.clone()) {
            Ok(client) => {
                match client.get_async_connection().await {
                    Ok(mut conn) => {
                        match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                            Ok(_) => {
                                let duration = start_time.elapsed().as_millis() as u64;
                                CheckResult {
                                    status: CheckStatus::Pass,
                                    message: "Redis connection successful".to_string(),
                                    timestamp: crate::shared::utils::time::now(),
                                    duration_ms: duration,
                                }
                            }
                            Err(e) => {
                                let duration = start_time.elapsed().as_millis() as u64;
                                CheckResult {
                                    status: CheckStatus::Fail,
                                    message: format!("Redis ping failed: {}", e),
                                    timestamp: crate::shared::utils::time::now(),
                                    duration_ms: duration,
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let duration = start_time.elapsed().as_millis() as u64;
                        CheckResult {
                            status: CheckStatus::Fail,
                            message: format!("Redis connection failed: {}", e),
                            timestamp: crate::shared::utils::time::now(),
                            duration_ms: duration,
                        }
                    }
                }
            }
            Err(e) => {
                let duration = start_time.elapsed().as_millis() as u64;
                CheckResult {
                    status: CheckStatus::Fail,
                    message: format!("Redis client creation failed: {}", e),
                    timestamp: crate::shared::utils::time::now(),
                    duration_ms: duration,
                }
            }
        }
    }
}

// Ethereum Health Check
struct EthereumHealthCheck {
    config: Config,
}

impl EthereumHealthCheck {
    fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl HealthCheck for EthereumHealthCheck {
    fn name(&self) -> &str {
        "ethereum"
    }

    async fn check(&self) -> CheckResult {
        let start_time = std::time::Instant::now();
        
        match reqwest::Client::new()
            .post(&self.config.ethereum.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let duration = start_time.elapsed().as_millis() as u64;
                    CheckResult {
                        status: CheckStatus::Pass,
                        message: "Ethereum RPC connection successful".to_string(),
                        timestamp: crate::shared::utils::time::now(),
                        duration_ms: duration,
                    }
                } else {
                    let duration = start_time.elapsed().as_millis() as u64;
                    CheckResult {
                        status: CheckStatus::Fail,
                        message: format!("Ethereum RPC returned status: {}", response.status()),
                        timestamp: crate::shared::utils::time::now(),
                        duration_ms: duration,
                    }
                }
            }
            Err(e) => {
                let duration = start_time.elapsed().as_millis() as u64;
                CheckResult {
                    status: CheckStatus::Fail,
                    message: format!("Ethereum RPC request failed: {}", e),
                    timestamp: crate::shared::utils::time::now(),
                    duration_ms: duration,
                }
            }
        }
    }
}

// System Health Check
struct SystemHealthCheck;

impl SystemHealthCheck {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthCheck for SystemHealthCheck {
    fn name(&self) -> &str {
        "system"
    }

    async fn check(&self) -> CheckResult {
        let start_time = std::time::Instant::now();
        
        // Check memory usage
        let memory_info = sysinfo::System::new_all();
        let memory_usage_percent = (memory_info.total_memory() - memory_info.free_memory()) as f64 / memory_info.total_memory() as f64 * 100.0;
        
        let status = if memory_usage_percent > 90.0 {
            CheckStatus::Fail
        } else if memory_usage_percent > 80.0 {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        };

        let message = format!("Memory usage: {:.1}%", memory_usage_percent);
        let duration = start_time.elapsed().as_millis() as u64;

        CheckResult {
            status,
            message,
            timestamp: crate::shared::utils::time::now(),
            duration_ms: duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_creation() {
        let status = HealthStatus {
            status: ServiceStatus::Healthy,
            timestamp: 1234567890,
            uptime_seconds: 100,
            version: "1.0.0".to_string(),
            checks: HashMap::new(),
        };

        assert_eq!(status.status, ServiceStatus::Healthy);
        assert_eq!(status.uptime_seconds, 100);
    }

    #[test]
    fn test_check_result_creation() {
        let result = CheckResult {
            status: CheckStatus::Pass,
            message: "Test check".to_string(),
            timestamp: 1234567890,
            duration_ms: 50,
        };

        assert_eq!(result.status, CheckStatus::Pass);
        assert_eq!(result.message, "Test check");
        assert_eq!(result.duration_ms, 50);
    }
} 