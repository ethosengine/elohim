//! Attestation coordinator — issuance, revocation, queries.
//!
//! Implements the consolidation defined in
//! `genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md`.
//!
//! Attestation entries are `Content` entries with `content_type` matching
//! `"attestation:<subtype>"` declared in some pillar manifest. This module
//! provides the coordinator-facing API for callers in elohim DNA and via
//! cross-DNA bridge for imagodei / infrastructure / mishpat callers.

use hdk::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueAttestationInput {
    pub attestation_kind: String,           // e.g. "attestation:humanness"
    pub subject_cid: String,
    pub subject_kind: String,               // agent | content | device | hub | computation | governance-action
    pub title: String,
    pub description: Option<String>,
    pub reach: String,                      // private | community | public | commons
    pub metadata: serde_json::Value,        // structured per per-subtype metadata schema
    pub parent_governance_action_cid: Option<String>,
    pub vote_value: Option<String>,         // approve | reject | abstain — only for vote attestations
    pub proof_class: String,                // witness (default) | audit | proof | confirmation
    pub proof_evidence: serde_json::Value,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AttestationOutput {
    pub cid: String,                        // EntryHash of the issued attestation Content
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeAttestationInput {
    pub attestation_cid: String,
    pub reason: String,
}

// Stubs — to be implemented in subsequent tasks
pub fn issue_attestation(_input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.3")
}

pub fn revoke_attestation(_input: RevokeAttestationInput) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.4")
}
