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

use content_store_integrity::{EntryTypes, LinkTypes, StringAnchor, GOVERNANCE_ACTION_KINDS};
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

/// Input for `propose_recovery_governance_action` — Recovery M4 Task 13.
///
/// Bespoke producer for the three recovery-related governance kinds. Differs
/// from the generic `ProposeGovernanceActionInput` in two load-bearing ways:
///
/// 1. The `metadata` field is a **flat object** whose fields are written at
///    the TOP LEVEL of `metadata_json` (not nested under `parameters_json`).
///    Tasks 4/5 readers expect top-level access (`metadata["human_id"]`,
///    `metadata["revoked_key"]`, `metadata["threshold_reached"]`,
///    `metadata["effective_at"]`, `metadata["is_active"]`,
///    `metadata["frozen_at_layer"]`).
/// 2. `subject_human_id` replaces `subject_cid` — recovery governance subjects
///    are humans (not arbitrary CIDs). The string is denormalised into
///    `related_node_ids` and surfaced through `subject_cid` on the output for
///    parity with the generic shape.
///
/// `supersedes_cid` is reserved for CREATE-only effectiveness transitions
/// (Task 4 constraint: never `update_entry`). When present, it is preserved
/// in metadata so projectors can follow the supersession chain.
#[derive(Serialize, Deserialize, Debug)]
pub struct ProposeRecoveryGovernanceActionInput {
    pub governance_kind: String, // "governance-action:recovery-request" | "...:key-revocation" | "...:identity-freeze"
    pub subject_human_id: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,
    pub closes_at: String,
    /// Already-flat object whose fields are merged at the top level of `metadata_json`.
    /// Example for a recovery-request: `{ "human_id": "...", "custodian_cids": [...],
    /// "trigger_type": "intimate", "session_pubkey": "...", "threshold_reached": false,
    /// "effective_at": null }`.
    pub metadata: serde_json::Value,
    /// Optional supersession pointer (CREATE-only effectiveness transitions).
    pub supersedes_cid: Option<String>,
}

/// Whitelist of governance kinds accepted by `propose_recovery_governance_action`.
/// Recovery-flow gates depend on top-level metadata fields written by this function;
/// non-recovery kinds must continue to use the generic `propose_governance_action`
/// path which nests caller-supplied data under `parameters_json`.
const RECOVERY_GOVERNANCE_KINDS: &[&str] = &[
    "governance-action:recovery-request",
    "governance-action:key-revocation",
    "governance-action:identity-freeze",
    // Recovery M4 T22 — Shamir custody setup records the custodian↔share-index
    // assignment manifest for an agent's split seed. Share bytes never travel
    // through this path (Floor G3 in attestation_validator.rs enforces that);
    // only the assignment metadata + (m,n) threshold + validity horizon.
    "governance-action:shamir-custody-setup",
];

/// Recovery M4 Task 13 — bespoke producer for recovery-shaped governance actions.
///
/// Writes a `Content` entry whose `metadata_json` carries caller-supplied fields
/// at the TOP LEVEL (mergeable with `threshold`, `closes_at`, `supersedes_cid`).
/// This is the shape that Tasks 4/5 readers consume; the generic
/// `propose_governance_action` nests caller fields under `parameters_json` and
/// would fail those gates.
///
/// CREATE-only per Task 4 constraint: effectiveness flips and supersessions
/// must produce a new Content entry rather than `update_entry` on the original.
/// Callers requesting a supersession pass `supersedes_cid`; the new entry
/// records the pointer in metadata, and downstream projectors detect the
/// chain via `query_effective_*` (which keep the most-recent-effective wins).
///
/// Post-commit signal emission is delegated to the existing
/// `post_commit` Content handler in lib.rs — it fires `ContentCommitted` for
/// every Content, and the prefix-routing dispatcher (Task 10) projects
/// `governance-action:*` kinds to the recovery_flows / key_revocations tables.
pub fn propose_recovery_governance_action(
    input: ProposeRecoveryGovernanceActionInput,
) -> ExternResult<GovernanceActionOutput> {
    if !RECOVERY_GOVERNANCE_KINDS.contains(&input.governance_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "propose_recovery_governance_action: unsupported governance_kind '{}' \
             (recovery producer accepts {:?})",
            input.governance_kind, RECOVERY_GOVERNANCE_KINDS
        ))));
    }
    // Defense in depth: validate against the codegen-emitted kinds catalog.
    if !GOVERNANCE_ACTION_KINDS.contains(&input.governance_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "unknown_governance_action_kind: {}",
            input.governance_kind
        ))));
    }

    let proposer_cid = agent_info()?.agent_initial_pubkey.to_string();

    // Compose metadata: caller-supplied fields at the top level, plus the
    // governance bookkeeping fields the readers also look at.
    let mut metadata = match input.metadata.clone() {
        serde_json::Value::Object(map) => serde_json::Value::Object(map),
        serde_json::Value::Null => serde_json::Value::Object(serde_json::Map::new()),
        other => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "propose_recovery_governance_action: `metadata` must be a JSON object, got {}",
                match other {
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::Bool(_) => "bool",
                    _ => "unknown",
                }
            ))));
        }
    };
    let metadata_obj = metadata.as_object_mut().expect("validated as object above");
    // Always-present bookkeeping fields (do not overwrite caller-supplied
    // values if they happened to be set — caller is authoritative on these).
    metadata_obj
        .entry("governance_kind".to_string())
        .or_insert(serde_json::json!(input.governance_kind));
    metadata_obj
        .entry("subject_cid".to_string())
        .or_insert(serde_json::json!(input.subject_human_id));
    metadata_obj
        .entry("threshold".to_string())
        .or_insert(input.threshold.clone());
    metadata_obj
        .entry("closes_at".to_string())
        .or_insert(serde_json::json!(input.closes_at));
    if let Some(ref supersedes) = input.supersedes_cid {
        metadata_obj
            .entry("supersedes_cid".to_string())
            .or_insert(serde_json::json!(supersedes));
    }

    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata: {e}"))))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Mirror propose_governance_action's placeholder id pattern. Real CID is
    // returned via entry_hash on the output.
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
        related_node_ids: vec![input.subject_human_id.clone()],
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

    // post_commit will emit `ContentCommitted`; the Task 10 prefix-routing
    // dispatcher consumes it and projects to recovery_flows / key_revocations.
    // No separate tally row is initialised — votes arrive as child attestations
    // via `vote_on_governance_action` and are tallied via projection.

    Ok(GovernanceActionOutput {
        cid: entry_hash.to_string(),
        governance_kind: input.governance_kind,
        subject_cid: input.subject_human_id,
        proposer_cid,
        closes_at: input.closes_at,
    })
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
    parent_cid: String,
) -> ExternResult<GovernanceActionWithChildren> {
    // Resolve the parent governance-action Content entry by its CID (entry hash).
    // Mirror B.6 adaptation: use get(AnyDhtHash) which accepts EntryHash,
    // not must_get_valid_record which only accepts ActionHash.
    let parent_entry_hash = EntryHash::try_from(parent_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid parent_cid: {e}"))))?;
    let parent_record = get(AnyDhtHash::from(parent_entry_hash), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent governance action not found".into())))?;
    let parent_content: content_store_integrity::Content = parent_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("parent is not a Content entry".into())))?;

    let proposer_cid = parent_content.author_id.clone().unwrap_or_default();
    let parent_metadata: serde_json::Value = serde_json::from_str(&parent_content.metadata_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode parent metadata: {e}"))))?;
    let subject_cid = parent_metadata["subject_cid"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let closes_at = parent_metadata["closes_at"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let parent_output = GovernanceActionOutput {
        cid: parent_cid.clone(),
        governance_kind: parent_content.content_type,
        subject_cid,
        proposer_cid,
        closes_at,
    };

    // Retrieve child attestations via GovernanceActionChild links.
    // B.3/issue_attestation creates these links with base = StringAnchor keyed by
    // "governance_action" + parent_cid (same pattern as AttestationToSubject anchor).
    let parent_anchor = StringAnchor::new("governance_action", &parent_cid);
    let parent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(parent_anchor))?;

    let child_links = get_links(
        LinkQuery::try_new(parent_anchor_hash, LinkTypes::GovernanceActionChild)?,
        GetStrategy::default(),
    )?;

    let mut children = Vec::with_capacity(child_links.len());
    for link in child_links {
        let child_entry_hash = link
            .target
            .into_entry_hash()
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("child link target is not an EntryHash".into())))?;

        let record = get(AnyDhtHash::from(child_entry_hash.clone()), GetOptions::default())?;
        let Some(record) = record else {
            continue;
        };

        let child_content: content_store_integrity::Content = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode child: {e}"))))?
            .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("child link target is not a Content entry".into())))?;

        let child_metadata: serde_json::Value =
            serde_json::from_str(&child_content.metadata_json).unwrap_or_default();
        let child_subject_cid = child_metadata["subject_cid"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        children.push(crate::attestation::AttestationOutput {
            cid: child_entry_hash.to_string(),
            attestation_kind: child_content.content_type,
            subject_cid: child_subject_cid,
            issuer_cid: child_content.author_id.unwrap_or_default(),
        });
    }

    Ok(GovernanceActionWithChildren {
        parent: parent_output,
        children,
    })
}
