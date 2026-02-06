//! Mode-aware content client
//!
//! Provides unified content access that routes to the appropriate backend
//! based on the client mode (browser, native, node).

use crate::cache::{WriteBuffer, WriteBufferConfig, WriteOp, WritePriority};
use crate::error::{Result, SdkError};
use crate::reach::ReachEnforcer;
use crate::traits::{ContentReadable, ContentWriteable};
use elohim_storage_client::{StorageClient, StorageConfig};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::PathBuf;

/// Client deployment mode
///
/// Determines which backend the client uses for content operations.
#[derive(Debug, Clone)]
pub enum ClientMode {
    /// Browser mode - uses Doorway projection API
    /// No local storage, doorway-dependent, no offline capability
    Browser {
        /// Doorway URL (e.g., "https://doorway.example.com")
        doorway_url: String,
        /// Optional API key for authenticated access
        api_key: Option<String>,
    },

    /// Native mode - uses local SQLite storage
    /// Full offline capability, P2P sync when online
    Native {
        /// Path to local storage directory
        storage_path: PathBuf,
        /// Optional remote storage URL for sync
        sync_url: Option<String>,
    },

    /// Node mode - local storage that serves to doorways
    /// Full offline capability, P2P sync, content serving
    Node {
        /// Path to local storage directory
        storage_path: PathBuf,
        /// URL of the elohim-storage service
        storage_url: String,
        /// This node's public URL for content serving
        public_url: Option<String>,
    },
}

impl ClientMode {
    /// Check if this mode supports offline operation
    pub fn supports_offline(&self) -> bool {
        matches!(self, ClientMode::Native { .. } | ClientMode::Node { .. })
    }

    /// Check if this mode requires doorway
    pub fn requires_doorway(&self) -> bool {
        matches!(self, ClientMode::Browser { .. })
    }
}

/// Unified content client for the Elohim SDK
///
/// Provides mode-aware content access that automatically routes to
/// the appropriate backend based on deployment mode.
///
/// # Phase A: No DHT
///
/// In Phase A, this client uses:
/// - Browser: Doorway → Projection Store → SQLite
/// - Native/Node: Local SQLite → P2P sync
///
/// No Holochain conductor calls are made for content operations.
/// DHT will be added in Phase B for agent-centric data only.
///
/// # App Scoping
///
/// Content is scoped by app_id to support multi-tenant isolation:
/// - `lamad`: Learning content (paths, concepts, quizzes)
/// - `elohim`: Shared infrastructure (resources, sensemaking)
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::{ContentClient, ClientMode};
///
/// // Browser mode for learning content
/// let client = ContentClient::new(
///     ClientMode::Browser {
///         doorway_url: "https://doorway.example.com".into(),
///         api_key: None,
///     },
///     "lamad",  // app_id for learning content
/// );
///
/// // Get content
/// let content = client.get::<Content>("manifesto").await?;
///
/// // Native mode with full offline support
/// let client = ContentClient::new(
///     ClientMode::Native {
///         storage_path: "/data/elohim".into(),
///         sync_url: Some("http://localhost:8090".into()),
///     },
///     "elohim",  // app_id for shared infrastructure
/// );
/// ```
pub struct ContentClient {
    mode: ClientMode,
    /// App ID for multi-tenant scoping (e.g., "lamad", "elohim")
    app_id: String,
    /// elohim-storage HTTP client (for sync operations in native/node modes)
    #[allow(dead_code)]
    storage: Option<StorageClient>,
    /// Write buffer for backpressure protection
    write_buffer: WriteBuffer,
    /// Reach enforcer for access control
    reach_enforcer: ReachEnforcer,
    /// HTTP client for browser mode (projection API)
    http_client: reqwest::Client,
}

impl ContentClient {
    /// Create a new content client with the specified mode and app scope
    ///
    /// # Arguments
    ///
    /// * `mode` - The client deployment mode (Browser, Native, or Node)
    /// * `app_id` - The app scope for multi-tenant isolation (e.g., "lamad", "elohim")
    pub fn new(mode: ClientMode, app_id: impl Into<String>) -> Self {
        let app_id = app_id.into();

        let storage = match &mode {
            ClientMode::Native { sync_url: Some(url), .. } |
            ClientMode::Node { storage_url: url, .. } => {
                Some(StorageClient::new(StorageConfig {
                    base_url: url.clone(),
                    app_id: app_id.clone(),
                    ..Default::default()
                }))
            }
            _ => None,
        };

        let buffer_config = match &mode {
            ClientMode::Browser { .. } => WriteBufferConfig::for_interactive(),
            ClientMode::Native { .. } => WriteBufferConfig::default(),
            ClientMode::Node { .. } => WriteBufferConfig::for_seeding(),
        };

        Self {
            mode,
            app_id,
            storage,
            write_buffer: WriteBuffer::new(buffer_config),
            reach_enforcer: ReachEnforcer::authenticated(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a content client for learning content (lamad app scope)
    pub fn for_lamad(mode: ClientMode) -> Self {
        Self::new(mode, "lamad")
    }

    /// Create a content client for shared infrastructure (elohim app scope)
    pub fn for_elohim(mode: ClientMode) -> Self {
        Self::new(mode, "elohim")
    }

    /// Create a client for anonymous browser access (defaults to lamad scope)
    pub fn anonymous_browser(doorway_url: impl Into<String>) -> Self {
        let mut client = Self::new(
            ClientMode::Browser {
                doorway_url: doorway_url.into(),
                api_key: None,
            },
            "lamad",
        );
        client.reach_enforcer = ReachEnforcer::anonymous();
        client
    }

    /// Get the client mode
    pub fn mode(&self) -> &ClientMode {
        &self.mode
    }

    /// Get the app ID (scope)
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Get the write buffer
    pub fn write_buffer(&self) -> &WriteBuffer {
        &self.write_buffer
    }

    /// Get content by ID
    ///
    /// Routes to appropriate backend based on mode:
    /// - Browser: GET {doorway}/api/v1/cache/{type}/{id}
    /// - Native/Node: GET {storage}/db/{type}/{id}
    pub async fn get<T: ContentReadable>(&self, id: &str) -> Result<Option<T>> {
        match &self.mode {
            ClientMode::Browser { doorway_url, api_key } => {
                self.get_from_projection::<T>(doorway_url, api_key.as_deref(), id).await
            }
            ClientMode::Native { sync_url: Some(url), .. } |
            ClientMode::Node { storage_url: url, .. } => {
                self.get_from_storage::<T>(url, id).await
            }
            ClientMode::Native { sync_url: None, .. } => {
                // Native without sync URL - can only read from local cache
                // TODO: Implement local SQLite read
                Err(SdkError::InvalidMode("Native mode without sync_url cannot fetch remote content".into()))
            }
        }
    }

    /// Get multiple content items by ID
    pub async fn get_batch<T: ContentReadable>(&self, ids: &[&str]) -> Result<HashMap<String, T>> {
        let mut results = HashMap::new();
        for id in ids {
            if let Some(content) = self.get::<T>(id).await? {
                results.insert(id.to_string(), content);
            }
        }
        Ok(results)
    }

    /// Save content (queues for write buffer)
    ///
    /// Content is queued in the write buffer and will be flushed
    /// to the backend when the buffer threshold is reached or
    /// when flush() is called.
    pub async fn save<T: ContentWriteable>(&self, content: &T) -> Result<()> {
        content.validate()?;

        let op = WriteOp::new(
            T::content_type(),
            content.content_id(),
            content.to_json()?,
            WritePriority::Normal,
        );

        self.write_buffer.queue(op).await
    }

    /// Save content with high priority (flushes immediately)
    pub async fn save_immediate<T: ContentWriteable>(&self, content: &T) -> Result<()> {
        content.validate()?;

        let op = WriteOp::new(
            T::content_type(),
            content.content_id(),
            content.to_json()?,
            WritePriority::High,
        );

        self.write_buffer.queue(op).await?;

        // Force immediate flush for high priority
        self.flush().await
    }

    /// Flush pending writes to backend
    pub async fn flush(&self) -> Result<()> {
        let batch = self.write_buffer.take_batch().await;
        if batch.is_empty() {
            return Ok(());
        }

        match &self.mode {
            ClientMode::Browser { doorway_url, api_key } => {
                self.flush_to_projection(doorway_url, api_key.as_deref(), &batch).await
            }
            ClientMode::Native { sync_url: Some(url), .. } |
            ClientMode::Node { storage_url: url, .. } => {
                self.flush_to_storage(url, &batch).await
            }
            ClientMode::Native { sync_url: None, .. } => {
                // TODO: Local-only flush to SQLite
                tracing::warn!("Flush to local-only native mode not yet implemented");
                Ok(())
            }
        }
    }

    /// Check current backpressure level (0-100)
    pub async fn backpressure(&self) -> u8 {
        self.write_buffer.backpressure().await
    }

    // === Private Implementation ===

    async fn get_from_projection<T: DeserializeOwned>(
        &self,
        doorway_url: &str,
        api_key: Option<&str>,
        id: &str,
    ) -> Result<Option<T>> {
        let content_type = std::any::type_name::<T>()
            .rsplit("::")
            .next()
            .unwrap_or("content")
            .to_lowercase();

        let url = format!("{}/api/v1/cache/{}/{}", doorway_url, content_type, id);

        let mut request = self.http_client.get(&url);
        if let Some(key) = api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::Network(format!("HTTP {} - {}", status, body)));
        }

        let content: T = response.json().await?;
        Ok(Some(content))
    }

    async fn get_from_storage<T: DeserializeOwned>(
        &self,
        storage_url: &str,
        id: &str,
    ) -> Result<Option<T>> {
        let content_type = std::any::type_name::<T>()
            .rsplit("::")
            .next()
            .unwrap_or("content")
            .to_lowercase();

        // Use app_id in the URL path for multi-tenant scoping
        let url = format!("{}/db/{}/{}/{}", storage_url, self.app_id, content_type, id);

        let response = self.http_client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::Storage(format!("HTTP {} - {}", status, body)));
        }

        let content: T = response.json().await?;
        Ok(Some(content))
    }

    async fn flush_to_projection(
        &self,
        doorway_url: &str,
        api_key: Option<&str>,
        batch: &[WriteOp],
    ) -> Result<()> {
        // Group by content type
        let mut by_type: HashMap<&str, Vec<&WriteOp>> = HashMap::new();
        for op in batch {
            by_type.entry(&op.content_type).or_default().push(op);
        }

        for (content_type, ops) in by_type {
            // Use app_id in the URL path for multi-tenant scoping
            let url = format!("{}/db/{}/{}/bulk", doorway_url, self.app_id, content_type);
            let items: Vec<_> = ops.iter().map(|op| &op.data).collect();

            let mut request = self.http_client.post(&url).json(&items);
            if let Some(key) = api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }

            let response = request.send().await?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Failed to flush {} items to projection: HTTP {} - {}", ops.len(), status, body);
            }
        }

        Ok(())
    }

    async fn flush_to_storage(
        &self,
        storage_url: &str,
        batch: &[WriteOp],
    ) -> Result<()> {
        // Group by content type
        let mut by_type: HashMap<&str, Vec<&WriteOp>> = HashMap::new();
        for op in batch {
            by_type.entry(&op.content_type).or_default().push(op);
        }

        for (content_type, ops) in by_type {
            // Use app_id in the URL path for multi-tenant scoping
            let url = format!("{}/db/{}/{}/bulk", storage_url, self.app_id, content_type);
            let items: Vec<_> = ops.iter().map(|op| &op.data).collect();

            let response = self.http_client.post(&url).json(&items).send().await?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                tracing::error!("Failed to flush {} items to storage: HTTP {} - {}", ops.len(), status, body);
            }
        }

        Ok(())
    }
}
