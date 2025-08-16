use thiserror::Error;

#[derive(Error, Debug)]
pub enum MevRelayError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Web3 error: {0}")]
    Web3(#[from] web3::Error),

    #[error("Ethers error: {0}")]
    Ethers(#[from] ethers::providers::ProviderError),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel send error: {0}")]
    ChannelSend(String),

    #[error("Channel receive error: {0}")]
    ChannelReceive(String),

    #[error("Invalid Ethereum address: {0}")]
    InvalidAddress(String),

    #[error("Invalid transaction hash: {0}")]
    InvalidTxHash(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Mempool monitoring error: {0}")]
    Mempool(String),

    #[error("Flashbots error: {0}")]
    Flashbots(String),

    #[error("Event parsing error: {0}")]
    EventParsing(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error("Protocol detection error: {0}")]
    ProtocolDetection(String),

    #[error("Message broker error: {0}")]
    MessageBroker(String),

    #[error("Health check error: {0}")]
    HealthCheck(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<config::ConfigError> for MevRelayError {
    fn from(err: config::ConfigError) -> Self {
        MevRelayError::Config(err.to_string())
    }
}

impl From<tokio::sync::mpsc::error::SendError<String>> for MevRelayError {
    fn from(err: tokio::sync::mpsc::error::SendError<String>) -> Self {
        MevRelayError::ChannelSend(err.to_string())
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for MevRelayError {
    fn from(err: tokio::sync::broadcast::error::RecvError) -> Self {
        MevRelayError::ChannelReceive(err.to_string())
    }
}

impl From<tokio::sync::mpsc::error::SendError<crate::events::domain::SwapEvent>> for MevRelayError {
    fn from(err: tokio::sync::mpsc::error::SendError<crate::events::domain::SwapEvent>) -> Self {
        MevRelayError::ChannelSend(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let config_error = MevRelayError::Config("Invalid config".to_string());
        assert!(matches!(config_error, MevRelayError::Config(_)));

        let mempool_error = MevRelayError::Mempool("Connection failed".to_string());
        assert!(matches!(mempool_error, MevRelayError::Mempool(_)));
    }

    #[test]
    fn test_error_display() {
        let error = MevRelayError::Config("Test error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Test error"));
    }
} 