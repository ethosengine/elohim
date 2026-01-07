//! HTTP client for elohim-storage sync API

use crate::error::{Result, StorageError};
use crate::types::*;
use base64::Engine;
use reqwest::{header, Client, StatusCode};
use std::time::Duration;

/// HTTP client for elohim-storage sync API
///
/// # Example
///
/// ```rust,no_run
/// use elohim_storage_client::{StorageClient, StorageConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StorageClient::new(StorageConfig {
///     base_url: "http://localhost:8080".into(),
///     app_id: "lamad".into(),
///     ..Default::default()
/// });
///
/// // List documents
/// let response = client.list_documents(Default::default()).await?;
///
/// // Get document heads
/// let heads = client.get_heads("graph:my-doc").await?;
/// # Ok(())
/// # }
/// ```
pub struct StorageClient {
    config: StorageConfig,
    client: Client,
}

impl StorageClient {
    /// Create a new storage client
    pub fn new(config: StorageConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        if let Some(ref api_key) = config.api_key {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .expect("Invalid API key"),
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, client }
    }

    // ==================== Sync API ====================

    /// List documents for this app
    pub async fn list_documents(&self, options: ListOptions) -> Result<ListDocumentsResponse> {
        let mut url = format!(
            "{}/sync/v1/{}/docs",
            self.config.base_url, self.config.app_id
        );

        let mut params = Vec::new();
        if let Some(ref prefix) = options.prefix {
            params.push(format!("prefix={}", prefix));
        }
        if let Some(offset) = options.offset {
            params.push(format!("offset={}", offset));
        }
        if let Some(limit) = options.limit {
            params.push(format!("limit={}", limit));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Get document info
    pub async fn get_document(&self, doc_id: &str) -> Result<GetDocumentResponse> {
        let url = format!(
            "{}/sync/v1/{}/docs/{}",
            self.config.base_url,
            self.config.app_id,
            urlencoding::encode(doc_id)
        );

        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Get current heads for a document
    pub async fn get_heads(&self, doc_id: &str) -> Result<GetHeadsResponse> {
        let url = format!(
            "{}/sync/v1/{}/docs/{}/heads",
            self.config.base_url,
            self.config.app_id,
            urlencoding::encode(doc_id)
        );

        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Get changes since given heads
    pub async fn get_changes_since(
        &self,
        doc_id: &str,
        have_heads: &[String],
    ) -> Result<GetChangesResponse> {
        let mut url = format!(
            "{}/sync/v1/{}/docs/{}/changes",
            self.config.base_url,
            self.config.app_id,
            urlencoding::encode(doc_id)
        );

        if !have_heads.is_empty() {
            url.push_str("?have=");
            url.push_str(&have_heads.join(","));
        }

        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Apply changes to a document
    pub async fn apply_changes(
        &self,
        doc_id: &str,
        changes: &[Vec<u8>],
    ) -> Result<ApplyChangesResponse> {
        let url = format!(
            "{}/sync/v1/{}/docs/{}/changes",
            self.config.base_url,
            self.config.app_id,
            urlencoding::encode(doc_id)
        );

        // Encode changes as base64
        let changes_b64: Vec<String> = changes
            .iter()
            .map(|c| base64::engine::general_purpose::STANDARD.encode(c))
            .collect();

        let body = ApplyChangesRequest {
            changes: changes_b64,
        };

        let response = self
            .client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get document count for this app
    pub async fn count_documents(&self) -> Result<u64> {
        let response = self
            .list_documents(ListOptions {
                limit: Some(0),
                ..Default::default()
            })
            .await?;
        Ok(response.total)
    }

    // ==================== Blob API ====================

    /// Store a blob
    pub async fn put_blob(&self, data: &[u8], mime_type: &str) -> Result<BlobManifest> {
        let url = format!("{}/blob/", self.config.base_url);

        let response = self
            .client
            .put(&url)
            .header(header::CONTENT_TYPE, mime_type)
            .body(data.to_vec())
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get a blob by hash or CID
    pub async fn get_blob(&self, hash_or_cid: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/blob/{}",
            self.config.base_url,
            urlencoding::encode(hash_or_cid)
        );

        let response = self.client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(StorageError::NotFound(hash_or_cid.to_string()));
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::Server {
                status,
                message: body,
            });
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Check if a blob exists
    pub async fn blob_exists(&self, hash_or_cid: &str) -> Result<bool> {
        let url = format!(
            "{}/shard/{}",
            self.config.base_url,
            urlencoding::encode(hash_or_cid)
        );

        let response = self.client.head(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// Get blob manifest
    pub async fn get_manifest(&self, hash_or_cid: &str) -> Result<BlobManifest> {
        let url = format!(
            "{}/manifest/{}",
            self.config.base_url,
            urlencoding::encode(hash_or_cid)
        );

        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    // ==================== Helper Methods ====================

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        if response.status() == StatusCode::NOT_FOUND {
            return Err(StorageError::NotFound("Resource not found".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::Server {
                status,
                message: body,
            });
        }

        let body = response.json().await?;
        Ok(body)
    }
}
