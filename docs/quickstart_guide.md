# Ethereum Log Indexer Quickstart Guide

## CLI Tool Details

### Available Options

```bash
Usage: ZamaAssignment [OPTIONS] --contract-address <CONTRACT_ADDRESS>

Options:
  -r, --rpc-url <RPC_URL>                    [default: http://localhost:8545]
  -c, --contract-address <CONTRACT_ADDRESS>  
  -s, --start-block <START_BLOCK>            [default: 0]
  -e, --end-block <END_BLOCK>                [default: 0]
  -d, --database <DATABASE>                  [default: ethereum_logs.db]
  -w, --websocket                            
      --alloy                                
      --adpetherrs                           
      --finality-depth <FINALITY_DEPTH>      [default: 12]
      --batch-size <BATCH_SIZE>              [default: 300]
  -h, --help                                 Print help
  -V, --version                              Print version
```

### Parameter Descriptions

- `--rpc-url`: Ethereum RPC endpoint URL (default: http://localhost:8545)
- `--contract-address`: Target smart contract address to index events from
- `--start-block`: Starting block number for historical indexing (default: 0)
- `--end-block`: Ending block number for historical indexing (default: 0)
- `--database`: SQLite database file path (default: ethereum_logs.db)
- `--websocket`: Enable WebSocket connection for real-time indexing
- `--alloy`: Use Alloy Ethereum client implementation
- `--adpetherrs`: Use Adaptive Ethers client implementation
- `--finality-depth`: Number of confirmations required for finality (default: 12)
- `--batch-size`: Number of blocks to process in each batch (default: 300)

## Historical Indexing

### Adaptive Ethers Client

```bash
./target/release/ZamaAssignment --adpetherrs --start-block 15000000 --end-block 15150000 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" --contract-address --adpetherrs "0xdac17f958d2ee523a2206206994597c13d831ec7" --database "historical_indexing.db"
```

**Command Breakdown:**
- **Client**: `--adpetherrs` (Adaptive Ethers implementation)
- **Block Range**: 15,000,000 to 15,150,000 (150,000 blocks)
- **RPC Endpoint**: Alchemy Ethereum mainnet
- **Contract**: USDT token contract (0xdac17f958d2ee523a2206206994597c13d831ec7)
- **Database**: historical_indexing.db

**Features Used:**
- Historical block processing
- Transfer event extraction
- Gap detection and backfill
- SQLite database storage

### Alloy Client

```bash
./target/release/ZamaAssignment --alloy --start-block 15000000 --end-block 15000010 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --database "alloy_historical_to_live.db"
```

**Command Breakdown:**
- **Client**: `--alloy` (Alloy implementation)
- **Block Range**: 15,000,000 to 15,000,010 (10 blocks)
- **RPC Endpoint**: Alchemy Ethereum mainnet
- **Contract**: USDT token contract (0xdac17f958d2ee523a2206206994597c13d831ec7)
- **Database**: alloy_historical_to_live.db

**Features Used:**
- Historical block processing
- Transfer event extraction
- Gap detection and backfill
- SQLite database storage

**Note**: Alloy client does not support WebSocket mode for real-time indexing.

## Transition to Real-Time Indexing

### WebSocket Mode - Historical to Real-Time Transition

```bash
./target/release/ZamaAssignment --adpetherrs --websocket --start-block 23244858 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/xU-dLtnVQ3YRW0Xafl5lE" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --database "transition_test.db" --batch-size 100
```

**Command Breakdown:**
- **Client**: `--adpetherrs` (Adaptive Ethers implementation)
- **WebSocket**: `--websocket` (enables real-time indexing)
- **Start Block**: 23,244,858 (resume from specific block)
- **RPC Endpoint**: Alchemy Ethereum mainnet
- **Contract**: USDT token contract (0xdac17f958d2ee523a2206206994597c13d831ec7)
- **Database**: transition_test.db
- **Batch Size**: 100 blocks per batch (optimized for performance)

**Features Used:**
- Historical indexing from start block to current safe tip
- WebSocket connection for real-time block processing
- Gap detection and automatic backfill
- Reorg detection and recovery
- Optimized batch processing
- SQLite database storage with write queue

**Workflow:**
1. **Historical Indexing**: Processes blocks from start_block to current safe tip
2. **Gap Detection**: Identifies and fills any gaps between historical and real-time data
3. **WebSocket Mode**: Switches to real-time processing of new blocks
4. **Reorg Recovery**: Automatically handles blockchain reorganizations

### Polling Mode - Historical to Real-Time Transition

```bash
./target/release/ZamaAssignment --adpetherrs --start-block 23244858 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/xU-dLtnVQ3YRW0Xafl5lE" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --database "transition_test.db" --batch-size 300
```

**Command Breakdown:**
- **Client**: `--adpetherrs` (Adaptive Ethers implementation)
- **No WebSocket Flag**: Uses polling mode (default)
- **Start Block**: 23,244,858 (resume from specific block)
- **RPC Endpoint**: Alchemy Ethereum mainnet
- **Contract**: USDT token contract (0xdac17f958d2ee523a2206206994597c13d831ec7)
- **Database**: transition_test.db
- **Batch Size**: 300 blocks per batch (historical processing)

**Features Used:**
- Historical indexing from start block to current safe tip
- Polling-based real-time block processing (15-second intervals)
- Single block processing for real-time blocks
- Continuity verification and reorg recovery
- Finality depth protection (12 blocks)
- SQLite database storage with write queue

**Workflow:**
1. **Historical Indexing**: Processes blocks from start_block to current safe tip
2. **Transition**: Seamlessly switches to polling mode
3. **Polling Mode**: Checks for new blocks every 15 seconds
4. **Single Block Processing**: Processes one block at a time for real-time data
5. **Finality Safety**: Only processes blocks that are 12 blocks behind the head

**Polling Mode Advantages:**
- **Simpler Architecture**: No WebSocket connection management
- **Reliable**: Works with any RPC provider (no WebSocket requirement)
- **Resource Efficient**: Lower memory usage than WebSocket buffering
- **Stable**: Less prone to connection issues
- **Finality Safe**: Built-in protection against reorgs

## Troubleshooting

### Alchemy RPC Error: "Internal server error. Forwarder error: 1002"

If you encounter this error:
```
Provider error: Deserialization Error: invalid type: string "Internal server error. Forwarder error: 1002", expected struct JsonRpcError at line 1 column 71. Response: {"jsonrpc":"2.0","error":"Internal server error. Forwarder error: 1002"}
```

**What is this error?**
This is a transient forwarder error from Alchemy RPC indicating temporary server-side issues or rate limiting.

**Temporary Solution:**
Reduce the batch size from the default (300) to a smaller value (~100) to reduce server load:

```bash
./target/release/ZamaAssignment --adpetherrs --websocket --start-block 23244858 --rpc-url "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY" --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7" --database "transition_test.db" --batch-size 100
```

**Note:** The current implementation does not handle this error optimally. A proper solution would implement:
- **Retry logic** with exponential backoff
- **Adaptive chunk sizing** that reduces batch size on errors
- **Concurrency clamping** to limit simultaneous requests

**Why this happens:**
- Alchemy free tier has rate limits and computational constraints
- Large batch sizes can trigger server-side throttling
- Network instability or temporary server issues
- The error is not currently handled by retry logic

## Fencing and Restart Behavior

**Important Note**: If you close the indexer and restart it immediately, you may encounter **fence waiting** behavior.

### What is Fencing?
Fencing is a safety mechanism implemented that ensures the indexer doesn't process blocks that are too close to the current chain head (unfinalized blocks). The indexer waits for blocks to reach a "safe tip" before processing them.

### Fence Waiting on Restart
When you restart the indexer immediately after closing it:

1. **Progress Check**: The indexer reads the last processed block from the database
2. **Safe Tip Calculation**: Calculates the current safe tip (head - finality_depth)
3. **Fence Wait**: If your progress is ahead of the safe tip, it waits for the fence to "catch up"
4. **Resume Processing**: Once the fence catches up, normal processing resumes

### Example Scenario
```
Last processed block: 23246173
Current head: 23246174
Safe tip: 23246162 (head - 12 blocks)
Result: Fence waiting until safe tip >= 23246173
```

### Why This Happens
- **Finality Safety**: Ensures you're not processing potentially reverted blocks
- **Consistency**: Maintains data integrity during restarts
- **Chain Reorgs**: Protects against processing blocks that might be orphaned

### Best Practices
- **Wait ~5 minutes** between stopping and restarting the indexer
- **Monitor the logs** for fence waiting messages
- **Don't panic** - this is normal behavior for data safety

## Database Schema

The indexer creates the following tables:
- `progress`: Tracks indexing progress and last processed block
- `blocks`: Stores block headers and metadata
- `transfers`: Contains transfer events with parsed data


# Release build (recommended for production)
cargo build --release

# Check for compilation errors
cargo check


## CLI Tools - Experimental Phase ⚠️

 CLI tool for database management. **Note: This tools are currently in experimental phase.**

### Database Query Tool (`db`)

A comprehensive tool for querying transfer events and database statistics.

#### Basic Usage
```bash
# Build the tool
cargo build --release --bin db

# Use the tool
./target/release/db --database transition_test.db <command>
```

#### Available Commands

**Main Options:**
- `-d, --database <DATABASE>` - Specify database file (default: ethereum_logs.db)
- `-h, --help` - Show help information

**1. Get Database Statistics**
```bash
./target/release/db --database transition_test.db stats
```
*Output: Total transfers, blocks, progress information, block ranges*

**2. View Recent Transfers**
```bash
./target/release/db --database transition_test.db recent --count 20
```
*Output: Last N transfer events with details (block, addresses, value, tx hash)*

**3. Get Transfers by Block**
```bash
./target/release/db --database transition_test.db block 23246367
```
*Output: All transfers in the specified block*

**4. Search by Address**
```bash
./target/release/db --database transition_test.db address 0xdac17f958d2ee523a2206206994597c13d831ec7 --limit 10
```
*Output: All transfers involving the specified address*

**5. Get Transaction Details**
```bash
./target/release/db --database transition_test.db tx 0x4eebfcd9f9fb9574adb4d19c666c64e6f080904304e3c5616cbbc4324940839f
```
*Output: All transfers in the specified transaction*

**6. Check Progress**
```bash
./target/release/db --database transition_test.db progress
```
*Output: Current indexing progress and statistics*

**7. List All Blocks**
```bash
./target/release/db --database transition_test.db blocks
```
*Output: All indexed blocks with batch information*

