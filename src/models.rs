use serde::{Deserialize, Serialize};
use ethers::types::{Log, H160, H256, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    pub from: H160,
    pub to: H160,
    pub value: U256,
    pub transaction_hash: H256,
    pub block_number: u64,
    pub log_index: u32,
    pub block_hash: H256,
    pub contract_address: H160,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub number: u64,
    pub hash: H256,
    pub parent_hash: H256,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TransferRecord {
    pub id: i64,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub transaction_hash: String,
    pub block_number: u64,
    pub log_index: u32,
    pub block_hash: String,
    pub contract_address: String,
    pub timestamp: Option<u64>,
    pub created_at: i64,
}


#[derive(Debug, Clone)]
pub struct IndexingProgress {
    pub last_processed_block: u64,
    pub total_events_processed: u64,
    pub last_processed_timestamp: i64,
}







pub const TRANSFER_EVENT_TOPIC: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

impl TransferEvent {
    pub fn from_log(log: Log, contract_address: H160) -> Option<Self> {
        use tracing::warn;

        if log.topics.len() != 3 {
            warn!("Invalid topics count: {} (expected 3)", log.topics.len());
            return None;
        }
        
        let expected_topic = H256::from_slice(&hex::decode(TRANSFER_EVENT_TOPIC).ok()?);
        if log.topics[0] != expected_topic {
            warn!("Invalid topic0: 0x{:x} (expected 0x{:x})", log.topics[0], expected_topic);
            return None;
        }
        
        let from = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let to = H160::from_slice(&log.topics[2].as_bytes()[12..]);
        
        if log.data.len() != 32 {
            warn!("Invalid data length: {} (expected 32)", log.data.len());
            return None;
        }
        
        let value = U256::from_big_endian(&log.data);
        
        let transaction_hash = log.transaction_hash?;
        let block_number = log.block_number?.as_u64();
        let log_index = log.log_index?.as_u32();
        let block_hash = log.block_hash?;
        
        Some(Self {
            from,
            to,
            value,
            transaction_hash,
            block_number,
            log_index,
            block_hash,
            contract_address,
            timestamp: None,
        })
    }
    
    pub fn to_record(&self) -> TransferRecord {
        TransferRecord {
            id: 0, 
            from_address: format!("0x{:x}", self.from),
            to_address: format!("0x{:x}", self.to),
            value: self.value.to_string(),
            transaction_hash: format!("0x{:x}", self.transaction_hash),
            block_number: self.block_number,
            log_index: self.log_index,
            block_hash: format!("0x{:x}", self.block_hash),
            contract_address: format!("0x{:x}", self.contract_address),
            timestamp: self.timestamp,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

impl BlockInfo {

} 