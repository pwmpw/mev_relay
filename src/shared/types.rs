use serde::{Deserialize, Serialize};
use std::fmt;

pub type Address = [u8; 20];
pub type TxHash = [u8; 32];
pub type BlockNumber = u64;
pub type GasPrice = u128;
pub type Wei = u128;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct H160(pub [u8; 20]);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct H256(pub [u8; 32]);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockIdentifier {
    pub number: BlockNumber,
    pub hash: H256,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub hash: H256,
    pub from: H160,
    pub to: Option<H160>,
    pub value: Wei,
    pub gas_price: GasPrice,
    pub gas_limit: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub block_number: Option<BlockNumber>,
    pub block_hash: Option<H256>,
    pub transaction_index: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub address: H160,
    pub token0: H160,
    pub token1: H160,
    pub fee_tier: u32,
    pub protocol: String,
}

impl fmt::Display for H160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl fmt::Display for H256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl From<[u8; 20]> for H160 {
    fn from(bytes: [u8; 20]) -> Self {
        H160(bytes)
    }
}

impl From<[u8; 32]> for H256 {
    fn from(bytes: [u8; 32]) -> Self {
        H256(bytes)
    }
}

impl From<H160> for [u8; 20] {
    fn from(h160: H160) -> Self {
        h160.0
    }
}

impl From<H256> for [u8; 32] {
    fn from(h256: H256) -> Self {
        h256.0
    }
}

impl From<&str> for H160 {
    fn from(s: &str) -> Self {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; 20];
        hex::decode_to_slice(s, &mut bytes).unwrap_or_default();
        H160(bytes)
    }
}

impl From<&str> for H256 {
    fn from(s: &str) -> Self {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes).unwrap_or_default();
        H256(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h160_display() {
        let addr = H160([1; 20]);
        let display = format!("{}", addr);
        assert!(display.starts_with("0x"));
        assert_eq!(display.len(), 42);
    }

    #[test]
    fn test_h256_display() {
        let hash = H256([1; 32]);
        let display = format!("{}", hash);
        assert!(display.starts_with("0x"));
        assert_eq!(display.len(), 66);
    }

    #[test]
    fn test_from_string() {
        let addr_str = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
        let addr = H160::from(addr_str);
        assert_eq!(format!("{}", addr), addr_str);
    }
} 