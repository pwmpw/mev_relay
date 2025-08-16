// Domain constants for MEV Relay

pub mod ethereum {
    // Ethereum network constants
    pub const MAINNET_CHAIN_ID: u64 = 1;
    pub const GOERLI_CHAIN_ID: u64 = 5;
    pub const SEPOLIA_CHAIN_ID: u64 = 11155111;
    
    // Gas constants
    pub const DEFAULT_GAS_LIMIT: u64 = 21000;
    pub const MAX_GAS_PRICE: u128 = 100_000_000_000_000; // 100 Gwei
    pub const MIN_GAS_PRICE: u128 = 1_000_000_000; // 1 Gwei
    
    // Block constants
    pub const MAX_BLOCK_RANGE: u64 = 10000;
    pub const DEFAULT_BLOCK_TIMEOUT: u64 = 30000; // 30 seconds
}

pub mod protocols {
    // DEX protocol addresses
    pub const UNISWAP_V2_ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
    pub const UNISWAP_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    pub const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
    
    // Protocol factory addresses
    pub const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
    pub const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
    pub const SUSHISWAP_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac";
}

pub mod redis {
    // Redis configuration constants
    pub const DEFAULT_CHANNEL: &str = "mev_swaps";
    pub const DEFAULT_POOL_SIZE: usize = 10;
    pub const DEFAULT_CONNECTION_TIMEOUT: u64 = 5000;
    pub const DEFAULT_READ_TIMEOUT: u64 = 3000;
}

pub mod monitoring {
    // Monitoring constants
    pub const DEFAULT_METRICS_PORT: u16 = 9090;
    pub const DEFAULT_METRICS_HOST: &str = "127.0.0.1";
    pub const HEALTH_CHECK_INTERVAL: u64 = 30; // seconds
    pub const STATS_COLLECTION_INTERVAL: u64 = 60; // seconds
}

pub mod mempool {
    // Mempool monitoring constants
    pub const DEFAULT_POLL_INTERVAL: u64 = 100; // milliseconds
    pub const DEFAULT_BATCH_SIZE: usize = 100;
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 50;
    pub const DEFAULT_REQUEST_TIMEOUT: u64 = 10000; // 10 seconds
}

pub mod flashbots {
    // Flashbots constants
    pub const DEFAULT_RPC_URL: &str = "https://relay.flashbots.net";
    pub const DEFAULT_POLL_INTERVAL: u64 = 1000; // 1 second
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 20;
    pub const DEFAULT_REQUEST_TIMEOUT: u64 = 15000; // 15 seconds
} 