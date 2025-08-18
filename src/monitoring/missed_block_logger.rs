use crate::monitoring::domain::{
    BlockInfo, BlockMonitor, BlockMonitoringConfig, MissedBlock, MissedBlockReason, 
    MissedBlockSeverity, MissedBlockStats, BlockMetrics
};
use crate::Result;
use serde_json;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// Configuration for missed block logging
#[derive(Debug, Clone)]
pub struct MissedBlockLoggerConfig {
    pub log_file_path: String,
    pub log_rotation_size: u64, // bytes
    pub log_retention_days: u32,
    pub enable_file_logging: bool,
    pub enable_console_logging: bool,
    pub enable_metrics: bool,
    pub alert_channels: Vec<AlertChannel>,
    pub batch_logging: bool,
    pub batch_size: usize,
    pub batch_timeout: Duration,
}

impl Default for MissedBlockLoggerConfig {
    fn default() -> Self {
        Self {
            log_file_path: "logs/missed_blocks.jsonl".to_string(),
            log_rotation_size: 10 * 1024 * 1024, // 10MB
            log_retention_days: 30,
            enable_file_logging: true,
            enable_console_logging: true,
            enable_metrics: true,
            alert_channels: vec![AlertChannel::Console],
            batch_logging: true,
            batch_size: 100,
            batch_timeout: Duration::from_secs(60),
        }
    }
}

/// Alert channels for missed block notifications
#[derive(Debug, Clone)]
pub enum AlertChannel {
    Console,
    File,
    Webhook(String), // URL
    Email(String),    // Email address
    Slack(String),    // Webhook URL
    Discord(String),  // Webhook URL
}

/// Missed block log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MissedBlockLogEntry {
    pub timestamp: u64,
    pub block_number: u64,
    pub reason: String,
    pub severity: String,
    pub expected_timestamp: u64,
    pub actual_timestamp: Option<u64>,
    pub recovery_attempts: u32,
    pub recovered: bool,
    pub metadata: HashMap<String, String>,
    pub alert_sent: bool,
}

/// Missed block logger service
pub struct MissedBlockLogger {
    config: MissedBlockLoggerConfig,
    monitor: Arc<RwLock<BlockMonitor>>,
    log_buffer: Arc<RwLock<Vec<MissedBlockLogEntry>>>,
    log_writer: Option<BufWriter<File>>,
    last_rotation: SystemTime,
    stats: Arc<RwLock<LoggerStats>>,
}

/// Logger statistics
#[derive(Debug, Clone, Default)]
pub struct LoggerStats {
    pub total_logged: u64,
    pub total_alerts: u64,
    pub total_errors: u64,
    pub last_log_time: Option<SystemTime>,
    pub last_alert_time: Option<SystemTime>,
    pub log_file_size: u64,
    pub rotation_count: u32,
}

impl MissedBlockLogger {
    /// Create a new missed block logger
    pub fn new(config: MissedBlockLoggerConfig, monitor: BlockMonitor) -> Result<Self> {
        let logger = Self {
            config,
            monitor: Arc::new(RwLock::new(monitor)),
            log_buffer: Arc::new(RwLock::new(Vec::new())),
            log_writer: None,
            last_rotation: SystemTime::now(),
            stats: Arc::new(RwLock::new(LoggerStats::default())),
        };
        
        // Initialize logging
        logger.initialize_logging()?;
        
        Ok(logger)
    }
    
    /// Initialize logging infrastructure
    fn initialize_logging(&self) -> Result<()> {
        if self.config.enable_file_logging {
            self.ensure_log_directory()?;
            self.rotate_log_if_needed()?;
        }
        Ok(())
    }
    
    /// Ensure log directory exists
    fn ensure_log_directory(&self) -> Result<()> {
        if let Some(parent) = Path::new(&self.config.log_file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
    
    /// Rotate log file if needed
    fn rotate_log_if_needed(&self) -> Result<()> {
        let log_path = Path::new(&self.config.log_file_path);
        
        if log_path.exists() {
            let metadata = std::fs::metadata(log_path)?;
            let file_size = metadata.len();
            
            if file_size >= self.config.log_rotation_size {
                self.perform_log_rotation()?;
            }
        }
        
        Ok(())
    }
    
    /// Perform log rotation
    fn perform_log_rotation(&self) -> Result<()> {
        let log_path = Path::new(&self.config.log_file_path);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        
        let rotated_path = format!("{}.{}", self.config.log_file_path, timestamp);
        
        if log_path.exists() {
            std::fs::rename(log_path, &rotated_path)?;
            info!("Log file rotated to: {}", rotated_path);
        }
        
        // Update stats
        {
            let mut stats = self.stats.blocking_write();
            stats.rotation_count += 1;
        }
        
        Ok(())
    }
    
    /// Start the logger service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting missed block logger service");
        
        // Start background tasks
        let log_buffer = self.log_buffer.clone();
        let config = self.config.clone();
        
        // Background logging task
        let logger_task = {
            let log_buffer = log_buffer.clone();
            let config = config.clone();
            tokio::spawn(async move {
                let mut interval = interval(config.batch_timeout);
                
                loop {
                    interval.tick().await;
                    
                    // Process buffered logs
                    if let Err(e) = Self::process_buffered_logs(&log_buffer, &config).await {
                        error!("Error processing buffered logs: {}", e);
                    }
                }
            })
        };
        
        // Background cleanup task
        let cleanup_task = {
            let config = self.config.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(3600)); // Every hour
                
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = Self::cleanup_old_logs(&config).await {
                        error!("Error cleaning up old logs: {}", e);
                    }
                }
            })
        };
        
        info!("Missed block logger service started");
        
        // Keep the service running
        tokio::select! {
            _ = logger_task => {},
            _ = cleanup_task => {},
        }
        
        Ok(())
    }
    
    /// Process a missed block
    pub async fn process_missed_block(&self, missed_block: &MissedBlock) -> Result<()> {
        let log_entry = MissedBlockLogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs(),
            block_number: missed_block.block_number,
            reason: format!("{:?}", missed_block.reason),
            severity: format!("{:?}", missed_block.severity),
            expected_timestamp: missed_block.expected_timestamp,
            actual_timestamp: missed_block.actual_timestamp,
            recovery_attempts: missed_block.recovery_attempts,
            recovered: missed_block.recovered,
            metadata: missed_block.metadata.clone(),
            alert_sent: false,
        };
        
        // Add to buffer
        {
            let mut buffer = self.log_buffer.write().await;
            buffer.push(log_entry.clone());
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_logged += 1;
            stats.last_log_time = Some(SystemTime::now());
        }
        
        // Console logging
        if self.config.enable_console_logging {
            self.log_to_console(&log_entry).await;
        }
        
        // Send alerts for high severity
        if missed_block.severity == MissedBlockSeverity::High || 
           missed_block.severity == MissedBlockSeverity::Critical {
            self.send_alert(missed_block).await?;
        }
        
        // Flush buffer if full
        {
            let buffer = self.log_buffer.read().await;
            if buffer.len() >= self.config.batch_size {
                if let Err(e) = Self::process_buffered_logs(&self.log_buffer, &self.config).await {
                    error!("Error flushing log buffer: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Log to console
    async fn log_to_console(&self, entry: &MissedBlockLogEntry) {
        let severity_emoji = match entry.severity.as_str() {
            "Low" => "🟡",
            "Medium" => "🟠",
            "High" => "🔴",
            "Critical" => "🚨",
            _ => "⚪",
        };
        
        warn!(
            "{} Missed block {}: {} (severity: {}) - expected: {}, actual: {}",
            severity_emoji,
            entry.block_number,
            entry.reason,
            entry.severity,
            entry.expected_timestamp,
            entry.actual_timestamp.unwrap_or(0)
        );
    }
    
    /// Send alert for missed block
    async fn send_alert(&self, missed_block: &MissedBlock) -> Result<()> {
        for channel in &self.config.alert_channels {
            match channel {
                AlertChannel::Console => {
                    self.send_console_alert(missed_block).await;
                }
                AlertChannel::File => {
                    self.send_file_alert(missed_block).await?;
                }
                AlertChannel::Webhook(url) => {
                    self.send_webhook_alert(missed_block, url).await?;
                }
                AlertChannel::Email(email) => {
                    self.send_email_alert(missed_block, email).await?;
                }
                AlertChannel::Slack(webhook) => {
                    self.send_slack_alert(missed_block, webhook).await?;
                }
                AlertChannel::Discord(webhook) => {
                    self.send_discord_alert(missed_block, webhook).await?;
                }
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_alerts += 1;
            stats.last_alert_time = Some(SystemTime::now());
        }
        
        Ok(())
    }
    
    /// Send console alert
    async fn send_console_alert(&self, missed_block: &MissedBlock) {
        let alert_msg = format!(
            "🚨 ALERT: Critical missed block {} detected!\n\
             Reason: {:?}\n\
             Severity: {:?}\n\
             Expected: {}\n\
             Actual: {}\n\
             Time: {:?}",
            missed_block.block_number,
            missed_block.reason,
            missed_block.severity,
            missed_block.expected_timestamp,
            missed_block.actual_timestamp.unwrap_or(0),
            missed_block.missed_at
        );
        
        error!("{}", alert_msg);
    }
    
    /// Send file alert
    async fn send_file_alert(&self, _missed_block: &MissedBlock) -> Result<()> {
        // Implementation for file-based alerts
        Ok(())
    }
    
    /// Send webhook alert
    async fn send_webhook_alert(&self, _missed_block: &MissedBlock, _url: &str) -> Result<()> {
        // Implementation for webhook alerts
        Ok(())
    }
    
    /// Send email alert
    async fn send_email_alert(&self, _missed_block: &MissedBlock, _email: &str) -> Result<()> {
        // Implementation for email alerts
        Ok(())
    }
    
    /// Send Slack alert
    async fn send_slack_alert(&self, _missed_block: &MissedBlock, _webhook: &str) -> Result<()> {
        // Implementation for Slack alerts
        Ok(())
    }
    
    /// Send Discord alert
    async fn send_discord_alert(&self, _missed_block: &MissedBlock, _webhook: &str) -> Result<()> {
        // Implementation for Discord alerts
        Ok(())
    }
    
    /// Process buffered logs
    async fn process_buffered_logs(
        log_buffer: &Arc<RwLock<Vec<MissedBlockLogEntry>>>,
        config: &MissedBlockLoggerConfig,
    ) -> Result<()> {
        let mut buffer = log_buffer.write().await;
        
        if buffer.is_empty() {
            return Ok(());
        }
        
        // Write to file if enabled
        if config.enable_file_logging {
            Self::write_logs_to_file(&buffer, config).await?;
        }
        
        // Clear buffer
        buffer.clear();
        
        Ok(())
    }
    
    /// Write logs to file
    async fn write_logs_to_file(
        logs: &[MissedBlockLogEntry],
        config: &MissedBlockLoggerConfig,
    ) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file_path)?;
        
        let mut writer = BufWriter::new(file);
        
        for log_entry in logs {
            let json_line = serde_json::to_string(log_entry)?;
            writeln!(writer, "{}", json_line)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    /// Cleanup old log files
    async fn cleanup_old_logs(config: &MissedBlockLoggerConfig) -> Result<()> {
        let log_dir = Path::new(&config.log_file_path).parent().unwrap_or(Path::new("."));
        let cutoff_time = SystemTime::now() - Duration::from_secs(config.log_retention_days as u64 * 24 * 3600);
        
        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "jsonl") {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                if modified < cutoff_time {
                                    if let Err(e) = std::fs::remove_file(&path) {
                                        warn!("Failed to remove old log file {:?}: {}", path, e);
                                    } else {
                                        debug!("Removed old log file: {:?}", path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get logger statistics
    pub async fn get_stats(&self) -> LoggerStats {
        self.stats.read().await.clone()
    }
    
    /// Get missed block statistics from monitor
    pub async fn get_missed_block_stats(&self) -> MissedBlockStats {
        let monitor = self.monitor.read().await;
        monitor.get_stats().clone()
    }
    
    /// Get block metrics
    pub async fn get_block_metrics(&self) -> BlockMetrics {
        let monitor = self.monitor.read().await;
        BlockMetrics::from_monitor(&monitor)
    }
    
    /// Generate missed block report
    pub async fn generate_report(&self) -> Result<String> {
        let monitor = self.monitor.read().await;
        let stats = monitor.get_stats();
        let missed_blocks = monitor.get_missed_blocks();
        let logger_stats = self.get_stats().await;
        
        let report = format!(
            "=== Missed Block Report ===\n\
             Generated: {}\n\
             \n\
             Monitoring Statistics:\n\
             - Total blocks missed: {}\n\
             - Total blocks recovered: {}\n\
             - Current missed streak: {}\n\
             - Longest missed streak: {}\n\
             - Missed block rate: {:.2}%\n\
             - Recovery rate: {:.2}%\n\
             \n\
             Logger Statistics:\n\
             - Total logged entries: {}\n\
             - Total alerts sent: {}\n\
             - Total errors: {}\n\
             - Log file size: {} bytes\n\
             - Rotation count: {}\n\
             \n\
             Recent Missed Blocks:\n{}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            stats.total_missed,
            stats.total_recovered,
            stats.current_missed_streak,
            stats.longest_missed_streak,
            stats.get_missed_rate(),
            stats.get_recovery_rate(),
            logger_stats.total_logged,
            logger_stats.total_alerts,
            logger_stats.total_errors,
            logger_stats.log_file_size,
            logger_stats.rotation_count,
            self.format_recent_missed_blocks(missed_blocks).await
        );
        
        Ok(report)
    }
    
    /// Format recent missed blocks for report
    async fn format_recent_missed_blocks(&self, missed_blocks: &HashMap<u64, MissedBlock>) -> String {
        if missed_blocks.is_empty() {
            return "No missed blocks in recent history.".to_string();
        }
        
        let mut blocks: Vec<_> = missed_blocks.values().collect();
        blocks.sort_by_key(|b| b.block_number);
        blocks.reverse(); // Most recent first
        
        let mut formatted = String::new();
        for block in blocks.iter().take(10) { // Show last 10
            formatted.push_str(&format!(
                "- Block {}: {:?} ({:?}) - {} attempts, recovered: {}\n",
                block.block_number,
                block.reason,
                block.severity,
                block.recovery_attempts,
                block.recovered
            ));
        }
        
        if blocks.len() > 10 {
            formatted.push_str(&format!("... and {} more missed blocks\n", blocks.len() - 10));
        }
        
        formatted
    }
    
    /// Export missed block data
    pub async fn export_data(&self, format: ExportFormat) -> Result<Vec<u8>> {
        let monitor = self.monitor.read().await;
        let missed_blocks = monitor.get_missed_blocks();
        
        match format {
            ExportFormat::Json => {
                let data = serde_json::to_vec_pretty(&*missed_blocks)?;
                Ok(data)
            }
            ExportFormat::Csv => {
                let csv_data = self.convert_to_csv(&missed_blocks).await;
                Ok(csv_data.into_bytes())
            }
        }
    }
    
    /// Convert missed blocks to CSV
    async fn convert_to_csv(&self, missed_blocks: &HashMap<u64, MissedBlock>) -> String {
        let mut csv = String::new();
        csv.push_str("block_number,reason,severity,expected_timestamp,actual_timestamp,recovery_attempts,recovered,missed_at\n");
        
        for block in missed_blocks.values() {
            csv.push_str(&format!(
                "{},{:?},{:?},{},{},{},{},{}\n",
                block.block_number,
                block.reason,
                block.severity,
                block.expected_timestamp,
                block.actual_timestamp.unwrap_or(0),
                block.recovery_attempts,
                block.recovered,
                block.missed_at
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_secs()
            ));
        }
        
        csv
    }
}

/// Export format for missed block data
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::domain::{BlockInfo, BlockMonitoringConfig, MissedBlockReason, MissedBlockSeverity};
    
    #[tokio::test]
    async fn test_missed_block_logger_creation() {
        let config = MissedBlockLoggerConfig::default();
        let monitor_config = BlockMonitoringConfig::default();
        let monitor = BlockMonitor::new(monitor_config);
        
        let logger = MissedBlockLogger::new(config, monitor);
        assert!(logger.is_ok());
    }
    
    #[tokio::test]
    async fn test_missed_block_logger_config() {
        let config = MissedBlockLoggerConfig::default();
        
        assert_eq!(config.log_retention_days, 30);
        assert_eq!(config.batch_size, 100);
        assert!(config.enable_file_logging);
        assert!(config.enable_console_logging);
    }
    
    #[tokio::test]
    async fn test_logger_stats() {
        let mut stats = LoggerStats::default();
        
        stats.total_logged = 10;
        stats.total_alerts = 5;
        stats.total_errors = 1;
        
        assert_eq!(stats.total_logged, 10);
        assert_eq!(stats.total_alerts, 5);
        assert_eq!(stats.total_errors, 1);
    }
    
    #[tokio::test]
    async fn test_alert_channels() {
        let channels = vec![
            AlertChannel::Console,
            AlertChannel::File,
            AlertChannel::Webhook("https://example.com/webhook".to_string()),
            AlertChannel::Email("admin@example.com".to_string()),
        ];
        
        assert_eq!(channels.len(), 4);
    }
} 