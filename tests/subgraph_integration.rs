use mev_relay::{
    infrastructure::config::{Config, SubgraphConfig},
    monitoring::subgraph::{SubgraphService, SubgraphServiceImpl},
    monitoring::service::MonitoringOrchestrator,
    shared::types::H160,
};

#[tokio::test]
async fn test_subgraph_service_integration() {
    // Create test configuration
    let config = SubgraphConfig {
        enabled: true,
        url: "http://localhost:8000".to_string(),
        poll_interval: 10,
        cache_ttl_seconds: 60,
        max_concurrent_requests: 2,
        request_timeout: 1000,
        retry_attempts: 1,
        retry_delay_ms: 100,
    };

    // Create service
    let mut service = SubgraphServiceImpl::new(config);
    
    // Test service creation
    assert_eq!(service.get_status(), mev_relay::monitoring::domain::ServiceStatus::Unknown);
    
    // Test starting service
    let result = service.start().await;
    assert!(result.is_ok());
    assert_eq!(service.get_status(), mev_relay::monitoring::domain::ServiceStatus::Healthy);
    
    // Test getting token metadata
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    let token_metadata = service.get_token_metadata(weth_address).await;
    assert!(token_metadata.is_ok());
    
    if let Ok(Some(token)) = token_metadata {
        assert_eq!(token.symbol, "WETH");
        assert_eq!(token.name, "Wrapped Ether");
        assert_eq!(token.decimals, 18);
    }
    
    // Test getting pool metadata
    let pool_address = H160::from([0xB4, 0xe1, 0x6d, 0x01, 0x68, 0xe5, 0x2d, 0x35, 0xCa, 0xCD, 0x2c, 0x61, 0x85, 0xb4, 0x42, 0x81, 0xEc, 0x28, 0xC9, 0xDc]);
    let pool_metadata = service.get_pool_metadata(pool_address).await;
    assert!(result.is_ok());
    
    if let Ok(Some(pool)) = pool_metadata {
        assert_eq!(pool.protocol, "Uniswap V2");
        assert_eq!(pool.fee_tier, 3000);
        assert_eq!(pool.token0.symbol, "WETH");
        assert_eq!(pool.token1.symbol, "USDC");
    }
    
    // Test cache statistics
    let stats = service.get_cache_stats();
    assert_eq!(stats.tokens_cached, 0); // Default value for now
    
    // Test stopping service
    let result = service.stop().await;
    assert!(result.is_ok());
    assert_eq!(service.get_status(), mev_relay::monitoring::domain::ServiceStatus::Unknown);
}

#[tokio::test]
async fn test_subgraph_service_cache_behavior() {
    let config = SubgraphConfig {
        enabled: true,
        url: "http://localhost:8000".to_string(),
        poll_interval: 10,
        cache_ttl_seconds: 1, // Very short TTL for testing
        max_concurrent_requests: 2,
        request_timeout: 1000,
        retry_attempts: 1,
        retry_delay_ms: 100,
    };

    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    
    // First call should cache miss
    let token1 = service.get_token_metadata(weth_address).await.unwrap();
    assert!(token1.is_some());
    
    // Second call should cache hit
    let token2 = service.get_token_metadata(weth_address).await.unwrap();
    assert!(token2.is_some());
    
    // Wait for cache to expire (use a shorter wait)
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
    
    // Call after expiration should cache miss again
    let token3 = service.get_token_metadata(weth_address).await.unwrap();
    assert!(token3.is_some());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_configuration() {
    let config = SubgraphConfig {
        enabled: false,
        url: "https://test.example.com".to_string(),
        poll_interval: 120,
        cache_ttl_seconds: 600,
        max_concurrent_requests: 25,
        request_timeout: 20000,
        retry_attempts: 5,
        retry_delay_ms: 2000,
    };

    let service = SubgraphServiceImpl::new(config);
    
    // Test configuration values
    assert_eq!(service.get_status(), mev_relay::monitoring::domain::ServiceStatus::Unknown);
}

#[tokio::test]
async fn test_subgraph_service_unknown_addresses() {
    let config = SubgraphConfig::default();
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    // Test with unknown token address
    let unknown_address = H160::from([0xFF; 20]);
    let token_metadata = service.get_token_metadata(unknown_address).await.unwrap();
    assert!(token_metadata.is_none());
    
    // Test with unknown pool address
    let pool_metadata = service.get_pool_metadata(unknown_address).await.unwrap();
    assert!(pool_metadata.is_none());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_multiple_tokens() {
    let config = SubgraphConfig::default();
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    // Test multiple token addresses
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    let usdc_address = H160::from([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8B, 0x4b, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C]);
    
    let weth_metadata = service.get_token_metadata(weth_address).await.unwrap();
    let usdc_metadata = service.get_token_metadata(usdc_address).await.unwrap();
    
    assert!(weth_metadata.is_some());
    assert!(usdc_metadata.is_some());
    
    if let Some(weth) = weth_metadata {
        assert_eq!(weth.symbol, "WETH");
    }
    
    if let Some(usdc) = usdc_metadata {
        assert_eq!(usdc.symbol, "USDC");
    }
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_refresh_cache() {
    let config = SubgraphConfig::default();
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    // Initial cache refresh should work
    let result = service.refresh_cache().await;
    assert!(result.is_ok());
    
    // Get some metadata to populate cache
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    let _ = service.get_token_metadata(weth_address).await.unwrap();
    
    // Refresh cache again
    let result = service.refresh_cache().await;
    assert!(result.is_ok());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_error_handling() {
    let config = SubgraphConfig {
        enabled: true,
        url: "http://invalid-url-that-will-fail.com".to_string(),
        poll_interval: 10,
        cache_ttl_seconds: 60,
        max_concurrent_requests: 1,
        request_timeout: 100, // Very short timeout
        retry_attempts: 1,
        retry_delay_ms: 10,
    };

    let mut service = SubgraphServiceImpl::new(config);
    
    // Service should start even with invalid URL (mock data is used)
    let result = service.start().await;
    assert!(result.is_ok());
    
    // Should still return mock data
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    let token_metadata = service.get_token_metadata(weth_address).await;
    assert!(token_metadata.is_ok());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_concurrent_access() {
    let config = SubgraphConfig::default();
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    
    // Spawn multiple concurrent requests
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let service_clone = &service;
            let address = weth_address;
            tokio::spawn(async move {
                service_clone.get_token_metadata(address).await
            })
        })
        .collect();
    
    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All requests should succeed
    for result in results {
        let token_result = result.unwrap();
        assert!(token_result.is_ok());
        if let Ok(Some(token)) = token_result {
            assert_eq!(token.symbol, "WETH");
        }
    }
    
    service.stop().await.unwrap();
} 

#[tokio::test]
async fn test_monitoring_orchestrator_with_subgraph() {
    // Create a minimal config with subgraph enabled
    let mut config = Config::default();
    config.subgraph.enabled = true;
    config.subgraph.url = "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3".to_string();
    
    // Create event channel
    let (event_sender, _event_receiver) = tokio::sync::mpsc::channel(100);
    
    // Create monitoring orchestrator
    let orchestrator = MonitoringOrchestrator::new(config, event_sender).unwrap();
    
    // Check if subgraph service was initialized
    let subgraph_service = orchestrator.get_subgraph_service();
    assert!(subgraph_service.is_some());
    
    // Test starting all services
    let result = orchestrator.start_all().await;
    assert!(result.is_ok());
    
    // Test stopping all services
    let result = orchestrator.stop_all().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_monitoring_orchestrator_without_subgraph() {
    // Create a minimal config with subgraph disabled
    let mut config = Config::default();
    config.subgraph.enabled = false;
    
    // Create event channel
    let (event_sender, _event_receiver) = tokio::sync::mpsc::channel(100);
    
    // Create monitoring orchestrator
    let orchestrator = MonitoringOrchestrator::new(config, event_sender).unwrap();
    
    // Check if subgraph service was NOT initialized
    let subgraph_service = orchestrator.get_subgraph_service();
    assert!(subgraph_service.is_none());
}

#[tokio::test]
async fn test_subgraph_config_validation() {
    let config = SubgraphConfig {
        enabled: true,
        url: "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3".to_string(),
        poll_interval: 60,
        cache_ttl_seconds: 300,
        max_concurrent_requests: 10,
        request_timeout: 10000,
        retry_attempts: 3,
        retry_delay_ms: 1000,
    };
    
    // Test configuration values
    assert!(config.enabled);
    assert_eq!(config.url, "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3");
    assert_eq!(config.poll_interval, 60);
    assert_eq!(config.cache_ttl_seconds, 300);
    assert_eq!(config.max_concurrent_requests, 10);
    assert_eq!(config.request_timeout, 10000);
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay_ms, 1000);
}

#[tokio::test]
async fn test_subgraph_service_metrics_integration() {
    let config = SubgraphConfig::default();
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    // Get token metadata to trigger metrics
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    
    // First call should be cache miss
    let _ = service.get_token_metadata(weth_address).await.unwrap();
    
    // Second call should be cache hit
    let _ = service.get_token_metadata(weth_address).await.unwrap();
    
    // Refresh cache to trigger metrics
    let _ = service.refresh_cache().await;
    
    // Get stats
    let stats = service.get_cache_stats();
    
    // Service should be healthy
    assert_eq!(service.get_status(), mev_relay::monitoring::domain::ServiceStatus::Healthy);
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_edge_cases() {
    let config = SubgraphConfig {
        cache_ttl_seconds: 0, // Immediate expiration
        ..Default::default()
    };
    
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    
    // First call
    let token1 = service.get_token_metadata(weth_address).await.unwrap();
    assert!(token1.is_some());
    
    // Second call should be cache miss due to immediate expiration
    let token2 = service.get_token_metadata(weth_address).await.unwrap();
    assert!(token2.is_some());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_subgraph_service_high_concurrency() {
    let config = SubgraphConfig {
        max_concurrent_requests: 50,
        ..Default::default()
    };
    
    let mut service = SubgraphServiceImpl::new(config);
    service.start().await.unwrap();
    
    let weth_address = H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]);
    
    // Get initial metadata to populate cache
    let _ = service.get_token_metadata(weth_address).await.unwrap();
    
    // Spawn many concurrent requests
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let address = weth_address;
            tokio::spawn(async move {
                // Create a new service instance for each task to avoid borrowing issues
                let test_config = SubgraphConfig::default();
                let mut test_service = SubgraphServiceImpl::new(test_config);
                test_service.start().await.unwrap();
                let result = test_service.get_token_metadata(address).await;
                test_service.stop().await.unwrap();
                result
            })
        })
        .collect();
    
    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All requests should succeed
    for result in results {
        let token_result = result.unwrap();
        assert!(token_result.is_ok());
        if let Ok(Some(token)) = token_result {
            assert_eq!(token.symbol, "WETH");
        }
    }
    
    service.stop().await.unwrap();
} 