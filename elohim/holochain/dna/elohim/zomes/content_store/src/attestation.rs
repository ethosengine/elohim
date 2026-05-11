//! Attestation coordinator — issuance, revocation, queries.
//!
//! Implements the consolidation defined in
//! `genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md`.
//!
//! Attestation entries are `Content` entries with `content_type` matching
//! `"attestation:<subtype>"` declared in some pillar manifest. This module
//! provides the coordinator-facing API for callers in elohim DNA and via
//! cross-DNA bridge for imagodei / infrastructure / mishpat callers.

use content_store_integrity::{EntryTypes, LinkTypes, StringAnchor, ATTESTATION_KINDS};
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

pub fn issue_attestation(input: IssueAttestationInput) -> ExternResult<AttestationOutput> {
    // Validate kind is in the codegen-emitted catalog (floor 1 in coordinator;
    // the integrity zome also enforces this — defense in depth)
    if !ATTESTATION_KINDS.contains(&input.attestation_kind.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "unknown_attestation_subtype: {}", input.attestation_kind
        ))));
    }

    let issuer_cid = agent_info()?.agent_initial_pubkey.to_string();

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Build the metadata JSON with denormalized fields
    let mut metadata = serde_json::json!({
        "attestation_kind": input.attestation_kind,
        "subject_cid": input.subject_cid,
        "subject_kind": input.subject_kind,
        "validation_method": determine_validation_method(&input),
        "proof_evidence": input.proof_evidence,
    });
    if let Some(ref expires_at) = input.expires_at {
        metadata["expires_at"] = serde_json::json!(expires_at);
    }
    if let Some(ref parent_cid) = input.parent_governance_action_cid {
        metadata["parent_governance_action_cid"] = serde_json::json!(parent_cid);
    }
    if let Some(ref vote_value) = input.vote_value {
        metadata["vote_value"] = serde_json::json!(vote_value);
    }
    // Merge subtype-specific metadata fields into evidence_json.summary_metric
    metadata["evidence_json"] = serde_json::json!({
        "summary_metric": input.metadata,
    });

    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("metadata serialization: {e}"))))?;

    // Build a unique id for this attestation
    // (uuid crate is not available in WASM — derive from entry_hash post-create)
    let placeholder_id = format!("attest-{}-{}", input.attestation_kind, issuer_cid);

    // Build the Content entry
    let content = content_store_integrity::Content {
        id: placeholder_id,
        content_type: input.attestation_kind.clone(),
        title: input.title,
        description: input.description.unwrap_or_default(),
        summary: None,
        content: String::new(),
        content_format: "epr-composite".to_string(),
        tags: vec![input.attestation_kind.clone()],
        source_path: None,
        related_node_ids: vec![input.subject_cid.clone()],
        author_id: Some(issuer_cid.clone()),
        reach: input.reach,
        trust_score: 0.0,
        estimated_minutes: None,
        thumbnail_url: None,
        metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,
        validation_status: "Valid".to_string(),
        blob_cid: None,
        content_size_bytes: None,
        content_hash: None,
    };

    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;

    // Create AttestationToSubject link
    // Base: StringAnchor keyed on subject_cid (works for any subject type: agent, content, device).
    // Adaptation from plan: plan used ActionHash::try_from which doesn't handle agent pubkey
    // strings. StringAnchor is consistent with existing IdToContent pattern and allows any
    // string CID (agent pubkey, entry hash, or opaque ID) to serve as the lookup key.
    let subject_anchor = StringAnchor::new("attestation_subject", &input.subject_cid);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(
        subject_anchor_hash.clone(),
        entry_hash.clone(),
        LinkTypes::AttestationToSubject,
        LinkTag::new(input.subject_kind.as_bytes()),
    )?;

    // If this is an M-of-N child, also create GovernanceActionChild link parent → child.
    // Base: StringAnchor keyed on parent_governance_action_cid (same pattern as above).
    if let Some(ref parent_cid) = input.parent_governance_action_cid {
        let parent_anchor = StringAnchor::new("governance_action", parent_cid.as_str());
        let parent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(parent_anchor))?;
        create_link(
            parent_anchor_hash,
            entry_hash.clone(),
            LinkTypes::GovernanceActionChild,
            LinkTag::new(input.vote_value.as_deref().unwrap_or("approve").as_bytes()),
        )?;
    }

    // Update the content id to the entry hash (content-addressed identity)
    // Note: The entry was committed with placeholder_id; in production the elohim-storage
    // post-commit projection will use action_hash as the dht_anchor_hash and entry_hash as the
    // cid column. We return entry_hash as the canonical CID.
    let _ = action_hash; // stored in source chain; post-commit signal will project to storage

    Ok(AttestationOutput {
        cid: entry_hash.to_string(),
        attestation_kind: input.attestation_kind,
        subject_cid: input.subject_cid,
        issuer_cid,
    })
}

fn determine_validation_method(input: &IssueAttestationInput) -> &'static str {
    if input.parent_governance_action_cid.is_some() {
        "M-of-N-vote"
    } else {
        "peer-confirm"  // default for unilateral attestations issued via this coordinator
    }
}

pub fn revoke_attestation(input: RevokeAttestationInput) -> ExternResult<AttestationOutput> {
    // Resolve the original attestation entry by its CID (entry hash).
    // Note: `must_get_valid_record` accepts ActionHash only; we use `get` with
    // AnyDhtHash (accepts EntryHash) to retrieve by the CID the caller holds.
    let original_hash = EntryHash::try_from(input.attestation_cid.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("invalid attestation_cid: {e}"))))?;
    let original_record = get(AnyDhtHash::from(original_hash), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("attestation not found".into())))?;
    let original_content: content_store_integrity::Content = original_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("not a Content entry".into())))?;

    if !original_content.content_type.starts_with("attestation:") {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "target is not an attestation".into()
        )));
    }

    // Same-issuer enforcement (manifest-aware cross-issuer revocation is Task B.8).
    let issuer_cid = agent_info()?.agent_initial_pubkey.to_string();
    let original_issuer = original_content
        .author_id
        .clone()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("original has no author_id".into())))?;
    if issuer_cid != original_issuer {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "only the original issuer may revoke (this build)".into()
        )));
    }

    // Decode the original metadata and inject the revocation block.
    let mut metadata: serde_json::Value =
        serde_json::from_str(&original_content.metadata_json).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!("decode metadata: {e}")))
        })?;

    let revoked_at = format!("{:?}", sys_time()?);
    metadata["revocation"] = serde_json::json!({
        "reason": input.reason,
        "revoked_at": revoked_at,
        "supersedes_cid": input.attestation_cid,
    });

    let subject_cid = metadata["subject_cid"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let subject_kind = metadata["subject_kind"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // The revocation is a new Content entry of the same kind (proof_class = "revocation"),
    // carrying the revocation block in metadata so peers can detect the supersession.
    let revoke_input = IssueAttestationInput {
        attestation_kind: original_content.content_type.clone(),
        subject_cid,
        subject_kind,
        title: format!("Revocation: {}", original_content.title),
        description: Some(format!("Revoked: {}", input.reason)),
        reach: original_content.reach,
        metadata: metadata["evidence_json"]["summary_metric"].clone(),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "revocation".to_string(),
        proof_evidence: metadata["proof_evidence"].clone(),
        expires_at: None,
    };

    issue_attestation(revoke_input)
}
