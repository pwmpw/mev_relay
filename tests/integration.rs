use mev_relay::{
    events::parser::EventParser,
    events::domain::SwapEvent,
    Result,
};

#[tokio::test]
async fn test_basic_event_parsing() -> Result<()> {
    // Test basic event parsing functionality
    let parser = EventParser::new();
    
    // Test with sample transaction data
    let tx_data = serde_json::json!({
        "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "from": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
        "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
        "value": "0x0",
        "gasPrice": "0x59682f00",
        "gas": "0x186a0",
        "nonce": "0x0",
        "input": "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
    });
    
    let block_info = mev_relay::events::domain::BlockInfo {
        number: 123456,
        hash: mev_relay::shared::types::H256([0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90]),
        timestamp: 12345678,
    };
    
    let result = parser.parse_mempool_transaction(&tx_data, &block_info)?;
    assert!(!result.is_empty());
    
    Ok(())
} 