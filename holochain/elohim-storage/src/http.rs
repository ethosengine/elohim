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
use crate::db::{self, ContentDb, ContentQuery};
use crate::error::StorageError;
use crate::import_api::ImportApi;
use crate::progress_hub::ProgressHub;
use crate::progress_ws;
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
    async fn handle_db_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
        content_db: Arc<ContentDb>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Strip /db/ prefix
        let sub_path = path.strip_prefix("/db/").unwrap_or("");

        // Route to specific handlers
        if sub_path == "stats" {
            return self.handle_db_stats(method, &content_db).await;
        }

        if sub_path == "content" {
            return self.handle_db_content_list(req, method, &content_db).await;
        }

        if sub_path == "content/bulk" {
            return self.handle_db_content_bulk(req, method, &content_db).await;
        }

        if let Some(content_id) = sub_path.strip_prefix("content/") {
            return self.handle_db_content_by_id(req, method, content_id, &content_db).await;
        }

        if sub_path == "paths" {
            return self.handle_db_paths_list(req, method, &content_db).await;
        }

        if sub_path == "paths/bulk" {
            return self.handle_db_paths_bulk(req, method, &content_db).await;
        }

        if let Some(path_id) = sub_path.strip_prefix("paths/") {
            return self.handle_db_path_by_id(req, method, path_id, &content_db).await;
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
        match method {
            Method::GET => {
                // Parse query params
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
                // Parse body
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

    /// POST /db/content/bulk - Bulk create content
    async fn handle_db_content_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
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

        let items: Vec<db::content::CreateContentInput> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let count = items.len();
        info!(count = count, "Bulk creating content");

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

    /// GET/DELETE /db/content/{id} - Get or delete content by ID
    async fn handle_db_content_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        content_id: &str,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
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

    /// GET /db/paths - List paths, POST /db/paths - Create path
    async fn handle_db_paths_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();

                let limit: u32 = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);
                let offset: u32 = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);

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

    /// POST /db/paths/bulk - Bulk create paths
    async fn handle_db_paths_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_db: &ContentDb,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
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
