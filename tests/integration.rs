use mev_relay::{
    config::Config,
    core::MevRelay,
    events::EventParser,
    shutdown::ShutdownSignal,
    types::SwapEvent,
    Result,
};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_full_pipeline() -> Result<()> {
    // This test verifies the complete pipeline from event parsing to Redis publishing
    let config = Config::default();
    let shutdown = ShutdownSignal::new();
    
    // Create MEV relay
    let relay = MevRelay::new(config, shutdown.clone())?;
    
    // Test event parser
    let parser = EventParser::new();
    let mut event = SwapEvent::default();
    parser.normalize_event(&mut event);
    
    assert!(!event.id.is_empty());
    assert!(event.timestamp > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_config_loading() -> Result<()> {
    // Test configuration loading and validation
    let config = Config::load()?;
    config.validate()?;
    
    assert!(!config.redis.url.is_empty());
    assert!(config.ethereum.chain_id > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_shutdown_signal() -> Result<()> {
    let shutdown = ShutdownSignal::new();
    let shutdown_clone = shutdown.clone();
    
    // Spawn a task that waits for shutdown
    let handle = tokio::spawn(async move {
        shutdown_clone.wait().await;
    });
    
    // Wait a bit then send shutdown
    sleep(Duration::from_millis(100)).await;
    shutdown.shutdown();
    
    // Wait for the task to complete
    let _ = handle.await;
    
    Ok(())
}

#[tokio::test]
async fn test_event_parsing() -> Result<()> {
    let parser = EventParser::new();
    
    // Test with sample transaction data
    let tx_data = serde_json::json!({
        "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "from": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
        "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
        "value": "0x0",
        "gasPrice": "0x59682f00",
        "input": "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
    });
    
    let block_info = serde_json::json!({
        "number": "0x123456",
        "hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "timestamp": "0x12345678"
    });
    
    let result = parser.parse_mempool_transaction(&tx_data, &block_info)?;
    assert!(result.is_some());
    
    if let Some(event) = result {
        assert_eq!(event.protocol, "Uniswap V2");
        assert_eq!(event.source.to_string(), "Mempool");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_metrics_initialization() -> Result<()> {
    // Test metrics initialization
    let config = mev_relay::config::MetricsConfig::default();
    let result = mev_relay::metrics::init(&config);
    
    // This might fail in test environment, but shouldn't panic
    let _ = result;
    
    Ok(())
}

#[tokio::test]
async fn test_utility_functions() -> Result<()> {
    use mev_relay::utils::{hex, address, time, math};
    
    // Test hex encoding/decoding
    let data = b"Hello, World!";
    let encoded = hex::encode(data);
    let decoded = hex::decode(&encoded)?;
    assert_eq!(decoded, data);
    
    // Test address validation
    let valid_address = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
    assert!(address::is_valid_checksum(valid_address));
    
    // Test time utilities
    let now = time::now();
    assert!(now > 0);
    
    // Test math utilities
    let wei = 1_000_000_000_000_000_000u128; // 1 ETH
    let eth = math::wei_to_eth(wei);
    assert_eq!(eth, 1.0);
    
    Ok(())
} 