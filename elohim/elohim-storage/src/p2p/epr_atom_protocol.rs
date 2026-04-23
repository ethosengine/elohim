//! EPR Atom Protocol — Request-response protocol for signed EPR atoms.
//!
//! Carries CBOR-encoded `elohim_epr::Envelope` bytes between peers. Coexists
//! with the legacy `/elohim/epr/1.0.0` (which serves EprHead via MessagePack).
//!
//! Wire format: 4-byte BE length prefix + CBOR body.
//!
//! This module defines transient wire types — they live only during a single
//! request-response exchange. No persistent source of truth is introduced.
//! The notarized source of truth remains the ed25519-signed CBOR Envelope
//! (content-addressed by CID). The `epr_atoms` table (Phase 2a) is its
//! operational projection.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-04-23-epr-phase-2c-libp2p-federation-design.md`

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for EPR atom federation.
pub const EPR_ATOM_PROTOCOL_ID: &str = "/elohim/epr-atom/1.0.0";

/// Max request size: 256 KB (accommodates FetchBatch of ~100 CIDs).
pub const MAX_REQUEST_SIZE: usize = 256 * 1024;

/// Max response size: 2 MB (headroom for future atom payload growth).
pub const MAX_RESPONSE_SIZE: usize = 2 * 1024 * 1024;

/// Max CIDs per batch request.
pub const MAX_BATCH_CIDS: usize = 128;

/// Protocol marker for libp2p negotiation.
#[derive(Debug, Clone)]
pub struct EprAtomProtocol;

impl AsRef<str> for EprAtomProtocol {
    fn as_ref(&self) -> &str {
        EPR_ATOM_PROTOCOL_ID
    }
}

/// Request variants — transient wire types (no persistent source of truth).
/// `tag` is the CBOR discriminator; shape matches the wire contract in
/// `elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum EprAtomRequest {
    /// Fetch a single atom by CID.
    Fetch { cid: String },
    /// Announce a new atom (push). Body is raw CBOR envelope bytes.
    Announce {
        #[serde(with = "serde_bytes")]
        envelope_bytes: Vec<u8>,
    },
    /// Fetch multiple atoms in one request. Bounded by `MAX_BATCH_CIDS`.
    FetchBatch { cids: Vec<String> },
}

/// Response variants — transient wire types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum EprAtomResponse {
    /// Single atom (raw CBOR envelope bytes).
    Atom {
        #[serde(with = "serde_bytes")]
        envelope_bytes: Vec<u8>,
    },
    /// Batch response — one entry per requested CID, `None` for missing/unauthorized.
    AtomBatch {
        #[serde(with = "serde_bytes_vec")]
        atoms: Vec<Option<Vec<u8>>>,
    },
    /// Ack for AnnounceAtom.
    Announced {
        accepted: bool,
        reason: Option<String>,
    },
    /// Atom missing OR reach gate failed (leak-free — caller can't distinguish).
    NotFound,
    /// Protocol-level error (malformed request, batch too large, etc.).
    Error { message: String },
}

/// Helper for `#[serde(with = "...")]` over `Vec<Option<Vec<u8>>>`.
/// Each `Some(bytes)` is serialized as a CBOR byte string; `None` as null.
mod serde_bytes_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_bytes::ByteBuf;

    pub fn serialize<S>(v: &[Option<Vec<u8>>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mapped: Vec<Option<ByteBuf>> = v
            .iter()
            .map(|o| o.as_ref().map(|b| ByteBuf::from(b.clone())))
            .collect();
        mapped.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Option<Vec<u8>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mapped: Vec<Option<ByteBuf>> = Vec::deserialize(d)?;
        Ok(mapped
            .into_iter()
            .map(|o| o.map(|b| b.into_vec()))
            .collect())
    }
}

/// Codec for the EPR atom protocol. CBOR body + 4-byte BE length prefix.
#[derive(Debug, Clone, Default)]
pub struct EprAtomCodec;

#[async_trait]
impl request_response::Codec for EprAtomCodec {
    type Protocol = EprAtomProtocol;
    type Request = EprAtomRequest;
    type Response = EprAtomResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_cbor(io, MAX_REQUEST_SIZE).await
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_cbor(io, MAX_RESPONSE_SIZE).await
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
        write_cbor(io, &request, MAX_REQUEST_SIZE).await
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
        write_cbor(io, &response, MAX_RESPONSE_SIZE).await
    }
}

async fn read_cbor<T, V>(io: &mut T, max_size: usize) -> io::Result<V>
where
    T: AsyncRead + Unpin + Send,
    V: serde::de::DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > max_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "epr-atom message too large: {} bytes (max {})",
                len, max_size
            ),
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    ciborium::de::from_reader(&buf[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("cbor decode: {}", e)))
}

async fn write_cbor<T, V>(io: &mut T, value: &V, max_size: usize) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
    V: serde::Serialize,
{
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("cbor encode: {}", e)))?;
    if buf.len() > max_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "outgoing epr-atom message too large: {} bytes (max {})",
                buf.len(),
                max_size
            ),
        ));
    }
    let len_buf = (buf.len() as u32).to_be_bytes();
    io.write_all(&len_buf).await?;
    io.write_all(&buf).await?;
    io.flush().await?;
    Ok(())
}
