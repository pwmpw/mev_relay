use crate::events::domain::SwapEvent;
use crate::Result;
use tracing::{debug, info, warn};

/// Event normalizer for ensuring consistency and standardization
pub struct EventNormalizer {
    // Configuration for normalization rules
    normalize_addresses: bool,
    normalize_timestamps: bool,
    validate_events: bool,
}

impl EventNormalizer {
    pub fn new() -> Self {
        Self {
            normalize_addresses: true,
            normalize_timestamps: true,
            validate_events: true,
        }
    }

    /// Normalize a single swap event
    pub fn normalize_event(&self, mut event: SwapEvent) -> Result<SwapEvent> {
        if self.validate_events {
            event.validate()
                .map_err(|e| anyhow::anyhow!("Event validation failed: {}", e))?;
        }

        if self.normalize_addresses {
            self.normalize_addresses(&mut event)?;
        }

        if self.normalize_timestamps {
            self.normalize_timestamps(&mut event)?;
        }

        // Add normalization tags
        event.add_tag("normalized".to_string());
        event.add_tag("processed".to_string());

        info!("Event {} normalized successfully", event.id.as_str());
        Ok(event)
    }

    /// Normalize multiple events
    pub fn normalize_events(&self, events: Vec<SwapEvent>) -> Result<Vec<SwapEvent>> {
        let mut normalized_events = Vec::new();
        let mut error_count = 0;

        for event in events {
            match self.normalize_event(event) {
                Ok(normalized_event) => {
                    normalized_events.push(normalized_event);
                }
                Err(e) => {
                    error_count += 1;
                    warn!("Failed to normalize event: {}", e);
                    // Continue processing other events
                }
            }
        }

        if error_count > 0 {
            warn!("Failed to normalize {} events", error_count);
        }

        info!("Normalized {} events successfully", normalized_events.len());
        Ok(normalized_events)
    }

    /// Normalize addresses in the event
    fn normalize_addresses(&self, event: &mut SwapEvent) -> Result<()> {
        // Ensure addresses are properly formatted
        // This could include checksum validation, case normalization, etc.
        
        // For now, just validate that addresses are not zero
        if event.transaction.from.0.iter().all(|&b| b == 0) {
            return Err(anyhow::anyhow!("Invalid from address"));
        }

        if let Some(to_addr) = event.transaction.to {
            if to_addr.0.iter().all(|&b| b == 0) {
                return Err(anyhow::anyhow!("Invalid to address"));
            }
        } else {
            return Err(anyhow::anyhow!("Missing to address"));
        }

        if event.swap_details.token_in.0.iter().all(|&b| b == 0) {
            return Err(anyhow::anyhow!("Invalid token_in address"));
        }

        if event.swap_details.token_out.0.iter().all(|&b| b == 0) {
            return Err(anyhow::anyhow!("Invalid token_out address"));
        }

        Ok(())
    }

    /// Normalize timestamps in the event
    fn normalize_timestamps(&self, event: &mut SwapEvent) -> Result<()> {
        let now = crate::shared::utils::time::now();
        
        // Ensure block timestamp is reasonable
        if event.block_info.timestamp > now + 3600 {
            // Block timestamp is more than 1 hour in the future
            warn!("Block timestamp {} is in the future, adjusting", event.block_info.timestamp);
            event.block_info.timestamp = now;
        }

        // Ensure created_at timestamp is reasonable
        if event.metadata.created_at > now + 60 {
            // Created timestamp is more than 1 minute in the future
            warn!("Created timestamp {} is in the future, adjusting", event.metadata.created_at);
            event.metadata.created_at = now;
        }

        Ok(())
    }

    /// Validate event data consistency
    pub fn validate_event_consistency(&self, event: &SwapEvent) -> Result<()> {
        // Check that amounts are reasonable
        if event.swap_details.amount_in == 0 {
            return Err(anyhow::anyhow!("Amount in cannot be zero"));
        }

        if event.swap_details.amount_out == 0 {
            return Err(anyhow::anyhow!("Amount out cannot be zero"));
        }

        // Check that gas price is reasonable
        if event.transaction.gas_price == 0 {
            return Err(anyhow::anyhow!("Gas price cannot be zero"));
        }

        // Check that gas limit is reasonable
        if event.transaction.gas_limit == 0 {
            return Err(anyhow::anyhow!("Gas limit cannot be zero"));
        }

        // Check that nonce is reasonable
        if event.transaction.nonce > 1000000 {
            warn!("Unusually high nonce: {}", event.transaction.nonce);
        }

        Ok(())
    }

    /// Get normalization statistics
    pub fn get_normalization_stats(&self) -> NormalizationStats {
        NormalizationStats {
            total_events_processed: 0, // This would be tracked in production
            successful_normalizations: 0,
            failed_normalizations: 0,
            validation_errors: 0,
        }
    }

    /// Configure normalization options
    pub fn configure(&mut self, options: NormalizationOptions) {
        self.normalize_addresses = options.normalize_addresses;
        self.normalize_timestamps = options.normalize_timestamps;
        self.validate_events = options.validate_events;
    }
}

impl Default for EventNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalization options
#[derive(Debug, Clone)]
pub struct NormalizationOptions {
    pub normalize_addresses: bool,
    pub normalize_timestamps: bool,
    pub validate_events: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            normalize_addresses: true,
            normalize_timestamps: true,
            validate_events: true,
        }
    }
}

/// Normalization statistics
#[derive(Debug, Clone)]
pub struct NormalizationStats {
    pub total_events_processed: u64,
    pub successful_normalizations: u64,
    pub failed_normalizations: u64,
    pub validation_errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::domain::{EventSource, ProtocolInfo, TransactionInfo, SwapDetails, BlockInfo};

    #[test]
    fn test_event_normalizer_creation() {
        let normalizer = EventNormalizer::new();
        assert!(normalizer.normalize_addresses);
        assert!(normalizer.normalize_timestamps);
        assert!(normalizer.validate_events);
    }

    #[test]
    fn test_normalization_options() {
        let mut normalizer = EventNormalizer::new();
        let options = NormalizationOptions {
            normalize_addresses: false,
            normalize_timestamps: false,
            validate_events: false,
        };

        normalizer.configure(options);
        assert!(!normalizer.normalize_addresses);
        assert!(!normalizer.normalize_timestamps);
        assert!(!normalizer.validate_events);
    }

    #[test]
    fn test_event_validation() {
        let normalizer = EventNormalizer::new();
        let event = SwapEvent::default();
        
        // This should fail validation since the default event has invalid data
        let result = normalizer.validate_event_consistency(&event);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalization_stats() {
        let normalizer = EventNormalizer::new();
        let stats = normalizer.get_normalization_stats();
        assert_eq!(stats.total_events_processed, 0);
        assert_eq!(stats.successful_normalizations, 0);
    }
} 