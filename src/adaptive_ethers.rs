use ethers::{
    providers::{Http, Provider, Ws, Middleware},
    types::{Block, Log, Filter, BlockNumber, H160, H256},
    utils::keccak256,
};
use crate::{
    models::{TransferEvent, BlockInfo},
    error::{IndexerError, IndexerResult},
};
use crate::client_trait::{EthereumClientTrait, BlockHeader};
use std::sync::Arc;
use tracing::{info, warn, debug};
use futures::StreamExt;
use tokio::time::{sleep, Duration};
use async_trait::async_trait;

#[derive(Clone)]
pub struct AdaptiveEthereumClient {
    provider: Arc<Provider<Http>>,
    ws_provider: Option<Arc<Provider<Ws>>>,
    pub contract_address: H160,
}

#[async_trait]
impl EthereumClientTrait for AdaptiveEthereumClient {
    async fn get_latest_block_number(&self) -> IndexerResult<u64> {
        let block = self.provider.get_block(BlockNumber::Latest).await?
            .ok_or_else(|| IndexerError::Ethereum("Failed to get latest block".to_string()))?;
        
        Ok(block.number.unwrap_or_default().as_u64())
    }

    async fn get_block_with_logs(&self, block_number: u64) -> IndexerResult<Option<BlockInfo>> {
        let block = self.get_block(block_number).await?;
        
        match block {
            Some(block) => {
                let block_info = BlockInfo {
                    number: block.number.unwrap_or_default().as_u64(),
                    hash: block.hash.unwrap_or_default(),
                    parent_hash: block.parent_hash,
                    timestamp: block.timestamp.as_u64(),
        
                };
                Ok(Some(block_info))
            }
            None => Ok(None),
        }
    }

    async fn get_block_header(&self, block_number: u64) -> IndexerResult<Option<BlockHeader>> {
        let block = self.get_block(block_number).await?;
        
        match block {
            Some(block) => {
                let header = BlockHeader {
                    number: block.number.unwrap_or_default().as_u64(),
                    hash: block.hash.unwrap_or_default().into(),
                    parent_hash: block.parent_hash.into(),
                    timestamp: block.timestamp.as_u64(),
                };
                Ok(Some(header))
            }
            None => Ok(None),
        }
    }

    async fn get_logs(&self, from_block: u64, to_block: u64, contract_address: Option<H160>) -> IndexerResult<Vec<TransferEvent>> {
        let transfer_topic = H256::from_slice(
            &keccak256(b"Transfer(address,address,uint256)")
        );

        let mut filter = Filter::new()
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()))
            .topic0(transfer_topic);

        if let Some(addr) = contract_address.or(Some(self.contract_address)) {
            filter = filter.address(addr);
        }

        let mut attempts = 0usize;
        let max_attempts = 3usize;

        loop {
            attempts += 1;
            match self.provider.get_logs(&filter).await {
                Ok(logs) => {
                    info!(
                        "Retrieved {} transfer logs from blocks {} to {}",
                        logs.len(), from_block, to_block
                    );

                    let mut events = Vec::with_capacity(logs.len());
                    for log in logs.iter() {
                        if let Some(evt) =
                            TransferEvent::from_log(log.clone(), self.contract_address)
                        {
                            events.push(evt);
                        } else {
                            warn!("Failed to parse log into TransferEvent - invalid format");
                        }
                    }

                    info!("Parsed {} valid TransferEvents", events.len());
                    return Ok(events);
                }
                Err(e) => {
                    if attempts >= max_attempts {
                        warn!(
                            "Failed to get logs for blocks {}-{} after {} attempts: {}",
                            from_block, to_block, max_attempts, e
                        );
                        return Err(IndexerError::Ethereum(format!(
                            "Failed to get logs for blocks {}-{}: {}",
                            from_block, to_block, e
                        )));
                    }

                    warn!(
                        "RPC call failed for blocks {}-{} (attempt {}/{}): {}. Retrying...",
                        from_block, to_block, attempts, max_attempts, e
                    );
                    let delay_ms = 200u64.saturating_mul(1 << (attempts - 1));
                    sleep(Duration::from_millis(delay_ms.min(1000))).await;
                }
            }
        }
    }



    async fn get_transfer_logs_for_block(&self, block_number: u64) -> IndexerResult<Vec<Log>> {
        let transfer_topic = H256::from_slice(
            &hex::decode(crate::models::TRANSFER_EVENT_TOPIC)
                .map_err(|e| IndexerError::Config(format!("Invalid transfer topic hex: {}", e)))?
        );
        
        let filter = Filter::new()
            .from_block(BlockNumber::Number(block_number.into()))
            .to_block(BlockNumber::Number(block_number.into()))
            .address(self.contract_address)
            .topic0(transfer_topic);
        
        let logs = self.provider.get_logs(&filter).await?;
        debug!("Retrieved {} transfer logs for block {}", logs.len(), block_number);
        
        Ok(logs)
    }

    async fn scan_blocks(&self, start_block: u64, end_block: u64, _batch_size: u64) -> IndexerResult<Vec<TransferEvent>> {
        let mut all_events = Vec::new();
        let mut current_block = start_block;
        
       
        const ETH_GETLOGS_BLOCK_LIMIT: u64 = 300;
        
        while current_block <= end_block {
            let batch_end = std::cmp::min(current_block + ETH_GETLOGS_BLOCK_LIMIT.saturating_sub(1), end_block);
            
            info!("Scanning blocks {} to {} for transfer events (300 block limit)", current_block, batch_end);
            
            let events = self.get_logs(current_block, batch_end, None).await?;
            all_events.extend(events);
            
            current_block = batch_end + 1;
            
    
            if current_block <= end_block {
                sleep(Duration::from_millis(200)).await;
            }
        }
        
        info!("Total transfer events found: {}", all_events.len());
        Ok(all_events)
    }

    
    async fn subscribe_to_new_blocks(&self) -> IndexerResult<tokio::sync::mpsc::Receiver<u64>> {
        if let Some(ws_provider) = &self.ws_provider {
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            
    
            let ws_provider_clone = ws_provider.clone();
            
    
            tokio::spawn(async move {
        
                match ws_provider_clone.subscribe_blocks().await {
                    Ok(mut stream) => {
                        while let Some(block) = stream.next().await {
                            if let Some(block_number) = block.number {
                                if tx.send(block_number.as_u64()).await.is_err() {
                            
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to subscribe to blocks in background task: {}", e);
                    }
                }
            });
            
            Ok(rx)
        } else {
            Err(IndexerError::Config("WebSocket not enabled".to_string()))
        }
    }

    async fn verify_block_finality(&self, block_number: u64, confirmations: u64) -> IndexerResult<bool> {
        let latest_block = self.get_latest_block_number().await?;
        

        if latest_block < block_number + confirmations {
            return Ok(false);
        }
        

        if latest_block >= block_number + confirmations {
            return Ok(true);
        }
        
        Ok(false)
    }

    async fn get_contract_info(&self) -> IndexerResult<(String, String, u8)> {

        let address_str = format!("0x{:x}", self.contract_address);
        Ok((
            "ERC20 Token".to_string(),
            address_str,
            18,
        ))
    }

    async fn health_check(&self) -> IndexerResult<bool> {
        match self.provider.get_block_number().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn get_contract_address(&self) -> ethers::types::H160 {
        self.contract_address
    }
}

impl AdaptiveEthereumClient {
    pub async fn new(rpc_url: &str, contract_address: &str, enable_ws: bool) -> IndexerResult<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| IndexerError::Other(format!("Failed to create HTTP provider: {}", e)))?;
        
        let ws_provider = if enable_ws {
    
            let ws_url = if rpc_url.starts_with("https://") {
                rpc_url.replace("https://", "wss://")
            } else if rpc_url.starts_with("http://") {
                rpc_url.replace("http://", "ws://")
            } else if rpc_url.starts_with("ws") {
                rpc_url.to_string()
            } else {
                warn!("Unknown RPC URL format for WebSocket: {}", rpc_url);
                return Err(IndexerError::Config("Invalid RPC URL format for WebSocket".to_string()));
            };
            
            info!("Attempting WebSocket connection to: {}", ws_url);
            
            match Provider::<Ws>::connect(&ws_url).await {
                Ok(ws) => {
                    info!("WebSocket connection established successfully");
                    Some(Arc::new(ws))
                }
                Err(e) => {
                    warn!("Failed to establish WebSocket connection: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        let contract_address = contract_address.parse::<H160>()
            .map_err(|_| IndexerError::Config("Invalid contract address".to_string()))?;
        
        Ok(Self {
            provider: Arc::new(provider),
            ws_provider,
            contract_address,
        })
    }
    
    async fn get_block(&self, block_number: u64) -> IndexerResult<Option<Block<H256>>> {
        let block = self.provider.get_block(BlockNumber::Number(block_number.into())).await?;
        Ok(block)
    }
}
