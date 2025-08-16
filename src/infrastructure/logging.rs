use crate::infrastructure::config::LoggingConfig;
use anyhow::Result;
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, format::JsonFields, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

pub struct Logging {
    config: LoggingConfig,
}

impl Logging {
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    pub fn init(&self) -> Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                let level = self.config.level.parse::<Level>().unwrap_or(Level::INFO);
                EnvFilter::new(format!("mev_relay={},tower=info", level))
            });

        let registry = Registry::default().with(env_filter);

        match self.config.format.as_str() {
            "json" => self.init_json_logging(registry)?,
            "text" => self.init_text_logging(registry)?,
            _ => {
                tracing::warn!("Unknown logging format '{}', defaulting to JSON", self.config.format);
                self.init_json_logging(registry)?;
            }
        }

        tracing::info!("Logging initialized with format: {}", self.config.format);
        Ok(())
    }

    fn init_json_logging(&self, registry: Registry) -> Result<()> {
        let json_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_timer(UtcTime::rfc_3339())
            .json();

        registry.with(json_layer).init();
        Ok(())
    }

    fn init_text_logging(&self, registry: Registry) -> Result<()> {
        let text_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_timer(UtcTime::rfc_3339())
            .with_ansi(true);

        registry.with(text_layer).init();
        Ok(())
    }

    pub fn set_level(&mut self, level: &str) -> Result<()> {
        let _: Level = level.parse()
            .map_err(|e| anyhow::anyhow!("Invalid log level '{}': {}", level, e))?;
        
        self.config.level = level.to_string();
        tracing::info!("Log level set to: {}", level);
        Ok(())
    }

    pub fn get_config(&self) -> &LoggingConfig {
        &self.config
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self::new(LoggingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_creation() {
        let config = LoggingConfig::default();
        let logging = Logging::new(config);
        assert_eq!(logging.get_config().format, "json");
    }

    #[test]
    fn test_logging_set_level() {
        let mut logging = Logging::default();
        assert!(logging.set_level("debug").is_ok());
        assert!(logging.set_level("invalid").is_err());
    }
} 