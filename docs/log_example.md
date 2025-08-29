# Log Examples

This document contains example logs from different indexing modes to help understand the application behavior and output format.

## Historical Indexing Mode

**Command:**
```bash
./target/release/ZamaAssignment --adpetherrs --start-block 23222692 --finality-depth 5 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --end-block 23224692
```

**Log Output:**
```
2025-08-27T07:22:45.832113Z  INFO ZamaAssignment: Starting Ethereum Log Indexer and Verifier
2025-08-27T07:22:45.832133Z  INFO ZamaAssignment: Using library: Adaptive ethers-rs
2025-08-27T07:22:45.832144Z  INFO ZamaAssignment: Starting Standard Indexer
2025-08-27T07:22:45.832159Z  INFO ZamaAssignment::database: Opening database at "ethereum_logs.db"
2025-08-27T07:22:45.832791Z  INFO ZamaAssignment::database: Connection pool created with 30 max connections
2025-08-27T07:22:45.832804Z  INFO ZamaAssignment::database: Starting database schema initialization
2025-08-27T07:22:45.833376Z  INFO ZamaAssignment::database: SQLite journal mode: wal
2025-08-27T07:22:45.833391Z  INFO ZamaAssignment::database: Database connection test successful: 1
2025-08-27T07:22:45.833396Z  INFO ZamaAssignment::database: Setting page size...
2025-08-27T07:22:45.833410Z  INFO ZamaAssignment::database: SQLite page size: 4096
2025-08-27T07:22:45.833418Z  INFO ZamaAssignment::database: VACUUMing database after page size change...
2025-08-27T07:22:45.848047Z  INFO ZamaAssignment::database: VACUUM completed
2025-08-27T07:22:45.848060Z  INFO ZamaAssignment::database: Setting cache size...
2025-08-27T07:22:45.848193Z  INFO ZamaAssignment::database: SQLite cache size: 16384 KB
2025-08-27T07:22:45.848202Z  INFO ZamaAssignment::database: Skipping mmap_size PRAGMA for compatibility
2025-08-27T07:22:45.848209Z  INFO ZamaAssignment::database: Setting synchronous mode...
2025-08-27T07:22:45.848224Z  INFO ZamaAssignment::database: SQLite synchronous mode: 1
2025-08-27T07:22:45.848232Z  INFO ZamaAssignment::database: Setting temp store...
2025-08-27T07:22:45.848245Z  INFO ZamaAssignment::database: SQLite temp store: 2
2025-08-27T07:22:45.848250Z  INFO ZamaAssignment::database: Setting foreign keys...
2025-08-27T07:22:45.848257Z  INFO ZamaAssignment::database: SQLite foreign keys: 0
2025-08-27T07:22:45.848262Z  INFO ZamaAssignment::database: Skipping busy_timeout PRAGMA for compatibility
2025-08-27T07:22:45.848280Z  INFO ZamaAssignment::database: Transfers table created/verified
2025-08-27T07:22:45.848295Z  INFO ZamaAssignment::database: Progress table created/verified
2025-08-27T07:22:45.848340Z  INFO ZamaAssignment::database: Initial progress record inserted/verified
2025-08-27T07:22:45.848355Z  INFO ZamaAssignment::database: Blocks table created/verified
2025-08-27T07:22:45.848367Z  INFO ZamaAssignment::database: Pending blocks table created/verified
2025-08-27T07:22:45.848382Z  INFO ZamaAssignment::database: Pending events table created/verified
2025-08-27T07:22:45.848387Z  INFO ZamaAssignment::database: Creating optimized indexes...
2025-08-27T07:22:45.848408Z  INFO ZamaAssignment::database: Created index on (transaction_hash, log_index)
2025-08-27T07:22:45.848420Z  INFO ZamaAssignment::database: Created index on block_number
2025-08-27T07:22:45.848437Z  INFO ZamaAssignment::database: Created indexes on from_address and to_address
2025-08-27T07:22:45.848448Z  INFO ZamaAssignment::database: Created index on blocks.number
2025-08-27T07:22:45.848453Z  INFO ZamaAssignment::database: Database schema initialized successfully
2025-08-27T07:22:45.848463Z  INFO ZamaAssignment::database: Database initialization completed successfully
2025-08-27T07:22:45.848472Z  INFO ZamaAssignment::indexer: Initializing Adaptive ethers-rs Ethereum client
2025-08-27T07:22:45.848676Z  INFO ZamaAssignment::indexer: Connected to ERC-20 contract: ERC20 Token (0xdac17f958d2ee523a2206206994597c13d831ec7) with 18 decimals
2025-08-27T07:22:45.848695Z  INFO ZamaAssignment::indexer: Database write queue created
2025-08-27T07:22:45.848701Z  INFO ZamaAssignment::indexer: Starting indexer with controlled handoff
2025-08-27T07:22:45.848723Z  INFO ZamaAssignment::database: Database write queue started
2025-08-27T07:22:48.247348Z  INFO ZamaAssignment::indexer: Head=23230936, Safe Tip=23230931, Finality Depth=5 (Polling mode)
2025-08-27T07:22:48.247369Z  INFO ZamaAssignment::database: Got database connection for progress query
2025-08-27T07:22:48.247439Z  INFO ZamaAssignment::database: Progress table has 1 rows
2025-08-27T07:22:48.247471Z  INFO ZamaAssignment::database: Prepared progress query statement
2025-08-27T07:22:48.247495Z  INFO ZamaAssignment::database: Successfully queried progress: block=0, events=0
2025-08-27T07:22:48.247506Z  INFO ZamaAssignment::indexer:  progress.last_processed_block = 0
2025-08-27T07:22:48.247512Z  INFO ZamaAssignment::indexer:  self.config.start_block = 23222692
2025-08-27T07:22:48.247545Z  INFO ZamaAssignment::indexer: safe_tip = 23230931
2025-08-27T07:22:48.247552Z  INFO ZamaAssignment::indexer: Using start block 23222692
2025-08-27T07:22:48.247557Z  INFO ZamaAssignment::indexer: Starting from block 23222692, targeting block 23224692
2025-08-27T07:22:48.247563Z  INFO ZamaAssignment::indexer: Running historical indexing from 23222692 to 23224692
2025-08-27T07:22:48.247568Z  INFO ZamaAssignment::indexer: Starting controlled handoff from 23222692 to 23224692
2025-08-27T07:22:48.247574Z  INFO ZamaAssignment::indexer: Running historical indexing to safe tip 23224692
2025-08-27T07:22:48.247596Z  INFO ZamaAssignment::indexer: Starting historical indexing with moving target from 23222692 to 23224692
2025-08-27T07:22:49.067701Z  INFO ZamaAssignment::indexer: Processing batch 23222692-23224692 (target: 23224692)
2025-08-27T07:22:49.067747Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23222692 to 23223191 for transfer events (500 block limit)
2025-08-27T07:22:49.067765Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23223192 to 23223691 for transfer events (500 block limit)
2025-08-27T07:22:49.067790Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23223692 to 23224191 for transfer events (500 block limit)
2025-08-27T07:22:49.067800Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23224692 to 23224692 for transfer events (500 block limit)
2025-08-27T07:22:49.067808Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23224192 to 23224691 for transfer events (500 block limit)
2025-08-27T07:22:49.680566Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 46 transfer logs from blocks 23224692 to 23224692
2025-08-27T07:22:49.680619Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 46
2025-08-27T07:22:49.681264Z  INFO ZamaAssignment::indexer: Batch 23224692-23224692 processed 46 events
2025-08-27T07:22:49.681726Z  INFO ZamaAssignment::database: Bulk inserted 46 transfers using prepared statement
2025-08-27T07:22:49.681735Z  INFO ZamaAssignment::database: Database write queue: Stored 46 transfers
2025-08-27T07:22:50.092068Z  INFO ZamaAssignment::indexer: Stored 1 boundary block headers for range 23224692-23224692
2025-08-27T07:23:09.095498Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 27598 transfer logs from blocks 23223692 to 23224191
2025-08-27T07:23:09.108301Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 27598
2025-08-27T07:23:09.514872Z  INFO ZamaAssignment::indexer: Batch 23223692-23224191 processed 27598 events
2025-08-27T07:23:09.971452Z  INFO ZamaAssignment::database: Bulk inserted 27598 transfers using prepared statement
2025-08-27T07:23:09.971475Z  INFO ZamaAssignment::database: Database write queue: Stored 27598 transfers
2025-08-27T07:23:10.237678Z  INFO ZamaAssignment::indexer: Stored 2 boundary block headers for range 23223692-23224191
2025-08-27T07:23:11.927007Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 26364 transfer logs from blocks 23222692 to 23223191
2025-08-27T07:23:11.937144Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 26364
2025-08-27T07:23:12.326488Z  INFO ZamaAssignment::indexer: Batch 23222692-23223191 processed 26364 events
2025-08-27T07:23:12.424830Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 33219 transfer logs from blocks 23223192 to 23223691
2025-08-27T07:23:12.443456Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 33219
2025-08-27T07:23:12.859881Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 30219 transfer logs from blocks 23224192 to 23224691
2025-08-27T07:23:12.876312Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 30219
2025-08-27T07:23:12.909500Z  INFO ZamaAssignment::database: Bulk inserted 26364 transfers using prepared statement
2025-08-27T07:23:12.909518Z  INFO ZamaAssignment::database: Database write queue: Stored 26364 transfers
2025-08-27T07:23:13.081223Z  INFO ZamaAssignment::indexer: Batch 23223192-23223691 processed 33219 events
2025-08-27T07:23:13.412760Z  INFO ZamaAssignment::indexer: Batch 23224192-23224691 processed 30219 events
2025-08-27T07:23:13.662730Z  INFO ZamaAssignment::indexer: Stored 2 boundary block headers for range 23222692-23223191
2025-08-27T07:23:13.717760Z  INFO ZamaAssignment::database: Bulk inserted 33219 transfers using prepared statement
2025-08-27T07:23:13.717779Z  INFO ZamaAssignment::database: Database write queue: Stored 33219 transfers
2025-08-27T07:23:14.387304Z  INFO ZamaAssignment::indexer: Stored 2 boundary block headers for range 23223192-23223691
2025-08-27T07:23:14.460893Z  INFO ZamaAssignment::database: Bulk inserted 30219 transfers using prepared statement
2025-08-27T07:23:14.460913Z  INFO ZamaAssignment::database: Database write queue: Stored 30219 transfers
2025-08-27T07:23:14.875729Z  INFO ZamaAssignment::indexer: Stored 2 boundary block headers for range 23224192-23224691
2025-08-27T07:23:14.875772Z  INFO ZamaAssignment::indexer: === BATCH 1 PROFILING ===
2025-08-27T07:23:14.875781Z  INFO ZamaAssignment::indexer: Batch duration: 26.40s
2025-08-27T07:23:14.875793Z  INFO ZamaAssignment::indexer: Blocks processed: 2001
2025-08-27T07:23:14.875797Z  INFO ZamaAssignment::indexer: Events processed: 117446
2025-08-27T07:23:14.875801Z  INFO ZamaAssignment::indexer: Events per second: 4448.86
2025-08-27T07:23:14.875806Z  INFO ZamaAssignment::indexer: Blocks per second: 75.80
2025-08-27T07:23:14.875810Z  INFO ZamaAssignment::indexer: Total events: 117446
2025-08-27T07:23:14.875813Z  INFO ZamaAssignment::indexer: ================================================
2025-08-27T07:23:15.375367Z  INFO ZamaAssignment::indexer: Reached target! Current block 23224693 > latest block 23224692
2025-08-27T07:23:15.375383Z  INFO ZamaAssignment::indexer: Historical indexing completed at block 23224692. Total events: 117446
2025-08-27T07:23:15.375391Z  INFO ZamaAssignment::indexer: Historical indexing completed, no live mode requested
2025-08-27T07:23:15.375418Z  INFO ZamaAssignment::database: Database write queue stopped
```

## Transition from Historical to Live Indexing Using WebSocket Mode

**Command:**
```bash
./target/release/ZamaAssignment --adpetherrs --start-block 23222692 --finality-depth 5 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --websocket
```

**Log Output:**
```
2025-08-27T07:23:58.613468Z  INFO ZamaAssignment: Starting Ethereum Log Indexer and Verifier
2025-08-27T07:23:58.613487Z  INFO ZamaAssignment: Using library: Adaptive ethers-rs
2025-08-27T07:23:58.613501Z  INFO ZamaAssignment: Starting Standard Indexer
2025-08-27T07:23:58.613511Z  INFO ZamaAssignment::database: Opening database at "ethereum_logs.db"
2025-08-27T07:23:58.614214Z  INFO ZamaAssignment::database: Connection pool created with 30 max connections
2025-08-27T07:23:58.614236Z  INFO ZamaAssignment::database: Starting database schema initialization
2025-08-27T07:23:58.614770Z  INFO ZamaAssignment::database: SQLite journal mode: wal
2025-08-27T07:23:58.614788Z  INFO ZamaAssignment::database: Database connection test successful: 1
2025-08-27T07:23:58.614797Z  INFO ZamaAssignment::database: Setting page size...
2025-08-27T07:23:58.614810Z  INFO ZamaAssignment::database: SQLite page size: 4096
2025-08-27T07:23:58.614818Z  INFO ZamaAssignment::database: VACUUMing database after page size change...
2025-08-27T07:23:59.317173Z  INFO ZamaAssignment::database: VACUUM completed
2025-08-27T07:23:59.317190Z  INFO ZamaAssignment::database: Setting cache size...
2025-08-27T07:23:59.317365Z  INFO ZamaAssignment::database: SQLite cache size: 16384 KB
2025-08-27T07:23:59.317373Z  INFO ZamaAssignment::database: Skipping mmap_size PRAGMA for compatibility
2025-08-27T07:23:59.317376Z  INFO ZamaAssignment::database: Setting synchronous mode...
2025-08-27T07:23:59.317385Z  INFO ZamaAssignment::database: SQLite synchronous mode: 1
2025-08-27T07:23:59.317389Z  INFO ZamaAssignment::database: Setting temp store...
2025-08-27T07:23:59.317395Z  INFO ZamaAssignment::database: SQLite temp store: 2
2025-08-27T07:23:59.317399Z  INFO ZamaAssignment::database: Setting foreign keys...
2025-08-27T07:23:59.317407Z  INFO ZamaAssignment::database: SQLite foreign keys: 0
2025-08-27T07:23:59.317412Z  INFO ZamaAssignment::database: Skipping busy_timeout PRAGMA for compatibility
2025-08-27T07:23:59.317430Z  INFO ZamaAssignment::database: Transfers table created/verified
2025-08-27T07:23:59.317445Z  INFO ZamaAssignment::database: Progress table created/verified
2025-08-27T07:23:59.317554Z  INFO ZamaAssignment::database: Initial progress record inserted/verified
2025-08-27T07:23:59.317572Z  INFO ZamaAssignment::database: Blocks table created/verified
2025-08-27T07:23:59.317584Z  INFO ZamaAssignment::database: Pending blocks table created/verified
2025-08-27T07:23:59.317602Z  INFO ZamaAssignment::database: Pending events table created/verified
2025-08-27T07:23:59.317607Z  INFO ZamaAssignment::database: Creating optimized indexes...
2025-08-27T07:23:59.317619Z  INFO ZamaAssignment::database: Created index on (transaction_hash, log_index)
2025-08-27T07:23:59.317630Z  INFO ZamaAssignment::database: Created index on block_number
2025-08-27T07:23:59.317646Z  INFO ZamaAssignment::database: Created indexes on from_address and to_address
2025-08-27T07:23:59.317657Z  INFO ZamaAssignment::database: Created index on blocks.number
2025-08-27T07:23:59.317662Z  INFO ZamaAssignment::database: Database schema initialized successfully
2025-08-27T07:23:59.317666Z  INFO ZamaAssignment::database: Database initialization completed successfully
2025-08-27T07:23:59.317673Z  INFO ZamaAssignment::indexer: Initializing Adaptive ethers-rs Ethereum client
2025-08-27T07:23:59.317860Z  INFO ZamaAssignment::adaptive_ethers: Attempting WebSocket connection to: wss://eth-mainnet.g.alchemy.com/v2/API_KEY
2025-08-27T07:24:00.952460Z  INFO ZamaAssignment::adaptive_ethers: WebSocket connection established successfully
2025-08-27T07:24:00.952499Z  INFO ZamaAssignment::indexer: Connected to ERC-20 contract: ERC20 Token (0xdac17f958d2ee523a2206206994597c13d831ec7) with 18 decimals
2025-08-27T07:24:00.952549Z  INFO ZamaAssignment::indexer: Database write queue created
2025-08-27T07:24:00.952561Z  INFO ZamaAssignment::indexer: Starting indexer with controlled handoff
2025-08-27T07:24:00.952683Z  INFO ZamaAssignment::database: Database write queue started
2025-08-27T07:24:01.770610Z  INFO ZamaAssignment::indexer: Head=23230943, Safe Tip=23230938, Finality Depth=5 (WebSocket mode)
2025-08-27T07:24:01.770627Z  INFO ZamaAssignment::database: Got database connection for progress query
2025-08-27T07:24:01.770681Z  INFO ZamaAssignment::database: Progress table has 1 rows
2025-08-27T07:24:01.770703Z  INFO ZamaAssignment::database: Prepared progress query statement
2025-08-27T07:24:01.770715Z  INFO ZamaAssignment::database: Successfully queried progress: block=23224692, events=117446
2025-08-27T07:24:01.770721Z  INFO ZamaAssignment::indexer:  progress.last_processed_block = 23224692
2025-08-27T07:24:01.770727Z  INFO ZamaAssignment::indexer:  self.config.start_block = 23222692
2025-08-27T07:24:01.770733Z  INFO ZamaAssignment::indexer: safe_tip = 23230938
2025-08-27T07:24:01.770739Z  INFO ZamaAssignment::indexer: Resuming from block 23224692
2025-08-27T07:24:01.770745Z  INFO ZamaAssignment::indexer: Starting from block 23224693, targeting block 23230938
2025-08-27T07:24:01.770752Z  INFO ZamaAssignment::indexer: Running historical indexing from 23224693 to 23230938
2025-08-27T07:24:01.770758Z  INFO ZamaAssignment::indexer: Starting controlled handoff from 23224693 to 23230938
2025-08-27T07:24:01.770764Z  INFO ZamaAssignment::indexer: Running historical indexing to safe tip 23230938
2025-08-27T07:24:01.770779Z  INFO ZamaAssignment::indexer: Starting historical indexing with moving target from 23224693 to 23230938
2025-08-27T07:24:03.000741Z  INFO ZamaAssignment::indexer: Processing batch 23224693-23230938 (target: 23230938)
2025-08-27T07:24:03.000781Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23224693 to 23225192 for transfer events (500 block limit)
2025-08-27T07:24:03.000790Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23225193 to 23225692 for transfer events (500 block limit)
2025-08-27T07:24:03.000832Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23225693 to 23226192 for transfer events (500 block limit)
2025-08-27T07:24:03.000819Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23226193 to 23226692 for transfer events (500 block limit)
2025-08-27T07:24:03.000850Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23226693 to 23227192 for transfer events (500 block limit)
2025-08-27T07:24:03.000848Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23227193 to 23227692 for transfer events (500 block limit)
2025-08-27T07:24:03.000905Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23228193 to 23228692 for transfer events (500 block limit)
2025-08-27T07:24:03.000889Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23228693 to 23229192 for transfer events (500 block limit)
2025-08-27T07:24:03.000931Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23229193 to 23229692 for transfer events (500 block limit)
2025-08-27T07:24:03.000949Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23230193 to 23230692 for transfer events (500 block limit)
2025-08-27T07:24:03.000888Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23227693 to 23228192 for transfer events (500 block limit)
2025-08-27T07:24:03.000923Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23229693 to 23230192 for transfer events (500 block limit)
2025-08-27T07:24:03.000935Z  INFO ZamaAssignment::adaptive_ethers: Scanning blocks 23230693 to 23230938 for transfer events (500 block limit)
2025-08-27T07:24:36.321940Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 21926 transfer logs from blocks 23226693 to 23227192
2025-08-27T07:24:36.329680Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 21926
2025-08-27T07:24:36.630039Z  INFO ZamaAssignment::indexer: Batch 23226693-23227192 processed 21926 events
2025-08-27T07:24:37.371554Z  INFO ZamaAssignment::database: Bulk inserted 21926 transfers using prepared statement
2025-08-27T07:24:37.371575Z  INFO ZamaAssignment::database: Database write queue: Stored 21926 transfers
2025-08-27T07:24:38.072712Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 22333 transfer logs from blocks 23227193 to 23227692
2025-08-27T07:24:38.077955Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 22333
2025-08-27T07:24:38.375364Z  INFO ZamaAssignment::indexer: Batch 23227193-23227692 processed 22333 events
2025-08-27T07:24:39.164695Z  INFO ZamaAssignment::database: Bulk inserted 22333 transfers using prepared statement
2025-08-27T07:24:39.164709Z  INFO ZamaAssignment::database: Database write queue: Stored 22333 transfers
2025-08-27T07:24:39.894400Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 24595 transfer logs from blocks 23226193 to 23226692
2025-08-27T07:24:39.900959Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 24595
2025-08-27T07:24:40.204192Z  INFO ZamaAssignment::indexer: Batch 23226193-23226692 processed 24595 events
2025-08-27T07:24:41.175769Z  INFO ZamaAssignment::database: Bulk inserted 24595 transfers using prepared statement
2025-08-27T07:24:41.175786Z  INFO ZamaAssignment::database: Database write queue: Stored 24595 transfers
2025-08-27T07:24:41.487770Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 25814 transfer logs from blocks 23225693 to 23226192
2025-08-27T07:24:41.497641Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 25814
2025-08-27T07:24:41.873142Z  INFO ZamaAssignment::indexer: Batch 23225693-23226192 processed 25814 events
2025-08-27T07:24:43.012801Z  INFO ZamaAssignment::database: Bulk inserted 25814 transfers using prepared statement
2025-08-27T07:24:43.012820Z  INFO ZamaAssignment::database: Database write queue: Stored 25814 transfers
2025-08-27T07:24:43.923559Z  INFO ZamaAssignment::adaptive_ethers: Retrieved 27539 transfer logs from blocks 23225193 to 23225692
2025-08-27T07:24:43.935681Z  INFO ZamaAssignment::adaptive_ethers: Total transfer events found: 27539
```

## Key Log Patterns to Understand

### **1. Database Initialization**
- Database connection and schema setup
- SQLite configuration (page size, cache, etc.)
- Table creation and index optimization

### **2. Client Initialization**
- Ethereum client setup
- Contract connection verification
- WebSocket connection establishment

### **3. Progress Tracking**
- Current block vs target block
- Safe tip calculation
- Resume from last processed block

### **4. Batch Processing**
- Block range scanning
- Transfer event retrieval
- Database bulk insertions
- Performance profiling

### **5. Mode Indicators**
- `(Polling mode)` - Historical indexing
- `(WebSocket mode)` - Real-time indexing
- Finality depth and safe tip calculations

## Notes

- **API_KEY**: Replace `API_KEY` with your actual Alchemy API key
- **Contract Address**: USDT token contract (0xdac17f958d2ee523a2206206994597c13d831ec7)
- **Performance**: Logs show ~4,448 events/second processing rate
- **Database**: SQLite with WAL mode and optimized indexes
- **Batch Size**: 300 blocks per batch for optimal performance in free tier can go upto 500
