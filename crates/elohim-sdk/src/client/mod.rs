//! Content client with mode-aware resolution
//!
//! Provides unified content access across different deployment modes:
//! - Browser: via Doorway → Projection Store
//! - Native: via local SQLite
//! - Node: via local SQLite with P2P sync

mod content_client;
mod projection_warmer;

pub use content_client::{ClientMode, ContentClient};
pub use projection_warmer::ProjectionWarmer;

/// Build a reqwest client that is schema-negotiation conformant by construction.
///
/// elohim-storage's `validate_schema_version_header` treats an absent
/// `X-Schema-Version` on a guarded `/db/**/bulk` write as a DEPRECATED client
/// contract: it logs "Bulk request missing X-Schema-Version header" and falls
/// back to `default_schema_version()`. Declaring the header as a client default
/// here (rather than per call site) means a future guarded route reached from
/// any SDK client is conformant by construction — the same pattern
/// `crates/elohim-storage-client` uses.
///
/// The value is read from the shared contract so it cannot drift from
/// `SUPPORTED_SCHEMA_VERSIONS` (`elohim/elohim-views/src/shared.rs`).
pub(crate) fn schema_conformant_http_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "X-Schema-Version",
        reqwest::header::HeaderValue::from(elohim_views::shared::default_schema_version()),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("Failed to build schema-conformant HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// On-the-wire proof that the SDK's shared client is schema-negotiation
    /// conformant: a bulk POST built from `schema_conformant_http_client()`
    /// carries `X-Schema-Version`, so elohim-storage's guarded bulk handlers
    /// take the present-header branch instead of the deprecated warn branch.
    #[tokio::test]
    async fn schema_conformant_client_sends_schema_version_header_on_bulk_post() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap();
            sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]")
                .await
                .unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });

        let client = schema_conformant_http_client();
        let _ = client
            .post(format!("http://{}/db/elohim/content/bulk", addr))
            .json(&serde_json::json!([]))
            .send()
            .await
            .unwrap();

        let request = server.await.unwrap().to_lowercase();
        let expected = format!(
            "x-schema-version: {}",
            elohim_views::shared::default_schema_version()
        );
        assert!(
            request.contains(&expected),
            "bulk POST must carry the schema header; got request:\n{}",
            request
        );
    }
}
