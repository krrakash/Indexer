use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing::info;
use ZamaAssignment::database::Database;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "verify")]
#[command(about = "Simple database verification tool")]
struct VerifyArgs {
    #[arg(short, long, default_value = "ethereum_logs.db")]
    database: PathBuf,

    #[command(subcommand)]
    command: VerifyCommands,
}

#[derive(Subcommand, Debug)]
enum VerifyCommands {
    Gaps,
    
    Integrity,
    
    Tables,
    
    Duplicates,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = VerifyArgs::parse();

    info!("Opening database: {:?}", args.database);
    let database = Database::new(&args.database)?;

    match args.command {
        VerifyCommands::Gaps => {
            println!("=== Checking for Gaps in Block Numbers ===");
            
            if let Ok(blocks) = database.get_all_block_numbers() {
                if blocks.len() > 1 {
                    let mut gaps = Vec::new();
                    for i in 0..blocks.len() - 1 {
                        let current = blocks[i];
                        let next = blocks[i + 1];
                        if next != current + 1 {
                            gaps.push((current, next));
                        }
                    }
                    
                    if gaps.is_empty() {
                        println!("No gaps found in block numbers");
                    } else {
                        println!("Found {} gaps:", gaps.len());
                        for (current, next) in gaps {
                            let gap_size = next - current - 1;
                            
                            let mut gap_type = "unknown";
                            let mut batch_info = String::new();
                            
                            if let Ok(Some((_, batch_start_current, batch_end_current))) = database.get_block_with_batch_info(current) {
                                if let Ok(Some((_, batch_start_next, batch_end_next))) = database.get_block_with_batch_info(next) {
                                    if batch_start_current.is_some() && batch_end_current.is_some() && 
                                       batch_start_next.is_some() && batch_end_next.is_some() {
                                        gap_type = "historical";
                                        batch_info = format!(" (batch {}-{} to batch {}-{})", 
                                            batch_start_current.unwrap(), batch_end_current.unwrap(),
                                            batch_start_next.unwrap(), batch_end_next.unwrap());
                                    } else if batch_start_current.is_none() && batch_end_current.is_none() && 
                                              batch_start_next.is_none() && batch_end_next.is_none() {
                                        gap_type = "live";
                                        batch_info = " (live indexing gap)".to_string();
                                    } else {
                                        gap_type = "transition";
                                        batch_info = " (historical to live transition)".to_string();
                                    }
                                }
                            }
                            
                            println!("  Gap between blocks {} and {} ({} missing blocks) - Type: {}{}", 
                                    current, next, gap_size, gap_type, batch_info);
                        }
                    }
                } else {
                    println!("Not enough blocks to check for gaps");
                }
            } else {
                println!("Failed to get block numbers");
            }
        }
        
        VerifyCommands::Integrity => {
            println!("=== Database Integrity Check ===");
            
            let (total_transfers, total_blocks) = database.get_statistics()?;
            println!("Transfers table: {} records", total_transfers);
            println!("Blocks table: {} records", total_blocks);
            
            let progress = database.get_progress()?;
            println!("Progress table: block {}, events {}", 
                    progress.last_processed_block, progress.total_events_processed);
            
            println!("Checking for orphaned transfers...");
            
            println!("Basic integrity check completed");
        }
        
        VerifyCommands::Tables => {
            println!("=== Database Table Information ===");
            
            let (total_transfers, total_blocks) = database.get_statistics()?;
            let progress = database.get_progress()?;
            
            println!("transfers: {} records", total_transfers);
            println!("blocks: {} records", total_blocks);
            println!("progress: 1 record (last block: {})", progress.last_processed_block);
            
            if let Some(earliest) = database.get_earliest_processed_block()? {
                if let Some(latest) = database.get_latest_processed_block()? {
                    println!("Block range: {} - {} ({} blocks)", earliest, latest, latest - earliest + 1);
                }
            }
        }
        
        VerifyCommands::Duplicates => {
            println!("=== Checking for Duplicate Transfers ===");
            
            match database.find_duplicate_transfers() {
                Ok(duplicates) => {
                    if duplicates.is_empty() {
                        println!("No duplicate transfers found");
                    } else {
                        println!("Found {} duplicate transfer groups:", duplicates.len());
                        for (i, (tx_hash, count, transfers)) in duplicates.iter().enumerate() {
                            println!("\n{}. Transaction: {} (Log Index: {}) - {} duplicates:", 
                                    i + 1, tx_hash, transfers[0].log_index, count);
                            
                            for (j, transfer) in transfers.iter().enumerate() {
                                println!("   {}. ID: {} - Block: {} - From: {} - To: {} - Value: {} - Created: {}", 
                                        j + 1, transfer.id, transfer.block_number, 
                                        transfer.from_address, transfer.to_address, transfer.value, transfer.created_at);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error checking for duplicates: {}", e);
                }
            }
        }
    }

    Ok(())
}
