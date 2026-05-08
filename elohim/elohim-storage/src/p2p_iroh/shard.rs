//! Phase 7 — shard transfer plane over iroh QUIC.
//!
//! Custom ALPN: [`SHARD_ALPN`] (`/elohim/shard/2.0.0`). Wire types
//! reused unchanged from [`crate::p2p::shard_protocol`]:
//! [`ShardRequest`] / [`ShardResponse`] / [`ContentInventoryItem`] /
//! [`ContentRecord`].
//!
//! Reed-Solomon coding logic stays in pure Rust (transport-agnostic);
//! this phase only migrates the request/response framing.
//!
//! ## Sketch
//!
//! Phase 7 ships wire-protocol scaffolding. Backend dispatch (shard
//! store reads, content lookup, push validation) is supplied as a
//! [`ShardBackend`] trait object. Phase 11 cutover wires the real
//! backend.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};

use super::codec::{read_frame_default, write_frame};
use crate::p2p::shard_protocol::{ShardRequest, ShardResponse};

/// Iroh-side ALPN for shard transfer. Distinct from libp2p's
/// `/elohim/shard/1.0.0` — version 2.0.0 marks "iroh transport,
/// identical MessagePack payloads."
///
/// Push-style payloads can hit the codec's [`super::codec::DEFAULT_MAX_FRAME_SIZE`]
/// (16 MiB) — sufficient for current shard sizes. Larger payloads need
/// `read_frame(stream, custom_cap)` with a higher ceiling.
pub const SHARD_ALPN: &[u8] = b"/elohim/shard/2.0.0";

#[async_trait::async_trait]
pub trait ShardBackend: Send + Sync + 'static {
    async fn handle(&self, req: ShardRequest) -> ShardResponse;
}

#[derive(Clone)]
pub struct IrohShardProtocol {
    backend: Arc<dyn ShardBackend>,
}

impl IrohShardProtocol {
    pub fn new(backend: Arc<dyn ShardBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohShardProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohShardProtocol").finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohShardProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await?;
        let req: ShardRequest = read_frame_default(&mut recv).await.map_err(io_to_accept)?;
        let res = self.backend.handle(req).await;
        write_frame(&mut send, &res).await.map_err(io_to_accept)?;
        send.finish().map_err(|e| {
            AcceptError::from_err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        })?;
        connection.closed().await;
        Ok(())
    }
}

pub struct IrohShardClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohShardClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(&self, peer: NodeAddr, req: &ShardRequest) -> Result<ShardResponse> {
        let conn = self.endpoint.connect(peer, SHARD_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: ShardResponse = read_frame_default(&mut recv).await?;
        Ok(res)
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}
