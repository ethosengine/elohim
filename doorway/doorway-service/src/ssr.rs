//! SSR-specific glue between elohim-render and doorway's resolver.
//!
//! ## ResolverFetcher
//!
//! `ResolverFetcher` implements `elohim_render::DataFetcher` by forwarding fetch
//! requests to the configured elohim-storage endpoint via `reqwest`. This lets
//! Angular SSR components call `fetch()` against the local storage sidecar during
//! server-side rendering without needing network access to the public doorway URL.
//!
//! The backing method is a direct `reqwest::Client` GET against `storage_base_url`
//! because `DoorwayResolver::resolve(content_type, id)` is scoped to
//! projection-cache / conductor lookups — it has no concept of arbitrary HTTP paths.
//!
//! TODO(Task 14): Once the tiered cache write-on-fetch is wired, route SSR fetches
//! through the projection cache so responses are cached and replayed on cache hit.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use elohim_render::{DataFetcher, FetchRequest, FetchResponse, RenderError, Result};

// =============================================================================
// ResolverFetcher
// =============================================================================

/// Forwards Angular SSR `fetch()` calls to the local elohim-storage sidecar.
///
/// Constructed with the storage base URL (e.g. `http://localhost:8090`). Any
/// `fetch()` call from the Angular bundle is translated to a `reqwest::Client`
/// request against that base, stripping any scheme/host from the incoming URL
/// first (the bundle's fetch calls use relative paths or doorway-originating
/// absolute URLs).
pub struct ResolverFetcher {
    storage_base: String,
    client: Arc<reqwest::Client>,
}

impl ResolverFetcher {
    /// Create a new `ResolverFetcher` targeting `storage_base_url`.
    ///
    /// `storage_base_url` should be the root of the elohim-storage HTTP API,
    /// e.g. `http://localhost:8090` (no trailing slash).
    pub fn new(storage_base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl DataFetcher for ResolverFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        let path = strip_to_path(&request.url);
        let url = format!("{}{}", self.storage_base, path);

        let method = request.method.to_ascii_uppercase();
        let req_builder = match method.as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            _ => self.client.get(&url),
        };

        // Forward request headers
        let mut req_builder = req_builder;
        for (k, v) in &request.headers {
            req_builder = req_builder.header(k.as_str(), v.as_str());
        }

        // Forward body if present
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage fetch error: {e}")))?;

        let status = resp.status().as_u16();

        // Collect response headers into a flat HashMap (last value wins on dupe keys)
        let mut headers: HashMap<String, String> = HashMap::new();
        for (name, value) in resp.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        let body = resp
            .bytes()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage read body: {e}")))?
            .to_vec();

        Ok(FetchResponse {
            status,
            headers,
            body,
            content_hash: None,
        })
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Strip scheme and host from a URL, returning just the path (and query).
///
/// `https://example.com/api/v1/foo?bar=1` → `/api/v1/foo?bar=1`
/// `/api/v1/foo` → `/api/v1/foo`
fn strip_to_path(url: &str) -> &str {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            return &after_scheme[slash_idx..];
        }
        return "/";
    }
    url
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_absolute_url_to_path() {
        assert_eq!(
            strip_to_path("https://example.com/api/v1/foo?bar=1"),
            "/api/v1/foo?bar=1"
        );
    }

    #[test]
    fn strip_absolute_url_no_path() {
        assert_eq!(strip_to_path("https://example.com"), "/");
    }

    #[test]
    fn strip_relative_url_unchanged() {
        assert_eq!(strip_to_path("/api/v1/foo"), "/api/v1/foo");
    }

    #[test]
    fn strip_relative_url_with_query() {
        assert_eq!(strip_to_path("/api/v1/foo?x=1"), "/api/v1/foo?x=1");
    }
}
