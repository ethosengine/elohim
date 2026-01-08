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

// ============================================================================
// Content Database API Types
// ============================================================================

/// Content metadata from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i64>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub validation_status: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Input for creating content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContentInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default = "default_content_format")]
    pub content_format: String,
    #[serde(default)]
    pub blob_hash: Option<String>,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<i64>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default = "default_reach")]
    pub reach: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_content_type() -> String { "concept".to_string() }
fn default_content_format() -> String { "markdown".to_string() }
fn default_reach() -> String { "public".to_string() }

/// Options for listing content
#[derive(Debug, Clone, Default, Serialize)]
pub struct ContentListOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

#[allow(dead_code)]  // Used by serde default
fn default_limit() -> u32 { 100 }

/// Response from list content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentListResponse {
    pub items: Vec<Content>,
    pub total: Option<u64>,
}

/// Result of bulk content operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResult {
    pub inserted: u64,
    pub skipped: u64,
    #[serde(default)]
    pub errors: Vec<String>,
}

// ============================================================================
// Path Database API Types
// ============================================================================

/// Path metadata from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub path_type: String,
    pub difficulty: Option<String>,
    pub estimated_duration: Option<String>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_alt: Option<String>,
    pub metadata_json: Option<String>,
    pub visibility: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Step within a path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub step_type: String,
    pub resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
    pub metadata_json: Option<String>,
}

/// Chapter within a path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub app_id: String,
    pub path_id: String,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Path with full details (chapters, steps, tags)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathWithDetails {
    #[serde(flatten)]
    pub path: Path,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub ungrouped_steps: Vec<Step>,
}

/// Input for creating a path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePathInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_path_type")]
    pub path_type: String,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub thumbnail_alt: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<CreateChapterInput>,
}

fn default_path_type() -> String { "guided".to_string() }
fn default_visibility() -> String { "public".to_string() }

/// Input for creating a chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChapterInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub steps: Vec<CreateStepInput>,
}

/// Input for creating a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStepInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_step_type")]
    pub step_type: String,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub resource_type: Option<String>,
    #[serde(default)]
    pub order_index: i32,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_step_type() -> String { "learn".to_string() }

/// Options for listing paths
#[derive(Debug, Clone, Default, Serialize)]
pub struct PathListOptions {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

/// Response from list paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathListResponse {
    pub items: Vec<Path>,
    pub total: Option<u64>,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub content_count: u64,
    pub path_count: u64,
    pub step_count: u64,
    pub unique_tags: u64,
}
