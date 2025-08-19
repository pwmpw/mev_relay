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
    pub filtering: FilteringConfig,
    pub subgraph: SubgraphConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteringConfig {
    pub enabled: bool,
    pub pool_addresses: Vec<String>,
    pub token_addresses: Vec<String>,
    pub min_liquidity_eth: f64,
    pub min_volume_24h_eth: f64,
    pub exclude_contracts: Vec<String>,
    pub include_protocols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphConfig {
    pub enabled: bool,
    pub url: String,
    pub poll_interval: u64,
    pub cache_ttl_seconds: u64,
    pub max_concurrent_requests: usize,
    pub request_timeout: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
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
            filtering: FilteringConfig::default(),
            subgraph: SubgraphConfig::default(),
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

impl Default for FilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_addresses: vec![
                // Popular Uniswap V2 pools
                "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc7".to_string(), // USDC/ETH
                "0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string(), // WETH/USDT
                // Popular Uniswap V3 pools
                "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".to_string(), // USDC/ETH 0.3%
                "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(), // USDC/ETH 0.05%
            ],
            token_addresses: vec![
                // Major tokens
                "0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string(), // WETH
                "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
                "0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string(), // USDC
                "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
            ],
            min_liquidity_eth: 100.0, // Minimum 100 ETH liquidity
            min_volume_24h_eth: 1000.0, // Minimum 1000 ETH 24h volume
            exclude_contracts: vec![
                // Exclude known MEV bots and contracts
                "0x0000000000000000000000000000000000000000".to_string(),
            ],
            include_protocols: vec![
                "Uniswap V2".to_string(),
                "Uniswap V3".to_string(),
                "SushiSwap".to_string(),
            ],
        }
    }
}

impl Default for SubgraphConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: "http://localhost:8000".to_string(),
            poll_interval: 60,
            cache_ttl_seconds: 300,
            max_concurrent_requests: 10,
            request_timeout: 10000,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.channel, "mev_swaps");
        assert_eq!(config.pool_size, 10);
        assert_eq!(config.connection_timeout, 5000);
        assert_eq!(config.read_timeout, 3000);
    }

    #[test]
    fn test_ethereum_config_default() {
        let config = EthereumConfig::default();
        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(config.ws_url, "ws://localhost:8546");
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.max_block_range, 1000);
        assert_eq!(config.block_timeout, 30000);
    }

    #[test]
    fn test_mempool_config_default() {
        let config = MempoolConfig::default();
        assert!(config.enabled);
        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(config.poll_interval, 100);
        assert_eq!(config.max_concurrent_requests, 50);
        assert_eq!(config.request_timeout, 10000);
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_flashbots_config_default() {
        let config = FlashbotsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.rpc_url, "https://relay.flashbots.net");
        assert_eq!(config.poll_interval, 1000);
        assert_eq!(config.auth_header, "");
        assert_eq!(config.max_concurrent_requests, 20);
        assert_eq!(config.request_timeout, 15000);
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9090);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "json");
        assert_eq!(config.output, "stdout");
    }

    #[test]
    fn test_filtering_config_default() {
        let config = FilteringConfig::default();
        assert!(config.enabled);
        assert!(!config.pool_addresses.is_empty());
        assert!(!config.token_addresses.is_empty());
        assert_eq!(config.min_liquidity_eth, 100.0);
        assert_eq!(config.min_volume_24h_eth, 1000.0);
        assert!(!config.exclude_contracts.is_empty());
        assert!(!config.include_protocols.is_empty());
        
        // Check specific default values
        assert!(config.pool_addresses.contains(&"0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc7".to_string()));
        assert!(config.include_protocols.contains(&"Uniswap V2".to_string()));
        assert!(config.include_protocols.contains(&"Uniswap V3".to_string()));
        assert!(config.include_protocols.contains(&"SushiSwap".to_string()));
    }

    #[test]
    fn test_subgraph_config_default() {
        let config = SubgraphConfig::default();
        assert!(config.enabled);
        assert_eq!(config.url, "http://localhost:8000");
        assert_eq!(config.poll_interval, 60);
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.request_timeout, 10000);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        
        // Test that all sub-configs are properly initialized
        assert_eq!(config.redis.url, "redis://localhost:6379");
        assert_eq!(config.ethereum.chain_id, 1);
        assert!(config.mempool.enabled);
        assert!(config.flashbots.enabled);
        assert!(config.metrics.enabled);
        assert!(config.filtering.enabled);
        assert!(config.subgraph.enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_redis_url() {
        let mut config = Config::default();
        config.redis.url = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_ethereum_rpc_url() {
        let mut config = Config::default();
        config.ethereum.rpc_url = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_chain_id() {
        let mut config = Config::default();
        config.ethereum.chain_id = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_filtering_config_custom_values() {
        let mut config = FilteringConfig::default();
        
        // Test custom pool addresses
        config.pool_addresses = vec![
            "0x1234567890123456789012345678901234567890".to_string(),
            "0x0987654321098765432109876543210987654321".to_string(),
        ];
        assert_eq!(config.pool_addresses.len(), 2);
        
        // Test custom token addresses
        config.token_addresses = vec![
            "0x1111111111111111111111111111111111111111".to_string(),
        ];
        assert_eq!(config.token_addresses.len(), 1);
        
        // Test custom thresholds
        config.min_liquidity_eth = 500.0;
        config.min_volume_24h_eth = 5000.0;
        assert_eq!(config.min_liquidity_eth, 500.0);
        assert_eq!(config.min_volume_24h_eth, 5000.0);
        
        // Test custom protocols
        config.include_protocols = vec![
            "Balancer".to_string(),
            "Curve".to_string(),
        ];
        assert_eq!(config.include_protocols.len(), 2);
        assert!(config.include_protocols.contains(&"Balancer".to_string()));
        assert!(config.include_protocols.contains(&"Curve".to_string()));
    }

    #[test]
    fn test_mempool_config_custom_values() {
        let mut config = MempoolConfig::default();
        
        config.poll_interval = 200;
        config.max_concurrent_requests = 100;
        config.request_timeout = 20000;
        config.batch_size = 200;
        
        assert_eq!(config.poll_interval, 200);
        assert_eq!(config.max_concurrent_requests, 100);
        assert_eq!(config.request_timeout, 20000);
        assert_eq!(config.batch_size, 200);
    }

    #[test]
    fn test_flashbots_config_custom_values() {
        let mut config = FlashbotsConfig::default();
        
        config.poll_interval = 500;
        config.auth_header = "Bearer test_key".to_string();
        config.max_concurrent_requests = 30;
        config.request_timeout = 20000;
        
        assert_eq!(config.poll_interval, 500);
        assert_eq!(config.auth_header, "Bearer test_key");
        assert_eq!(config.max_concurrent_requests, 30);
        assert_eq!(config.request_timeout, 20000);
    }

    #[test]
    fn test_subgraph_config_custom_values() {
        let mut config = SubgraphConfig::default();
        
        config.url = "http://localhost:8001".to_string();
        config.poll_interval = 120;
        config.cache_ttl_seconds = 600;
        config.max_concurrent_requests = 20;
        config.request_timeout = 20000;
        config.retry_attempts = 5;
        config.retry_delay_ms = 2000;
        
        assert_eq!(config.url, "http://localhost:8001");
        assert_eq!(config.poll_interval, 120);
        assert_eq!(config.cache_ttl_seconds, 600);
        assert_eq!(config.max_concurrent_requests, 20);
        assert_eq!(config.request_timeout, 20000);
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.retry_delay_ms, 2000);
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        
        assert_eq!(config.redis.url, cloned.redis.url);
        assert_eq!(config.ethereum.chain_id, cloned.ethereum.chain_id);
        assert_eq!(config.mempool.enabled, cloned.mempool.enabled);
        assert_eq!(config.flashbots.enabled, cloned.flashbots.enabled);
        assert_eq!(config.filtering.enabled, cloned.filtering.enabled);
        assert_eq!(config.subgraph.enabled, cloned.subgraph.enabled);
    }

    #[test]
    fn test_filtering_config_clone() {
        let config = FilteringConfig::default();
        let cloned = config.clone();
        
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.pool_addresses, cloned.pool_addresses);
        assert_eq!(config.token_addresses, cloned.token_addresses);
        assert_eq!(config.min_liquidity_eth, cloned.min_liquidity_eth);
        assert_eq!(config.min_volume_24h_eth, cloned.min_volume_24h_eth);
        assert_eq!(config.include_protocols, cloned.include_protocols);
    }

    #[test]
    fn test_subgraph_config_clone() {
        let config = SubgraphConfig::default();
        let cloned = config.clone();
        
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.url, cloned.url);
        assert_eq!(config.poll_interval, cloned.poll_interval);
        assert_eq!(config.cache_ttl_seconds, cloned.cache_ttl_seconds);
        assert_eq!(config.max_concurrent_requests, cloned.max_concurrent_requests);
        assert_eq!(config.request_timeout, cloned.request_timeout);
        assert_eq!(config.retry_attempts, cloned.retry_attempts);
        assert_eq!(config.retry_delay_ms, cloned.retry_delay_ms);
    }

    #[test]
    fn test_config_serde_serialization() {
        let config = Config::default();
        
        // Test that we can serialize to TOML
        let toml_string = toml::to_string(&config);
        assert!(toml_string.is_ok());
        
        // Test that we can deserialize from TOML
        let toml_str = toml_string.unwrap();
        let deserialized: Result<Config, _> = toml::from_str(&toml_str);
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_filtering_config_serde_serialization() {
        let config = FilteringConfig::default();
        
        // Test that we can serialize to TOML
        let toml_string = toml::to_string(&config);
        assert!(toml_string.is_ok());
        
        // Test that we can deserialize from TOML
        let toml_str = toml_string.unwrap();
        let deserialized: Result<FilteringConfig, _> = toml::from_str(&toml_str);
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_subgraph_config_serde_serialization() {
        let config = SubgraphConfig::default();
        
        // Test that we can serialize to TOML
        let toml_string = toml::to_string(&config);
        assert!(toml_string.is_ok());
        
        // Test that we can deserialize from TOML
        let toml_str = toml_string.unwrap();
        let deserialized: Result<SubgraphConfig, _> = toml::from_str(&toml_str);
        assert!(deserialized.is_ok());
    }
} 