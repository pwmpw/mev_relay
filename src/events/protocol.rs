use crate::shared::types::H160;
use crate::shared::constants::protocols;
use std::collections::HashMap;

/// DEX protocol information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocol {
    pub name: String,
    pub version: String,
    pub router_address: H160,
    pub factory_address: H160,
    pub swap_signatures: Vec<String>,
    pub fee_tiers: Vec<u32>,
}

/// Protocol registry for managing known DEX protocols
#[derive(Debug)]
pub struct ProtocolRegistry {
    protocols: HashMap<H160, Protocol>,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            protocols: HashMap::new(),
        };
        registry.register_default_protocols();
        registry
    }

    /// Register a new protocol
    pub fn register(&mut self, protocol: Protocol) {
        self.protocols.insert(protocol.router_address, protocol);
    }

    /// Get protocol by router address
    pub fn get_by_router(&self, address: &H160) -> Option<&Protocol> {
        self.protocols.get(address)
    }

    /// Get protocol by factory address
    pub fn get_by_factory(&self, address: &H160) -> Option<&Protocol> {
        self.protocols.values().find(|p| p.factory_address == *address)
    }

    /// Check if an address is a known protocol router
    pub fn is_known_router(&self, address: &H160) -> bool {
        self.protocols.contains_key(address)
    }

    /// Check if an address is a known protocol factory
    pub fn is_known_factory(&self, address: &H160) -> bool {
        self.protocols.values().any(|p| p.factory_address == *address)
    }

    /// Get all registered protocols
    pub fn get_all(&self) -> Vec<&Protocol> {
        self.protocols.values().collect()
    }

    /// Detect protocol from transaction data
    pub fn detect_from_transaction(&self, to_address: &H160, input_data: &str) -> Option<&Protocol> {
        if let Some(protocol) = self.get_by_router(to_address) {
            if self.is_swap_transaction(input_data, protocol) {
                return Some(protocol);
            }
        }
        None
    }

    /// Check if transaction data matches protocol's swap signature
    fn is_swap_transaction(&self, input_data: &str, protocol: &Protocol) -> bool {
        if input_data.len() < 10 {
            return false;
        }

        let method_signature = &input_data[..10];
        protocol.swap_signatures.contains(&method_signature.to_string())
    }

    /// Register default protocols
    fn register_default_protocols(&mut self) {
        // Uniswap V2
        self.register(Protocol {
            name: "Uniswap".to_string(),
            version: "V2".to_string(),
            router_address: H160::from(protocols::UNISWAP_V2_ROUTER),
            factory_address: H160::from(protocols::UNISWAP_V2_FACTORY),
            swap_signatures: vec![
                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".to_string(), // Swap event
                "0x38ed173900000000000000000000000000000000000000000000000000000000".to_string(), // swapExactTokensForTokens
                "0x7ff36ab500000000000000000000000000000000000000000000000000000000".to_string(), // swapExactETHForTokens
                "0x18cbafe500000000000000000000000000000000000000000000000000000000".to_string(), // swapExactTokensForETH
            ],
            fee_tiers: vec![3000], // 0.3%
        });

        // Uniswap V3
        self.register(Protocol {
            name: "Uniswap".to_string(),
            version: "V3".to_string(),
            router_address: H160::from(protocols::UNISWAP_V3_ROUTER),
            factory_address: H160::from(protocols::UNISWAP_V3_FACTORY),
            swap_signatures: vec![
                "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e3fb1d3e1fe".to_string(), // Swap event
                "0x414bf389000000000000000000000000000000000000000000000000000000000".to_string(), // exactInputSingle
                "0x5c11d795000000000000000000000000000000000000000000000000000000000".to_string(), // exactInput
                "0xdb3e219800000000000000000000000000000000000000000000000000000000".to_string(), // exactOutputSingle
            ],
            fee_tiers: vec![100, 500, 3000, 10000], // 0.01%, 0.05%, 0.3%, 1%
        });

        // SushiSwap
        self.register(Protocol {
            name: "SushiSwap".to_string(),
            version: "V2".to_string(),
            router_address: H160::from(protocols::SUSHISWAP_ROUTER),
            factory_address: H160::from(protocols::SUSHISWAP_FACTORY),
            swap_signatures: vec![
                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".to_string(), // Swap event
                "0x38ed173900000000000000000000000000000000000000000000000000000000".to_string(), // swapExactTokensForTokens
                "0x7ff36ab500000000000000000000000000000000000000000000000000000000".to_string(), // swapExactETHForTokens
                "0x18cbafe500000000000000000000000000000000000000000000000000000000".to_string(), // swapExactTokensForETH
            ],
            fee_tiers: vec![3000], // 0.3%
        });
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol detection service
pub struct ProtocolDetectionService {
    registry: ProtocolRegistry,
}

impl ProtocolDetectionService {
    pub fn new() -> Self {
        Self {
            registry: ProtocolRegistry::new(),
        }
    }

    /// Detect protocol from transaction
    pub fn detect_protocol(&self, to_address: &H160, input_data: &str) -> Option<&Protocol> {
        self.registry.detect_from_transaction(to_address, input_data)
    }

    /// Get protocol by router address
    pub fn get_protocol(&self, router_address: &H160) -> Option<&Protocol> {
        self.registry.get_by_router(router_address)
    }

    /// Check if address is a known protocol
    pub fn is_known_protocol(&self, address: &H160) -> bool {
        self.registry.is_known_router(address) || self.registry.is_known_factory(address)
    }

    /// Get all known protocols
    pub fn get_all_protocols(&self) -> Vec<&Protocol> {
        self.registry.get_all()
    }
}

impl Default for ProtocolDetectionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::H160;

    #[test]
    fn test_protocol_registry() {
        let registry = ProtocolRegistry::new();
        
        // Test known protocols
        assert!(registry.is_known_router(&H160::from(protocols::UNISWAP_V2_ROUTER)));
        assert!(registry.is_known_router(&H160::from(protocols::UNISWAP_V3_ROUTER)));
        assert!(registry.is_known_router(&H160::from(protocols::SUSHISWAP_ROUTER)));
        
        // Test unknown protocol
        assert!(!registry.is_known_router(&H160([0; 20])));
    }

    #[test]
    fn test_protocol_detection() {
        let service = ProtocolDetectionService::new();
        
        // Test Uniswap V2 detection
        let uniswap_v2_router = H160::from(protocols::UNISWAP_V2_ROUTER);
        let swap_signature = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
        
        let detected = service.detect_protocol(&uniswap_v2_router, swap_signature);
        assert!(detected.is_some());
        
        if let Some(protocol) = detected {
            assert_eq!(protocol.name, "Uniswap");
            assert_eq!(protocol.version, "V2");
        }
    }

    #[test]
    fn test_protocol_fee_tiers() {
        let registry = ProtocolRegistry::new();
        
        // Test Uniswap V3 fee tiers
        let uniswap_v3 = registry.get_by_router(&H160::from(protocols::UNISWAP_V3_ROUTER)).unwrap();
        assert_eq!(uniswap_v3.fee_tiers, vec![100, 500, 3000, 10000]);
        
        // Test Uniswap V2 fee tiers
        let uniswap_v2 = registry.get_by_router(&H160::from(protocols::UNISWAP_V2_ROUTER)).unwrap();
        assert_eq!(uniswap_v2.fee_tiers, vec![3000]);
    }
} 