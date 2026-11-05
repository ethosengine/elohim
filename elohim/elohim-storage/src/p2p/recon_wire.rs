//! libp2p request-response framing for `/elohim/inventory-recon/1.0.0`.
//!
//! The protocol and its messages are [`super::inventory_recon`]'s ([`ReconRequest`],
//! [`ReconResponse`], negentropy-v1 pages of at most `RECON_PAGE_LIMIT` bytes); this module
//! only frames them: a 4-byte big-endian length, then the dag-cbor body. A frame longer than
//! one page plus the envelope is refused before it is read (C3 bounded input).

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use std::io;

use super::inventory_recon::{ReconRequest, ReconResponse, RECON_PAGE_LIMIT, RECON_PROTOCOL};

/// Room for the dag-cbor envelope around one page (`recon` id, `sequence`, headers).
const ENVELOPE_ROOM: usize = 256;

/// Largest frame either side reads.
pub const RECON_MAX_FRAME: usize = RECON_PAGE_LIMIT + ENVELOPE_ROOM;

/// The negentropy-v1 reply a responder sends when its step has nothing further to say:
/// the protocol version byte alone (`inventory_recon`'s `VERSION`), which the initiator
/// reads as "no ranges left" and finishes on.
pub const RECON_EMPTY_REPLY: [u8; 1] = [0x61];

#[derive(Debug, Clone)]
pub struct ReconProtocol;

impl AsRef<str> for ReconProtocol {
    fn as_ref(&self) -> &str {
        RECON_PROTOCOL
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReconCodec;

async fn read_frame<T, M>(io: &mut T) -> io::Result<M>
where
    T: AsyncRead + Unpin + Send,
    M: serde::de::DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > RECON_MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("recon frame of {len} bytes (max {RECON_MAX_FRAME})"),
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    serde_ipld_dagcbor::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

async fn write_frame<T, M>(io: &mut T, message: &M) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
    M: serde::Serialize,
{
    let data = serde_ipld_dagcbor::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if data.len() > RECON_MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "recon frame of {} bytes (max {RECON_MAX_FRAME})",
                data.len()
            ),
        ));
    }
    io.write_all(&(data.len() as u32).to_be_bytes()).await?;
    io.write_all(&data).await?;
    io.flush().await
}

#[async_trait]
impl request_response::Codec for ReconCodec {
    type Protocol = ReconProtocol;
    type Request = ReconRequest;
    type Response = ReconResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<ReconRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<ReconResponse>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        request: ReconRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &request).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        response: ReconResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::inventory_recon::{ReconSet, Reconciler};
    use elohim_epr::address::compute_bytes_cid;

    #[tokio::test]
    async fn request_and_response_round_trip_through_the_frame() {
        let cids: Vec<_> = (0u32..50)
            .map(|n| compute_bytes_cid(&n.to_le_bytes()))
            .collect();
        let set = ReconSet::new(&cids);
        let request = ReconRequest::new(Reconciler::initiator(&set).initiate());
        let mut buf = Vec::new();
        write_frame(&mut buf, &request).await.unwrap();
        let read: ReconRequest = read_frame(&mut buf.as_slice()).await.unwrap();
        assert_eq!(read, request);
        read.admit().unwrap();

        let response = ReconResponse {
            msg: RECON_EMPTY_REPLY.to_vec(),
            sequence: 7,
        };
        let mut buf = Vec::new();
        write_frame(&mut buf, &response).await.unwrap();
        let read: ReconResponse = read_frame(&mut buf.as_slice()).await.unwrap();
        assert_eq!(read, response);
    }

    #[tokio::test]
    async fn an_oversize_frame_is_refused_before_it_is_read() {
        let mut buf = ((RECON_MAX_FRAME + 1) as u32).to_be_bytes().to_vec();
        buf.extend(std::iter::repeat_n(0u8, 16));
        let read: io::Result<ReconRequest> = read_frame(&mut buf.as_slice()).await;
        assert_eq!(read.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn the_empty_reply_ends_an_initiator_session() {
        let cids: Vec<_> = (0u32..3)
            .map(|n| compute_bytes_cid(&n.to_le_bytes()))
            .collect();
        let set = ReconSet::new(&cids);
        let step = Reconciler::initiator(&set)
            .reconcile(&RECON_EMPTY_REPLY)
            .expect("the version byte alone is a well-formed message");
        assert!(step.next.is_none());
    }
}
