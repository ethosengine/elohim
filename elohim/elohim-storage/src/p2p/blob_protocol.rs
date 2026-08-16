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
///
/// # Q3 (minutes-quiesce W1.2): `Manifest`
///
/// **This is a deliberate, evidence-based exception to the crate's default
/// wire-evolution guidance** ("additive `Option` field with
/// `skip_serializing_if`, never a new variant" — see the T4 wire rule /
/// `head_corpus_digest` on `ViewFederationRequest`). That guidance targets a
/// STRUCT-rooted wire type, where an absent key round-trips for free. Here
/// the wire ROOT is the enum itself, and `rmp_serde`'s default (non-`_named`)
/// encoding treats unit variants and payload-carrying variants differently:
/// a unit variant (`NotFound`) serializes as a bare MessagePack string,
/// while a newtype variant (`Found(Vec<u8>)`) serializes as a **map with
/// exactly one entry** (`serialize_newtype_variant` in `rmp-serde`) — and its
/// `deserialize_enum` REJECTS a map with any other entry count
/// (`Error::LengthMismatch`). A first attempt wrapped this enum in a struct
/// with `#[serde(flatten)] outcome` plus an additive `shard_manifest: Option<
/// ShardManifest>` field; it looked identical to the struct pattern used
/// elsewhere, but `#[serde(flatten)]` unconditionally forces map
/// serialization on the flattened value — even `NotFound` (previously a bare
/// string) became `{"NotFound": null}`, changing the wire shape for EVERY
/// reply, not just manifest-bearing ones. `wire_compat_new_bytes_with_manifest_decode_on_old_struct`
/// caught this empirically (`LengthMismatch(2)` decoding a 2-entry map) —
/// the byte-identical-when-unused property was violated even in the common
/// case. A new `Manifest` variant does NOT touch `Found`/`NotFound`/`Error`'s
/// encoding at all, so those three stay byte-identical in both directions
/// (proven by `wire_compat_*` below) — the only decode failure is an old
/// peer receiving an actual `Manifest` reply, which is the SAME failure
/// class (a benign decode `Err`, never a panic or silent corruption — it
/// surfaces through the codec's `read_response` as an `io::Error`, which
/// `race_fetch`'s caller already treats as an ordinary failed candidate) a
/// flattened extra field would also have produced for that one case. Net:
/// switching variants trades nothing away and fixes the common-case
/// regression.
///
/// **Fleet rollout rule:** the responder ships before any caller constructs
/// a `Manifest` reply — locally both sides are one binary (this crate's
/// fleet moves together per deploy), so a peer old enough to have never
/// heard of `Manifest` exists only in the brief window of a rolling
/// restart, and even then degrades to a single failed fetch attempt, not a
/// crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobFetchResponse {
    /// Peer has the blob; bytes attached.
    Found(Vec<u8>),
    /// Peer does not have the blob (and no manifest either).
    NotFound,
    /// Peer encountered an internal error trying to read the blob.
    Error(String),
    /// Q3: no direct bytes under the requested hash, but this peer resolved
    /// a durable [`ShardManifest`] for it — the requester can pivot to a
    /// swarm shard-fetch (Q4) instead of a dead-end miss. Boxed: `ShardManifest`
    /// (~240 bytes: several `String`s + a `Vec<String>` + two `Option<String>`s)
    /// would otherwise bloat every `BlobFetchResponse` value to its size
    /// regardless of variant (clippy::large_enum_variant).
    Manifest(Box<crate::sharding::ShardManifest>),
}

impl BlobFetchResponse {
    /// Bytes served directly.
    pub fn found(bytes: Vec<u8>) -> Self {
        Self::Found(bytes)
    }

    /// Plain miss — no bytes, no manifest.
    pub fn not_found() -> Self {
        Self::NotFound
    }

    /// Internal error trying to read the blob.
    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error(msg.into())
    }

    /// Q3: no direct bytes under the requested hash, but this peer resolved a
    /// durable [`ShardManifest`] for it.
    pub fn manifest_only(manifest: crate::sharding::ShardManifest) -> Self {
        Self::Manifest(Box::new(manifest))
    }

    /// Bounded, single-line summary for logging.
    ///
    /// A `send_response` Err payload is the whole unsent response — never
    /// Debug-format it: `Found` carries the full blob bytes. Renders the byte
    /// length instead (Loki drops log entries over 256KB).
    pub fn summary(&self) -> String {
        match self {
            Self::Found(bytes) => format!("Found({} bytes)", bytes.len()),
            Self::NotFound => "NotFound".to_string(),
            Self::Error(msg) => format!("Error({msg})"),
            Self::Manifest(m) => format!("Manifest({} shards)", m.shard_hashes.len()),
        }
    }
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
        let r = BlobFetchResponse::found(b"hello".to_vec());
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        match back {
            BlobFetchResponse::Found(b) => assert_eq!(b, b"hello"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn response_notfound_roundtrips() {
        let r = BlobFetchResponse::not_found();
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert!(matches!(back, BlobFetchResponse::NotFound));
    }

    #[test]
    fn response_error_roundtrips() {
        let r = BlobFetchResponse::error("disk failed");
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        match back {
            BlobFetchResponse::Error(msg) => assert_eq!(msg, "disk failed"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// Minimal fixture for wire-compat tests — field values are irrelevant to
    /// the shape assertion, only that the manifest round-trips intact.
    fn fixture_manifest() -> crate::sharding::ShardManifest {
        crate::sharding::ShardManifest {
            blob_cid: "bafkreitest".to_string(),
            blob_hash: "sha256-abc".to_string(),
            total_size: 2_000_000,
            mime_type: "application/octet-stream".to_string(),
            encoding: "chunked".to_string(),
            data_shards: 2,
            total_shards: 2,
            shard_size: 1_000_000,
            shard_hashes: vec!["sha256-shard0".to_string(), "sha256-shard1".to_string()],
            reach: "commons".to_string(),
            author_id: None,
            created_at: "2026-08-16T00:00:00Z".to_string(),
            verified_at: None,
        }
    }

    /// Q3 wire-compat #1: round-trip a `Manifest` response.
    #[test]
    fn wire_compat_manifest_roundtrips() {
        let manifest = fixture_manifest();
        let r = BlobFetchResponse::manifest_only(manifest.clone());
        let bytes = rmp_serde::to_vec(&r).unwrap();
        let back: BlobFetchResponse = rmp_serde::from_slice(&bytes).unwrap();
        match back {
            BlobFetchResponse::Manifest(m) => {
                assert_eq!(m.blob_hash, manifest.blob_hash);
                assert_eq!(m.shard_hashes, manifest.shard_hashes);
            }
            other => panic!("expected Manifest, got {other:?}"),
        }
    }

    /// Q3 wire-compat #2: bytes produced by a PRE-Q3 peer (the bare 3-variant
    /// enum with no `Manifest` variant at all) must decode cleanly on the
    /// CURRENT 4-variant type for every leg that existed before Q3.
    /// `LegacyBlobFetchResponse` mirrors the exact pre-Q3 wire type. This is
    /// the property that actually matters for a rolling deploy: an old
    /// responder's `Found`/`NotFound`/`Error` replies must keep decoding on
    /// new code exactly as before, and this test proves it byte-for-byte —
    /// unlike the flatten-based first attempt, adding the `Manifest` variant
    /// touches NONE of these three encodings.
    #[test]
    fn wire_compat_old_bytes_decode_on_new_enum() {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        enum LegacyBlobFetchResponse {
            Found(Vec<u8>),
            NotFound,
            Error(String),
        }

        for legacy in [
            LegacyBlobFetchResponse::Found(b"legacy-bytes".to_vec()),
            LegacyBlobFetchResponse::NotFound,
            LegacyBlobFetchResponse::Error("legacy disk failed".to_string()),
        ] {
            let bytes = rmp_serde::to_vec(&legacy).unwrap();
            let decoded: BlobFetchResponse = rmp_serde::from_slice(&bytes)
                .expect("current BlobFetchResponse must decode every pre-Q3 wire shape");
            match (legacy, decoded) {
                (LegacyBlobFetchResponse::Found(a), BlobFetchResponse::Found(b)) => {
                    assert_eq!(a, b)
                }
                (LegacyBlobFetchResponse::NotFound, BlobFetchResponse::NotFound) => {}
                (LegacyBlobFetchResponse::Error(a), BlobFetchResponse::Error(b)) => {
                    assert_eq!(a, b)
                }
                (legacy, decoded) => {
                    panic!("shape mismatch: legacy={legacy:?} decoded={decoded:?}")
                }
            }
        }
    }

    /// Q3 wire-compat #3: a CURRENT `Found`/`NotFound`/`Error` reply (the
    /// common, everyday case — including a responder that resolved NO
    /// manifest and replied plain `NotFound`) must decode correctly on a
    /// PRE-Q3 peer during a rolling-deploy window. This is the property the
    /// flatten-based first attempt actually violated (it changed `NotFound`
    /// itself from a bare MessagePack string to a `{"NotFound": null}` map,
    /// breaking every reply, not just manifest-bearing ones) — proving it
    /// here pins the regression this design closes.
    ///
    /// A `Manifest` reply is NOT included: an old peer has never heard of
    /// that variant name, so decoding it fails (a benign `Err`, not a panic
    /// — see the type-level doc comment on `BlobFetchResponse`). That
    /// failure is confined to the rolling-restart window and to the one
    /// fetch attempt that hit it; it is not this test's job to assert a
    /// property `rmp_serde`'s enum encoding cannot provide.
    #[test]
    fn wire_compat_new_bytes_decode_on_old_enum() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        enum LegacyBlobFetchResponse {
            Found(Vec<u8>),
            NotFound,
            Error(String),
        }

        let cases = [
            (
                BlobFetchResponse::found(b"current-bytes".to_vec()),
                LegacyBlobFetchResponse::Found(b"current-bytes".to_vec()),
            ),
            (
                BlobFetchResponse::not_found(),
                LegacyBlobFetchResponse::NotFound,
            ),
            (
                BlobFetchResponse::error("current disk failed"),
                LegacyBlobFetchResponse::Error("current disk failed".to_string()),
            ),
        ];
        for (current, expected) in cases {
            let bytes = rmp_serde::to_vec(&current).unwrap();
            let decoded: LegacyBlobFetchResponse = rmp_serde::from_slice(&bytes)
                .expect("a pre-Q3 peer must still decode a non-Manifest current response");
            assert_eq!(decoded, expected);
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
        let response = BlobFetchResponse::found(payload.clone());

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
