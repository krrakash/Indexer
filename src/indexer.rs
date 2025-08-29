use crate::{
    config::Config,
    database::{Database, DatabaseWriteQueue},
    ethereum::EthereumClient,
    models::TransferEvent,
    error::{IndexerError, IndexerResult},
};
use crate::adaptive_ethers::AdaptiveEthereumClient;
use crate::alloy_client::AlloyEthereumClient;
use crate::client_trait::EthereumClientTrait;
use tokio::time::{sleep, Duration};
use tokio::select;
use tracing::{info, warn, error, debug};
use futures::{StreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use hex;
use tokio::sync::mpsc::{self, Receiver, Sender};
use std::collections::HashMap;
use tokio::sync::Mutex;
use crate::client_trait::BlockHeader;
use crate::models::TransferRecord;
use std::cmp::min;

#[derive(Debug, Clone)]
enum ReorgRecoveryType {
    Live,
    WebSocket,
}

#[derive(Debug, Clone)]
struct BlockData {
    block_number: u64,
    block_header: BlockHeader,
    events: Vec<TransferRecord>,
}

pub struct Indexer {
    config: Config,
    database: Database,
    write_queue: DatabaseWriteQueue,
    ethereum_client: Arc<dyn EthereumClientTrait + Send + Sync>,
    is_running: bool,

    websocket_handle: Option<tokio::task::JoinHandle<IndexerResult<()>>>,
    websocket_receiver: Option<Receiver<BlockData>>,
    websocket_buffer: Option<Arc<Mutex<HashMap<u64, BlockData>>>>,
}

impl Indexer {
    pub async fn new(config: Config) -> IndexerResult<Self> {
        config.validate().map_err(|e| IndexerError::Config(e))?;
        let database = Database::new(&config.database_path())?;

        let ethereum_client: Arc<dyn EthereumClientTrait + Send + Sync> = if config.use_alloy {
            info!("Initializing Alloy Ethereum client");
            let client = AlloyEthereumClient::new(config.rpc_url.clone(), &config.contract_address).await?;
            Arc::new(client)
        } else if config.use_adaptive_ethers {
            info!("Initializing Adaptive ethers-rs Ethereum client");
            let client = AdaptiveEthereumClient::new(&config.rpc_url, &config.contract_address, config.websocket).await?;
            Arc::new(client)
        } else {
            info!("Initializing ethers-rs Ethereum client");
            let client = EthereumClient::new(&config.rpc_url, &config.contract_address, config.websocket).await?;
            Arc::new(client)
        };


        match ethereum_client.get_contract_info().await {
            Ok((name, symbol, decimals)) => {
                info!("Connected to ERC-20 contract: {} ({}) with {} decimals", name, symbol, decimals);
            }
            Err(e) => {
                warn!("Failed to get contract info: {}", e);
            }
        }




        let write_queue = database.create_write_queue();
        info!("Database write queue created");

        Ok(Self {
            config,
            database,
            write_queue,
            ethereum_client,
            is_running: false,

            websocket_handle: None,
            websocket_receiver: None,
            websocket_buffer: None,
        })
    }
    
    pub async fn start(&mut self) -> IndexerResult<()> {
        if self.is_running {
            return Err(IndexerError::Config("Service already running".to_string()));
        }
        
        self.is_running = true;
        info!("Starting indexer");
        

        let head = self.ethereum_client.get_latest_block_number().await?;
        
       
        let finality_depth = self.config.finality_depth;
        let mut safe_tip = head.saturating_sub(finality_depth);
        
        info!("Head={}, Safe Tip={}, Finality Depth={} ({} mode)", head, safe_tip, finality_depth, if self.config.websocket { "WebSocket" } else { "Polling" });
        


        
        let progress = self.database.get_progress()?;
        info!(" progress.last_processed_block = {}", progress.last_processed_block);
        info!(" self.config.start_block = {}", self.config.start_block);
        info!("safe_tip = {}", safe_tip);
        
        let mut start_block = if progress.last_processed_block > 0 {
           
            info!("Resuming from block {}", progress.last_processed_block);
            progress.last_processed_block + 1
        } else if self.config.start_block > 0 {

            info!("Using start block {}", self.config.start_block);
            self.config.start_block
        } else {

            info!("Fresh start from safe tip {}", safe_tip);
            safe_tip
        };
        
        let mut target_block = if self.config.end_block > 0 {
            self.config.end_block
        } else {
            safe_tip
        };
        
        info!("Starting from block {}, targeting block {}", start_block, target_block);
        debug!("start_block={}, target_block={}, start_block < target_block = {}", start_block, target_block, start_block < target_block);
        
       
        if progress.last_processed_block > safe_tip {
            info!(
                "Progress ({}) is ahead of safe tip ({}). Waiting for fence to catch up…",
                progress.last_processed_block, safe_tip
            );
            loop {
                sleep(Duration::from_secs(2)).await;
                let head = self.ethereum_client.get_latest_block_number().await?;
                safe_tip = head.saturating_sub(finality_depth);
                if safe_tip >= progress.last_processed_block {
                    info!("Fence caught up (safe_tip = {}). Continuing.", safe_tip);
                    break;
                }
            }
            let progress = self.database.get_progress()?;
            start_block = progress.last_processed_block + 1;
            target_block = safe_tip;
        }

        if start_block < target_block {

            info!("Running historical indexing from {} to {}", start_block, target_block);
            self.transition_handoff(start_block, target_block).await?;
        } else {

            info!("Already at safe tip, starting live mode");
            if self.config.websocket {
                self.start_realtime_indexing().await?;
            } else {
                self.start_polling_indexing().await?;
            }
        }
        
        Ok(())
    }


    
        
    async fn transition_handoff(&mut self, start_block: u64, safe_tip: u64) -> IndexerResult<()> {
        let mut current_start_block = start_block;
        let mut current_safe_tip = safe_tip;
        
        loop {
            info!("Starting indexing from {} to {}", current_start_block, current_safe_tip);
            
            
            info!("Running historical indexing to safe tip {}", current_safe_tip);
            self.index_historical_blocks(current_start_block, current_safe_tip).await?;
            
           
            if self.config.websocket && self.config.end_block == 0 {
                info!("Historical indexing completed, starting WebSocket mode");
                
                
                let (websocket_sender, websocket_receiver) = mpsc::channel::<BlockData>(100);
                let websocket_buffer = Arc::new(Mutex::new(HashMap::new()));
                
                self.websocket_receiver = Some(websocket_receiver);
                self.websocket_buffer = Some(websocket_buffer.clone());
                
                let ethereum_client = self.ethereum_client.clone();
                let handle = tokio::spawn(async move {
                    Self::websocket_buffering_task(ethereum_client, websocket_buffer, websocket_sender, "buffering task").await
                });
                
               
                sleep(Duration::from_secs(2)).await;
                
               
                info!("Detecting and filling gaps before WebSocket processing");
                self.gap_filler().await?;
                info!("Gap filling completed successfully");
                
                
                info!("Starting live WebSocket processing");
                self.start_live_sequential_processing_handoff().await?;
                
                break;
            } else if !self.config.websocket && self.config.end_block == 0 {
                info!("Historical indexing completed, starting polling mode");
                self.start_polling_indexing().await?;
                break;
            } else {
                info!("Historical indexing completed, no live mode requested");
                break;
            }
        }
        
        Ok(())
    }
    
    
    async fn gap_filler(&mut self) -> IndexerResult<()> {
        info!("Starting gap detection before WebSocket processing");
        

        let progress = self.database.get_progress()?;
        let historical_end = progress.last_processed_block;
        
        info!("Historical indexing completed at block {}", historical_end);
        debug!("WebSocket buffer is running but processing will not start until gaps are filled");
        

        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 30; 
        
        loop {
            let websocket_start = {
                if let Some(websocket_buffer) = &self.websocket_buffer {
                    let buffer_guard = websocket_buffer.lock().await;
                    let pending_blocks: Vec<u64> = buffer_guard.keys().cloned().collect();
                    
                    if !pending_blocks.is_empty() {
                        Some(*pending_blocks.iter().min().unwrap())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            
                        if let Some(websocket_start) = websocket_start {
                
                let gap_start = historical_end + 1;
                let gap_end = websocket_start.saturating_sub(1);
                
                info!("Gap: {} to {} ({} blocks)", gap_start, gap_end, gap_end - gap_start + 1);
                
                
                if gap_start <= gap_end {
                    let gap_size = gap_end - gap_start + 1;
                    if gap_size > 1 {
                        let mut missing_blocks = Vec::new();
                        for block_number in gap_start..=gap_end {
                            missing_blocks.push(block_number);
                        }
                        
                        warn!("Significant gaps detected: {} blocks", missing_blocks.len());
                        info!("Filling gaps using HTTP backfill");
                    } else {
                        info!("Expected 1-block gap during historical to live transition, skipping gap fill");
                        break;
                    }
                    
                    info!("Fetching gap events for blocks {} to {}", gap_start, gap_end);
                    let gap_events = self.ethereum_client.scan_blocks(gap_start, gap_end, gap_end - gap_start + 1).await?;
                    
            
                    let mut events_processed = 0;
                    let mut new_events = Vec::new();
                    
                    for event in &gap_events {
                        let record = event.to_record();
                        
                        
                        if !self.database.transfer_exists(&record.transaction_hash, record.log_index)? {
                            new_events.push(record);
                            events_processed += 1;
                        }
                    }
                    
            
                    if !new_events.is_empty() {
                        self.database.store_transfers_batch(&new_events)?;
                        info!("Stored {} gap events", events_processed);
                    }
                    
            
                    info!("Fetching and storing gap blocks in batch");
                    let mut block_headers = Vec::new();
                    for block_number in gap_start..=gap_end {
                        if let Ok(Some(block_header)) = self.ethereum_client.get_block_header(block_number).await {
                            block_headers.push(block_header);
                        } else {
                            warn!("Failed to get block header for gap block {}", block_number);
                        }
                    }
                    
                    if !block_headers.is_empty() {
                        self.database.store_block_headers_batch(&block_headers)?;
                        info!("Stored {} gap block headers in batch", block_headers.len());
                    }
                    
                    info!("Gap filling completed for {} blocks", gap_size);
                    
            
                    debug!("Verifying all gap blocks are stored in blocks table");
                    for block_number in gap_start..=gap_end {
                        match self.database.get_block_header(block_number) {
                            Ok(Some(_)) => {
                                debug!("Verified gap block {} is stored in blocks", block_number);
                            }
                            Ok(None) => {
                                error!("Gap block {} not found in blocks table", block_number);
                                return Err(IndexerError::Other(format!("Gap block {} not stored in blocks table", block_number)));
                            }
                            Err(e) => {
                                error!("Error verifying gap block {}: {}", block_number, e);
                                return Err(IndexerError::Other(format!("Error verifying gap block {}: {}", block_number, e)));
                            }
                        }
                    }
                    debug!("All {} gap blocks verified and stored in blocks table", gap_size);
                    info!("Updating progress after gap filling");
                    let progress = self.database.get_progress()?;
                    self.write_queue.update_progress(gap_end, progress.total_events_processed + events_processed)?;
                    info!("Progress updated: last_processed_block = {}, total_events = {}", gap_end, progress.total_events_processed + events_processed);
                    
                    // Now verify continuity after gaps are filled
                    debug!("Checking continuity at boundary: historical block {} - gap block {}", historical_end, gap_start);
                    let continuity_ok = self.verify_realtime_continuity(gap_start).await?;
                    if !continuity_ok {
                        warn!("Continuity broken at gap start boundary, triggering WebSocket reorg recovery");
                        let recovery_block = self.handle_reorg_recovery(gap_start, ReorgRecoveryType::WebSocket).await?;
                        info!("WebSocket reorg recovery completed at block {}, continuing", recovery_block);
                        return Ok(());
                    }
                    
                    debug!("Checking continuity at boundary: gap block {} and WebSocket block {}", gap_end, websocket_start);
                    let continuity_ok = self.verify_realtime_continuity(websocket_start).await?;
                    if !continuity_ok {
                        warn!("Continuity broken at gap end boundary, triggering WebSocket reorg recovery");
                        let recovery_block = self.handle_reorg_recovery(websocket_start, ReorgRecoveryType::WebSocket).await?;
                        info!("WebSocket reorg recovery completed at block {}, continuing", recovery_block);
                        return Ok(());
                    }
                         
                    debug!("Continuity verified at gap boundaries");
                } else {
                    info!("No gaps detected - WebSocket start ({}) immediately follows historical end ({})", websocket_start, historical_end);
                }
                
                break;
            }
            
            retry_count += 1;
            if retry_count >= MAX_RETRIES {
                warn!("No WebSocket blocks received after {} retries, proceeding without gap detection", MAX_RETRIES);
                break;
            }
            
            debug!("Waiting for first WebSocket block (attempt {}/{}), sleeping 1 second...", retry_count, MAX_RETRIES);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        

        debug!("Final verification - ensuring WebSocket buffer is ready for processing");
        
        if let Some(websocket_buffer) = &self.websocket_buffer {
            let buffer_guard = websocket_buffer.lock().await;
            let pending_blocks: Vec<u64> = buffer_guard.keys().cloned().collect();
            
            if !pending_blocks.is_empty() {
                let min_block = *pending_blocks.iter().min().unwrap();
                let max_block = *pending_blocks.iter().max().unwrap();
                info!("WebSocket buffer ready with {} blocks ({} to {})", pending_blocks.len(), min_block, max_block);
            } else {
                info!("WebSocket buffer is empty");
            }
        }
        
        info!("Gap detection and filling completed");
        Ok(())
    }
    

    async fn verify_continuity_for_block(
        ethereum_client: &Arc<dyn EthereumClientTrait + Send + Sync>,
        database: &Database,
        block_number: u64,
    ) -> IndexerResult<bool> {
                    debug!("Checking continuity for block {}", block_number);
        

        let block_header = match ethereum_client.get_block_header(block_number).await? {
            Some(header) => {
                info!("  Retrieved block {} header from Ethereum", block_number);
                info!("  Block {} hash: 0x{}", block_number, hex::encode(header.hash));
                info!("  Block {} parent_hash: 0x{}", block_number, hex::encode(header.parent_hash));
                header
            },
            None => {
                error!("  Block {} not found on Ethereum", block_number);
                return Ok(false);
            }
        };
        

        let prev_block_number = block_number.saturating_sub(1);
        info!("  Looking for previous block {} in database", prev_block_number);
        
        let prev_block_header = database.get_block_header(prev_block_number)?;
        
        match prev_block_header {
            Some(prev) => {
                info!("  Found previous block {} in database", prev_block_number);
                info!("  Previous block hash: 0x{}", hex::encode(prev.hash));
                info!("  Current block parent_hash: 0x{}", hex::encode(block_header.parent_hash));
                info!("  Hash comparison: {} == {}", hex::encode(prev.hash), hex::encode(block_header.parent_hash));
                
                if block_header.parent_hash == prev.hash {
                    info!("  SUCCESS - Block {} parent_hash matches block {} hash", block_number, prev_block_number);
                    Ok(true)
                } else {
                    warn!(" FAILURE - Block {} parent_hash MISMATCH block {} hash", block_number, prev_block_number);
                    warn!(" Expected: 0x{}", hex::encode(prev.hash));
                    warn!(" Got: 0x{}", hex::encode(block_header.parent_hash));
                    Ok(false)
                }
            }
            None => {
                if block_number == 0 {
                    info!("  Block 0: skipping continuity check");
                    Ok(true)
                } else {
                    warn!("  Prev block {} missing in DB; continuity fails", prev_block_number);
                    Ok(false)
                }
            }
        }
    }
    
    
    async fn process_block_data(
        database: &Database,
        write_queue: &DatabaseWriteQueue,
        block_data: BlockData,
    ) -> IndexerResult<()> {
        
        if !block_data.events.is_empty() {
            database.store_transfers_batch(&block_data.events)?;
            info!("Stored {} events for block {} in transfers table", block_data.events.len(), block_data.block_number);
        }
        
        
        database.store_block_header_simple(&block_data.block_header)?;
                    info!("Stored block {} in blocks table", block_data.block_number);
        
        Ok(())
    }
    

    async fn start_websocket_buffering(&mut self, fence_start: u64) -> IndexerResult<tokio::task::JoinHandle<IndexerResult<()>>> {
        info!("WEBSOCKET BUFFERING: Starting WebSocket buffering from fence {}", fence_start);

        let ethereum_client = self.ethereum_client.clone();

        
        let (websocket_sender, websocket_receiver) = mpsc::channel::<BlockData>(100);
        let websocket_buffer = Arc::new(Mutex::new(HashMap::new()));


        self.websocket_receiver = Some(websocket_receiver);
        self.websocket_buffer = Some(websocket_buffer.clone());

        let handle = tokio::spawn(async move {
            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 3;

            loop {
                match ethereum_client.subscribe_to_new_blocks().await {
                    Ok(mut block_stream) => {
                        info!(" WEBSOCKET BUFFERING: WebSocket connection established");
                        retry_count = 0; 

                        while let Some(block_number) = block_stream.recv().await {
                            if block_number >= fence_start {
                                info!("WEBSOCKET BUFFERING: Buffering block {} (fence_start: {})", block_number, fence_start);

                                
                                if let Ok(Some(block_header)) = ethereum_client.get_block_header(block_number).await {
                                    let logs = ethereum_client.get_transfer_logs_for_block(block_number).await?;

                                
                                    let mut block_events = Vec::new();
                                    for log in logs {
                                            if let Some(transfer_event) = TransferEvent::from_log(log, ethereum_client.get_contract_address()) {
                                                block_events.push(transfer_event.to_record());
                                            }
                                        }

                                    let events_count = block_events.len();
                                    let block_data = BlockData {
                                        block_number,
                                        block_header,
                                        events: block_events,
                                    };

                                    
                                    {
                                        let mut buffer_guard = websocket_buffer.lock().await;
                                        buffer_guard.insert(block_number, block_data.clone());
                                    }

                                    
                                    if let Err(e) = websocket_sender.send(block_data).await {
                                        error!(" WEBSOCKET BUFFERING: Failed to send block data: {}", e);
                                    } else {
                                        info!(" WEBSOCKET BUFFERING: Sent block {} with {} events via channel", block_number, events_count);
                                    }
                                }
                            } else {
                                debug!(" WEBSOCKET BUFFERING: Ignoring block {} (below fence_start: {})", block_number, fence_start);
                            }
                        }
                    }
                    Err(e) => {
                        retry_count += 1;
                        error!(" WEBSOCKET BUFFERING: WebSocket connection failed (attempt {}/{}): {}", retry_count, MAX_RETRIES, e);

                        if retry_count >= MAX_RETRIES {
                            error!(" WEBSOCKET BUFFERING: Max retries exceeded, failing fast");
                            return Err(e); 
                        }

                        
                        let delay = std::time::Duration::from_secs(2_u64.pow(retry_count));
                        info!(" WEBSOCKET BUFFERING: Retrying in {:?}", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        });

        Ok(handle)
    }
    
    
    async fn index_historical_blocks(&mut self, start_block: u64, initial_safe_tip: u64) -> IndexerResult<()> {
        info!("Starting historical indexing with moving target from {} to {}", start_block, initial_safe_tip);
        
        let batch_size = self.config.batch_size;
        let max_concurrent_batches = 15;
        
        let mut current_block = start_block;
        let mut total_events = 0;
        let mut batch_count = 0;
        let mut total_batch_time = std::time::Duration::ZERO;
        
        loop {
    
                        let head = self.ethereum_client.get_latest_block_number().await?;
            
            
            let finality_depth = if self.config.websocket { self.config.finality_depth } else { 0 };
            let safe_tip = head.saturating_sub(finality_depth);
            
            debug!("Head={}, Safe Tip={}, Current Block={}", head, safe_tip, current_block);
            
    
            let latest_block = if self.config.end_block > 0 {
                self.config.end_block  
            } else {
                safe_tip
            };
            
            
            if current_block > latest_block {
                info!("Reached target! Current block {} > latest block {}", current_block, latest_block);
                break;
            }
            
            let batch_start_time = std::time::Instant::now();
            
    
            let spawn_boundaries = self.calculate_spawn_boundaries(
                current_block, 
                latest_block, 
                max_concurrent_batches, 
                batch_size
            );
            
            
            let batch_start = spawn_boundaries.first().map(|(start, _)| *start).unwrap_or(current_block);
            let batch_end = spawn_boundaries.last().map(|(_, end)| *end).unwrap_or(current_block);
            
            
            if let Ok(Some(start_header)) = self.ethereum_client.get_block_header(batch_start).await {
                if let Err(e) = self.database.store_block_header(&start_header, Some(batch_start), Some(batch_end)) {
                    error!("Failed to store batch start header for block {}: {}", batch_start, e);
                }
            }
            
            if batch_end != batch_start {
                if let Ok(Some(end_header)) = self.ethereum_client.get_block_header(batch_end).await {
                    if let Err(e) = self.database.store_block_header(&end_header, Some(batch_start), Some(batch_end)) {
                        error!("Failed to store batch end header for block {}: {}", batch_end, e);
                    }
                }
            }
            
            info!("Processing batch {}-{} (target: {})", batch_start, batch_end, latest_block);
            
            
            let mut futures = FuturesUnordered::new();
            
            for (spawn_start, spawn_end) in spawn_boundaries.clone() {
                let ethereum_client = self.ethereum_client.clone();
                let database = self.database.clone();
                let write_queue = self.write_queue.clone();
                
                futures.push(tokio::spawn(async move {
                    process_block_batch(&ethereum_client, &database, &write_queue, spawn_start, spawn_end).await
                }));
            }
            
            
            let mut batch_results = Vec::new();
            while let Some(result) = futures.next().await {
                match result {
                    Ok(Ok((events_processed, blocks_processed))) => {
                        total_events += events_processed;
                        batch_results.push((events_processed, blocks_processed));
                    }
                    Ok(Err(e)) => {
                        error!("Batch processing failed: {}", e);
                    }
                    Err(e) => {
                        error!("Task join failed: {}", e);
                    }
                }
            }
            
           
            let batch_duration = batch_start_time.elapsed();
            total_batch_time += batch_duration;
            batch_count += 1;
            
            let blocks_processed_this_batch: u64 = batch_results.iter().map(|(_, blocks)| blocks).sum();
            let events_processed_this_batch: u64 = batch_results.iter().map(|(events, _)| events).sum();
            
            info!("Batch {}: {} events, {} blocks in {:.2?}", batch_count, events_processed_this_batch, blocks_processed_this_batch, batch_duration);
            
            
            current_block += blocks_processed_this_batch;
            
            
            if let Some((_, _blocks_processed)) = batch_results.last() {
            
                self.write_queue.update_progress(current_block.saturating_sub(1), total_events)?;
            }
            
           
            sleep(Duration::from_millis(100)).await;
        }
        
        info!("Historical indexing completed at block {}. Total events: {}", current_block.saturating_sub(1), total_events);
        

        if current_block.saturating_sub(1) == start_block.saturating_sub(1) {
            let end_block = if self.config.end_block > 0 {
                self.config.end_block
            } else {
                self.ethereum_client.get_latest_block_number().await?
            };
            info!("No blocks processed, updating progress to end block {}", end_block);
            self.write_queue.update_progress(end_block, total_events)?;
        }
        
        Ok(())
    }
    
    
    
    
    
    async fn start_realtime_indexing(&mut self) -> IndexerResult<()> {
        info!("Starting real-time indexing with buffered WebSocket and gap handling");
        
        let progress = self.database.get_progress()?;
        let last_processed_block = progress.last_processed_block;
        
        info!("Transitioning from historical (block {}) to real-time indexing", last_processed_block);
        
        let ethereum_client = self.ethereum_client.clone();
        
        let (websocket_sender, websocket_receiver) = mpsc::channel::<BlockData>(100);
        let websocket_buffer = Arc::new(Mutex::new(HashMap::new()));
        
        self.websocket_receiver = Some(websocket_receiver);
        self.websocket_buffer = Some(websocket_buffer.clone());
        
        
        let ethereum_client_clone = ethereum_client.clone();
        let handle = tokio::spawn(async move {
            Self::websocket_buffering_task(ethereum_client_clone, websocket_buffer, websocket_sender, "buffering task with gap handling").await
        });
        
        
        sleep(Duration::from_secs(2)).await;
        
        
        self.start_buffered_realtime_processing().await?;
        
        Ok(())
    }
    

    fn calculate_spawn_boundaries(
        &self,
        start_block: u64,
        end_block: u64,
        max_spawns: usize,
        batch_size: u64,
    ) -> Vec<(u64, u64)> {
        let mut boundaries = Vec::new();
        let mut current = start_block;
        
        for _ in 0..max_spawns {
            if current > end_block {
                break;
            }
            
            let spawn_end = std::cmp::min(current + batch_size.saturating_sub(1), end_block);
            boundaries.push((current, spawn_end));
            current = spawn_end + 1;
        }
        
        boundaries
    }

   

    async fn process_gap_blocks(
        &self,
        gap_start: u64,
        gap_end: u64,
    ) -> IndexerResult<u64> {
        info!("Processing gap blocks {} to {} ({} blocks)", gap_start, gap_end, gap_end - gap_start + 1);
        
        let gap_events = self.ethereum_client.scan_blocks(gap_start, gap_end, gap_end - gap_start + 1).await?;
        
        let mut events_processed = 0;
        let mut new_events = Vec::new();
        
        for event in &gap_events {
            let record = event.to_record();
            
            if let Ok(Some(_)) = self.database.get_transfer_by_tx_and_log(
                &record.transaction_hash,
                record.log_index
            ) {
                debug!("Duplicate event found: tx={}, log_index={}", 
                       record.transaction_hash, record.log_index);
                continue;
            }
            
            new_events.push(record);
            events_processed += 1;
        }
        
        if !new_events.is_empty() {
            info!("Storing {} gap events in batch", new_events.len());
            queue_store_batch(&self.write_queue, new_events)?;
        }
        
        
        let mut block_headers = Vec::new();
        for gap_block in gap_start..=gap_end {
            if let Ok(Some(block_header)) = self.ethereum_client.get_block_header(gap_block).await {
                block_headers.push(block_header);
            }
        }
        
        if !block_headers.is_empty() {
            self.database.store_block_headers_batch(&block_headers)?;
        }
        
        let progress = self.database.get_progress()?;
        let new_total_events = progress.total_events_processed + events_processed;
        self.database.update_progress(gap_end, new_total_events)?;
        
        info!("Gap filled! Processed {} events for {} blocks", events_processed, gap_end - gap_start + 1);
        Ok(events_processed)
    }

    async fn verify_realtime_continuity(&self, block_number: u64) -> IndexerResult<bool> {
        debug!("Checking continuity for block {}", block_number);
        
       
        let block_header = match self.ethereum_client.get_block_header(block_number).await? {
            Some(block_header) => {
                debug!("Retrieved block {} header from Ethereum", block_number);
                block_header
            },
            None => {
                error!("Block {} not found on Ethereum", block_number);
                return Ok(false);
            }
        };
        
      
        debug!("Looking for previous block {} in database", block_number.saturating_sub(1));
        let prev_block_header = match self.database.get_block_header(block_number.saturating_sub(1)) {
            Ok(h) => h,
            Err(e) => {
                error!("Error getting previous block {} from blocks: {}", block_number.saturating_sub(1), e);
                None
            }
        };
        
        match prev_block_header {
            Some(prev) => {
                        debug!("Retrieved previous block {} header from database", block_number.saturating_sub(1));
                
                
                        debug!("  Block {} → Block {}", block_number.saturating_sub(1), block_number);
                
                if block_header.parent_hash == prev.hash {
                    debug!("Continuity check passed for block {}", block_number);
                    debug!("Continuity verified for block {}", block_number);
                    Ok(true) 
                } else {
                    warn!("Block {} parent_hash mismatch block {} hash", block_number, block_number.saturating_sub(1));
                    warn!("Expected: 0x{}", hex::encode(prev.hash));
                    warn!("Got: 0x{}", hex::encode(block_header.parent_hash));
                    warn!("Continuity broken at block {}: parent_hash mismatch", block_number);
                    warn!("Expected parent: 0x{}", hex::encode(prev.hash));
                    warn!("Got parent: 0x{}", hex::encode(block_header.parent_hash));
                    Ok(false) 
                }
            }
            None => {
                warn!(
                    "Previous block {} missing in DB for real-time block {} -> treating as gap",
                    block_number.saturating_sub(1), block_number
                );
                Ok(false)
            }
        }
    }

    async fn verify_continuity_and_recover(
        &mut self,
        block_number: u64,
        context: &str,
    ) -> IndexerResult<bool> {
        let continuity_ok = self.verify_realtime_continuity(block_number).await?;
        if !continuity_ok {
            warn!("Continuity broken at {} boundary, triggering reorg recovery", context);
            let recovery_block = self.handle_live_reorg_recovery(block_number).await?;
            info!("Reorg recovery completed at block {}, continuing", recovery_block);
            return Ok(false);
        }
        info!("Continuity verified at {} boundary", context);
        Ok(true)
    }

    async fn handle_live_reorg_recovery(&mut self, block_number: u64) -> IndexerResult<u64> {
        info!("Starting reorg recovery from block {}", block_number);
        
        
        info!("Linear backtracking");
        let linear_recovery = self. linear_backtrack(block_number).await?;
        
        if linear_recovery.is_some() {
            let good_block = linear_recovery.unwrap();
            info!("Linear backtracking found fork point at block {}", good_block);
            return self.rollback_to_block(good_block).await;
        }
        
        
        info!("Binary search on blocks");
        let live_recovery = self.binary_search_blocks(block_number).await?;
        
        if live_recovery.is_some() {
            let good_block = live_recovery.unwrap();
            info!(" Binary search on blocks found fork point at block {}", good_block);
            return self.rollback_to_block(good_block).await;
        }
        
       
        warn!(" All recovery strategies failed, rolling back to genesis");
        return self.rollback_to_block(0).await;
    }
    
   
    
    async fn handle_reorg_recovery(
        &mut self,
        block_number: u64,
        recovery_type: ReorgRecoveryType,
    ) -> IndexerResult<u64> {
        match recovery_type {
            ReorgRecoveryType::Live => {
                info!("Starting live reorg recovery from block {}", block_number);
                self.handle_live_reorg_recovery(block_number).await
            }
            ReorgRecoveryType::WebSocket => {
                info!("Starting comprehensive WebSocket recovery from block {}", block_number);
                
                info!("Clearing WebSocket buffer and channel of invalid blocks");
                self.clear_websocket_buffer().await?;
                self.clear_websocket_channel().await?;
                
                info!("Finding and rolling back to last good block");
                let last_good_block = self.handle_live_reorg_recovery(block_number).await?;
                
                info!("Waiting for first new WebSocket block after reorg");
                let first_websocket_block = self.wait_for_first_websocket_block_after_reorg().await?;
                
                info!("Detecting gap between {} and {}", last_good_block, first_websocket_block);
                self.detect_and_fill_gap_after_reorg(last_good_block, first_websocket_block).await?;
                
                info!("Resuming WebSocket processing");
                info!("Recovery completed successfully at block {}", last_good_block);
                
                Ok(last_good_block)
            }
        }
    }
    
    
    async fn clear_websocket_buffer(&mut self) -> IndexerResult<()> {
        info!(" Clearing WebSocket buffer of invalid blocks");
        
        if let Some(websocket_buffer) = &self.websocket_buffer {
            let mut buffer_guard = websocket_buffer.lock().await;
            let cleared_count = buffer_guard.len();
            buffer_guard.clear();
            info!(" Cleared {} invalid blocks from WebSocket buffer", cleared_count);
        } else {
            info!("ℹ No WebSocket buffer to clear");
        }
        
        Ok(())
    }

    async fn clear_websocket_channel(&mut self) -> IndexerResult<()> {
        info!(" Clearing WebSocket channel of invalid blocks");
        
        if let Some(receiver) = &mut self.websocket_receiver {
            let mut cleared_count = 0;
            while let Ok(_) = receiver.try_recv() {
                cleared_count += 1;
            }
            info!(" Cleared {} invalid blocks from WebSocket channel", cleared_count);
        } else {
            info!("ℹ No WebSocket receiver to clear");
        }
        
        Ok(())
    }
    

    async fn wait_for_first_websocket_block_after_reorg(&mut self) -> IndexerResult<u64> {
        info!(" Waiting for first new WebSocket block after reorg");
        
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 30; 
        
        loop {
            let websocket_start = {
                if let Some(websocket_buffer) = &self.websocket_buffer {
                    let buffer_guard = websocket_buffer.lock().await;
                    let pending_blocks: Vec<u64> = buffer_guard.keys().cloned().collect();
                    
                    if !pending_blocks.is_empty() {
                        Some(*pending_blocks.iter().min().unwrap())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            
            if let Some(websocket_start) = websocket_start {
                info!(" First new WebSocket block received: {}", websocket_start);
                return Ok(websocket_start);
            }
            
            retry_count += 1;
            if retry_count >= MAX_RETRIES {
                warn!(" No WebSocket blocks received after {} retries, using current head", MAX_RETRIES);
                let head = self.ethereum_client.get_latest_block_number().await?;
                return Ok(head);
            }
            
            info!(" Waiting for first WebSocket block (attempt {}/{}), sleeping 1 second...", retry_count, MAX_RETRIES);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    
   
    async fn detect_and_fill_gap_after_reorg(&mut self, last_good_block: u64, first_websocket_block: u64) -> IndexerResult<()> {
        info!(" Detecting gap between last good block {} and first WebSocket block {}", last_good_block, first_websocket_block);
        
        let gap_start = last_good_block + 1;
        let gap_end = first_websocket_block.saturating_sub(1);
        
        if gap_start <= gap_end {
            let gap_size = gap_end - gap_start + 1;
            info!("Reorg gap detected: blocks {} to {} ({} blocks)", gap_start, gap_end, gap_size);
            
            
            info!("Fetching gap events for blocks {} to {}", gap_start, gap_end);
            let gap_events = self.ethereum_client.scan_blocks(gap_start, gap_end, gap_size).await?;
            
            let mut events_processed = 0;
            let mut new_events = Vec::new();
            
            for event in &gap_events {
                let record = event.to_record();
                
                if !self.database.transfer_exists(&record.transaction_hash, record.log_index)? {
                    new_events.push(record);
                    events_processed += 1;
                }
            }
            
            if !new_events.is_empty() {
                self.database.store_transfers_batch(&new_events)?;
                info!("Stored {} gap events for blocks {} to {}", events_processed, gap_start, gap_end);
            }
            
            info!("Fetching and storing gap blocks in batch");
            let mut block_headers = Vec::new();
            for block_number in gap_start..=gap_end {
                if let Ok(Some(block_header)) = self.ethereum_client.get_block_header(block_number).await {
                    block_headers.push(block_header);
                } else {
                    warn!("Failed to get block header for gap block {}", block_number);
                }
            }
            
            if !block_headers.is_empty() {
                self.database.store_block_headers_batch(&block_headers)?;
                info!("Stored {} gap block headers in batch", block_headers.len());
            }
            
            debug!("Checking continuity at boundary: last good block {} ↔ gap block {}", last_good_block, gap_start);
            let continuity_ok = self.verify_realtime_continuity(gap_start).await?;
            if !continuity_ok {
                warn!("Continuity broken at gap start boundary");
                return Err(IndexerError::Other(format!("Continuity broken at gap start boundary {}", gap_start)));
            }
            
            debug!("Checking continuity at boundary: gap block {} ↔ WebSocket block {}", gap_end, first_websocket_block);
            let continuity_ok = self.verify_realtime_continuity(first_websocket_block).await?;
            if !continuity_ok {
                warn!("Continuity broken at gap end boundary");
                return Err(IndexerError::Other(format!("Continuity broken at gap end boundary {}", first_websocket_block)));
            }
            
            info!("Continuity verified at gap boundaries");
            
            info!(" Gap filling completed for {} blocks", gap_size);
        } else {
            info!("No gaps detected - WebSocket start ({}) immediately follows last good block ({})", first_websocket_block, last_good_block);
        }
        
        Ok(())
    }
    
    
    async fn  linear_backtrack(&self, reorg_block: u64) -> IndexerResult<Option<u64>> {
        info!(" Searching blocks {} to {}", reorg_block, reorg_block.saturating_sub(20));
        
        let mut current_block = reorg_block;
        let mut backtrack_count = 0;
        let max_backtrack = 20;
        
        while current_block > 0 && backtrack_count < max_backtrack {
            backtrack_count += 1;
            
            if backtrack_count % 5 == 0 {
                info!(" Step {} - Checking block {}", backtrack_count, current_block);
            }
            
            
            if let Some(db_header) = self.database.get_block_header(current_block)? {
                if let Ok(Some(actual_header)) = self.ethereum_client.get_block_header(current_block).await {
                    if db_header.hash == actual_header.hash && db_header.parent_hash == actual_header.parent_hash {
                        info!(" FOUND GOOD BLOCK at {} after {} steps", current_block, backtrack_count);
                        return Ok(Some(current_block));
                    }
                }
            }
            
            current_block -= 1;
        }
        
        info!(" No good block found in 20-block search");
        Ok(None)
    }
    
   
    async fn binary_search_blocks(&self, reorg_block: u64) -> IndexerResult<Option<u64>> {
        let blocks = self.database.get_all_block_numbers()?;
        
        if blocks.is_empty() {
            info!(" No blocks found");
            return Ok(None);
        }
        
        info!(" Searching {} blocks", blocks.len());
        
        
        let mut left: isize = 0;
        let mut right: isize = blocks.len().saturating_sub(1) as isize;
        let mut best_block = None;
        let mut search_count = 0;
        
        while left <= right {
            search_count += 1;
            let mid = ((left + right) / 2) as usize;
            let block_number = blocks[mid];
            
            if search_count % 3 == 0 {
                info!(" Step {} - Checking block {} (range: {} to {})", 
                      search_count, block_number, blocks[left as usize], blocks[right as usize]);
            }
            
           
            if let Some(db_header) = self.database.get_block_header(block_number)? {
                if let Ok(Some(actual_header)) = self.ethereum_client.get_block_header(block_number).await {
                    if db_header.hash == actual_header.hash && db_header.parent_hash == actual_header.parent_hash {
                       
                        best_block = Some(block_number);
                        info!(" Found good block at {}, continuing search for better", block_number);
                        left = mid as isize + 1;  
                    } else {
                       
                        info!(" Block {} is bad, searching lower", block_number);
                        right = mid.saturating_sub(1) as isize;
                    }
                } else {
                   
                    info!(" Couldn't fetch block {}, searching lower", block_number);
                    right = mid.saturating_sub(1) as isize;
                }
            } else {
                
                info!(" Block {} not in database, searching lower", block_number);
                right = mid.saturating_sub(1) as isize;
            }
        }
        
        match best_block {
            Some(good_block) => {
                info!(" FOUND BEST BLOCK at {} after {} searches", good_block, search_count);
                Ok(Some(good_block))
            }
            None => {
                info!(" No good block found after {} searches", search_count);
                Ok(None)
            }
        }
    }
    
  
   
    async fn rollback_to_block(&mut self, good_block: u64) -> IndexerResult<u64> {
        info!(" Rolling back database from current state to block {}", good_block);
        
        
        let deleted_blocks = self.database.rollback_blocks_to(good_block)?;
        info!(" Deleted {} blocks", deleted_blocks);
        
    
        

        let progress = self.database.get_progress()?;
        self.database.update_progress(good_block, progress.total_events_processed)?;
        
                  info!(" Database successfully rolled back to block {} (deleted {} blocks)", 
               good_block, deleted_blocks);
        Ok(good_block)
    }
    

    
    async fn start_polling_indexing(&mut self) -> IndexerResult<()> {
        info!("Starting polling-based indexing with single block processing");
        
        loop {
            let latest_block = match self.ethereum_client.get_latest_block_number().await {
                Ok(block) => block,
                Err(e) => {
                    error!("Failed to get latest block: {}", e);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };
    
            let progress = self.database.get_progress()?;
            let next_block = if progress.last_processed_block > 0 {
                progress.last_processed_block + 1
            } else {
                self.config.start_block
            };
    
            let finality_depth = self.config.finality_depth;
            let safe_latest_block = if latest_block > finality_depth {
                latest_block.saturating_sub(finality_depth)
            } else {
                latest_block
            };
    
            info!(
                "Polling: next_block={}, safe_latest_block={}, latest_block={}, finality_depth={}",
                next_block, safe_latest_block, latest_block, finality_depth
            );
    
            if next_block <= safe_latest_block {
                info!(
                    "Processing new blocks from {} to {} with single block processing",
                    next_block, safe_latest_block
                );
    
                for block_number in next_block..=safe_latest_block {
                    let continuity_ok = self.verify_realtime_continuity(block_number).await?;
                    if !continuity_ok {
                        warn!("Continuity broken in polling mode at block {}", block_number);
                        let recovery_block = self.handle_live_reorg_recovery(block_number).await?;
                        info!("Live reorg recovery completed, continuing from block {}", recovery_block);
                        break; 
                    }
    
                    if let Err(e) = self.process_new_block(block_number).await {
                        error!("Failed to process block {}: {}", block_number, e);
                    }
                }
            } else {
                info!("No new blocks to process: next_block ({}) > safe_latest_block ({})", next_block, safe_latest_block);
            }
    
            sleep(Duration::from_secs(15)).await;
        }
    }
    
    

    
    async fn process_new_block(&self, block_number: u64) -> IndexerResult<()> {
        
        let logs = self.ethereum_client.get_transfer_logs_for_block(block_number).await?;
        
        let mut events_processed = 0;
        let mut new_events = Vec::new();
        
        for log in logs {
            if let Some(transfer_event) = TransferEvent::from_log(log, self.ethereum_client.get_contract_address()) {
                let record = transfer_event.to_record();
                
                
                if let Ok(Some(_)) = self.database.get_transfer_by_tx_and_log(
                    &record.transaction_hash,
                    record.log_index
                ) {
                    debug!("Duplicate event found: tx={}, log_index={}", 
                           record.transaction_hash, record.log_index);
                    continue;
                }
                
                new_events.push(record);
                events_processed += 1;
            }
        }
        
        let block_header = match self.ethereum_client.get_block_header(block_number).await {
            Ok(Some(header)) => header,
            Ok(None) => {
                error!("Block header not found for block {}", block_number);
                return Err(IndexerError::Other(format!("Block header not found for block {}", block_number)));
            }
            Err(e) => {
                error!("Failed to get block header for block {}: {}", block_number, e);
                return Err(e);
            }
        };
        
        let progress = self.database.get_progress()?;
        let new_total_events = progress.total_events_processed + events_processed;
        
        self.write_queue.store_block_with_events_and_progress(new_events, block_header, block_number, new_total_events)?;
        
        if events_processed > 0 {
            info!("Processed {} transfer events from block {}", events_processed, block_number);
        }
        
        Ok(())
    }
    

    

    

    

    

    
    
    async fn websocket_buffering_task(   
        ethereum_client: Arc<dyn EthereumClientTrait + Send + Sync>,
        buffer: Arc<Mutex<HashMap<u64, BlockData>>>,
        sender: Sender<BlockData>,
        task_name: &str,
    ) -> IndexerResult<()> {
        
        let mut block_stream = match ethereum_client.subscribe_to_new_blocks().await {
            Ok(stream) => {
                info!("WebSocket subscription established successfully for {}", task_name);
                stream
            }
            Err(e) => {
                warn!("WebSocket subscription failed for {}: {}, cannot continue", task_name, e);
                return Err(IndexerError::Other(format!("WebSocket subscription failed: {}", e)));
            }
        };
        
        while let Some(block_number) = block_stream.recv().await {
            debug!("WebSocket block received: {}", block_number);
            
            if let Ok(Some(block_header)) = ethereum_client.get_block_header(block_number).await {
                let logs = ethereum_client.get_transfer_logs_for_block(block_number).await?;
                let mut events = Vec::new();
                
                for log in logs {
                    if let Some(transfer_event) = TransferEvent::from_log(log, ethereum_client.get_contract_address()) {
                        events.push(transfer_event.to_record());
                    }
                }
                
                let block_data = BlockData {
                    block_number,
                    block_header,
                    events,
                };
                
                {
                    let mut buffer_guard = buffer.lock().await;
                    buffer_guard.insert(block_number, block_data.clone());
                }
                
                if let Err(e) = sender.send(block_data).await {
                    error!("Failed to send block data: {}", e);
                }
            }
        }
        
        Ok(())
    }
    

    async fn start_live_sequential_processing_handoff(&mut self) -> IndexerResult<()> {
        info!("Starting live sequential processing");
        
        if let Some(mut receiver) = self.websocket_receiver.take() {
            while let Some(block_data) = receiver.recv().await {
                info!("Processing live block {}", block_data.block_number);
                
                
                let progress = self.database.get_progress()?;
                if progress.last_processed_block > 0 {
                    if let Ok(Some(last_block)) = self.database.get_block_header(progress.last_processed_block) {
                        info!("Block {}: parent_hash=0x{}, last_block_hash=0x{}", 
                               block_data.block_number, 
                               hex::encode(block_data.block_header.parent_hash),
                               hex::encode(last_block.hash));
                        
                        if block_data.block_header.parent_hash != last_block.hash {
                            warn!("Reorg detected at block {}: parent hash mismatch with last processed block {}", 
                                  block_data.block_number, progress.last_processed_block);
                            let recovery_block = self.handle_reorg_recovery(block_data.block_number, ReorgRecoveryType::WebSocket).await?;
                            info!("Reorg recovery completed, resuming from block {}", recovery_block);
                            continue;
                        }
                    }
                }
                
                self.process_live_block_with_verification(block_data).await?;
            }
        }
        
        Ok(())
    }

    async fn start_buffered_realtime_processing(&mut self) -> IndexerResult<()> {
        info!("Starting buffered real-time processing with gap handling");
        
        let progress = self.database.get_progress()?;
        let mut last_processed_block = progress.last_processed_block;
        
        let finality_depth = if self.config.websocket { self.config.finality_depth } else { 0 };
        let mut current_safe_tip = None;
        
        if let Some(mut receiver) = self.websocket_receiver.take() {
            while let Some(block_data) = receiver.recv().await {
                let block_number = block_data.block_number;
                
                // Skip blocks before the configured start block
                if self.config.start_block > 0 && block_number < self.config.start_block {
                    info!("Skipping block {} (before start block {})", block_number, self.config.start_block);
                    continue;
                }
                
                info!("Processing buffered block {}", block_number);
                
                
                if current_safe_tip.is_none() {
                    let head = self.ethereum_client.get_latest_block_number().await?;
                    current_safe_tip = Some(head.saturating_sub(finality_depth));
                    info!("Updated safe tip: head={}, safe_tip={}", head, current_safe_tip.unwrap());
                }
                
                let safe_tip = current_safe_tip.unwrap();
                if block_number > safe_tip {
                    let should_update = (block_number - safe_tip) % 5 == 0;
                    if should_update {
                        let head = self.ethereum_client.get_latest_block_number().await?;
                        current_safe_tip = Some(head.saturating_sub(finality_depth));
                        info!("Updated safe tip: head={}, safe_tip={}", head, current_safe_tip.unwrap());
                    }
                }
                
                let safe_tip = current_safe_tip.unwrap();
                if block_number > safe_tip {
                    info!("Block {} beyond safe tip {}, waiting for finality", block_number, safe_tip);
                }
                
               
                let expected_block = last_processed_block + 1;
                
                // Only detect gaps if we're not starting fresh (last_processed_block > 0)
                if block_number > expected_block && last_processed_block > 0 {
                    info!("Critical gap detected: blocks {} to {}", expected_block, block_number.saturating_sub(1));
                    info!("Starting gap backfill for {} blocks", block_number.saturating_sub(expected_block));
                    
                    let gap_start = expected_block;
                    let gap_end = block_number.saturating_sub(1);
                    
                    let events_processed = self.process_gap_blocks(gap_start, gap_end).await?;
                    
                    
                    if !self.verify_continuity_and_recover(block_number, "live boundary after gap fill").await? {
                        last_processed_block = self.handle_live_reorg_recovery(block_number).await?;
                        continue;
                    }
                    
            
                    last_processed_block = gap_end;
                } else if block_number > expected_block && last_processed_block == 0 {
                    info!("Fresh start detected: processing block {} as first block (no gap backfill needed)", block_number);
                } else if block_number < expected_block {
                    info!("Received out-of-order block {} (expected {}), skipping", block_number, expected_block);
                    continue;
                } else {
                    info!("No gap detected, processing block {} in sequence", block_number);
                }
                
                
                if last_processed_block > 0 {
                    if let Ok(Some(last_block)) = self.database.get_block_header(last_processed_block) {
                        if block_data.block_header.parent_hash != last_block.hash {
                            warn!("Reorg detected at block {}: parent hash mismatch with last processed block {}", 
                                  block_data.block_number, last_processed_block);
                            let recovery_block = self.handle_reorg_recovery(block_data.block_number, ReorgRecoveryType::WebSocket).await?;
                            info!("Reorg recovery completed, resuming from block {}", recovery_block);
                            last_processed_block = recovery_block;
                            continue;
                        }
                    }
                }
                
                // Process the block
                self.process_live_block_with_verification(block_data).await?;
                last_processed_block = block_number;
            }
        }
        
        Ok(())
    }
    
   
    async fn process_live_block_with_verification(&mut self, block_data: BlockData) -> IndexerResult<()> {
        let block_number = block_data.block_number;
        let events_count = block_data.events.len() as u64;
        
        let progress = self.database.get_progress()?;
        let new_total_events = progress.total_events_processed + events_count;
        
        self.write_queue.store_block_with_events_and_progress(block_data.events, block_data.block_header, block_number, new_total_events)?;
        
        debug!("Live block {} processed with {} events", block_number, events_count);
        
        if let Some(buf) = &self.websocket_buffer {
            let _ = buf.lock().await.remove(&block_number);
        }
        Ok(())
    }
    



} 

async fn process_block_batch(
    ethereum_client: &Arc<dyn EthereumClientTrait + Send + Sync>,
    database: &Database,
    write_queue: &DatabaseWriteQueue,
    start_block: u64,
    end_block: u64,
) -> IndexerResult<(u64, u64)> {

    info!("Processing batch: blocks {} to {}", start_block, end_block);
    
    let mut events_processed = 0;
    let blocks_processed;
    
    let events = ethereum_client.scan_blocks(start_block, end_block, end_block - start_block + 1).await?;
    info!("Retrieved {} transfer events", events.len());
   
    let mut new_events = Vec::new();
    for event in events.iter() {
        let record = event.to_record();
        
        match database.get_transfer_by_tx_and_log(&record.transaction_hash, record.log_index) {
            Ok(Some(_)) => {
                continue;
            }
            Ok(None) => {
                new_events.push(record);
                events_processed += 1;
            }
            Err(e) => {
                warn!("Error checking for duplicates: {}", e);
                new_events.push(record);
                events_processed += 1;
            }
        }
    }
    
   
    if events_processed > 0 {
        info!("Batch {}-{} processed {} events", start_block, end_block, events_processed);
    }
    
    blocks_processed = end_block - start_block + 1;
    
    
    let mut block_headers = Vec::new();
    
    match ethereum_client.get_block_header(start_block).await {
        Ok(Some(block_header)) => {
            block_headers.push(block_header);
        }
        Ok(None) => {
            warn!("Block header not found for start block {}", start_block);
        }
        Err(e) => {
            error!("Failed to get block header for start block {}: {}", start_block, e);
        }
    }

    if start_block != end_block {
        match ethereum_client.get_block_header(end_block).await {
            Ok(Some(block_header)) => {
                block_headers.push(block_header);
            }
            Ok(None) => {
                warn!("Block header not found for end block {}", end_block);
            }
            Err(e) => {
                error!("Failed to get block header for end block {}: {}", end_block, e);
            }
        }
    }
    
    
    if !new_events.is_empty() || !block_headers.is_empty() {
        write_queue.store_gap_fill(new_events, block_headers, end_block, 0, Some(start_block), Some(end_block))?;
    }
    
    info!("Batch {}-{}: {} events processed", start_block, end_block, events_processed);
    Ok((events_processed, blocks_processed))
} 


fn queue_store_batch(
    write_queue: &DatabaseWriteQueue,
    transfer_records: Vec<crate::models::TransferRecord>,
) -> IndexerResult<()> {
    
    write_queue.store_transfers(transfer_records)
}



