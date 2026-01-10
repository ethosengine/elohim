//! HTTP API for shard storage
//!
//! Provides REST endpoints for storing and retrieving shards:
//!
//! ## Shard API (Direct Shard Access)
//! - `PUT /shard/{shard_hash}` - Store a shard
//! - `GET /shard/{shard_hash}` - Retrieve a shard
//! - `HEAD /shard/{shard_hash}` - Check if shard exists
//!
//! ## Blob API (Convenience - Auto-Sharding)
//! - `PUT /blob/{blob_hash}` - Store blob, auto-create manifest
//! - `GET /blob/{blob_hash}` - Reassemble blob from shards
//! - `GET /manifest/{blob_hash}` - Get shard manifest
//!
//! ## Example Usage
//!
//! ```bash
//! # Store a shard
//! curl -X PUT -H "Content-Type: application/octet-stream" \
//!      --data-binary @video-chunk.bin \
//!      http://localhost:8090/shard/sha256-abc123
//!
//! # Retrieve a shard
//! curl http://localhost:8090/shard/sha256-abc123 > chunk.bin
//!
//! # Store a blob (auto-shards into manifest)
//! curl -X PUT -H "Content-Type: video/mp4" \
//!      --data-binary @video.mp4 \
//!      http://localhost:8090/blob/sha256-xyz789
//!
//! # Get manifest to see shards
//! curl http://localhost:8090/manifest/sha256-xyz789
//! ```

use crate::blob_store::BlobStore;
use crate::db::{self, ContentDb, ContentQuery, DbPool, AppContext};
use crate::db::{human_relationships, contributor_presences, economic_events, content_mastery, stewardship_allocations};
use crate::error::StorageError;
use crate::import_api::ImportApi;
use crate::progress_hub::ProgressHub;
use crate::progress_ws;
use crate::services::{response, Services};
use crate::sharding::{ShardEncoder, ShardManifest};
use crate::sync::SyncManager;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// HTTP server state
pub struct HttpServer {
    blob_store: Arc<BlobStore>,
    manifests: Arc<RwLock<std::collections::HashMap<String, ShardManifest>>>,
    bind_addr: SocketAddr,
    /// Optional Import API for handling /import/* routes
    import_api: Option<Arc<RwLock<ImportApi>>>,
    /// Progress hub for WebSocket streaming
    progress_hub: Option<Arc<ProgressHub>>,
    /// Sync manager for CRDT document sync
    sync_manager: Option<Arc<SyncManager>>,
    /// SQLite content database for structured data
    content_db: Option<Arc<ContentDb>>,
    /// Diesel connection pool for new entity endpoints
    db_pool: Option<DbPool>,
    /// Service layer for business logic
    services: Option<Arc<Services>>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(blob_store: Arc<BlobStore>, bind_addr: SocketAddr) -> Self {
        Self {
            blob_store,
            manifests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bind_addr,
            import_api: None,
            progress_hub: None,
            sync_manager: None,
            content_db: None,
            db_pool: None,
            services: None,
        }
    }

    /// Set the Import API handler
    pub fn with_import_api(mut self, import_api: Arc<RwLock<ImportApi>>) -> Self {
        self.import_api = Some(import_api);
        self
    }

    /// Set the Progress Hub for WebSocket streaming
    pub fn with_progress_hub(mut self, hub: Arc<ProgressHub>) -> Self {
        self.progress_hub = Some(hub);
        self
    }

    /// Set the Sync Manager for CRDT document sync
    pub fn with_sync_manager(mut self, sync_manager: Arc<SyncManager>) -> Self {
        self.sync_manager = Some(sync_manager);
        self
    }

    /// Set the SQLite Content Database
    pub fn with_content_db(mut self, content_db: Arc<ContentDb>) -> Self {
        self.content_db = Some(content_db);
        self
    }

    /// Set the Service layer for business logic
    pub fn with_services(mut self, services: Arc<Services>) -> Self {
        self.services = Some(services);
        self
    }

    /// Set the Diesel connection pool for new entity endpoints
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Run the HTTP server
    pub async fn run(self: Arc<Self>) -> Result<(), StorageError> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!(addr = %self.bind_addr, "HTTP server listening");

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let server = self.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let server = server.clone();
                    async move { server.handle_request(req).await }
                });

                // Enable HTTP upgrades for WebSocket support
                // Without .with_upgrades(), WebSocket handshakes fail immediately
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    warn!(addr = %remote_addr, error = %err, "Connection error");
                }
            });
        }
    }

    /// Route requests to handlers
    async fn handle_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        debug!(method = %method, path = %path, "Incoming request");

        let result = match (method, path.as_str()) {
            // Health check
            (Method::GET, "/health") => self.handle_health().await,

            // Shard API
            (Method::PUT, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_put_shard(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_get_shard(hash).await
            }
            (Method::HEAD, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_head_shard(hash).await
            }

            // Blob API (convenience with auto-sharding)
            (Method::PUT, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                self.handle_put_blob(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                self.handle_get_blob(hash).await
            }

            // Manifest API
            (Method::GET, p) if p.starts_with("/manifest/") => {
                let hash = p.strip_prefix("/manifest/").unwrap_or("");
                self.handle_get_manifest(hash).await
            }

            // WebSocket upgrade for progress streaming
            (Method::GET, "/import/progress") if progress_ws::is_websocket_upgrade(&req) => {
                if let Some(ref hub) = self.progress_hub {
                    match progress_ws::handle_progress_upgrade(req, Arc::clone(hub)).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            error!(error = %e, "WebSocket upgrade failed");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Full::new(Bytes::from("WebSocket upgrade failed")))
                                .unwrap())
                        }
                    }
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Progress hub not enabled"}"#
                        )))
                        .unwrap())
                }
            }

            // Import API (forwarded from doorway)
            (_, p) if p.starts_with("/import/") => {
                if let Some(ref import_api) = self.import_api {
                    // Lazy reconnection: if HcClient is not connected, attempt to reconnect
                    // This handles the case where elohim-storage starts before the hApp is installed
                    {
                        let api = import_api.read().await;
                        if api.needs_reconnect() {
                            drop(api); // Release read lock before acquiring write lock
                            let mut api_write = import_api.write().await;
                            if api_write.needs_reconnect() { // Double-check after acquiring write lock
                                info!("Import API: Attempting lazy reconnection to conductor...");
                                match api_write.connect_conductor().await {
                                    Ok(_) => info!("Import API: Lazy reconnection successful"),
                                    Err(e) => warn!(error = %e, "Import API: Lazy reconnection failed"),
                                }
                            }
                        }
                    }

                    let api = import_api.read().await;
                    api.handle_request(req, &path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Import API not enabled. Set ENABLE_IMPORT_API=true"}"#
                        )))
                        .unwrap())
                }
            }

            // Sync API: /sync/v1/{app_id}/docs[/{doc_id}[/heads|/changes]]
            (method, p) if p.starts_with("/sync/v1/") => {
                if let Some(ref sync_manager) = self.sync_manager {
                    self.handle_sync_request(req, method, &path, sync_manager.clone()).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Sync API not enabled"}"#
                        )))
                        .unwrap())
                }
            }

            // Database API: Content, Paths, Stats
            (method, p) if p.starts_with("/db/") => {
                if let Some(ref content_db) = self.content_db {
                    self.handle_db_request(req, method, &path, content_db.clone()).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#
                        )))
                        .unwrap())
                }
            }

            // HTML5 App serving: /apps/{app_id}/{file_path}
            (Method::GET, p) if p.starts_with("/apps/") => {
                if let Some(ref content_db) = self.content_db {
                    self.handle_app_request(&path, content_db.clone()).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#
                        )))
                        .unwrap())
                }
            }

            // Not found
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap()),
        };

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                error!(error = %e, "Request error");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from(format!("Error: {}", e))))
                    .unwrap())
            }
        }
    }

    /// Health check endpoint
    async fn handle_health(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let stats = self.blob_store.stats().await?;
        let body = serde_json::json!({
            "status": "ok",
            "blobs": stats.total_blobs,
            "bytes": stats.total_bytes,
            "manifests": self.manifests.read().await.len(),
            "import_enabled": self.import_api.is_some(),
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// PUT /shard/{hash} - Store a shard
    async fn handle_put_shard(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Read body
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;
        let data = body.to_bytes();

        // Verify hash - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash.strip_prefix("sha256-").unwrap_or(&computed_hash);
        let expected_hex = expected_hash.strip_prefix("sha256-").unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Store shard
        let result = self.blob_store.store(&data).await?;

        info!(
            hash = %result.hash,
            size = result.size_bytes,
            existed = result.already_existed,
            "Stored shard"
        );

        let body = serde_json::json!({
            "hash": result.hash,
            "size_bytes": result.size_bytes,
            "already_existed": result.already_existed,
        });

        Ok(Response::builder()
            .status(if result.already_existed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            })
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// GET /shard/{hash} - Retrieve a shard
    async fn handle_get_shard(
        &self,
        hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing shard hash")))
                .unwrap());
        }

        match self.blob_store.get(hash).await {
            Ok(data) => {
                info!(hash = %hash, size = data.len(), "Serving shard");

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CONTENT_LENGTH, data.len())
                    .header(header::ETAG, format!("\"{}\"", hash))
                    .header(
                        header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable",
                    )
                    .body(Full::new(Bytes::from(data)))
                    .unwrap())
            }
            Err(StorageError::NotFound(_)) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Shard not found")))
                .unwrap()),
            Err(e) => Err(e),
        }
    }

    /// HEAD /shard/{hash} - Check if shard exists
    async fn handle_head_shard(
        &self,
        hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        match self.blob_store.size(hash).await {
            Ok(size) => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, size)
                .header(header::ETAG, format!("\"{}\"", hash))
                .body(Full::new(Bytes::new()))
                .unwrap()),
            Err(_) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap()),
        }
    }

    /// PUT /blob/{hash} - Store blob with auto-sharding
    async fn handle_put_blob(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Get content type from header
        let mime_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read body
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;
        let data = body.to_bytes().to_vec();

        // Verify hash if provided - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash.strip_prefix("sha256-").unwrap_or(&computed_hash);
        let expected_hex = expected_hash.strip_prefix("sha256-").unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Create shard encoder and generate manifest
        let encoder = ShardEncoder::new(crate::sharding::ShardConfig::default());
        let manifest = encoder.create_manifest(&data, &mime_type, "commons");

        // Store each shard
        for (i, shard_hash) in manifest.shard_hashes.iter().enumerate() {
            // For "none" encoding, the whole blob is one shard
            let shard_data = if manifest.encoding == "none" {
                data.clone()
            } else {
                // For chunked encoding, split the data
                let start = i * manifest.shard_size as usize;
                let end = ((i + 1) * manifest.shard_size as usize).min(data.len());
                data[start..end].to_vec()
            };

            // Verify shard hash matches
            let actual_hash = BlobStore::compute_hash(&shard_data);
            if actual_hash != *shard_hash {
                warn!(
                    expected = %shard_hash,
                    actual = %actual_hash,
                    index = i,
                    "Shard hash mismatch during blob storage"
                );
            }

            self.blob_store.store(&shard_data).await?;
        }

        // Store manifest
        self.manifests
            .write()
            .await
            .insert(manifest.blob_hash.clone(), manifest.clone());

        info!(
            blob_hash = %manifest.blob_hash,
            total_size = manifest.total_size,
            shards = manifest.shard_hashes.len(),
            encoding = %manifest.encoding,
            "Stored blob with manifest"
        );

        let body = serde_json::to_string(&manifest)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap())
    }

    /// GET /blob/{hash} - Reassemble blob from shards
    async fn handle_get_blob(
        &self,
        hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        // Get manifest
        let manifest = self.manifests.read().await.get(hash).cloned();
        let manifest = match manifest {
            Some(m) => m,
            None => {
                // Try direct blob lookup (for non-sharded blobs)
                match self.blob_store.get(hash).await {
                    Ok(data) => {
                        return Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/octet-stream")
                            .header(header::CONTENT_LENGTH, data.len())
                            .body(Full::new(Bytes::from(data)))
                            .unwrap());
                    }
                    Err(_) => {
                        return Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::from("Blob not found")))
                            .unwrap());
                    }
                }
            }
        };

        // Reassemble from shards
        let mut data = Vec::with_capacity(manifest.total_size as usize);

        for shard_hash in &manifest.shard_hashes {
            let shard_data = self.blob_store.get(shard_hash).await?;
            data.extend_from_slice(&shard_data);
        }

        // Truncate to actual size (last shard may be padded)
        data.truncate(manifest.total_size as usize);

        info!(
            hash = %hash,
            size = data.len(),
            shards = manifest.shard_hashes.len(),
            "Serving reassembled blob"
        );

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &manifest.mime_type)
            .header(header::CONTENT_LENGTH, data.len())
            .header(header::ETAG, format!("\"{}\"", hash))
            .body(Full::new(Bytes::from(data)))
            .unwrap())
    }

    /// GET /manifest/{hash} - Get shard manifest
    async fn handle_get_manifest(
        &self,
        hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        let manifest = self.manifests.read().await.get(hash).cloned();

        match manifest {
            Some(m) => {
                let body = serde_json::to_string(&m)
                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Manifest not found")))
                .unwrap()),
        }
    }

    /// Handle sync API requests
    ///
    /// Routes:
    /// - GET /sync/v1/{app_id}/docs - List documents
    /// - GET /sync/v1/{app_id}/docs/{doc_id}/heads - Get document heads
    /// - GET /sync/v1/{app_id}/docs/{doc_id}/changes?have={heads} - Get changes since heads
    /// - POST /sync/v1/{app_id}/docs/{doc_id}/changes - Apply changes
    async fn handle_sync_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Parse path: /sync/v1/{app_id}/docs[/{doc_id}[/heads|/changes]]
        let parts: Vec<&str> = path.trim_start_matches("/sync/v1/").split('/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Missing app_id"}"#)))
                .unwrap());
        }

        let app_id = parts[0];

        // /sync/v1/{app_id}/docs
        if parts.len() == 2 && parts[1] == "docs" {
            return self.handle_sync_list_docs(method, app_id, &req, sync_manager).await;
        }

        // /sync/v1/{app_id}/docs/{doc_id}
        if parts.len() == 3 && parts[1] == "docs" {
            let doc_id = parts[2];
            return self.handle_sync_doc(method, app_id, doc_id, req, sync_manager).await;
        }

        // /sync/v1/{app_id}/docs/{doc_id}/{action}
        if parts.len() == 4 && parts[1] == "docs" {
            let doc_id = parts[2];
            let action = parts[3];

            return match action {
                "heads" => self.handle_sync_heads(method, app_id, doc_id, sync_manager).await,
                "changes" => self.handle_sync_changes(method, app_id, doc_id, req, sync_manager).await,
                _ => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Unknown action: {}"}}"#,
                        action
                    ))))
                    .unwrap()),
            };
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Invalid sync path"}"#)))
            .unwrap())
    }

    /// GET /sync/v1/{app_id}/docs - List documents
    async fn handle_sync_list_docs(
        &self,
        method: Method,
        app_id: &str,
        req: &Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        // Parse query params: ?prefix=&offset=&limit=
        let query = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();

        let prefix = params.get("prefix").map(|s| s.as_str());
        let offset: u32 = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);
        let limit: u32 = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);

        match sync_manager.list_documents(app_id, prefix, offset, limit).await {
            Ok((docs, total)) => {
                let documents: Vec<serde_json::Value> = docs
                    .into_iter()
                    .map(|d| {
                        serde_json::json!({
                            "doc_id": d.doc_id,
                            "doc_type": d.doc_type,
                            "change_count": d.change_count,
                            "last_modified": d.last_modified,
                            "heads": d.heads,
                        })
                    })
                    .collect();

                let body = serde_json::json!({
                    "app_id": app_id,
                    "documents": documents,
                    "total": total,
                    "offset": offset,
                    "limit": limit,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(app_id = %app_id, error = %e, "Failed to list documents");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "{}"}}"#,
                        e
                    ))))
                    .unwrap())
            }
        }
    }

    /// Handle document-level requests
    async fn handle_sync_doc(
        &self,
        method: Method,
        app_id: &str,
        doc_id: &str,
        _req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // Return document info
                match sync_manager.get_heads(app_id, doc_id).await {
                    Ok(heads) => {
                        if heads.is_empty() {
                            return Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error": "Document not found: {}"}}"#,
                                    doc_id
                                ))))
                                .unwrap());
                        }

                        let body = serde_json::json!({
                            "app_id": app_id,
                            "doc_id": doc_id,
                            "heads": heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get document");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "{}"}}"#,
                                e
                            ))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// GET /sync/v1/{app_id}/docs/{doc_id}/heads - Get document heads
    async fn handle_sync_heads(
        &self,
        method: Method,
        app_id: &str,
        doc_id: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        match sync_manager.get_heads(app_id, doc_id).await {
            Ok(heads) => {
                let body = serde_json::json!({
                    "app_id": app_id,
                    "doc_id": doc_id,
                    "heads": heads,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get heads");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "{}"}}"#,
                        e
                    ))))
                    .unwrap())
            }
        }
    }

    /// GET/POST /sync/v1/{app_id}/docs/{doc_id}/changes
    async fn handle_sync_changes(
        &self,
        method: Method,
        app_id: &str,
        doc_id: &str,
        req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // GET changes since given heads
                let query = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                // Parse have_heads from comma-separated list
                let have_heads: Vec<String> = params
                    .get("have")
                    .map(|s| s.split(',').map(|h| h.to_string()).collect())
                    .unwrap_or_default();

                match sync_manager.get_changes_since(app_id, doc_id, &have_heads).await {
                    Ok((changes, new_heads)) => {
                        // Encode changes as base64 for JSON transport
                        let changes_b64: Vec<String> = changes
                            .iter()
                            .map(|c| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, c))
                            .collect();

                        let body = serde_json::json!({
                            "app_id": app_id,
                            "doc_id": doc_id,
                            "changes": changes_b64,
                            "new_heads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "{}"}}"#,
                                e
                            ))))
                            .unwrap())
                    }
                }
            }
            Method::POST => {
                // Apply changes from client
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                // Parse JSON body: { "changes": ["base64..."] }
                let payload: serde_json::Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                let changes_b64 = payload["changes"]
                    .as_array()
                    .ok_or_else(|| StorageError::Internal("Missing 'changes' array".to_string()))?;

                // Decode base64 changes
                let changes: Vec<Vec<u8>> = changes_b64
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok())
                    .collect();

                if changes.is_empty() {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(r#"{"error": "No valid changes"}"#)))
                        .unwrap());
                }

                match sync_manager.apply_changes(app_id, doc_id, changes).await {
                    Ok(new_heads) => {
                        info!(app_id = %app_id, doc_id = %doc_id, heads = ?new_heads, "Applied changes via HTTP");

                        let body = serde_json::json!({
                            "app_id": app_id,
                            "doc_id": doc_id,
                            "new_heads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to apply changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "{}"}}"#,
                                e
                            ))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Database API handlers
    // =========================================================================

    /// Handle database API requests
    ///
    /// Routes:
    /// - GET /db/stats - Database statistics
    /// - GET /db/content - List content (with query params)
    /// - GET /db/content/{id} - Get content by ID
    /// - POST /db/content - Create single content
    /// - POST /db/content/bulk - Bulk create content
    /// - GET /db/paths - List paths
    /// - GET /db/paths/{id} - Get path by ID with steps
    /// - POST /db/paths - Create single path
    /// - POST /db/paths/bulk - Bulk create paths
    /// - GET /db/relationships - List relationships (with query params)
    /// - GET /db/relationships/{id} - Get relationship by ID
    /// - POST /db/relationships - Create relationship
    /// - POST /db/relationships/bulk - Bulk create relationships
    /// - GET /db/relationships/graph/{content_id} - Get content graph
    /// - GET /db/knowledge-maps - List knowledge maps
    /// - GET /db/knowledge-maps/{id} - Get knowledge map by ID
    /// - POST /db/knowledge-maps - Create knowledge map
    /// - PUT /db/knowledge-maps/{id} - Update knowledge map
    /// - DELETE /db/knowledge-maps/{id} - Delete knowledge map
    /// - GET /db/path-extensions - List path extensions
    /// - GET /db/path-extensions/{id} - Get path extension by ID
    /// - POST /db/path-extensions - Create path extension
    /// - PUT /db/path-extensions/{id} - Update path extension
    /// - DELETE /db/path-extensions/{id} - Delete path extension
    ///
    /// Extract app context from path, supporting both:
    /// - New: /db/{app_id}/content/... -> AppContext(app_id)
    /// - Legacy: /db/content/... -> AppContext("lamad") for backwards compatibility
    fn extract_app_context(sub_path: &str) -> (db::AppContext, &str) {
        // Check if path starts with a known resource type (legacy route)
        let legacy_prefixes = ["content", "paths", "stats"];
        for prefix in &legacy_prefixes {
            if sub_path == *prefix || sub_path.starts_with(&format!("{}/", prefix)) {
                // Legacy route: default to 'lamad' for learning content
                return (db::AppContext::default_lamad(), sub_path);
            }
        }

        // New route: /db/{app_id}/...
        if let Some(slash_pos) = sub_path.find('/') {
            let app_id = &sub_path[..slash_pos];
            let resource_path = &sub_path[slash_pos + 1..];
            return (db::AppContext::new(app_id), resource_path);
        }

        // Just app_id with no resource (e.g., /db/lamad -> stats for that app)
        if !sub_path.is_empty() && !legacy_prefixes.contains(&sub_path) {
            return (db::AppContext::new(sub_path), "stats");
        }

        // Fallback to default
        (db::AppContext::default_lamad(), sub_path)
    }

    async fn handle_db_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
        content_db: Arc<ContentDb>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Strip /db/ prefix
        let sub_path = path.strip_prefix("/db/").unwrap_or("");

        // Extract app context (supports both legacy and new routes)
        let (app_ctx, resource_path) = Self::extract_app_context(sub_path);
        debug!(app_id = %app_ctx.app_id, resource_path = %resource_path, "DB request routing");

        // Route to specific handlers
        // Note: Handlers currently use legacy rusqlite code.
        // They will be updated to use Diesel with app_ctx in a future phase.
        if resource_path == "stats" {
            return self.handle_db_stats(method, &content_db).await;
        }

        if resource_path == "content" {
            return self.handle_db_content_list(req, method, &content_db).await;
        }

        if resource_path == "content/bulk" {
            return self.handle_db_content_bulk(req, method, &content_db).await;
        }

        if let Some(content_id) = resource_path.strip_prefix("content/") {
            return self.handle_db_content_by_id(req, method, content_id, &content_db).await;
        }

        if resource_path == "paths" {
            return self.handle_db_paths_list(req, method, &content_db).await;
        }

        if resource_path == "paths/bulk" {
            return self.handle_db_paths_bulk(req, method, &content_db).await;
        }

        if let Some(path_id) = resource_path.strip_prefix("paths/") {
            return self.handle_db_path_by_id(req, method, path_id, &content_db).await;
        }

        // Relationships routes
        if resource_path == "relationships" {
            return self.handle_db_relationships_list(req, method, &content_db).await;
        }

        if resource_path == "relationships/bulk" {
            return self.handle_db_relationships_bulk(req, method, &content_db).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/graph/") {
            return self.handle_db_content_graph(req, method, rel_id, &content_db).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/") {
            return self.handle_db_relationship_by_id(req, method, rel_id, &content_db).await;
        }

        // Knowledge maps routes
        if resource_path == "knowledge-maps" {
            return self.handle_db_knowledge_maps_list(req, method, &content_db).await;
        }

        if let Some(map_id) = resource_path.strip_prefix("knowledge-maps/") {
            return self.handle_db_knowledge_map_by_id(req, method, map_id, &content_db).await;
        }

        // Path extensions routes
        if resource_path == "path-extensions" {
            return self.handle_db_path_extensions_list(req, method, &content_db).await;
        }

        if let Some(ext_id) = resource_path.strip_prefix("path-extensions/") {
            return self.handle_db_path_extension_by_id(req, method, ext_id, &content_db).await;
        }

        // ============================================================================
        // Diesel-based entity routes (using db_pool)
        // ============================================================================

        // Human relationships routes (Diesel)
        if resource_path == "human-relationships" {
            return self.handle_human_relationships_list(req, method, &app_ctx).await;
        }

        if let Some(rel_path) = resource_path.strip_prefix("human-relationships/") {
            // Check for action sub-paths first
            if let Some(rest) = rel_path.strip_suffix("/consent") {
                return self.handle_human_relationship_consent(req, method, rest, &app_ctx).await;
            }
            if let Some(rest) = rel_path.strip_suffix("/custody") {
                return self.handle_human_relationship_custody(req, method, rest, &app_ctx).await;
            }
            // Fall back to generic ID handler
            return self.handle_human_relationship_by_id(req, method, rel_path, &app_ctx).await;
        }

        // Contributor presences routes (Diesel)
        if resource_path == "presences" {
            return self.handle_presences_list(req, method, &app_ctx).await;
        }

        if resource_path == "presences/bulk" {
            return self.handle_presences_bulk(req, method, &app_ctx).await;
        }

        if let Some(presence_path) = resource_path.strip_prefix("presences/") {
            // Check for action sub-paths first
            if let Some(rest) = presence_path.strip_suffix("/stewardship") {
                return self.handle_presence_stewardship(req, method, rest, &app_ctx).await;
            }
            if let Some(rest) = presence_path.strip_suffix("/claim") {
                return self.handle_presence_claim(req, method, rest, &app_ctx).await;
            }
            if let Some(rest) = presence_path.strip_suffix("/verify-claim") {
                return self.handle_presence_verify_claim(req, method, rest, &app_ctx).await;
            }
            // Fall back to generic ID handler
            return self.handle_presence_by_id(req, method, presence_path, &app_ctx).await;
        }

        // Economic events routes (Diesel)
        if resource_path == "events" {
            return self.handle_events_list(req, method, &app_ctx).await;
        }

        if resource_path == "events/bulk" {
            return self.handle_events_bulk(req, method, &app_ctx).await;
        }

        if let Some(event_id) = resource_path.strip_prefix("events/") {
            return self.handle_event_by_id(req, method, event_id, &app_ctx).await;
        }

        // Content mastery routes (Diesel)
        if resource_path == "mastery" {
            return self.handle_mastery_list(req, method, &app_ctx).await;
        }

        if resource_path == "mastery/bulk" {
            return self.handle_mastery_bulk(req, method, &app_ctx).await;
        }

        if let Some(mastery_path) = resource_path.strip_prefix("mastery/") {
            // Support /mastery/human/{human_id} and /mastery/{id}
            if let Some(human_id) = mastery_path.strip_prefix("human/") {
                return self.handle_mastery_for_human(req, method, human_id, &app_ctx).await;
            }
            return self.handle_mastery_by_id(req, method, mastery_path, &app_ctx).await;
        }

        // Stewardship allocations routes (Diesel)
        if resource_path == "allocations" {
            return self.handle_allocations_list(req, method, &app_ctx).await;
        }

        if resource_path == "allocations/bulk" {
            return self.handle_allocations_bulk(req, method, &app_ctx).await;
        }

        if let Some(alloc_path) = resource_path.strip_prefix("allocations/") {
            // Support /allocations/content/{content_id} and /allocations/steward/{steward_id}
            if let Some(content_id) = alloc_path.strip_prefix("content/") {
                return self.handle_allocations_for_content(req, method, content_id, &app_ctx).await;
            }
            if let Some(steward_id) = alloc_path.strip_prefix("steward/") {
                return self.handle_allocations_for_steward(req, method, steward_id, &app_ctx).await;
            }
            // Check for action sub-paths
            if let Some(rest) = alloc_path.strip_suffix("/dispute") {
                return self.handle_allocation_dispute(req, method, rest, &app_ctx).await;
            }
            if let Some(rest) = alloc_path.strip_suffix("/resolve") {
                return self.handle_allocation_resolve(req, method, rest, &app_ctx).await;
            }
            return self.handle_allocation_by_id(req, method, alloc_path, &app_ctx).await;
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Unknown database endpoint"}"#)))
            .unwrap())
    }

    /// GET /db/stats - Database statistics
    async fn handle_db_stats(
        &self,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling (uses response helpers)
        if self.services.is_some() {
            if method != Method::GET {
                return Ok(response::method_not_allowed());
            }
            // Stats come from content_db directly - it's a database-level operation
            return Ok(response::from_result(content_db.stats()));
        }

        // Legacy fallback
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        match content_db.stats() {
            Ok(stats) => {
                let body = serde_json::to_string(&stats)
                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }
            Err(e) => {
                error!(error = %e, "Failed to get database stats");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                    .unwrap())
            }
        }
    }

    /// GET /db/content - List content, POST /db/content - Create content
    async fn handle_db_content_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Parse query params (needed for both service and legacy paths)
        let query_str = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect();

        let query = ContentQuery {
            content_type: params.get("content_type").cloned(),
            content_format: params.get("content_format").cloned(),
            tags: params
                .get("tags")
                .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                .unwrap_or_default(),
            search: params.get("search").cloned(),
            limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
            offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
        };

        // Use service layer if available
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    match services.content.list(&query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });
                            Ok(response::ok(&body))
                        }
                        Err(e) => Ok(response::error_response(e)),
                    }
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();

                    let input: db::content::CreateContentInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                    Ok(response::from_create_result(services.content.create(input)))
                }
                _ => Ok(response::method_not_allowed()),
            }
        } else {
            // Fallback to direct repository calls (legacy)
            match method {
                Method::GET => {
                    content_db.with_conn(|conn| {
                        match db::content::list_content(conn, &query) {
                            Ok(items) => {
                                let body = serde_json::json!({
                                    "items": items,
                                    "count": items.len(),
                                    "limit": query.limit,
                                    "offset": query.offset,
                                });

                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(body.to_string())))
                                    .unwrap())
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to list content");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();

                    let input: db::content::CreateContentInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                    content_db.with_conn_mut(|conn| {
                        match db::content::create_content(conn, input) {
                            Ok(content) => {
                                let body = serde_json::to_string(&content)
                                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                                Ok(Response::builder()
                                    .status(StatusCode::CREATED)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(body)))
                                    .unwrap())
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to create content");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                    .unwrap()),
            }
        }
    }

    /// POST /db/content/bulk - Bulk create content
    async fn handle_db_content_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;
        let body_bytes = body.to_bytes();

        let items: Vec<db::content::CreateContentInput> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let count = items.len();
        info!(count = count, "Bulk creating content");

        // Use service layer if available
        if let Some(ref services) = self.services {
            Ok(response::from_result(services.content.bulk_create(items)))
        } else {
            // Fallback to direct repository calls (legacy)
            content_db.with_conn_mut(|conn| {
                match db::content::bulk_create_content(conn, items) {
                    Ok(result) => {
                        info!(inserted = result.inserted, skipped = result.skipped, "Bulk content creation complete");

                        let body = serde_json::to_string(&result)
                            .map_err(|e| StorageError::Internal(e.to_string()))?;

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body)))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to bulk create content");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            })
        }
    }

    /// GET/DELETE /db/content/{id} - Get or delete content by ID
    async fn handle_db_content_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        content_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Use service layer if available
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    let result = services.content.get(content_id);
                    Ok(response::from_option(result, &format!("Content not found: {}", content_id)))
                }
                Method::DELETE => {
                    // Use cascade delete to also remove relationships
                    let result = services.content.delete_cascade(content_id);
                    Ok(response::from_delete_bool_result(result, &format!("Content not found: {}", content_id)))
                }
                _ => Ok(response::method_not_allowed()),
            }
        } else {
            // Fallback to direct repository calls (legacy)
            match method {
                Method::GET => {
                    content_db.with_conn(|conn| {
                        match db::content::get_content(conn, content_id) {
                            Ok(Some(content)) => {
                                let body = serde_json::to_string(&content)
                                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(body)))
                                    .unwrap())
                            }
                            Ok(None) => Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error": "Content not found: {}"}}"#,
                                    content_id
                                ))))
                                .unwrap()),
                            Err(e) => {
                                error!(error = %e, content_id = %content_id, "Failed to get content");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                Method::DELETE => {
                    content_db.with_conn_mut(|conn| {
                        match db::content::delete_content(conn, content_id) {
                            Ok(true) => Ok(Response::builder()
                                .status(StatusCode::NO_CONTENT)
                                .body(Full::new(Bytes::new()))
                                .unwrap()),
                            Ok(false) => Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error": "Content not found: {}"}}"#,
                                    content_id
                                ))))
                                .unwrap()),
                            Err(e) => {
                                error!(error = %e, content_id = %content_id, "Failed to delete content");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                    .unwrap()),
            }
        }
    }

    /// GET /db/paths - List paths, POST /db/paths - Create path
    async fn handle_db_paths_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let query_str = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect();

        let limit: u32 = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);
        let offset: u32 = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);

        // Use service layer if available
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    match services.path.list(limit, offset) {
                        Ok(paths) => {
                            let body = serde_json::json!({
                                "items": paths,
                                "count": paths.len(),
                                "limit": limit,
                                "offset": offset,
                            });
                            Ok(response::ok(&body))
                        }
                        Err(e) => Ok(response::error_response(e)),
                    }
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();

                    let input: db::paths::CreatePathInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                    Ok(response::from_create_result(services.path.create(input)))
                }
                _ => Ok(response::method_not_allowed()),
            }
        } else {
            // Fallback to direct repository calls (legacy)
            match method {
                Method::GET => {
                    content_db.with_conn(|conn| {
                        match db::paths::list_paths(conn, limit, offset) {
                            Ok(paths) => {
                                let body = serde_json::json!({
                                    "items": paths,
                                    "count": paths.len(),
                                    "limit": limit,
                                    "offset": offset,
                                });

                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(body.to_string())))
                                    .unwrap())
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to list paths");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();

                    let input: db::paths::CreatePathInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                    content_db.with_conn_mut(|conn| {
                        match db::paths::create_path(conn, input) {
                            Ok(path) => {
                                let body = serde_json::to_string(&path)
                                    .map_err(|e| StorageError::Internal(e.to_string()))?;

                                Ok(Response::builder()
                                    .status(StatusCode::CREATED)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(body)))
                                    .unwrap())
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to create path");
                                Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                    .unwrap())
                            }
                        }
                    })
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                    .unwrap()),
            }
        }
    }

    /// POST /db/paths/bulk - Bulk create paths
    async fn handle_db_paths_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            if method != Method::POST {
                return Ok(response::method_not_allowed());
            }

            let body = req.collect().await.map_err(|e| {
                StorageError::Internal(format!("Failed to read body: {}", e))
            })?;
            let body_bytes = body.to_bytes();

            let paths: Vec<db::paths::CreatePathInput> = serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let count = paths.len();
            info!(count = count, "Bulk creating paths via service");

            return Ok(response::from_result(services.path.bulk_create(paths)));
        }

        // Legacy fallback
        if method != Method::POST {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;
        let body_bytes = body.to_bytes();

        let paths: Vec<db::paths::CreatePathInput> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let count = paths.len();
        info!(count = count, "Bulk creating paths");

        content_db.with_conn_mut(|conn| {
            match db::paths::bulk_create_paths(conn, paths) {
                Ok(result) => {
                    info!(inserted = result.inserted, skipped = result.skipped, "Bulk path creation complete");

                    let body = serde_json::to_string(&result)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
                Err(e) => {
                    error!(error = %e, "Failed to bulk create paths");
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                        .unwrap())
                }
            }
        })
    }

    /// GET/DELETE /db/paths/{id} - Get or delete path by ID
    async fn handle_db_path_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        path_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    let result = services.path.get_with_steps(path_id);
                    return Ok(response::from_option(result, &format!("Path not found: {}", path_id)));
                }
                Method::DELETE => {
                    let result = services.path.delete(path_id);
                    return Ok(response::from_delete_bool_result(result, &format!("Path not found: {}", path_id)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                content_db.with_conn(|conn| {
                    // Return path with all steps
                    match db::paths::get_path_with_steps(conn, path_id) {
                        Ok(Some(path_with_steps)) => {
                            let body = serde_json::to_string(&path_with_steps)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Ok(None) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Path not found: {}"}}"#,
                                path_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, path_id = %path_id, "Failed to get path");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::DELETE => {
                content_db.with_conn_mut(|conn| {
                    match db::paths::delete_path(conn, path_id) {
                        Ok(true) => Ok(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .body(Full::new(Bytes::new()))
                            .unwrap()),
                        Ok(false) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Path not found: {}"}}"#,
                                path_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, path_id = %path_id, "Failed to delete path");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Relationship handlers
    // =========================================================================

    /// GET /db/relationships - List relationships, POST - Create relationship
    async fn handle_db_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            let query_str = req.uri().query().unwrap_or("");
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query_str.as_bytes())
                    .into_owned()
                    .collect();

            let query = db::relationships::RelationshipQuery {
                content_id: params.get("content_id").cloned(),
                direction: params.get("direction").cloned(),
                relationship_type: params.get("relationship_type").cloned(),
                limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
            };

            match method {
                Method::GET => {
                    match services.relationship.list(&query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });
                            return Ok(response::ok(&body));
                        }
                        Err(e) => return Ok(response::error_response(e)),
                    }
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();
                    let input: db::relationships::CreateRelationshipInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                    return Ok(response::from_create_result(services.relationship.create(input)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();

                let query = db::relationships::RelationshipQuery {
                    content_id: params.get("content_id").cloned(),
                    direction: params.get("direction").cloned(),
                    relationship_type: params.get("relationship_type").cloned(),
                    limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                    offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
                };

                content_db.with_conn(|conn| {
                    match db::relationships::list_relationships(conn, &query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body.to_string())))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to list relationships");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                let input: db::relationships::CreateRelationshipInput = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                content_db.with_conn_mut(|conn| {
                    match db::relationships::create_relationship(conn, input) {
                        Ok(rel) => {
                            let body = serde_json::to_string(&rel)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::CREATED)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to create relationship");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// POST /db/relationships/bulk - Bulk create relationships
    async fn handle_db_relationships_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            if method != Method::POST {
                return Ok(response::method_not_allowed());
            }

            let body = req.collect().await.map_err(|e| {
                StorageError::Internal(format!("Failed to read body: {}", e))
            })?;
            let body_bytes = body.to_bytes();

            let inputs: Vec<db::relationships::CreateRelationshipInput> = serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            return Ok(response::from_result(services.relationship.bulk_create(inputs)));
        }

        // Legacy fallback
        if method != Method::POST {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;
        let body_bytes = body.to_bytes();

        let inputs: Vec<db::relationships::CreateRelationshipInput> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        content_db.with_conn_mut(|conn| {
            match db::relationships::bulk_create_relationships(conn, inputs) {
                Ok(result) => {
                    let body = serde_json::to_string(&result)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
                Err(e) => {
                    error!(error = %e, "Failed to bulk create relationships");
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                        .unwrap())
                }
            }
        })
    }

    /// GET /db/relationships/graph/{content_id} - Get content graph
    async fn handle_db_content_graph(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            if method != Method::GET {
                return Ok(response::method_not_allowed());
            }

            let query_str = req.uri().query().unwrap_or("");
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query_str.as_bytes())
                    .into_owned()
                    .collect();

            let relationship_types: Option<Vec<String>> = params.get("types")
                .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

            return Ok(response::from_result(services.relationship.get_graph(content_id, relationship_types.as_deref())));
        }

        // Legacy fallback
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        let query_str = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect();

        let relationship_types: Option<Vec<String>> = params.get("types")
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect());

        content_db.with_conn(|conn| {
            match db::relationships::get_content_graph(conn, content_id, relationship_types.as_deref()) {
                Ok(graph) => {
                    let body = serde_json::to_string(&graph)
                        .map_err(|e| StorageError::Internal(e.to_string()))?;

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
                Err(e) => {
                    error!(error = %e, content_id = %content_id, "Failed to get content graph");
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                        .unwrap())
                }
            }
        })
    }

    /// GET/DELETE /db/relationships/{id} - Get or delete relationship by ID
    async fn handle_db_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        rel_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    let result = services.relationship.get(rel_id);
                    return Ok(response::from_option(result, &format!("Relationship not found: {}", rel_id)));
                }
                Method::DELETE => {
                    let result = services.relationship.delete(rel_id);
                    return Ok(response::from_delete_bool_result(result, &format!("Relationship not found: {}", rel_id)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                content_db.with_conn(|conn| {
                    match db::relationships::get_relationship(conn, rel_id) {
                        Ok(Some(rel)) => {
                            let body = serde_json::to_string(&rel)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Ok(None) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Relationship not found: {}"}}"#,
                                rel_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, rel_id = %rel_id, "Failed to get relationship");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::DELETE => {
                content_db.with_conn_mut(|conn| {
                    match db::relationships::delete_relationship(conn, rel_id) {
                        Ok(true) => Ok(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .body(Full::new(Bytes::new()))
                            .unwrap()),
                        Ok(false) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Relationship not found: {}"}}"#,
                                rel_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, rel_id = %rel_id, "Failed to delete relationship");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Knowledge Map handlers
    // =========================================================================

    /// GET /db/knowledge-maps - List knowledge maps, POST - Create knowledge map
    async fn handle_db_knowledge_maps_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            let query_str = req.uri().query().unwrap_or("");
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query_str.as_bytes())
                    .into_owned()
                    .collect();

            let query = db::knowledge_maps::KnowledgeMapQuery {
                owner_id: params.get("owner_id").cloned(),
                map_type: params.get("map_type").cloned(),
                subject_id: params.get("subject_id").cloned(),
                visibility: params.get("visibility").cloned(),
                limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
            };

            match method {
                Method::GET => {
                    match services.knowledge.list_knowledge_maps(&query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });
                            return Ok(response::ok(&body));
                        }
                        Err(e) => return Ok(response::error_response(e)),
                    }
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();
                    let input: db::knowledge_maps::CreateKnowledgeMapInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                    return Ok(response::from_create_result(services.knowledge.create_knowledge_map(input)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();

                let query = db::knowledge_maps::KnowledgeMapQuery {
                    owner_id: params.get("owner_id").cloned(),
                    map_type: params.get("map_type").cloned(),
                    subject_id: params.get("subject_id").cloned(),
                    visibility: params.get("visibility").cloned(),
                    limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                    offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
                };

                content_db.with_conn(|conn| {
                    match db::knowledge_maps::list_knowledge_maps(conn, &query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body.to_string())))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to list knowledge maps");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                let input: db::knowledge_maps::CreateKnowledgeMapInput = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                content_db.with_conn_mut(|conn| {
                    match db::knowledge_maps::create_knowledge_map(conn, input) {
                        Ok(map) => {
                            let body = serde_json::to_string(&map)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::CREATED)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to create knowledge map");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// GET/PUT/DELETE /db/knowledge-maps/{id} - Knowledge map by ID
    async fn handle_db_knowledge_map_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        map_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    let result = services.knowledge.get_knowledge_map(map_id);
                    return Ok(response::from_option(result, &format!("Knowledge map not found: {}", map_id)));
                }
                Method::PUT => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();
                    let input: db::knowledge_maps::CreateKnowledgeMapInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                    return Ok(response::from_result(services.knowledge.update_knowledge_map(map_id, input)));
                }
                Method::DELETE => {
                    let result = services.knowledge.delete_knowledge_map(map_id);
                    return Ok(response::from_delete_bool_result(result, &format!("Knowledge map not found: {}", map_id)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                content_db.with_conn(|conn| {
                    match db::knowledge_maps::get_knowledge_map(conn, map_id) {
                        Ok(Some(map)) => {
                            let body = serde_json::to_string(&map)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Ok(None) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Knowledge map not found: {}"}}"#,
                                map_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, map_id = %map_id, "Failed to get knowledge map");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::PUT => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                let input: db::knowledge_maps::CreateKnowledgeMapInput = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                content_db.with_conn_mut(|conn| {
                    match db::knowledge_maps::update_knowledge_map(conn, map_id, input) {
                        Ok(map) => {
                            let body = serde_json::to_string(&map)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Err(StorageError::NotFound(_)) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Knowledge map not found: {}"}}"#,
                                map_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, map_id = %map_id, "Failed to update knowledge map");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::DELETE => {
                content_db.with_conn_mut(|conn| {
                    match db::knowledge_maps::delete_knowledge_map(conn, map_id) {
                        Ok(true) => Ok(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .body(Full::new(Bytes::new()))
                            .unwrap()),
                        Ok(false) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Knowledge map not found: {}"}}"#,
                                map_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, map_id = %map_id, "Failed to delete knowledge map");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Path Extension handlers
    // =========================================================================

    /// GET /db/path-extensions - List path extensions, POST - Create path extension
    async fn handle_db_path_extensions_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            let query_str = req.uri().query().unwrap_or("");
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query_str.as_bytes())
                    .into_owned()
                    .collect();

            let query = db::path_extensions::PathExtensionQuery {
                base_path_id: params.get("base_path_id").cloned(),
                extended_by: params.get("extended_by").cloned(),
                visibility: params.get("visibility").cloned(),
                forked_from: params.get("forked_from").cloned(),
                limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
            };

            match method {
                Method::GET => {
                    match services.knowledge.list_path_extensions(&query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });
                            return Ok(response::ok(&body));
                        }
                        Err(e) => return Ok(response::error_response(e)),
                    }
                }
                Method::POST => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();
                    let input: db::path_extensions::CreatePathExtensionInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                    return Ok(response::from_create_result(services.knowledge.create_path_extension(input)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();

                let query = db::path_extensions::PathExtensionQuery {
                    base_path_id: params.get("base_path_id").cloned(),
                    extended_by: params.get("extended_by").cloned(),
                    visibility: params.get("visibility").cloned(),
                    forked_from: params.get("forked_from").cloned(),
                    limit: params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100),
                    offset: params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0),
                };

                content_db.with_conn(|conn| {
                    match db::path_extensions::list_path_extensions(conn, &query) {
                        Ok(items) => {
                            let body = serde_json::json!({
                                "items": items,
                                "count": items.len(),
                                "limit": query.limit,
                                "offset": query.offset,
                            });

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body.to_string())))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to list path extensions");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                let input: db::path_extensions::CreatePathExtensionInput = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                content_db.with_conn_mut(|conn| {
                    match db::path_extensions::create_path_extension(conn, input) {
                        Ok(ext) => {
                            let body = serde_json::to_string(&ext)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::CREATED)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to create path extension");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// GET/PUT/DELETE /db/path-extensions/{id} - Path extension by ID
    async fn handle_db_path_extension_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        ext_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Service-based handling
        if let Some(ref services) = self.services {
            match method {
                Method::GET => {
                    let result = services.knowledge.get_path_extension(ext_id);
                    return Ok(response::from_option(result, &format!("Path extension not found: {}", ext_id)));
                }
                Method::PUT => {
                    let body = req.collect().await.map_err(|e| {
                        StorageError::Internal(format!("Failed to read body: {}", e))
                    })?;
                    let body_bytes = body.to_bytes();
                    let input: db::path_extensions::CreatePathExtensionInput = serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                    return Ok(response::from_result(services.knowledge.update_path_extension(ext_id, input)));
                }
                Method::DELETE => {
                    let result = services.knowledge.delete_path_extension(ext_id);
                    return Ok(response::from_delete_bool_result(result, &format!("Path extension not found: {}", ext_id)));
                }
                _ => return Ok(response::method_not_allowed()),
            }
        }

        // Legacy fallback
        match method {
            Method::GET => {
                content_db.with_conn(|conn| {
                    match db::path_extensions::get_path_extension(conn, ext_id) {
                        Ok(Some(ext)) => {
                            let body = serde_json::to_string(&ext)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Ok(None) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Path extension not found: {}"}}"#,
                                ext_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, ext_id = %ext_id, "Failed to get path extension");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::PUT => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let body_bytes = body.to_bytes();

                let input: db::path_extensions::CreatePathExtensionInput = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                content_db.with_conn_mut(|conn| {
                    match db::path_extensions::update_path_extension(conn, ext_id, input) {
                        Ok(ext) => {
                            let body = serde_json::to_string(&ext)
                                .map_err(|e| StorageError::Internal(e.to_string()))?;

                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                        }
                        Err(StorageError::NotFound(_)) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Path extension not found: {}"}}"#,
                                ext_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, ext_id = %ext_id, "Failed to update path extension");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            Method::DELETE => {
                content_db.with_conn_mut(|conn| {
                    match db::path_extensions::delete_path_extension(conn, ext_id) {
                        Ok(true) => Ok(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .body(Full::new(Bytes::new()))
                            .unwrap()),
                        Ok(false) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Path extension not found: {}"}}"#,
                                ext_id
                            ))))
                            .unwrap()),
                        Err(e) => {
                            error!(error = %e, ext_id = %ext_id, "Failed to delete path extension");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                .unwrap())
                        }
                    }
                })
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // HTML5 App serving handlers
    // =========================================================================

    /// Handle HTML5 app file requests
    ///
    /// Route: GET /apps/{app_id}/{file_path}
    ///
    /// 1. Look up content by appId (contentFormat=html5-app)
    /// 2. Get blob_hash from content record
    /// 3. Fetch ZIP from blob store
    /// 4. Extract requested file
    /// 5. Return with appropriate Content-Type
    async fn handle_app_request(
        &self,
        path: &str,
        content_db: Arc<ContentDb>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use std::io::Read;
        use zip::ZipArchive;

        // Parse path: /apps/{app_id}/{file_path}
        let remainder = path.strip_prefix("/apps/").unwrap_or("");
        let (app_id, file_path) = match remainder.find('/') {
            Some(pos) => (&remainder[..pos], &remainder[pos + 1..]),
            None => (remainder, "index.html"),
        };

        if app_id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Missing app_id"}"#)))
                .unwrap());
        }

        // Validate file_path for path traversal
        if file_path.contains("..") || file_path.contains('\0') || file_path.starts_with('/') {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Invalid file path"}"#)))
                .unwrap());
        }

        debug!(app_id = %app_id, file_path = %file_path, "App file request");

        // Query content by appId (look for html5-app with matching content.appId)
        let content_record = content_db.with_conn(|conn| {
            // Query all html5-app content and find one with matching appId
            let query = ContentQuery {
                content_format: Some("html5-app".to_string()),
                limit: 100,
                ..Default::default()
            };

            match db::content::list_content(conn, &query) {
                Ok(items) => {
                    for item in items {
                        // Parse content_body field as JSON and check appId
                        if let Some(ref content_body) = item.content_body {
                            if let Ok(content_obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                                if let Some(content_app_id) = content_obj.get("appId").and_then(|v| v.as_str()) {
                                    if content_app_id == app_id {
                                        return Ok(Some(item));
                                    }
                                }
                            }
                        }
                    }
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        })?;

        let content = match content_record {
            Some(c) => c,
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "App not found: {}"}}"#,
                        app_id
                    ))))
                    .unwrap());
            }
        };

        // Get blob_hash from content record
        let blob_hash = match &content.blob_hash {
            Some(hash) if !hash.is_empty() => hash.clone(),
            _ => {
                // Try metadata.blobHash or metadata.blob_hash
                let metadata: serde_json::Value = content.metadata_json.as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::json!({}));

                metadata.get("blobHash")
                    .or_else(|| metadata.get("blob_hash"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            }
        };

        if blob_hash.is_empty() {
            // Get fallback URL if available
            let content_obj: serde_json::Value = content.content_body.as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::json!({}));
            let fallback = content_obj.get("fallbackUrl").and_then(|v| v.as_str());

            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    if let Some(url) = fallback {
                        format!(r#"{{"error": "App ZIP not available", "fallback": "{}"}}"#, url)
                    } else {
                        r#"{"error": "App ZIP not available (no blob_hash)"}"#.to_string()
                    }
                )))
                .unwrap());
        }

        debug!(app_id = %app_id, blob_hash = %blob_hash, "Found blob hash");

        // Fetch ZIP from blob store
        let zip_data = match self.blob_store.get(&blob_hash).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "App ZIP blob not found: {}"}}"#,
                        blob_hash
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        debug!(app_id = %app_id, zip_size = zip_data.len(), "Fetched ZIP blob");

        // Extract file from ZIP
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Invalid ZIP archive");
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid ZIP archive: {}"}}"#,
                        e
                    ))))
                    .unwrap());
            }
        };

        // Normalize file path
        let normalized_path = file_path.trim_start_matches('/');

        // Find the file in the archive
        // First, try to find an exact match or a suffix match in the file list
        let file_index = {
            let mut exact_idx = None;
            let mut suffix_idx = None;

            for i in 0..archive.len() {
                if let Ok(f) = archive.by_index(i) {
                    let name = f.name();
                    if name == normalized_path {
                        exact_idx = Some(i);
                        break;
                    }
                    if suffix_idx.is_none() &&
                       (name.ends_with(normalized_path) || name.ends_with(&format!("/{}", normalized_path))) {
                        suffix_idx = Some(i);
                    }
                }
            }

            exact_idx.or(suffix_idx)
        };

        let file_index = match file_index {
            Some(idx) => idx,
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "File not found in app: {}"}}"#,
                        normalized_path
                    ))))
                    .unwrap());
            }
        };

        let mut file = match archive.by_index(file_index) {
            Ok(f) => f,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read file from ZIP: {}"}}"#,
                        e
                    ))))
                    .unwrap());
            }
        };

        // Read file contents
        let mut contents = Vec::new();
        if let Err(e) = file.read_to_end(&mut contents) {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to read file contents: {}"}}"#,
                    e
                ))))
                .unwrap());
        }

        // Determine content type from file extension
        let content_type = Self::get_mime_type(file_path);

        info!(
            app_id = %app_id,
            file_path = %file_path,
            content_type = %content_type,
            size = contents.len(),
            "Serving app file"
        );

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, contents.len())
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .header("X-App-Id", app_id)
            .body(Full::new(Bytes::from(contents)))
            .unwrap())
    }

    // ========================================================================
    // Diesel-based Entity Handlers
    // ========================================================================

    /// Helper to get a Diesel connection from the pool
    fn get_diesel_conn(&self) -> Result<crate::db::PooledConn, StorageError> {
        self.db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Diesel pool not configured".into()))?
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// GET/POST /db/human-relationships - List or create human relationships
    async fn handle_human_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: human_relationships::HumanRelationshipQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match human_relationships::list_human_relationships(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let input: human_relationships::CreateHumanRelationshipInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match human_relationships::create_human_relationship(&mut conn, ctx, input) {
                    Ok(rel) => Ok(response::created(&rel)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/human-relationships/{id}
    async fn handle_human_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match human_relationships::get_human_relationship(&mut conn, ctx, id) {
                    Ok(Some(rel)) => Ok(response::ok(&rel)),
                    Ok(None) => Ok(response::not_found(&format!("Human relationship {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match human_relationships::delete_human_relationship(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!("Human relationship {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/human-relationships/{id}/consent - Update consent on a relationship
    async fn handle_human_relationship_consent(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        #[derive(Deserialize)]
        struct ConsentInput {
            party_id: String,
            consent: bool,
        }

        let input: ConsentInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let consent_update = human_relationships::ConsentUpdate {
            consent_given: input.consent,
        };

        match human_relationships::update_consent(&mut conn, ctx, id, &input.party_id, &consent_update) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/human-relationships/{id}/custody - Update custody settings on a relationship
    async fn handle_human_relationship_custody(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        #[derive(Deserialize)]
        struct CustodyInput {
            party_id: String,
            enabled: bool,
            #[serde(default)]
            auto_custody: Option<bool>,
            #[serde(default)]
            emergency_access: Option<bool>,
        }

        let input: CustodyInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let custody_update = human_relationships::CustodyUpdate {
            custody_enabled: input.enabled,
            auto_custody_enabled: input.auto_custody,
            emergency_access_enabled: input.emergency_access,
        };

        match human_relationships::update_custody(&mut conn, ctx, id, &input.party_id, &custody_update) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/presences - List or create contributor presences
    async fn handle_presences_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: contributor_presences::ContributorPresenceQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match contributor_presences::list_contributor_presences(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let input: contributor_presences::CreateContributorPresenceInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match contributor_presences::create_contributor_presence(&mut conn, ctx, input) {
                    Ok(presence) => Ok(response::created(&presence)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/presences/{id}
    async fn handle_presence_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match contributor_presences::get_contributor_presence(&mut conn, ctx, id) {
                    Ok(Some(presence)) => Ok(response::ok(&presence)),
                    Ok(None) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match contributor_presences::delete_contributor_presence(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/presences/{id}/stewardship - Initiate stewardship of a presence
    async fn handle_presence_stewardship(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        let input: contributor_presences::InitiateStewardshipInput =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match contributor_presences::initiate_stewardship(&mut conn, ctx, id, &input) {
            Ok(presence) => Ok(response::ok(&presence)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/claim - Initiate claim of a presence
    async fn handle_presence_claim(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        let input: contributor_presences::InitiateClaimInput =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match contributor_presences::initiate_claim(&mut conn, ctx, id, &input) {
            Ok(presence) => Ok(response::ok(&presence)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/verify-claim - Verify a claim (sets state to claimed)
    async fn handle_presence_verify_claim(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match contributor_presences::verify_claim(&mut conn, ctx, id) {
            Ok(presence) => Ok(response::ok(&presence)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/events - List or record economic events
    async fn handle_events_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: economic_events::EconomicEventQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match economic_events::list_economic_events(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let input: economic_events::CreateEconomicEventInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match economic_events::record_event(&mut conn, ctx, input) {
                    Ok(event) => Ok(response::created(&event)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/events/{id}
    async fn handle_event_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match economic_events::get_economic_event(&mut conn, ctx, id) {
                    Ok(Some(event)) => Ok(response::ok(&event)),
                    Ok(None) => Ok(response::not_found(&format!("Event {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/POST /db/mastery - List or create content mastery records
    async fn handle_mastery_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: content_mastery::MasteryQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match content_mastery::list_mastery(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req.collect().await.map_err(|e| {
                    StorageError::Internal(format!("Failed to read body: {}", e))
                })?;
                let input: content_mastery::CreateMasteryInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match content_mastery::upsert_mastery(&mut conn, ctx, input) {
                    Ok(mastery) => Ok(response::created(&mastery)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/{id}
    async fn handle_mastery_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match content_mastery::get_mastery(&mut conn, ctx, id) {
                    Ok(Some(mastery)) => Ok(response::ok(&mastery)),
                    Ok(None) => Ok(response::not_found(&format!("Mastery record {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/human/{human_id} - Get all mastery records for a human
    async fn handle_mastery_for_human(
        &self,
        _req: Request<Incoming>,
        method: Method,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match content_mastery::get_mastery_for_human(&mut conn, ctx, human_id) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                            "human_id": human_id,
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Bulk Endpoints (for seeding/import operations)
    // =========================================================================

    /// POST /db/presences/bulk - Bulk create contributor presences
    async fn handle_presences_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        let inputs: Vec<contributor_presences::CreateContributorPresenceInput> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match contributor_presences::bulk_create_presences(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/events/bulk - Bulk record economic events
    async fn handle_events_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        let inputs: Vec<economic_events::CreateEconomicEventInput> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match economic_events::bulk_record_events(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/mastery/bulk - Bulk create/update mastery records
    async fn handle_mastery_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req.collect().await.map_err(|e| {
            StorageError::Internal(format!("Failed to read body: {}", e))
        })?;

        let inputs: Vec<content_mastery::CreateMasteryInput> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match content_mastery::bulk_upsert_mastery(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    // =========================================================================
    // Stewardship Allocation handlers
    // =========================================================================

    /// GET/POST /db/allocations - List or create stewardship allocations
    async fn handle_allocations_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                // Parse query params
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();

                let query = stewardship_allocations::AllocationQuery {
                    content_id: params.get("content_id").cloned(),
                    steward_presence_id: params.get("steward_presence_id").cloned(),
                    governance_state: params.get("governance_state").cloned(),
                    active_only: params.get("active_only").map(|s| s == "true"),
                    limit: params.get("limit").and_then(|s| s.parse().ok()),
                    offset: params.get("offset").and_then(|s| s.parse().ok()),
                };

                match stewardship_allocations::list_allocations(&mut conn, app_ctx, &query) {
                    Ok(allocations) => Ok(response::ok(&allocations)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req.collect().await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input: stewardship_allocations::CreateAllocationInput = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

                match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                    Ok(allocation) => Ok(response::created(&allocation)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/{id}, DELETE /db/allocations/{id}
    async fn handle_allocation_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match stewardship_allocations::get_allocation_by_id(&mut conn, app_ctx, id) {
                    Ok(allocation) => Ok(response::ok(&allocation)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::PUT => {
                let body = req.collect().await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input: stewardship_allocations::UpdateAllocationInput = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

                match stewardship_allocations::update_allocation(&mut conn, app_ctx, id, &input) {
                    Ok(allocation) => Ok(response::ok(&allocation)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match stewardship_allocations::delete_allocation(&mut conn, app_ctx, id) {
                    Ok(()) => Ok(response::no_content()),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/content/{content_id} - Get content stewardship aggregate
    async fn handle_allocations_for_content(
        &self,
        _req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_content_stewardship(&mut conn, app_ctx, content_id) {
            Ok(stewardship) => Ok(response::ok(&stewardship)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/allocations/steward/{steward_id} - Get allocations for a steward
    async fn handle_allocations_for_steward(
        &self,
        _req: Request<Incoming>,
        method: Method,
        steward_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_allocations_for_steward(&mut conn, app_ctx, steward_id) {
            Ok(allocations) => Ok(response::ok(&allocations)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/dispute - File a dispute on an allocation
    async fn handle_allocation_dispute(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct DisputeInput {
            dispute_id: String,
            disputed_by: String,
            reason: String,
        }

        let body = req.collect().await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: DisputeInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::file_dispute(&mut conn, app_ctx, allocation_id, &input.dispute_id, &input.disputed_by, &input.reason) {
            Ok(allocation) => Ok(response::ok(&allocation)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/resolve - Resolve a dispute (Elohim ratification)
    async fn handle_allocation_resolve(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ResolveInput {
            ratifier_id: String,
            new_state: String,
        }

        let body = req.collect().await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: ResolveInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::resolve_dispute(&mut conn, app_ctx, allocation_id, &input.ratifier_id, &input.new_state) {
            Ok(allocation) => Ok(response::ok(&allocation)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/bulk - Bulk create stewardship allocations
    async fn handle_allocations_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self.db_pool.as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool.get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        let body = req.collect().await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let inputs: Vec<stewardship_allocations::CreateAllocationInput> = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let mut created = 0;
        let mut failed = 0;
        let mut errors: Vec<String> = Vec::new();

        for input in inputs {
            match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                Ok(_) => created += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", input.content_id, e));
                }
            }
        }

        #[derive(serde::Serialize)]
        struct BulkResult {
            created: usize,
            failed: usize,
            errors: Vec<String>,
        }

        Ok(response::ok(&BulkResult { created, failed, errors }))
    }

    /// Get MIME type for a file path based on extension
    fn get_mime_type(path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("html") | Some("htm") => "text/html; charset=utf-8",
            Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            Some("eot") => "application/vnd.ms-fontobject",
            Some("wasm") => "application/wasm",
            Some("mp3") => "audio/mpeg",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("ogg") => "audio/ogg",
            Some("wav") => "audio/wav",
            Some("txt") => "text/plain; charset=utf-8",
            Some("xml") => "application/xml",
            Some("pdf") => "application/pdf",
            Some("zip") => "application/zip",
            Some("map") => "application/json", // source maps
            _ => "application/octet-stream",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_format() {
        let hash = BlobStore::compute_hash(b"test data");
        assert!(hash.starts_with("sha256-"));
        assert_eq!(hash.len(), 7 + 64); // "sha256-" + 64 hex chars
    }
}
