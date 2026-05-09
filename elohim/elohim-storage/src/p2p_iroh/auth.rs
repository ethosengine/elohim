//! Phase 9 — identity / trust auth planes over iroh QUIC.
//!
//! Two ALPNs:
//! - [`IDENTITY_HANDSHAKE_ALPN`] = `/elohim/identity-handshake/2.0.0`
//! - [`TRUST_ALPN`] = `/elohim/trust/2.0.0`
//!
//! Wire types reused unchanged from [`crate::p2p::identity_handshake`]
//! and [`crate::p2p::trust_protocol`] respectively. MessagePack body,
//! length-prefixed via [`super::codec`]. 4 KiB / 1 KiB caps for identity
//! handshake match the libp2p side; 64 KiB / 4 KiB for trust.
//!
//! ## Sketch
//!
//! Phase 9 ships wire-protocol scaffolding for the auth and trust
//! planes. Backend dispatch (binding verification, anti-replay, trust
//! ceiling computation) lives in trait objects supplied by the daemon
//! at Phase 11 cutover. Reach-authorization is an internal service
//! that consumes identity context — no separate ALPN needed.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};

use super::codec::{read_frame, write_frame};
use crate::p2p::identity_handshake::{IdentityHandshakeRequest, IdentityHandshakeResponse};
use crate::p2p::trust_protocol::{TrustHandshake, TrustResponse};

// ============================================================================
// Identity handshake plane
// ============================================================================

pub const IDENTITY_HANDSHAKE_ALPN: &[u8] = b"/elohim/identity-handshake/2.0.0";

/// Caps — match `crate::p2p::identity_handshake::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE}`.
const IDENTITY_HANDSHAKE_REQ_MAX: usize = 4 * 1024;
const IDENTITY_HANDSHAKE_RES_MAX: usize = 1024;

#[async_trait::async_trait]
pub trait IdentityHandshakeBackend: Send + Sync + 'static {
    async fn handle(&self, req: IdentityHandshakeRequest) -> IdentityHandshakeResponse;
}

#[derive(Clone)]
pub struct IrohIdentityHandshakeProtocol {
    backend: Arc<dyn IdentityHandshakeBackend>,
}

impl IrohIdentityHandshakeProtocol {
    pub fn new(backend: Arc<dyn IdentityHandshakeBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohIdentityHandshakeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohIdentityHandshakeProtocol")
            .finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohIdentityHandshakeProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()),
            };
            let req: IdentityHandshakeRequest = read_frame(&mut recv, IDENTITY_HANDSHAKE_REQ_MAX)
                .await
                .map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame(&mut send, &res).await.map_err(io_to_accept)?;
            send.finish()
                .map_err(|e| AcceptError::from_err(io::Error::other(e.to_string())))?;
        }
    }
}

pub struct IrohIdentityHandshakeClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohIdentityHandshakeClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(
        &self,
        peer: NodeAddr,
        req: &IdentityHandshakeRequest,
    ) -> Result<IdentityHandshakeResponse> {
        let conn = self.endpoint.connect(peer, IDENTITY_HANDSHAKE_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: IdentityHandshakeResponse =
            read_frame(&mut recv, IDENTITY_HANDSHAKE_RES_MAX).await?;
        Ok(res)
    }
}

// ============================================================================
// Trust handshake plane
// ============================================================================

pub const TRUST_ALPN: &[u8] = b"/elohim/trust/2.0.0";

/// Caps — match `crate::p2p::trust_protocol::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE}`.
const TRUST_REQ_MAX: usize = 64 * 1024;
const TRUST_RES_MAX: usize = 4 * 1024;

#[async_trait::async_trait]
pub trait TrustBackend: Send + Sync + 'static {
    async fn handle(&self, req: TrustHandshake) -> TrustResponse;
}

#[derive(Clone)]
pub struct IrohTrustProtocol {
    backend: Arc<dyn TrustBackend>,
}

impl IrohTrustProtocol {
    pub fn new(backend: Arc<dyn TrustBackend>) -> Self {
        Self { backend }
    }
}

impl std::fmt::Debug for IrohTrustProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohTrustProtocol").finish_non_exhaustive()
    }
}

impl ProtocolHandler for IrohTrustProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => return Ok(()),
            };
            let req: TrustHandshake = read_frame(&mut recv, TRUST_REQ_MAX)
                .await
                .map_err(io_to_accept)?;
            let res = self.backend.handle(req).await;
            write_frame(&mut send, &res).await.map_err(io_to_accept)?;
            send.finish()
                .map_err(|e| AcceptError::from_err(io::Error::other(e.to_string())))?;
        }
    }
}

pub struct IrohTrustClient<'a> {
    endpoint: &'a Endpoint,
}

impl<'a> IrohTrustClient<'a> {
    pub fn new(endpoint: &'a Endpoint) -> Self {
        Self { endpoint }
    }

    pub async fn request(&self, peer: NodeAddr, req: &TrustHandshake) -> Result<TrustResponse> {
        let conn = self.endpoint.connect(peer, TRUST_ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, req).await?;
        send.finish()?;
        let res: TrustResponse = read_frame(&mut recv, TRUST_RES_MAX).await?;
        Ok(res)
    }
}

fn io_to_accept(e: io::Error) -> AcceptError {
    AcceptError::from_err(e)
}
