//! EPR Protocol - Request-response protocol for EPR Head resolution
//!
//! Enables cross-peer content discovery: peers publish EPR Heads to the DHT
//! on ingestion, and other peers resolve them via Kademlia lookup + direct
//! request-response.
//!
//! Wire format: 4-byte BE length prefix + MessagePack body.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for EPR resolution
pub const EPR_PROTOCOL_ID: &str = "/elohim/epr/1.0.0";

/// Max request size: 1MB
const MAX_REQUEST_SIZE: usize = 1024 * 1024;

/// Max response size: 64KB (EPR Heads are ~500B; batch of 100 is ~50KB)
const MAX_RESPONSE_SIZE: usize = 64 * 1024;

/// EPR protocol definition
#[derive(Debug, Clone)]
pub struct EprProtocol;

impl AsRef<str> for EprProtocol {
    fn as_ref(&self) -> &str {
        EPR_PROTOCOL_ID
    }
}

/// EPR request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EprRequest {
    /// Resolve an EPR Head by content ID
    Resolve {
        id: String,
        agent_pubkey: Option<String>,
    },
    /// Announce an EPR Head to the network (MessagePack-encoded EprHead)
    Announce { head: Vec<u8> },
    /// Resolve multiple EPR Heads in one request
    ResolveBatch { ids: Vec<String> },
    /// Get document-tier content by ID (stub — not yet implemented)
    GetDocument { id: String },
}

/// EPR response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EprResponse {
    /// Single EPR Head (MessagePack-encoded)
    Head(Vec<u8>),
    /// Batch of EPR Heads (MessagePack-encoded)
    HeadBatch(Vec<Vec<u8>>),
    /// Announcement acknowledgment
    Announced {
        accepted: bool,
        reason: Option<String>,
    },
    /// Content not found
    NotFound,
    /// Access denied — reach gate failed
    AccessDenied {
        required_reach: String,
        reason: String,
    },
    /// Error
    Error(String),
}

/// Codec for EPR request/response
#[derive(Debug, Clone, Default)]
pub struct EprCodec;

#[async_trait]
impl request_response::Codec for EprCodec {
    type Protocol = EprProtocol;
    type Request = EprRequest;
    type Response = EprResponse;

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
                    "EPR request too large: {} bytes (max {})",
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
                    "EPR response too large: {} bytes (max {})",
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
    fn test_epr_request_resolve_roundtrip() {
        let request = EprRequest::Resolve {
            id: "fct-module-01-church-dilemma".to_string(),
            agent_pubkey: Some("uhCAk_test_agent".to_string()),
        };
        let bytes = rmp_serde::to_vec(&request).unwrap();
        let decoded: EprRequest = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprRequest::Resolve { id, agent_pubkey } => {
                assert_eq!(id, "fct-module-01-church-dilemma");
                assert_eq!(agent_pubkey.unwrap(), "uhCAk_test_agent");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_request_announce_roundtrip() {
        let head_bytes = vec![0xA0, 0x01, 0x02, 0x03];
        let request = EprRequest::Announce {
            head: head_bytes.clone(),
        };
        let bytes = rmp_serde::to_vec(&request).unwrap();
        let decoded: EprRequest = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprRequest::Announce { head } => assert_eq!(head, head_bytes),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_request_resolve_batch_roundtrip() {
        let request = EprRequest::ResolveBatch {
            ids: vec!["id-1".to_string(), "id-2".to_string(), "id-3".to_string()],
        };
        let bytes = rmp_serde::to_vec(&request).unwrap();
        let decoded: EprRequest = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprRequest::ResolveBatch { ids } => {
                assert_eq!(ids.len(), 3);
                assert_eq!(ids[0], "id-1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_request_get_document_roundtrip() {
        let request = EprRequest::GetDocument {
            id: "doc-42".to_string(),
        };
        let bytes = rmp_serde::to_vec(&request).unwrap();
        let decoded: EprRequest = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprRequest::GetDocument { id } => assert_eq!(id, "doc-42"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_head_roundtrip() {
        let head_bytes = vec![0x01, 0x02, 0x03, 0x04];
        let response = EprResponse::Head(head_bytes.clone());
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::Head(data) => assert_eq!(data, head_bytes),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_head_batch_roundtrip() {
        let response = EprResponse::HeadBatch(vec![vec![0x01, 0x02], vec![0x03, 0x04]]);
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::HeadBatch(batch) => {
                assert_eq!(batch.len(), 2);
                assert_eq!(batch[0], vec![0x01, 0x02]);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_announced_roundtrip() {
        let response = EprResponse::Announced {
            accepted: true,
            reason: None,
        };
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::Announced { accepted, reason } => {
                assert!(accepted);
                assert!(reason.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_announced_with_reason_roundtrip() {
        let response = EprResponse::Announced {
            accepted: false,
            reason: Some("invalid format".to_string()),
        };
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::Announced { accepted, reason } => {
                assert!(!accepted);
                assert_eq!(reason.unwrap(), "invalid format");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_not_found_roundtrip() {
        let response = EprResponse::NotFound;
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert!(matches!(decoded, EprResponse::NotFound));
    }

    #[test]
    fn test_epr_response_access_denied_roundtrip() {
        let response = EprResponse::AccessDenied {
            required_reach: "trusted".to_string(),
            reason: "No relationship with content steward".to_string(),
        };
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::AccessDenied {
                required_reach,
                reason,
            } => {
                assert_eq!(required_reach, "trusted");
                assert_eq!(reason, "No relationship with content steward");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_epr_response_error_roundtrip() {
        let response = EprResponse::Error("something broke".to_string());
        let bytes = rmp_serde::to_vec(&response).unwrap();
        let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            EprResponse::Error(msg) => assert_eq!(msg, "something broke"),
            _ => panic!("Wrong variant"),
        }
    }
}
