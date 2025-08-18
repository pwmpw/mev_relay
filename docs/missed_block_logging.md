# Missed Block Logging System

The MEV Relay project includes a comprehensive missed block logging system designed to track, monitor, and alert on missed blockchain blocks. This system is critical for ensuring no MEV opportunities are lost due to network issues, node failures, or other disruptions.

## Overview

The missed block logging system consists of several key components:

1. **Block Monitor** - Detects and tracks missed blocks
2. **Missed Block Logger** - Logs missed blocks and sends alerts
3. **Recovery System** - Attempts to recover missed blocks
4. **Statistics & Metrics** - Provides monitoring and reporting capabilities

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Block Chain   │───▶│  Block Monitor   │───▶│ Missed Block    │
│                 │    │                  │    │    Logger      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   Statistics     │    │   Alert System  │
                       │   & Metrics      │    │                 │
                       └──────────────────┘    └─────────────────┘
```

## Components

### 1. Block Monitor

The `BlockMonitor` is responsible for:
- Processing incoming blocks
- Detecting gaps in block sequences
- Identifying delayed blocks
- Managing recovery attempts
- Maintaining statistics

#### Configuration

```rust
pub struct BlockMonitoringConfig {
    pub expected_block_time: Duration,        // Expected time between blocks
    pub max_block_delay: Duration,            // Maximum acceptable delay
    pub retry_attempts: u32,                  // Number of recovery attempts
    pub retry_delay: Duration,                // Delay between retry attempts
    pub alert_threshold: u32,                 // Blocks missed before alerting
    pub enable_recovery: bool,                // Enable recovery mechanisms
    pub log_missed_blocks: bool,              // Enable logging
    pub metrics_enabled: bool,                // Enable metrics collection
}
```

#### Default Configuration

- **Expected Block Time**: 12 seconds (Ethereum mainnet)
- **Max Block Delay**: 30 seconds
- **Retry Attempts**: 3
- **Retry Delay**: 5 seconds
- **Alert Threshold**: 5 consecutive missed blocks
- **Recovery**: Enabled
- **Logging**: Enabled
- **Metrics**: Enabled

### 2. Missed Block Logger

The `MissedBlockLogger` handles:
- Persistent logging of missed blocks
- Alert generation and distribution
- Log rotation and cleanup
- Data export in multiple formats
- Performance monitoring

#### Configuration

```rust
pub struct MissedBlockLoggerConfig {
    pub log_file_path: String,                // Path to log file
    pub log_rotation_size: u64,               // Max log file size (bytes)
    pub log_retention_days: u32,              // Log retention period
    pub enable_file_logging: bool,            // Enable file logging
    pub enable_console_logging: bool,         // Enable console logging
    pub enable_metrics: bool,                 // Enable metrics
    pub alert_channels: Vec<AlertChannel>,    // Alert destinations
    pub batch_logging: bool,                  // Enable batch logging
    pub batch_size: usize,                    // Batch size for logging
    pub batch_timeout: Duration,              // Batch timeout
}
```

#### Default Configuration

- **Log File Path**: `logs/missed_blocks.jsonl`
- **Rotation Size**: 10MB
- **Retention**: 30 days
- **File Logging**: Enabled
- **Console Logging**: Enabled
- **Metrics**: Enabled
- **Alert Channels**: Console only
- **Batch Logging**: Enabled
- **Batch Size**: 100 entries
- **Batch Timeout**: 60 seconds

### 3. Alert Channels

The system supports multiple alert channels:

```rust
pub enum AlertChannel {
    Console,                    // Console output
    File,                       // File-based alerts
    Webhook(String),            // HTTP webhook
    Email(String),              // Email notifications
    Slack(String),              // Slack webhook
    Discord(String),            // Discord webhook
}
```

### 4. Missed Block Reasons

The system categorizes missed blocks by reason:

```rust
pub enum MissedBlockReason {
    NetworkLatency,             // Network delays
    NodeUnresponsive,           // Node not responding
    RpcError,                   // RPC call failures
    InvalidBlock,               // Invalid block data
    ChainReorg,                 // Chain reorganization
    GasLimitExceeded,           // Gas limit exceeded
    NonceMismatch,             // Nonce issues
    InsufficientFunds,          // Insufficient funds
    ContractRevert,             // Contract revert
    Timeout,                    // Operation timeout
    ConnectionLost,             // Connection lost
    RateLimitExceeded,          // Rate limit exceeded
    Unknown,                    // Unknown reason
}
```

### 5. Severity Levels

Missed blocks are classified by severity:

```rust
pub enum MissedBlockSeverity {
    Low,                        // Minor delays, no MEV impact
    Medium,                     // Moderate delays, some MEV impact
    High,                       // Significant delays, major MEV impact
    Critical,                   // Complete failure, severe MEV impact
}
```

## Usage

### Basic Setup

```rust
use mev_relay::monitoring::{
    BlockMonitor, BlockMonitoringConfig,
    MissedBlockLogger, MissedBlockLoggerConfig
};

// Create block monitor
let monitor_config = BlockMonitoringConfig::default();
let mut monitor = BlockMonitor::new(monitor_config);

// Create logger
let logger_config = MissedBlockLoggerConfig::default();
let logger = MissedBlockLogger::new(logger_config, monitor)?;

// Start logging service
logger.start().await?;
```

### Processing Blocks

```rust
// Process incoming blocks
let block = BlockInfo {
    number: 1000,
    hash: "0x...".to_string(),
    timestamp: 1600000000,
    // ... other fields
};

monitor.process_block(&block)?;
```

### Handling Missed Blocks

```rust
// Check for missed blocks
if monitor.should_alert() {
    let alert_message = monitor.get_alert_message();
    println!("Alert: {}", alert_message.unwrap());
}

// Get statistics
let stats = monitor.get_stats();
println!("Total missed: {}", stats.total_missed);
println!("Recovery rate: {:.2}%", stats.get_recovery_rate());
```

### Logging and Alerts

```rust
// Process missed block for logging
let missed_block = MissedBlock {
    block_number: 1001,
    reason: MissedBlockReason::NetworkLatency,
    severity: MissedBlockSeverity::Medium,
    // ... other fields
};

logger.process_missed_block(&missed_block).await?;
```

## Log Format

### JSON Lines Format

Each missed block is logged as a JSON line:

```json
{"timestamp":1640995200,"block_number":1001,"reason":"NetworkLatency","severity":"Medium","expected_timestamp":1640995212,"actual_timestamp":1640995220,"recovery_attempts":0,"recovered":false,"metadata":{},"alert_sent":false}
```

### CSV Export

Data can be exported in CSV format:

```csv
block_number,reason,severity,expected_timestamp,actual_timestamp,recovery_attempts,recovered,missed_at
1001,NetworkLatency,Medium,1640995212,1640995220,0,false,1640995220
1002,NodeUnresponsive,High,1640995224,1640995240,1,true,1640995240
```

## Monitoring and Metrics

### Key Metrics

- **Blocks Processed**: Total blocks processed
- **Blocks Missed**: Total blocks missed
- **Blocks Recovered**: Total blocks recovered
- **Current Missed Streak**: Consecutive missed blocks
- **Longest Missed Streak**: Longest consecutive missed blocks
- **Missed Block Rate**: Percentage of missed blocks
- **Recovery Rate**: Percentage of recovered blocks
- **Average Recovery Time**: Average time to recover blocks

### Prometheus Metrics

The system exposes Prometheus metrics for monitoring:

```prometheus
# HELP mev_relay_blocks_missed_total Total number of missed blocks
# TYPE mev_relay_blocks_missed_total counter
mev_relay_blocks_missed_total{reason="NetworkLatency"} 15

# HELP mev_relay_blocks_recovered_total Total number of recovered blocks
# TYPE mev_relay_blocks_recovered_total counter
mev_relay_blocks_recovered_total 12

# HELP mev_relay_missed_block_duration_seconds Time to recover missed blocks
# TYPE mev_relay_missed_block_duration_seconds histogram
mev_relay_missed_block_duration_seconds_bucket{le="5"} 8
```

## Recovery Mechanisms

### Automatic Recovery

The system attempts automatic recovery of missed blocks:

1. **Detection**: Identify missed blocks based on sequence gaps
2. **Analysis**: Determine reason and severity
3. **Recovery**: Attempt to recover block data
4. **Verification**: Verify recovered data integrity
5. **Cleanup**: Remove recovered blocks from tracking

### Recovery Strategies

- **RPC Retry**: Retry failed RPC calls
- **Alternative Nodes**: Query alternative node endpoints
- **Block Explorer**: Use block explorer APIs as fallback
- **Peer Networks**: Query peer networks for missing data

## Alerting

### Alert Thresholds

- **Low Severity**: No immediate action required
- **Medium Severity**: Monitor closely, potential MEV impact
- **High Severity**: Immediate attention required, significant MEV impact
- **Critical Severity**: Emergency response required, severe MEV impact

### Alert Channels

1. **Console**: Immediate console output with emojis
2. **File**: Persistent file-based alerts
3. **Webhook**: HTTP POST to configured endpoints
4. **Email**: SMTP-based email notifications
5. **Slack**: Slack webhook integration
6. **Discord**: Discord webhook integration

### Alert Messages

```
🚨 ALERT: Critical missed block 1001 detected!
Reason: NodeUnresponsive
Severity: Critical
Expected: 1640995212
Actual: 1640995240
Time: 2022-01-01 12:00:00 UTC
```

## Performance Considerations

### Batch Processing

- **Batch Size**: Configurable batch size for logging
- **Batch Timeout**: Automatic flush timeout
- **Memory Management**: Efficient memory usage with batching

### Log Rotation

- **Size-based**: Rotate when file size exceeds limit
- **Time-based**: Optional time-based rotation
- **Compression**: Compress old log files
- **Cleanup**: Automatic cleanup of old logs

### Resource Management

- **Connection Pooling**: Efficient connection management
- **Async Operations**: Non-blocking async operations
- **Memory Limits**: Configurable memory limits
- **Error Handling**: Graceful error handling and recovery

## Testing

### Test Categories

1. **Basic Monitoring**: Core functionality tests
2. **High-Throughput**: Performance under load
3. **Edge Cases**: Boundary conditions and errors
4. **Recovery**: Recovery mechanism tests
5. **Integration**: End-to-end system tests

### Running Tests

```bash
# Run all missed block tests
make missed-block-test

# Run specific test categories
make missed-block-test-basic
make missed-block-test-throughput
make missed-block-test-edge-cases

# Run with detailed output
make missed-block-test-all
```

## Configuration Examples

### Production Configuration

```toml
[monitoring]
expected_block_time = 12
max_block_delay = 30
retry_attempts = 5
retry_delay = 10
alert_threshold = 3
enable_recovery = true
log_missed_blocks = true
metrics_enabled = true

[logging]
log_file_path = "/var/log/mev_relay/missed_blocks.jsonl"
log_rotation_size = 100MB
log_retention_days = 90
enable_file_logging = true
enable_console_logging = false
batch_size = 1000
batch_timeout = 300

[alerts]
channels = ["webhook", "slack", "email"]
webhook_url = "https://alerts.example.com/webhook"
slack_webhook = "https://hooks.slack.com/services/..."
email_address = "alerts@example.com"
```

### Development Configuration

```toml
[monitoring]
expected_block_time = 12
max_block_delay = 60
retry_attempts = 2
retry_delay = 5
alert_threshold = 10
enable_recovery = true
log_missed_blocks = true
metrics_enabled = true

[logging]
log_file_path = "logs/missed_blocks.jsonl"
log_rotation_size = 10MB
log_retention_days = 7
enable_file_logging = true
enable_console_logging = true
batch_size = 100
batch_timeout = 60

[alerts]
channels = ["console"]
```

## Troubleshooting

### Common Issues

1. **High Missed Block Rate**
   - Check network connectivity
   - Verify node endpoints
   - Review alert thresholds

2. **Recovery Failures**
   - Check retry configuration
   - Verify alternative data sources
   - Review error logs

3. **Performance Issues**
   - Adjust batch sizes
   - Review log rotation settings
   - Monitor memory usage

4. **Alert Spam**
   - Increase alert thresholds
   - Implement alert throttling
   - Review alert channel configuration

### Debug Mode

Enable debug logging for troubleshooting:

```bash
export RUST_LOG=debug
make missed-block-test-all
```

### Health Checks

Monitor system health:

```bash
# Check system status
make status

# View logs
make logs

# Check metrics
curl http://localhost:9090/metrics
```

## Integration

### With Existing Systems

The missed block logging system integrates with:

- **Prometheus**: Metrics collection and monitoring
- **Grafana**: Dashboard and visualization
- **Redis**: Caching and persistence
- **PostgreSQL**: Data storage and analysis
- **Docker**: Containerized deployment

### API Endpoints

```rust
// Health check
GET /health

// Statistics
GET /stats

// Metrics
GET /metrics

// Report generation
GET /report

// Data export
GET /export?format=json
GET /export?format=csv
```

## Future Enhancements

### Planned Features

1. **Machine Learning**: Predictive missed block detection
2. **Advanced Recovery**: Multi-chain recovery strategies
3. **Real-time Alerts**: WebSocket-based real-time notifications
4. **Analytics**: Advanced analytics and reporting
5. **Integration**: More alert channel integrations

### Contributing

To contribute to the missed block logging system:

1. **Fork the repository**
2. **Create a feature branch**
3. **Add tests for new functionality**
4. **Update documentation**
5. **Submit a pull request**

## Conclusion

The missed block logging system provides comprehensive monitoring and alerting for blockchain block processing, ensuring no MEV opportunities are lost due to technical issues. With configurable alerting, efficient logging, and robust recovery mechanisms, it provides the foundation for reliable MEV relay operations.

For questions or issues, please refer to the project documentation or create an issue in the project repository. 