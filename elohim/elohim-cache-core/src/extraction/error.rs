//! Cache error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Budget exceeded: limit {limit} bytes, requested {requested} bytes")]
    BudgetExceeded { limit: u64, requested: u64 },

    #[error("Invalid cache key: {0}")]
    InvalidKey(String),

    #[error("Cache backend unavailable")]
    BackendUnavailable,
}
