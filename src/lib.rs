pub mod shared;
pub mod events;

// Re-export commonly used types
pub use shared::types::{H160, H256};
pub use events::domain::EventId;
pub use events::domain::SwapEvent;
pub use events::filter::PoolFilter;

// Re-export result type
pub type Result<T> = anyhow::Result<T>; 