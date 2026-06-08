//! Mishpat Commitment projection handler (Slice-2a T5).
//!
//! Receives `MishpatSignal::CommitmentCommitted` from the mishpat DNA
//! post-commit hook and projects the commitment into the `mishpat_commitments`
//! SQLite table with `dht_anchor_hash = action_hash` (notarised provenance).
//!
//! ## Why this exists
//!
//! Mishpat commitments are notarised on the DHT but were never projected to
//! storage. The `ProjectionCommitmentFetcher` (T6) reads from
//! `mishpat_commitments` and refuses a bounds-pass on rows with a NULL
//! `dht_anchor_hash`. Before this handler existed the table was always empty
//! in production, so every bounds check failed with "commitment not found".
//!
//! ## Payload shape
//!
//! The Commitment entry has three fields on the DHT wire:
//! - `action` — action discriminator string ("delegates-compute", etc.)
//! - `payload_json` — JSON string holding the action-specific policy envelope
//! - `signed_at` — caller-supplied timestamp string passed straight through the
//!   coordinator (as of T1, ISO-8601, e.g. "2026-06-10T00:00:00Z"). The bounds
//!   validator uses `valid_from`/`valid_until` from the inner `payload_json` for
//!   time-based checks; this field is metadata only.
//!
//! For `delegates-compute` the payload carries `scope`, `provider`,
//! `recipient`, `bounds` (object), `valid_from`, `valid_until`.
//! For `replicates-dwelling` the payload carries different fields — see
//! `parse_commitment_payload` for action-specific extraction logic.
//!
//! ## CID
//!
//! The `cid` column uses the Holochain `entry_hash` (base64-encoded content
//! address of the Commitment entry). This is stable across re-publications of
//! the same content and matches what the bounds_validator receives via the
//! `bounded_by` field on economic events.
//!
//! ## Wire format
//!
//! The storage-side `CommitmentPayload` mirrors the DNA `Commitment` struct
//! field-for-field. The `MishpatSignal` tag/content structure is:
//! ```json
//! { "type": "CommitmentCommitted",
//!   "payload": { "action_hash": "...", "entry_hash": "...",
//!                "commitment": { "action": "...", "payload_json": "...",
//!                                "signed_at": "..." },
//!                "author": "..." } }
//! ```

use tracing::warn;

use crate::db::models::NewMishpatCommitment;

// ============================================================================
// Projection result — upsert a new row, or revoke an existing target row
// ============================================================================

/// Outcome of parsing a `Commitment` wire payload.
///
/// Most actions project a NEW `mishpat_commitments` row (`Upsert`). The
/// `revokes-commitment` action is different: it does NOT create a row — it
/// supersedes a previously-notarized commitment by setting `revoked_at` on the
/// TARGET row (the original commitment's CID). The signal handler dispatches on
/// this enum: `Upsert` → `upsert_with_anchor`; `Revoke` → `set_revoked_at`.
///
/// `large_enum_variant` is allowed: this is a short-lived projection value
/// (constructed from a parse, matched once by the signal handler, then dropped)
/// — never stored in bulk. Boxing the `Upsert` row would ripple a `Box`/deref
/// through every consumer (`signals.rs`, the reconciler, integration tests) for
/// no real-world memory benefit.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CommitmentProjection {
    /// Project a new commitment row into `mishpat_commitments`.
    Upsert(NewMishpatCommitment),
    /// Revoke an existing commitment by CID (sets `revoked_at` on that row).
    Revoke {
        /// CID of the original commitment being superseded.
        target_cid: String,
        /// ISO-8601 revocation timestamp (`signed_at` from the revoke payload).
        signed_at: String,
    },
}

// ============================================================================
// Wire mirror of DNA Commitment entry
// ============================================================================

/// Storage-side mirror of the mishpat DNA `Commitment` entry as it arrives in
/// the `CommitmentCommitted` signal payload.
///
/// Field names must match the DNA struct exactly (serialised via MessagePack /
/// JSON over the Holochain signal wire). The `action` and `payload_json` and
/// `signed_at` fields exactly mirror `mishpat_integrity::Commitment`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentPayload {
    /// Discriminator string: "delegates-compute" | "replicates-dwelling" |
    /// "acknowledges-reach-change" | … (new actions extend the mishpat coordinator).
    pub action: String,
    /// JSON-encoded policy envelope specific to `action`. Parsed by
    /// `parse_commitment_payload` to extract scope/provider/recipient/bounds.
    pub payload_json: String,
    /// Caller-supplied timestamp string, passed straight through the coordinator
    /// (as of T1, ISO-8601, e.g. "2026-06-10T00:00:00Z"). The bounds validator
    /// uses `valid_from`/`valid_until` from the inner `payload_json` for time-based
    /// checks; this field is metadata only.
    pub signed_at: String,
}

// ============================================================================
// Pure parse fn — factored for unit-testability without a conductor
// ============================================================================

/// Parse a Mishpat `Commitment` wire payload into a `NewMishpatCommitment` row.
///
/// This is the testable core of the projection handler — no I/O, no DB,
/// no conductor. The test suite exercises this directly with fixture payloads.
///
/// ## Argument semantics
/// - `action`       — discriminator string from `Commitment.action`
/// - `payload_json` — raw JSON from `Commitment.payload_json`
/// - `entry_hash`   — Holochain entry hash (base64); used as the row `cid`
/// - `action_hash`  — Holochain action hash (base64); stored as `dht_anchor_hash`
///
/// ## Extraction strategy per action
///
/// `delegates-compute` — payload carries `scope`, `provider`, `recipient`,
/// `bounds` (object), `valid_from`, `valid_until` at the top level.
///
/// `replicates-dwelling` — payload carries `provider_dwelling_hub_id` as
/// `provider`, `recipient_dwelling_hub_id` as `recipient`, capacity/ratio
/// policy serialised into `bounds_json`, `valid_from`, `valid_until`.
/// `scope` is derived as `"replicates-dwelling"`.
///
/// `acknowledges-reach-change` — attestation only; no compute policy bounds.
/// We project it with `scope = "acknowledges-reach-change"`, empty bounds `{}`,
/// and `valid_from = signed_at` / `valid_until = signed_at` (point-in-time).
///
/// Unknown actions fall back to the same empty-bounds treatment, logging a
/// warning so operators can detect new action types that need richer extraction.
pub fn parse_commitment_payload(
    action: &str,
    payload_json: &str,
    entry_hash: &str,
    action_hash: &str,
) -> Result<CommitmentProjection, String> {
    let payload: serde_json::Value = serde_json::from_str(payload_json)
        .map_err(|e| format!("Commitment payload_json not valid JSON: {e}"))?;

    match action {
        "delegates-compute" => parse_delegates_compute(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::Upsert),
        "replicates-dwelling" => parse_replicates_dwelling(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::Upsert),
        "replicates-commons" => parse_replicates_commons(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::Upsert),
        "acknowledges-reach-change" => {
            parse_acknowledges_reach_change(&payload, entry_hash, action_hash)
                .map(CommitmentProjection::Upsert)
        }
        "revokes-commitment" => parse_revokes_commitment(&payload),
        other => {
            warn!(
                action = %other,
                entry_hash = %entry_hash,
                "mishpat_projection: unknown commitment action — projecting with empty bounds"
            );
            let provider = payload
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let recipient = payload
                .get("recipient")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let valid_from = payload
                .get("valid_from")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let valid_until = payload
                .get("valid_until")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(CommitmentProjection::Upsert(NewMishpatCommitment {
                cid: entry_hash.to_string(),
                action: other.to_string(),
                scope: other.to_string(),
                provider,
                recipient,
                bounds_json: "{}".to_string(),
                valid_from,
                valid_until,
                revoked_at: None,
                state: "proposed".to_string(),
                dht_anchor_hash: Some(action_hash.to_string()),
            }))
        }
    }
}

fn parse_delegates_compute(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    let scope = payload
        .get("scope")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "delegates-compute payload missing 'scope'".to_string())?
        .to_string();
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "delegates-compute payload missing 'provider'".to_string())?
        .to_string();
    let recipient = payload
        .get("recipient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "delegates-compute payload missing 'recipient'".to_string())?
        .to_string();
    let valid_from = payload
        .get("valid_from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "delegates-compute payload missing 'valid_from'".to_string())?
        .to_string();
    let valid_until = payload
        .get("valid_until")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "delegates-compute payload missing 'valid_until'".to_string())?
        .to_string();
    // `bounds` is the full policy object; round-trip to compact JSON for storage.
    // Fail-closed: a notarized row with absent bounds would let T6 grant an
    // empty-bounds pass — require the field rather than silently defaulting.
    let bounds_json = payload
        .get("bounds")
        .map(|b| b.to_string())
        .ok_or_else(|| "delegates-compute payload missing 'bounds'".to_string())?;

    Ok(NewMishpatCommitment {
        cid: entry_hash.to_string(),
        action: "delegates-compute".to_string(),
        scope,
        provider,
        recipient,
        bounds_json,
        valid_from,
        valid_until,
        revoked_at: None,
        state: "proposed".to_string(),
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}

fn parse_replicates_dwelling(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    // provider is the hub offering the capacity
    let provider = payload
        .get("provider_dwelling_hub_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "replicates-dwelling payload missing 'provider_dwelling_hub_id'".to_string()
        })?
        .to_string();
    let recipient = payload
        .get("recipient_dwelling_hub_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "replicates-dwelling payload missing 'recipient_dwelling_hub_id'".to_string()
        })?
        .to_string();
    let valid_from = payload
        .get("valid_from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "replicates-dwelling payload missing 'valid_from'".to_string())?
        .to_string();
    let valid_until = payload
        .get("valid_until")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "replicates-dwelling payload missing 'valid_until'".to_string())?
        .to_string();

    // Encode the full policy fields relevant to the validator as bounds_json.
    let bounds_json = serde_json::json!({
        "capacity_bytes": payload.get("capacity_bytes"),
        "scope_filter":   payload.get("scope_filter"),
        "provider_role":  payload.get("provider_role"),
        "rotation_ttl_days": payload.get("rotation_ttl_days"),
        "grace_period_days": payload.get("grace_period_days"),
        "ratio_attestation": payload.get("ratio_attestation"),
    })
    .to_string();

    Ok(NewMishpatCommitment {
        cid: entry_hash.to_string(),
        action: "replicates-dwelling".to_string(),
        scope: "replicates-dwelling".to_string(),
        provider,
        recipient,
        bounds_json,
        valid_from,
        valid_until,
        revoked_at: None,
        state: "proposed".to_string(),
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}

/// Parse a `replicates-commons` Commitment payload (Slice-2b).
///
/// Variant-dispatched on the `variant` field (mirrors the DNA coordinator's
/// `validate_replicates_commons` and the typed `ReplicatesCommonsPayload`):
///
/// - `content`  — a provide of a specific EPR. `head_ref` is the logical key
///   the provide-reconciler dedups on, so we store it as `recipient`. The
///   bounds carry `epr_scope: [head_ref]` (so the bounds_validator's scope
///   check 4b clears), `rate_per_minute`, `reach_ceiling`, and the optional
///   `closure_rule`. NO donut. `provider`/`valid_from`/`valid_until` are
///   required and fail-closed (the load-bearing
///   `replicates_commons_notarized_gate` integration seatbelt depends on the
///   validity window bracketing the event and on `epr_scope`).
/// - `capacity` — a byte-budget pledge to the commons tier. There is NO
///   counterparty (`recipient` stays empty); `commons_bytes` and the donut
///   `ratio_attestation` are folded into `bounds_json` (T13 reads
///   `commons_bytes`/`ratio_attestation` from the row — they have no dedicated
///   column). The typed `Capacity` view carries no validity window, so
///   `valid_from`/`valid_until` default to empty.
///
/// Fail-closed: a notarized row with absent required fields would let a later
/// stage grant an empty-bounds pass — require each field rather than defaulting.
fn parse_replicates_commons(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    // Reach MUST be commons for this action (defense-in-depth; the validator
    // and DNA coordinator both enforce, but the projection refuses to land a
    // mis-reached row). The capacity view carries no `reach` field, so only the
    // content variant is reach-checked here.
    let variant = payload
        .get("variant")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "replicates-commons payload missing 'variant'".to_string())?;

    match variant {
        "content" => {
            let reach = payload
                .get("reach")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "replicates-commons content payload missing 'reach'".to_string())?;
            if reach != "commons" {
                return Err(format!(
                    "replicates-commons reach must be 'commons', got '{reach}'"
                ));
            }

            let head_ref = payload
                .get("head_ref")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "replicates-commons content payload missing 'head_ref'".to_string())?
                .to_string();
            let provider = payload
                .get("provider")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "replicates-commons payload missing 'provider'".to_string())?
                .to_string();
            // The commons commitment scopes the provide to exactly `head_ref`. The
            // bounds_validator's scope check (4b) reads `epr_scope` as an array; a
            // single-EPR commons commitment IS scoped to that one EPR, so we project
            // `epr_scope: [head_ref]`. This is the substrate-correct bridge from the
            // Slice-2b schema (which carries `rate_per_minute` + `reach_ceiling`) to the
            // validator's 7-check shape. `reach_ceiling` (commons) clears check 5.
            let bounds_json = serde_json::json!({
                "epr_scope": [head_ref],
                "rate_per_minute": payload.pointer("/bounds/rate_per_minute"),
                "reach_ceiling": payload.pointer("/bounds/reach_ceiling"),
                "closure_rule": payload.get("closure_rule"),
            })
            .to_string();
            let valid_from = payload
                .get("valid_from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "replicates-commons payload missing 'valid_from'".to_string())?
                .to_string();
            let valid_until = payload
                .get("valid_until")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "replicates-commons payload missing 'valid_until'".to_string())?
                .to_string();

            Ok(NewMishpatCommitment {
                cid: entry_hash.to_string(),
                action: "replicates-commons".to_string(),
                scope: "replicates-commons".to_string(),
                provider,
                recipient: head_ref,
                bounds_json,
                valid_from,
                valid_until,
                revoked_at: None,
                state: "proposed".to_string(),
                dht_anchor_hash: Some(action_hash.to_string()),
            })
        }
        "capacity" => {
            let commons_bytes = payload
                .get("commons_bytes")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    "replicates-commons capacity variant missing/invalid 'commons_bytes'"
                        .to_string()
                })?;
            if commons_bytes == 0 {
                return Err("replicates-commons commons_bytes must be > 0".to_string());
            }
            let ratio_attestation = payload.get("ratio_attestation").ok_or_else(|| {
                "replicates-commons capacity variant missing 'ratio_attestation'".to_string()
            })?;
            // T13 reads commons_bytes + ratio_attestation straight off bounds_json
            // (no dedicated column). reach_ceiling/rate_per_minute (if present in
            // the bounds object) thread through for diagnostics.
            let bounds_json = serde_json::json!({
                "commons_bytes": commons_bytes,
                "ratio_attestation": ratio_attestation,
                "rate_per_minute": payload.pointer("/bounds/rate_per_minute"),
                "reach_ceiling": payload.pointer("/bounds/reach_ceiling"),
            })
            .to_string();
            // The capacity pledge has no counterparty (recipient stays empty)
            // and no validity window in the typed `Capacity` view shape. A
            // `provider` may be carried on the DHT payload (the pledging agent);
            // thread it through when present, default empty otherwise.
            let provider = payload
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            Ok(NewMishpatCommitment {
                cid: entry_hash.to_string(),
                action: "replicates-commons".to_string(),
                scope: "replicates-commons".to_string(),
                provider,
                // capacity pledge has no counterparty.
                recipient: String::new(),
                bounds_json,
                valid_from: String::new(),
                valid_until: String::new(),
                revoked_at: None,
                state: "proposed".to_string(),
                dht_anchor_hash: Some(action_hash.to_string()),
            })
        }
        other => Err(format!(
            "replicates-commons unknown variant '{other}' (expected 'content' | 'capacity')"
        )),
    }
}

/// Parse a `revokes-commitment` Commitment payload (Slice-2b).
///
/// A revoke does NOT create a new row — it supersedes a previously-notarized
/// commitment. We extract the `target_cid` and `signed_at` and return a
/// [`CommitmentProjection::Revoke`]; the signal handler applies it via
/// `mishpat_commitments::set_revoked_at(target_cid, signed_at)`.
///
/// Fail-closed: an empty `target_cid` or absent `signed_at` would silently
/// no-op (revoke nothing / revoke without a timestamp) — reject both.
fn parse_revokes_commitment(payload: &serde_json::Value) -> Result<CommitmentProjection, String> {
    let target_cid = payload
        .get("target_cid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "revokes-commitment payload missing 'target_cid'".to_string())?
        .to_string();
    if target_cid.is_empty() {
        return Err("revokes-commitment 'target_cid' must not be empty".to_string());
    }
    let signed_at = payload
        .get("signed_at")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "revokes-commitment payload missing 'signed_at'".to_string())?
        .to_string();

    Ok(CommitmentProjection::Revoke {
        target_cid,
        signed_at,
    })
}

fn parse_acknowledges_reach_change(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    let acknowledger = payload
        .get("acknowledger")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let target = payload
        .get("target_epr_cid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // Point-in-time: valid_from = valid_until = signed_at from payload.
    // Fail-closed: a notarized row with empty validity timestamps is invalid —
    // require the field rather than silently allowing an anchor with no time bounds.
    let signed_at = payload
        .get("signed_at")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "acknowledges-reach-change payload missing 'signed_at'".to_string())?
        .to_string();

    Ok(NewMishpatCommitment {
        cid: entry_hash.to_string(),
        action: "acknowledges-reach-change".to_string(),
        scope: "acknowledges-reach-change".to_string(),
        provider: acknowledger,
        recipient: target,
        bounds_json: "{}".to_string(),
        valid_from: signed_at.clone(),
        valid_until: signed_at,
        revoked_at: None,
        state: "proposed".to_string(),
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Unwrap an `Upsert` projection to its row, panicking on `Revoke`. Used by
    /// every well-formed-upsert test (the router now returns a
    /// `CommitmentProjection`).
    fn unwrap_upsert(p: CommitmentProjection) -> NewMishpatCommitment {
        match p {
            CommitmentProjection::Upsert(row) => row,
            other => panic!("expected Upsert, got {other:?}"),
        }
    }

    // ── delegates-compute ────────────────────────────────────────────────────

    fn delegates_compute_payload() -> String {
        serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            "provider": "agent:matthew-steward",
            "recipient": "agent:deploy-svc-matthew",
            "bounds": {
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            },
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z"
        })
        .to_string()
    }

    #[test]
    fn parse_delegates_compute_well_formed() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "delegates-compute",
                &delegates_compute_payload(),
                "uhCEk-entry-abc",
                "uhCkk-action-xyz",
            )
            .expect("well-formed delegates-compute must parse"),
        );

        assert_eq!(row.cid, "uhCEk-entry-abc");
        assert_eq!(row.action, "delegates-compute");
        assert_eq!(row.scope, "republish-epr");
        assert_eq!(row.provider, "agent:matthew-steward");
        assert_eq!(row.recipient, "agent:deploy-svc-matthew");
        assert_eq!(row.valid_from, "2026-05-28T00:00:00Z");
        assert_eq!(row.valid_until, "2026-08-26T00:00:00Z");
        assert_eq!(row.state, "proposed");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-action-xyz"),
            "dht_anchor_hash must be set to action_hash"
        );
        assert!(row.revoked_at.is_none());

        // bounds_json must round-trip to a valid JSON object with the expected keys
        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert_eq!(bounds["reach_ceiling"], "commons");
        assert_eq!(bounds["rate_per_hour"], 30);
        assert_eq!(bounds["rotation_ttl_days"], 90);
    }

    #[test]
    fn parse_delegates_compute_missing_scope_fails() {
        let payload = serde_json::json!({
            "action": "delegates-compute",
            // "scope" deliberately omitted
            "provider": "agent:x",
            "recipient": "agent:y",
            "bounds": {},
            "valid_from": "2026-01-01T00:00:00Z",
            "valid_until": "2026-12-31T00:00:00Z"
        })
        .to_string();

        let result = parse_commitment_payload("delegates-compute", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing scope must return Err");
        assert!(
            result.unwrap_err().contains("scope"),
            "error must mention 'scope'"
        );
    }

    #[test]
    fn parse_delegates_compute_missing_provider_fails() {
        let payload = serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            // "provider" deliberately omitted
            "recipient": "agent:y",
            "bounds": {},
            "valid_from": "2026-01-01T00:00:00Z",
            "valid_until": "2026-12-31T00:00:00Z"
        })
        .to_string();

        let result = parse_commitment_payload("delegates-compute", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing provider must return Err");
        assert!(
            result.unwrap_err().contains("provider"),
            "error must mention 'provider'"
        );
    }

    // ── replicates-dwelling ───────────────────────────────────────────────────

    fn replicates_dwelling_payload() -> String {
        serde_json::json!({
            "action": "replicates-dwelling",
            "provider_dwelling_hub_id": "hub:A",
            "recipient_dwelling_hub_id": "hub:B",
            "provider_role": "steward_mutual",
            "capacity_bytes": 50_000_000_000u64,
            "scope_filter": {"epr_kinds": ["Content"]},
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z",
            "grace_period_days": 14,
            "rotation_ttl_days": 90,
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-x"
            }
        })
        .to_string()
    }

    #[test]
    fn parse_replicates_dwelling_well_formed() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "replicates-dwelling",
                &replicates_dwelling_payload(),
                "uhCEk-dwelling-entry",
                "uhCkk-dwelling-action",
            )
            .expect("well-formed replicates-dwelling must parse"),
        );

        assert_eq!(row.cid, "uhCEk-dwelling-entry");
        assert_eq!(row.action, "replicates-dwelling");
        assert_eq!(row.scope, "replicates-dwelling");
        assert_eq!(row.provider, "hub:A");
        assert_eq!(row.recipient, "hub:B");
        assert_eq!(row.valid_from, "2026-05-28T00:00:00Z");
        assert_eq!(row.valid_until, "2026-08-26T00:00:00Z");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-dwelling-action")
        );

        // bounds_json must contain capacity and ratio_attestation
        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert_eq!(bounds["capacity_bytes"], 50_000_000_000u64);
        assert_eq!(bounds["provider_role"], "steward_mutual");
        assert_eq!(bounds["rotation_ttl_days"], 90);
    }

    #[test]
    fn parse_replicates_dwelling_missing_provider_fails() {
        let payload = serde_json::json!({
            "action": "replicates-dwelling",
            // "provider_dwelling_hub_id" deliberately omitted
            "recipient_dwelling_hub_id": "hub:B",
            "capacity_bytes": 50_000_000_000u64,
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z"
        })
        .to_string();

        let result = parse_commitment_payload("replicates-dwelling", &payload, "eh1", "ah1");
        assert!(
            result.is_err(),
            "missing provider_dwelling_hub_id must return Err"
        );
        assert!(
            result.unwrap_err().contains("provider_dwelling_hub_id"),
            "error must mention 'provider_dwelling_hub_id'"
        );
    }

    // ── acknowledges-reach-change ─────────────────────────────────────────────

    fn acknowledges_reach_change_payload() -> String {
        serde_json::json!({
            "action": "acknowledges-reach-change",
            "acknowledger": "agent:matthew-steward",
            "target_epr_cid": "bafy-new-epr-head-cid",
            "new_reach": "community",
            "signed_at": "2026-05-29T00:00:00Z"
        })
        .to_string()
    }

    #[test]
    fn parse_acknowledges_reach_change_well_formed() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "acknowledges-reach-change",
                &acknowledges_reach_change_payload(),
                "uhCEk-ack-entry",
                "uhCkk-ack-action",
            )
            .expect("well-formed acknowledges-reach-change must parse"),
        );

        assert_eq!(row.cid, "uhCEk-ack-entry");
        assert_eq!(row.action, "acknowledges-reach-change");
        assert_eq!(row.scope, "acknowledges-reach-change");
        assert_eq!(row.provider, "agent:matthew-steward");
        assert_eq!(row.recipient, "bafy-new-epr-head-cid");
        // point-in-time: valid_from == valid_until == signed_at
        assert_eq!(row.valid_from, "2026-05-29T00:00:00Z");
        assert_eq!(row.valid_until, "2026-05-29T00:00:00Z");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-ack-action"));
        assert_eq!(row.bounds_json, "{}");
    }

    // ── replicates-commons ────────────────────────────────────────────────────

    fn replicates_commons_content_payload() -> String {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:lamad-spa-head-cid",
            "reach": "commons",
            "bounds": { "rate_per_minute": 60, "reach_ceiling": "commons" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string()
    }

    #[test]
    fn parse_replicates_commons_content_well_formed() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "replicates-commons",
                &replicates_commons_content_payload(),
                "uhCEk-commons-entry",
                "uhCkk-commons-action",
            )
            .expect("well-formed replicates-commons content payload must parse"),
        );

        assert_eq!(row.cid, "uhCEk-commons-entry");
        assert_eq!(row.action, "replicates-commons");
        assert_eq!(row.scope, "replicates-commons");
        assert_eq!(row.provider, "agent:provider-x");
        // content variant: recipient == head_ref
        assert_eq!(row.recipient, "epr:lamad-spa-head-cid");
        assert_eq!(row.valid_from, "2026-06-01T00:00:00Z");
        assert_eq!(row.valid_until, "2026-09-01T00:00:00Z");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-commons-action"));

        // bounds_json carries epr_scope=[head_ref] (the validator's scope check
        // reads this) + reach_ceiling=commons.
        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert_eq!(bounds["epr_scope"][0], "epr:lamad-spa-head-cid");
        assert_eq!(bounds["reach_ceiling"], "commons");
        assert_eq!(bounds["rate_per_minute"], 60);
    }

    #[test]
    fn parse_replicates_commons_missing_head_ref_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            // "head_ref" deliberately omitted
            "reach": "commons",
            "bounds": { "rate_per_minute": 60, "reach_ceiling": "commons" }
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing head_ref must return Err");
        assert!(
            result.unwrap_err().contains("head_ref"),
            "error must mention 'head_ref'"
        );
    }

    #[test]
    fn parse_replicates_commons_missing_provider_fails() {
        // Fail-closed: a notarized row with an empty provider must NOT project —
        // provider is a required field (mirrors the delegates-compute parser).
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:lamad-spa-head-cid",
            "reach": "commons",
            "bounds": { "rate_per_minute": 60, "reach_ceiling": "commons" },
            // "provider" deliberately omitted
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing provider must return Err");
        assert!(
            result.unwrap_err().contains("provider"),
            "error must mention 'provider'"
        );
    }

    #[test]
    fn parse_replicates_commons_missing_valid_from_fails() {
        // Fail-closed: validity window timestamps are required.
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:lamad-spa-head-cid",
            "reach": "commons",
            "bounds": { "rate_per_minute": 60, "reach_ceiling": "commons" },
            "provider": "agent:provider-x",
            // "valid_from" deliberately omitted
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing valid_from must return Err");
        assert!(
            result.unwrap_err().contains("valid_from"),
            "error must mention 'valid_from'"
        );
    }

    // ── replicates-commons (capacity variant) ─────────────────────────────────

    fn replicates_commons_capacity_payload() -> String {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            "commons_bytes": 25_000_000_000u64,
            "reach": "commons",
            "bounds": { "rate_per_minute": 6, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-ratio"
            }
        })
        .to_string()
    }

    #[test]
    fn parse_replicates_commons_capacity_well_formed() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "replicates-commons",
                &replicates_commons_capacity_payload(),
                "uhCEk-cap-entry",
                "uhCkk-cap-action",
            )
            .expect("well-formed capacity-variant must parse"),
        );

        assert_eq!(row.action, "replicates-commons");
        assert_eq!(row.scope, "replicates-commons");
        // capacity variant has no head_ref → recipient is empty (no counterparty)
        assert_eq!(row.recipient, "");
        // no validity window in the typed Capacity shape
        assert_eq!(row.valid_from, "");
        assert_eq!(row.valid_until, "");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-cap-action"));

        // T13 reads commons_bytes + ratio_attestation straight off bounds_json.
        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert_eq!(bounds["commons_bytes"], 25_000_000_000u64);
        assert_eq!(bounds["ratio_attestation"]["commons_pct"], 20);
        assert_eq!(
            bounds["ratio_attestation"]["effective_ratio_cid"],
            "bafkrei-ratio"
        );
    }

    #[test]
    fn parse_replicates_commons_capacity_missing_commons_bytes_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            // "commons_bytes" deliberately omitted
            "reach": "commons",
            "bounds": { "rate_per_minute": 6, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-ratio"
            }
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing commons_bytes must return Err");
        assert!(
            result.unwrap_err().contains("commons_bytes"),
            "error must mention 'commons_bytes'"
        );
    }

    #[test]
    fn parse_replicates_commons_capacity_zero_bytes_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            "commons_bytes": 0u64,
            "reach": "commons",
            "bounds": { "rate_per_minute": 6, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-ratio"
            }
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "zero commons_bytes must return Err");
        assert!(
            result.unwrap_err().contains("commons_bytes"),
            "error must mention 'commons_bytes'"
        );
    }

    #[test]
    fn parse_replicates_commons_unknown_variant_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "bogus",
            "reach": "commons"
        })
        .to_string();
        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "unknown variant must return Err");
        assert!(
            result.unwrap_err().contains("variant"),
            "error must mention 'variant'"
        );
    }

    #[test]
    fn parse_replicates_commons_missing_variant_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            // "variant" deliberately omitted
            "head_ref": "epr:x",
            "reach": "commons"
        })
        .to_string();
        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing variant must return Err");
        assert!(
            result.unwrap_err().contains("variant"),
            "error must mention 'variant'"
        );
    }

    // ── revokes-commitment ────────────────────────────────────────────────────

    #[test]
    fn parse_revokes_commitment_yields_revoke_projection() {
        let payload = serde_json::json!({
            "action": "revokes-commitment",
            "target_cid": "uhCEk-original-commons",
            "reason": "pin removed",
            "signed_at": "2026-06-10T00:00:00Z"
        })
        .to_string();

        let proj = parse_commitment_payload(
            "revokes-commitment",
            &payload,
            "uhCEk-revoke-entry",
            "uhCkk-revoke-action",
        )
        .expect("well-formed revoke must parse");

        match proj {
            CommitmentProjection::Revoke {
                target_cid,
                signed_at,
            } => {
                assert_eq!(target_cid, "uhCEk-original-commons");
                assert_eq!(signed_at, "2026-06-10T00:00:00Z");
            }
            other => panic!("expected Revoke, got {other:?}"),
        }
    }

    #[test]
    fn parse_revokes_commitment_missing_target_cid_fails() {
        let payload = serde_json::json!({
            "action": "revokes-commitment",
            // "target_cid" deliberately omitted
            "signed_at": "2026-06-10T00:00:00Z"
        })
        .to_string();
        let result = parse_commitment_payload("revokes-commitment", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing target_cid must return Err");
        assert!(
            result.unwrap_err().contains("target_cid"),
            "error must mention 'target_cid'"
        );
    }

    #[test]
    fn parse_revokes_commitment_missing_signed_at_fails() {
        let payload = serde_json::json!({
            "action": "revokes-commitment",
            "target_cid": "uhCEk-original-commons"
            // "signed_at" deliberately omitted
        })
        .to_string();
        let result = parse_commitment_payload("revokes-commitment", &payload, "eh1", "ah1");
        assert!(result.is_err(), "missing signed_at must return Err");
        assert!(
            result.unwrap_err().contains("signed_at"),
            "error must mention 'signed_at'"
        );
    }

    // ── unknown action ────────────────────────────────────────────────────────

    #[test]
    fn parse_unknown_action_projects_with_empty_bounds() {
        let payload = serde_json::json!({
            "action": "future-action-v99",
            "provider": "agent:x",
            "recipient": "agent:y",
            "valid_from": "2026-01-01T00:00:00Z",
            "valid_until": "2026-12-31T00:00:00Z"
        })
        .to_string();

        let row = unwrap_upsert(
            parse_commitment_payload("future-action-v99", &payload, "eh-u1", "ah-u1")
                .expect("unknown action must not error — falls back to empty bounds"),
        );

        assert_eq!(row.action, "future-action-v99");
        assert_eq!(row.scope, "future-action-v99");
        assert_eq!(row.bounds_json, "{}");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("ah-u1"));
    }

    // ── JSON round-trip guard ─────────────────────────────────────────────────

    #[test]
    fn bounds_json_round_trips_to_valid_json() {
        // All action types should produce valid JSON bounds_json that
        // serde_json can parse. This guards against serialization regressions.
        for (action, payload) in [
            ("delegates-compute", delegates_compute_payload()),
            ("replicates-dwelling", replicates_dwelling_payload()),
            (
                "acknowledges-reach-change",
                acknowledges_reach_change_payload(),
            ),
            ("replicates-commons", replicates_commons_content_payload()),
        ] {
            let row = unwrap_upsert(
                parse_commitment_payload(action, &payload, "eh", "ah").expect("must parse"),
            );
            serde_json::from_str::<serde_json::Value>(&row.bounds_json).unwrap_or_else(|e| {
                panic!(
                    "bounds_json for action={action} is not valid JSON: {e}; got: {}",
                    row.bounds_json
                )
            });
        }
    }

    // ── signal wire shape guard ───────────────────────────────────────────────

    /// Guard: the JSON wire shape from the DNA's `CommitmentCommitted` signal
    /// must decode into our storage-side `CommitmentPayload` struct.
    ///
    /// `signed_at` is a caller-supplied timestamp string passed straight through
    /// the coordinator (as of T1, ISO-8601). It is opaque to the decoder — any
    /// string decodes — so the bounds validator uses `valid_from`/`valid_until`
    /// from the inner `payload_json` for time-based checks, not this field.
    #[test]
    fn decode_commitment_payload_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "action": "delegates-compute",
            "payload_json": "{\"action\":\"delegates-compute\",\"scope\":\"republish-epr\",\"provider\":\"agent:alice\",\"recipient\":\"agent:bob\",\"bounds\":{\"epr_scope\":[\"epr:lamad-spa\"],\"reach_ceiling\":\"commons\",\"rate_per_hour\":30,\"rotation_ttl_days\":90},\"valid_from\":\"2026-05-28T00:00:00Z\",\"valid_until\":\"2026-08-26T00:00:00Z\"}",
            "signed_at": "2026-06-10T00:00:00Z"
        });

        let payload: CommitmentPayload = serde_json::from_value(wire)
            .expect("DNA wire shape must decode into CommitmentPayload");

        assert_eq!(payload.action, "delegates-compute");
        // caller-supplied timestamp, passed through opaque
        assert_eq!(payload.signed_at, "2026-06-10T00:00:00Z");
        assert!(!payload.payload_json.is_empty());
    }
}
