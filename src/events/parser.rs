use crate::{
    events::{domain::SwapEvent, protocol::ProtocolDetectionService},
    shared::{types::H160, types::H256},
};
use crate::Result;
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// Event parser for converting raw transaction data to SwapEvent
pub struct EventParser {
    protocol_detector: ProtocolDetectionService,
}

impl EventParser {
    pub fn new() -> Self {
        Self {
            protocol_detector: ProtocolDetectionService::new(),
        }
    }

    /// Parse a mempool transaction and extract swap events
    pub fn parse_mempool_transaction(
        &self,
        tx_data: &Value,
        block_info: &crate::events::domain::BlockInfo,
    ) -> Result<Vec<SwapEvent>> {
        let mut events = Vec::new();

        // Extract basic transaction information
        let transaction_info = self.extract_transaction_info(tx_data)?;
        
        // Check if this is a swap transaction
        if self.is_swap_transaction(tx_data) {
            if let Some(swap_event) = self.create_swap_event(
                transaction_info,
                block_info,
                crate::events::domain::EventSource::Mempool,
                tx_data,
            )? {
                events.push(swap_event);
            }
        }

        Ok(events)
    }

    /// Parse a Flashbots bundle and extract swap events
    pub fn parse_flashbots_bundle(
        &self,
        bundle_data: &Value,
        block_info: &crate::events::domain::BlockInfo,
    ) -> Result<Vec<SwapEvent>> {
        let mut events = Vec::new();

        if let Some(transactions) = bundle_data.get("transactions").and_then(|t| t.as_array()) {
            for tx_data in transactions {
                let transaction_info = self.extract_transaction_info(tx_data)?;
                
                if self.is_swap_transaction(tx_data) {
                    if let Some(swap_event) = self.create_swap_event(
                        transaction_info,
                        block_info,
                        crate::events::domain::EventSource::Flashbots,
                        tx_data,
                    )? {
                        events.push(swap_event);
                    }
                }
            }
        }

        Ok(events)
    }

    /// Check if a transaction is a swap transaction
    fn is_swap_transaction(&self, tx_data: &Value) -> bool {
        // Check if transaction has input data (contract interaction)
        if let Some(input) = tx_data.get("input").and_then(|i| i.as_str()) {
            if input.len() < 10 {
                return false;
            }

            // Check if the transaction is to a known DEX protocol
            if let Some(to) = tx_data.get("to").and_then(|t| t.as_str()) {
                let to_address = H160::from(to);
                if self.protocol_detector.is_known_protocol(&to_address) {
                    return true;
                }
            }
        }

        false
    }

    /// Extract transaction information from raw data
    fn extract_transaction_info(&self, tx_data: &Value) -> Result<crate::events::domain::TransactionInfo> {
        let hash = self.extract_hash(tx_data, "hash")?;
        let from = self.extract_address(tx_data, "from")?;
        let to = tx_data.get("to")
            .and_then(|t| t.as_str())
            .map(|t| self.extract_address_from_str(t))
            .transpose()?;

        let value = self.extract_wei(tx_data, "value")?;
        let gas_price = self.extract_gas_price(tx_data, "gasPrice")?;
        let gas_limit = self.extract_u64(tx_data, "gas")?;
        let nonce = self.extract_u64(tx_data, "nonce")?;

        Ok(crate::events::domain::TransactionInfo {
            hash,
            from,
            to,
            value,
            gas_price,
            gas_limit,
            gas_used: 0,
            nonce,
        })
    }

    /// Create a swap event from parsed data
    fn create_swap_event(
        &self,
        transaction_info: crate::events::domain::TransactionInfo,
        block_info: &crate::events::domain::BlockInfo,
        source: crate::events::domain::EventSource,
        tx_data: &Value,
    ) -> Result<Option<SwapEvent>> {
        // Extract swap-specific details
        let swap_details = self.extract_swap_details(tx_data)?;
        
        // Detect protocol
        let protocol = if let Some(to) = &transaction_info.to {
            if let Some(protocol) = self.protocol_detector.get_protocol(to) {
                crate::events::domain::ProtocolInfo {
                    name: protocol.name.clone(),
                    version: protocol.version.clone(),
                    address: protocol.router_address,
                }
            } else {
                crate::events::domain::ProtocolInfo {
                    name: "unknown".to_string(),
                    version: "unknown".to_string(),
                    address: *to,
                }
            }
        } else {
            return Ok(None); // No recipient address
        };

        // Create block info
        let block_info_domain = crate::events::domain::BlockInfo {
            number: block_info.number,
            hash: H256::from(block_info.hash.0),
            timestamp: block_info.timestamp,
        };

        // Create the swap event
        let event = SwapEvent::new(
            transaction_info,
            swap_details,
            block_info_domain,
            source,
            protocol,
        );

        Ok(Some(event))
    }

    /// Extract swap details from transaction data
    fn extract_swap_details(&self, tx_data: &Value) -> Result<crate::events::domain::SwapDetails> {
        // This is a simplified implementation
        // In production, you'd implement sophisticated parsing based on protocol-specific logic
        
        let input_data = tx_data.get("input")
            .and_then(|i| i.as_str())
            .unwrap_or("");

        // For now, create placeholder swap details
        // In production, you'd parse the actual swap parameters
        let swap_details = crate::events::domain::SwapDetails {
            token_in: H160([0; 20]), // Placeholder
            token_out: H160([0; 20]), // Placeholder
            amount_in: 0, // Placeholder
            amount_out: 0, // Placeholder
            pool_address: None,
            fee_tier: None,
        };

        Ok(swap_details)
    }

    /// Extract hash from transaction data
    fn extract_hash(&self, tx_data: &Value, field: &str) -> Result<H256> {
        let hash_str = tx_data.get(field)
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing {} field", field))?;

        Ok(H256::from(hash_str))
    }

    /// Extract address from transaction data
    fn extract_address(&self, tx_data: &Value, field: &str) -> Result<H160> {
        let address_str = tx_data.get(field)
            .and_then(|a| a.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing {} field", field))?;

        Ok(H160::from(address_str))
    }

    /// Extract address from string
    fn extract_address_from_str(&self, address_str: &str) -> Result<H160> {
        Ok(H160::from(address_str))
    }

    /// Extract Wei value from transaction data
    fn extract_wei(&self, tx_data: &Value, field: &str) -> Result<u128> {
        let value_str = tx_data.get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing {} field", field))?;

        let value = if value_str.starts_with("0x") {
            let hex_str = value_str.strip_prefix("0x").unwrap();
            if hex_str.is_empty() {
                0
            } else {
                u128::from_str_radix(hex_str, 16)?
            }
        } else {
            value_str.parse::<u128>()?
        };

        Ok(value)
    }

    /// Extract gas price from transaction data
    fn extract_gas_price(&self, tx_data: &Value, field: &str) -> Result<u128> {
        let gas_price_str = tx_data.get(field)
            .and_then(|g| g.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing {} field", field))?;

        let gas_price = if gas_price_str.starts_with("0x") {
            let hex_str = gas_price_str.strip_prefix("0x").unwrap();
            if hex_str.is_empty() {
                0
            } else {
                u128::from_str_radix(hex_str, 16)?
            }
        } else {
            gas_price_str.parse::<u128>()?
        };

        Ok(gas_price)
    }

    /// Extract u64 value from transaction data
    fn extract_u64(&self, tx_data: &Value, field: &str) -> Result<u64> {
        let value_str = tx_data.get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing {} field", field))?;

        let value = if value_str.starts_with("0x") {
            let hex_str = value_str.strip_prefix("0x").unwrap();
            if hex_str.is_empty() {
                0
            } else {
                u64::from_str_radix(hex_str, 16)?
            }
        } else {
            value_str.parse::<u64>()?
        };

        Ok(value)
    }

    /// Get the protocol detector
    pub fn get_protocol_detector(&self) -> &ProtocolDetectionService {
        &self.protocol_detector
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_parser_creation() {
        let parser = EventParser::new();
        assert!(parser.get_protocol_detector().get_all_protocols().len() > 0);
    }

    #[test]
    fn test_transaction_parsing() {
        let parser = EventParser::new();
        let tx_data = json!({
            "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "from": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
            "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
            "value": "0x0",
            "gasPrice": "0x4a817c800",
            "gas": "0x186a0",
            "nonce": "0x0",
            "input": "0x38ed173900000000000000000000000000000000000000000000000000000000"
        });

        let block_info = crate::events::domain::BlockInfo {
            number: 12345,
            hash: H256([1; 32]),
            timestamp: 1234567890,
        };

        let events = parser.parse_mempool_transaction(&tx_data, &block_info).unwrap();
        // The parser should detect this as a swap transaction and return 1 event
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_swap_transaction_detection() {
        let parser = EventParser::new();
        let tx_data = json!({
            "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
            "input": "0x38ed173900000000000000000000000000000000000000000000000000000000"
        });

        assert!(parser.is_swap_transaction(&tx_data));
    }

    #[test]
    fn test_non_swap_transaction() {
        let parser = EventParser::new();
        let tx_data = json!({
            "to": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
            "input": "0x"
        });

        assert!(!parser.is_swap_transaction(&tx_data));
    }
} 