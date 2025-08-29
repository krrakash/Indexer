use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    

    
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] r2d2::Error),
    
    #[error("Ethereum error: {0}")]
    Ethereum(String),
    
    #[error("Provider error: {0}")]
    Provider(#[from] ethers::providers::ProviderError),
    

    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

pub type IndexerResult<T> = Result<T, IndexerError>; 