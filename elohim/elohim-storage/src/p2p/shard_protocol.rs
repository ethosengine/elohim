//! Shard Protocol - Request-response protocol for shard transfer

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for shard transfer
pub const SHARD_PROTOCOL_ID: &str = "/elohim/shard/1.0.0";

/// Shard protocol definition
#[derive(Debug, Clone)]
pub struct ShardProtocol;

impl AsRef<str> for ShardProtocol {
    fn as_ref(&self) -> &str {
        SHARD_PROTOCOL_ID
    }
}

/// Shard request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardRequest {
    /// Get a shard by hash
    Get { hash: String },
    /// Check if peer has a shard
    Have { hash: String },
    /// Push a shard to peer (replication)
    Push { hash: String, data: Vec<u8> },
    /// List content inventory (EPR Head summaries for replication discovery)
    ListContent {
        /// Filter by reach level. None = all reachable content.
        /// Valid values are defined in `crate::generated_enums::CORE_REACH_LEVELS`
        /// (e.g., "public", "commons"). Must match the stored DB value exactly.
        reach_filter: Option<String>,
        /// Pagination offset
        offset: u32,
        /// Pagination limit (max items per response)
        limit: u32,
    },
    /// Get a full content record by ID (metadata + body, not just blob bytes)
    GetContent { id: String },
}

/// Lightweight content summary for inventory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentInventoryItem {
    pub id: String,
    pub title: String,
    pub content_type: String,
    pub content_format: String,
    pub reach: String,
    pub blob_cid: Option<String>,
    pub updated_at: String,
}

/// Full content record for replication (everything needed to recreate locally)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRecord {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i32>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub content_body: Option<String>,
}

/// Shard response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardResponse {
    /// Shard data
    Data(Vec<u8>),
    /// Whether peer has the shard
    Have(bool),
    /// Push acknowledgment
    PushAck,
    /// Shard not found
    NotFound,
    /// Error
    Error(String),
    /// Content inventory listing
    ContentList {
        items: Vec<ContentInventoryItem>,
        total: u64,
        has_more: bool,
    },
    /// Full content record (boxed to keep enum size small)
    Content(Box<ContentRecord>),
    /// Content not found
    ContentNotFound,
}

/// Codec for shard request/response
#[derive(Debug, Clone, Default)]
pub struct ShardCodec;

#[async_trait]
impl request_response::Codec for ShardCodec {
    type Protocol = ShardProtocol;
    type Request = ShardRequest;
    type Response = ShardResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read data
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        // Deserialize
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read data
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        // Deserialize
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = rmp_serde::to_vec(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;

        // Write data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize
        let data = rmp_serde::to_vec(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;

        // Write data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }
}
