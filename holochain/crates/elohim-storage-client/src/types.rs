//! Types for storage client API

use serde::{Deserialize, Serialize};

/// Client configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base URL for elohim-storage HTTP API
    pub base_url: String,
    /// Application ID for namespacing
    pub app_id: String,
    /// Optional API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds (default: 30)
    pub timeout_secs: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            app_id: "default".to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }
}

/// Document metadata from list operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Document ID
    pub doc_id: String,
    /// Document type (e.g., "graph", "path", "personal")
    pub doc_type: String,
    /// Number of changes in the document
    pub change_count: u64,
    /// Last modified timestamp (Unix millis)
    pub last_modified: u64,
    /// Current heads (hex-encoded change hashes)
    pub heads: Vec<String>,
}

/// Response from list documents endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDocumentsResponse {
    /// Application ID
    pub app_id: String,
    /// List of documents
    pub documents: Vec<DocumentInfo>,
    /// Total count (for pagination)
    pub total: u64,
    /// Pagination offset
    pub offset: u32,
    /// Pagination limit
    pub limit: u32,
}

/// Response from get document endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDocumentResponse {
    /// Application ID
    pub app_id: String,
    /// Document ID
    pub doc_id: String,
    /// Current heads (hex-encoded change hashes)
    pub heads: Vec<String>,
}

/// Response from get heads endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHeadsResponse {
    /// Application ID
    pub app_id: String,
    /// Document ID
    pub doc_id: String,
    /// Current heads (hex-encoded change hashes)
    pub heads: Vec<String>,
}

/// Response from get changes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetChangesResponse {
    /// Application ID
    pub app_id: String,
    /// Document ID
    pub doc_id: String,
    /// Changes as base64-encoded blobs
    pub changes: Vec<String>,
    /// New heads after applying these changes
    pub new_heads: Vec<String>,
}

/// Response from apply changes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyChangesResponse {
    /// Application ID
    pub app_id: String,
    /// Document ID
    pub doc_id: String,
    /// New heads after applying changes
    pub new_heads: Vec<String>,
}

/// Request body for apply changes
#[derive(Debug, Clone, Serialize)]
pub struct ApplyChangesRequest {
    /// Changes as base64-encoded blobs
    pub changes: Vec<String>,
}

/// Blob manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobManifest {
    /// Blob hash
    pub blob_hash: String,
    /// Blob CID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_cid: Option<String>,
    /// MIME type
    pub mime_type: String,
    /// Total size in bytes
    pub total_size: u64,
    /// Shard size
    pub shard_size: u32,
    /// Encoding method
    pub encoding: String,
    /// List of shard hashes
    pub shard_hashes: Vec<String>,
    /// Data parity shards
    pub data_parity: (u32, u32),
    /// Reach level
    pub reach: String,
}

/// Options for list documents
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Filter by document type prefix
    pub prefix: Option<String>,
    /// Pagination offset
    pub offset: Option<u32>,
    /// Pagination limit
    pub limit: Option<u32>,
}
