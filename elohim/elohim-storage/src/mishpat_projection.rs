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
//! - `signed_at` — epoch-seconds string from `sys_time()` in the coordinator
//!   (e.g. "1748390400"), NOT ISO-8601
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
    /// Epoch-seconds string from `sys_time()` in the coordinator (e.g. "1748390400").
    /// NOT ISO-8601. The bounds validator uses `valid_from`/`valid_until` from
    /// the inner `payload_json` (which ARE ISO-8601); this field is metadata only.
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
) -> Result<NewMishpatCommitment, String> {
    let payload: serde_json::Value = serde_json::from_str(payload_json)
        .map_err(|e| format!("Commitment payload_json not valid JSON: {e}"))?;

    match action {
        "delegates-compute" => parse_delegates_compute(&payload, entry_hash, action_hash),
        "replicates-dwelling" => parse_replicates_dwelling(&payload, entry_hash, action_hash),
        "acknowledges-reach-change" => {
            parse_acknowledges_reach_change(&payload, entry_hash, action_hash)
        }
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
            Ok(NewMishpatCommitment {
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
            })
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
        let row = parse_commitment_payload(
            "delegates-compute",
            &delegates_compute_payload(),
            "uhCEk-entry-abc",
            "uhCkk-action-xyz",
        )
        .expect("well-formed delegates-compute must parse");

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
        let row = parse_commitment_payload(
            "replicates-dwelling",
            &replicates_dwelling_payload(),
            "uhCEk-dwelling-entry",
            "uhCkk-dwelling-action",
        )
        .expect("well-formed replicates-dwelling must parse");

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
        let row = parse_commitment_payload(
            "acknowledges-reach-change",
            &acknowledges_reach_change_payload(),
            "uhCEk-ack-entry",
            "uhCkk-ack-action",
        )
        .expect("well-formed acknowledges-reach-change must parse");

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

        let row = parse_commitment_payload("future-action-v99", &payload, "eh-u1", "ah-u1")
            .expect("unknown action must not error — falls back to empty bounds");

        assert_eq!(row.action, "future-action-v99");
        assert_eq!(row.scope, "future-action-v99");
        assert_eq!(row.bounds_json, "{}");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("ah-u1"));
    }

    // ── JSON round-trip guard ─────────────────────────────────────────────────

    #[test]
    fn bounds_json_round_trips_to_valid_json() {
        // All three action types should produce valid JSON bounds_json that
        // serde_json can parse. This guards against serialization regressions.
        for (action, payload) in [
            ("delegates-compute", delegates_compute_payload()),
            ("replicates-dwelling", replicates_dwelling_payload()),
            (
                "acknowledges-reach-change",
                acknowledges_reach_change_payload(),
            ),
        ] {
            let row = parse_commitment_payload(action, &payload, "eh", "ah").expect("must parse");
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
    /// `signed_at` is an epoch-seconds string from `sys_time()` in the coordinator
    /// (e.g. "1748390400"), NOT ISO-8601. The bounds validator uses `valid_from`/
    /// `valid_until` from the inner `payload_json` for time-based checks.
    #[test]
    fn decode_commitment_payload_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "action": "delegates-compute",
            "payload_json": "{\"action\":\"delegates-compute\",\"scope\":\"republish-epr\",\"provider\":\"agent:alice\",\"recipient\":\"agent:bob\",\"bounds\":{\"epr_scope\":[\"epr:lamad-spa\"],\"reach_ceiling\":\"commons\",\"rate_per_hour\":30,\"rotation_ttl_days\":90},\"valid_from\":\"2026-05-28T00:00:00Z\",\"valid_until\":\"2026-08-26T00:00:00Z\"}",
            "signed_at": "1748390400"
        });

        let payload: CommitmentPayload = serde_json::from_value(wire)
            .expect("DNA wire shape must decode into CommitmentPayload");

        assert_eq!(payload.action, "delegates-compute");
        // epoch-seconds string, not ISO-8601
        assert_eq!(payload.signed_at, "1748390400");
        assert!(!payload.payload_json.is_empty());
    }
}
