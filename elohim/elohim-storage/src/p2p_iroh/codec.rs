//! Length-prefixed MessagePack frame codec for iroh QUIC streams.
//!
//! All custom-ALPN protocols on the iroh path share this wire format —
//! identical to the libp2p side's `request_response::Codec` impls in
//! [`crate::p2p::blob_protocol`], [`crate::p2p::sync_protocol`], etc.
//! Cutover removes one transport, never two divergent message schemas.
//!
//! Wire format:
//! ```text
//! +---------+-------------------+
//! | u32 BE  |  MessagePack body |
//! | length  |    (length bytes) |
//! +---------+-------------------+
//! ```

use std::io;

use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Default cap for inbound frames. Individual protocols may override this
/// — set higher for shard-style payloads, lower for control messages.
pub const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MiB

/// Hard upper bound on inbound frame size to prevent runaway allocation.
/// No protocol may legitimately exceed this. Mirrors libp2p side's
/// `HARD_MAX_RESPONSE_SIZE` convention.
pub const HARD_MAX_FRAME_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// Read a length-prefixed MessagePack frame from a stream.
///
/// `max_size` caps the inbound length and is itself capped at
/// [`HARD_MAX_FRAME_SIZE`]. Use the `read_frame_default` shortcut when the
/// caller is OK with [`DEFAULT_MAX_FRAME_SIZE`].
pub async fn read_frame<T, R>(stream: &mut R, max_size: usize) -> io::Result<T>
where
    T: DeserializeOwned,
    R: AsyncRead + Unpin,
{
    let max = max_size.min(HARD_MAX_FRAME_SIZE);

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {max}"),
        ));
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Read a frame using [`DEFAULT_MAX_FRAME_SIZE`].
pub async fn read_frame_default<T, R>(stream: &mut R) -> io::Result<T>
where
    T: DeserializeOwned,
    R: AsyncRead + Unpin,
{
    read_frame(stream, DEFAULT_MAX_FRAME_SIZE).await
}

/// Write a length-prefixed MessagePack frame to a stream.
pub async fn write_frame<T, W>(stream: &mut W, value: &T) -> io::Result<()>
where
    T: Serialize,
    W: AsyncWrite + Unpin,
{
    let body = rmp_serde::to_vec_named(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let len = u32::try_from(body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "frame body exceeds u32::MAX"))?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tokio::io::duplex;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Hello {
        msg: String,
        n: u32,
    }

    #[tokio::test]
    async fn round_trips_through_duplex() {
        let (mut a, mut b) = duplex(64 * 1024);
        let payload = Hello {
            msg: "hi".into(),
            n: 42,
        };

        let payload_clone = Hello {
            msg: payload.msg.clone(),
            n: payload.n,
        };
        let writer = tokio::spawn(async move {
            write_frame(&mut a, &payload_clone).await.unwrap();
            // Drop a to close
        });
        let received: Hello = read_frame_default(&mut b).await.unwrap();

        writer.await.unwrap();
        assert_eq!(received, payload);
    }

    #[tokio::test]
    async fn rejects_oversized_frame() {
        let (mut a, mut b) = duplex(64 * 1024);

        // Write a frame claiming 1 GiB length.
        tokio::spawn(async move {
            let big_len: u32 = 1_000_000_000;
            tokio::io::AsyncWriteExt::write_all(&mut a, &big_len.to_be_bytes())
                .await
                .ok();
        });

        let result: io::Result<Hello> = read_frame(&mut b, 1024).await;
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("frame too large"));
    }

    #[tokio::test]
    async fn caps_at_hard_max() {
        // Even when caller passes a higher max, we cap at HARD_MAX_FRAME_SIZE.
        let (mut a, mut b) = duplex(64);

        tokio::spawn(async move {
            let big_len: u32 = (HARD_MAX_FRAME_SIZE as u32) + 1;
            tokio::io::AsyncWriteExt::write_all(&mut a, &big_len.to_be_bytes())
                .await
                .ok();
        });

        let result: io::Result<Hello> = read_frame(&mut b, usize::MAX).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn truncated_frame_returns_io_error() {
        let (mut a, mut b) = duplex(64);

        // Write a header claiming 100 bytes but send only 4 bytes of body.
        tokio::spawn(async move {
            let len: u32 = 100;
            tokio::io::AsyncWriteExt::write_all(&mut a, &len.to_be_bytes())
                .await
                .ok();
            tokio::io::AsyncWriteExt::write_all(&mut a, b"abcd")
                .await
                .ok();
            // Drop a to signal EOF
        });

        let result: io::Result<Hello> = read_frame_default(&mut b).await;
        let err = result.unwrap_err();
        // read_exact on truncated stream → UnexpectedEof
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn write_then_read_preserves_complex_struct() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Nested {
            outer: Vec<u8>,
            inner: Hello,
        }
        let (mut a, mut b) = duplex(64 * 1024);
        let payload = Nested {
            outer: vec![1, 2, 3, 4, 5],
            inner: Hello {
                msg: "deep".into(),
                n: 7,
            },
        };
        let payload_clone = Nested {
            outer: payload.outer.clone(),
            inner: Hello {
                msg: payload.inner.msg.clone(),
                n: payload.inner.n,
            },
        };
        tokio::spawn(async move {
            write_frame(&mut a, &payload_clone).await.unwrap();
        });
        let got: Nested = read_frame_default(&mut b).await.unwrap();
        assert_eq!(got, payload);
    }
}
