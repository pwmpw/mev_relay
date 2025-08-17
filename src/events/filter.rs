use crate::{
    events::domain::SwapEvent,
    shared::config::FilteringConfig,
    shared::types::{H160, H256},
};
use std::collections::HashSet;
use tracing::{debug, info, warn};

/// Pool filtering service for filtering swap events based on various criteria
#[derive(Clone)]
pub struct PoolFilter {
    config: FilteringConfig,
    pool_addresses: HashSet<H160>,
    token_addresses: HashSet<H160>,
    exclude_contracts: HashSet<H160>,
}

impl PoolFilter {
    pub fn new(config: FilteringConfig) -> Self {
        let mut filter = Self {
            config,
            pool_addresses: HashSet::new(),
            token_addresses: HashSet::new(),
            exclude_contracts: HashSet::new(),
        };
        
        filter.initialize_filter_sets();
        filter
    }

    /// Initialize filter sets from configuration
    fn initialize_filter_sets(&mut self) {
        // Clear existing filter sets
        self.pool_addresses.clear();
        self.token_addresses.clear();
        self.exclude_contracts.clear();
        
        // Convert pool addresses
        for addr_str in &self.config.pool_addresses {
            if let Ok(address) = self.parse_address(addr_str) {
                self.pool_addresses.insert(address);
                debug!("Added pool address to filter: {}", addr_str);
            } else {
                warn!("Invalid pool address in config: {}", addr_str);
            }
        }

        // Convert token addresses
        for addr_str in &self.config.token_addresses {
            if let Ok(address) = self.parse_address(addr_str) {
                self.token_addresses.insert(address);
                debug!("Added token address to filter: {}", addr_str);
            } else {
                warn!("Invalid token address in config: {}", addr_str);
            }
        }

        // Convert exclude contracts
        for addr_str in &self.config.exclude_contracts {
            if let Ok(address) = self.parse_address(addr_str) {
                self.exclude_contracts.insert(address);
                debug!("Added exclude contract: {}", addr_str);
            } else {
                warn!("Invalid exclude contract in config: {}", addr_str);
            }
        }

        info!(
            "Pool filter initialized with {} pools, {} tokens, {} excluded contracts",
            self.pool_addresses.len(),
            self.token_addresses.len(),
            self.exclude_contracts.len()
        );
    }

    /// Check if a swap event should be included based on filtering criteria
    pub fn should_include_event(&self, event: &SwapEvent) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Check if transaction is from excluded contracts
        if self.exclude_contracts.contains(&event.transaction.from) {
            debug!("Excluding event from excluded contract: {}", event.transaction.from);
            return false;
        }

        // Check if transaction is to excluded contracts
        if let Some(to) = event.transaction.to {
            if self.exclude_contracts.contains(&to) {
                debug!("Excluding event to excluded contract: {}", to);
                return false;
            }
        }

        // Check if pool address is in our filter
        if let Some(pool_addr) = event.swap_details.pool_address {
            if !self.pool_addresses.contains(&pool_addr) {
                debug!("Excluding event from non-filtered pool: {}", pool_addr);
                return false;
            }
        }

        // Check if tokens are in our filter
        if !self.token_addresses.contains(&event.swap_details.token_in) {
            debug!("Excluding event with non-filtered token_in: {}", event.swap_details.token_in);
            return false;
        }

        if !self.token_addresses.contains(&event.swap_details.token_out) {
            debug!("Excluding event with non-filtered token_out: {}", event.swap_details.token_out);
            return false;
        }

        // Check protocol inclusion
        if !self.config.include_protocols.contains(&event.protocol.name) {
            debug!("Excluding event from non-included protocol: {}", event.protocol.name);
            return false;
        }

        // Check minimum liquidity (if pool address is available)
        if let Some(pool_addr) = event.swap_details.pool_address {
            if !self.check_minimum_liquidity(pool_addr) {
                debug!("Excluding event from low liquidity pool: {}", pool_addr);
                return false;
            }
        }

        // Check minimum volume (24h)
        if !self.check_minimum_volume(event) {
            debug!("Excluding event with low volume");
            return false;
        }

        debug!("Event passed all filters: {}", event.id.as_str());
        true
    }

    /// Filter a list of events
    pub fn filter_events(&self, events: &[SwapEvent]) -> Vec<SwapEvent> {
        if !self.config.enabled {
            return events.to_vec();
        }

        let filtered: Vec<SwapEvent> = events
            .iter()
            .filter(|event| self.should_include_event(event))
            .cloned()
            .collect();

        let filtered_count = filtered.len();
        let total_count = events.len() - filtered_count;
        
        if total_count > 0 {
            info!(
                "Filtered {} events, kept {} events ({}% filtered)",
                total_count,
                filtered_count,
                (total_count as f64 / (total_count + filtered_count) as f64 * 100.0) as u32
            );
        }

        filtered
    }

    /// Check if a pool meets minimum liquidity requirements
    fn check_minimum_liquidity(&self, _pool_address: H160) -> bool {
        // TODO: Implement actual liquidity checking
        // This would typically involve:
        // 1. Querying the pool contract for reserves
        // 2. Converting to ETH value
        // 3. Comparing against min_liquidity_eth
        
        // For now, return true to avoid blocking events
        true
    }

    /// Check if an event meets minimum volume requirements
    fn check_minimum_volume(&self, event: &SwapEvent) -> bool {
        // Convert amount_in to ETH value for volume checking
        let amount_in_eth = self.wei_to_eth(event.swap_details.amount_in);
        
        // Simple volume check - in production you'd want to track 24h volume
        amount_in_eth >= self.config.min_volume_24h_eth / 1000.0 // Rough estimate
    }

    /// Parse address string to H160
    fn parse_address(&self, addr_str: &str) -> Result<H160, String> {
        if addr_str.len() != 42 || !addr_str.starts_with("0x") {
            return Err(format!("Invalid address format: {}", addr_str));
        }

        let mut bytes = [0u8; 20];
        for (i, byte) in hex::decode(&addr_str[2..])
            .map_err(|e| format!("Invalid hex: {}", e))?
            .iter()
            .enumerate()
        {
            if i < 20 {
                bytes[i] = *byte;
            }
        }

        Ok(H160(bytes))
    }

    /// Convert Wei to ETH
    fn wei_to_eth(&self, wei: u128) -> f64 {
        wei as f64 / 1_000_000_000_000_000_000.0
    }

    /// Get filter statistics
    pub fn get_filter_stats(&self) -> FilterStats {
        FilterStats {
            total_pools: self.pool_addresses.len(),
            total_tokens: self.token_addresses.len(),
            excluded_contracts: self.exclude_contracts.len(),
            min_liquidity_eth: self.config.min_liquidity_eth,
            min_volume_24h_eth: self.config.min_volume_24h_eth,
            included_protocols: self.config.include_protocols.clone(),
        }
    }

    /// Update filter configuration
    pub fn update_config(&mut self, new_config: FilteringConfig) {
        self.config = new_config;
        self.initialize_filter_sets();
        info!("Pool filter configuration updated");
    }

    /// Add a pool address to the filter
    pub fn add_pool_address(&mut self, address: H160) {
        self.pool_addresses.insert(address);
        info!("Added pool address to filter: {}", address);
    }

    /// Remove a pool address from the filter
    pub fn remove_pool_address(&mut self, address: &H160) {
        if self.pool_addresses.remove(address) {
            info!("Removed pool address from filter: {}", address);
        }
    }

    /// Check if an address is in the pool filter
    pub fn is_pool_filtered(&self, address: &H160) -> bool {
        self.pool_addresses.contains(address)
    }

    /// Check if a token is in the token filter
    pub fn is_token_filtered(&self, address: &H160) -> bool {
        self.token_addresses.contains(address)
    }
}

/// Filter statistics
#[derive(Debug, Clone)]
pub struct FilterStats {
    pub total_pools: usize,
    pub total_tokens: usize,
    pub excluded_contracts: usize,
    pub min_liquidity_eth: f64,
    pub min_volume_24h_eth: f64,
    pub included_protocols: Vec<String>,
}

impl Default for PoolFilter {
    fn default() -> Self {
        Self::new(FilteringConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::domain::{EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo, EventMetadata};

    fn create_test_swap_event(
        pool_address: Option<H160>,
        token_in: H160,
        token_out: H160,
        protocol_name: &str,
        from: H160,
        to: Option<H160>,
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
                amount_in: 1_000_000_000_000_000_000, // 1 ETH
                amount_out: 950_000_000_000_000_000,
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

    #[test]
    fn test_pool_filter_creation() {
        let config = FilteringConfig::default();
        let filter = PoolFilter::new(config);
        
        assert!(filter.config.enabled);
        assert!(!filter.pool_addresses.is_empty());
        assert!(!filter.token_addresses.is_empty());
    }

    #[test]
    fn test_address_parsing() {
        let config = FilteringConfig::default();
        let filter = PoolFilter::new(config);
        
        let valid_address = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
        let result = filter.parse_address(valid_address);
        assert!(result.is_ok());
        
        let invalid_address = "invalid";
        let result = filter.parse_address(invalid_address);
        assert!(result.is_err());
        
        let short_address = "0x123";
        let result = filter.parse_address(short_address);
        assert!(result.is_err());
        
        let no_prefix = "742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
        let result = filter.parse_address(no_prefix);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_stats() {
        let config = FilteringConfig::default();
        let filter = PoolFilter::new(config);
        
        let stats = filter.get_filter_stats();
        assert!(stats.total_pools > 0);
        assert!(stats.total_tokens > 0);
        assert_eq!(stats.min_liquidity_eth, 100.0);
        assert_eq!(stats.min_volume_24h_eth, 1000.0);
        assert!(stats.included_protocols.contains(&"Uniswap V2".to_string()));
    }

    #[test]
    fn test_pool_filter_operations() {
        let config = FilteringConfig::default();
        let mut filter = PoolFilter::new(config);
        
        let test_address = H160([1; 20]);
        
        // Test adding pool address
        filter.add_pool_address(test_address);
        assert!(filter.is_pool_filtered(&test_address));
        
        // Test removing pool address
        filter.remove_pool_address(&test_address);
        assert!(!filter.is_pool_filtered(&test_address));
    }

    #[test]
    fn test_event_filtering_by_pool() {
        let mut config = FilteringConfig::default();
        config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
        config.token_addresses = vec!["0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string()]; // WETH
        config.enabled = true;
        
        let filter = PoolFilter::new(config);
        
        // Event with matching pool address
        let pool_address = H160([0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90]);
        // Use a token address that's in the default config (WETH)
        let token_address = H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]);
        let event = create_test_swap_event(
            Some(pool_address),
            token_address,
            token_address,
            "Uniswap V2",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(filter.should_include_event(&event));
        
        // Event with non-matching pool address
        let non_matching_pool = H160([0xFF; 20]);
        let event = create_test_swap_event(
            Some(non_matching_pool),
            token_address,
            token_address,
            "Uniswap V2",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(!filter.should_include_event(&event));
    }

    #[test]
    fn test_event_filtering_by_token() {
        let mut config = FilteringConfig::default();
        config.token_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
        config.pool_addresses = vec!["0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string()]; // WETH pool
        config.enabled = true;
        
        let filter = PoolFilter::new(config);
        
        let token_address = H160([0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90]);
        
        // Event with matching token addresses
        let event = create_test_swap_event(
            Some(H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C])), // WETH pool from default config
            token_address,
            token_address,
            "Uniswap V2",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(filter.should_include_event(&event));
        
        // Event with non-matching token addresses
        let event = create_test_swap_event(
            Some(H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C])), // WETH pool from default config
            H160([0xFF; 20]),
            H160([0xEE; 20]),
            "Uniswap V2",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(!filter.should_include_event(&event));
    }

    #[test]
    fn test_event_filtering_by_protocol() {
        let mut config = FilteringConfig::default();
        config.include_protocols = vec!["Uniswap V2".to_string()];
        config.pool_addresses = vec!["0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string()]; // WETH pool
        config.token_addresses = vec!["0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string()]; // WETH
        config.enabled = true;
        
        let filter = PoolFilter::new(config);
        
        // Event with matching protocol
        let event = create_test_swap_event(
            Some(H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C])), // WETH pool from default config
            H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]), // WETH from default config
            H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]), // WETH from default config
            "Uniswap V2",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(filter.should_include_event(&event));
        
        // Event with non-matching protocol
        let event = create_test_swap_event(
            Some(H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C])), // WETH pool from default config
            H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]), // WETH from default config
            H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]), // WETH from default config
            "SushiSwap",
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        assert!(!filter.should_include_event(&event));
    }

    #[test]
    fn test_event_filtering_by_excluded_contracts() {
        let mut config = FilteringConfig::default();
        config.exclude_contracts = vec!["0x1234567890123456789012345678901234567890".to_string()];
        config.enabled = true;
        
        let filter = PoolFilter::new(config);
        
        let excluded_address = H160([0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90]);
        
        // Event from excluded contract
        let event = create_test_swap_event(
            Some(H160([5; 20])),
            H160([1; 20]),
            H160([2; 20]),
            "Uniswap V2",
            excluded_address,
            Some(H160([4; 20])),
        );
        
        assert!(!filter.should_include_event(&event));
        
        // Event to excluded contract
        let event = create_test_swap_event(
            Some(H160([5; 20])),
            H160([1; 20]),
            H160([2; 20]),
            "Uniswap V2",
            H160([3; 20]),
            Some(excluded_address),
        );
        
        assert!(!filter.should_include_event(&event));
    }

    #[test]
    fn test_filtering_disabled() {
        let mut config = FilteringConfig::default();
        config.enabled = false;
        
        let filter = PoolFilter::new(config);
        
        let event = create_test_swap_event(
            Some(H160([5; 20])),
            H160([0xFF; 20]), // Non-matching token
            H160([0xEE; 20]), // Non-matching token
            "Unknown Protocol", // Non-matching protocol
            H160([3; 20]),
            Some(H160([4; 20])),
        );
        
        // Should include event when filtering is disabled
        assert!(filter.should_include_event(&event));
    }

    #[test]
    fn test_filter_events_batch() {
        let mut config = FilteringConfig::default();
        config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
        config.token_addresses = vec!["0xA0b86a33E6441b8c4C8C3C8C3C8C3C8C3C8C3C8C".to_string()]; // WETH
        config.enabled = true;
        
        let filter = PoolFilter::new(config);
        
        let pool_address = H160([0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90]);
        
        // Use a token address that's in the default config (WETH)
        let token_address = H160([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8c, 0x4C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C, 0x3C, 0x8C]);
        
        let events = vec![
            create_test_swap_event(
                Some(pool_address),
                token_address,
                token_address,
                "Uniswap V2",
                H160([3; 20]),
                Some(H160([4; 20])),
            ),
            create_test_swap_event(
                Some(H160([0xFF; 20])), // Non-matching pool
                token_address,
                token_address,
                "Uniswap V2",
                H160([3; 20]),
                Some(H160([4; 20])),
            ),
            create_test_swap_event(
                Some(pool_address),
                token_address,
                token_address,
                "Uniswap V2",
                H160([3; 20]),
                Some(H160([4; 20])),
            ),
        ];
        
        let filtered = filter.filter_events(&events);
        assert_eq!(filtered.len(), 2); // Should keep 2 out of 3 events
    }

    #[test]
    fn test_wei_to_eth_conversion() {
        let config = FilteringConfig::default();
        let filter = PoolFilter::new(config);
        
        let one_eth_wei = 1_000_000_000_000_000_000u128;
        let half_eth_wei = 500_000_000_000_000_000u128;
        
        assert_eq!(filter.wei_to_eth(one_eth_wei), 1.0);
        assert_eq!(filter.wei_to_eth(half_eth_wei), 0.5);
        assert_eq!(filter.wei_to_eth(0), 0.0);
    }

    #[test]
    fn test_config_update() {
        let config = FilteringConfig::default();
        let mut filter = PoolFilter::new(config);
        
        let initial_stats = filter.get_filter_stats();
        let initial_pool_count = initial_stats.total_pools;
        
        let mut new_config = FilteringConfig::default();
        new_config.pool_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
        new_config.token_addresses = vec!["0x1234567890123456789012345678901234567890".to_string()];
        new_config.include_protocols = vec!["Uniswap V2".to_string()]; // Override protocols too
        
        filter.update_config(new_config);
        
        let updated_stats = filter.get_filter_stats();
        assert_eq!(updated_stats.total_pools, 1);
        assert_eq!(updated_stats.total_tokens, 1);
        
        // Verify that pool count changed
        assert_ne!(updated_stats.total_pools, initial_pool_count);
    }

    #[test]
    fn test_token_filtering() {
        let config = FilteringConfig::default();
        let filter = PoolFilter::new(config);
        
        let test_token = H160([1; 20]);
        
        // Test that default config includes some tokens
        assert!(filter.token_addresses.len() > 0);
        
        // Test adding and checking a token
        let mut filter = PoolFilter::new(FilteringConfig::default());
        filter.add_pool_address(test_token); // Reusing add_pool_address for testing
        
        // Note: This test is limited since we don't have a public add_token method
        // In a real implementation, you might want to add such a method
    }
} 