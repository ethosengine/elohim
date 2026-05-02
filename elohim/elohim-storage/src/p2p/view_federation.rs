//! `/elohim/view-federation/1.0.0` — request-response protocol for federated
//! view-slice fetch across household peers.
//!
//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output
//! in `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Wire-protocol messages, not stored. Federation is gated cryptographically
//! by DHT-notarized `AgentPeerBinding`s — verification happens after response
//! receipt by the `Federator` in `services/federator.rs` (lands in F-T21).
//!
//! ## Wire format
//!
//! Length-prefixed (4-byte big-endian `u32`) MessagePack via `rmp_serde` —
//! identical shape to [`super::blob_protocol`]. The cap is
//! [`MAX_PAYLOAD`] (256 KiB) and is enforced both at length-prefix-read time
//! (rejecting an oversized claim before any allocation) and at
//! serialized-frame-write time (rejecting an oversized payload before
//! sending it).
//!
//! ## Size limits
//!
//! 256 KiB is generous for the two slice kinds the protocol carries today:
//! - `ViewKind::Cluster`: ~2 devices × ~1 KiB metadata each (typical).
//! - `ViewKind::PeerTopology`: a small set of household reciprocation edges.
//!
//! It is also small enough to bound memory from a malicious peer claiming a
//! multi-MB length prefix.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use std::io;

use crate::views::{ViewFederationRequest, ViewFederationResponse};

/// Protocol identifier for federated view-slice fetch.
pub const VIEW_FEDERATION_PROTOCOL_ID: &str = "/elohim/view-federation/1.0.0";

/// Hard cap on a single request OR response payload — 256 KiB.
///
/// Sized for cluster + peer-topology slices today; intentionally not config
/// because the protocol is uniform across peers. If a future slice kind needs
/// more headroom, bump this constant in lockstep with the responder side.
pub const MAX_PAYLOAD: usize = 256 * 1024;

/// Marker type for the federation protocol — implements `AsRef<str>` so
/// `request_response::Behaviour::with_codec` can advertise the protocol ID.
#[derive(Debug, Clone)]
pub struct ViewFederationProtocol;

impl AsRef<str> for ViewFederationProtocol {
    fn as_ref(&self) -> &str {
        VIEW_FEDERATION_PROTOCOL_ID
    }
}

/// Codec for the federation protocol. Length-prefixed MessagePack frames.
#[derive(Debug, Clone, Default)]
pub struct ViewFederationCodec;

#[async_trait]
impl request_response::Codec for ViewFederationCodec {
    type Protocol = ViewFederationProtocol;
    type Request = ViewFederationRequest;
    type Response = ViewFederationResponse;

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
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("view-federation request too large: {len} > {MAX_PAYLOAD}"),
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
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("view-federation response too large: {len} > {MAX_PAYLOAD}"),
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
        let data = rmp_serde::to_vec_named(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if data.len() > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation request exceeds MAX_PAYLOAD: {} > {MAX_PAYLOAD}",
                    data.len()
                ),
            ));
        }
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation request frame too large for u32 prefix: {} bytes",
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
        let data = rmp_serde::to_vec_named(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if data.len() > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation response exceeds MAX_PAYLOAD: {} > {MAX_PAYLOAD}",
                    data.len()
                ),
            ));
        }
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation response frame too large for u32 prefix: {} bytes",
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
    use crate::views::{Freshness, FreshnessState, JsonVal, ViewKind, ViewSlice};
    use libp2p::request_response::Codec;

    #[test]
    fn protocol_id_is_canonical() {
        assert_eq!(
            ViewFederationProtocol.as_ref(),
            "/elohim/view-federation/1.0.0"
        );
    }

    #[test]
    fn request_msgpack_round_trips() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_abc".to_string(),
            request_id: "req_001".to_string(),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.agent_cid, "agent_abc");
        assert_eq!(back.view_kind, ViewKind::Cluster);
        assert_eq!(back.request_id, "req_001");
    }

    #[tokio::test]
    async fn codec_round_trips_request_via_async_buffer() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let req = ViewFederationRequest {
            view_kind: ViewKind::PeerTopology,
            agent_cid: "agent_xyz".to_string(),
            request_id: "req_002".to_string(),
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = Cursor::new(&mut buf);
            codec
                .write_request(&proto, &mut writer, req.clone())
                .await
                .expect("write_request");
        }
        let mut reader = Cursor::new(&buf);
        let back = codec
            .read_request(&proto, &mut reader)
            .await
            .expect("read_request");
        assert_eq!(back.agent_cid, req.agent_cid);
        assert_eq!(back.view_kind, req.view_kind);
        assert_eq!(back.request_id, req.request_id);
    }

    #[tokio::test]
    async fn codec_round_trips_response_via_async_buffer() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let response = ViewFederationResponse {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_abc".to_string(),
            request_id: "req_003".to_string(),
            slice: ViewSlice {
                peer_id: "12D3KooWAAA".to_string(),
                view_kind: ViewKind::Cluster,
                freshness: Freshness {
                    state: FreshnessState::Live,
                    stale_since_ms: None,
                },
                payload: JsonVal(serde_json::json!({"devices": []})),
                signature: "sig-base64".to_string(),
            },
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = Cursor::new(&mut buf);
            codec
                .write_response(&proto, &mut writer, response.clone())
                .await
                .expect("write_response");
        }
        let mut reader = Cursor::new(&buf);
        let back = codec
            .read_response(&proto, &mut reader)
            .await
            .expect("read_response");
        assert_eq!(back.agent_cid, response.agent_cid);
        assert_eq!(back.request_id, response.request_id);
        assert_eq!(back.view_kind, response.view_kind);
        assert_eq!(back.slice.peer_id, response.slice.peer_id);
        assert_eq!(back.slice.signature, response.slice.signature);
    }

    #[tokio::test]
    async fn codec_rejects_oversized_request_length_prefix() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        // The codec must reject before it allocates the body buffer.
        let oversized_len: u32 = (MAX_PAYLOAD as u32) + 1;
        let mut input = Vec::new();
        input.extend_from_slice(&oversized_len.to_be_bytes());
        let mut reader = Cursor::new(&input);
        let result = codec.read_request(&proto, &mut reader).await;
        assert!(
            result.is_err(),
            "codec should reject request length prefix > MAX_PAYLOAD"
        );
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn codec_rejects_oversized_response_length_prefix() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let oversized_len: u32 = (MAX_PAYLOAD as u32) + 1;
        let mut input = Vec::new();
        input.extend_from_slice(&oversized_len.to_be_bytes());
        let mut reader = Cursor::new(&input);
        let result = codec.read_response(&proto, &mut reader).await;
        assert!(
            result.is_err(),
            "codec should reject response length prefix > MAX_PAYLOAD"
        );
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
