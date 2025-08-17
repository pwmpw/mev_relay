use crate::shared::types::{H160, H256, Wei, GasPrice};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core domain entity representing a swap event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwapEvent {
    pub id: EventId,
    pub transaction: TransactionInfo,
    pub swap_details: SwapDetails,
    pub block_info: BlockInfo,
    pub source: EventSource,
    pub protocol: ProtocolInfo,
    pub metadata: EventMetadata,
}

/// Unique identifier for an event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventId(pub String);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionInfo {
    pub hash: H256,
    pub from: H160,
    pub to: Option<H160>,
    pub value: Wei,
    pub gas_price: GasPrice,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub nonce: u64,
}

/// Swap-specific details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwapDetails {
    pub token_in: H160,
    pub token_out: H160,
    pub amount_in: Wei,
    pub amount_out: Wei,
    pub pool_address: Option<H160>,
    pub fee_tier: Option<u32>,
}

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockInfo {
    pub number: u64,
    pub hash: H256,
    pub timestamp: u64,
}

/// Event source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSource {
    Mempool,
    Flashbots,
    Block,
}

impl std::fmt::Display for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSource::Mempool => write!(f, "Mempool"),
            EventSource::Flashbots => write!(f, "Flashbots"),
            EventSource::Block => write!(f, "Block"),
        }
    }
}

/// Protocol information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolInfo {
    pub name: String,
    pub version: String,
    pub address: H160,
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventMetadata {
    pub created_at: u64,
    pub processed_at: Option<u64>,
    pub tags: Vec<String>,
}

impl EventMetadata {
    pub fn new() -> Self {
        Self {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            processed_at: None,
            tags: Vec::new(),
        }
    }

    pub fn mark_processed(&mut self) {
        self.processed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapEvent {
    /// Create a new swap event
    pub fn new(
        transaction: TransactionInfo,
        swap_details: SwapDetails,
        block_info: BlockInfo,
        source: EventSource,
        protocol: ProtocolInfo,
    ) -> Self {
        Self {
            id: EventId::new(),
            transaction,
            swap_details,
            block_info,
            source,
            protocol,
            metadata: EventMetadata::new(),
        }
    }

    /// Mark the event as processed
    pub fn mark_processed(&mut self) {
        self.metadata.mark_processed();
    }

    /// Add a tag to the event
    pub fn add_tag(&mut self, tag: String) {
        self.metadata.add_tag(tag);
    }

    /// Check if the event is from mempool
    pub fn is_from_mempool(&self) -> bool {
        matches!(self.source, EventSource::Mempool)
    }

    /// Check if the event is from flashbots
    pub fn is_from_flashbots(&self) -> bool {
        matches!(self.source, EventSource::Flashbots)
    }

    /// Get the event age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if now > self.metadata.created_at {
            now - self.metadata.created_at
        } else {
            0
        }
    }

    /// Validate the event
    pub fn validate(&self) -> Result<(), String> {
        if self.transaction.hash.0.iter().all(|&b| b == 0) {
            return Err("Invalid transaction hash".to_string());
        }

        if self.transaction.from.0.iter().all(|&b| b == 0) {
            return Err("Invalid from address".to_string());
        }

        if let Some(to_addr) = self.transaction.to {
            if to_addr.0.iter().all(|&b| b == 0) {
                return Err("Invalid to address".to_string());
            }
        } else {
            return Err("Missing to address".to_string());
        }

        if self.block_info.number == 0 {
            return Err("Invalid block number".to_string());
        }

        Ok(())
    }
}

impl Default for SwapEvent {
    fn default() -> Self {
        Self {
            id: EventId::default(),
                                transaction: TransactionInfo {
                        hash: H256([0; 32]),
                        from: H160([0; 20]),
                        to: Some(H160([0; 20])),
                        value: 0,
                        gas_price: 0,
                        gas_limit: 0,
                        gas_used: 0,
                        nonce: 0,
                    },
            swap_details: SwapDetails {
                token_in: H160([0; 20]),
                token_out: H160([0; 20]),
                amount_in: 0,
                amount_out: 0,
                pool_address: None,
                fee_tier: None,
            },
            block_info: BlockInfo {
                number: 0,
                hash: H256([0; 32]),
                timestamp: 0,
            },
            source: EventSource::Mempool,
            protocol: ProtocolInfo {
                name: "unknown".to_string(),
                version: "unknown".to_string(),
                address: H160([0; 20]),
            },
            metadata: EventMetadata::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{H160, H256};

    #[test]
    fn test_swap_event_creation() {
        let transaction = TransactionInfo {
            hash: H256([1; 32]),
            from: H160([1; 20]),
            to: Some(H160([2; 20])),
            value: 0,
            gas_price: 20_000_000_000,
            gas_limit: 100000,
            gas_used: 100000,
            nonce: 1,
        };

        let swap_details = SwapDetails {
            token_in: H160([3; 20]),
            token_out: H160([4; 20]),
            amount_in: 1_000_000_000_000_000_000,
            amount_out: 950_000_000_000_000_000,
            pool_address: Some(H160([5; 20])),
            fee_tier: Some(3000),
        };

        let block_info = BlockInfo {
            number: 12345678,
            hash: H256([6; 32]),
            timestamp: 1234567890,
        };

        let protocol = ProtocolInfo {
            name: "Uniswap V2".to_string(),
            version: "2.0".to_string(),
            address: H160([7; 20]),
        };

        let event = SwapEvent::new(
            transaction,
            swap_details,
            block_info,
            EventSource::Mempool,
            protocol,
        );

        assert!(!event.id.as_str().is_empty());
        assert!(event.is_from_mempool());
        assert!(!event.is_from_flashbots());
        assert_eq!(event.age_seconds(), 0);
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_event_validation() {
        let mut event = SwapEvent::default();
        assert!(event.validate().is_err());

        event.transaction.hash = H256([1; 32]);
        event.transaction.from = H160([1; 20]);
        event.transaction.to = Some(H160([2; 20]));
        event.block_info.number = 1;
        
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_event_metadata() {
        let mut event = SwapEvent::default();
        assert!(event.metadata.processed_at.is_none());

        event.mark_processed();
        assert!(event.metadata.processed_at.is_some());

        event.add_tag("high_value".to_string());
        event.add_tag("high_value".to_string()); // Duplicate
        assert_eq!(event.metadata.tags.len(), 1);
        assert!(event.metadata.tags.contains(&"high_value".to_string()));
    }
} 