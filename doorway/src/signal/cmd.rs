//! SBD protocol commands
//!
//! Defines the binary message format for the SBD signaling protocol.
//! Messages have a 32-byte header - if it starts with 28 zero bytes,
//! the last 4 bytes indicate a command. Otherwise, the header is
//! the destination public key for message forwarding.

use std::io::{Error, Result};

/// Command prefix: 28 zero bytes
pub const CMD_PREFIX: &[u8; 28] = &[0u8; 28];

/// Header size (same as Ed25519 pubkey size)
const HDR_SIZE: usize = 32;

/// Signature size for auth response
const SIG_SIZE: usize = 64;

/// Nonce size for auth request
const NONCE_SIZE: usize = 32;

/// Command identifiers (last 4 bytes of header)
const F_KEEPALIVE: &[u8; 4] = b"keep";
const F_LIMIT_BYTE_NANOS: &[u8; 4] = b"lbrt";
const F_LIMIT_IDLE_MILLIS: &[u8; 4] = b"lidl";
const F_AUTH_REQ: &[u8; 4] = b"areq";
const F_AUTH_RES: &[u8; 4] = b"ares";
const F_READY: &[u8; 4] = b"srdy";

/// SBD commands that can be received from clients
#[derive(Debug)]
pub enum SbdCmd {
    /// Forward message to another peer (payload starts with dest pubkey)
    Message(Vec<u8>),
    /// Keepalive ping
    Keepalive,
    /// Authentication response (64-byte signature)
    AuthRes([u8; SIG_SIZE]),
    /// Unknown command (ignored)
    Unknown,
}

impl SbdCmd {
    /// Parse a binary payload into a command
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < HDR_SIZE {
            return Err(Error::other("payload too short"));
        }

        // Check if this is a command (starts with CMD_PREFIX)
        if &payload[..28] == CMD_PREFIX {
            match &payload[28..32] {
                x if x == F_KEEPALIVE => Ok(SbdCmd::Keepalive),
                x if x == F_AUTH_RES => {
                    if payload.len() != HDR_SIZE + SIG_SIZE {
                        return Err(Error::other("invalid auth response length"));
                    }
                    let mut sig = [0u8; SIG_SIZE];
                    sig.copy_from_slice(&payload[HDR_SIZE..]);
                    Ok(SbdCmd::AuthRes(sig))
                }
                _ => Ok(SbdCmd::Unknown),
            }
        } else {
            // It's a message to forward
            Ok(SbdCmd::Message(payload.to_vec()))
        }
    }

    /// Construct rate limit message (byte-nanos)
    pub fn limit_byte_nanos(limit: i32) -> Vec<u8> {
        let mut out = Vec::with_capacity(HDR_SIZE + 4);
        out.extend_from_slice(CMD_PREFIX);
        out.extend_from_slice(F_LIMIT_BYTE_NANOS);
        out.extend_from_slice(&limit.to_be_bytes());
        out
    }

    /// Construct idle timeout message (milliseconds)
    pub fn limit_idle_millis(limit: i32) -> Vec<u8> {
        let mut out = Vec::with_capacity(HDR_SIZE + 4);
        out.extend_from_slice(CMD_PREFIX);
        out.extend_from_slice(F_LIMIT_IDLE_MILLIS);
        out.extend_from_slice(&limit.to_be_bytes());
        out
    }

    /// Construct auth request message (32-byte nonce)
    pub fn auth_req(nonce: &[u8; NONCE_SIZE]) -> Vec<u8> {
        let mut out = Vec::with_capacity(HDR_SIZE + NONCE_SIZE);
        out.extend_from_slice(CMD_PREFIX);
        out.extend_from_slice(F_AUTH_REQ);
        out.extend_from_slice(nonce);
        out
    }

    /// Construct ready message (authentication complete)
    pub fn ready() -> Vec<u8> {
        let mut out = Vec::with_capacity(HDR_SIZE);
        out.extend_from_slice(CMD_PREFIX);
        out.extend_from_slice(F_READY);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keepalive() {
        let mut msg = Vec::new();
        msg.extend_from_slice(CMD_PREFIX);
        msg.extend_from_slice(F_KEEPALIVE);

        match SbdCmd::parse(&msg).unwrap() {
            SbdCmd::Keepalive => {}
            _ => panic!("expected Keepalive"),
        }
    }

    #[test]
    fn test_parse_auth_res() {
        let mut msg = Vec::new();
        msg.extend_from_slice(CMD_PREFIX);
        msg.extend_from_slice(F_AUTH_RES);
        msg.extend_from_slice(&[0u8; 64]); // signature

        match SbdCmd::parse(&msg).unwrap() {
            SbdCmd::AuthRes(sig) => {
                assert_eq!(sig.len(), 64);
            }
            _ => panic!("expected AuthRes"),
        }
    }

    #[test]
    fn test_parse_message() {
        let mut msg = Vec::new();
        msg.extend_from_slice(&[1u8; 32]); // non-zero header = dest pubkey
        msg.extend_from_slice(b"hello");

        match SbdCmd::parse(&msg).unwrap() {
            SbdCmd::Message(payload) => {
                assert_eq!(payload.len(), 37);
            }
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn test_limit_byte_nanos() {
        let msg = SbdCmd::limit_byte_nanos(8000);
        assert_eq!(msg.len(), 36);
        assert_eq!(&msg[..28], CMD_PREFIX);
        assert_eq!(&msg[28..32], F_LIMIT_BYTE_NANOS);
    }

    #[test]
    fn test_limit_idle_millis() {
        let msg = SbdCmd::limit_idle_millis(10000);
        assert_eq!(msg.len(), 36);
        assert_eq!(&msg[..28], CMD_PREFIX);
        assert_eq!(&msg[28..32], F_LIMIT_IDLE_MILLIS);
    }

    #[test]
    fn test_auth_req() {
        let nonce = [42u8; 32];
        let msg = SbdCmd::auth_req(&nonce);
        assert_eq!(msg.len(), 64);
        assert_eq!(&msg[..28], CMD_PREFIX);
        assert_eq!(&msg[28..32], F_AUTH_REQ);
        assert_eq!(&msg[32..], &nonce);
    }

    #[test]
    fn test_ready() {
        let msg = SbdCmd::ready();
        assert_eq!(msg.len(), 32);
        assert_eq!(&msg[..28], CMD_PREFIX);
        assert_eq!(&msg[28..32], F_READY);
    }
}
