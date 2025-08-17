use mev_relay::{
    events::filter::{PoolFilter, FilterStats},
    shared::config::FilteringConfig,
    events::domain::{SwapEvent, EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo, EventMetadata},
    shared::types::{H160, H256},
};

// Test helper functions
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
    SwapEvent {
        id: EventId::new(),
        transaction: TransactionInfo {
            hash: H256([1; 32]),
            from,
            to,
            value: 0,
            gas_price: 20_000_000_000,
            gas_limit: 100000,
            gas_used: 50000,
            nonce: 1,
        },
        swap_details: SwapDetails {
            token_in,
            token_out,
            amount_in,
            amount_out,
            pool_address,
            fee_tier: Some(3000),
        },
        block_info: BlockInfo {
            number: 12345,
            hash: H256([2; 32]),
            timestamp: 1234567890,
        },
        source: EventSource::Mempool,
        protocol: ProtocolInfo {
            name: protocol_name.to_string(),
            version: "2.0".to_string(),
            address: H160([3; 20]),
        },
        metadata: EventMetadata::new(),
    }
}

fn create_test_address(hex_str: &str) -> H160 {
    let mut bytes = [0u8; 20];
    let hex_bytes = hex::decode(hex_str.strip_prefix("0x").unwrap_or(hex_str))
        .expect("Invalid hex string");
    
    for (i, &byte) in hex_bytes.iter().enumerate() {
        if i < 20 {
            bytes[i] = byte;
        }
    }
    
    H160(bytes)
}

#[test]
fn test_filtering_config_loading() {
    let config = FilteringConfig::default();
    
    // Test that filtering config is properly loaded
    assert!(config.enabled);
    assert!(!config.pool_addresses.is_empty());
    assert!(!config.token_addresses.is_empty());
    assert_eq!(config.min_liquidity_eth, 100.0);
    assert_eq!(config.min_volume_24h_eth, 1000.0);
    
    // Test specific default values (these come from the loaded config)
    assert!(config.pool_addresses.contains(&"0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string()));
    assert!(config.include_protocols.contains(&"Uniswap V2".to_string()));
    assert!(config.include_protocols.contains(&"SushiSwap".to_string()));
}

#[test]
fn test_pool_filter_initialization() {
    let config = FilteringConfig::default();
    let filter = PoolFilter::new(config);
    
    // Test that filter is properly initialized
    let stats = filter.get_filter_stats();
    assert!(stats.total_pools > 0);
    assert!(stats.total_tokens > 0);
    assert!(!stats.included_protocols.is_empty());
    assert_eq!(stats.min_liquidity_eth, 100.0);
    assert_eq!(stats.min_volume_24h_eth, 1000.0);
}

#[test]
fn test_pool_filter_with_custom_config() {
    let mut config = FilteringConfig::default();
    
    // Set custom filtering criteria
    config.pool_addresses = vec![
        "0x1234567890123456789012345678901234567890".to_string(),
        "0x0987654321098765432109876543210987654321".to_string(),
    ];
    config.token_addresses = vec![
        "0x1111111111111111111111111111111111111111".to_string(),
        "0x2222222222222222222222222222222222222222".to_string(),
    ];
    config.include_protocols = vec!["Custom Protocol".to_string()];
    config.min_liquidity_eth = 500.0;
    config.min_volume_24h_eth = 5000.0;
    
    let filter = PoolFilter::new(config);
    let stats = filter.get_filter_stats();
    
    assert_eq!(stats.total_pools, 2);
    assert_eq!(stats.total_tokens, 2);
    assert_eq!(stats.min_liquidity_eth, 500.0);
    assert_eq!(stats.min_volume_24h_eth, 5000.0);
    assert!(stats.included_protocols.contains(&"Custom Protocol".to_string()));
}

#[test]
fn test_event_filtering_integration() {
    let mut config = FilteringConfig::default();
    
    // Set specific pool and token addresses for testing
    config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
    config.token_addresses = vec!["0x1111111111111111111111111111111111111111".to_string()];
    config.include_protocols = vec!["Test Protocol".to_string()];
    config.enabled = true;
    
    let filter = PoolFilter::new(config);
    
    // Create test addresses
    let pool_address = create_test_address("0x1234567890123456789012345678901234567890");
    let token_address = create_test_address("0x1111111111111111111111111111111111111111");
    
    // Test event that should pass all filters
    let valid_event = create_test_swap_event(
        Some(pool_address),
        token_address,
        token_address,
        "Test Protocol",
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    );
    
    assert!(filter.should_include_event(&valid_event));
    
    // Test event with non-matching pool
    let invalid_pool_event = create_test_swap_event(
        Some(H160([0xFF; 20])),
        token_address,
        token_address,
        "Test Protocol",
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    );
    
    assert!(!filter.should_include_event(&invalid_pool_event));
    
    // Test event with non-matching token
    let invalid_token_event = create_test_swap_event(
        Some(pool_address),
        H160([0xFF; 20]),
        H160([0xEE; 20]),
        "Test Protocol",
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    );
    
    assert!(!filter.should_include_event(&invalid_token_event));
    
    // Test event with non-matching protocol
    let invalid_protocol_event = create_test_swap_event(
        Some(pool_address),
        token_address,
        token_address,
        "Wrong Protocol",
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    );
    
    assert!(!filter.should_include_event(&invalid_protocol_event));
}

#[test]
fn test_batch_event_filtering() {
    let mut config = FilteringConfig::default();
    config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
    config.token_addresses = vec!["0x1111111111111111111111111111111111111111".to_string()];
    config.include_protocols = vec!["Test Protocol".to_string()];
    config.enabled = true;
    
    let filter = PoolFilter::new(config);
    
    let pool_address = create_test_address("0x1234567890123456789012345678901234567890");
    let token_address = create_test_address("0x1111111111111111111111111111111111111111");
    
    let events = vec![
        // Valid event
        create_test_swap_event(
            Some(pool_address),
            token_address,
            token_address,
            "Test Protocol",
            H160([0x33; 20]),
            Some(H160([0x44; 20])),
            1_000_000_000_000_000_000, // amount_in
            950_000_000_000_000_000,   // amount_out
        ),
        // Invalid pool
        create_test_swap_event(
            Some(H160([0xFF; 20])),
            token_address,
            token_address,
            "Test Protocol",
            H160([0x33; 20]),
            Some(H160([0x44; 20])),
            1_000_000_000_000_000_000, // amount_in
            950_000_000_000_000_000,   // amount_out
        ),
        // Valid event
        create_test_swap_event(
            Some(pool_address),
            token_address,
            token_address,
            "Test Protocol",
            H160([0x33; 20]),
            Some(H160([0x44; 20])),
            1_000_000_000_000_000_000, // amount_in
            950_000_000_000_000_000,   // amount_out
        ),
        // Invalid token
        create_test_swap_event(
            Some(pool_address),
            H160([0xFF; 20]),
            H160([0xEE; 20]),
            "Test Protocol",
            H160([0x33; 20]),
            Some(H160([0x44; 20])),
            1_000_000_000_000_000_000, // amount_in
            950_000_000_000_000_000,   // amount_out
        ),
    ];
    
    let filtered = filter.filter_events(&events);
    
    // Should keep 2 out of 4 events
    assert_eq!(filtered.len(), 2);
    
    // Verify that only valid events are kept
    for event in filtered {
        assert_eq!(event.swap_details.pool_address.unwrap(), pool_address);
        assert_eq!(event.swap_details.token_in, token_address);
        assert_eq!(event.swap_details.token_out, token_address);
        assert_eq!(event.protocol.name, "Test Protocol");
    }
}

#[test]
fn test_filtering_disabled() {
    let mut config = FilteringConfig::default();
    config.enabled = false;
    
    let filter = PoolFilter::new(config);
    
    // Create an event that would normally be filtered out
    let event = create_test_swap_event(
        Some(H160([0xFF; 20])), // Non-matching pool
        H160([0xFF; 20]),       // Non-matching token
        H160([0xEE; 20]),       // Non-matching token
        "Wrong Protocol",        // Non-matching protocol
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000, // amount_in
        950_000_000_000_000_000,   // amount_out
    );
    
    // Should include event when filtering is disabled
    assert!(filter.should_include_event(&event));
    
    // Test batch filtering
    let events = vec![event.clone(), event.clone()];
    let filtered = filter.filter_events(&events);
    
    // Should keep all events when filtering is disabled
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_config_update() {
    let config = FilteringConfig::default();
    let mut filter = PoolFilter::new(config);
    
    let initial_stats = filter.get_filter_stats();
    let initial_pool_count = initial_stats.total_pools;
    
    // Create new config with different values
    let mut new_config = FilteringConfig::default();
    new_config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
    new_config.token_addresses = vec!["0x1111111111111111111111111111111111111111".to_string()];
    new_config.min_liquidity_eth = 500.0;
    new_config.min_volume_24h_eth = 5000.0;
    
    // Update filter configuration
    filter.update_config(new_config);
    
    let updated_stats = filter.get_filter_stats();
    
    // Verify that configuration was updated
    assert_eq!(updated_stats.total_pools, 1);
    assert_eq!(updated_stats.total_tokens, 1);
    assert_eq!(updated_stats.min_liquidity_eth, 500.0);
    assert_eq!(updated_stats.min_volume_24h_eth, 5000.0);
    
    // Verify that pool count changed
    assert_ne!(updated_stats.total_pools, initial_pool_count);
}

#[test]
fn test_filter_operations() {
    let config = FilteringConfig::default();
    let mut filter = PoolFilter::new(config);
    
    let test_address = H160([0xAA; 20]);
    
    // Test adding pool address
    filter.add_pool_address(test_address);
    assert!(filter.is_pool_filtered(&test_address));
    
    // Test removing pool address
    filter.remove_pool_address(&test_address);
    assert!(!filter.is_pool_filtered(&test_address));
    
    // Test token filtering (using pool address methods for testing)
    let test_token = H160([0xBB; 20]);
    filter.add_pool_address(test_token);
    assert!(filter.is_pool_filtered(&test_token));
}

 