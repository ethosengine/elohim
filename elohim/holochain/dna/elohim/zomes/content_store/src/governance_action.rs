//! Governance-action coordinator — propose, vote, query.
//!
//! Implements the M-of-N pattern from spec §4: parent governance-action Content +
//! child attestation Content + derived tally projection. Voting is implemented
//! by issuing a child attestation Content; this module provides the wrapper
//! that ensures the child carries the correct parent_governance_action_cid
//! and is committed against the validator floors for M-of-N children.

use hdk::prelude::*;

use crate::attestation::AttestationOutput;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProposeGovernanceActionInput {
    pub governance_kind: String,             // e.g. "governance-action:renewal-request"
    pub subject_cid: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,        // see governance-action-metadata.schema.json
    pub eligibility_predicate: Option<serde_json::Value>,
    pub ballot_format: String,
    pub closes_at: String,                   // RFC3339 UTC
    pub parameters: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceActionOutput {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteOnGovernanceActionInput {
    pub parent_governance_action_cid: String,
    pub vote_value: String,                  // approve | reject | abstain
    pub vote_weight: Option<String>,
    pub evidence: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceActionWithChildren {
    pub parent: GovernanceActionOutput,
    pub children: Vec<AttestationOutput>,
}

pub fn propose_governance_action(
    _input: ProposeGovernanceActionInput,
) -> ExternResult<GovernanceActionOutput> {
    unimplemented!("Task B.5")
}

pub fn vote_on_governance_action(
    _input: VoteOnGovernanceActionInput,
) -> ExternResult<AttestationOutput> {
    unimplemented!("Task B.6")
}

pub fn get_governance_action_with_children(
    _parent_cid: String,
) -> ExternResult<GovernanceActionWithChildren> {
    unimplemented!("Task B.7")
}
