use crate::{
    models::{TransferEvent, BlockInfo},
    error::IndexerResult,
};
use async_trait::async_trait;


#[async_trait]
pub trait EthereumClientTrait: Send + Sync {
   
    async fn get_latest_block_number(&self) -> IndexerResult<u64>;
    
   
    async fn get_block_with_logs(&self, block_number: u64) -> IndexerResult<Option<BlockInfo>>;
    
   
    async fn get_block_header(&self, block_number: u64) -> IndexerResult<Option<BlockHeader>>;
    
    
    async fn get_logs(&self, from_block: u64, to_block: u64, contract_address: Option<ethers::types::H160>) -> IndexerResult<Vec<TransferEvent>>;
    
   
    async fn get_transfer_logs_for_block(&self, block_number: u64) -> IndexerResult<Vec<ethers::types::Log>>;
    
    
    async fn scan_blocks(&self, start_block: u64, end_block: u64, batch_size: u64) -> IndexerResult<Vec<TransferEvent>>;
    
   
    fn get_contract_address(&self) -> ethers::types::H160;
    
    
    async fn get_contract_info(&self) -> IndexerResult<(String, String, u8)>;
    
   
    async fn health_check(&self) -> IndexerResult<bool>;
    
   
    async fn verify_block_finality(&self, block_number: u64, confirmations: u64) -> IndexerResult<bool>;
    
   
    async fn subscribe_to_new_blocks(&self) -> IndexerResult<tokio::sync::mpsc::Receiver<u64>>;
}


#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub number: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
}
