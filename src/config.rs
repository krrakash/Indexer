use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub contract_address: String,
    pub start_block: u64,
    pub end_block: u64,
    pub database_path: String,
    pub websocket: bool,
    pub use_alloy: bool,
    pub use_adaptive_ethers: bool,


    pub finality_depth: u64,
    pub overlap_size: u64,
    pub tail_http_batch_size: u64,
    pub batch_size: u64,
}

impl Config {
    pub fn new(
        rpc_url: String,
        contract_address: String,
        start_block: u64,
        end_block: u64,
        database_path: String,
        websocket: bool,
    ) -> Self {
        Self {
            rpc_url,
            contract_address,
            start_block,
            end_block,
            database_path,
            websocket,
            use_alloy: false,
            use_adaptive_ethers: false,
            
            finality_depth: 12,
            overlap_size: 32,
            tail_http_batch_size: 50,
            batch_size: 300,
        }
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.rpc_url.is_empty() {
            return Err("RPC URL cannot be empty".to_string());
        }
        
        if self.contract_address.is_empty() {
            return Err("Contract address cannot be empty".to_string());
        }
        
        if !self.contract_address.starts_with("0x") || self.contract_address.len() != 42 {
            return Err("Invalid contract address format".to_string());
        }
        
        if self.database_path.is_empty() {
            return Err("Database path cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    pub fn database_path(&self) -> PathBuf {
        PathBuf::from(&self.database_path)
    }
} 