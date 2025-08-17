use mev_relay::{
    events::domain::{SwapEvent, EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo, EventMetadata},
    shared::types::{H160, H256},
};

/// Create a test swap event with customizable parameters
pub fn create_test_swap_event(
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

/// Create a test swap event with default values
pub fn create_default_test_swap_event() -> SwapEvent {
    create_test_swap_event(
        Some(H160([0xAA; 20])),
        H160([0x11; 20]),
        H160([0x22; 20]),
        "Test Protocol",
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000, // 1 ETH
        950_000_000_000_000_000,   // 0.95 ETH
    )
}

/// Create a test address from a hex string
pub fn create_test_address(hex_str: &str) -> H160 {
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

/// Create a test hash from a hex string
pub fn create_test_hash(hex_str: &str) -> H256 {
    let mut bytes = [0u8; 32];
    let hex_bytes = hex::decode(hex_str.strip_prefix("0x").unwrap_or(hex_str))
        .expect("Invalid hex string");
    
    for (i, &byte) in hex_bytes.iter().enumerate() {
        if i < 32 {
            bytes[i] = byte;
        }
    }
    
    H256(bytes)
}

/// Create a vector of test swap events
pub fn create_test_swap_events(count: usize) -> Vec<SwapEvent> {
    (0..count)
        .map(|i| {
            create_test_swap_event(
                Some(H160([i as u8; 20])),
                H160([0x11; 20]),
                H160([0x22; 20]),
                "Test Protocol",
                H160([0x33; 20]),
                Some(H160([0x44; 20])),
                1_000_000_000_000_000_000,
                950_000_000_000_000_000,
            )
        })
        .collect()
}

/// Create a test swap event that should pass all filters
pub fn create_valid_test_swap_event(
    pool_address: H160,
    token_address: H160,
    protocol_name: &str,
) -> SwapEvent {
    create_test_swap_event(
        Some(pool_address),
        token_address,
        token_address,
        protocol_name,
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    )
}

/// Create a test swap event that should be filtered out
pub fn create_invalid_test_swap_event(
    pool_address: H160,
    token_address: H160,
    protocol_name: &str,
) -> SwapEvent {
    create_test_swap_event(
        Some(pool_address),
        token_address,
        token_address,
        protocol_name,
        H160([0x33; 20]),
        Some(H160([0x44; 20])),
        1_000_000_000_000_000_000,
        950_000_000_000_000_000,
    )
}

/// Assert that two H160 addresses are equal
pub fn assert_address_eq(actual: &H160, expected: &H160) {
    assert_eq!(actual.0, expected.0, "Addresses should be equal");
}

/// Assert that two H256 hashes are equal
pub fn assert_hash_eq(actual: &H256, expected: &H256) {
    assert_eq!(actual.0, expected.0, "Hashes should be equal");
}

/// Create a test configuration with minimal settings
pub fn create_minimal_test_config() -> mev_relay::shared::config::FilteringConfig {
    mev_relay::shared::config::FilteringConfig {
        enabled: true,
        pool_addresses: vec![
            "0x1234567890123456789012345678901234567890".to_string(),
        ],
        token_addresses: vec![
            "0x1111111111111111111111111111111111111111".to_string(),
        ],
        min_liquidity_eth: 10.0,
        min_volume_24h_eth: 100.0,
        exclude_contracts: vec![
            "0x0000000000000000000000000000000000000000".to_string(),
        ],
        include_protocols: vec![
            "Test Protocol".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_address() {
        let address = create_test_address("0x1234567890123456789012345678901234567890");
        assert_eq!(address.0[0], 0x12);
        assert_eq!(address.0[19], 0x90);
    }

    #[test]
    fn test_create_test_hash() {
        let hash = create_test_hash("0x1234567890123456789012345678901234567890123456789012345678901234");
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0x34);
    }

    #[test]
    fn test_create_test_swap_events() {
        let events = create_test_swap_events(3);
        assert_eq!(events.len(), 3);
        
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.swap_details.pool_address.unwrap().0[0], i as u8);
        }
    }

    #[test]
    fn test_create_valid_test_swap_event() {
        let pool_address = H160([0xAA; 20]);
        let token_address = H160([0x11; 20]);
        let protocol_name = "Test Protocol";
        
        let event = create_valid_test_swap_event(pool_address, token_address, protocol_name);
        
        assert_eq!(event.swap_details.pool_address.unwrap(), pool_address);
        assert_eq!(event.swap_details.token_in, token_address);
        assert_eq!(event.swap_details.token_out, token_address);
        assert_eq!(event.protocol.name, protocol_name);
    }

    #[test]
    fn test_assert_address_eq() {
        let addr1 = H160([0xAA; 20]);
        let addr2 = H160([0xAA; 20]);
        
        assert_address_eq(&addr1, &addr2);
    }

    #[test]
    fn test_assert_hash_eq() {
        let hash1 = H256([0xBB; 32]);
        let hash2 = H256([0xBB; 32]);
        
        assert_hash_eq(&hash1, &hash2);
    }
} 