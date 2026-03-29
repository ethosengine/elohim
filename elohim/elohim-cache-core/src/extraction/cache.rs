//! ExtractionCache — budget-aware cache with pluggable backend

/// Configuration for the extraction cache.
pub struct ExtractionCacheConfig {
    /// Maximum total cache size in bytes.
    pub max_bytes: u64,
}

/// Metadata for a cached app extraction.
pub struct AppCacheEntry {
    /// App identifier (e.g., EPR CID).
    pub app_id: String,
    /// Total size in bytes of all cached files for this app.
    pub total_bytes: u64,
}

/// Statistics snapshot for the extraction cache.
pub struct ExtractionCacheStats {
    /// Number of cached apps.
    pub app_count: u32,
    /// Total bytes used.
    pub total_bytes: u64,
    /// Budget limit in bytes.
    pub budget_bytes: u64,
}

/// Budget-aware extraction cache with pluggable backend.
pub struct ExtractionCache;
