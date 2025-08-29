pub mod adaptive_ethers;
pub mod alloy_client;
pub mod client_trait;

pub mod config;
pub mod database;
pub mod error;
pub mod ethereum;
pub mod indexer;
pub mod models;

pub use client_trait::EthereumClientTrait;
pub use config::Config;
pub use error::{IndexerError, IndexerResult};
pub use indexer::Indexer;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use ethers::types::{Log, H160, H256, U256, Bytes, U64};

    #[test]
    fn test_config_validation() {
        let valid_config = Config {
            rpc_url: "http://localhost:8545".to_string(),
            contract_address: "0x1234567890123456789012345678901234567890".to_string(),
            start_block: 0,
            database_path: "test.db".to_string(),
            websocket: false,
        };
        
        assert!(valid_config.validate().is_ok());
        
        let invalid_config = Config {
            rpc_url: "".to_string(),
            contract_address: "0x1234567890123456789012345678901234567890".to_string(),
            start_block: 0,
            database_path: "test.db".to_string(),
            websocket: false,
        };
        
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_database_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let db = Database::new(&db_path);
        assert!(db.is_ok());
        

        let db = db.unwrap();
        let progress = db.get_progress();
        assert!(progress.is_ok());
        
        let progress = progress.unwrap();
        assert_eq!(progress.last_processed_block, 0);
        assert_eq!(progress.total_events_processed, 0);
    }

    #[test]
    fn test_transfer_event_parsing() {

        let contract_address = H160::from_slice(&hex::decode("1234567890123456789012345678901234567890").unwrap());
        let from_address = H160::from_slice(&hex::decode("1111111111111111111111111111111111111111").unwrap());
        let to_address = H160::from_slice(&hex::decode("2222222222222222222222222222222222222222").unwrap());
        let value = U256::from(1000000000000000000u64);
        
        let transfer_topic = H256::from_slice(&hex::decode(crate::models::TRANSFER_EVENT_TOPIC).unwrap());
        let from_topic = H256::from_slice(&hex::decode("0000000000000000000000001111111111111111111111111111111111111111").unwrap());
        let to_topic = H256::from_slice(&hex::decode("0000000000000000000000002222222222222222222222222222222222222222").unwrap());
        
        let mut data = [0u8; 32];
        value.to_big_endian(&mut data);
        
        let log = Log {
            address: contract_address,
            topics: vec![transfer_topic, from_topic, to_topic],
            data: Bytes::from(data),
            block_hash: Some(H256::from_slice(&hex::decode("3333333333333333333333333333333333333333333333333333333333333333").unwrap())),
            block_number: Some(U64::from(1000u64)),
            transaction_hash: Some(H256::from_slice(&hex::decode("4444444444444444444444444444444444444444444444444444444444444444").unwrap())),
            transaction_index: Some(U64::from(0u64)),
            log_index: Some(U256::from(0u64)),
            transaction_log_index: Some(U256::from(0u64)),
            log_type: None,
            removed: Some(false),
        };
        
        let transfer_event = TransferEvent::from_log(log, contract_address);
        assert!(transfer_event.is_some());
        
        let transfer_event = transfer_event.unwrap();
        assert_eq!(transfer_event.from, from_address);
        assert_eq!(transfer_event.to, to_address);
        assert_eq!(transfer_event.value, value);
        assert_eq!(transfer_event.block_number, 1000);
        assert_eq!(transfer_event.log_index, 0);
    }

    #[test]
    fn test_error_types() {
        let db_error = rusqlite::Error::InvalidPath("test".to_string().into());
        let indexer_error: IndexerError = db_error.into();
        
        match indexer_error {
            IndexerError::Database(_) => assert!(true),
            _ => assert!(false, "Expected Database error"),
        }
    }
} 