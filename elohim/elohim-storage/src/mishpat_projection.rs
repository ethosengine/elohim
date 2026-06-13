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
use elohim_epr::Reach;

/// Parse a reach string into the DNA-notarized schema-8 [`Reach`] enum, or
/// `None` if it is outside that vocabulary.
///
/// `Reach::openness()` is the ordinal the `reach_ceiling >= reach` well-ordering
/// check compares against (the schema enum's `_ordinal`). This is used ONLY for
/// the degrade-open well-ordering check — a `None` (reach outside the schema-8
/// vocabulary the DNA owns) is NOT a rejection; the reach reads through
/// unmodified (spec §6 — never re-vocabularize; production content carries
/// `local`/`household`/`neighborhood`/… reaches the schema-8 enum does not).
fn parse_reach(reach: &str) -> Option<Reach> {
    serde_json::from_value::<Reach>(serde_json::Value::String(reach.to_string())).ok()
}

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
        // `replicates-content` is the reach-general content-provide action;
        // `replicates-commons` is the migration-window alias (the author still
        // emits it until Stage B). Both route to the same parser.
        "replicates-content" | "replicates-commons" => {
            parse_replicates_commons(&payload, entry_hash, action_hash)
                .map(CommitmentProjection::Upsert)
        }
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
/// `validate_replicates_commons` and the typed `ReplicatesContentPayload`):
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
    // Reach READ-THROUGH (spec §6 — never re-vocabularize). The content `reach`
    // is whatever `content::reach` already holds; the projection faithfully
    // projects what the authoritative layer accepted and must NOT impose a
    // stricter vocabulary gate. Production content carries reach values outside
    // the schema-8 DNA vocabulary (`local`, `household`, `neighborhood`,
    // `agent-private`, …) — gating on schema-8 membership here would silently
    // drop those exact rows (the non-fatal side-projection swallow makes the
    // drop invisible), which is the regression this change exists to prevent.
    // The reach is required (a content provide must declare one) and read
    // through to the projected row. The capacity view carries no `reach`, so
    // only the content variant is reach-handled here. The *consent* question —
    // is this node eligible to make this offer — lives one layer up in the
    // provide reconciler; this is the projection floor only.
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
            // Well-ordering (reach_ceiling >= reach), DEGRADE-OPEN: enforce ONLY
            // when BOTH reach and reach_ceiling parse to the schema-8 vocabulary
            // the DNA owns (§6 — the check is scoped to that one vocabulary and
            // makes no claim about the others). An unknown-vocab reach reads
            // through; it is never rejected for being unknown.
            if let (Some(reach_level), Some(ceiling)) = (
                parse_reach(reach),
                payload
                    .pointer("/bounds/reach_ceiling")
                    .and_then(|v| v.as_str()),
            ) {
                if let Some(ceiling_level) = parse_reach(ceiling) {
                    if ceiling_level.openness() < reach_level.openness() {
                        return Err(format!(
                            "replicates-commons reach_ceiling '{ceiling}' is more restrictive than reach '{reach}'"
                        ));
                    }
                }
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
            //
            // Rate-bound bridge: the commons schema carries `rate_per_minute`, but
            // bounds_validator check 6 reads `rate_per_hour`. Emit BOTH —
            // `rate_per_minute` for audit/display and the derived
            // `rate_per_hour = rate_per_minute * 60` so check 6 actually enforces.
            // When no rate is declared, omit `rate_per_hour` entirely (don't emit a
            // null) so check 6 is genuinely skipped rather than reading a null bound.
            let rate_per_minute = payload
                .pointer("/bounds/rate_per_minute")
                .and_then(|v| v.as_u64());
            // Carry the content's top-level `reach` onto the projected row's
            // bounds under a key DISTINCT from `reach_ceiling`. `reach_ceiling`
            // is the bound (>= reach); `reach` is the content's own reach, and
            // it is the value `provide_projection_for` reads to scope the
            // `content:<reach>` provide row. Threading `reach_ceiling` instead
            // would silently mis-scope a household commitment to its (wider)
            // ceiling and the snapshot — scoped to `content:household` — would
            // never count it.
            let mut bounds = serde_json::json!({
                "epr_scope": [head_ref],
                "reach": reach,
                "reach_ceiling": payload.pointer("/bounds/reach_ceiling"),
                "closure_rule": payload.get("closure_rule"),
            });
            if let Some(rpm) = rate_per_minute {
                let bounds_obj = bounds
                    .as_object_mut()
                    .expect("json! object literal is always an object");
                bounds_obj.insert("rate_per_minute".to_string(), serde_json::json!(rpm));
                bounds_obj.insert("rate_per_hour".to_string(), serde_json::json!(rpm * 60));
            }
            let bounds_json = bounds.to_string();
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
            // Defense-in-depth reach check (SOFT): the typed `Capacity` view may
            // omit `reach`, so we only reject when it is present-and-wrong. A
            // commons capacity pledge that declares a non-commons reach must not
            // land (mirrors the content arm's hard reach gate).
            if let Some(reach) = payload.get("reach").and_then(|v| v.as_str()) {
                if reach != "commons" {
                    return Err(format!(
                        "replicates-commons capacity reach must be 'commons', got '{reach}'"
                    ));
                }
            }

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

/// A standing provide-projection derived from a notarized commitment (Epic B).
///
/// When a `replicates-commons` **content** Commitment lands, the steward is
/// making a standing offer to provide content at a reach. The resilience
/// snapshot and `PeerSelection` read this as a `rea_commitments` `provide` /
/// `content:<reach>` row; this descriptor carries exactly what the side-
/// projection needs (the SQLite write lives in the signal handler, keeping this
/// module I/O-free and unit-testable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideProjection {
    /// Commitment provider == conductor agent key == `humans.agent_pub_key`.
    pub provider: String,
    /// Reach class for the `content:<reach>` scope, read through from the
    /// content commitment's declared `reach` (the content's own reach, NOT the
    /// `reach_ceiling` bound). Non-commons content projects a non-commons
    /// provide row so the resilience snapshot can count it at the content's reach.
    pub reach: String,
    /// The commitment's `action_hash` for provenance on the projected row.
    pub dht_anchor_hash: Option<String>,
}

/// Decide whether a just-projected commitment should ALSO record a standing
/// `rea_commitments` provide row, and with what `(provider, reach)`.
///
/// Returns `Some` only for `replicates-content` / `replicates-commons`
/// (migration-window alias) **content** commitments: the content variant
/// carries a `head_ref` recipient (a real per-content offer); the **capacity**
/// variant is a byte pledge with no counterparty and must NOT mint a
/// content-reach provide row. Any other action returns `None` (custody-blob,
/// delegates-compute, etc. are not content provision). The reach is read
/// through from the row's bounds (the content's own declared reach), so
/// non-commons content projects a non-commons provide row.
///
/// Pure — no DB, no conductor. The caller in `signals.rs` performs the write
/// via [`crate::db::rea_commitments::record_provide_from_content_commitment`].
pub fn provide_projection_for(row: &NewMishpatCommitment) -> Option<ProvideProjection> {
    // Accept both the renamed action and the migration-window alias. The author
    // still emits `replicates-commons` until Stage B; `replicates-content` is
    // the post-rename action string both validators honor for one window.
    if row.action != "replicates-commons" && row.action != "replicates-content" {
        return None;
    }
    // The content variant sets recipient = head_ref; the capacity variant sets
    // recipient = "" (pure pledge, no per-content offer). Only the content
    // variant mints a provide row.
    if row.recipient.is_empty() {
        return None;
    }
    // Read the content's reach through from the bounds the content arm wrote
    // (the `reach` key, distinct from `reach_ceiling`). A row missing it (e.g.
    // a capacity-shaped bounds object that slipped through with a recipient) is
    // not a content provide — skip rather than fabricate a reach.
    let bounds: serde_json::Value = serde_json::from_str(&row.bounds_json).ok()?;
    let reach = bounds.get("reach").and_then(|v| v.as_str())?.to_string();
    Some(ProvideProjection {
        provider: row.provider.clone(),
        reach,
        dht_anchor_hash: row.dht_anchor_hash.clone(),
    })
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
    if signed_at.is_empty() {
        return Err("revokes-commitment 'signed_at' must not be empty".to_string());
    }

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
        // The content's own reach is carried distinctly from reach_ceiling so
        // provide_projection_for reads the right scope. Here both are commons.
        assert_eq!(bounds["reach"], "commons");
        assert_eq!(bounds["reach_ceiling"], "commons");
        assert_eq!(bounds["rate_per_minute"], 60);
        // Rate-bound bridge: bounds_validator check 6 reads `rate_per_hour`, so the
        // projection must derive it as rate_per_minute * 60 (60 * 60 == 3600).
        assert_eq!(
            bounds["rate_per_hour"], 3600,
            "rate_per_hour must be rate_per_minute * 60 so check 6 enforces"
        );
    }

    #[test]
    fn parse_replicates_commons_content_no_rate_omits_rate_per_hour() {
        // When no rate is declared, the projection must NOT emit a null
        // `rate_per_hour` — it omits the key entirely so bounds_validator check 6
        // is genuinely skipped (rather than reading a null bound).
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:lamad-spa-head-cid",
            "reach": "commons",
            "bounds": { "reach_ceiling": "commons" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();

        let row = unwrap_upsert(
            parse_commitment_payload("replicates-commons", &payload, "eh-norate", "ah-norate")
                .expect("content payload without a rate must still parse"),
        );

        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert!(
            bounds.get("rate_per_hour").is_none(),
            "rate_per_hour must be OMITTED (not null) when no rate is declared, got: {}",
            row.bounds_json
        );
        assert!(
            bounds.get("rate_per_minute").is_none(),
            "rate_per_minute must be omitted when no rate is declared"
        );
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
    fn parse_replicates_commons_capacity_wrong_reach_fails() {
        // Defense-in-depth: a capacity pledge that declares a non-commons reach
        // must be rejected (soft check — only fires when `reach` is present).
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            "commons_bytes": 25_000_000_000u64,
            "reach": "household",
            "bounds": { "rate_per_minute": 6, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-ratio"
            }
        })
        .to_string();

        let result = parse_commitment_payload("replicates-commons", &payload, "eh1", "ah1");
        assert!(
            result.is_err(),
            "capacity with non-commons reach must return Err"
        );
        assert!(
            result.unwrap_err().contains("reach"),
            "error must mention 'reach'"
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

    // ── provide_projection_for (Epic B side-projection descriptor) ────────────

    #[test]
    fn provide_projection_for_content_commitment_yields_provider_and_commons() {
        let row = unwrap_upsert(
            parse_commitment_payload(
                "replicates-commons",
                &replicates_commons_content_payload(),
                "eh-pp",
                "ah-pp",
            )
            .expect("content payload parses"),
        );
        let p = provide_projection_for(&row).expect("content commitment yields a provide");
        assert_eq!(p.reach, "commons");
        assert_eq!(p.dht_anchor_hash.as_deref(), Some("ah-pp"));
        // provider is whatever the content payload carried.
        assert!(!p.provider.is_empty(), "provider threads through");
    }

    /// A non-commons content commitment must project the content's own reach —
    /// read through from the payload, not hard-coded commons, and NOT the
    /// `reach_ceiling`. This is the net-new read-through (and the discriminating
    /// reach != reach_ceiling case) the whole non-commons-counting change exists
    /// for. `intimate` (ordinal 3) content under a `community` (ordinal 6)
    /// ceiling: well-ordering holds; the projected provide reach MUST be
    /// `intimate` (the content's reach), NOT `community` (the ceiling). Threading
    /// the ceiling instead would silently mis-scope the row and the snapshot —
    /// scoped `content:intimate` — would never count it.
    #[test]
    fn provide_projection_for_non_commons_reach_reads_through() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:intimate-record",
            "reach": "intimate",
            "bounds": { "reach_ceiling": "community" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();

        let row = unwrap_upsert(
            parse_commitment_payload("replicates-commons", &payload, "eh-int", "ah-int")
                .expect("intimate-reach content payload parses (well-ordered ceiling)"),
        );
        // The bounds carry both reach (intimate) and reach_ceiling (community);
        // the provide reach is read from `reach`, NOT `reach_ceiling`.
        let bounds: serde_json::Value = serde_json::from_str(&row.bounds_json).unwrap();
        assert_eq!(bounds["reach"], "intimate");
        assert_eq!(bounds["reach_ceiling"], "community");

        let p = provide_projection_for(&row).expect("intimate content yields a provide");
        assert_eq!(
            p.reach, "intimate",
            "provide reach must be the content reach (intimate), not the ceiling (community)"
        );
    }

    /// The `replicates-content` action string (the post-rename action) parses
    /// through the same arm as `replicates-commons` (the migration-window alias).
    #[test]
    fn parse_replicates_content_action_alias_parses() {
        let payload = serde_json::json!({
            "action": "replicates-content",
            "variant": "content",
            "head_ref": "epr:community-record",
            "reach": "community",
            "bounds": { "reach_ceiling": "commons" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();
        let row = unwrap_upsert(
            parse_commitment_payload("replicates-content", &payload, "eh-rc", "ah-rc")
                .expect("replicates-content action parses through the same arm"),
        );
        let p = provide_projection_for(&row).expect("replicates-content yields a provide");
        assert_eq!(p.reach, "community");
    }

    /// `reach_ceiling` more restrictive than `reach` violates the well-ordering
    /// and must be rejected at parse time (structural floor).
    #[test]
    fn parse_replicates_commons_ceiling_below_reach_fails() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:community-record",
            "reach": "community",
            "bounds": { "reach_ceiling": "intimate" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();
        let result = parse_commitment_payload("replicates-commons", &payload, "eh-bad", "ah-bad");
        assert!(
            result.is_err(),
            "reach_ceiling intimate < reach community must reject"
        );
        assert!(
            result.unwrap_err().contains("more restrictive"),
            "error must explain the well-ordering violation"
        );
    }

    /// A reach value OUTSIDE the schema-8 DNA vocabulary (`neighborhood`,
    /// `local`, `household`, …) READS THROUGH — it is NOT rejected. Production
    /// content carries such reaches in bulk; gating on schema-8 membership would
    /// silently drop those rows (spec §6: read-through, never re-vocabularize).
    /// The unknown reach is projected to `content:<reach>` for the provide row.
    #[test]
    fn parse_replicates_commons_non_schema8_reach_reads_through() {
        let payload = serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "epr:neighborhood-record",
            "reach": "neighborhood",
            // reach_ceiling also outside schema-8 — well-ordering degrades open.
            "bounds": { "reach_ceiling": "neighborhood" },
            "provider": "agent:provider-x",
            "valid_from": "2026-06-01T00:00:00Z",
            "valid_until": "2026-09-01T00:00:00Z"
        })
        .to_string();
        let row = unwrap_upsert(
            parse_commitment_payload("replicates-commons", &payload, "eh-nb", "ah-nb")
                .expect("an unknown-vocab reach must read through, not reject"),
        );
        let bounds: serde_json::Value = serde_json::from_str(&row.bounds_json).unwrap();
        assert_eq!(
            bounds["reach"], "neighborhood",
            "the non-schema-8 reach is read through to the projected row"
        );
        let p = provide_projection_for(&row).expect("yields a provide");
        assert_eq!(p.reach, "neighborhood");
    }

    #[test]
    fn provide_projection_for_capacity_commitment_yields_none() {
        // A capacity pledge has no head_ref recipient — it is a byte pledge, not
        // a per-content offer, so it must NOT mint a content-reach provide row.
        let row = unwrap_upsert(
            parse_commitment_payload(
                "replicates-commons",
                &replicates_commons_capacity_payload(),
                "eh-cap",
                "ah-cap",
            )
            .expect("capacity payload parses"),
        );
        assert!(
            provide_projection_for(&row).is_none(),
            "capacity pledge yields no provide projection"
        );
    }

    #[test]
    fn provide_projection_for_non_commons_action_yields_none() {
        // A delegates-compute commitment is not commons content provision.
        let row = unwrap_upsert(
            parse_commitment_payload(
                "delegates-compute",
                &delegates_compute_payload(),
                "eh-dc",
                "ah-dc",
            )
            .expect("delegates-compute parses"),
        );
        assert!(
            provide_projection_for(&row).is_none(),
            "non-replicates-commons action yields no provide projection"
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
