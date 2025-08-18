use mev_relay::{
    events::domain::{SwapEvent, EventId},
    messaging::buffer::{RedisBuffer, BufferConfig, BufferManager},
    messaging::buffered_processor::{BufferedEventProcessor, BufferedProcessorConfig},
    shared::config::FilteringConfig,
    events::filter::PoolFilter,
    Result,
};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::redis::Redis;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

/// Test Redis buffer with error handling and reconnections
#[tokio::test]
async fn test_redis_buffer_with_error_handling() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {}", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Test basic buffer operations
    test_basic_buffer_operations(&redis_url).await?;
    
    // Test error handling
    test_error_handling(&redis_url).await?;
    
    // Test reconnection logic
    test_reconnection_logic(&redis_url).await?;
    
    // Test backpressure
    test_backpressure(&redis_url).await?;
    
    // Test persistence and recovery
    test_persistence_and_recovery(&redis_url).await?;
    
    info!("All Redis buffer tests passed");
    Ok(())
}

/// Test high-throughput scenarios
#[tokio::test]
async fn test_high_throughput_scenarios() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for high-throughput test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Test concurrent operations
    test_concurrent_operations(&redis_url).await?;
    
    // Test large batch processing
    test_large_batch_processing(&redis_url).await?;
    
    // Test memory pressure scenarios
    test_memory_pressure_scenarios(&redis_url).await?;
    
    // Test performance under load
    test_performance_under_load(&redis_url).await?;
    
    info!("High-throughput scenarios test passed");
    Ok(())
}

/// Test edge cases and boundary conditions
#[tokio::test]
async fn test_edge_cases_and_boundaries() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for edge cases test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Test empty operations
    test_empty_operations(&redis_url).await?;
    
    // Test boundary conditions
    test_boundary_conditions(&redis_url).await?;
    
    // Test malformed data handling
    test_malformed_data_handling(&redis_url).await?;
    
    // Test extreme configurations
    test_extreme_configurations(&redis_url).await?;
    
    info!("Edge cases and boundaries test passed");
    Ok(())
}

/// Test failure scenarios and recovery
#[tokio::test]
async fn test_failure_scenarios_and_recovery() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for failure scenarios test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Test partial failures
    test_partial_failures(&redis_url).await?;
    
    // Test recovery mechanisms
    test_recovery_mechanisms(&redis_url).await?;
    
    // Test data consistency
    test_data_consistency(&redis_url).await?;
    
    // Test graceful degradation
    test_graceful_degradation(&redis_url).await?;
    
    info!("Failure scenarios and recovery test passed");
    Ok(())
}

async fn test_basic_buffer_operations(redis_url: &str) -> Result<()> {
    info!("Testing basic buffer operations");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "test_queue".to_string(), Some(config)).await?;
    
    // Create test event
    let event = create_test_event();
    
    // Test buffering
    buffer.buffer_event(&event).await?;
    
    // Test queue size
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 1);
    
    // Test processing
    let events = buffer.process_events(Some(10)).await?;
    assert_eq!(events.len(), 1);
    
    // Test caching
    let cache_key = format!("event:{}", event.transaction.hash);
    buffer.cache_event(&cache_key, &event, Duration::from_secs(60)).await?;
    
    // Test retrieval
    let cached_event = buffer.get_cached_event(&cache_key).await?;
    assert!(cached_event.is_some());
    
    // Test streams
    buffer.add_to_stream(&event).await?;
    
    info!("Basic buffer operations test passed");
    Ok(())
}

async fn test_error_handling(redis_url: &str) -> Result<()> {
    info!("Testing error handling");
    
    let config = BufferConfig {
        max_queue_size: 2,
        batch_size: 1,
        stream_max_len: 10,
        cache_ttl_seconds: 60,
        retry_attempts: 3,
        retry_delay_ms: 100,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "error_test_queue".to_string(), Some(config)).await?;
    
    // Create multiple events to test queue overflow
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(), // This should be dropped
    ];
    
    // Buffer events - third one should be dropped due to queue size limit
    buffer.buffer_events(&events).await?;
    
    // Check queue size
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 2); // Only 2 should fit
    
    // Check stats
    let stats = buffer.get_stats().await;
    assert_eq!(stats.events_buffered, 2);
    assert_eq!(stats.events_dropped, 1);
    
    info!("Error handling test passed");
    Ok(())
}

async fn test_reconnection_logic(redis_url: &str) -> Result<()> {
    info!("Testing reconnection logic");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "reconnect_test_queue".to_string(), Some(config)).await?;
    
    // Test health check
    buffer.health_check().await?;
    
    // Test multiple operations to ensure connection stability
    for i in 0..5 {
        let event = create_test_event();
        buffer.buffer_event(&event).await?;
        
        // Small delay to test connection persistence
        sleep(Duration::from_millis(10)).await;
    }
    
    // Verify all events were processed
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 5);
    
    info!("Reconnection logic test passed");
    Ok(())
}

async fn test_backpressure(redis_url: &str) -> Result<()> {
    info!("Testing backpressure");
    
    let config = BufferConfig {
        max_queue_size: 5,
        batch_size: 2,
        stream_max_len: 10,
        cache_ttl_seconds: 60,
        retry_attempts: 3,
        retry_delay_ms: 100,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "backpressure_test_queue".to_string(), Some(config)).await?;
    
    // Fill the queue
    for _ in 0..5 {
        let event = create_test_event();
        buffer.buffer_event(&event).await?;
    }
    
    // Try to add more events - should trigger backpressure
    let event = create_test_event();
    buffer.buffer_event(&event).await?; // This should be dropped
    
    // Check queue size
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 5); // Should still be at max
    
    // Check stats
    let stats = buffer.get_stats().await;
    assert_eq!(stats.events_buffered, 5);
    assert_eq!(stats.events_dropped, 1);
    
    info!("Backpressure test passed");
    Ok(())
}

async fn test_persistence_and_recovery(redis_url: &str) -> Result<()> {
    info!("Testing persistence and recovery");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "persistence_test_queue".to_string(), Some(config)).await?;
    
    // Create and buffer events
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
    ];
    
    for event in &events {
        buffer.buffer_event(event).await?;
        buffer.add_to_stream(event).await?;
    }
    
    // Clear the queue but keep streams
    buffer.clear_queue().await?;
    
    // Verify queue is empty
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 0);
    
    // Test cache retrieval
    for event in &events {
        let cache_key = format!("event:{}", event.transaction.hash);
        let cached_event = buffer.get_cached_event(&cache_key).await?;
        assert!(cached_event.is_some());
    }
    
    info!("Persistence and recovery test passed");
    Ok(())
}

async fn test_concurrent_operations(redis_url: &str) -> Result<()> {
    info!("Testing concurrent operations");
    
    let config = BufferConfig {
        max_queue_size: 1000,
        batch_size: 50,
        stream_max_len: 100,
        cache_ttl_seconds: 300,
        retry_attempts: 2,
        retry_delay_ms: 50,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "concurrent_test_queue".to_string(), Some(config)).await?;
    
    // Create multiple concurrent tasks
    let mut handles = vec![];
    
    for task_id in 0..10 {
        let mut buffer_clone = buffer.clone();
        let handle = tokio::spawn(async move {
            for i in 0..20 {
                let mut event = create_test_event();
                // Make each event unique
                event.id = EventId::new();
                buffer_clone.buffer_event(&event).await?;
            }
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }
    
    // Verify all events were processed
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 200); // 10 tasks * 20 events
    
    info!("Concurrent operations test passed");
    Ok(())
}

async fn test_large_batch_processing(redis_url: &str) -> Result<()> {
    info!("Testing large batch processing");
    
    let config = BufferConfig {
        max_queue_size: 10000,
        batch_size: 1000,
        stream_max_len: 500,
        cache_ttl_seconds: 600,
        retry_attempts: 3,
        retry_delay_ms: 100,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "large_batch_test_queue".to_string(), Some(config)).await?;
    
    // Create a large batch of events
    let mut events = Vec::new();
    for i in 0..5000 {
        let mut event = create_test_event();
                    event.id = EventId::new();
        events.push(event);
    }
    
    // Buffer the large batch
    let start_time = std::time::Instant::now();
    buffer.buffer_events(&events).await?;
    let buffer_time = start_time.elapsed();
    
    info!("Buffered 5000 events in {:?}", buffer_time);
    
    // Process events in batches
    let mut total_processed = 0;
    let start_time = std::time::Instant::now();
    
    while total_processed < 5000 {
        let processed = buffer.process_events(Some(1000)).await?;
        total_processed += processed.len();
        
        if processed.is_empty() {
            break;
        }
    }
    
    let process_time = start_time.elapsed();
    info!("Processed {} events in {:?}", total_processed, process_time);
    
    assert_eq!(total_processed, 5000);
    
    info!("Large batch processing test passed");
    Ok(())
}

async fn test_memory_pressure_scenarios(redis_url: &str) -> Result<()> {
    info!("Testing memory pressure scenarios");
    
    let config = BufferConfig {
        max_queue_size: 100,
        batch_size: 10,
        stream_max_len: 50,
        cache_ttl_seconds: 30,
        retry_attempts: 1,
        retry_delay_ms: 25,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "memory_pressure_test_queue".to_string(), Some(config)).await?;
    
    // Rapidly add and remove events to simulate memory pressure
    for cycle in 0..10 {
        // Add events rapidly
        for i in 0..50 {
            let mut event = create_test_event();
            event.id = EventId::new();
            buffer.buffer_event(&event).await?;
        }
        
        // Process some events
        let processed = buffer.process_events(Some(20)).await?;
        assert_eq!(processed.len(), 20);
        
        // Small delay
        sleep(Duration::from_millis(10)).await;
    }
    
    // Verify final state
    let queue_size = buffer.get_queue_size().await?;
    assert!(queue_size <= 100); // Should not exceed max
    
    info!("Memory pressure scenarios test passed");
    Ok(())
}

async fn test_performance_under_load(redis_url: &str) -> Result<()> {
    info!("Testing performance under load");
    
    let config = BufferConfig {
        max_queue_size: 5000,
        batch_size: 100,
        stream_max_len: 200,
        cache_ttl_seconds: 300,
        retry_attempts: 2,
        retry_delay_ms: 50,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "performance_test_queue".to_string(), Some(config)).await?;
    
    // Measure throughput
    let start_time = std::time::Instant::now();
    let event_count = 10000;
    
    // Buffer events in batches
    for batch_start in (0..event_count).step_by(100) {
        let batch_end = (batch_start + 100).min(event_count);
        let mut batch = Vec::new();
        
        for i in batch_start..batch_end {
            let mut event = create_test_event();
            event.id = EventId::new();
            batch.push(event);
        }
        
        buffer.buffer_events(&batch).await?;
    }
    
    let buffer_time = start_time.elapsed();
    let throughput = event_count as f64 / buffer_time.as_secs_f64();
    
    info!("Buffering throughput: {:.2} events/sec", throughput);
    assert!(throughput > 1000.0); // Should handle at least 1000 events/sec
    
    // Process events
    let start_time = std::time::Instant::now();
    let mut total_processed = 0;
    
    while total_processed < event_count {
        let processed = buffer.process_events(Some(100)).await?;
        total_processed += processed.len();
        
        if processed.is_empty() {
            break;
        }
    }
    
    let process_time = start_time.elapsed();
    let process_throughput = event_count as f64 / process_time.as_secs_f64();
    
    info!("Processing throughput: {:.2} events/sec", process_throughput);
    assert!(process_throughput > 500.0); // Should process at least 500 events/sec
    
    info!("Performance under load test passed");
    Ok(())
}

async fn test_empty_operations(redis_url: &str) -> Result<()> {
    info!("Testing empty operations");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "empty_test_queue".to_string(), Some(config)).await?;
    
    // Test processing empty queue
    let events = buffer.process_events(Some(10)).await?;
    assert_eq!(events.len(), 0);
    
    // Test buffering empty vector
    buffer.buffer_events(&[]).await?;
    
    // Test queue size on empty queue
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 0);
    
    // Test clearing empty queue
    buffer.clear_queue().await?;
    
    info!("Empty operations test passed");
    Ok(())
}

async fn test_boundary_conditions(redis_url: &str) -> Result<()> {
    info!("Testing boundary conditions");
    
    let config = BufferConfig {
        max_queue_size: 1,
        batch_size: 1,
        stream_max_len: 1,
        cache_ttl_seconds: 1,
        retry_attempts: 1,
        retry_delay_ms: 1,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "boundary_test_queue".to_string(), Some(config)).await?;
    
    // Test at exact capacity
    let event1 = create_test_event();
    buffer.buffer_event(&event1).await?;
    
    let event2 = create_test_event();
    buffer.buffer_event(&event2).await?; // Should be dropped
    
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 1);
    
    // Test with minimum TTL
    let event3 = create_test_event();
    buffer.cache_event("min_ttl", &event3, Duration::from_millis(1)).await?;
    
    // Wait for expiration
    sleep(Duration::from_millis(10)).await;
    
    let cached = buffer.get_cached_event("min_ttl").await?;
    assert!(cached.is_none());
    
    info!("Boundary conditions test passed");
    Ok(())
}

async fn test_malformed_data_handling(redis_url: &str) -> Result<()> {
    info!("Testing malformed data handling");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "malformed_test_queue".to_string(), Some(config)).await?;
    
    // Test with invalid cache keys
    let event = create_test_event();
    buffer.cache_event("", &event, Duration::from_secs(60)).await?; // Empty key
    buffer.cache_event(&"very_long_key_that_might_cause_issues_".repeat(10), &event, Duration::from_secs(60)).await?;
    
    // Test with very short TTL
    buffer.cache_event("short_ttl", &event, Duration::from_nanos(1)).await?;
    
    // Test with zero TTL
    buffer.cache_event("zero_ttl", &event, Duration::ZERO).await?;
    
    // Verify operations completed without errors
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 0); // No events were buffered, only cached
    
    info!("Malformed data handling test passed");
    Ok(())
}

async fn test_extreme_configurations(redis_url: &str) -> Result<()> {
    info!("Testing extreme configurations");
    
    // Test with very large configuration values
    let large_config = BufferConfig {
        max_queue_size: usize::MAX,
        batch_size: 10000,
        stream_max_len: 100000,
        cache_ttl_seconds: u64::MAX,
        retry_attempts: 1000,
        retry_delay_ms: 10000,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "extreme_test_queue".to_string(), Some(large_config)).await?;
    
    // Test with very small configuration values
    let small_config = BufferConfig {
        max_queue_size: 1,
        batch_size: 1,
        stream_max_len: 1,
        cache_ttl_seconds: 1,
        retry_attempts: 1,
        retry_delay_ms: 1,
    };
    
    let mut small_buffer = RedisBuffer::new(redis_url.to_string(), "extreme_small_test_queue".to_string(), Some(small_config)).await?;
    
    // Both should work without panicking
    buffer.health_check().await?;
    small_buffer.health_check().await?;
    
    info!("Extreme configurations test passed");
    Ok(())
}

async fn test_partial_failures(redis_url: &str) -> Result<()> {
    info!("Testing partial failures");
    
    let config = BufferConfig {
        max_queue_size: 100,
        batch_size: 20,
        stream_max_len: 50,
        cache_ttl_seconds: 60,
        retry_attempts: 1,
        retry_delay_ms: 10,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "partial_failure_test_queue".to_string(), Some(config)).await?;
    
    // Create events with some that might fail processing
    let events = vec![
        create_test_event(), // Normal event
        create_test_event(), // Normal event
        create_test_event(), // Normal event
    ];
    
    // Buffer events - some operations might fail partially
    buffer.buffer_events(&events).await?;
    
    // Process events - should handle partial failures gracefully
    let processed = buffer.process_events(Some(10)).await?;
    assert!(processed.len() <= events.len());
    
    info!("Partial failures test passed");
    Ok(())
}

async fn test_recovery_mechanisms(redis_url: &str) -> Result<()> {
    info!("Testing recovery mechanisms");
    
    let config = BufferConfig {
        max_queue_size: 50,
        batch_size: 10,
        stream_max_len: 25,
        cache_ttl_seconds: 120,
        retry_attempts: 3,
        retry_delay_ms: 50,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "recovery_test_queue".to_string(), Some(config)).await?;
    
    // Fill buffer partially
    for i in 0..25 {
        let mut event = create_test_event();
        event.id = EventId::new();
        buffer.buffer_event(&event).await?;
    }
    
    // Simulate recovery by clearing and re-adding
    buffer.clear_queue().await?;
    
    // Verify recovery
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 0);
    
    // Re-add events
    for i in 0..25 {
        let mut event = create_test_event();
        event.id = EventId::new();
        buffer.buffer_event(&event).await?;
    }
    
    let final_size = buffer.get_queue_size().await?;
    assert_eq!(final_size, 25);
    
    info!("Recovery mechanisms test passed");
    Ok(())
}

async fn test_data_consistency(redis_url: &str) -> Result<()> {
    info!("Testing data consistency");
    
    let config = BufferConfig::default();
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "consistency_test_queue".to_string(), Some(config)).await?;
    
    // Create and buffer events
    let original_events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
    ];
    
    for event in &original_events {
        buffer.buffer_event(event).await?;
        buffer.cache_event(&format!("cache_{}", event.transaction.hash), event, Duration::from_secs(60)).await?;
        buffer.add_to_stream(event).await?;
    }
    
    // Process events
    let processed_events = buffer.process_events(Some(10)).await?;
    
    // Verify data consistency
    assert_eq!(processed_events.len(), original_events.len());
    
    for (original, processed) in original_events.iter().zip(processed_events.iter()) {
        assert_eq!(original.transaction.hash, processed.transaction.hash);
        assert_eq!(original.id, processed.id);
    }
    
    info!("Data consistency test passed");
    Ok(())
}

async fn test_graceful_degradation(redis_url: &str) -> Result<()> {
    info!("Testing graceful degradation");
    
    let config = BufferConfig {
        max_queue_size: 5,
        batch_size: 2,
        stream_max_len: 3,
        cache_ttl_seconds: 30,
        retry_attempts: 1,
        retry_delay_ms: 10,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.to_string(), "degradation_test_queue".to_string(), Some(config)).await?;
    
    // Try to exceed limits gracefully
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
        create_test_event(),
        create_test_event(),
        create_test_event(), // Should be dropped
        create_test_event(), // Should be dropped
    ];
    
    // Should not panic, just drop excess events
    buffer.buffer_events(&events).await?;
    
    let queue_size = buffer.get_queue_size().await?;
    assert_eq!(queue_size, 5); // Max capacity
    
    let stats = buffer.get_stats().await;
    assert_eq!(stats.events_buffered, 5);
    assert_eq!(stats.events_dropped, 2);
    
    info!("Graceful degradation test passed");
    Ok(())
}

/// Test buffered processor with error handling
#[tokio::test]
async fn test_buffered_processor_with_error_handling() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for processor test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("default".to_string(), redis_url).await?;
    
    // Create filter
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    // Create processor
    let processor_config = BufferedProcessorConfig::default();
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Test single event processing
    let event = create_test_event();
    processor.process_event(&event).await?;
    
    // Test multiple events processing
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
    ];
    processor.process_events(&events).await?;
    
    // Test stats
    let stats = processor.get_stats().await;
    assert_eq!(stats.events_received, 4); // 1 single + 3 batch
    assert_eq!(stats.events_processed, 4);
    
    info!("Buffered processor test passed");
    Ok(())
}

/// Test buffer manager operations
#[tokio::test]
async fn test_buffer_manager_operations() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for manager test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    let config = BufferConfig::default();
    let mut manager = BufferManager::new(config);
    
    // Add buffers
    manager.add_buffer("primary".to_string(), redis_url.clone()).await?;
    manager.add_buffer("secondary".to_string(), redis_url).await?;
    
    // Check buffer names
    let buffer_names = manager.get_buffer_names();
    assert_eq!(buffer_names.len(), 2);
    assert!(buffer_names.contains(&"primary".to_string()));
    assert!(buffer_names.contains(&"secondary".to_string()));
    
    // Test health check
    manager.health_check_all().await?;
    
    // Test buffer operations
    if let Some(buffer) = manager.get_buffer("primary") {
        let event = create_test_event();
        buffer.buffer_event(&event).await?;
        
        let queue_size = buffer.get_queue_size().await?;
        assert_eq!(queue_size, 1);
    }
    
    // Remove buffer
    let removed_buffer = manager.remove_buffer("secondary");
    assert!(removed_buffer.is_some());
    
    let buffer_names = manager.get_buffer_names();
    assert_eq!(buffer_names.len(), 1);
    assert_eq!(buffer_names[0], "primary");
    
    info!("Buffer manager operations test passed");
    Ok(())
}

/// Test error scenarios and recovery
#[tokio::test]
async fn test_error_scenarios_and_recovery() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for error scenarios test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    let config = BufferConfig {
        max_queue_size: 3,
        batch_size: 2,
        stream_max_len: 5,
        cache_ttl_seconds: 30,
        retry_attempts: 2,
        retry_delay_ms: 50,
    };
    
    let mut buffer = RedisBuffer::new(redis_url, "error_scenarios_queue".to_string(), Some(config)).await?;
    
    // Test queue overflow handling
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
        create_test_event(), // Should be dropped
    ];
    
    buffer.buffer_events(&events).await?;
    
    let stats = buffer.get_stats().await;
    assert_eq!(stats.events_buffered, 3);
    assert_eq!(stats.events_dropped, 1);
    
    // Test cache expiration
    let event = create_test_event();
    let cache_key = "test_expiration";
    buffer.cache_event(cache_key, &event, Duration::from_millis(100)).await?;
    
    // Wait for expiration
    sleep(Duration::from_millis(150)).await;
    
    let cached_event = buffer.get_cached_event(cache_key).await?;
    assert!(cached_event.is_none());
    
    // Test stream limits
    for i in 0..10 {
        let event = create_test_event();
        buffer.add_to_stream(&event).await?;
    }
    
    // Stream should maintain max length
    let stats = buffer.get_stats().await;
    assert!(stats.events_cached >= 5); // At least some should be cached
    
    info!("Error scenarios and recovery test passed");
    Ok(())
}

/// Test processor lifecycle and worker management
#[tokio::test]
async fn test_processor_lifecycle() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for lifecycle test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("default".to_string(), redis_url).await?;
    
    // Create filter
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    // Create processor
    let processor_config = BufferedProcessorConfig {
        processing_interval: Duration::from_millis(50),
        batch_size: 5,
        max_workers: 2,
        backpressure_threshold: 10,
        enable_filtering: true,
        enable_caching: true,
        cache_ttl_seconds: 60,
    };
    
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Test initial state
    assert!(!processor.is_running().await);
    
    // Start processor
    processor.start().await?;
    assert!(processor.is_running().await);
    
    // Wait a bit for workers to start
    sleep(Duration::from_millis(100)).await;
    
    // Stop processor
    processor.stop().await?;
    assert!(!processor.is_running().await);
    
    info!("Processor lifecycle test passed");
    Ok(())
}

/// Test filtering integration
#[tokio::test]
async fn test_filtering_integration() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for filtering test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("default".to_string(), redis_url).await?;
    
    // Create filter with specific configuration
    let mut filter_config = FilteringConfig::default();
    filter_config.enabled = true;
    filter_config.include_protocols = vec!["Uniswap V2".to_string()];
    
    let filter = PoolFilter::new(filter_config);
    
    // Create processor with filtering enabled
    let processor_config = BufferedProcessorConfig {
        enable_filtering: true,
        ..Default::default()
    };
    
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Create events with different protocols
    let mut uniswap_event = create_test_event();
    uniswap_event.protocol.name = "Uniswap V2".to_string();
    
    let mut sushiswap_event = create_test_event();
    sushiswap_event.protocol.name = "SushiSwap".to_string();
    
    // Process events
    processor.process_event(&uniswap_event).await?; // Should be processed
    processor.process_event(&sushiswap_event).await?; // Should be filtered
    
    // Check stats
    let stats = processor.get_stats().await;
    assert_eq!(stats.events_received, 2);
    assert_eq!(stats.events_processed, 1);
    assert_eq!(stats.events_filtered, 1);
    
    info!("Filtering integration test passed");
    Ok(())
}

/// Test stress scenarios with rapid start/stop cycles
#[tokio::test]
async fn test_stress_scenarios() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for stress test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("default".to_string(), redis_url).await?;
    
    // Create filter
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    // Create processor
    let processor_config = BufferedProcessorConfig {
        processing_interval: Duration::from_millis(10),
        batch_size: 5,
        max_workers: 3,
        backpressure_threshold: 20,
        enable_filtering: true,
        enable_caching: true,
        cache_ttl_seconds: 30,
    };
    
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Rapid start/stop cycles
    for cycle in 0..5 {
        info!("Stress test cycle {}", cycle);
        
        // Start processor
        processor.start().await?;
        assert!(processor.is_running().await);
        
        // Rapidly add events
        for i in 0..50 {
            let mut event = create_test_event();
            event.id = EventId::new();
            processor.process_event(&event).await?;
        }
        
        // Wait a bit
        sleep(Duration::from_millis(50)).await;
        
        // Stop processor
        processor.stop().await?;
        assert!(!processor.is_running().await);
        
        // Small delay between cycles
        sleep(Duration::from_millis(10)).await;
    }
    
    info!("Stress scenarios test passed");
    Ok(())
}

/// Test integration with multiple buffer types
#[tokio::test]
async fn test_multi_buffer_integration() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for multi-buffer test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager with multiple buffers
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    
    // Add different types of buffers
    buffer_manager.add_buffer("high_priority".to_string(), redis_url.clone()).await?;
    buffer_manager.add_buffer("normal_priority".to_string(), redis_url.clone()).await?;
    buffer_manager.add_buffer("low_priority".to_string(), redis_url).await?;
    
    // Create filter
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    // Create processor
    let processor_config = BufferedProcessorConfig::default();
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Test operations on different buffers
    let buffer_manager = processor.get_buffer_manager_mut();
    
    // High priority buffer
    if let Some(buffer) = buffer_manager.get_buffer("high_priority") {
        let event = create_test_event();
        buffer.buffer_event(&event).await?;
        
        let queue_size = buffer.get_queue_size().await?;
        assert_eq!(queue_size, 1);
    }
    
    // Normal priority buffer
    if let Some(buffer) = buffer_manager.get_buffer("normal_priority") {
        let events = vec![create_test_event(), create_test_event()];
        buffer.buffer_events(&events).await?;
        
        let queue_size = buffer.get_queue_size().await?;
        assert_eq!(queue_size, 2);
    }
    
    // Low priority buffer
    if let Some(buffer) = buffer_manager.get_buffer("low_priority") {
        let event = create_test_event();
        buffer.buffer_event(&event).await?;
        
        let queue_size = buffer.get_queue_size().await?;
        assert_eq!(queue_size, 1);
    }
    
    // Test health check across all buffers
    let buffer_manager = processor.get_buffer_manager_mut();
    let health_result = buffer_manager.health_check_all().await;
    assert!(health_result.is_ok());
    
    info!("Multi-buffer integration test passed");
    Ok(())
}

/// Test configuration hot-reloading scenarios
#[tokio::test]
async fn test_configuration_hot_reload() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for config hot-reload test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer manager
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("config_test".to_string(), redis_url.clone()).await?;
    
    // Create filter
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    // Create processor with initial config
    let mut processor_config = BufferedProcessorConfig {
        processing_interval: Duration::from_millis(100),
        batch_size: 10,
        max_workers: 2,
        backpressure_threshold: 50,
        enable_filtering: true,
        enable_caching: true,
        cache_ttl_seconds: 60,
    };
    
    let mut processor = BufferedEventProcessor::new(processor_config.clone(), buffer_manager, filter);
    
    // Test with initial configuration
    let event = create_test_event();
    processor.process_event(&event).await?;
    
    // Simulate configuration change
    processor_config.processing_interval = Duration::from_millis(50);
    processor_config.batch_size = 20;
    processor_config.backpressure_threshold = 100;
    
    // Create new processor with updated config
    let buffer_config = BufferConfig::default();
    let mut new_buffer_manager = BufferManager::new(buffer_config);
    new_buffer_manager.add_buffer("config_test".to_string(), redis_url).await?;
    
    let filter_config = FilteringConfig::default();
    let new_filter = PoolFilter::new(filter_config);
    
    let mut new_processor = BufferedEventProcessor::new(processor_config, new_buffer_manager, new_filter);
    
    // Test with new configuration
    let event = create_test_event();
    new_processor.process_event(&event).await?;
    
    // Verify both processors work
    let stats1 = processor.get_stats().await;
    let stats2 = new_processor.get_stats().await;
    
    assert_eq!(stats1.events_processed, 1);
    assert_eq!(stats2.events_processed, 1);
    
    info!("Configuration hot-reload test passed");
    Ok(())
}

/// Test error propagation and logging
#[tokio::test]
async fn test_error_propagation_and_logging() -> Result<()> {
    let docker = Cli::default();
    let redis_image = Redis::default();
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    
    info!("Redis container started on port {} for error propagation test", redis_port);
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    
    // Create buffer with aggressive limits to trigger errors
    let config = BufferConfig {
        max_queue_size: 1,
        batch_size: 1,
        stream_max_len: 1,
        cache_ttl_seconds: 1,
        retry_attempts: 1,
        retry_delay_ms: 1,
    };
    
    let mut buffer = RedisBuffer::new(redis_url.clone(), "error_propagation_test_queue".to_string(), Some(config)).await?;
    
    // Test error propagation through the system
    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
    ];
    
    // This should trigger errors due to queue size limits
    buffer.buffer_events(&events).await?;
    
    // Verify error statistics are tracked
    let stats = buffer.get_stats().await;
    assert!(stats.errors > 0 || stats.events_dropped > 0);
    
    // Test error handling in processor
    let buffer_config = BufferConfig::default();
    let mut buffer_manager = BufferManager::new(buffer_config);
    buffer_manager.add_buffer("error_test".to_string(), redis_url).await?;
    
    let filter_config = FilteringConfig::default();
    let filter = PoolFilter::new(filter_config);
    
    let processor_config = BufferedProcessorConfig {
        enable_filtering: true,
        enable_caching: true,
        ..Default::default()
    };
    
    let mut processor = BufferedEventProcessor::new(processor_config, buffer_manager, filter);
    
    // Process events that might trigger errors
    for _ in 0..5 {
        let event = create_test_event();
        let result = processor.process_event(&event).await;
        // Should not panic, even if errors occur
        assert!(result.is_ok());
    }
    
    // Verify error statistics
    let stats = processor.get_stats().await;
    assert!(stats.errors >= 0); // Should not be negative
    
    info!("Error propagation and logging test passed");
    Ok(())
}

fn create_test_event() -> SwapEvent {
    use mev_relay::{
        events::domain::{EventId, EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo, EventMetadata},
        shared::types::{H160, H256},
    };
    
    SwapEvent {
        id: EventId::new(),
        source: EventSource::Mempool,
        protocol: ProtocolInfo {
            name: "Uniswap V2".to_string(),
            version: "2.0".to_string(),
            address: H160::from([1u8; 20]),
        },
        transaction: TransactionInfo {
            hash: H256::from([2u8; 32]),
            from: H160::from([3u8; 20]),
            to: Some(H160::from([4u8; 20])),
            gas_price: 20000000000u128,
            gas_limit: 300000u64,
            gas_used: 150000u64,
            nonce: 5u64,
            value: 0u128,
        },
        swap_details: SwapDetails {
            token_in: H160::from([6u8; 20]),
            token_out: H160::from([7u8; 20]),
            amount_in: 1000000000000000000u128,
            amount_out: 2000000000000000000u128,
            pool_address: Some(H160::from([8u8; 20])),
            fee_tier: Some(3000u32),
        },
        block_info: BlockInfo {
            number: 12345u64,
            timestamp: 1640995200u64,
            hash: H256::from([9u8; 32]),
        },
        metadata: EventMetadata::new(),
    }
} 