use clap::Parser;
use tracing::{info, error};
use anyhow::Result;

mod config;
mod database;
mod ethereum;
mod adaptive_ethers;
mod alloy_client;
mod client_trait;
mod indexer;

mod models;
mod error;

use config::Config;
use indexer::Indexer;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    
    #[arg(short, long, default_value = "http://localhost:8545")]
    rpc_url: String,

    
    #[arg(short, long)]
    contract_address: String,

   
    #[arg(short, long, default_value = "0")]
    start_block: u64,

   
    #[arg(short, long, default_value = "0")]
    end_block: u64,

   
    #[arg(short, long, default_value = "ethereum_logs.db")]
    database: String,

   
    #[arg(short, long)]
    websocket: bool,

    
    #[arg(long)]
    alloy: bool,

  
    #[arg(long)]
    adpetherrs: bool,
   
    #[arg(long, default_value = "12")]
    finality_depth: u64,

    #[arg(long, default_value = "300")]
    batch_size: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    

    let use_alloy = args.alloy;
    let use_adaptive_ethers = args.adpetherrs;
    
    
    if use_alloy && use_adaptive_ethers {
        return Err(anyhow::anyhow!("Cannot use both --alloy and --adpetherrs flags"));
    }
    
    
    if args.websocket && use_alloy {
        return Err(anyhow::anyhow!("WebSocket mode is not supported with --alloy flag"));
    }
    
   
    if args.end_block > 0 && args.websocket {
        return Err(anyhow::anyhow!("Cannot use --end-block with --websocket flag"));
    }
    
    
    if args.end_block > 0 && args.start_block >= args.end_block {
        return Err(anyhow::anyhow!("start_block must be less than end_block"));
    }
    

    
    let client_type = if use_alloy {
        "Alloy"
    } else if use_adaptive_ethers {
        "Adaptive ethers-rs"
    } else {
        "ethers-rs"
    };
    
    info!("Starting Ethereum Log Indexer and Verifier");
    info!("Using library: {}", client_type);
    
    let config = Config {
        rpc_url: args.rpc_url,
        contract_address: args.contract_address,
        start_block: args.start_block,
        end_block: args.end_block,
        database_path: args.database,
        websocket: args.websocket,
        use_alloy: args.alloy,
        use_adaptive_ethers: args.adpetherrs,


        finality_depth: args.finality_depth,
        overlap_size: 32,
        tail_http_batch_size: 50,
        batch_size: args.batch_size,
    };
    

    info!("Starting Standard Indexer");
    let mut indexer = Indexer::new(config).await?;
    if let Err(e) = indexer.start().await {
        error!("Indexer service failed: {}", e);
        return Err(anyhow::anyhow!("Indexer service failed: {}", e));
    }
    
    Ok(())
}
