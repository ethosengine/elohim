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

/// Hard cap on the size of an inbound `BlobFetchRequest` frame. The request
/// payload is just a content-address string (≤ ~80 bytes for sha256-hex);
/// 4 KiB is generous and bounds the receive buffer regardless of what a peer
/// claims to send. Distinct from the response cap so a small request channel
/// does not get sized to the much larger blob payload limit.
pub const MAX_REQUEST_SIZE: usize = 4 * 1024;

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
        // T21 review fix #3: bound inbound requests by a tight `MAX_REQUEST_SIZE`
        // (4 KiB) rather than the much larger response cap. A request payload is
        // just a content-address string and never approaches the response limit.
        if len > MAX_REQUEST_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("blob fetch request too large: {len} > {MAX_REQUEST_SIZE}"),
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
        // T21 review fix #2: guard against silent truncation in the `usize → u32`
        // length-prefix conversion. The receive side caps payload size, but the
        // write side could otherwise emit a corrupted frame if a caller bypassed
        // the cap (e.g. constructing `BlobFetchResponse::Found(huge_vec)` directly).
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "serialized blob request too large for u32 length prefix: {} bytes",
                    data.len()
                ),
            )
        })?;
        io.write_all(&len.to_be_bytes()).await?;
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
        // T21 review fix #2: see `write_request`.
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "serialized blob response too large for u32 length prefix: {} bytes",
                    data.len()
                ),
            )
        })?;
        io.write_all(&len.to_be_bytes()).await?;
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

    /// T21 review fix #1: validate-or-die assumption — `parse_content_address`
    /// rejects path-traversal payloads. The inbound BlobProtocol handler runs
    /// requests through this gate before touching the filesystem, so any input
    /// that this rejects is treated as `NotFound` upstream. Re-asserting it here
    /// guards against future drift in the validator contract.
    #[test]
    fn parse_content_address_rejects_path_traversal() {
        use crate::blob_store::BlobStore;
        // Classic traversal payloads
        assert!(BlobStore::parse_content_address("../../etc/passwd").is_err());
        assert!(BlobStore::parse_content_address("/etc/passwd").is_err());
        assert!(BlobStore::parse_content_address("..").is_err());
        assert!(BlobStore::parse_content_address("../").is_err());
        assert!(BlobStore::parse_content_address("sha256-../../etc/passwd").is_err());
        // And the happy path still works
        let valid = format!("sha256-{}", "a".repeat(64));
        assert!(BlobStore::parse_content_address(&valid).is_ok());
        let valid_raw = "a".repeat(64);
        assert!(BlobStore::parse_content_address(&valid_raw).is_ok());
    }

    /// T21 review fix #2/#6: round-trip a sizable response through the codec
    /// over an in-memory `futures::io::Cursor` to prove the wire format works
    /// end-to-end at sizes well above the 100-byte happy path. Catches future
    /// regressions in length-prefix handling without requiring a real swarm.
    #[tokio::test]
    async fn codec_roundtrips_large_response_through_async_buffer() {
        use futures::io::Cursor;
        use libp2p::request_response::Codec;

        let payload = vec![0xAB_u8; 1_000_000];
        let response = BlobFetchResponse::Found(payload.clone());

        // Write side: encode via the codec into an in-memory buffer.
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = Cursor::new(&mut buf);
            let mut codec = BlobCodec::default();
            codec
                .write_response(&BlobProtocol, &mut writer, response)
                .await
                .expect("write_response");
        }

        // Read side: decode the buffer back.
        let mut reader = Cursor::new(buf);
        let mut codec = BlobCodec::default();
        let decoded = codec
            .read_response(&BlobProtocol, &mut reader)
            .await
            .expect("read_response");
        match decoded {
            BlobFetchResponse::Found(bytes) => {
                assert_eq!(bytes.len(), payload.len());
                assert_eq!(bytes, payload);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }
}
