use config::{Config as ConfigSource, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub redis: RedisConfig,
    pub ethereum: EthereumConfig,
    pub mempool: MempoolConfig,
    pub flashbots: FlashbotsConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub channel: String,
    pub pool_size: usize,
    pub connection_timeout: u64,
    pub read_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub chain_id: u64,
    pub max_block_range: u64,
    pub block_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    pub enabled: bool,
    pub rpc_url: String,
    pub poll_interval: u64,
    pub max_concurrent_requests: usize,
    pub request_timeout: u64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashbotsConfig {
    pub enabled: bool,
    pub rpc_url: String,
    pub poll_interval: u64,
    pub auth_header: String,
    pub max_concurrent_requests: usize,
    pub request_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
        
        info!("Loading configuration from: {}", config_path);

        let config = ConfigSource::builder()
            .add_source(File::from(Path::new(&config_path)).required(false))
            .add_source(File::from(Path::new("config/default.toml")).required(false))
            .add_source(
                Environment::default()
                    .prefix("MEV_RELAY")
                    .separator("_")
                    .ignore_empty(true),
            )
            .build()?;

        let config: Config = config.try_deserialize()?;
        
        info!("Configuration loaded successfully");
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.redis.url.is_empty() {
            return Err("Redis URL cannot be empty".to_string());
        }

        if self.ethereum.rpc_url.is_empty() {
            return Err("Ethereum RPC URL cannot be empty".to_string());
        }

        if self.ethereum.chain_id == 0 {
            return Err("Chain ID must be greater than 0".to_string());
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            redis: RedisConfig::default(),
            ethereum: EthereumConfig::default(),
            mempool: MempoolConfig::default(),
            flashbots: FlashbotsConfig::default(),
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            channel: "mev_swaps".to_string(),
            pool_size: 10,
            connection_timeout: 5000,
            read_timeout: 3000,
        }
    }
}

impl Default for EthereumConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            ws_url: "ws://localhost:8546".to_string(),
            chain_id: 1,
            max_block_range: 1000,
            block_timeout: 30000,
        }
    }
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rpc_url: "http://localhost:8545".to_string(),
            poll_interval: 100,
            max_concurrent_requests: 50,
            request_timeout: 10000,
            batch_size: 100,
        }
    }
}

impl Default for FlashbotsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rpc_url: "https://relay.flashbots.net".to_string(),
            poll_interval: 1000,
            auth_header: "".to_string(),
            max_concurrent_requests: 20,
            request_timeout: 15000,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 9090,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            output: "stdout".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_failure() {
        let mut config = Config::default();
        config.redis.url = "".to_string();
        assert!(config.validate().is_err());
    }
} 