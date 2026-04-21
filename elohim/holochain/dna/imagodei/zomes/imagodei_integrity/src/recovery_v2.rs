//! Recovery Protocol Phase 2 — Socially Derived Identity Recovery
//!
//! Entry types and validation for seed-quorum-based identity recovery.
//! See: genesis/docs/superpowers/specs/2026-04-21-recovery-protocol-phase-2-design.md
//!
//! New entry types (prefixed with the module to distinguish from legacy
//! attestation-vote-based recovery in lib.rs):
//! - RecoverySeedCommitment: on-DHT public half + thresholds, no holder list
//! - RecoveryQuorumRequest: claimant's request, authored by hosting doorway
//! - KeyRotation: authoritative claim "new agent X is Matthew now"
//! - HeldRecoveryShare: private source-chain entry on holder devices
//! - MyRecoveryAuthorization: optional private audit log on holder devices

use hdi::prelude::*;

// =============================================================================
// Public Enums
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryMode {
    Normal,
    Stewarded { grant_hash: ActionHash },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConfidenceTier {
    None,
    Light,
    Deep,
    Constitutional,
}

// Entry type structs and validation functions will be added in subsequent tasks.
