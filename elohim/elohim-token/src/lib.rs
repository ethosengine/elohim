//! Elohim Token — Settlement Bridge Interface
//!
//! Defines the chain-agnostic interface for bridging elohim-tokens
//! from the Holochain P2P substrate to any settlement chain.
//!
//! The protocol doesn't care which chain — only that the contract is met.
//! Every token carries proof of witnessed contribution (provenance hash),
//! not proof of computation or capital.

use serde::{Deserialize, Serialize};

/// Opaque hash bytes — chain implementations choose their own hash function.
pub type Hash = Vec<u8>;

/// Cryptographic proof that an elohim agent evaluated and approved a bridge crossing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElohimSignature {
    /// The elohim agent that signed
    pub agent_id: String,
    /// The cryptographic signature bytes
    pub signature: Vec<u8>,
    /// Which model version performed the evaluation
    pub model_version: String,
    /// Hash of the inference context (for reproducibility)
    pub inference_hash: String,
    /// When the evaluation occurred
    pub timestamp: String,
}

/// Constitutional context carried across the bridge.
/// Settlement chain validators can verify that bridged tokens
/// come from a governance layer with known parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalContext {
    pub governance_layer: String,
    pub social_contract_health: f32,
    pub dignity_floor: f32,
    pub soft_ceiling: f32,
    pub hard_ceiling: f32,
}

/// Receipt from a successful bridge-out operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeReceipt {
    /// Transaction ID on the settlement chain
    pub bridge_tx_id: String,
    /// Amount bridged
    pub amount: f64,
    /// Merkle root of the REA event chain backing these tokens
    pub provenance_root: Hash,
    /// Which settlement chain this was bridged to
    pub settlement_chain: String,
    /// When the bridge occurred
    pub timestamp: String,
}

/// Proof that tokens are backed by witnessed contribution.
/// Any settlement chain can verify this without seeing the full event history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceProof {
    /// Merkle root of all mint events backing these tokens
    pub merkle_root: Hash,
    /// Number of witnessed events in the proof
    pub event_count: u32,
    /// Total token amount backed by this proof
    pub total_amount: f64,
    /// The agent whose tokens are proven
    pub agent_id: String,
    /// The governance layer context
    pub governance_layer: String,
    /// Summaries of each event (no private data crosses the bridge)
    pub event_summaries: Vec<ProvenanceEventSummary>,
}

/// Summary of a single provenance event — carries enough to verify
/// without exposing private REA event details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEventSummary {
    pub event_id: String,
    pub action: String,
    pub amount: f32,
    pub mint_tier: String,
    pub timestamp: String,
}

/// Mint tier — micro (deterministic per-event) or discernment (elohim wisdom).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MintTier {
    /// Deterministic micro-mint from recognition pipeline
    Micro,
    /// Elohim discernment mint — periodic pattern recognition with attestation
    Discernment,
}

/// The settlement bridge interface.
///
/// Chain-specific implementations provide the actual bridging logic.
/// The protocol doesn't care which chain — only that:
/// 1. Tokens carry provenance proof (witnessed contribution, not computation)
/// 2. Constitutional context travels with the tokens
/// 3. Elohim signature attests to the bridge crossing
///
/// ResponsibilityDemandParam still applies — large bridge-outs
/// trigger obligation requirements before the bridge clears.
pub trait SettlementBridge {
    type Error: std::fmt::Debug;

    /// Bridge tokens out from Holochain to the settlement chain.
    fn bridge_out(
        &self,
        amount: f64,
        provenance_root: Hash,
        constitutional_ctx: ConstitutionalContext,
        elohim_sig: ElohimSignature,
    ) -> Result<BridgeReceipt, Self::Error>;

    /// Bridge tokens back in from settlement chain to Holochain.
    fn bridge_in(
        &self,
        receipt: BridgeReceipt,
    ) -> Result<(), Self::Error>;

    /// Verify that a provenance root is valid on the settlement chain.
    fn verify_provenance(
        &self,
        root: Hash,
    ) -> Result<bool, Self::Error>;
}
