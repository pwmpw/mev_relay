use crate::infrastructure::config::SubgraphConfig;
use crate::infrastructure::metrics;
use crate::monitoring::domain::{
    TokenMetadata, PoolMetadata, SubgraphQueryResult, SubgraphCacheStats, ServiceStatus
};
use crate::shared::types::H160;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

/// Subgraph service trait
#[async_trait]
pub trait SubgraphService: Send + Sync {
    /// Get token metadata by address
    async fn get_token_metadata(&self, address: H160) -> Result<Option<TokenMetadata>>;
    
    /// Get pool metadata by address
    async fn get_pool_metadata(&self, address: H160) -> Result<Option<PoolMetadata>>;
    
    /// Refresh cache with latest data from subgraph
    async fn refresh_cache(&mut self) -> Result<()>;
    
    /// Get cache statistics
    fn get_cache_stats(&self) -> SubgraphCacheStats;
    
    /// Get service status
    fn get_status(&self) -> ServiceStatus;
}

/// Subgraph service implementation
pub struct SubgraphServiceImpl {
    config: SubgraphConfig,
    client: Client,
    token_cache: Arc<RwLock<HashMap<H160, CachedTokenMetadata>>>,
    pool_cache: Arc<RwLock<HashMap<H160, CachedPoolMetadata>>>,
    stats: Arc<RwLock<SubgraphCacheStats>>,
    status: ServiceStatus,
}

/// Cached token metadata with TTL
#[derive(Debug, Clone)]
struct CachedTokenMetadata {
    metadata: TokenMetadata,
    cached_at: u64,
}

/// Cached pool metadata with TTL
#[derive(Debug, Clone)]
struct CachedPoolMetadata {
    metadata: PoolMetadata,
    cached_at: u64,
}

impl SubgraphServiceImpl {
    pub fn new(config: SubgraphConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.request_timeout))
            .build()
            .unwrap_or_default();

        Self {
            config,
            client,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SubgraphCacheStats::default())),
            status: ServiceStatus::Unknown,
        }
    }

    /// Start the subgraph service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting subgraph service");
        self.status = ServiceStatus::Healthy;
        
        // Initial cache refresh
        self.refresh_cache().await?;
        
        // Start background refresh loop
        let config = self.config.clone();
        let token_cache = Arc::clone(&self.token_cache);
        let pool_cache = Arc::clone(&self.pool_cache);
        let stats = Arc::clone(&self.stats);
        
        tokio::spawn(async move {
            Self::background_refresh_loop(config, token_cache, pool_cache, stats).await;
        });

        Ok(())
    }

    /// Stop the subgraph service
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping subgraph service");
        self.status = ServiceStatus::Unknown;
        Ok(())
    }

    /// Background refresh loop
    async fn background_refresh_loop(
        config: SubgraphConfig,
        token_cache: Arc<RwLock<HashMap<H160, CachedTokenMetadata>>>,
        pool_cache: Arc<RwLock<HashMap<H160, CachedPoolMetadata>>>,
        stats: Arc<RwLock<SubgraphCacheStats>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(config.poll_interval));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = Self::refresh_cache_impl(&config, &token_cache, &pool_cache, &stats).await {
                error!("Failed to refresh subgraph cache: {}", e);
                let mut stats_guard = stats.write().await;
                stats_guard.errors += 1;
            }
        }
    }

    /// Implementation of cache refresh
    async fn refresh_cache_impl(
        config: &SubgraphConfig,
        token_cache: &Arc<RwLock<HashMap<H160, CachedTokenMetadata>>>,
        pool_cache: &Arc<RwLock<HashMap<H160, CachedPoolMetadata>>>,
        stats: &Arc<RwLock<SubgraphCacheStats>>,
    ) -> Result<()> {
        debug!("Refreshing subgraph cache");
        
        let start_time = std::time::Instant::now();
        
        // Fetch popular tokens
        let popular_tokens = Self::fetch_popular_tokens(config).await?;
        
        // Update token cache
        {
            let mut cache = token_cache.write().await;
            for token in popular_tokens {
                cache.insert(
                    token.address,
                    CachedTokenMetadata {
                        metadata: token,
                        cached_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    },
                );
            }
        }

        // Fetch popular pools
        let popular_pools = Self::fetch_popular_pools(config).await?;
        
        // Update pool cache
        {
            let mut cache = pool_cache.write().await;
            for pool in popular_pools {
                cache.insert(
                    pool.address,
                    CachedPoolMetadata {
                        metadata: pool,
                        cached_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    },
                );
            }
        }

        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.last_refresh = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            stats_guard.refresh_count += 1;
            stats_guard.tokens_cached = token_cache.read().await.len() as u64;
            stats_guard.pools_cached = pool_cache.read().await.len() as u64;
        }

        // Record metrics
        let duration = start_time.elapsed().as_secs_f64();
        metrics::record_subgraph_refresh();
        metrics::record_subgraph_query_duration(duration);
        metrics::record_subgraph_tokens_cached(token_cache.read().await.len() as f64);
        metrics::record_subgraph_pools_cached(pool_cache.read().await.len() as f64);
        metrics::record_subgraph_last_refresh(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as f64
        );

        info!("Subgraph cache refreshed successfully");
        Ok(())
    }

    /// Fetch popular tokens from subgraph
    async fn fetch_popular_tokens(config: &SubgraphConfig) -> Result<Vec<TokenMetadata>> {
        // This would be a real GraphQL query to the subgraph
        // For now, return mock data
        Ok(vec![
            TokenMetadata {
                address: H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]),
                symbol: "WETH".to_string(),
                name: "Wrapped Ether".to_string(),
                decimals: 18,
                total_supply: Some("1000000000000000000000000".to_string()),
                volume_24h: Some(1000000.0),
                liquidity: Some(500000.0),
                price_usd: Some(2000.0),
                last_updated: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
            TokenMetadata {
                address: H160::from([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8B, 0x4b, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C]),
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                total_supply: Some("1000000000000000000000000".to_string()),
                volume_24h: Some(500000.0),
                liquidity: Some(300000.0),
                price_usd: Some(1.0),
                last_updated: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        ])
    }

    /// Fetch popular pools from subgraph
    async fn fetch_popular_pools(config: &SubgraphConfig) -> Result<Vec<PoolMetadata>> {
        // This would be a real GraphQL query to the subgraph
        // For now, return mock data
        let weth = TokenMetadata {
            address: H160::from([0xC0, 0x2a, 0xA3, 0x9b, 0x22, 0x3F, 0xE8, 0xD0, 0xA0, 0xe5, 0xC4, 0xF2, 0x7e, 0xAD, 0x90, 0x83, 0xC7, 0x56, 0xCc, 0x2]),
            symbol: "WETH".to_string(),
            name: "Wrapped Ether".to_string(),
            decimals: 18,
            total_supply: Some("1000000000000000000000000".to_string()),
            volume_24h: Some(1000000.0),
            liquidity: Some(500000.0),
            price_usd: Some(2000.0),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let usdc = TokenMetadata {
            address: H160::from([0xA0, 0xb8, 0x6a, 0x33, 0xE6, 0x44, 0x1b, 0x8B, 0x4b, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C, 0x2C, 0x0C, 0x3d, 0x2C]),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            total_supply: Some("1000000000000000000000000".to_string()),
            volume_24h: Some(500000.0),
            liquidity: Some(300000.0),
            price_usd: Some(1.0),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        Ok(vec![
            PoolMetadata {
                address: H160::from([0xB4, 0xe1, 0x6d, 0x01, 0x68, 0xe5, 0x2d, 0x35, 0xCa, 0xCD, 0x2c, 0x61, 0x85, 0xb4, 0x42, 0x81, 0xEc, 0x28, 0xC9, 0xDc]),
                token0: weth.clone(),
                token1: usdc.clone(),
                fee_tier: 3000,
                protocol: "Uniswap V2".to_string(),
                liquidity: 1000000.0,
                volume_24h: 500000.0,
                tvl_usd: 2000000.0,
                last_updated: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        ])
    }

    /// Check if cached item is expired
    fn is_expired(&self, cached_at: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now - cached_at > self.config.cache_ttl_seconds
    }
}

#[async_trait]
impl SubgraphService for SubgraphServiceImpl {
    async fn get_token_metadata(&self, address: H160) -> Result<Option<TokenMetadata>> {
        let cache = self.token_cache.read().await;
        
        if let Some(cached) = cache.get(&address) {
            if !self.is_expired(cached.cached_at) {
                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                metrics::record_subgraph_cache_hit();
                return Ok(Some(cached.metadata.clone()));
            }
        }

        // Cache miss
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
            metrics::record_subgraph_cache_miss();
        }

        // Try to fetch from subgraph
        let tokens = Self::fetch_popular_tokens(&self.config).await?;
        if let Some(token) = tokens.into_iter().find(|t| t.address == address) {
            // Cache the result
            let mut cache = self.token_cache.write().await;
            cache.insert(
                address,
                CachedTokenMetadata {
                    metadata: token.clone(),
                    cached_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            );
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    async fn get_pool_metadata(&self, address: H160) -> Result<Option<PoolMetadata>> {
        let cache = self.pool_cache.read().await;
        
        if let Some(cached) = cache.get(&address) {
            if !self.is_expired(cached.cached_at) {
                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                metrics::record_subgraph_cache_hit();
                return Ok(Some(cached.metadata.clone()));
            }
        }

        // Cache miss
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
            metrics::record_subgraph_cache_miss();
        }

        // Try to fetch from subgraph
        let pools = Self::fetch_popular_pools(&self.config).await?;
        if let Some(pool) = pools.into_iter().find(|p| p.address == address) {
            // Cache the result
            let mut cache = self.pool_cache.write().await;
            cache.insert(
                address,
                CachedPoolMetadata {
                    metadata: pool.clone(),
                    cached_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            );
            Ok(Some(pool))
        } else {
            Ok(None)
        }
    }

    async fn refresh_cache(&mut self) -> Result<()> {
        Self::refresh_cache_impl(
            &self.config,
            &self.token_cache,
            &self.pool_cache,
            &self.stats,
        ).await
    }

    fn get_cache_stats(&self) -> SubgraphCacheStats {
        // This is a simplified version - in practice you'd want to clone the stats
        // For now, return default stats
        SubgraphCacheStats::default()
    }

    fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::SubgraphConfig;

    #[tokio::test]
    async fn test_subgraph_service_new() {
        let config = SubgraphConfig::default();
        let service = SubgraphServiceImpl::new(config);
        
        assert_eq!(service.get_status(), ServiceStatus::Unknown);
        assert_eq!(service.get_cache_stats().tokens_cached, 0);
    }

    #[tokio::test]
    async fn test_subgraph_service_start() {
        let config = SubgraphConfig::default();
        let mut service = SubgraphServiceImpl::new(config);
        
        let result = service.start().await;
        assert!(result.is_ok());
        assert_eq!(service.get_status(), ServiceStatus::Healthy);
    }

    #[tokio::test]
    async fn test_subgraph_service_stop() {
        let config = SubgraphConfig::default();
        let mut service = SubgraphServiceImpl::new(config);
        
        // Start first
        service.start().await.unwrap();
        assert_eq!(service.get_status(), ServiceStatus::Healthy);
        
        // Then stop
        let result = service.stop().await;
        assert!(result.is_ok());
        assert_eq!(service.get_status(), ServiceStatus::Unknown);
    }

    #[tokio::test]
    async fn test_token_metadata_serialization() {
        let token = TokenMetadata {
            address: H160::from([0x01; 20]),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            decimals: 18,
            total_supply: Some("1000000".to_string()),
            volume_24h: Some(1000.0),
            liquidity: Some(500.0),
            price_usd: Some(1.0),
            last_updated: 1234567890,
        };

        let serialized = serde_json::to_string(&token);
        assert!(serialized.is_ok());

        let deserialized: Result<TokenMetadata, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[tokio::test]
    async fn test_pool_metadata_serialization() {
        let token0 = TokenMetadata {
            address: H160::from([0x01; 20]),
            symbol: "TOKEN0".to_string(),
            name: "Token 0".to_string(),
            decimals: 18,
            total_supply: None,
            volume_24h: None,
            liquidity: None,
            price_usd: None,
            last_updated: 1234567890,
        };

        let token1 = TokenMetadata {
            address: H160::from([0x02; 20]),
            symbol: "TOKEN1".to_string(),
            name: "Token 1".to_string(),
            decimals: 6,
            total_supply: None,
            volume_24h: None,
            liquidity: None,
            price_usd: None,
            last_updated: 1234567890,
        };

        let pool = PoolMetadata {
            address: H160::from([0x03; 20]),
            token0,
            token1,
            fee_tier: 3000,
            protocol: "Uniswap V2".to_string(),
            liquidity: 1000000.0,
            volume_24h: 500000.0,
            tvl_usd: 2000000.0,
            last_updated: 1234567890,
        };

        let serialized = serde_json::to_string(&pool);
        assert!(serialized.is_ok());

        let deserialized: Result<PoolMetadata, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = SubgraphConfig {
            cache_ttl_seconds: 1,
            ..Default::default()
        };
        
        let service = SubgraphServiceImpl::new(config);
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Test not expired
        assert!(!service.is_expired(now));
        
        // Test expired
        assert!(service.is_expired(now - 2));
    }

    #[tokio::test]
    async fn test_fetch_popular_tokens() {
        let config = SubgraphConfig::default();
        let tokens = SubgraphServiceImpl::fetch_popular_tokens(&config).await.unwrap();
        
        assert!(!tokens.is_empty());
        assert!(tokens.len() >= 2); // Should have at least WETH and USDC
        
        // Check WETH
        let weth = tokens.iter().find(|t| t.symbol == "WETH").unwrap();
        assert_eq!(weth.name, "Wrapped Ether");
        assert_eq!(weth.decimals, 18);
        assert!(weth.volume_24h.is_some());
        assert!(weth.liquidity.is_some());
        assert!(weth.price_usd.is_some());
        
        // Check USDC
        let usdc = tokens.iter().find(|t| t.symbol == "USDC").unwrap();
        assert_eq!(usdc.name, "USD Coin");
        assert_eq!(usdc.decimals, 6);
    }

    #[tokio::test]
    async fn test_fetch_popular_pools() {
        let config = SubgraphConfig::default();
        let pools = SubgraphServiceImpl::fetch_popular_pools(&config).await.unwrap();
        
        assert!(!pools.is_empty());
        assert!(pools.len() >= 1); // Should have at least one pool
        
        let pool = &pools[0];
        assert_eq!(pool.protocol, "Uniswap V2");
        assert_eq!(pool.fee_tier, 3000);
        assert_eq!(pool.token0.symbol, "WETH");
        assert_eq!(pool.token1.symbol, "USDC");
        assert!(pool.liquidity > 0.0);
        assert!(pool.volume_24h > 0.0);
        assert!(pool.tvl_usd > 0.0);
    }

    #[tokio::test]
    async fn test_subgraph_query_result_serialization() {
        let query_result = SubgraphQueryResult {
            data: Some("test_data".to_string()),
            errors: None,
        };

        let serialized = serde_json::to_string(&query_result);
        assert!(serialized.is_ok());

        let deserialized: Result<SubgraphQueryResult<String>, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[tokio::test]
    async fn test_subgraph_error_serialization() {
        let error = SubgraphError {
            message: "Test error".to_string(),
            locations: Some(vec![SubgraphLocation { line: 1, column: 1 }]),
            path: Some(vec!["query".to_string(), "tokens".to_string()]),
        };

        let serialized = serde_json::to_string(&error);
        assert!(serialized.is_ok());

        let deserialized: Result<SubgraphError, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }

    #[tokio::test]
    async fn test_subgraph_cache_stats() {
        let mut stats = SubgraphCacheStats::default();
        
        assert_eq!(stats.tokens_cached, 0);
        assert_eq!(stats.pools_cached, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.refresh_count, 0);
        assert_eq!(stats.errors, 0);
        
        // Test incrementing
        stats.tokens_cached = 10;
        stats.pools_cached = 5;
        stats.cache_hits = 100;
        stats.cache_misses = 20;
        stats.refresh_count = 15;
        stats.errors = 2;
        
        assert_eq!(stats.tokens_cached, 10);
        assert_eq!(stats.pools_cached, 5);
        assert_eq!(stats.cache_hits, 100);
        assert_eq!(stats.cache_misses, 20);
        assert_eq!(stats.refresh_count, 15);
        assert_eq!(stats.errors, 2);
    }

    #[tokio::test]
    async fn test_service_status_clone() {
        let status = ServiceStatus::Healthy;
        let cloned = status.clone();
        
        assert_eq!(status, cloned);
        assert_eq!(format!("{:?}", status), "Healthy");
    }

    #[tokio::test]
    async fn test_subgraph_config_default() {
        let config = SubgraphConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.url, "http://localhost:8000");
        assert_eq!(config.poll_interval, 60);
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.request_timeout, 10000);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }

    #[tokio::test]
    async fn test_subgraph_config_custom() {
        let config = SubgraphConfig {
            enabled: false,
            url: "https://custom.example.com".to_string(),
            poll_interval: 120,
            cache_ttl_seconds: 600,
            max_concurrent_requests: 25,
            request_timeout: 20000,
            retry_attempts: 5,
            retry_delay_ms: 2000,
        };
        
        assert!(!config.enabled);
        assert_eq!(config.url, "https://custom.example.com");
        assert_eq!(config.poll_interval, 120);
        assert_eq!(config.cache_ttl_seconds, 600);
        assert_eq!(config.max_concurrent_requests, 25);
        assert_eq!(config.request_timeout, 20000);
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.retry_delay_ms, 2000);
    }
} 