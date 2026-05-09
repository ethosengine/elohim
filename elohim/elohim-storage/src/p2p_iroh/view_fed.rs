//! Phase 8 — view-federation plane over iroh QUIC.
//!
//! Custom ALPN: [`VIEW_FED_ALPN`] (`/elohim/view-federation/2.0.0`).
//! Wire types — [`ViewFederationRequest`] / [`ViewFederationResponse`]
//! — reused unchanged from [`crate::views`]. MessagePack body, length-
//! prefixed via [`super::codec`]. 256 KiB cap matches the libp2p side's
//! `MAX_PAYLOAD`.
//!
//! ## Sketch
//!
//! Phase 8 ships wire-protocol scaffolding. Backend dispatch (slice
//! lookup, signing, freshness state) lives in a [`ViewFederationBackend`]
//! trait object — the production daemon supplies the real dispatch
//! (see `crate::p2p::view_federation::build_response_slice`) at Phase 11
//! cutover.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};

use super::codec::{read_frame, write_frame};
use crate::views::{ViewFederationRequest, ViewFederationResponse};

/// Iroh-side ALPN. Distinct from libp2p's
/// `/elohim/view-federation/1.0.0` — version 2.0.0 marks "iroh transport,
/// identical MessagePack payloads."
pub const VIEW_FED_ALPN: &[u8] = b"/elohim/view-federation/2.0.0";

/// 256 KiB cap on a single inbound view-federation frame, matching the
/// libp2p side's `MAX_PAYLOAD` (`crate::p2p::view_federation::MAX_PAYLOAD`).
pub const MAX_PAYLOAD: usize = 256 * 1024;

#[async_trait::async_trait]
pub trait ViewFederationBackend: Send + Sync + 'static {
    async fn handle(&self, req: ViewFederationRequest) -> ViewFederationResponse;
}

#[derive(Clone)]
pub struct IrohViewFederationProtocol {
    backend: Arc<dyn ViewFederationBackend>,
}

impl IrohViewFederationProtocol {
    pub fn new(backend: Arc<dyn ViewFederationBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohViewFederationProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohViewFederationProtocol")
            .finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohViewFederationProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()),
            };
            let req: ViewFederationRequest = read_frame(&mut recv, MAX_PAYLOAD)
                .await
                .map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame(&mut send, &res).await.map_err(io_to_accept)?;
            send.finish()
                .map_err(|e| AcceptError::from_err(io::Error::other(e.to_string())))?;
        }
    }
}

pub struct IrohViewFederationClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohViewFederationClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(
        &self,
        peer: NodeAddr,
        req: &ViewFederationRequest,
    ) -> Result<ViewFederationResponse> {
        let conn = self.endpoint.connect(peer, VIEW_FED_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: ViewFederationResponse = read_frame(&mut recv, MAX_PAYLOAD).await?;
        Ok(res)
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}
