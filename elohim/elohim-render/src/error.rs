use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("module load failed: {0}")]
    ModuleLoad(String),

    #[error("render timed out after {limit_ms}ms")]
    Timeout { limit_ms: u64 },

    #[error("isolate out of memory (limit: {limit_mb}MB)")]
    OutOfMemory { limit_mb: u64 },

    #[error("data fetch failed: {0}")]
    DataFetch(String),

    #[error("renderer panicked: {0}")]
    Panic(String),

    #[error("unsupported render spec: {0}")]
    UnsupportedSpec(String),

    #[error("render output exceeded limit of {limit_bytes} bytes (actual: {actual_bytes} bytes)")]
    OutputTooLarge {
        limit_bytes: usize,
        actual_bytes: usize,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RenderError>;
