use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("module load failed: {0}")]
    ModuleLoad(String),

    #[error("render timed out after {limit_ms}ms")]
    Timeout { limit_ms: u64 },

    /// The render queue is saturated — a render is already in flight (and one
    /// queued) on the single sequential isolate, so this request is shed rather
    /// than queued behind a possibly-stuck render. The caller should fall back
    /// fast (e.g. serve the CSR shell) instead of waiting. See the 2026-06-13
    /// doorway-freeze RCA: an unbounded queue let a stuck render back-pressure
    /// onto the runtime.
    #[error("render queue saturated (busy) — shedding to fast fallback")]
    Busy,

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

    #[error("bootstrap error: {0}")]
    Bootstrap(String),
}

pub type Result<T> = std::result::Result<T, RenderError>;
