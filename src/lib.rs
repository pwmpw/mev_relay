// MEV Relay - Domain-Driven Design Architecture
// 
// Core domains:
// - monitoring: Ethereum mempool and Flashbots monitoring
// - events: Event parsing, normalization, and processing
// - messaging: Redis pub/sub and event distribution
// - infrastructure: Configuration, logging, metrics, health checks
// - shared: Common types, utilities, and error handling

pub mod monitoring;
pub mod events;
pub mod messaging;
pub mod infrastructure;
pub mod shared;

// Re-exports for convenience
pub use infrastructure::config::Config;
pub use infrastructure::app::MevRelay;
pub use shared::error::MevRelayError;
pub use events::domain::SwapEvent;
pub use shared::types::*;

pub type Result<T> = anyhow::Result<T>; 