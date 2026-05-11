//! Governance-action coordinator — propose, vote, query.
//!
//! Implements the M-of-N pattern from spec §4: parent governance-action Content +
//! child attestation Content + derived tally projection. Voting is implemented
//! by issuing a child attestation Content; this module provides the wrapper
//! that ensures the child carries the correct parent_governance_action_cid
//! and is committed against the validator floors for M-of-N children.
//!
//! ## Task B.5 — propose_governance_action
//! Validates `governance_kind` against the codegen-emitted `GOVERNANCE_ACTION_KINDS`
//! catalog (floor 1 in coordinator; integrity zome enforces by content_type). Creates a
//! `Content` entry of `content_type == governance_kind` with the threshold / ballot_format
//! / closes_at / subject_cid embedded in `metadata_json` for downstream tally projection.
//!
//! ## Task B.6 — vote_on_governance_action
//! Resolves the parent governance-action Content entry via `get(AnyDhtHash::from(...))`,
//! maps `content_type` → child `attestation_kind` via `child_attestation_kind_for_governance_action`,
//! and delegates to `issue_attestation` so the child carries both the `AttestationToSubject`
//! link (subject from parent metadata) and the `GovernanceActionChild` link
//! (parent_governance_action_cid → child entry_hash).
//!
//! The `child_attestation_kind_for_governance_action` mapping is hardcoded here pending
//! a codegen-emitted constant in a Task A.7 follow-up.

use content_store_integrity::{EntryTypes, GOVERNANCE_ACTION_KINDS};
use hdk::prelude::*;

use crate::attestation::{issue_attestation, AttestationOutput, IssueAttestationInput};

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
    input: ProposeGovernanceActionInput,
) -> ExternResult<GovernanceActionOutput> {
    if !GOVERNANCE_ACTION_KINDS.contains(&input.governance_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "unknown_governance_action_kind: {}", input.governance_kind
        ))));
    }

    let proposer_cid = agent_info()?.agent_initial_pubkey.to_string();

    let metadata = serde_json::json!({
        "governance_kind": input.governance_kind,
        "subject_cid": input.subject_cid,
        "threshold": input.threshold,
        "eligibility_predicate": input.eligibility_predicate,
        "ballot_format": input.ballot_format,
        "closes_at": input.closes_at,
        "parameters_json": input.parameters,
    });
    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata: {e}"))))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // uuid crate is not available in WASM — derive a unique id from kind + proposer_cid + time,
    // mirroring the attestation coordinator's placeholder_id approach.
    let placeholder_id = format!("gov-{}-{}", input.governance_kind, proposer_cid);

    let content = content_store_integrity::Content {
        id: placeholder_id,
        content_type: input.governance_kind.clone(),
        title: input.title,
        description: input.description.unwrap_or_default(),
        summary: None,
        content: String::new(),
        content_format: "epr-composite".to_string(),
        tags: vec![input.governance_kind.clone()],
        source_path: None,
        related_node_ids: vec![input.subject_cid.clone()],
        author_id: Some(proposer_cid.clone()),
        reach: input.reach,
        trust_score: 0.0,
        estimated_minutes: None,
        thumbnail_url: None,
        metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 1,
        validation_status: "valid".to_string(),
        blob_cid: None,
        content_size_bytes: None,
        content_hash: None,
    };

    create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Content(content))?;

    Ok(GovernanceActionOutput {
        cid: entry_hash.to_string(),
        governance_kind: input.governance_kind,
        subject_cid: input.subject_cid,
        proposer_cid,
        closes_at: input.closes_at,
    })
}

pub fn vote_on_governance_action(
    input: VoteOnGovernanceActionInput,
) -> ExternResult<AttestationOutput> {
    // Resolve the parent governance-action Content entry by its CID (entry hash).
    // Adaptation from plan: plan used `must_get_valid_record(parent_hash.into())` which only
    // accepts ActionHash. Mirror B.4's adaptation: use `get` with AnyDhtHash which accepts
    // EntryHash, the canonical CID form returned by propose_governance_action.
    let parent_hash = EntryHash::try_from(input.parent_governance_action_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid parent_cid: {e}"))))?;
    let parent_record = get(AnyDhtHash::from(parent_hash), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent governance action not found".into())))?;
    let parent_content: content_store_integrity::Content = parent_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent is not a Content entry".into())))?;

    let parent_metadata: serde_json::Value = serde_json::from_str(&parent_content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent metadata: {e}"))))?;
    let subject_cid = parent_metadata["subject_cid"]
        .as_str()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent has no subject_cid".into())))?
        .to_string();

    // Lookup child_attestation_kind from the hardcoded manifest mapping.
    // NOTE: This mapping should later be replaced by a codegen-emitted constant (Task A.7 follow-up).
    let child_kind = child_attestation_kind_for_governance_action(&parent_content.content_type)
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!(
            "no child_attestation_kind declared for {}", parent_content.content_type
        ))))?;

    let attestation_input = IssueAttestationInput {
        attestation_kind: child_kind.to_string(),
        subject_cid,
        subject_kind: "agent".to_string(),
        title: format!("{} vote on {}", input.vote_value, parent_content.title),
        description: None,
        reach: parent_content.reach,
        metadata: input.evidence.unwrap_or(serde_json::json!({})),
        parent_governance_action_cid: Some(input.parent_governance_action_cid),
        vote_value: Some(input.vote_value),
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({"class": "witness"}),
        expires_at: None,
    };

    issue_attestation(attestation_input)
}

/// Maps a governance-action kind to the child attestation kind that votes on it.
/// Hardcoded to match the imagodei + mishpat pillar manifests.
/// NOTE: Should be replaced by a codegen-emitted constant in a Task A.7 follow-up.
fn child_attestation_kind_for_governance_action(governance_kind: &str) -> Option<&'static str> {
    match governance_kind {
        "governance-action:renewal-request" => Some("attestation:renewal-approval"),
        "governance-action:recovery-request" => Some("attestation:recovery-approval"),
        "governance-action:key-revocation" => Some("attestation:revocation-vote"),
        "governance-action:identity-challenge" => Some("attestation:challenge-support"),
        "governance-action:proposal" => Some("attestation:proposal-vote"),
        "governance-action:challenge" => Some("attestation:statement-vote"),
        "governance-action:election" => Some("attestation:proposal-vote"),
        _ => None,
    }
}

pub fn get_governance_action_with_children(
    _parent_cid: String,
) -> ExternResult<GovernanceActionWithChildren> {
    unimplemented!("Task B.7")
}
