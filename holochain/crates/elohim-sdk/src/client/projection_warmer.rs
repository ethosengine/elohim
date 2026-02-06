//! Projection cache warmer
//!
//! Warms the Doorway projection cache from P2P nodes.
//! In Phase A (no DHT), this uses HTTP push instead of DHT signals.

use crate::error::{Result, SdkError};
use crate::traits::ContentReadable;
use serde::{Deserialize, Serialize};

/// Request to warm the projection cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmRequest {
    /// Content type (e.g., "content", "path")
    #[serde(rename = "type")]
    pub content_type: String,
    /// Content ID
    pub id: String,
    /// The content data
    pub data: serde_json::Value,
    /// TTL in seconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

/// Response from warming request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmResponse {
    /// Whether the content was cached
    pub cached: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Warms the Doorway projection cache from P2P nodes
///
/// In Phase A (no DHT), projection warming happens via HTTP push
/// instead of DHT signals. P2P nodes push their content to doorways
/// to make it available to web2.0 users.
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::ProjectionWarmer;
///
/// let warmer = ProjectionWarmer::new("https://doorway.example.com");
///
/// // Warm a single content item
/// warmer.warm(&content).await?;
///
/// // Warm multiple items
/// warmer.warm_batch(&contents).await?;
/// ```
pub struct ProjectionWarmer {
    /// Doorway URL
    doorway_url: String,
    /// API key for authentication
    api_key: Option<String>,
    /// HTTP client
    http_client: reqwest::Client,
}

impl ProjectionWarmer {
    /// Create a new projection warmer
    pub fn new(doorway_url: impl Into<String>) -> Self {
        Self {
            doorway_url: doorway_url.into(),
            api_key: None,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a warmer with API key authentication
    pub fn with_api_key(doorway_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            doorway_url: doorway_url.into(),
            api_key: Some(api_key.into()),
            http_client: reqwest::Client::new(),
        }
    }

    /// Warm a single content item in the projection cache
    pub async fn warm<T: ContentReadable>(&self, content: &T) -> Result<WarmResponse> {
        let request = WarmRequest {
            content_type: T::content_type().to_string(),
            id: content.content_id().to_string(),
            data: serde_json::to_value(content)?,
            ttl_secs: Some(T::cache_ttl()),
        };

        self.send_warm_request(&request).await
    }

    /// Warm multiple content items
    pub async fn warm_batch<T: ContentReadable>(&self, contents: &[T]) -> Result<Vec<WarmResponse>> {
        let mut responses = Vec::new();

        for content in contents {
            match self.warm(content).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    tracing::error!("Failed to warm content {}: {}", content.content_id(), e);
                    responses.push(WarmResponse {
                        cached: false,
                        message: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(responses)
    }

    /// Warm from raw JSON data
    pub async fn warm_raw(
        &self,
        content_type: &str,
        id: &str,
        data: serde_json::Value,
        ttl_secs: Option<u64>,
    ) -> Result<WarmResponse> {
        let request = WarmRequest {
            content_type: content_type.to_string(),
            id: id.to_string(),
            data,
            ttl_secs,
        };

        self.send_warm_request(&request).await
    }

    /// Invalidate a content item in the projection cache
    pub async fn invalidate(&self, content_type: &str, id: &str) -> Result<()> {
        let url = format!("{}/projection/invalidate", self.doorway_url);

        let body = serde_json::json!({
            "type": content_type,
            "id": id,
        });

        let mut request = self.http_client.post(&url).json(&body);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::Network(format!(
                "Failed to invalidate projection: HTTP {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    async fn send_warm_request(&self, request: &WarmRequest) -> Result<WarmResponse> {
        let url = format!("{}/projection/warm", self.doorway_url);

        let mut http_request = self.http_client.post(&url).json(request);
        if let Some(ref key) = self.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", key));
        }

        let response = http_request.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::Network(format!(
                "Failed to warm projection: HTTP {} - {}",
                status, body
            )));
        }

        let warm_response: WarmResponse = response.json().await?;
        Ok(warm_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warm_request_serialization() {
        let request = WarmRequest {
            content_type: "content".to_string(),
            id: "manifesto".to_string(),
            data: serde_json::json!({"title": "Manifesto"}),
            ttl_secs: Some(3600),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"content\""));
        assert!(json.contains("\"id\":\"manifesto\""));
    }
}
