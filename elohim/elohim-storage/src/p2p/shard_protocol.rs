//! Shard Protocol - Request-response protocol for shard transfer

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for shard transfer
pub const SHARD_PROTOCOL_ID: &str = "/elohim/shard/1.0.0";

/// Maximum serialized frame accepted by a shard-bearing transport.
///
/// This is deliberately distinct from the generic 16 MiB control-frame
/// default used by the iroh codec. The ceiling bounds the wire shape; it
/// is not a whole-blob admission limit or an arbitrary-size streaming claim.
pub const SHARD_TRANSFER_MAX_FRAME_SIZE: usize = 64 * 1024 * 1024;

/// The largest blob payload a single shard frame can carry.
///
/// Byte-bearing fields ride the wire as a MessagePack `bin` value, so the
/// envelope around them is a handful of bytes (variant tag, `bin32` header,
/// the hash string on a `Push`). One KiB of headroom keeps the arithmetic
/// honest without pretending to be exact.
///
/// Ask this BEFORE materialising a payload. A provider that reads and hashes
/// bytes it can never frame burns the request's whole cost to arrive at a
/// refusal — which is precisely the fault of 2026-09-13 (see
/// `serialize_bounded_messagepack`).
pub const SHARD_PAYLOAD_MAX_BYTES: usize = SHARD_TRANSFER_MAX_FRAME_SIZE - 1024;

/// Whether a payload of `len` bytes can be framed by this protocol at all.
pub fn payload_fits_frame(len: u64) -> bool {
    len <= SHARD_PAYLOAD_MAX_BYTES as u64
}

/// Serialize into a buffer that refuses growth beyond `max_size`.
///
/// The limit is enforced by the `Write` implementation while MessagePack is
/// encoding, rather than after an unbounded `to_vec` allocation. `named`
/// preserves the two established wire encodings: libp2p uses positional
/// structs and iroh uses named structs.
///
/// **Byte fields must be `bin`, never a sequence of integers.** A `Vec<u8>`
/// serialized by plain serde is a MessagePack *array*, one integer per byte:
/// every byte ≥ `0x80` costs two bytes on the wire, and every byte costs one
/// call through this `Write` adapter. Measured on matthew's live 231 MB
/// rs-4-7 object (2026-09-13), 51.3% of its bytes are ≥ `0x80`, so each of
/// its seven 57,948,918-byte shards encoded to **87,695,655 bytes** — past
/// the 64 MiB ceiling. Every shard serve therefore read 58 MB off disk,
/// logged `Serving shard`, spent seconds encoding ~58 million array elements
/// on an HTTP worker thread, and *then* refused the frame. The bytes never
/// reached the wire, the requesters' `missing_shards` stayed at 7 forever,
/// and their retries pinned both workers of the 2-thread server runtime at
/// 100% (health latency 8 s+). `serde_bytes` makes the same shard a
/// 57,948,924-byte `bin` written with one `write_all`.
///
/// rmp-serde's decoder accepts `bin` where a `Vec<u8>` field is declared and
/// a sequence where a `serde_bytes` field is declared, so this is a
/// serializer-side change only — frames stay readable in both directions and
/// the protocol id does not move.
pub(crate) fn serialize_bounded_messagepack<T: Serialize>(
    value: &T,
    max_size: usize,
    named: bool,
    label: &str,
) -> io::Result<Vec<u8>> {
    struct BoundedBuffer {
        bytes: Vec<u8>,
        max_size: usize,
        rejected_size: Option<usize>,
    }

    impl io::Write for BoundedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let next_size =
                self.bytes.len().checked_add(buf.len()).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "frame size overflow")
                })?;
            if next_size > self.max_size {
                self.rejected_size = Some(next_size);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "serialized frame exceeds configured limit",
                ));
            }
            std::io::Write::write(&mut self.bytes, buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut output = BoundedBuffer {
        bytes: Vec::new(),
        max_size,
        rejected_size: None,
    };
    let result = if named {
        let mut serializer = rmp_serde::encode::Serializer::new(&mut output).with_struct_map();
        value.serialize(&mut serializer)
    } else {
        let mut serializer = rmp_serde::encode::Serializer::new(&mut output);
        value.serialize(&mut serializer)
    };
    if let Err(error) = result {
        if let Some(size) = output.rejected_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{label} frame too large: {size} > {max_size}"),
            ));
        }
        return Err(io::Error::new(io::ErrorKind::InvalidData, error));
    }
    Ok(output.bytes)
}

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
    Push {
        hash: String,
        /// `bin`, never an integer array — see `serialize_bounded_messagepack`.
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
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
    /// Ask for the shard manifest of a composite (RS-sharded) blob — the
    /// pivot a whole-bytes `Get` miss takes on the iroh plane, mirroring the
    /// libp2p blob protocol's `BlobFetchReply::Manifest`. Appended last so an
    /// older peer never receives it unless it asked (positional codec).
    GetManifest { hash: String },
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
    /// Shard data. `bin`, never an integer array — see
    /// `serialize_bounded_messagepack`.
    Data(#[serde(with = "serde_bytes")] Vec<u8>),
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
    /// A durable shard manifest for the requested composite hash (boxed —
    /// see `FetchOutcome::Manifest`). Only ever sent in reply to `GetManifest`.
    Manifest(Box<crate::sharding::ShardManifest>),
}

impl ShardResponse {
    /// Bounded, single-line summary for logging.
    ///
    /// A `send_response` Err payload is the whole unsent response — never
    /// Debug-format it: `Data`, `ContentList`, and `Content` can embed blob
    /// bytes or the full inventory (observed 1+ MB single log lines; Loki
    /// drops entries over 256KB). Renders variant + size facts only.
    pub fn summary(&self) -> String {
        match self {
            Self::Data(bytes) => format!("Data({} bytes)", bytes.len()),
            Self::Manifest(m) => format!("Manifest({} shards)", m.shard_hashes.len()),
            Self::Have(has) => format!("Have({has})"),
            Self::PushAck => "PushAck".to_string(),
            Self::NotFound => "NotFound".to_string(),
            Self::Error(msg) => format!("Error({msg})"),
            Self::ContentList {
                items,
                total,
                has_more,
            } => format!(
                "ContentList({} items, total={total}, has_more={has_more})",
                items.len()
            ),
            Self::Content(_) => "Content(record)".to_string(),
            Self::ContentNotFound => "ContentNotFound".to_string(),
        }
    }
}

/// Codec for shard request/response
#[derive(Debug, Clone, Default)]
pub struct ShardCodec;

async fn read_bounded_frame<T, R>(io: &mut R, label: &str) -> io::Result<T>
where
    T: serde::de::DeserializeOwned,
    R: AsyncRead + Unpin + Send,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > SHARD_TRANSFER_MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} frame too large: {len} > {SHARD_TRANSFER_MAX_FRAME_SIZE}"),
        ));
    }

    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

async fn write_bounded_frame<T, W>(io: &mut W, value: &T, label: &str) -> io::Result<()>
where
    T: Serialize,
    W: AsyncWrite + Unpin + Send,
{
    // Keep positional encoding here: changing to named fields or MessagePack
    // bin values would change the established `/elohim/shard/1.0.0` wire.
    let data = serialize_bounded_messagepack(value, SHARD_TRANSFER_MAX_FRAME_SIZE, false, label)?;
    let len = u32::try_from(data.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} frame exceeds u32 length prefix: {}", data.len()),
        )
    })?;
    io.write_all(&len.to_be_bytes()).await?;
    io.write_all(&data).await?;
    io.flush().await
}

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
        read_bounded_frame(io, "shard request").await
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_bounded_frame(io, "shard response").await
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
        write_bounded_frame(io, &request, "shard request").await
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
        write_bounded_frame(io, &response, "shard response").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_list_summary_stays_bounded() {
        // Regression guard for the Loki >256KB log-drop class: a large inventory
        // must render as counts, never as its Debug dump (which is ~MBs here).
        let items: Vec<ContentInventoryItem> = (0..10_000)
            .map(|i| ContentInventoryItem {
                id: format!("content-{i:08}"),
                title: format!("A reasonably long content title number {i} with padding"),
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                reach: "public".to_string(),
                blob_cid: Some(format!("bafkrei{i:056}")),
                updated_at: "2026-07-03T00:00:00Z".to_string(),
            })
            .collect();
        let resp = ShardResponse::ContentList {
            items,
            total: 10_000,
            has_more: true,
        };

        let summary = resp.summary();
        assert!(
            summary.len() < 256,
            "summary must stay bounded, got {} bytes: {summary}",
            summary.len()
        );
        assert!(summary.contains("10000 items"));
    }

    #[test]
    fn data_summary_reports_length_not_contents() {
        let resp = ShardResponse::Data(vec![0u8; 5_000_000]);
        let summary = resp.summary();
        assert!(summary.len() < 64, "got: {summary}");
        assert!(summary.contains("5000000 bytes"));
    }

    #[tokio::test]
    async fn bounded_writer_preserves_the_existing_small_request_wire() {
        use futures::io::Cursor;
        use libp2p::request_response::Codec;

        let request = ShardRequest::Push {
            hash: "sha256-small".to_string(),
            data: b"existing-wire".to_vec(),
        };
        let historical = rmp_serde::to_vec(&request).expect("historical serializer");
        let mut framed = Vec::new();
        let mut writer = Cursor::new(&mut framed);
        ShardCodec
            .write_request(&ShardProtocol, &mut writer, request)
            .await
            .expect("bounded writer");

        assert_eq!(&framed[..4], &(historical.len() as u32).to_be_bytes());
        assert_eq!(&framed[4..], historical.as_slice());
    }

    #[tokio::test]
    async fn shard_reader_refuses_oversize_prefix_before_payload_read() {
        use futures::io::Cursor;
        use libp2p::request_response::Codec;

        let claimed = u32::try_from(SHARD_TRANSFER_MAX_FRAME_SIZE + 1).unwrap();
        let mut reader = Cursor::new(claimed.to_be_bytes().to_vec());
        let error = ShardCodec
            .read_request(&ShardProtocol, &mut reader)
            .await
            .expect_err("the length prefix alone must be enough to refuse");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("frame too large"));
        assert_eq!(reader.position(), 4, "no payload bytes may be requested");
    }

    #[tokio::test]
    async fn shard_writer_refuses_oversize_body_before_any_network_bytes_are_written() {
        use futures::io::Cursor;
        use libp2p::request_response::Codec;

        // Byte fields ride as `bin` (one wire byte per payload byte), so a
        // payload is oversize only when it genuinely exceeds the budget.
        let request = ShardRequest::Push {
            hash: "sha256-oversize".to_string(),
            data: vec![0xff; SHARD_TRANSFER_MAX_FRAME_SIZE + 1],
        };
        let mut framed = Vec::new();
        let mut writer = Cursor::new(&mut framed);
        let error = ShardCodec
            .write_request(&ShardProtocol, &mut writer, request)
            .await
            .expect_err("serialized request must exceed the hard frame budget");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("frame too large"));
        assert!(
            framed.is_empty(),
            "a refused frame must write no prefix or body"
        );
    }

    /// The 2026-09-13 provider-saturation fault, as arithmetic.
    ///
    /// A shard of real (compressed) bytes is ~51% bytes >= 0x80. Encoded as a
    /// MessagePack array of integers those cost two bytes each, so a
    /// 57,948,918-byte shard became 87,695,655 wire bytes and no frame could
    /// ever carry it. As `bin` the same shard is its own length plus a small
    /// header.
    #[test]
    fn a_real_sized_shard_fits_one_frame() {
        // Mirror the live shard's byte distribution rather than an all-0xff
        // worst case: alternating low/high bytes is the 50% that was measured.
        const SHARD_LEN: usize = 57_948_918;
        let payload: Vec<u8> = (0..SHARD_LEN)
            .map(|i| if i % 2 == 0 { 0x41 } else { 0xff })
            .collect();

        let encoded = serialize_bounded_messagepack(
            &ShardResponse::Data(payload),
            SHARD_TRANSFER_MAX_FRAME_SIZE,
            false,
            "shard response",
        )
        .expect("a single shard must fit a single frame");

        assert!(
            encoded.len() < SHARD_LEN + 1024,
            "a bin-encoded shard must not expand: {} wire bytes for {SHARD_LEN} payload bytes",
            encoded.len()
        );
        assert!(payload_fits_frame(SHARD_LEN as u64));
    }

    /// The whole 231 MB composite still cannot be framed — correctly. It is
    /// reachable through the manifest pivot, and the provider must learn that
    /// from the declared size BEFORE it reads and hashes a quarter-gigabyte.
    #[test]
    fn a_whole_composite_is_refused_by_declared_size() {
        assert!(!payload_fits_frame(231_795_670));
        assert!(payload_fits_frame(SHARD_PAYLOAD_MAX_BYTES as u64));
        assert!(!payload_fits_frame(SHARD_PAYLOAD_MAX_BYTES as u64 + 1));
    }

    /// Wire compatibility in BOTH directions, which is what lets the
    /// serializer change without moving `/elohim/shard/1.0.0`.
    ///
    /// rmp-serde reads a `bin` into a plain `Vec<u8>` field and a sequence
    /// into a `serde_bytes` field, so a peer on either side of this change
    /// decodes a peer on the other.
    #[test]
    fn bin_and_array_byte_encodings_decode_interchangeably() {
        use serde::{Deserialize, Serialize};

        // A stand-in for a peer built before this change: byte fields with no
        // `serde_bytes`, everything else identical.
        #[derive(Serialize, Deserialize)]
        enum LegacyShardResponse {
            Data(Vec<u8>),
            #[allow(dead_code)]
            Have(bool),
        }

        let payload: Vec<u8> = (0..=255u8).cycle().take(4096).collect();

        let new_frame =
            rmp_serde::to_vec(&ShardResponse::Data(payload.clone())).expect("new serializer");
        let legacy_frame = rmp_serde::to_vec(&LegacyShardResponse::Data(payload.clone()))
            .expect("legacy serializer");

        assert!(
            new_frame.len() < legacy_frame.len(),
            "bin must be the smaller encoding: {} vs {}",
            new_frame.len(),
            legacy_frame.len()
        );

        // New bytes -> old peer.
        let decoded: LegacyShardResponse =
            rmp_serde::from_slice(&new_frame).expect("an old peer must read a bin frame");
        let LegacyShardResponse::Data(bytes) = decoded else {
            panic!("variant must round-trip");
        };
        assert_eq!(bytes, payload);

        // Old bytes -> new peer.
        let decoded: ShardResponse =
            rmp_serde::from_slice(&legacy_frame).expect("a new peer must read an array frame");
        let ShardResponse::Data(bytes) = decoded else {
            panic!("variant must round-trip");
        };
        assert_eq!(bytes, payload);
    }
}
