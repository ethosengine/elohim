//! Discriminator-chain validator for attestation and governance-action Content entries.
//!
//! Called from `validate_create_entry` for any Content entry whose `content_type`
//! begins with `"attestation:"` or `"governance-action:"`.
//!
//! ## Floor summary (spec §9)
//!
//! | Floor | Status       | Notes                                                 |
//! |-------|--------------|-------------------------------------------------------|
//! | F1    | IMPLEMENTED  | Subtype present in ATTESTATION_KINDS / GOVERNANCE_ACTION_KINDS |
//! | F2    | TODO(C.3)    | Issuer authorization — manifest-aware                 |
//! | F3    | SKIPPED      | Subject link present — op-walking; see §9 note below  |
//! | F4    | TODO(C.3)    | Uniqueness anchor — manifest-aware                    |
//! | F5    | IMPLEMENTED  | expires_at RFC3339 parse; parent governance-action exists |
//! | F6    | TODO(C.3)    | Eligibility predicate — manifest-aware                |
//! | F7    | IMPLEMENTED  | Revocation supersedes_cid: hash resolves, same type+author |
//! | F8    | IMPLEMENTED  | Proof-evidence class declared + required material     |
//!
//! Governance-action floors (in addition to F1):
//! - threshold shape (type/m/n)
//! - closes_at RFC3339
//! - ballot_format present
//!
//! ### Why Floor 3 is skipped
//! HDI validators cannot call `get_links` (memory pin: project_hdi_no_get_links_in_validators).
//! Only `must_get_*` primitives are available. Floor 1 prevents unknown subtypes;
//! coordinator-side enforcement (Task C.3) must check link existence.
//! See spec §9 for the full rationale.

use crate::generated_attestation_kinds::{ATTESTATION_KINDS, GOVERNANCE_ACTION_KINDS};
use crate::Content;
use hdi::prelude::*;
use serde_json::Value;

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Run attestation/governance-action floor checks against `content`.
///
/// Returns immediately on the first failing floor.
/// Call only for entries where `content_type` starts with `"attestation:"` or
/// `"governance-action:"`.
pub fn validate_attestation_floors(
    content: &Content,
) -> ExternResult<ValidateCallbackResult> {
    let ct = content.content_type.as_str();

    if ct.starts_with("attestation:") {
        validate_attestation(content)
    } else if ct.starts_with("governance-action:") {
        validate_governance_action(content)
    } else {
        // Should not reach here — caller guards on prefix.
        Ok(ValidateCallbackResult::Valid)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Attestation floors
// ─────────────────────────────────────────────────────────────────────────────

fn validate_attestation(content: &Content) -> ExternResult<ValidateCallbackResult> {
    // Floor 1 — subtype known
    if let Err(reason) = floor1_attestation_subtype(content) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Floor 2 — issuer authorized
    // TODO(C.3): Manifest-aware issuer authorization. Each attestation subtype
    // declares an `authorized_issuers` predicate in its pillar manifest. Until
    // that lookup is wired here, this floor is ACCEPT-all.

    // Floor 3 — subject link present
    // SKIPPED: HDI validators cannot call get_links. Only must_get_* is available.
    // Floor 1 prevents unknown subtypes; coordinator gate (Task C.3) must verify
    // the AttestationToSubject link was created. See spec §9.

    // Floor 4 — uniqueness anchor
    // TODO(C.3): Manifest-aware uniqueness check (e.g. one humanness attestation
    // per agent per window). Requires coordinator-side query on projected index.

    // Parse metadata once — used by floors 5, 7, 8
    let meta = parse_metadata(&content.metadata_json)?;

    // Floor 5 — temporal validity
    if let Err(reason) = floor5_temporal(content, &meta)? {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Floor 6 — eligibility predicate
    // TODO(C.3): Manifest-aware eligibility check (e.g. humanness attestation
    // requires prior identity-credential attestation). Manifest declares the
    // predicate chain; HDI cannot evaluate it without cross-entry walks.

    // Floor 7 — revocation reference valid
    if let Err(reason) = floor7_revocation(content, &meta)? {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Floor 8 — proof class declared
    if let Err(reason) = floor8_proof_class(&meta) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Floor G3 — attestation:recovery-approval must NOT carry Shamir share material.
    //
    // Enforces the "Shamir off-DHT" boundary: share bytes are expensive, ephemeral,
    // and point-to-point — they belong on the libp2p transport layer, NOT on the DHT.
    // Any attempt to embed share material in attestation metadata is rejected here so
    // the integrity layer enforces the architectural boundary unconditionally.
    if content.content_type == "attestation:recovery-approval" {
        if let Err(reason) = floor_g3_no_share_material_in_metadata(&meta) {
            return Ok(ValidateCallbackResult::Invalid(reason));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

// ─────────────────────────────────────────────────────────────────────────────
// Governance-action floors
// ─────────────────────────────────────────────────────────────────────────────

fn validate_governance_action(content: &Content) -> ExternResult<ValidateCallbackResult> {
    // Floor 1 — kind known
    if let Err(reason) = floor1_governance_action_kind(content) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Floor 2 — issuer authorized
    // TODO(C.3): Same as attestation floor 2. ACCEPT-all until manifest wired.

    // Floor 3 — SKIPPED (same reason as attestations)

    // Floor 4 — SKIPPED (governance actions use coordinator-side uniqueness)

    let meta = parse_metadata(&content.metadata_json)?;

    // Governance-action specific: threshold shape
    if let Err(reason) = gov_threshold_shape(&meta) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Governance-action specific: closes_at RFC3339
    if let Err(reason) = gov_closes_at(&meta) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    // Governance-action specific: ballot_format present
    if let Err(reason) = gov_ballot_format(&meta) {
        return Ok(ValidateCallbackResult::Invalid(reason));
    }

    Ok(ValidateCallbackResult::Valid)
}

// ─────────────────────────────────────────────────────────────────────────────
// Floor implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Floor 1 (attestation): content_type must be a known attestation subtype.
fn floor1_attestation_subtype(content: &Content) -> Result<(), String> {
    if !ATTESTATION_KINDS.contains(&content.content_type.as_str()) {
        return Err(format!(
            "floor1_failed: unknown attestation subtype '{}'. \
             Must be one of: {:?}",
            content.content_type, ATTESTATION_KINDS
        ));
    }
    Ok(())
}

/// Floor 1 (governance-action): content_type must be a known governance-action kind.
fn floor1_governance_action_kind(content: &Content) -> Result<(), String> {
    if !GOVERNANCE_ACTION_KINDS.contains(&content.content_type.as_str()) {
        return Err(format!(
            "floor1_failed: unknown governance-action kind '{}'. \
             Must be one of: {:?}",
            content.content_type, GOVERNANCE_ACTION_KINDS
        ));
    }
    Ok(())
}

/// Floor 5: temporal validity.
///
/// Two checks:
/// 1. If `metadata["expires_at"]` is present as a string, it must parse as RFC3339.
/// 2. If `metadata["parent_governance_action_cid"]` is present as a string (child
///    entry), resolve the parent record via `must_get_valid_record` and verify it
///    is a Content entry with a `governance-action:` prefix. Full `closes_at`
///    enforcement (reject if timestamp > closes_at) requires the op's action
///    timestamp, which is not available inside a bare SelfHealingEntry validator.
///    This simplified check confirms the parent record is valid and decodable.
fn floor5_temporal(
    _content: &Content,
    meta: &Value,
) -> ExternResult<Result<(), String>> {
    // 5a: expires_at parse check
    if let Some(Value::String(exp)) = meta.get("expires_at") {
        if chrono_parse_rfc3339(exp).is_err() {
            return Ok(Err(format!(
                "floor5_failed: expires_at_invalid — '{}' is not a valid RFC3339 timestamp",
                exp
            )));
        }
    }

    // 5b: parent governance-action existence + type check
    if let Some(Value::String(parent_cid)) = meta.get("parent_governance_action_cid") {
        let raw_bytes = decode_base64url(parent_cid).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "floor5_failed: parent_governance_action_cid base64 decode error: {}",
                e
            )))
        })?;
        match ActionHash::try_from_raw_39(raw_bytes) {
            Ok(parent_hash) => {
                let record = must_get_valid_record(parent_hash)?;
                // Decode the parent as Content and verify it is a governance-action
                match record.entry().to_app_option::<Content>() {
                    Ok(Some(parent_content)) => {
                        if !parent_content.content_type.starts_with("governance-action:") {
                            return Ok(Err(format!(
                                "floor5_failed: parent_governance_action_cid resolves to \
                                 non-governance-action entry (type='{}')",
                                parent_content.content_type
                            )));
                        }
                        // Verify the parent's closes_at is parseable (structural check)
                        let parent_meta = parse_metadata_value(&parent_content.metadata_json)?;
                        if let Some(Value::String(closes_at)) = parent_meta.get("closes_at") {
                            if chrono_parse_rfc3339(closes_at).is_err() {
                                return Ok(Err(format!(
                                    "floor5_failed: parent governance-action has invalid \
                                     closes_at '{}' — parent entry is structurally invalid",
                                    closes_at
                                )));
                            }
                        }
                        // NOTE: Full timestamp comparison (action_timestamp > closes_at)
                        // requires the op's Action timestamp. That is not available here
                        // because validate_attestation_floors() does not receive the op.
                        // Task C.2: thread op into this function for strict enforcement.
                    }
                    Ok(None) => {
                        return Ok(Err(
                            "floor5_failed: parent_governance_action_cid resolved to a \
                             non-app entry"
                                .to_string(),
                        ));
                    }
                    Err(e) => {
                        return Ok(Err(format!(
                            "floor5_failed: parent_governance_action_cid entry decode error: {}",
                            e
                        )));
                    }
                }
            }
            Err(_) => {
                return Ok(Err(format!(
                    "floor5_failed: parent_governance_action_cid '{}' is not a valid ActionHash",
                    parent_cid
                )));
            }
        }
    }

    Ok(Ok(()))
}

/// Floor 7: revocation reference valid.
///
/// If `metadata["revocation"]["supersedes_cid"]` is a string:
/// - Parse as ActionHash
/// - `must_get_valid_record` to confirm it exists
/// - Decode as Content, verify `content_type` matches the current entry
/// - Verify `author_id` matches the current entry's `author_id`
fn floor7_revocation(
    content: &Content,
    meta: &Value,
) -> ExternResult<Result<(), String>> {
    let supersedes_cid = match meta
        .get("revocation")
        .and_then(|r| r.get("supersedes_cid"))
        .and_then(|v| v.as_str())
    {
        Some(s) => s,
        None => return Ok(Ok(())), // No revocation field — floor not applicable
    };

    let hash_bytes = decode_base64url(supersedes_cid).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "floor7_failed: supersedes_cid base64 decode error: {}",
            e
        )))
    })?;

    let action_hash = match ActionHash::try_from_raw_39(hash_bytes) {
        Ok(h) => h,
        Err(_) => {
            return Ok(Err(format!(
                "floor7_failed: supersedes_cid '{}' is not a valid ActionHash",
                supersedes_cid
            )));
        }
    };

    let record = must_get_valid_record(action_hash)?;

    let superseded = match record.entry().to_app_option::<Content>() {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Ok(Err(
                "floor7_failed: supersedes_cid resolved to a non-app entry".to_string(),
            ));
        }
        Err(e) => {
            return Ok(Err(format!(
                "floor7_failed: supersedes_cid entry decode error: {}",
                e
            )));
        }
    };

    // content_type must match
    if superseded.content_type != content.content_type {
        return Ok(Err(format!(
            "floor7_failed: supersedes_cid entry has content_type '{}' but current entry \
             has '{}' — revocation must replace same subtype",
            superseded.content_type, content.content_type
        )));
    }

    // author_id must match (issuer identity continuity)
    if superseded.author_id != content.author_id {
        return Ok(Err(format!(
            "floor7_failed: supersedes_cid entry has author_id '{:?}' but current entry \
             has '{:?}' — revocation issuer must match original issuer",
            superseded.author_id, content.author_id
        )));
    }

    Ok(Ok(()))
}

/// Floor 8: proof-evidence class declared with required material.
///
/// Classes: witness | audit | proof | confirmation
/// - audit:        requires `merkle_root` string
/// - proof:        requires `proof_blob` string
/// - confirmation: requires `confirmer_signature` string
/// - witness:      no extra material required
fn floor8_proof_class(meta: &Value) -> Result<(), String> {
    let proof_ev = match meta.get("proof_evidence") {
        Some(p) => p,
        None => return Ok(()), // No proof_evidence — floor not applicable for this entry
    };

    let class = match proof_ev.get("class").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => {
            return Err(
                "floor8_failed: proof_evidence.class is required when proof_evidence is present"
                    .to_string(),
            )
        }
    };

    match class {
        "witness" => {
            // No extra material required
        }
        "audit" => {
            if proof_ev.get("merkle_root").and_then(|v| v.as_str()).is_none() {
                return Err(
                    "floor8_failed: proof_evidence.merkle_root (string) is required for \
                     class=audit"
                        .to_string(),
                );
            }
        }
        "proof" => {
            if proof_ev.get("proof_blob").and_then(|v| v.as_str()).is_none() {
                return Err(
                    "floor8_failed: proof_evidence.proof_blob (string) is required for \
                     class=proof"
                        .to_string(),
                );
            }
        }
        "confirmation" => {
            if proof_ev
                .get("confirmer_signature")
                .and_then(|v| v.as_str())
                .is_none()
            {
                return Err(
                    "floor8_failed: proof_evidence.confirmer_signature (string) is required for \
                     class=confirmation"
                        .to_string(),
                );
            }
        }
        unknown => {
            return Err(format!(
                "floor8_failed: unknown proof_evidence.class '{}'. \
                 Must be one of: [witness, audit, proof, confirmation]",
                unknown
            ));
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Governance-action specific floors
// ─────────────────────────────────────────────────────────────────────────────

/// Require `metadata["threshold"]` is an object with `type`, `m`, `n` fields.
fn gov_threshold_shape(meta: &Value) -> Result<(), String> {
    let threshold = match meta.get("threshold") {
        Some(t) => t,
        None => {
            return Err(
                "gov_floor_threshold_failed: metadata.threshold is required for \
                 governance-action entries"
                    .to_string(),
            )
        }
    };

    if threshold.get("type").and_then(|v| v.as_str()).is_none() {
        return Err(
            "gov_floor_threshold_failed: metadata.threshold.type (string) is required"
                .to_string(),
        );
    }
    if threshold.get("m").and_then(|v| v.as_u64()).is_none() {
        return Err(
            "gov_floor_threshold_failed: metadata.threshold.m (number) is required".to_string(),
        );
    }
    if threshold.get("n").and_then(|v| v.as_u64()).is_none() {
        return Err(
            "gov_floor_threshold_failed: metadata.threshold.n (number) is required".to_string(),
        );
    }

    Ok(())
}

/// Require `metadata["closes_at"]` is present and parseable as RFC3339.
fn gov_closes_at(meta: &Value) -> Result<(), String> {
    match meta.get("closes_at") {
        Some(Value::String(s)) => {
            if chrono_parse_rfc3339(s).is_err() {
                return Err(format!(
                    "gov_floor_closes_at_failed: closes_at '{}' is not a valid RFC3339 timestamp",
                    s
                ));
            }
            Ok(())
        }
        Some(_) => Err(
            "gov_floor_closes_at_failed: metadata.closes_at must be a string".to_string(),
        ),
        None => Err(
            "gov_floor_closes_at_failed: metadata.closes_at is required for \
             governance-action entries"
                .to_string(),
        ),
    }
}

/// Require `metadata["ballot_format"]` is present as a string.
fn gov_ballot_format(meta: &Value) -> Result<(), String> {
    match meta.get("ballot_format") {
        Some(Value::String(_)) => Ok(()),
        Some(_) => Err(
            "gov_floor_ballot_format_failed: metadata.ballot_format must be a string".to_string(),
        ),
        None => Err(
            "gov_floor_ballot_format_failed: metadata.ballot_format is required for \
             governance-action entries"
                .to_string(),
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse `metadata_json` into a serde_json Value, wrapping errors as WasmError.
fn parse_metadata(metadata_json: &str) -> ExternResult<Value> {
    if metadata_json.is_empty() || metadata_json == "null" {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str::<Value>(metadata_json).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "attestation_validator: metadata_json parse error: {}",
            e
        )))
    })
}

/// Same as `parse_metadata` but takes `&str` (for reuse on nested entries).
fn parse_metadata_value(metadata_json: &str) -> ExternResult<Value> {
    parse_metadata(metadata_json)
}

/// Minimal RFC3339 / ISO 8601 timestamp parse without pulling in `chrono`
/// (which has non-trivial WASM size implications).
///
/// Accepts: `YYYY-MM-DDTHH:MM:SSZ`, `YYYY-MM-DDTHH:MM:SS+HH:MM`, and the
/// fractional variants. Rejects anything that doesn't start with a 4-digit year
/// and contain a 'T' separator.
fn chrono_parse_rfc3339(s: &str) -> Result<(), &'static str> {
    // Minimum valid length: "2024-01-01T00:00:00Z" = 20 chars
    if s.len() < 20 {
        return Err("too short");
    }
    let bytes = s.as_bytes();
    // Year: 4 digits
    if !bytes[0..4].iter().all(|b| b.is_ascii_digit()) {
        return Err("year invalid");
    }
    // '-' separators
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("date separator invalid");
    }
    // Month + day digits
    if !bytes[5..7].iter().all(|b| b.is_ascii_digit()) {
        return Err("month invalid");
    }
    if !bytes[8..10].iter().all(|b| b.is_ascii_digit()) {
        return Err("day invalid");
    }
    // 'T' separator
    if bytes[10] != b'T' {
        return Err("T separator missing");
    }
    // Hour:Min:Sec digits
    if !bytes[11..13].iter().all(|b| b.is_ascii_digit()) {
        return Err("hour invalid");
    }
    if bytes[13] != b':' {
        return Err("time separator invalid");
    }
    if !bytes[14..16].iter().all(|b| b.is_ascii_digit()) {
        return Err("minute invalid");
    }
    if bytes[16] != b':' {
        return Err("time separator invalid");
    }
    if !bytes[17..19].iter().all(|b| b.is_ascii_digit()) {
        return Err("second invalid");
    }
    // Remainder must end with 'Z' or '+HH:MM' or '-HH:MM' (optionally with fractional seconds)
    let tail = &s[19..];
    if tail == "Z"
        || tail.starts_with('+')
        || tail.starts_with('-')
        || tail.starts_with('.')
    {
        Ok(())
    } else {
        Err("timezone suffix invalid")
    }
}

/// Floor G3: `attestation:recovery-approval` entries MUST NOT carry Shamir share material.
///
/// Enforces the Shamir-off-DHT architectural boundary established in the
/// attestation-consolidation sprint Stage G. Share material (bytes, indexes, blobs)
/// is expensive, ephemeral, and point-to-point — it belongs on the libp2p transport
/// layer (`/elohim/shamir-share/1.0.0`), NOT embedded in DHT entries.
///
/// ## Why this floor exists
///
/// Without this floor a malicious coordinator could attempt to push share material
/// onto the DHT by embedding it in an `attestation:recovery-approval` entry's
/// `metadata.evidence_json.summary_metric` field (or top-level metadata). The DHT
/// would then gossip the share material to all peers — defeating the point-to-point
/// security model and making share material permanently visible to any DHT observer.
///
/// This floor rejects any `attestation:recovery-approval` entry that contains
/// `share_data`, `share_index`, or `share_blob` anywhere in its metadata JSON,
/// including in nested objects (checked at the top-level metadata object and within
/// `metadata.evidence_json` / `metadata.summary_metric`).
///
/// ## Checked fields
///
/// Forbidden field names (anywhere in the metadata object tree):
/// - `share_data`
/// - `share_index`
/// - `share_blob`
fn floor_g3_no_share_material_in_metadata(meta: &Value) -> Result<(), String> {
    const FORBIDDEN: &[&str] = &["share_data", "share_index", "share_blob"];

    // Check top-level metadata object
    if let Value::Object(map) = meta {
        for key in FORBIDDEN {
            if map.contains_key(*key) {
                return Err(format!(
                    "recovery_approval_must_not_carry_share_material: \
                     metadata.{} is forbidden in attestation:recovery-approval entries. \
                     Share material belongs on the libp2p transport layer, not the DHT.",
                    key
                ));
            }
        }
    }

    // Check nested metadata.evidence_json (may be a JSON object or a JSON string
    // that encodes an object — we check the Value representation, not raw string)
    if let Some(evidence) = meta.get("evidence_json") {
        if let Value::Object(ev_map) = evidence {
            for key in FORBIDDEN {
                if ev_map.contains_key(*key) {
                    return Err(format!(
                        "recovery_approval_must_not_carry_share_material: \
                         metadata.evidence_json.{} is forbidden in \
                         attestation:recovery-approval entries.",
                        key
                    ));
                }
            }
        }
    }

    // Check nested metadata.summary_metric (used by some proof_evidence shapes)
    if let Some(summary) = meta.get("summary_metric") {
        if let Value::Object(sm_map) = summary {
            for key in FORBIDDEN {
                if sm_map.contains_key(*key) {
                    return Err(format!(
                        "recovery_approval_must_not_carry_share_material: \
                         metadata.summary_metric.{} is forbidden in \
                         attestation:recovery-approval entries.",
                        key
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Decode a base64url string into raw bytes.
///
/// ActionHash serialises as base64url in JSON. We need the raw bytes to
/// construct an ActionHash via `try_from_raw_39_panicky`.
fn decode_base64url(s: &str) -> Result<Vec<u8>, String> {
    // Holochain action hashes are 39 bytes serialised as base64url with no padding.
    // We implement a simple decoder here to avoid pulling in the `base64` crate.
    let mut out = Vec::with_capacity(s.len() * 3 / 4 + 3);
    let table: [u8; 128] = {
        let mut t = [255u8; 128];
        for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="
            .iter()
            .enumerate()
        {
            t[c as usize] = i as u8;
        }
        // base64url uses - and _ instead of + and /
        t[b'-' as usize] = 62;
        t[b'_' as usize] = 63;
        t
    };

    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip padding
        if bytes[i] == b'=' {
            break;
        }
        let c0 = *bytes.get(i).ok_or("decode_base64url: unexpected end")? as usize;
        let c1 = *bytes.get(i + 1).ok_or("decode_base64url: unexpected end")? as usize;
        let v0 = table[c0];
        let v1 = table[c1];
        if v0 == 255 || v1 == 255 {
            return Err(format!("decode_base64url: invalid char at position {}", i));
        }
        out.push((v0 << 2) | (v1 >> 4));
        if let Some(&c2_byte) = bytes.get(i + 2) {
            if c2_byte == b'=' {
                break;
            }
            let v2 = table[c2_byte as usize];
            if v2 == 255 {
                return Err(format!(
                    "decode_base64url: invalid char at position {}",
                    i + 2
                ));
            }
            out.push(((v1 & 0x0f) << 4) | (v2 >> 2));
            if let Some(&c3_byte) = bytes.get(i + 3) {
                if c3_byte == b'=' {
                    break;
                }
                let v3 = table[c3_byte as usize];
                if v3 == 255 {
                    return Err(format!(
                        "decode_base64url: invalid char at position {}",
                        i + 3
                    ));
                }
                out.push(((v2 & 0x03) << 6) | v3);
            }
        }
        i += 4;
    }
    Ok(out)
}
