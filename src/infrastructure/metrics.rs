use crate::infrastructure::config::MetricsConfig;
use anyhow::Result;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use tracing::{error, info};

pub struct Metrics {
    config: MetricsConfig,
    handle: Option<PrometheusHandle>,
}

impl Metrics {
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            handle: None,
        }
    }

    pub fn init(config: &MetricsConfig) -> Result<()> {
        if !config.enabled {
            info!("Metrics collection disabled");
            return Ok(());
        }

        info!("Initializing metrics collection on {}:{}", config.host, config.port);

        let addr: SocketAddr = format!("{}:{}", config.host, config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid metrics address: {}", e))?;

        let handle = PrometheusBuilder::new()
            .with_http_listener(addr)
            .install()
            .map_err(|e| anyhow::anyhow!("Failed to install Prometheus metrics: {}", e))?;

        info!("Metrics collection started successfully on {}", addr);

        // Initialize default metrics
        Self::init_default_metrics();

        Ok(())
    }

    fn init_default_metrics() {
        // Initialize counters to 0
        counter!("mev_relay_events_total", 0);
        counter!("mev_relay_events_mempool", 0);
        counter!("mev_relay_events_flashbots", 0);
        counter!("mev_relay_events_block", 0);
        
        counter!("mev_relay_errors_total", 0);
        counter!("mev_relay_redis_errors", 0);
        counter!("mev_relay_rpc_errors", 0);
        
        // Initialize gauges
        gauge!("mev_relay_uptime_seconds", 0.0);
        gauge!("mev_relay_active_connections", 0.0);
        gauge!("mev_relay_pending_events", 0.0);
        
        // Initialize histograms
        histogram!("mev_relay_event_processing_duration_seconds", 0.0);
        histogram!("mev_relay_redis_publish_duration_seconds", 0.0);
        histogram!("mev_relay_rpc_request_duration_seconds", 0.0);
    }

    pub fn increment_events_processed(source: &str) {
        counter!("mev_relay_events_total", 1);
        
        match source {
            "Mempool" => counter!("mev_relay_events_mempool", 1),
            "Flashbots" => counter!("mev_relay_events_flashbots", 1),
            "Block" => counter!("mev_relay_events_block", 1),
            _ => counter!("mev_relay_events_unknown", 1),
        }
    }

    pub fn increment_errors(error_type: &str) {
        counter!("mev_relay_errors_total", 1);
        
        match error_type {
            "redis" => counter!("mev_relay_redis_errors", 1),
            "rpc" => counter!("mev_relay_rpc_errors", 1),
            _ => counter!("mev_relay_errors_unknown", 1),
        }
    }

    pub fn set_uptime(seconds: f64) {
        gauge!("mev_relay_uptime_seconds", seconds);
    }

    pub fn set_active_connections(count: f64) {
        gauge!("mev_relay_active_connections", count);
    }

    pub fn set_pending_events(count: f64) {
        gauge!("mev_relay_pending_events", count);
    }

    pub fn record_event_processing_duration(duration: f64) {
        histogram!("mev_relay_event_processing_duration_seconds", duration);
    }

    pub fn record_redis_publish_duration(duration: f64) {
        histogram!("mev_relay_redis_publish_duration_seconds", duration);
    }

    pub fn record_rpc_request_duration(duration: f64) {
        histogram!("mev_relay_rpc_request_duration_seconds", duration);
    }

    pub fn record_gas_price(gas_price: u128) {
        // Convert to Gwei for better readability
        let gas_price_gwei = gas_price as f64 / 1_000_000_000.0;
        histogram!("mev_relay_gas_price_gwei", gas_price_gwei);
    }

    pub fn record_transaction_value(value: u128) {
        // Convert to ETH for better readability
        let value_eth = value as f64 / 1_000_000_000_000_000_000.0;
        histogram!("mev_relay_transaction_value_eth", value_eth);
    }

    pub fn record_protocol_events(protocol: &str, count: u64) {
        counter!("mev_relay_protocol_events_total", count, "protocol" => protocol.to_string());
    }

    pub fn record_pool_events(pool_address: &str, count: u64) {
        counter!("mev_relay_pool_events_total", count, "pool_address" => pool_address.to_string());
    }

    pub fn record_block_processing_duration(block_number: u64, duration: f64) {
        histogram!("mev_relay_block_processing_duration_seconds", duration, "block_number" => block_number.to_string());
    }

    pub fn record_mempool_size(size: u64) {
        gauge!("mev_relay_mempool_size", size as f64);
    }

    pub fn record_flashbots_bundle_size(bundle_size: u64) {
        histogram!("mev_relay_flashbots_bundle_size", bundle_size as f64);
    }

    pub fn record_redis_connection_pool_size(pool_size: u64) {
        gauge!("mev_relay_redis_connection_pool_size", pool_size as f64);
    }

    pub fn record_redis_connection_pool_available(available: u64) {
        gauge!("mev_relay_redis_connection_pool_available", available as f64);
    }

    pub fn record_redis_connection_pool_in_use(in_use: u64) {
        gauge!("mev_relay_redis_connection_pool_in_use", in_use as f64);
    }

    pub fn record_ethereum_block_height(block_height: u64) {
        gauge!("mev_relay_ethereum_block_height", block_height as f64);
    }

    pub fn record_ethereum_block_timestamp(timestamp: u64) {
        gauge!("mev_relay_ethereum_block_timestamp", timestamp as f64);
    }

    pub fn record_ethereum_gas_price(gas_price: u128) {
        let gas_price_gwei = gas_price as f64 / 1_000_000_000.0;
        gauge!("mev_relay_ethereum_gas_price_gwei", gas_price_gwei);
    }

    pub fn record_ethereum_pending_transactions(count: u64) {
        gauge!("mev_relay_ethereum_pending_transactions", count as f64);
    }

    pub fn record_ethereum_queued_transactions(count: u64) {
        gauge!("mev_relay_ethereum_queued_transactions", count as f64);
    }
}

/// Initialize all metrics
pub fn init_metrics() {
    info!("Initializing metrics");
    
    // Initialize counters to 0
    counter!("mev_relay_events_total", 0);
    counter!("mev_relay_events_by_source", 0);
    counter!("mev_relay_events_by_protocol", 0);
    counter!("mev_relay_redis_publish_total", 0);
    counter!("mev_relay_redis_publish_errors", 0);
    counter!("mev_relay_redis_subscribe_total", 0);
    counter!("mev_relay_redis_subscribe_errors", 0);
    counter!("mev_relay_mempool_blocks_processed", 0);
    counter!("mev_relay_flashbots_blocks_processed", 0);
    counter!("mev_relay_missed_blocks_total", 0);
    counter!("mev_relay_subgraph_cache_hits", 0);
    counter!("mev_relay_subgraph_cache_misses", 0);
    counter!("mev_relay_subgraph_refresh_total", 0);
    counter!("mev_relay_subgraph_errors", 0);
    
    // Initialize gauges
    gauge!("mev_relay_active_connections", 0.0);
    gauge!("mev_relay_pending_events", 0.0);
    gauge!("mev_relay_current_block_height", 0.0);
    gauge!("mev_relay_current_gas_price", 0.0);
    gauge!("mev_relay_subgraph_tokens_cached", 0.0);
    gauge!("mev_relay_subgraph_pools_cached", 0.0);
    gauge!("mev_relay_subgraph_last_refresh", 0.0);
    
    // Initialize histograms
    histogram!("mev_relay_event_processing_duration_seconds", 0.0);
    histogram!("mev_relay_redis_publish_duration_seconds", 0.0);
    histogram!("mev_relay_redis_subscribe_duration_seconds", 0.0);
    histogram!("mev_relay_block_processing_duration_seconds", 0.0);
    histogram!("mev_relay_subgraph_query_duration_seconds", 0.0);
    
    info!("Metrics initialized successfully");
}

/// Record event processing metrics
pub fn record_event_processed(source: &str, protocol: &str, duration: f64) {
    counter!("mev_relay_events_total", 1);
    counter!("mev_relay_events_by_source", 1, "source" => source.to_string());
    counter!("mev_relay_events_by_protocol", 1, "protocol" => protocol.to_string());
    histogram!("mev_relay_event_processing_duration_seconds", duration);
}

/// Record Redis metrics
pub fn record_redis_publish(duration: f64, success: bool) {
    if success {
        counter!("mev_relay_redis_publish_total", 1);
    } else {
        counter!("mev_relay_redis_publish_errors", 1);
    }
    histogram!("mev_relay_redis_publish_duration_seconds", duration);
}

pub fn record_redis_subscribe(duration: f64, success: bool) {
    if success {
        counter!("mev_relay_redis_subscribe_total", 1);
    } else {
        counter!("mev_relay_redis_subscribe_errors", 1);
    }
    histogram!("mev_relay_redis_subscribe_duration_seconds", duration);
}

/// Record mempool metrics
pub fn record_mempool_block_processed() {
    counter!("mev_relay_mempool_blocks_processed", 1);
}

/// Record Flashbots metrics
pub fn record_flashbots_block_processed() {
    counter!("mev_relay_flashbots_blocks_processed", 1);
}

/// Record missed block metrics
pub fn record_missed_block() {
    counter!("mev_relay_missed_blocks_total", 1);
}

/// Record connection metrics
pub fn record_active_connections(count: f64) {
    gauge!("mev_relay_active_connections", count);
}

/// Record pending events metrics
pub fn record_pending_events(count: f64) {
    gauge!("mev_relay_pending_events", count);
}

/// Record blockchain metrics
pub fn record_block_height(height: f64) {
    gauge!("mev_relay_current_block_height", height);
}

pub fn record_gas_price(price: f64) {
    gauge!("mev_relay_current_gas_price", price);
}

/// Record subgraph metrics
pub fn record_subgraph_cache_hit() {
    counter!("mev_relay_subgraph_cache_hits", 1);
}

pub fn record_subgraph_cache_miss() {
    counter!("mev_relay_subgraph_cache_misses", 1);
}

pub fn record_subgraph_refresh() {
    counter!("mev_relay_subgraph_refresh_total", 1);
}

pub fn record_subgraph_error() {
    counter!("mev_relay_subgraph_errors", 1);
}

pub fn record_subgraph_tokens_cached(count: f64) {
    gauge!("mev_relay_subgraph_tokens_cached", count);
}

pub fn record_subgraph_pools_cached(count: f64) {
    gauge!("mev_relay_subgraph_pools_cached", count);
}

pub fn record_subgraph_last_refresh(timestamp: f64) {
    gauge!("mev_relay_subgraph_last_refresh", timestamp);
}

pub fn record_subgraph_query_duration(duration: f64) {
    histogram!("mev_relay_subgraph_query_duration_seconds", duration);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let config = MetricsConfig::default();
        let metrics = Metrics::new(config);
        assert!(metrics.handle.is_none());
    }

    #[test]
    fn test_metrics_functions() {
        // These should not panic
        Metrics::increment_events_processed("Mempool");
        Metrics::increment_errors("redis");
        Metrics::set_uptime(100.0);
        Metrics::set_active_connections(5.0);
        Metrics::record_event_processing_duration(0.1);
        Metrics::record_gas_price(20_000_000_000); // 20 Gwei
        Metrics::record_transaction_value(1_000_000_000_000_000_000); // 1 ETH
    }
} 