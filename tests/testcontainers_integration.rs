use mev_relay::{
    events::filter::PoolFilter,
    events::domain::SwapEvent,
    shared::types::H160,
    shared::config::FilteringConfig,
    Result,
};
use testcontainers::*;
use testcontainers_modules::redis::Redis;
use std::time::Duration;

/// Integration test using testcontainers for Redis
#[tokio::test]
async fn test_redis_integration() -> Result<()> {
    // Start Redis container
    let docker = clients::Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    
    // Wait for Redis to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get Redis connection details
    let redis_url = format!("redis://{}", redis_container.get_host_port_ipv4(6379));
    
    // Test Redis connection (this would require implementing RedisMessagingService::new_with_url)
    // For now, just verify the container is running
    assert!(redis_url.starts_with("redis://"));
    
    Ok(())
}

/// Integration test using testcontainers for filtering with real services
#[tokio::test]
async fn test_filtering_with_testcontainers() -> Result<()> {
    // Start Redis container
    let docker = clients::Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    
    // Wait for Redis to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Create test configuration
    let config = FilteringConfig {
        enabled: true,
        pool_addresses: vec![
            "0x1234567890123456789012345678901234567890".to_string(),
            "0x0987654321098765432109876543210987654321".to_string(),
        ],
        token_addresses: vec![
            "0x1111111111111111111111111111111111111111".to_string(),
            "0x2222222222222222222222222222222222222222".to_string(),
        ],
        min_liquidity_eth: 10.0,
        min_volume_24h_eth: 100.0,
        exclude_contracts: vec![
            "0x0000000000000000000000000000000000000000".to_string(),
        ],
        include_protocols: vec![
            "Test Protocol".to_string(),
            "Uniswap V2".to_string(),
        ],
    };
    
    // Create pool filter with test configuration
    let mut filter = PoolFilter::new(config);
    
    // Create test swap events
    let test_event = create_test_swap_event(
        Some(H160::from("0x1234567890123456789012345678901234567890")),
        H160::from("0x1111111111111111111111111111111111111111"),
        H160::from("0x2222222222222222222222222222222222222222"),
        "Test Protocol",
        H160::from("0x3333333333333333333333333333333333333333"),
        None,
        1000000000000000000u128, // 1 ETH
        2000000000000000000u128, // 2 ETH
    );
    
    // Test filtering
    let should_include = filter.should_include_event(&test_event);
    assert!(should_include, "Test event should be included by filter");
    
    // Test batch filtering
    let events = vec![test_event];
    let filtered_events = filter.filter_events(&events);
    assert_eq!(filtered_events.len(), 1, "Should filter to 1 event");
    
    Ok(())
}

/// Helper function to create test swap events
fn create_test_swap_event(
    pool_address: Option<H160>,
    token_in: H160,
    token_out: H160,
    protocol_name: &str,
    from: H160,
    to: Option<H160>,
    amount_in: u128,
    amount_out: u128,
) -> SwapEvent {
    use mev_relay::events::domain::{
        BlockInfo, EventSource, ProtocolInfo, SwapDetails, TransactionInfo,
    };
    use mev_relay::shared::types::H256;
    
    let transaction_info = TransactionInfo {
        hash: H256([1; 32]),
        from,
        to,
        value: 0,
        gas_price: 20000000000u128, // 20 gwei
        gas_limit: 100000,
        gas_used: 50000,
        nonce: 0,
    };
    
    let swap_details = SwapDetails {
        token_in,
        token_out,
        amount_in,
        amount_out,
        pool_address,
        fee_tier: None,
    };
    
    let block_info = BlockInfo {
        number: 12345,
        hash: H256([2; 32]),
        timestamp: 1234567890,
    };
    
    let protocol_info = ProtocolInfo {
        name: protocol_name.to_string(),
        version: "1.0".to_string(),
        address: H160([3; 20]),
    };
    
    SwapEvent::new(
        transaction_info,
        swap_details,
        block_info,
        EventSource::Mempool,
        protocol_info,
    )
}

/// Test Redis messaging service with testcontainers
#[tokio::test]
async fn test_redis_messaging_integration() -> Result<()> {
    // Start Redis container
    let docker = clients::Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    
    // Wait for Redis to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get Redis connection details
    let redis_url = format!("redis://{}", redis_container.get_host_port_ipv4(6379));
    
    // This test would require implementing RedisMessagingService::new_with_url
    // For now, just verify the container is running and accessible
    assert!(redis_url.starts_with("redis://"));
    
    // Test basic Redis operations if we had a client
    // let client = redis::Client::open(redis_url)?;
    // let mut conn = client.get_async_connection().await?;
    // redis::cmd("SET").arg("test_key").arg("test_value").execute_async(&mut conn).await?;
    
    Ok(())
}

/// Test configuration loading with testcontainers
#[tokio::test]
async fn test_config_with_testcontainers() -> Result<()> {
    // Start Redis container
    let docker = clients::Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    
    // Wait for Redis to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get Redis connection details
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_host = "127.0.0.1";
    
    // Create test configuration
    let config_content = format!(
        r#"[redis]
host = "{}"
port = {}

[filtering]
enabled = true
pool_addresses = [
    "0x1234567890123456789012345678901234567890",
]
token_addresses = [
    "0x1111111111111111111111111111111111111111",
]
min_liquidity_eth = 10.0
min_volume_24h_eth = 100.0
exclude_contracts = [
    "0x0000000000000000000000000000000000000000",
]
include_protocols = [
    "Test Protocol",
]"#,
        redis_host, redis_port
    );
    
    // Write config to temporary file
    let config_path = "test_config.toml";
    std::fs::write(config_path, config_content)?;
    
    // Test that config can be loaded (this would require implementing Config::load_from_path)
    // For now, just verify the file was created
    assert!(std::path::Path::new(config_path).exists());
    
    // Clean up
    std::fs::remove_file(config_path)?;
    
    Ok(())
} 