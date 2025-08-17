use std::time::Duration;
use testcontainers::*;
use testcontainers_modules::redis::Redis;
use testcontainers_modules::postgres::Postgres;

/// Test container configuration for integration tests
pub struct TestContainers {
    pub redis_port: u16,
    pub postgres_port: u16,
}

impl TestContainers {
    /// Create new test containers for integration tests
    pub async fn new() -> Self {
        let docker = clients::Cli::default();
        
        // Start Redis container
        let redis_image = Redis::default();
        let redis = docker.run(redis_image);
        let redis_port = redis.get_host_port_ipv4(6379);
        
        // Start PostgreSQL container
        let postgres_image = Postgres::default()
            .with_db_name("mev_relay_test")
            .with_user("test_user")
            .with_password("test_password");
        let postgres = docker.run(postgres_image);
        let postgres_port = postgres.get_host_port_ipv4(5432);
        
        // Wait for services to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        Self { redis_port, postgres_port }
    }
    
    /// Get Redis connection string
    pub fn redis_url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.redis_port)
    }
    
    /// Get PostgreSQL connection string
    pub fn postgres_url(&self) -> String {
        format!("postgresql://test_user:test_password@127.0.0.1:{}:5432/mev_relay_test", self.postgres_port)
    }
    
    /// Get Redis host and port for configuration
    pub fn redis_host_port(&self) -> (String, u16) {
        let host = "127.0.0.1".to_string();
        (host, self.redis_port)
    }
    
    /// Get PostgreSQL host and port for configuration
    pub fn postgres_host_port(&self) -> (String, u16) {
        let host = "127.0.0.1".to_string();
        (host, self.postgres_port)
    }
}

/// Test configuration builder for integration tests
pub struct TestConfig {
    pub redis_url: String,
    pub postgres_url: String,
    pub redis_host: String,
    pub redis_port: u16,
    pub postgres_host: String,
    pub postgres_port: u16,
}

impl TestConfig {
    /// Create test configuration from test containers
    pub fn from_containers(containers: &TestContainers) -> Self {
        let (redis_host, redis_port) = containers.redis_host_port();
        let (postgres_host, postgres_port) = containers.postgres_host_port();
        
        Self {
            redis_url: containers.redis_url(),
            postgres_url: containers.postgres_url(),
            redis_host,
            redis_port,
            postgres_host,
            postgres_port,
        }
    }
    
    /// Create a test configuration file content
    pub fn to_toml(&self) -> String {
        format!(
            r#"[redis]
host = "{}"
port = {}
url = "{}"

[postgres]
host = "{}"
port = {}
url = "{}"

[filtering]
enabled = true
pool_addresses = [
    "0x1234567890123456789012345678901234567890",
    "0x0987654321098765432109876543210987654321",
]
token_addresses = [
    "0x1111111111111111111111111111111111111111",
    "0x2222222222222222222222222222222222222222",
]
min_liquidity_eth = 10.0
min_volume_24h_eth = 100.0
exclude_contracts = [
    "0x0000000000000000000000000000000000000000",
]
include_protocols = [
    "Test Protocol",
    "Uniswap V2",
]"#,
            self.redis_host, self.redis_port, self.redis_url,
            self.postgres_host, self.postgres_port, self.postgres_url
        )
    }
}

/// Helper function to create test configuration file
pub async fn create_test_config() -> (TestContainers, TestConfig) {
    let containers = TestContainers::new().await;
    let config = TestConfig::from_containers(&containers);
    (containers, config)
}

/// Helper function to write test configuration to file
pub fn write_test_config(config: &TestConfig, path: &str) -> std::io::Result<()> {
    std::fs::write(path, config.to_toml())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_testcontainers_creation() {
        let containers = TestContainers::new().await;
        
        // Verify Redis is accessible
        let redis_url = containers.redis_url();
        assert!(redis_url.starts_with("redis://"));
        
        // Verify PostgreSQL is accessible
        let postgres_url = containers.postgres_url();
        assert!(postgres_url.starts_with("postgresql://"));
    }
    
    #[tokio::test]
    async fn test_config_generation() {
        let containers = TestContainers::new().await;
        let config = TestConfig::from_containers(&containers);
        
        // Verify configuration values
        assert!(!config.redis_url.is_empty());
        assert!(!config.postgres_url.is_empty());
        assert!(config.redis_port > 0);
        assert!(config.postgres_port > 0);
        
        // Verify TOML generation
        let toml_content = config.to_toml();
        assert!(toml_content.contains("redis"));
        assert!(toml_content.contains("postgres"));
        assert!(toml_content.contains("filtering"));
    }
} 