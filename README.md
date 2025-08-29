# Ethereum Indexer for ERC-20 Transfer Events

An Ethereum event indexing system designed to track ERC-20 Transfer events efficiently and reliably. Supports historical backfill, seamless transition to live indexing, and robust protection against blockchain reorganizations.

## 🚀 Features

- **Multi-Client Support**: ethers-rs and Alloy clients
- **Dual Indexing Modes**: WebSocket and HTTP polling
- **Historical Backfill**: Efficient batch processing (280-320 blocks/sec)
- **Live Indexing**: Real-time event processing with finality protection
- **Reorg Recovery**: Two-phase detection (linear + binary search)
- **Gap Detection**: Automatic backfilling of missing blocks
- **Database Optimization**: SQLite with WAL mode and connection pooling
- **CLI Tools**: Database querying and verification utilities

## 📊 Performance

- **Historical Indexing**: 280-320 blocks/second
- **Event Processing**: ~4,448 events/second
- **Batch Size**: 300 blocks (configurable up to 500)
- **Parallel Processing**: 15 concurrent batches
- **Database**: Optimized SQLite with atomic transactions

## 🏗️ Architecture

The system consists of three major layers:

1. **Client Layer**: Multiple Ethereum client implementations
2. **Orchestration Layer**: Historical → Transition → Live indexing phases
3. **Database Layer**: Optimized SQLite with write queue pattern

## 🛠️ Quick Start

### Prerequisites

- Rust 1.70+ 
- SQLite3
- Alchemy API key (or other Ethereum RPC provider)

### Installation

```bash
# Clone the repository
git clone https://github.com/krrakash/Ethereum-Indexer.git
cd Ethereum-Indexer

# Build the project
cargo build --release

# Build CLI tools
cargo build --release --bin db
cargo build --release --bin verify
```

### Basic Usage

```bash
# Historical indexing
./target/release/ZamaAssignment --adpetherrs \
  --start-block 15000000 --end-block 15150000 \
  --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" \
  --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7"

# Live indexing with WebSocket
./target/release/ZamaAssignment --adpetherrs --websocket \
  --start-block 23222692 \
  --rpc-url "https://eth-mainnet.g.alchemy.com/v2/API_KEY" \
  --contract-address "0xdac17f958d2ee523a2206206994597c13d831ec7"
```

**📖 For detailed usage instructions, configuration examples, and troubleshooting, see the [Quickstart Guide](docs/quickstart_guide.md)**

## 🗄️ CLI Tools

```bash
# Database query tool
./target/release/db --database your_db.db stats
./target/release/db --database your_db.db recent --count 10

# Database verification tool  
./target/release/verify --database your_db.db gaps
./target/release/verify --database your_db.db duplicates
```

**📖 For complete CLI tool documentation and examples, see the [Quickstart Guide](docs/quickstart_guide.md)**

## 📁 Project Structure

```
ZamaAssignment/
├── src/                    # Source code
│   ├── main.rs            # Application entry point
│   ├── indexer.rs         # Core indexing logic
│   ├── database.rs        # Database operations
│   ├── adaptive_ethers.rs # Adaptive ethers client
│   ├── alloy_client.rs    # Alloy client
│   └── bin/               # CLI tools
│       ├── db.rs          # Database query tool
│       └── verify.rs      # Database verification tool
├── docs/                  # Detailed documentation
│   ├── quickstart_guide.md    # Complete usage guide
│   ├── PROJECT_STRUCTURE.md   # Code structure & functions
│   └── log_example.md         # Log examples & patterns
└── Cargo.toml             # Project configuration
```

## 📚 Documentation

**Detailed documentation is available in the `docs/` folder:**

- **[📖 Quickstart Guide](docs/quickstart_guide.md)**: Complete usage instructions, configuration examples, and troubleshooting
- **[🏗️ Project Structure](docs/PROJECT_STRUCTURE.md)**: Detailed code architecture, function documentation, and file descriptions
- **[📋 Log Examples](docs/log_example.md)**: Sample logs and performance patterns

## 🔧 Configuration

**📖 For complete configuration options, parameter descriptions, and advanced usage, see the [Quickstart Guide](docs/quickstart_guide.md)**

## 🛡️ Key Features

### Reorg Protection
- **Linear backtracking**: Fast recovery for shallow reorgs
- **Binary search**: Efficient deep reorg recovery
- **Continuity verification**: Hash and parent hash validation
- **Safe rollback**: Atomic database operations

### Performance Optimization
- **Parallel batch processing**: 15 concurrent tasks
- **Write queue pattern**: Eliminates database contention
- **Prepared statements**: Optimized database operations
- **Connection pooling**: Efficient resource management

### Reliability Features
- **Gap detection**: Automatic backfilling
- **Finality depth**: Protection against unfinalized blocks
- **Error recovery**: Retry mechanisms with exponential backoff
- **Atomic transactions**: Database consistency guarantees

## 🆘 Support

For detailed usage instructions, troubleshooting, and advanced configuration, please refer to the documentation in the `docs/` folder:

- **[Quickstart Guide](docs/quickstart_guide.md)** - Complete setup and usage
- **[Project Structure](docs/PROJECT_STRUCTURE.md)** - Code architecture and functions
- **[Log Examples](docs/log_example.md)** - Performance patterns and debugging

## 🏆 Performance Benchmarks

- **Historical Indexing**: 280-320 blocks/second (500 batch size)
- **Event Processing**: ~4,448 events/second

---

**For comprehensive documentation and advanced usage, please visit the `docs/` folder.**
