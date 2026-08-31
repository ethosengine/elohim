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

/// Cap on a single inbound view-federation frame — now ACTUALLY matching the
/// libp2p side's `MAX_PAYLOAD` (`crate::p2p::view_federation::MAX_PAYLOAD`,
/// 1 MiB).
///
/// DRIFT CURED (2026-08-31, measured live): this constant was 256 KiB while
/// its own comment claimed parity with libp2p's 1 MiB — so every responder
/// that trimmed to the 1 MiB budget produced frames an iroh READER refused
/// (`frame too large: 536378 > 262144`, workspace peer W2 pulling fleet
/// inventory). Fleet-to-fleet traffic survived on the libp2p leg; an
/// iroh-only peer failed EVERY oversized exchange. The robustness split:
/// readers are LIBERAL (this 1 MiB cap), senders are CONSERVATIVE (the
/// inventory trim budget stays under the 256 KiB floor deployed fleet
/// readers still enforce — see `INVENTORY_PAYLOAD_BUDGET`).
pub const MAX_PAYLOAD: usize = crate::p2p::view_federation::MAX_PAYLOAD;

/// The frame cap OLD deployed iroh readers (pre-2026-08-31 binaries) still
/// enforce. Senders that must be readable by a mixed fleet size their
/// payloads under THIS, not under [`MAX_PAYLOAD`].
pub const DEPLOYED_READER_FLOOR: usize = 256 * 1024;

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
