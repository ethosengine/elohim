//! Elohim protocol handlers
//!
//! Implements the request-response codec for sync protocol messages.
//! Wire format: 4-byte big-endian length prefix + MessagePack payload.

use std::io;

use futures::prelude::*;
use libp2p::request_response;
use libp2p::StreamProtocol;

use crate::sync::protocol::SyncMessage;

/// Protocol identifiers
pub const SYNC_PROTOCOL: &str = "/elohim/sync/1.0.0";
#[allow(dead_code)]
pub const SHARD_PROTOCOL: &str = "/elohim/shard/1.0.0";
#[allow(dead_code)]
pub const CLUSTER_PROTOCOL: &str = "/elohim/cluster/1.0.0";

/// Maximum message size (10 MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Length prefix size (4 bytes big-endian)
const LENGTH_PREFIX_SIZE: usize = 4;

/// Codec for the Elohim sync protocol.
/// Uses MessagePack serialization with length-prefixed framing.
#[derive(Debug, Clone)]
pub struct SyncCodec;

impl SyncCodec {
    #[allow(dead_code)]
    pub fn protocol() -> StreamProtocol {
        StreamProtocol::new(SYNC_PROTOCOL)
    }
}

#[async_trait::async_trait]
impl request_response::Codec for SyncCodec {
    type Protocol = StreamProtocol;
    type Request = SyncMessage;
    type Response = SyncMessage;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_message(io).await
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_message(io).await
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_message(io, &req).await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_message(io, &resp).await
    }
}

/// Read a length-prefixed MessagePack message from the stream.
async fn read_message<T>(io: &mut T) -> io::Result<SyncMessage>
where
    T: AsyncRead + Unpin + Send,
{
    let mut len_buf = [0u8; LENGTH_PREFIX_SIZE];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "message too large: {} bytes (max {})",
                len, MAX_MESSAGE_SIZE
            ),
        ));
    }

    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;

    rmp_serde::from_slice(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("msgpack decode: {}", e)))
}

/// Write a length-prefixed MessagePack message to the stream.
async fn write_message<T>(io: &mut T, msg: &SyncMessage) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
{
    let payload = rmp_serde::to_vec(msg).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("msgpack encode: {}", e))
    })?;

    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "message too large: {} bytes (max {})",
                payload.len(),
                MAX_MESSAGE_SIZE
            ),
        ));
    }

    let len = (payload.len() as u32).to_be_bytes();
    io.write_all(&len).await?;
    io.write_all(&payload).await?;
    io.flush().await?;

    Ok(())
}
