//! Ed25519 signing and verification, plus AgentKeypair.
//! Implementation: Task 6+.

use crate::error::Result;
use crate::signature::Signature;

/// Placeholder — will be replaced with the real AgentKeypair struct.
pub struct AgentKeypair;

/// Placeholder sign function.
pub fn sign(_keypair: &AgentKeypair, _msg: &[u8]) -> Result<Signature> {
    unimplemented!("Task 6: Ed25519 signing")
}

/// Placeholder verify function.
pub fn verify(_pubkey: &[u8], _msg: &[u8], _sig: &Signature) -> Result<()> {
    unimplemented!("Task 6: Ed25519 verification")
}
