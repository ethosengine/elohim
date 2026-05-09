//! Phase 5 — Automerge CRDT sync over iroh QUIC.
//!
//! Custom ALPN: [`SYNC_ALPN`]. Wire format: length-prefixed MessagePack
//! frames via [`super::codec`]. Wire types — [`SyncRequest`] and
//! [`SyncResponse`] — are reused unchanged from [`crate::p2p::sync_protocol`].
//! Cutover removes the libp2p transport, never the message schema.
//!
//! ## Sketch
//!
//! Phase 5 scaffolds the wire transport. Backend dispatch — looking up
//! Automerge documents, computing change deltas, applying remote changes
//! — lives in a `SyncBackend` trait object supplied by the caller. The
//! stub backend used in the parity test echoes the request shape so we
//! can assert wire-byte parity without wiring the full sync engine.
//!
//! ## Server side
//!
//! [`IrohSyncProtocol`] implements [`iroh::protocol::ProtocolHandler`].
//! For each accepted connection it loops over `accept_bi`, handling
//! streams sequentially. On each bidi stream:
//! 1. read a [`SyncRequest`] via `read_frame_default`
//! 2. dispatch to the configured [`SyncBackend`]
//! 3. write a [`SyncResponse`] via `write_frame`
//! 4. close the send side
//!
//! Loop exits when the peer closes the connection (`accept_bi` errors).
//! This supports both fresh-conn-per-request callers (Phase 5
//! [`IrohSyncClient::request`]) and stream-reuse callers (perf benches;
//! Phase 11 connection pools).
//!
//! ## Client side
//!
//! [`IrohSyncClient::request`] opens a fresh QUIC connection on
//! [`SYNC_ALPN`], writes the request, reads exactly one response, and
//! returns it. Connections are not pooled in Phase 5; that's a Phase 11
//! optimization.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};

use super::codec::{read_frame_default, write_frame};
use crate::p2p::sync_protocol::{SyncRequest, SyncResponse};

/// Iroh-side ALPN for the storage CRDT sync protocol.
///
/// Distinct from the libp2p `SYNC_PROTOCOL_ID` ("/elohim/storage-sync/1.0.0")
/// — version-bumped to 2.0.0 to mark "iroh transport, identical
/// payloads." Phase 11 cutover may consolidate.
pub const SYNC_ALPN: &[u8] = b"/elohim/sync/2.0.0";

/// Backend trait the sync handler dispatches each [`SyncRequest`] into.
///
/// Implementations live in the storage daemon's sync service. Tests use
/// stub backends (e.g. echo, or fixed-response) to exercise wire framing
/// without touching Automerge.
#[async_trait::async_trait]
pub trait SyncBackend: Send + Sync + 'static {
    async fn handle(&self, req: SyncRequest) -> SyncResponse;
}

/// [`ProtocolHandler`] that decodes [`SyncRequest`], dispatches into a
/// [`SyncBackend`], and writes the [`SyncResponse`] back on the same
/// bidi stream. One stream = one request/response round-trip.
#[derive(Clone)]
pub struct IrohSyncProtocol {
    backend: Arc<dyn SyncBackend>,
}

impl IrohSyncProtocol {
    pub fn new(backend: Arc<dyn SyncBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohSyncProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohSyncProtocol").finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohSyncProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                // Peer closed the connection — clean exit.
                Err(_) => return Ok(()),
            };
            let req: SyncRequest = read_frame_default(&mut recv).await.map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame(&mut send, &res).await.map_err(io_to_accept)?;
            send.finish()
                .map_err(|e| AcceptError::from_err(io::Error::other(e.to_string())))?;
        }
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}

/// Client-side helper. Opens a fresh QUIC bidi stream on [`SYNC_ALPN`],
/// writes one [`SyncRequest`], reads one [`SyncResponse`], returns it.
pub struct IrohSyncClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohSyncClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(&self, peer: NodeAddr, req: &SyncRequest) -> Result<SyncResponse> {
        let conn = self.endpoint.connect(peer, SYNC_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: SyncResponse = read_frame_default(&mut recv).await?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial backend that echoes the request shape back as a structurally
    /// matching response — useful for unit-testing the handler shape.
    struct EchoBackend;

    #[async_trait::async_trait]
    impl SyncBackend for EchoBackend {
        async fn handle(&self, req: SyncRequest) -> SyncResponse {
            match req {
                SyncRequest::GetHeads { h_app_id, doc_id } => SyncResponse::Heads {
                    h_app_id,
                    doc_id,
                    heads: vec![],
                    change_count: 0,
                },
                SyncRequest::SyncChanges {
                    h_app_id, doc_id, ..
                } => SyncResponse::Changes {
                    h_app_id,
                    doc_id,
                    changes: vec![],
                    has_more: false,
                    new_heads: vec![],
                },
                SyncRequest::GetChanges {
                    h_app_id, doc_id, ..
                } => SyncResponse::RequestedChanges {
                    h_app_id,
                    doc_id,
                    changes: vec![],
                    not_found: vec![],
                },
                SyncRequest::AnnounceChange {
                    h_app_id, doc_id, ..
                } => SyncResponse::ChangeAck {
                    h_app_id,
                    doc_id,
                    was_new: false,
                },
                SyncRequest::ListDocuments { h_app_id, .. } => SyncResponse::DocumentList {
                    h_app_id,
                    documents: vec![],
                    total: 0,
                    has_more: false,
                },
            }
        }
    }

    #[tokio::test]
    async fn echo_backend_dispatches_each_variant() {
        let backend = EchoBackend;
        let req = SyncRequest::GetHeads {
            h_app_id: "lamad".into(),
            doc_id: "doc-1".into(),
        };
        let res = backend.handle(req).await;
        match res {
            SyncResponse::Heads {
                h_app_id, doc_id, ..
            } => {
                assert_eq!(h_app_id, "lamad");
                assert_eq!(doc_id, "doc-1");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}
