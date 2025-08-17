use serde::{Deserialize, Serialize};

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

impl Default for FilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_addresses: vec![
                "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(), // Uniswap V2 Router
                "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string(), // SushiSwap Router
            ],
            token_addresses: vec![
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                "0xA0b86a33E6441b8B4b0C3d2C2C0C3d2C2C0C3d2C".to_string(), // USDC
            ],
            min_liquidity_eth: 100.0,
            min_volume_24h_eth: 1000.0,
            exclude_contracts: vec![
                "0x0000000000000000000000000000000000000000".to_string(),
            ],
            include_protocols: vec![
                "Uniswap V2".to_string(),
                "SushiSwap".to_string(),
            ],
        }
    }
} 