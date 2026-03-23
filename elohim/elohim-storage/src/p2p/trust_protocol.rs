//! Trust Negotiation Protocol — per-connection credential exchange
//!
//! On ConnectionEstablished, peers exchange trust credentials (CIDs of
//! memberships, relationships, attestations, stewardship). The receiving
//! peer verifies CIDs against the DHT via conductor, caches the result,
//! and returns the verified reach ceiling + TTL.
//!
//! Wire format: 4-byte BE length prefix + MessagePack body.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

pub const TRUST_PROTOCOL_ID: &str = "/elohim/trust/1.0.0";

const MAX_REQUEST_SIZE: usize = 64 * 1024;
const MAX_RESPONSE_SIZE: usize = 4 * 1024;

#[derive(Debug, Clone)]
pub struct TrustProtocol;

impl AsRef<str> for TrustProtocol {
    fn as_ref(&self) -> &str {
        TRUST_PROTOCOL_ID
    }
}

/// Trust handshake request — peer presents credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustHandshake {
    pub agent_pubkey: String,
    pub membership_cids: Vec<String>,
    pub relationship_cids: Vec<String>,
    pub attestation_cids: Vec<String>,
    pub stewardship_cids: Vec<String>,
}

/// Trust handshake response — verified reach ceiling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustResponse {
    Verified {
        reach_ceiling: String,
        ttl_seconds: u64,
    },
    Rejected {
        reason: String,
    },
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct TrustCodec;

#[async_trait]
impl request_response::Codec for TrustCodec {
    type Protocol = TrustProtocol;
    type Request = TrustHandshake;
    type Response = TrustResponse;

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

        if len > MAX_REQUEST_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Trust request too large: {} bytes (max {})",
                    len, MAX_REQUEST_SIZE
                ),
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

        if len > MAX_RESPONSE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Trust response too large: {} bytes (max {})",
                    len, MAX_RESPONSE_SIZE
                ),
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
        let data = rmp_serde::to_vec(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
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
        let data = rmp_serde::to_vec(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_roundtrip() {
        let handshake = TrustHandshake {
            agent_pubkey: "uhCAk_test".to_string(),
            membership_cids: vec!["bafkrei-mem1".to_string()],
            relationship_cids: vec![],
            attestation_cids: vec!["bafkrei-att1".to_string()],
            stewardship_cids: vec![],
        };
        let bytes = rmp_serde::to_vec(&handshake).unwrap();
        let decoded: TrustHandshake = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.agent_pubkey, "uhCAk_test");
        assert_eq!(decoded.membership_cids.len(), 1);
        assert_eq!(decoded.attestation_cids.len(), 1);
    }

    #[test]
    fn response_verified_roundtrip() {
        let resp = TrustResponse::Verified {
            reach_ceiling: "trusted".to_string(),
            ttl_seconds: 3600,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: TrustResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            TrustResponse::Verified {
                reach_ceiling,
                ttl_seconds,
            } => {
                assert_eq!(reach_ceiling, "trusted");
                assert_eq!(ttl_seconds, 3600);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn response_rejected_roundtrip() {
        let resp = TrustResponse::Rejected {
            reason: "invalid agent".to_string(),
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: TrustResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            TrustResponse::Rejected { reason } => assert_eq!(reason, "invalid agent"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn response_error_roundtrip() {
        let resp = TrustResponse::Error("conductor unavailable".to_string());
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: TrustResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            TrustResponse::Error(msg) => assert_eq!(msg, "conductor unavailable"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn empty_handshake_roundtrip() {
        let handshake = TrustHandshake {
            agent_pubkey: "uhCAk_minimal".to_string(),
            membership_cids: vec![],
            relationship_cids: vec![],
            attestation_cids: vec![],
            stewardship_cids: vec![],
        };
        let bytes = rmp_serde::to_vec(&handshake).unwrap();
        let decoded: TrustHandshake = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.agent_pubkey, "uhCAk_minimal");
        assert!(decoded.membership_cids.is_empty());
    }
}
