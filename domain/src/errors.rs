use std::result::Result as StdResult;
use thiserror::Error;

pub type Result<T> = StdResult<T, DomainError>;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Calculation error: {0}")]
    Calculation(String),
}
