use crate::client_trait::{EthereumClientTrait, BlockHeader};
use crate::models::{TransferEvent, BlockInfo};
use crate::error::IndexerResult;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{info, warn};


use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::eth::{Filter, Log as AlloyLog, Block, BlockTransactionsKind};
use alloy::primitives::{Address, B256, keccak256};
use alloy::rpc::types::BlockId;
use alloy::transports::http::Http;


use ethers::types::{Log, H160, H256, Bytes};
use ethers::providers::ProviderError as EthersProviderError;

pub struct AlloyEthereumClient {
    provider: RootProvider<Http<reqwest::Client>>,
    contract_address: Address,
    rpc_url: String,
}

impl AlloyEthereumClient {
    pub async fn new(rpc_url: String, contract_address: &str) -> IndexerResult<Self> {
        let url = reqwest::Url::parse(&rpc_url)
            .map_err(|e| crate::error::IndexerError::Provider(EthersProviderError::CustomError(format!("Invalid URL: {}", e))))?;
        
        let provider = ProviderBuilder::new().on_http(url);
        let contract_address = contract_address.parse::<Address>()
            .map_err(|e| crate::error::IndexerError::Provider(EthersProviderError::CustomError(format!("Invalid contract address: {}", e))))?;
        
        Ok(Self {
            provider,
            contract_address,
            rpc_url,
        })
    }

    fn convert_alloy_log_to_ethers_log(&self, alloy_log: &AlloyLog) -> Option<Log> {
        let topics: Vec<H256> = alloy_log.topics().iter()
            .map(|t| H256::from_slice(t.as_slice()))
            .collect();
        
        Some(Log {
            address: H160::from_slice(alloy_log.address().as_slice()),
            topics,
            data: Bytes::from(alloy_log.data().clone().data.0),
            block_hash: alloy_log.block_hash.map(|h| H256::from_slice(h.as_slice())),
            block_number: alloy_log.block_number.map(|n| n.into()),
            transaction_hash: alloy_log.transaction_hash.map(|h| H256::from_slice(h.as_slice())),
            transaction_index: alloy_log.transaction_index.map(|i| i.into()),
            log_index: alloy_log.log_index.map(|i| i.into()),
            transaction_log_index: alloy_log.log_index.map(|i| i.into()),
            log_type: None,
            removed: Some(false),
        })
    }

    fn convert_alloy_block_to_block_info(&self, block: &Block) -> Option<BlockInfo> {
        Some(BlockInfo {
            number: block.header.number?,
            hash: H256::from_slice(block.header.hash?.as_slice()),
            parent_hash: H256::from_slice(block.header.parent_hash.as_slice()),
            timestamp: block.header.timestamp,

        })
    }

    fn convert_alloy_block_to_header(&self, block: &Block) -> Option<BlockHeader> {
        Some(BlockHeader {
            number: block.header.number?,
            hash: block.header.hash?.into(),
            parent_hash: block.header.parent_hash.into(),
            timestamp: block.header.timestamp,
        })
    }

    fn convert_provider_error(&self, error: impl std::fmt::Display) -> crate::error::IndexerError {
        crate::error::IndexerError::Provider(EthersProviderError::CustomError(error.to_string()))
    }
}

#[async_trait]
impl EthereumClientTrait for AlloyEthereumClient {
    async fn get_latest_block_number(&self) -> IndexerResult<u64> {
        let block_number = self.provider.get_block_number().await
            .map_err(|e| self.convert_provider_error(e))?;
        Ok(block_number)
    }

    async fn get_block_with_logs(&self, block_number: u64) -> IndexerResult<Option<BlockInfo>> {
        let block = self.provider.get_block(BlockId::Number(block_number.into()), BlockTransactionsKind::Full).await
            .map_err(|e| self.convert_provider_error(e))?;
        
        let block = match block {
            Some(b) => b,
            None => return Ok(None),
        };


        let block_info = self.convert_alloy_block_to_block_info(&block)
            .ok_or_else(|| crate::error::IndexerError::Provider(EthersProviderError::CustomError("Failed to convert block".to_string())))?;
        
        Ok(Some(block_info))
    }

    async fn get_block_header(&self, block_number: u64) -> IndexerResult<Option<BlockHeader>> {

        let block = self.provider.get_block(BlockId::Number(block_number.into()), BlockTransactionsKind::Hashes).await
            .map_err(|e| self.convert_provider_error(e))?;
        
        let block = match block {
            Some(b) => b,
            None => return Ok(None),
        };


        let header = self.convert_alloy_block_to_header(&block)
            .ok_or_else(|| crate::error::IndexerError::Provider(EthersProviderError::CustomError("Failed to convert block header".to_string())))?;
        
        Ok(Some(header))
    }

    async fn get_logs(&self, from_block: u64, to_block: u64, contract_address: Option<H160>) -> IndexerResult<Vec<TransferEvent>> {
        let transfer_topic: B256 = keccak256(b"Transfer(address,address,uint256)").into();

        let mut filter = Filter::new()
            .event_signature(transfer_topic)
            .from_block(from_block)
            .to_block(to_block);

        if let Some(addr) = contract_address {
            filter = filter.address(Address::from_slice(addr.0.as_slice()));
        } else {
            filter = filter.address(self.contract_address);
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
                        if let Some(ethers_log) = self.convert_alloy_log_to_ethers_log(log) {
                            if let Some(evt) = TransferEvent::from_log(ethers_log, H160::from_slice(self.contract_address.as_slice())) {
                                events.push(evt);
                            } else {
                                warn!("Failed to parse log into TransferEvent - invalid format");
                            }
                        } else {
                            warn!("Failed to convert alloy log to ethers log");
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
                        return Err(crate::error::IndexerError::Ethereum(format!(
                            "Failed to get logs for blocks {}-{}: {}",
                            from_block, to_block, e
                        )));
                    }

                    warn!(
                        "RPC call failed for blocks {}-{} (attempt {}/{}): {}. Retrying...",
                        from_block, to_block, attempts, max_attempts, e
                    );
                    let delay_ms = 200u64.saturating_mul(1 << (attempts - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms.min(1000))).await;
                }
            }
        }
    }

    async fn get_transfer_logs_for_block(&self, block_number: u64) -> IndexerResult<Vec<Log>> {
        let transfer_topic: B256 = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
            .parse()
            .map_err(|e| crate::error::IndexerError::Provider(EthersProviderError::CustomError(format!("Invalid transfer topic: {}", e))))?;


        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(transfer_topic)
            .from_block(block_number)
            .to_block(block_number);

        let logs: Vec<AlloyLog> = self.provider.get_logs(&filter).await
            .map_err(|e| self.convert_provider_error(e))?;


        let mut ethers_logs = Vec::new();
        for log in logs {
            if let Some(ethers_log) = self.convert_alloy_log_to_ethers_log(&log) {
                ethers_logs.push(ethers_log);
            }
        }

        Ok(ethers_logs)
    }

    async fn scan_blocks(&self, start_block: u64, end_block: u64, batch_size: u64) -> IndexerResult<Vec<TransferEvent>> {
        let mut all_events = Vec::new();
        let mut current_block = start_block;

        while current_block <= end_block {
            let batch_end = std::cmp::min(current_block + batch_size.saturating_sub(1), end_block);
            
            let batch_events = self.get_logs(current_block, batch_end, None).await?;
            all_events.extend(batch_events);
            
            current_block = batch_end + 1;
        }

        Ok(all_events)
    }

    async fn subscribe_to_new_blocks(&self) -> IndexerResult<mpsc::Receiver<u64>> {

        let (tx, rx) = mpsc::channel(100);
        drop(tx);
        Ok(rx)
    }

    async fn verify_block_finality(&self, block_number: u64, confirmations: u64) -> IndexerResult<bool> {
        let latest_block = self.get_latest_block_number().await?;
        let required_confirmations = latest_block.saturating_sub(block_number);
        Ok(required_confirmations >= confirmations)
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
        match self.get_latest_block_number().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn get_contract_address(&self) -> H160 {
        H160::from_slice(self.contract_address.as_slice())
    }
}
