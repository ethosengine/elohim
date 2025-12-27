//! HTML5 App bundle serving routes
//!
//! This module provides routing for serving HTML5 applications from zip bundles
//! stored by content publishers in the DHT.

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Method, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for app serving
#[derive(Debug, Clone)]
pub enum AppError {
    /// App ID not found in content registry
    AppNotFound(String),
    /// Requested file not found within the app bundle
    FileNotFound(String),
    /// Failed to fetch bundle from publisher
    FetchFailed(String),
    /// Failed to extract zip bundle
    ExtractionFailed(String),
    /// HTTP method not allowed
    MethodNotAllowed,
    /// Internal server error
    InternalError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::AppNotFound(id) => write!(f, "App not found: {}", id),
            AppError::FileNotFound(path) => write!(f, "File not found: {}", path),
            AppError::FetchFailed(msg) => write!(f, "Fetch failed: {}", msg),
            AppError::ExtractionFailed(msg) => write!(f, "Extraction failed: {}", msg),
            AppError::MethodNotAllowed => write!(f, "Method not allowed"),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

/// Metadata about an HTML5 application
#[derive(Debug, Clone)]
pub struct AppMetadata {
    /// Unique app identifier
    pub app_id: String,
    /// Content hash of the zip bundle
    pub bundle_hash: String,
    /// Optional version string
    pub version: Option<String>,
}

/// Cache entry for extracted app files
struct CacheEntry {
    /// Extracted files: path -> content
    files: HashMap<String, Vec<u8>>,
    /// Bundle hash this was extracted from
    bundle_hash: String,
    /// Approximate size in bytes
    size_bytes: usize,
}

/// In-memory cache for extracted HTML5 app bundles
pub struct AppCache {
    /// Maximum cache size in bytes
    max_bytes: usize,
    /// Cached app bundles: app_id -> extracted files
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Current cache size in bytes
    current_bytes: RwLock<usize>,
}

impl AppCache {
    /// Create a new app cache with the specified maximum size in megabytes
    pub fn new(max_mb: usize) -> Self {
        Self {
            max_bytes: max_mb * 1024 * 1024,
            entries: RwLock::new(HashMap::new()),
            current_bytes: RwLock::new(0),
        }
    }

    /// Get a file from the cache
    pub async fn get(&self, app_id: &str, path: &str) -> Option<Vec<u8>> {
        let entries = self.entries.read().await;
        entries
            .get(app_id)
            .and_then(|entry| entry.files.get(path).cloned())
    }

    /// Check if an app is cached with the given bundle hash
    pub async fn is_cached(&self, app_id: &str, bundle_hash: &str) -> bool {
        let entries = self.entries.read().await;
        entries
            .get(app_id)
            .map(|e| e.bundle_hash == bundle_hash)
            .unwrap_or(false)
    }

    /// Insert extracted files into the cache
    pub async fn insert(&self, app_id: String, bundle_hash: String, files: HashMap<String, Vec<u8>>) {
        let size: usize = files.values().map(|v| v.len()).sum();

        // Evict if necessary
        self.evict_if_needed(size).await;

        let entry = CacheEntry {
            files,
            bundle_hash,
            size_bytes: size,
        };

        {
            let mut entries = self.entries.write().await;
            // Remove old entry size if exists
            if let Some(old) = entries.remove(&app_id) {
                let mut current = self.current_bytes.write().await;
                *current = current.saturating_sub(old.size_bytes);
            }
            entries.insert(app_id, entry);
        }

        {
            let mut current = self.current_bytes.write().await;
            *current += size;
        }
    }

    /// Evict entries to make room for new data
    async fn evict_if_needed(&self, needed: usize) {
        let current = *self.current_bytes.read().await;
        if current + needed <= self.max_bytes {
            return;
        }

        // Simple eviction: remove oldest entries until we have space
        // A more sophisticated LRU implementation could be added later
        let mut entries = self.entries.write().await;
        let mut current_bytes = self.current_bytes.write().await;

        while *current_bytes + needed > self.max_bytes && !entries.is_empty() {
            // Remove first entry (not truly LRU, but simple)
            if let Some(key) = entries.keys().next().cloned() {
                if let Some(entry) = entries.remove(&key) {
                    *current_bytes = current_bytes.saturating_sub(entry.size_bytes);
                }
            }
        }
    }

    /// List files in a cached app bundle
    pub async fn list_files(&self, app_id: &str) -> Option<Vec<String>> {
        let entries = self.entries.read().await;
        entries
            .get(app_id)
            .map(|e| e.files.keys().cloned().collect())
    }
}

/// Trait for fetching content from publishers
#[async_trait]
pub trait AsyncPublisherFetch: Send + Sync {
    /// Fetch content by hash from a publisher
    async fn fetch(&self, hash: &str) -> Result<Vec<u8>, String>;
}

/// Handle an app request
///
/// Routes GET/HEAD requests for /apps/{app-id}/{path} to serve files from
/// HTML5 application bundles.
pub async fn handle_app_request<F, P>(
    req: Request<Incoming>,
    cache: Arc<AppCache>,
    content_lookup: F,
    publisher: P,
) -> Result<Response<Full<Bytes>>, AppError>
where
    F: Fn(&str) -> Option<AppMetadata>,
    P: AsyncPublisherFetch,
{
    // Only allow GET and HEAD
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return Err(AppError::MethodNotAllowed);
    }

    let path = req.uri().path();

    // Parse /apps/{app-id}/{file-path}
    let path = path.strip_prefix("/apps/").ok_or_else(|| {
        AppError::InternalError("Invalid apps path".to_string())
    })?;

    let (app_id, file_path) = match path.find('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => (path, "index.html"), // Default to index.html
    };

    if app_id.is_empty() {
        return Err(AppError::AppNotFound("empty app ID".to_string()));
    }

    // Normalize file path
    let file_path = if file_path.is_empty() {
        "index.html"
    } else {
        file_path
    };

    // Look up app metadata
    let metadata = content_lookup(app_id)
        .ok_or_else(|| AppError::AppNotFound(app_id.to_string()))?;

    // Check cache
    if !cache.is_cached(app_id, &metadata.bundle_hash).await {
        // Fetch and extract bundle
        let bundle_data = publisher
            .fetch(&metadata.bundle_hash)
            .await
            .map_err(AppError::FetchFailed)?;

        // Extract zip
        let files = extract_zip(&bundle_data)
            .map_err(|e| AppError::ExtractionFailed(e.to_string()))?;

        // Cache extracted files
        cache
            .insert(app_id.to_string(), metadata.bundle_hash.clone(), files)
            .await;
    }

    // Get file from cache
    let content = cache
        .get(app_id, file_path)
        .await
        .ok_or_else(|| AppError::FileNotFound(file_path.to_string()))?;

    // Determine content type
    let content_type = guess_content_type(file_path);

    // Build response
    let body = if req.method() == Method::HEAD {
        Bytes::new()
    } else {
        Bytes::from(content)
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(body))
        .unwrap())
}

/// Extract files from a zip bundle
fn extract_zip(data: &[u8]) -> Result<HashMap<String, Vec<u8>>, std::io::Error> {
    use std::io::{Cursor, Read};

    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut files = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            let name = file.name().to_string();
            // Normalize path: remove leading slashes and handle nested dirs
            let normalized = name.trim_start_matches('/').to_string();
            if !normalized.is_empty() {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                files.insert(normalized, contents);
            }
        }
    }

    Ok(files)
}

/// Guess MIME type from file extension
fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("webp") => "image/webp",
        Some("webm") => "video/webm",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("wasm") => "application/wasm",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(guess_content_type("style.css"), "text/css; charset=utf-8");
        assert_eq!(guess_content_type("app.js"), "application/javascript; charset=utf-8");
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(guess_content_type("image.png"), "image/png");
        assert_eq!(guess_content_type("unknown"), "application/octet-stream");
    }

    #[tokio::test]
    async fn test_app_cache() {
        let cache = AppCache::new(1); // 1MB

        let mut files = HashMap::new();
        files.insert("index.html".to_string(), b"<html></html>".to_vec());
        files.insert("style.css".to_string(), b"body {}".to_vec());

        cache.insert("test-app".to_string(), "hash123".to_string(), files).await;

        assert!(cache.is_cached("test-app", "hash123").await);
        assert!(!cache.is_cached("test-app", "different-hash").await);
        assert!(!cache.is_cached("other-app", "hash123").await);

        let content = cache.get("test-app", "index.html").await;
        assert_eq!(content, Some(b"<html></html>".to_vec()));

        let missing = cache.get("test-app", "missing.js").await;
        assert!(missing.is_none());
    }
}
