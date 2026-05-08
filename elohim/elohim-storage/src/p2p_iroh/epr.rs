//! Phase 6 — EPR resolution + EPR-atom planes over iroh QUIC.
//!
//! Two ALPNs share this module because they're a single conceptual
//! plane (Elohim Protocol Resource resolution + transient atom transfer)
//! that's split into separate request/response handlers in the libp2p
//! stack. The wire types are reused unchanged from
//! [`crate::p2p::epr_protocol`] and [`crate::p2p::epr_atom_protocol`].
//!
//! ## Encoding
//!
//! - [`EPR_ALPN`] (`/elohim/epr/2.0.0`): MessagePack (rmp_serde) — same
//!   as libp2p `EprCodec`.
//! - [`EPR_ATOM_ALPN`] (`/elohim/epr-atom/2.0.0`): CBOR (ciborium) — same
//!   as libp2p `EprAtomCodec`. EPR-atom canonical bytes are CBOR per the
//!   schema in `elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json`.
//!
//! Both protocols use the shared length-prefix codec ([`super::codec`]):
//! 4-byte BE length + body bytes. Encoding differs, framing does not.
//!
//! ## Sketch
//!
//! Phase 6 ships wire-protocol scaffolding. Backend dispatch (resolver
//! lookups, atom store reads, integrity-notify routing) lives in trait
//! objects supplied by the daemon. Phase 11 wires the real backends.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};

use super::codec::{read_frame_cbor_default, read_frame_default, write_frame, write_frame_cbor};
use crate::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse};
use crate::p2p::epr_protocol::{EprRequest, EprResponse};

/// Iroh-side ALPN for EPR resolution. Distinct from libp2p's
/// `/elohim/epr/1.0.0` — version 2.0.0 marks "iroh transport, identical
/// MessagePack payloads."
pub const EPR_ALPN: &[u8] = b"/elohim/epr/2.0.0";

/// Iroh-side ALPN for EPR-atom transfer. CBOR-encoded, identical to the
/// libp2p `EprAtomProtocol` (1.0.0) wire shape.
pub const EPR_ATOM_ALPN: &[u8] = b"/elohim/epr-atom/2.0.0";

// ============================================================================
// EPR resolution plane (MessagePack)
// ============================================================================

#[async_trait::async_trait]
pub trait EprBackend: Send + Sync + 'static {
    async fn handle(&self, req: EprRequest) -> EprResponse;
}

/// [`ProtocolHandler`] for EPR resolution. One bidi stream =
/// one [`EprRequest`] → one [`EprResponse`] round-trip.
#[derive(Clone)]
pub struct IrohEprProtocol {
    backend: Arc<dyn EprBackend>,
}

impl IrohEprProtocol {
    pub fn new(backend: Arc<dyn EprBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohEprProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohEprProtocol").finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohEprProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()),
            };
            let req: EprRequest =
                read_frame_default(&mut recv).await.map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame(&mut send, &res).await.map_err(io_to_accept)?;
            send.finish().map_err(|e| {
                AcceptError::from_err(io::Error::new(io::ErrorKind::Other, e.to_string()))
            })?;
        }
    }
}

pub struct IrohEprClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohEprClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(&self, peer: NodeAddr, req: &EprRequest) -> Result<EprResponse> {
        let conn = self.endpoint.connect(peer, EPR_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: EprResponse = read_frame_default(&mut recv).await?;
        Ok(res)
    }
}

// ============================================================================
// EPR-atom plane (CBOR)
// ============================================================================

#[async_trait::async_trait]
pub trait EprAtomBackend: Send + Sync + 'static {
    async fn handle(&self, req: EprAtomRequest) -> EprAtomResponse;
}

#[derive(Clone)]
pub struct IrohEprAtomProtocol {
    backend: Arc<dyn EprAtomBackend>,
}

impl IrohEprAtomProtocol {
    pub fn new(backend: Arc<dyn EprAtomBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohEprAtomProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohEprAtomProtocol")
            .finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohEprAtomProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()),
            };
            let req: EprAtomRequest = read_frame_cbor_default(&mut recv)
                .await
                .map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame_cbor(&mut send, &res)
                .await
                .map_err(io_to_accept)?;
            send.finish().map_err(|e| {
                AcceptError::from_err(io::Error::new(io::ErrorKind::Other, e.to_string()))
            })?;
        }
    }
}

pub struct IrohEprAtomClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohEprAtomClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(&self, peer: NodeAddr, req: &EprAtomRequest) -> Result<EprAtomResponse> {
        let conn = self.endpoint.connect(peer, EPR_ATOM_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame_cbor(&mut send, req).await?;
        send.finish()?;
        let res: EprAtomResponse = read_frame_cbor_default(&mut recv).await?;
        Ok(res)
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}
