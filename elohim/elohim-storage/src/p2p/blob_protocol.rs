//! Blob fetch request-response protocol — `/elohim/blob/1.0.0`.
//!
//! Targeted peer-to-peer blob fetch. Differs from [`super::shard_protocol`] in
//! one critical way: this protocol routes requests to an **explicit `peer_id`**
//! (not any-connected-peer), which is exactly the shape needed by the T17/T20
//! race-fetch helper for parallel-batch first-responder semantics.
//!
//! # Wire format
//!
//! Length-prefixed (4-byte big-endian `u32`) MessagePack via `rmp_serde`,
//! identical to [`super::shard_protocol::ShardCodec`].
//!
//! # Size limits
//!
//! Maximum response size is configurable via [`BlobCodec::with_max_response_size`]
//! and capped at [`HARD_MAX_RESPONSE_SIZE`] (64 MiB) at the codec layer to bound
//! memory usage on the receive side regardless of what the peer claims to send.
//! The default cap ([`DEFAULT_MAX_RESPONSE_SIZE`], 16 MiB) matches `MAX_INLINE_SIZE`
//! in `blob_store.rs`; larger blobs are stored as chunked content and would need
//! a chunked-fetch variant to traverse this protocol.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for explicit-peer blob fetch.
pub const BLOB_PROTOCOL_ID: &str = "/elohim/blob/1.0.0";

/// Default cap on response size if no override is configured. 16 MiB matches
/// `MAX_INLINE_SIZE` in `blob_store.rs`.
pub const DEFAULT_MAX_RESPONSE_SIZE: usize = 16 * 1024 * 1024;

/// Hard upper bound on response size regardless of config. Prevents OOM from
/// a malicious peer claiming a multi-GB length prefix.
pub const HARD_MAX_RESPONSE_SIZE: usize = 64 * 1024 * 1024;

/// Blob protocol marker type for the request-response behaviour.
#[derive(Debug, Clone)]
pub struct BlobProtocol;

impl AsRef<str> for BlobProtocol {
    fn as_ref(&self) -> &str {
        BLOB_PROTOCOL_ID
    }
}

/// Blob fetch request — asks a specific peer for the bytes of a single blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobFetchRequest {
    /// Either a sha256-prefixed identifier (e.g. `"sha256-<hex>"`) or a raw hex
    /// content address. The receiver passes this through to
    /// [`crate::blob_store::BlobStore::get`].
    pub hash: String,
}

/// Blob fetch response — the targeted peer's verdict on the requested blob.
///
/// `Found` carries the verified bytes. The caller is still responsible for
/// sha256 verification (per T19/T20) — the codec does not re-hash on receive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobFetchResponse {
    /// Peer has the blob; bytes attached.
    Found(Vec<u8>),
    /// Peer does not have the blob.
    NotFound,
    /// Peer encountered an internal error trying to read the blob.
    Error(String),
}

/// Codec for the blob fetch protocol. Length-prefixed MessagePack frames.
#[derive(Debug, Clone)]
pub struct BlobCodec {
    max_response_size: usize,
}

impl Default for BlobCodec {
    fn default() -> Self {
        Self {
            max_response_size: DEFAULT_MAX_RESPONSE_SIZE,
        }
    }
}

impl BlobCodec {
    /// Construct a codec with a custom maximum response size, capped at
    /// [`HARD_MAX_RESPONSE_SIZE`].
    pub fn with_max_response_size(max: usize) -> Self {
        Self {
            max_response_size: max.min(HARD_MAX_RESPONSE_SIZE),
        }
    }

    /// Inspect the configured maximum response size.
    pub fn max_response_size(&self) -> usize {
        self.max_response_size
    }
}

#[async_trait]
impl request_response::Codec for BlobCodec {
    type Protocol = BlobProtocol;
    type Request = BlobFetchRequest;
    type Response = BlobFetchResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > self.max_response_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "blob fetch request too large: {len} > {}",
                    self.max_response_size
                ),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
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
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > self.max_response_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "blob fetch response too large: {len} > {}",
                    self.max_response_size
                ),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
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
        let data = rmp_serde::to_vec(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&(data.len() as u32).to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
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
        let data = rmp_serde::to_vec(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&(data.len() as u32).to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrips_via_messagepack() {
        let req = BlobFetchRequest {
            hash: "sha256-abc".to_string(),
        };
        let bytes = rmp_serde::to_vec(&req).unwrap();
        let back: BlobFetchRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.hash, "sha256-abc");
    }

    #[test]
    fn response_found_roundtrips() {
        let r = BlobFetchResponse::Found(b"hello".to_vec());
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        match back {
            BlobFetchResponse::Found(b) => assert_eq!(b, b"hello"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn response_notfound_roundtrips() {
        let r = BlobFetchResponse::NotFound;
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert!(matches!(back, BlobFetchResponse::NotFound));
    }

    #[test]
    fn response_error_roundtrips() {
        let r = BlobFetchResponse::Error("disk failed".to_string());
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        match back {
            BlobFetchResponse::Error(msg) => assert_eq!(msg, "disk failed"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn codec_default_uses_default_max_response_size() {
        let codec = BlobCodec::default();
        assert_eq!(codec.max_response_size(), DEFAULT_MAX_RESPONSE_SIZE);
    }

    #[test]
    fn codec_with_max_caps_at_hard_limit() {
        let codec = BlobCodec::with_max_response_size(usize::MAX);
        assert_eq!(codec.max_response_size(), HARD_MAX_RESPONSE_SIZE);
    }

    #[test]
    fn codec_with_max_below_hard_limit_is_preserved() {
        let codec = BlobCodec::with_max_response_size(8 * 1024 * 1024);
        assert_eq!(codec.max_response_size(), 8 * 1024 * 1024);
    }

    #[test]
    fn protocol_id_is_canonical() {
        assert_eq!(BlobProtocol.as_ref(), "/elohim/blob/1.0.0");
    }
}
