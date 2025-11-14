use std::error::Error;
use std::result::Result as StdResult;
use thiserror::Error as ThisError;

pub type Result<T> = StdResult<T, DomainError>;

#[derive(ThisError, Debug)]
pub enum DomainError {
    #[error("invalid configuration: {message}")]
    Configuration { message: String },

    #[error("invalid dimensions {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("invalid input: {message}")]
    InvalidInput { message: String },

    #[error("calculation error: {operation} failed - {reason}")]
    Calculation { operation: String, reason: String },

    #[error("adapter error")]
    Adapter {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}
