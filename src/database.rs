use rusqlite::{params, Connection, OptionalExtension};
use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;
use crate::models::{TransferRecord, BlockInfo, IndexingProgress};
use crate::error::{IndexerResult, IndexerError};
use crate::client_trait::BlockHeader;
use ethers::types::H256;
use tracing::{info, warn, error};
use std::path::Path;
use std::time::Duration;


fn parse_hex_to_bytes(hex_str: &str) -> IndexerResult<[u8; 32]> {
    let s = hex_str.trim_start_matches("0x");
    let v = hex::decode(s).map_err(|e| IndexerError::Other(format!("Invalid hex: {}", e)))?;
    
    if v.len() != 32 {
        return Err(IndexerError::Other(format!("Hex length != 32: {}", v.len())));
    }
    
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Ok(out)
}

#[derive(Clone)]
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> IndexerResult<Self> {
        info!("Opening database at {:?}", path.as_ref());
       
        let manager = SqliteConnectionManager::file(path);
        
        
        let pool = Pool::builder()
            .max_size(30)  
            .min_idle(Some(5))   
            .max_lifetime(Some(Duration::from_secs(600)))  
            .idle_timeout(Some(Duration::from_secs(120)))  
            .build(manager)?;
        
        info!("Connection pool created with 30 max connections");
        
        
        let conn = pool.get()?;
        Self::initialize_database(&conn)?;
        
        info!("Database initialization completed successfully");
        Ok(Self { pool })
    }
    
    fn initialize_database(conn: &Connection) -> IndexerResult<()> {
        info!("Starting database schema initialization");
        
       
        let journal_mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        info!("SQLite journal mode: {}", journal_mode);
        
        
        if journal_mode != "wal" {
            warn!("Failed to enable WAL mode, got: {}", journal_mode);
        }
        
        
        let test_result: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
        info!("Database connection test successful: {}", test_result);
        
       
        info!("Setting page size");
        conn.execute("PRAGMA page_size = 65536", [])?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        info!("SQLite page size: {}", page_size);
        
        
        if page_size != 65536 {
            info!("VACUUMing database after page size change");
            conn.execute("VACUUM", [])?;
            info!("VACUUM completed");
        }
        
      
        info!("Setting cache size");
        conn.execute("PRAGMA cache_size = -16384", [])?; 
        let cache_size: i64 = conn.query_row("PRAGMA cache_size", [], |row| row.get(0))?;
        info!("SQLite cache size: {} KB", -cache_size);
        
        
        info!("Skipping mmap_size PRAGMA for compatibility");
        
        
        info!("Setting synchronous mode");
        conn.execute("PRAGMA synchronous = NORMAL", [])?;
        let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        info!("SQLite synchronous mode: {}", synchronous);
        
        
        info!("Setting temp store");
        conn.execute("PRAGMA temp_store = MEMORY", [])?;
        let temp_store: i64 = conn.query_row("PRAGMA temp_store", [], |row| row.get(0))?;
        info!("SQLite temp store: {}", temp_store);
        
        
        info!("Setting foreign keys");
        conn.execute("PRAGMA foreign_keys = OFF", [])?;
        let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        info!("SQLite foreign keys: {}", foreign_keys);
        
      
        conn.execute(
            "CREATE TABLE IF NOT EXISTS transfers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                value TEXT NOT NULL,
                transaction_hash TEXT NOT NULL,
                block_number INTEGER NOT NULL,
                log_index INTEGER NOT NULL,
                block_hash TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                timestamp INTEGER,
                created_at INTEGER NOT NULL,
                UNIQUE(transaction_hash, log_index)
            )",
            [],
        )?;
        info!("Transfers table created");
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS progress (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_processed_block INTEGER NOT NULL DEFAULT 0,
                total_events_processed INTEGER NOT NULL DEFAULT 0,
                last_processed_timestamp INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        info!("Progress table created");
        
        conn.execute(
            "INSERT OR IGNORE INTO progress (id, last_processed_block, total_events_processed, last_processed_timestamp, created_at, updated_at)
             VALUES (1, 0, 0, 0, ?, ?)",
            params![chrono::Utc::now().timestamp(), chrono::Utc::now().timestamp()],
        )?;
        info!("Initial progress record inserted/verified");
        
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                number INTEGER PRIMARY KEY,
                hash TEXT NOT NULL,
                parent_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                batch_start INTEGER,
                batch_end INTEGER,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        info!("Blocks table created");
        
      
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_blocks (
                number INTEGER PRIMARY KEY,
                hash TEXT NOT NULL,
                parent_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                ingested_at INTEGER NOT NULL
            )",
            [],
        )?;
        info!("Pending blocks table created");
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id INTEGER NOT NULL DEFAULT 1,
                block_number INTEGER NOT NULL,
                transaction_hash TEXT NOT NULL,
                log_index INTEGER NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                value TEXT NOT NULL,
                block_hash TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                timestamp INTEGER,
                ingested_at INTEGER NOT NULL,
                UNIQUE(chain_id, block_number, transaction_hash, log_index)
            )",
            [],
        )?;
        info!("Pending events table created");
        
        info!("Creating optimized indexes");
        
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transfers_tx_log ON transfers(transaction_hash, log_index)",
            [],
        )?;
        info!("Created index on (transaction_hash, log_index)");
        
       
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transfers_block ON transfers(block_number)",
            [],
        )?;
        info!("Created index on block_number");
        
      
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transfers_from ON transfers(from_address)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transfers_to ON transfers(to_address)",
            [],
        )?;
        info!("Created indexes on from_address and to_address");
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_number ON blocks(number)",
            [],
        )?;
        info!("Created index on blocks.number");
        
        

        
        

        
        info!("Database schema initialized successfully");
        Ok(())
    }
    

    
    pub fn store_transfers_batch(&self, transfers: &[TransferRecord]) -> IndexerResult<()> {
        if transfers.is_empty() {
            return Ok(());
        }
        
        let mut conn = self.pool.get()?;
        
        
        let tx = conn.transaction()?;
        
        {
            
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO transfers 
                 (from_address, to_address, value, transaction_hash, block_number, log_index, block_hash, contract_address, timestamp, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )?;
            
            for transfer in transfers {
                stmt.execute(params![
                    transfer.from_address,
                    transfer.to_address,
                    transfer.value,
                    transfer.transaction_hash,
                    transfer.block_number,
                    transfer.log_index,
                    transfer.block_hash,
                    transfer.contract_address,
                    transfer.timestamp,
                    transfer.created_at,
                ])?;
            }
        } 
        
        
        tx.commit()?;
        
        info!("Bulk inserted {} transfers", transfers.len());
        Ok(())
    }
    

    

    
    pub fn update_progress(&self, last_block: u64, total_events: u64) -> IndexerResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE progress SET 
             last_processed_block = ?, 
             total_events_processed = ?, 
             last_processed_timestamp = ?,
             updated_at = ?
             WHERE id = 1",
            params![
                last_block,
                total_events,
                chrono::Utc::now().timestamp(),
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }
    
    pub fn get_progress(&self) -> IndexerResult<IndexingProgress> {
        let conn = self.pool.get()?;
        info!("Got database connection for progress query");
        
        
        let test_count: i64 = conn.query_row("SELECT COUNT(*) FROM progress", [], |row| row.get(0))?;
        info!("Progress table has {} rows", test_count);
        
        let mut stmt = conn.prepare(
            "SELECT last_processed_block, total_events_processed, last_processed_timestamp 
             FROM progress WHERE id = 1"
        )?;
        info!("Prepared progress query statement");
        
        let progress = stmt.query_row([], |row| {
            Ok(IndexingProgress {
                last_processed_block: row.get(0)?,
                total_events_processed: row.get(1)?,
                last_processed_timestamp: row.get(2)?,
            })
        })?;
        info!("Successfully queried progress: block={}, events={}", 
              progress.last_processed_block, progress.total_events_processed);
        
        Ok(progress)
    }
    
    pub fn get_transfer_by_tx_and_log(&self, tx_hash: &str, log_index: u32) -> IndexerResult<Option<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE transaction_hash = ? AND log_index = ?"
        )?;
        
        let transfer = stmt.query_row(params![tx_hash, log_index], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        }).optional()?;
        
        Ok(transfer)
    }
    
    
    pub fn transfer_exists(&self, tx_hash: &str, log_index: u32) -> IndexerResult<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT 1 FROM transfers WHERE transaction_hash = ? AND log_index = ? LIMIT 1"
        )?;
        
        let exists = stmt.query_row(params![tx_hash, log_index], |_| Ok(())).is_ok();
        Ok(exists)
    }
    
    pub fn get_transfers_by_block(&self, block_number: u64) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE block_number = ?
             ORDER BY log_index"
        )?;
        
        let transfers = stmt.query_map(params![block_number], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }
    
    pub fn get_transfers_by_address(&self, address: &str) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE from_address = ? OR to_address = ?
             ORDER BY block_number DESC, log_index DESC 
             LIMIT 100"
        )?;
        
        let transfers = stmt.query_map(params![address, address], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }
    
    pub fn get_block_info(&self, block_number: u64) -> IndexerResult<Option<BlockInfo>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT number, hash, parent_hash, timestamp
             FROM blocks 
             WHERE number = ?"
        )?;
        
        let block = stmt.query_row(params![block_number], |row| {
            let hash_str: String = row.get(1)?;
            let parent_hash_str: String = row.get(2)?;
            
            Ok(BlockInfo {
                number: row.get(0)?,
                hash: H256::from_slice(&parse_hex_to_bytes(&hash_str)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?),
                parent_hash: H256::from_slice(&parse_hex_to_bytes(&parent_hash_str)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?),
                timestamp: row.get(3)?,
            })
        }).optional()?;
        
        Ok(block)
    }

   
    pub fn get_block_by_number(&self, block_number: u64) -> IndexerResult<Option<BlockInfo>> {
        self.get_block_info(block_number)
    }

    
    pub fn rollback_to_block(&self, block_number: u64) -> IndexerResult<()> {
        let mut conn = self.pool.get()?;
        
        info!("Rolling back database to block {}", block_number);
        
       
        let tx = conn.transaction()?;
        
       

        
        
        let deleted_transfers = tx.execute(
            "DELETE FROM transfers WHERE block_number > ?",
            params![block_number],
        )?;
        
       
        let deleted_blocks = tx.execute(
            "DELETE FROM blocks WHERE number > ?",
            params![block_number],
        )?;
        
               
        tx.execute(
            "UPDATE progress SET last_processed_block = ?, updated_at = ? WHERE id = 1",
            params![block_number, chrono::Utc::now().timestamp()],
        )?;
        
       
        tx.commit()?;
        
        info!("Rollback completed: deleted {} transfers, {} blocks", 
              deleted_transfers, deleted_blocks);
        Ok(())
    }

    

    
    pub fn get_statistics(&self) -> IndexerResult<(u64, u64)> {
        let conn = self.pool.get()?;
        
        let total_transfers: i64 = conn.query_row("SELECT COUNT(*) FROM transfers", [], |row| row.get(0))?;
        let total_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))?;
        
        Ok((total_transfers as u64, total_blocks as u64))
    }

   
    pub fn get_transfers_by_transaction(&self, tx_hash: &str) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE transaction_hash = ?
             ORDER BY log_index"
        )?;
        
        let transfers = stmt.query_map(params![tx_hash], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }

    pub fn get_transfers_by_value_range(&self, min_value: &str, max_value: &str, limit: usize) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE CAST(value AS INTEGER) BETWEEN ? AND ?
             ORDER BY CAST(value AS INTEGER) DESC
             LIMIT ?"
        )?;
        
        let transfers = stmt.query_map(params![min_value, max_value, limit], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }

    pub fn get_recent_transfers(&self, count: usize) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             ORDER BY created_at DESC
             LIMIT ?"
        )?;
        
        let transfers = stmt.query_map(params![count], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }

    pub fn search_transfers(&self, from: &Option<String>, to: &Option<String>, min_block: Option<u64>, max_block: Option<u64>, limit: usize) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        
        let mut query = String::from(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE 1=1"
        );
        
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        if let Some(from_addr) = from {
            query.push_str(" AND from_address = ?");
            params.push(Box::new(from_addr.clone()));
        }
        
        if let Some(to_addr) = to {
            query.push_str(" AND to_address = ?");
            params.push(Box::new(to_addr.clone()));
        }
        
        if let Some(min) = min_block {
            query.push_str(" AND block_number >= ?");
            params.push(Box::new(min));
        }
        
        if let Some(max) = max_block {
            query.push_str(" AND block_number <= ?");
            params.push(Box::new(max));
        }
        
        query.push_str(" ORDER BY block_number DESC, log_index DESC LIMIT ?");
        params.push(Box::new(limit));
        
        let mut stmt = conn.prepare(&query)?;
        
        let transfers = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }

    pub fn export_transfers(&self, from_block: Option<u64>, to_block: Option<u64>, limit: usize) -> IndexerResult<Vec<TransferRecord>> {
        let conn = self.pool.get()?;
        
        let mut query = String::from(
            "SELECT id, from_address, to_address, value, transaction_hash, block_number, log_index, 
                    block_hash, contract_address, timestamp, created_at
             FROM transfers 
             WHERE 1=1"
        );
        
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        if let Some(from) = from_block {
            query.push_str(" AND block_number >= ?");
            params.push(Box::new(from));
        }
        
        if let Some(to) = to_block {
            query.push_str(" AND block_number <= ?");
            params.push(Box::new(to));
        }
        
        query.push_str(" ORDER BY block_number, log_index LIMIT ?");
        params.push(Box::new(limit));
        
        let mut stmt = conn.prepare(&query)?;
        
        let transfers = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                from_address: row.get(1)?,
                to_address: row.get(2)?,
                value: row.get(3)?,
                transaction_hash: row.get(4)?,
                block_number: row.get(5)?,
                log_index: row.get(6)?,
                block_hash: row.get(7)?,
                contract_address: row.get(8)?,
                timestamp: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        
        let mut result = Vec::new();
        for transfer in transfers {
            result.push(transfer?);
        }
        
        Ok(result)
    }

   
    pub fn get_earliest_processed_block(&self) -> IndexerResult<Option<u64>> {
        let conn = self.pool.get()?;
        let result: Option<i64> = conn.query_row(
            "SELECT MIN(number) FROM blocks",
            [],
            |row| {
                let val: Option<i64> = row.get(0)?;
                Ok(val)
            }
        ).optional()?.flatten();
        
        Ok(result.map(|n| n as u64))
    }

  
    pub fn get_latest_processed_block(&self) -> IndexerResult<Option<u64>> {
        let conn = self.pool.get()?;
        let result: Option<i64> = conn.query_row(
            "SELECT MAX(number) FROM blocks",
            [],
            |row| {
                let val: Option<i64> = row.get(0)?;
                Ok(val)
            }
        ).optional()?.flatten();
        
        Ok(result.map(|n| n as u64))
    }

    pub fn store_block_header(&self, block_header: &BlockHeader, batch_start: Option<u64>, batch_end: Option<u64>) -> IndexerResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO blocks (number, hash, parent_hash, timestamp, batch_start, batch_end, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                block_header.number,
                format!("0x{}", hex::encode(block_header.hash)),
                format!("0x{}", hex::encode(block_header.parent_hash)),
                block_header.timestamp,
                batch_start,
                batch_end,
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }
    
    
    pub fn store_block_header_simple(&self, block_header: &BlockHeader) -> IndexerResult<()> {
        self.store_block_header(block_header, None, None)
    }
    
    pub fn store_block_headers_batch(&self, block_headers: &[BlockHeader]) -> IndexerResult<()> {
        if block_headers.is_empty() {
            return Ok(());
        }
        
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO blocks (number, hash, parent_hash, timestamp, created_at)
                 VALUES (?, ?, ?, ?, ?)"
            )?;
            
            for block_header in block_headers {
                stmt.execute(params![
                    block_header.number,
                    format!("0x{}", hex::encode(block_header.hash)),
                    format!("0x{}", hex::encode(block_header.parent_hash)),
                    block_header.timestamp,
                    chrono::Utc::now().timestamp(),
                ])?;
            }
        }
        
        tx.commit()?;
        Ok(())
    }
    
    pub fn get_block_header(&self, block_number: u64) -> IndexerResult<Option<BlockHeader>> {
        let conn = self.pool.get()?;
        let result: Option<(String, String, i64)> = conn.query_row(
            "SELECT hash, parent_hash, timestamp FROM blocks WHERE number = ?",
            params![block_number],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            }
        ).optional()?;
        
        match result {
            Some((hash_str, parent_hash_str, timestamp)) => {
                let hash_str: String = hash_str;
                let parent_hash_str: String = parent_hash_str;
                
                let hash = parse_hex_to_bytes(&hash_str)?;
                let parent_hash = parse_hex_to_bytes(&parent_hash_str)?;
                
                Ok(Some(BlockHeader {
                    number: block_number,
                    hash,
                    parent_hash,
                    timestamp: timestamp as u64,
                }))
            }
            None => Ok(None)
        }
    }
    
    pub fn get_block_with_batch_info(&self, block_number: u64) -> IndexerResult<Option<(BlockHeader, Option<u64>, Option<u64>)>> {
        let conn = self.pool.get()?;
        let result: Option<(String, String, i64, Option<i64>, Option<i64>)> = conn.query_row(
            "SELECT hash, parent_hash, timestamp, batch_start, batch_end FROM blocks WHERE number = ?",
            params![block_number],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            }
        ).optional()?;
        
        match result {
            Some((hash_str, parent_hash_str, timestamp, batch_start, batch_end)) => {
                let hash_str: String = hash_str;
                let parent_hash_str: String = parent_hash_str;
                
                let hash = parse_hex_to_bytes(&hash_str)?;
                let parent_hash = parse_hex_to_bytes(&parent_hash_str)?;
                
                let block_header = BlockHeader {
                    number: block_number,
                    hash,
                    parent_hash,
                    timestamp: timestamp as u64,
                };
                
                Ok(Some((block_header, batch_start.map(|x| x as u64), batch_end.map(|x| x as u64))))
            }
            None => Ok(None)
        }
    }
    

    
    

    
    

    
    

    
    
    pub fn rollback_blocks_to(&self, block_number: u64) -> IndexerResult<u64> {
        let conn = self.pool.get()?;
        let deleted_count = conn.execute(
            "DELETE FROM blocks WHERE number > ?",
            params![block_number],
        )?;
        Ok(deleted_count as u64)
    }

   
    pub fn get_all_block_numbers(&self) -> IndexerResult<Vec<u64>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT number FROM blocks ORDER BY number")?;
        let rows = stmt.query_map([], |row| {
            let number: i64 = row.get(0)?;
            Ok(number as u64)
        })?;
        
        let mut block_numbers = Vec::new();
        for row in rows {
            block_numbers.push(row?);
        }
        
        Ok(block_numbers)
    }

   


    
    pub fn initialize_schema(&self) -> IndexerResult<()> {
        let conn = self.pool.get()?;
        
       
        conn.execute(
            "CREATE TABLE IF NOT EXISTS transfers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                transaction_hash TEXT NOT NULL,
                log_index INTEGER NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                value TEXT NOT NULL,
                block_number INTEGER NOT NULL,
                block_hash TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(block_number, transaction_hash, log_index)
            )",
            [],
        )?;
        
        
                conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                number INTEGER PRIMARY KEY,
                hash TEXT NOT NULL,
                parent_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                batch_start INTEGER,
                batch_end INTEGER,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
    
        conn.execute(
            "CREATE TABLE IF NOT EXISTS progress (
                id INTEGER PRIMARY KEY,
                last_processed_block INTEGER NOT NULL DEFAULT 0,
                total_events_processed INTEGER NOT NULL DEFAULT 0,
                last_processed_timestamp INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        
       
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id INTEGER NOT NULL DEFAULT 1,
                block_number INTEGER NOT NULL,
                transaction_hash TEXT NOT NULL,
                log_index INTEGER NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                value TEXT NOT NULL,
                block_hash TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                timestamp INTEGER,
                ingested_at INTEGER NOT NULL,
                UNIQUE(chain_id, block_number, transaction_hash, log_index)
            )",
            [],
        )?;
        
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_blocks (
                number INTEGER PRIMARY KEY,
                hash TEXT NOT NULL,
                parent_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                ingested_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        
        conn.execute(
            "INSERT OR IGNORE INTO progress (id, last_processed_block, total_events_processed, last_processed_timestamp, updated_at)
             VALUES (1, 0, 0, 0, ?)",
            params![chrono::Utc::now().timestamp()],
        )?;
        
        Ok(())
    }

} 


use tokio::sync::mpsc;

#[derive(Debug)]
pub enum DatabaseOperation {
    StoreTransfers(Vec<TransferRecord>),
    StoreCheckpointBlock(BlockHeader, u64, u64),
    StoreLiveBlock(BlockHeader),
    UpdateProgress(u64, u64),
    StoreBlockWithEventsAndProgress {
        transfers: Vec<TransferRecord>,
        block_header: BlockHeader,
        last_block: u64,
        total_events: u64,
    },
    StoreGapFill {
        transfers: Vec<TransferRecord>,
        block_headers: Vec<BlockHeader>,
        last_block: u64,
        total_events: u64,
        batch_start: Option<u64>,
        batch_end: Option<u64>,
    },
}

#[derive(Clone)]
pub struct DatabaseWriteQueue {
    sender: mpsc::UnboundedSender<DatabaseOperation>,
    handle: std::sync::Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<IndexerResult<()>>>>>,
}

impl DatabaseWriteQueue {
    pub fn new(database: Database) -> Self {
        
        let (tx, mut rx) = mpsc::unbounded_channel::<DatabaseOperation>();
        
        let handle = tokio::spawn(async move {
            info!("Database write queue started");
            
            while let Some(operation) = rx.recv().await {
                match operation {
                    DatabaseOperation::StoreTransfers(transfers) => {
                        if let Err(e) = database.store_transfers_batch(&transfers) {
                            error!("Database write queue: Failed to store transfers: {}", e);
                        } else {
                            info!("Database write queue: Stored {} transfers", transfers.len());
                        }
                    }
                    DatabaseOperation::StoreCheckpointBlock(header, batch_start, batch_end) => {
                        if let Err(e) = database.store_checkpoint_block_header(&header, batch_start, batch_end) {
                            error!("Database write queue: Failed to store checkpoint block: {}", e);
                        } else {
                            info!("Database write queue: Stored checkpoint block {}", header.number);
                        }
                    }

                    DatabaseOperation::UpdateProgress(last_block, total_events) => {
                        const MAX_RETRIES: u32 = 3;
                        let mut retry_count = 0;
                        let mut success = false;
                        while retry_count < MAX_RETRIES && !success {
                            match database.update_progress(last_block, total_events) {
                                Ok(_) => {
                                    info!("Database write queue: Updated progress: last_block={}, total_events={}", last_block, total_events);
                                    success = true;
                                }
                                Err(e) => {
                                    retry_count += 1;
                                    if retry_count >= MAX_RETRIES {
                                        error!("Database write queue: Failed to update progress after {} retries: {}", MAX_RETRIES, e);
                                    } else {
                                        warn!("Database write queue: Failed to update progress (attempt {}/{}), retrying: {}", retry_count, MAX_RETRIES, e);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * retry_count as u64)).await;
                                    }
                                }
                            }
                        }
                    }
                    DatabaseOperation::StoreLiveBlock(header) => {
                        if let Err(e) = database.store_live_block_header(&header) {
                            error!("Database write queue: Failed to store live block: {}", e);
                        } else {
                            info!("Database write queue: Stored live block {}", header.number);
                        }
                    }
                    DatabaseOperation::StoreBlockWithEventsAndProgress { transfers, block_header, last_block, total_events } => {
                        const MAX_RETRIES: u32 = 3;
                        let mut retry_count = 0;
                        let mut success = false;
                        
                        while retry_count < MAX_RETRIES && !success {
                            match database.store_block_with_events_and_progress_atomic(&transfers, &block_header, last_block, total_events) {
                                Ok(_) => {
                                    info!("Database write queue: Stored block {} with {} events and updated progress", block_header.number, transfers.len());
                                    success = true;
                                }
                                Err(e) => {
                                    retry_count += 1;
                                    if retry_count >= MAX_RETRIES {
                                        error!("Database write queue: Failed to store block with events and progress   after {} retries: {}", MAX_RETRIES, e);
                                    } else {
                                        warn!("Database write queue: Failed to store block with events and progress   (attempt {}/{}), retrying: {}", retry_count, MAX_RETRIES, e);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * retry_count as u64)).await;
                                    }
                                }
                            }
                        }
                    }
                    DatabaseOperation::StoreGapFill { transfers, block_headers, last_block, total_events, batch_start, batch_end } => {
                        const MAX_RETRIES: u32 = 3;
                        let mut retry_count = 0;
                        let mut success = false;
                        
                        while retry_count < MAX_RETRIES && !success {
                            match database.store_gap_fill_atomic(&transfers, &block_headers, last_block, total_events, batch_start, batch_end) {
                                Ok(_) => {
                                    info!("Database write queue: Stored gap fill with {} events and {} blocks  ", transfers.len(), block_headers.len());
                                    success = true;
                                }
                                Err(e) => {
                                    retry_count += 1;
                                    if retry_count >= MAX_RETRIES {
                                        error!("Database write queue: Failed to store gap fill   after {} retries: {}", MAX_RETRIES, e);
                                    } else {
                                        warn!("Database write queue: Failed to store gap fill   (attempt {}/{}), retrying: {}", retry_count, MAX_RETRIES, e);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * retry_count as u64)).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            info!("Database write queue stopped");
            Ok(())
        });
        
        Self { 
            sender: tx, 
            handle: std::sync::Arc::new(tokio::sync::Mutex::new(Some(handle))) 
        }
    }
    
    pub fn store_transfers(&self, transfers: Vec<TransferRecord>) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::StoreTransfers(transfers))
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    
    pub fn store_checkpoint_block(&self, header: BlockHeader, batch_start: u64, batch_end: u64) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::StoreCheckpointBlock(header, batch_start, batch_end))
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    

    
    pub fn update_progress(&self, last_block: u64, total_events: u64) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::UpdateProgress(last_block, total_events))
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    
    pub fn store_live_block_header(&self, header: BlockHeader) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::StoreLiveBlock(header))
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    
    pub fn store_block_with_events_and_progress(&self, transfers: Vec<TransferRecord>, block_header: BlockHeader, last_block: u64, total_events: u64) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::StoreBlockWithEventsAndProgress { transfers, block_header, last_block, total_events })
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    
    pub fn store_gap_fill(&self, transfers: Vec<TransferRecord>, block_headers: Vec<BlockHeader>, last_block: u64, total_events: u64, batch_start: Option<u64>, batch_end: Option<u64>) -> IndexerResult<()> {
        self.sender.send(DatabaseOperation::StoreGapFill { transfers, block_headers, last_block, total_events, batch_start, batch_end })
            .map_err(|e| crate::error::IndexerError::Other(format!("Failed to send to write queue: {}", e)))
    }
    
    pub async fn shutdown(self) -> IndexerResult<()> {
        drop(self.sender);
        let mut handle_guard = self.handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.await
                .map_err(|e| crate::error::IndexerError::Other(format!("Write queue task failed: {}", e)))?
        } else {
            Ok(())
        }
    }
}

impl Database {
    pub fn create_write_queue(&self) -> DatabaseWriteQueue {
        DatabaseWriteQueue::new(self.clone())
    }
    
    pub fn store_checkpoint_block_header(&self, header: &BlockHeader, batch_start: u64, batch_end: u64) -> IndexerResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO checkpoint_blocks (number, hash, parent_hash, timestamp, batch_start, batch_end, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                header.number,
                format!("0x{}", hex::encode(header.hash)),
                format!("0x{}", hex::encode(header.parent_hash)),
                header.timestamp,
                batch_start,
                batch_end,
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }
    
    pub fn store_live_block_header(&self, header: &BlockHeader) -> IndexerResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO blocks (number, hash, parent_hash, timestamp, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                header.number,
                format!("0x{}", hex::encode(header.hash)),
                format!("0x{}", hex::encode(header.parent_hash)),
                header.timestamp,
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }
    
    pub fn store_block_with_events_and_progress_atomic(&self, transfers: &[TransferRecord], block_header: &BlockHeader, last_block: u64, total_events: u64) -> IndexerResult<()> {
        const MAX_RETRIES: u32 = 3;
        let mut retry_count = 0;
        
        info!("Storing {} transfers for block {}", transfers.len(), block_header.number);
        
        loop {
            let mut conn = self.pool.get()?;
            
            let tx = conn.transaction()?;
            
            if !transfers.is_empty() {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO transfers 
                     (from_address, to_address, value, transaction_hash, block_number, log_index, block_hash, contract_address, timestamp, created_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )?;
                
                for transfer in transfers.iter() {
                    stmt.execute(params![
                        transfer.from_address,
                        transfer.to_address,
                        transfer.value,
                        transfer.transaction_hash,
                        transfer.block_number,
                        transfer.log_index,
                        transfer.block_hash,
                        transfer.contract_address,
                        transfer.timestamp,
                        transfer.created_at,
                    ])?;
                }
            }
            
            tx.execute(
                "INSERT OR REPLACE INTO blocks (number, hash, parent_hash, timestamp, created_at)
                 VALUES (?, ?, ?, ?, ?)",
                params![
                    block_header.number,
                    format!("0x{}", hex::encode(block_header.hash)),
                    format!("0x{}", hex::encode(block_header.parent_hash)),
                    block_header.timestamp,
                    chrono::Utc::now().timestamp(),
                ],
            )?;
            
            tx.execute(
                "UPDATE progress SET 
                 last_processed_block = ?, 
                 total_events_processed = ?, 
                 last_processed_timestamp = ?,
                 updated_at = ?
                 WHERE id = 1",
                params![
                    last_block,
                    total_events,
                    chrono::Utc::now().timestamp(),
                    chrono::Utc::now().timestamp(),
                ],
            )?;
            
            match tx.commit() {
                Ok(_) => {
                    info!(" Stored block {} with {} events and updated progress", block_header.number, transfers.len());
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        error!("Failed to commit transaction after {} retries: {}", MAX_RETRIES, e);
                        return Err(crate::error::IndexerError::Database(e));
                    }
                    warn!("Transaction commit failed (attempt {}/{}), retrying: {}", retry_count, MAX_RETRIES, e);
                    std::thread::sleep(std::time::Duration::from_millis(100 * retry_count as u64));
                }
            }
        }
    }

    pub fn store_gap_fill_atomic(&self, transfers: &[TransferRecord], block_headers: &[BlockHeader], last_block: u64, total_events: u64, batch_start: Option<u64>, batch_end: Option<u64>) -> IndexerResult<()> {
        let mut conn = self.pool.get()?;
        
        let tx = conn.transaction()?;
        
        if !transfers.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO transfers 
                 (from_address, to_address, value, transaction_hash, block_number, log_index, block_hash, contract_address, timestamp, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )?;
            
            for transfer in transfers {
                stmt.execute(params![
                    transfer.from_address,
                    transfer.to_address,
                    transfer.value,
                    transfer.transaction_hash,
                    transfer.block_number,
                    transfer.log_index,
                    transfer.block_hash,
                    transfer.contract_address,
                    transfer.timestamp,
                    transfer.created_at,
                ])?;
            }
        }
        
        for block_header in block_headers {
            tx.execute(
                "INSERT OR REPLACE INTO blocks (number, hash, parent_hash, timestamp, batch_start, batch_end, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    block_header.number,
                    format!("0x{}", hex::encode(block_header.hash)),
                    format!("0x{}", hex::encode(block_header.parent_hash)),
                    block_header.timestamp,
                    batch_start,
                    batch_end,
                    chrono::Utc::now().timestamp(),
                ],
            )?;
        }
        
        tx.execute(
            "UPDATE progress SET 
             last_processed_block = ?, 
             total_events_processed = ?, 
             last_processed_timestamp = ?,
             updated_at = ?
             WHERE id = 1",
            params![
                last_block,
                total_events,
                chrono::Utc::now().timestamp(),
                chrono::Utc::now().timestamp(),
            ],
        )?;
        
        tx.commit()?;
        
        info!("Stored gap fill with {} events and {} blocks", transfers.len(), block_headers.len());
        Ok(())
    }

    pub fn find_duplicate_transfers(&self) -> IndexerResult<Vec<(String, i32, Vec<TransferRecord>)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT transaction_hash, log_index, COUNT(*) as count
             FROM transfers 
             GROUP BY transaction_hash, log_index 
             HAVING COUNT(*) > 1
             ORDER BY count DESC, transaction_hash, log_index"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;
        
        let mut duplicates = Vec::new();
        for row in rows {
            let (tx_hash, log_index, count) = row?;
            
            let mut stmt2 = conn.prepare(
                "SELECT id, transaction_hash, log_index, from_address, to_address, value, 
                        block_number, block_hash, contract_address, timestamp, created_at
                 FROM transfers 
                 WHERE transaction_hash = ? AND log_index = ?
                 ORDER BY id"
            )?;
            
            let transfer_rows = stmt2.query_map(params![tx_hash, log_index], |row| {
                Ok(TransferRecord {
                    id: row.get(0)?,
                    transaction_hash: row.get(1)?,
                    log_index: row.get(2)?,
                    from_address: row.get(3)?,
                    to_address: row.get(4)?,
                    value: row.get(5)?,
                    block_number: row.get(6)?,
                    block_hash: row.get(7)?,
                    contract_address: row.get(8)?,
                    timestamp: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?;
            
            let mut transfers = Vec::new();
            for transfer_row in transfer_rows {
                transfers.push(transfer_row?);
            }
            
            duplicates.push((tx_hash, count, transfers));
        }
        
        Ok(duplicates)
    }
}