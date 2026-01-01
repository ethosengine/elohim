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
use crate::error::StorageError;
use crate::import_api::ImportApi;
use crate::progress_hub::ProgressHub;
use crate::progress_ws;
use crate::sharding::{ShardEncoder, ShardManifest};
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

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
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

        // Verify hash
        let computed_hash = BlobStore::compute_hash(&data);
        if !expected_hash.is_empty() && computed_hash != expected_hash {
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

        // Verify hash if provided
        let computed_hash = BlobStore::compute_hash(&data);
        if !expected_hash.is_empty() && computed_hash != expected_hash {
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
