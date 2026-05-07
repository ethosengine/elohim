use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    /// Optional content hash for cache-key derivation.
    pub content_hash: Option<String>,
}

#[async_trait]
pub trait DataFetcher: Send + Sync {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse>;
}
