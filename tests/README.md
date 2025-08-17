# MEV Relay Test Suite

This document describes the comprehensive test suite for the MEV Relay project, including unit tests, integration tests, and specialized Redis buffering tests.

## Test Categories

### 1. Unit Tests
- **Command**: `cargo test`
- **Description**: Tests individual functions and modules in isolation
- **Coverage**: Core functionality, error handling, edge cases
- **Speed**: Fast execution, no external dependencies

### 2. Integration Tests
- **Command**: `make integration-test`
- **Description**: Tests module interactions and end-to-end workflows
- **Coverage**: Cross-module functionality, data flow, error propagation
- **Speed**: Medium execution, tests internal module interactions

### 3. Testcontainers Integration Tests
- **Command**: `make testcontainers-test`
- **Description**: Tests with real Redis and PostgreSQL containers
- **Coverage**: Database interactions, connection handling, persistence
- **Speed**: Slower execution, requires Docker

### 4. Redis Buffer Integration Tests
- **Command**: `make redis-buffer-test`
- **Description**: Comprehensive Redis buffering system tests
- **Coverage**: Error handling, reconnections, performance, edge cases
- **Speed**: Medium execution, requires Redis container

## Redis Buffer Test Suite

The Redis buffer test suite is the most comprehensive testing component, covering all aspects of the buffering system:

### Test Categories

#### Basic Operations (`make redis-buffer-test-basic`)
- Event buffering and retrieval
- Queue management
- Caching with TTL
- Stream persistence
- Health checks

#### High-Throughput Scenarios (`make redis-buffer-test-throughput`)
- Concurrent operations (10 tasks × 20 events)
- Large batch processing (5000 events)
- Memory pressure scenarios
- Performance under load (10,000 events)
- Throughput measurements

#### Edge Cases & Boundaries (`make redis-buffer-test-edge-cases`)
- Empty operations
- Boundary conditions (min/max values)
- Malformed data handling
- Extreme configurations
- Zero and maximum TTL values

#### Failure Scenarios & Recovery (`make redis-buffer-test-failures`)
- Partial failures
- Recovery mechanisms
- Data consistency verification
- Graceful degradation
- Error propagation

#### Buffered Processor (`make redis-buffer-test-processor`)
- Event processing pipeline
- Filtering integration
- Worker management
- Statistics tracking
- Lifecycle management

#### Stress Testing (`make redis-buffer-test-stress`)
- Rapid start/stop cycles
- Worker pool stress
- Connection stability
- Resource management
- Performance degradation

#### Integration & Multi-Buffer (`make redis-buffer-test-integration`)
- Multiple buffer types
- Configuration hot-reloading
- Error propagation and logging
- Cross-buffer operations
- Health monitoring

### Performance Benchmarks

The test suite includes performance benchmarks to ensure the system meets production requirements:

- **Buffering Throughput**: >1000 events/second
- **Processing Throughput**: >500 events/second
- **Concurrent Operations**: 10 concurrent tasks
- **Large Batch Processing**: 5000+ events
- **Memory Pressure**: 10 rapid cycles with 50 events each

### Error Handling & Resilience

Tests verify the system's resilience under various failure conditions:

- **Connection Failures**: Automatic reconnection
- **Queue Overflow**: Graceful backpressure
- **Data Corruption**: Malformed data handling
- **Resource Exhaustion**: Memory and connection limits
- **Partial Failures**: Graceful degradation

## Running Tests

### Quick Test Commands

```bash
# Run all tests
make test-all

# Run specific test categories
make test                    # Unit tests only
make integration-test        # Integration tests
make testcontainers-test     # Container-based tests
make redis-buffer-test       # Redis buffer tests

# Run specific Redis buffer test categories
make redis-buffer-test-basic        # Basic operations
make redis-buffer-test-throughput   # Performance tests
make redis-buffer-test-edge-cases   # Edge cases
make redis-buffer-test-failures     # Failure scenarios
make redis-buffer-test-processor    # Processor tests
make redis-buffer-test-stress       # Stress testing
make redis-buffer-test-integration  # Integration scenarios
```

### Test with Output

```bash
# Run all Redis buffer tests with detailed output
make redis-buffer-test-all

# Run specific tests with output
make redis-buffer-test-basic
make redis-buffer-test-throughput
```

### Test Configuration

Tests can be configured using environment variables:

```bash
# Set test timeout
export TEST_TIMEOUT=300

# Enable verbose logging
export RUST_LOG=debug

# Set Redis connection parameters
export REDIS_HOST=127.0.0.1
export REDIS_PORT=6379
```

## Test Environment Requirements

### Prerequisites

- Rust toolchain (latest stable)
- Docker and Docker Compose
- Testcontainers support
- Redis (for buffer tests)
- PostgreSQL (for integration tests)

### Docker Services

The test suite uses Docker containers for external dependencies:

```bash
# Start required services
make docker-run

# Check service status
make status

# View service logs
make docker-logs

# Stop services
make docker-stop
```

## Test Results & Reporting

### Output Format

Tests provide structured output including:
- Test execution time
- Performance metrics
- Error counts and types
- Resource usage statistics
- Success/failure rates

### Example Output

```
Running Redis buffer integration tests...
test_redis_buffer_with_error_handling ... ok
test_high_throughput_scenarios ... ok
test_edge_cases_and_boundaries ... ok
test_failure_scenarios_and_recovery ... ok
test_buffered_processor_with_error_handling ... ok
test_stress_scenarios ... ok
test_multi_buffer_integration ... ok
test_configuration_hot_reload ... ok
test_error_propagation_and_logging ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
✓ Redis buffer integration tests complete
```

### Performance Metrics

Key performance indicators tracked:
- **Events/Second**: Processing throughput
- **Latency**: Response time percentiles
- **Memory Usage**: Peak and average consumption
- **Error Rate**: Failures per operation
- **Recovery Time**: Time to restore from failures

## Continuous Integration

### CI/CD Pipeline

The test suite is designed for CI/CD environments:
- **Fast Feedback**: Unit tests run in <30 seconds
- **Comprehensive Coverage**: Integration tests cover all scenarios
- **Docker Integration**: Containerized test environment
- **Parallel Execution**: Concurrent test execution where possible
- **Failure Reporting**: Detailed error information for debugging

### GitHub Actions

Tests are automatically run on:
- Pull requests
- Push to main branch
- Release tags
- Scheduled runs (nightly)

## Troubleshooting

### Common Issues

1. **Docker Not Running**
   ```bash
   # Check Docker status
   docker info
   
   # Start Docker
   sudo systemctl start docker
   ```

2. **Port Conflicts**
   ```bash
   # Check port usage
   netstat -tulpn | grep :6379
   
   # Stop conflicting services
   sudo systemctl stop redis-server
   ```

3. **Test Timeouts**
   ```bash
   # Increase timeout
   export TEST_TIMEOUT=600
   
   # Run with verbose output
   make redis-buffer-test-all
   ```

4. **Memory Issues**
   ```bash
   # Check system resources
   free -h
   
   # Clean Docker resources
   make docker-clean
   ```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Set debug level
export RUST_LOG=debug

# Run tests with debug output
RUST_LOG=debug make redis-buffer-test-all
```

## Contributing

### Adding New Tests

When adding new tests:

1. **Follow Naming Convention**: `test_<category>_<scenario>`
2. **Include Documentation**: Clear test description
3. **Add Makefile Target**: Update Makefile with new test commands
4. **Update Coverage**: Ensure new functionality is tested
5. **Performance Testing**: Include benchmarks for new features

### Test Guidelines

- **Isolation**: Tests should not depend on each other
- **Cleanup**: Always clean up test data and resources
- **Assertions**: Use meaningful assertions with clear error messages
- **Performance**: Include performance benchmarks for critical paths
- **Error Cases**: Test both success and failure scenarios

## Conclusion

The MEV Relay test suite provides comprehensive coverage of all system components, ensuring reliability, performance, and resilience in production environments. The Redis buffer tests specifically validate the core buffering functionality with extensive error handling and performance testing.

For questions or issues with the test suite, please refer to the project documentation or create an issue in the project repository. 