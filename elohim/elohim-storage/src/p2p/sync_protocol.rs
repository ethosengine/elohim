//! Sync Protocol - Request-response protocol for Automerge CRDT document sync
//!
//! This protocol enables offline-first document synchronization between nodes.
//! It uses Automerge's efficient sync protocol under the hood.
//!
//! ## Protocol Flow
//!
//! ```text
//! Node A                              Node B
//!   │                                    │
//!   ├──── SyncRequest(doc_id, heads) ───►│
//!   │                                    │
//!   │◄─── SyncResponse(changes, more) ───┤
//!   │                                    │
//!   │     (apply changes locally)        │
//!   │                                    │
//!   ├──── SyncRequest(new_heads) ───────►│  (if more=true)
//!   │                                    │
//!   │◄─── SyncResponse([], false) ───────┤  (sync complete)
//! ```

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for storage CRDT sync.
///
/// Renamed from `/elohim/sync/1.0.0` to avoid collision with elohim-node's
/// `/elohim/doc-sync/1.0.0` when both run in the unified swarm.
pub const SYNC_PROTOCOL_ID: &str = "/elohim/storage-sync/1.0.0";

/// Sync protocol definition
#[derive(Debug, Clone)]
pub struct SyncProtocol;

impl AsRef<str> for SyncProtocol {
    fn as_ref(&self) -> &str {
        SYNC_PROTOCOL_ID
    }
}

/// Sync request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Get current sync state for a document
    /// Returns the document's heads (latest change hashes)
    GetHeads {
        /// Application namespace (e.g., "lamad", "calendar")
        h_app_id: String,
        doc_id: String,
    },

    /// Request changes since given heads
    /// This is the core Automerge sync request
    SyncChanges {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// The requester's current heads (change hashes they have)
        have_heads: Vec<String>,
        /// Optional: bloom filter of changes they have (for efficiency)
        bloom_filter: Option<Vec<u8>>,
    },

    /// Request specific changes by hash (for targeted fetch)
    GetChanges {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// Specific change hashes to retrieve
        change_hashes: Vec<String>,
    },

    /// Announce a new change (push notification)
    AnnounceChange {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// The new change hash
        change_hash: String,
        /// Optional: the change data itself (for eager push)
        change_data: Option<Vec<u8>>,
    },

    /// List documents this peer is willing to sync
    ListDocuments {
        /// Application namespace
        h_app_id: String,
        /// Optional filter by document type/prefix
        prefix: Option<String>,
        /// Pagination
        offset: u32,
        limit: u32,
    },
}

/// Sync response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    /// Document heads response
    Heads {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// Current heads (latest change hashes)
        heads: Vec<String>,
        /// Total number of changes in the document
        change_count: u64,
    },

    /// Changes response
    Changes {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// Serialized Automerge changes
        changes: Vec<Vec<u8>>,
        /// Whether there are more changes to sync
        has_more: bool,
        /// The new heads after applying these changes
        new_heads: Vec<String>,
    },

    /// Specific changes response
    RequestedChanges {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// Map of hash -> change data
        changes: Vec<(String, Vec<u8>)>,
        /// Hashes that were not found
        not_found: Vec<String>,
    },

    /// Change announcement acknowledgment
    ChangeAck {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
        /// Whether the change was new to this peer
        was_new: bool,
    },

    /// Document list response
    DocumentList {
        /// Application namespace
        h_app_id: String,
        documents: Vec<DocumentInfo>,
        /// Total count for pagination
        total: u64,
        has_more: bool,
    },

    /// Document not found
    NotFound {
        /// Application namespace
        h_app_id: String,
        doc_id: String,
    },

    /// Error response
    Error { message: String },
}

impl SyncResponse {
    /// Bounded, single-line summary for logging.
    ///
    /// A `send_response` Err payload is the whole unsent response — never
    /// Debug-format it: `Changes`/`RequestedChanges` embed serialized Automerge
    /// change bytes and `DocumentList` the full document set. Renders counts
    /// only (Loki drops log entries over 256KB).
    pub fn summary(&self) -> String {
        match self {
            Self::Heads {
                doc_id,
                heads,
                change_count,
                ..
            } => format!(
                "Heads(doc={doc_id}, {} heads, change_count={change_count})",
                heads.len()
            ),
            Self::Changes {
                doc_id,
                changes,
                has_more,
                new_heads,
                ..
            } => format!(
                "Changes(doc={doc_id}, {} changes, has_more={has_more}, {} new_heads)",
                changes.len(),
                new_heads.len()
            ),
            Self::RequestedChanges {
                doc_id,
                changes,
                not_found,
                ..
            } => format!(
                "RequestedChanges(doc={doc_id}, {} changes, {} not_found)",
                changes.len(),
                not_found.len()
            ),
            Self::ChangeAck {
                doc_id, was_new, ..
            } => format!("ChangeAck(doc={doc_id}, was_new={was_new})"),
            Self::DocumentList {
                documents,
                total,
                has_more,
                ..
            } => format!(
                "DocumentList({} documents, total={total}, has_more={has_more})",
                documents.len()
            ),
            Self::NotFound { doc_id, .. } => format!("NotFound(doc={doc_id})"),
            Self::Error { message } => format!("Error({message})"),
        }
    }
}

/// Information about a syncable document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Document ID
    pub doc_id: String,
    /// Document type (e.g., "graph", "path", "personal")
    pub doc_type: String,
    /// Number of changes
    pub change_count: u64,
    /// Last modified timestamp (Unix millis)
    pub last_modified: u64,
    /// Current heads
    pub heads: Vec<String>,
}

/// Codec for sync request/response
#[derive(Debug, Clone, Default)]
pub struct SyncCodec;

#[async_trait]
impl request_response::Codec for SyncCodec {
    type Protocol = SyncProtocol;
    type Request = SyncRequest;
    type Response = SyncResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check: max 16MB request
        if len > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Request too large",
            ));
        }

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

        // Sanity check: max 64MB response (changes can be large)
        if len > 64 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Response too large",
            ));
        }

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

/// Hard ceiling on a `ListDocuments` page chain (offset bound). Bounds a
/// lying always-`has_more` peer to `MAX/page` requests per round instead of an
/// unbounded, backpressure-immune request loop. Honest stores below the cap
/// are unaffected; a store larger than the cap syncs its first `MAX` docs per
/// round (raise deliberately if a real corpus ever approaches it).
pub const MAX_SYNC_LIST_OFFSET: u32 = 100_000;

/// Compute the next `ListDocuments` page offset after a `DocumentList` response.
///
/// `None` means the chain ends: the peer reported no further pages, OR the
/// cursor stopped advancing (zero-progress page / `u32` saturation — either
/// would re-request the same page forever), OR the `MAX_SYNC_LIST_OFFSET`
/// cap was reached (a peer claiming `has_more` forever must not drive an
/// unbounded loop).
pub fn next_doc_list_offset(offset: u32, page_len: usize, has_more: bool) -> Option<u32> {
    if !has_more || page_len == 0 {
        return None;
    }
    let next = offset.saturating_add(u32::try_from(page_len).unwrap_or(u32::MAX));
    (next > offset && next <= MAX_SYNC_LIST_OFFSET).then_some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_request_serialization() {
        let request = SyncRequest::GetHeads {
            h_app_id: "lamad".to_string(),
            doc_id: "test-doc".to_string(),
        };
        let bytes = rmp_serde::to_vec(&request).unwrap();
        let decoded: SyncRequest = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            SyncRequest::GetHeads { h_app_id, doc_id } => {
                assert_eq!(h_app_id, "lamad");
                assert_eq!(doc_id, "test-doc");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_sync_response_serialization() {
        let response = SyncResponse::Heads {
            h_app_id: "lamad".to_string(),
            doc_id: "test-doc".to_string(),
            heads: vec!["abc123".to_string()],
            change_count: 42,
        };
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: SyncResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            SyncResponse::Heads {
                h_app_id,
                doc_id,
                heads,
                change_count,
            } => {
                assert_eq!(h_app_id, "lamad");
                assert_eq!(doc_id, "test-doc");
                assert_eq!(heads.len(), 1);
                assert_eq!(change_count, 42);
            }
            _ => panic!("Wrong variant"),
        }
    }

    /// The DocumentList page cursor: follow `has_more` until the horizon is
    /// drained. A doc past the first page must never be silently unsyncable
    /// (586-row corpus today, 1000-doc page — growth flips this from latent to
    /// live). Every non-advancing or unbounded shape terminates: `page_len == 0`
    /// with `has_more` is a lying/buggy peer (advance-by-zero = same page
    /// forever); saturation at `u32::MAX` would likewise stop advancing, so it
    /// terminates rather than re-requesting the identical page; and a peer
    /// reporting `has_more` forever is bounded by `MAX_SYNC_LIST_OFFSET`.
    #[test]
    fn next_doc_list_offset_follows_has_more() {
        assert_eq!(next_doc_list_offset(0, 1000, true), Some(1000));
        assert_eq!(next_doc_list_offset(1000, 500, false), None);
        assert_eq!(next_doc_list_offset(0, 0, true), None);
        assert_eq!(
            next_doc_list_offset(u32::MAX, 5, true),
            None,
            "a non-advancing (saturated) cursor must terminate, not spin on one page"
        );
        assert_eq!(
            next_doc_list_offset(MAX_SYNC_LIST_OFFSET - 1000, 1000, true),
            Some(MAX_SYNC_LIST_OFFSET),
            "the last in-bound page is still requested"
        );
        assert_eq!(
            next_doc_list_offset(MAX_SYNC_LIST_OFFSET, 1000, true),
            None,
            "a lying always-has_more peer is bounded by the chain cap"
        );
    }
}
