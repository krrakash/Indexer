use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing::info;
use ZamaAssignment::database::Database;
use ZamaAssignment::models::TransferRecord;
use std::path::PathBuf;
use hex;

#[derive(Parser, Debug)]
#[command(name = "db")]
#[command(about = "Simple database query tool for Ethereum transfer events")]
struct DbArgs {
    #[arg(short, long, default_value = "ethereum_logs.db")]
    database: PathBuf,

    #[command(subcommand)]
    command: DbCommands,
}

#[derive(Subcommand, Debug)]
enum DbCommands {
    Stats,
    
    Recent {
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    
    Block {
        number: u64,
    },
    
    Address {
        address: String,
        
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    
    Tx {
        hash: String,
    },
    
    Progress,
    
    Blocks,
}

fn format_transfer(transfer: &TransferRecord) -> String {
    format!(
        "Block {} | {} -> {} | Value: {} | Tx: {}",
        transfer.block_number,
        transfer.from_address,
        transfer.to_address,
        transfer.value,
        transfer.transaction_hash
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = DbArgs::parse();

    info!("Opening database: {:?}", args.database);
    let database = Database::new(&args.database)?;

    match args.command {
        DbCommands::Stats => {
            let (total_transfers, total_blocks) = database.get_statistics()?;
            let progress = database.get_progress()?;
            
            println!("=== Database Statistics ===");
            println!("Total transfers: {}", total_transfers);
            println!("Total blocks: {}", total_blocks);
            println!("Last processed block: {}", progress.last_processed_block);
            println!("Total events processed: {}", progress.total_events_processed);
            
            if let Some(earliest) = database.get_earliest_processed_block()? {
                println!("Earliest block: {}", earliest);
            }
            if let Some(latest) = database.get_latest_processed_block()? {
                println!("Latest block: {}", latest);
            }
        }
        
        DbCommands::Recent { count } => {
            let transfers = database.get_recent_transfers(count)?;
            println!("=== Recent {} Transfers ===", count);
            if transfers.is_empty() {
                println!("No transfers found.");
            } else {
                for (i, transfer) in transfers.iter().enumerate() {
                    println!("{}. {}", i + 1, format_transfer(transfer));
                }
            }
        }
        
        DbCommands::Block { number } => {
            let transfers = database.get_transfers_by_block(number)?;
            println!("=== Transfers in Block {} ===", number);
            if transfers.is_empty() {
                println!("No transfers found in block {}.", number);
            } else {
                println!("Found {} transfers:", transfers.len());
                for (i, transfer) in transfers.iter().enumerate() {
                    println!("{}. {}", i + 1, format_transfer(transfer));
                }
            }
        }
        
        DbCommands::Address { address, limit } => {
            let transfers = database.get_transfers_by_address(&address)?;
            println!("=== Transfers for Address {} ===", address);
            if transfers.is_empty() {
                println!("No transfers found for address {}.", address);
            } else {
                println!("Found {} transfers (showing first {}):", transfers.len(), limit);
                for (i, transfer) in transfers.iter().take(limit).enumerate() {
                    println!("{}. {}", i + 1, format_transfer(transfer));
                }
                if transfers.len() > limit {
                    println!("... and {} more", transfers.len() - limit);
                }
            }
        }
        
        DbCommands::Tx { hash } => {
            let transfers = database.get_transfers_by_transaction(&hash)?;
            println!("=== Transfers for Transaction {} ===", hash);
            if transfers.is_empty() {
                println!("No transfers found for transaction {}.", hash);
            } else {
                println!("Found {} transfers:", transfers.len());
                for (i, transfer) in transfers.iter().enumerate() {
                    println!("{}. {}", i + 1, format_transfer(transfer));
                }
            }
        }
        
        DbCommands::Progress => {
            let progress = database.get_progress()?;
            println!("=== Progress Information ===");
            println!("Last processed block: {}", progress.last_processed_block);
            println!("Total events processed: {}", progress.total_events_processed);
            println!("Last processed timestamp: {}", progress.last_processed_timestamp);
        }
        
        DbCommands::Blocks => {
            let blocks = database.get_all_block_numbers()?;
            println!("=== All Stored Blocks ===");
            println!("Total blocks: {}", blocks.len());
            
            for (i, block_number) in blocks.iter().enumerate() {
                if let Ok(Some((block_header, batch_start, batch_end))) = database.get_block_with_batch_info(*block_number) {
                    let indexing_type = if batch_start.is_some() && batch_end.is_some() {
                        format!("historical (batch {}-{})", batch_start.unwrap(), batch_end.unwrap())
                    } else {
                        "live".to_string()
                    };
                    
                    println!("{}. Block {} - Indexed: {} - Hash: 0x{} - Timestamp: {}", 
                            i + 1, block_number, indexing_type, hex::encode(block_header.hash), block_header.timestamp);
                } else {
                    println!("{}. Block {} - Indexed: unknown - No header data", i + 1, block_number);
                }
            }
        }
    }

    Ok(())
}
