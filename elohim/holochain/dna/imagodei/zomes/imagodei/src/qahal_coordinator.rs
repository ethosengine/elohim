//! Qahal coordinator: atomic multi-step orchestration for Collective + Collab flows.
//!
//! Per spec §2 + §5.1. Coordinator-only authority gates (link traversal happens here,
//! integrity-zome validators remain pure-data).

use hdk::prelude::*;
use imagodei_integrity::qahal::{
    Collective, Membership, MembershipRole, MemberKind, CollabAgreement,
};
use imagodei_integrity::{EntryTypes, LinkTypes};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollectiveInput {
    pub charter: String,
    pub display_name: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollabAgreementInput {
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation_json: String,
    pub commons_pool_tribute: f64,
    pub governance_terms_json: String,
    pub initial_tier: String,
    pub display_name_for_qahal: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttestCollabAgreementInput {
    pub agreement_action_hash: ActionHash,
    pub attesting_collective_cid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawMembershipInput {
    pub membership_action_hash: ActionHash,
    pub collab_qahal_cid: String,
}

// Coordinator functions land in Tasks 4–6.
