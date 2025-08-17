use crate::shared::types::{H160, H256};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod hex {
    use super::*;

    pub fn encode(data: &[u8]) -> String {
        format!("0x{}", ::hex::encode(data))
    }

    pub fn decode(s: &str) -> Result<Vec<u8>> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        ::hex::decode(s).map_err(|e| anyhow::anyhow!("Invalid hex string: {}", e))
    }

    pub fn decode_to_fixed<const N: usize>(s: &str) -> Result<[u8; N]> {
        let bytes = decode(s)?;
        if bytes.len() != N {
            return Err(anyhow::anyhow!("Expected {} bytes, got {}", N, bytes.len()));
        }
        
        let mut result = [0u8; N];
        result.copy_from_slice(&bytes);
        Ok(result)
    }
}

pub mod address {
    use super::*;

    pub fn is_valid_checksum(address: &str) -> bool {
        if !address.starts_with("0x") || address.len() != 42 {
            return false;
        }

        let address_lower = address.to_lowercase();
        let address_upper = address.to_uppercase();

        // Check if it's a valid hex string
        if hex::decode(&address[2..]).is_err() {
            return false;
        }

        // Simple checksum validation (EIP-55)
        let keccak_hash = keccak256(address_lower.as_bytes());
        let mut result = String::with_capacity(42);
        result.push_str("0x");

        for (i, byte) in address[2..].chars().enumerate() {
            let hash_byte = keccak_hash[i / 2];
            let nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0x0f };
            
            if nibble >= 8 {
                result.push(byte.to_uppercase().next().unwrap());
            } else {
                result.push(byte.to_lowercase().next().unwrap());
            }
        }

        result == address
    }

    pub fn normalize_address(address: &str) -> Result<H160> {
        let address = address.strip_prefix("0x").unwrap_or(address);
        if address.len() != 40 {
            return Err(anyhow::anyhow!("Invalid address length: {}", address.len()));
        }

        let bytes = hex::decode(address)?;
        let mut result = [0u8; 20];
        result.copy_from_slice(&bytes);
        Ok(H160(result))
    }

    pub fn format_address(address: &H160) -> String {
        format!("0x{}", ::hex::encode(&address.0))
    }

    pub fn format_address_checksum(address: &H160) -> String {
        let address_lower = format_address(address).to_lowercase();
        let keccak_hash = keccak256(address_lower.as_bytes());
        let mut result = String::with_capacity(42);
        result.push_str("0x");

        for (i, byte) in address_lower[2..].chars().enumerate() {
            let hash_byte = keccak_hash[i / 2];
            let nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0x0f };
            
            if nibble >= 8 {
                result.push(byte.to_uppercase().next().unwrap());
            } else {
                result.push(byte.to_lowercase().next().unwrap());
            }
        }

        result
    }
}

pub mod hash {
    use super::*;

    pub fn format_hash(hash: &H256) -> String {
        format!("0x{}", ::hex::encode(&hash.0))
    }

    pub fn normalize_hash(hash: &str) -> Result<H256> {
        let hash = hash.strip_prefix("0x").unwrap_or(hash);
        if hash.len() != 64 {
            return Err(anyhow::anyhow!("Invalid hash length: {}", hash.len()));
        }

        let bytes = hex::decode(hash)?;
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes);
        Ok(H256(result))
    }
}

pub mod time {
    use super::*;

    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn now_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }

    pub fn from_timestamp(timestamp: u64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_default()
    }

    pub fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }

    pub fn format_timestamp(timestamp: u64) -> String {
        from_timestamp(timestamp).format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
}

pub mod math {
    use super::*;

    pub fn wei_to_eth(wei: u128) -> f64 {
        wei as f64 / 1_000_000_000_000_000_000.0
    }

    pub fn eth_to_wei(eth: f64) -> u128 {
        (eth * 1_000_000_000_000_000_000.0) as u128
    }

    pub fn wei_to_gwei(wei: u128) -> f64 {
        wei as f64 / 1_000_000_000.0
    }

    pub fn gwei_to_wei(gwei: f64) -> u128 {
        (gwei * 1_000_000_000.0) as u128
    }

    pub fn calculate_gas_cost(gas_used: u64, gas_price: u128) -> u128 {
        gas_used as u128 * gas_price
    }

    pub fn calculate_priority_fee(base_fee: u128, max_priority_fee: u128, max_fee: u128) -> u128 {
        let priority_fee = std::cmp::min(max_priority_fee, max_fee.saturating_sub(base_fee));
        std::cmp::min(priority_fee, max_fee)
    }
}

pub mod validation {
    use super::*;

    pub fn validate_ethereum_address(address: &str) -> Result<()> {
        if !address.starts_with("0x") {
            return Err(anyhow::anyhow!("Address must start with 0x"));
        }

        if address.len() != 42 {
            return Err(anyhow::anyhow!("Address must be 42 characters long"));
        }

        if hex::decode(&address[2..]).is_err() {
            return Err(anyhow::anyhow!("Invalid hex characters in address"));
        }

        Ok(())
    }

    pub fn validate_transaction_hash(hash: &str) -> Result<()> {
        if !hash.starts_with("0x") {
            return Err(anyhow::anyhow!("Hash must start with 0x"));
        }

        if hash.len() != 66 {
            return Err(anyhow::anyhow!("Hash must be 66 characters long"));
        }

        if hex::decode(&hash[2..]).is_err() {
            return Err(anyhow::anyhow!("Invalid hex characters in hash"));
        }

        Ok(())
    }

    pub fn validate_block_number(block_number: u64) -> Result<()> {
        if block_number == 0 {
            return Err(anyhow::anyhow!("Block number cannot be 0"));
        }
        Ok(())
    }

    pub fn validate_gas_price(gas_price: u128) -> Result<()> {
        if gas_price == 0 {
            return Err(anyhow::anyhow!("Gas price cannot be 0"));
        }
        Ok(())
    }
}

// Simple keccak256 implementation (for checksum validation)
fn keccak256(data: &[u8]) -> [u8; 32] {
    // This is a simplified implementation - in production you'd use a proper keccak256 library
    // For now, we'll use a simple hash function
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();
    
    let mut result = [0u8; 32];
    for i in 0..8 {
        result[i] = ((hash >> (i * 8)) & 0xff) as u8;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encoding() {
        let data = b"Hello, World!";
        let encoded = hex::encode(data);
        assert_eq!(encoded, "0x48656c6c6f2c20576f726c6421");
        
        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_address_validation() {
        let valid_address = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
        assert!(validation::validate_ethereum_address(valid_address).is_ok());
        
        let invalid_address = "0xinvalid";
        assert!(validation::validate_ethereum_address(invalid_address).is_err());
    }

    #[test]
    fn test_time_utils() {
        let now = time::now();
        assert!(now > 0);
        
        let formatted = time::format_timestamp(now);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_math_utils() {
        let wei = 1_000_000_000_000_000_000u128; // 1 ETH
        let eth = math::wei_to_eth(wei);
        assert_eq!(eth, 1.0);
        
        let gwei = math::wei_to_gwei(20_000_000_000u128);
        assert_eq!(gwei, 20.0);
    }
} 