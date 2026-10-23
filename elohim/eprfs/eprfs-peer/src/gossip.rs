//! The gossip ALPN behind admission: only the person's admitted devices are taken as gossip
//! neighbours (spec §3.2).
//!
//! iroh-gossip accepts any peer that dials its ALPN. [`GatedGossip`] sits in front of it on the
//! router and hands it only connections whose authenticated node id admission maps to a device;
//! every other connection is closed unread and counted. Heads are still judged one by one
//! ([`crate::Channel::judge`]): a device dropped by a reload keeps an already-open connection
//! until it closes, and its heads are dropped as unadmitted meanwhile.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use iroh::endpoint::{Connection, VarInt};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh_gossip::net::Gossip;

use crate::connections::{ConnectionBook, NOT_ADMITTED};
use crate::gate::SharedAdmission;

/// iroh-gossip, answering admitted devices only. Mount it under [`iroh_gossip::ALPN`].
#[derive(Debug, Clone)]
pub struct GatedGossip {
    gossip: Gossip,
    admission: SharedAdmission,
    refused: Arc<AtomicU64>,
    book: ConnectionBook,
}

impl GatedGossip {
    pub fn new(gossip: Gossip, admission: SharedAdmission) -> GatedGossip {
        GatedGossip {
            gossip,
            admission,
            refused: Arc::new(AtomicU64::new(0)),
            book: ConnectionBook::default(),
        }
    }

    /// Record accepted connections in `book` and count refusals there, so a reload that drops a
    /// device can close what it holds open.
    pub fn with_book(mut self, book: ConnectionBook) -> GatedGossip {
        self.book = book;
        self
    }

    /// Connections closed because their node id was not admitted.
    pub fn refused(&self) -> u64 {
        self.refused.load(Ordering::Relaxed)
    }
}

impl ProtocolHandler for GatedGossip {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // The handshake proved the peer holds this node id's key. Fail closed on a poisoned lock.
        let remote = connection.remote_node_id()?;
        let admitted = self
            .admission
            .read()
            .is_ok_and(|admission| admission.admits(&remote).is_some());
        if !admitted {
            self.refused.fetch_add(1, Ordering::Relaxed);
            self.book.refused(remote);
            connection.close(VarInt::from_u32(NOT_ADMITTED), b"");
            return Ok(());
        }
        self.book.admitted(remote, &connection);
        self.gossip.accept(connection).await
    }

    async fn shutdown(&self) {
        ProtocolHandler::shutdown(&self.gossip).await;
    }
}
