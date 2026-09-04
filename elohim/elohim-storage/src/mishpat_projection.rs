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

use crate::db::models::{NewIdentityHead, NewLens, NewMishpatCommitment};
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
    /// Project a new lens row into `lenses` (the `author-lens` action — a Lens is
    /// a Commitment, but its projection target is the A-class `lenses` table, not
    /// `mishpat_commitments`). Lens-market S3.
    UpsertLens(NewLens),
    /// Project a new identity-head row into `identity_heads` (the `binds-identity`
    /// action — the identity-head declaration is a Commitment, but its projection
    /// target is the A-class `identity_heads` table, not `mishpat_commitments`).
    /// Wave C1 of the identity-head-key-lineage arc.
    UpsertIdentityHead(NewIdentityHead),
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
        // **Holochain Evolution Epic, Task 11 (Station 2, measured on the
        // household mesh 2026-09-04).** Without this arm a `migrates-lineage`
        // commitment fell to the `other =>` fallback and projected
        // `state: "proposed"` — and `release_adoption::verify::verify_path`
        // establishes a path ONLY on `"active"`, so every correctly notarized
        // migration path was refused `path_not_notarized` with the detail
        // "path <cid> is proposed, not active". Nothing in this codebase ever
        // wrote `"active"` to this column (`set_state` had exactly one caller:
        // a unit test), so the positive branch of the whole rung was
        // unreachable.
        //
        // A LINEAGE commitment's notarization IS its activation, and that is a
        // property of this action class rather than a shortcut: it was created
        // through the arm the coordinator validates as a lineage commitment,
        // and there is no separate acceptance step to wait for. Its only other
        // lifecycle state is REVOKED, which arrives as a `revokes-commitment`
        // setting `revoked_at` — the field `verify_path` checks BEFORE it looks
        // at `state`. `delegates-compute` and friends keep `"proposed"` because
        // they have a real acceptance step this class does not have.
        //
        // On the quorum, precisely: `validate_lineage_signatures` runs
        // AUTHOR-SIDE ONLY, in the authoring conductor's wasm at create time.
        // Receiving peers do NOT re-verify — mishpat integrity's
        // `commitment_action_requirements` has no lineage arm, so nothing
        // re-runs those signature checks on gossip. "Active" here states the
        // LIFECYCLE, never the quorum's soundness on this peer; the quorum's
        // own enforcement gap is Station 10's, filed with the roster gap it
        // shares.
        //
        // `action` is the ENTRY's discriminator (`signals.rs` passes
        // `commitment.action`), never `payload_json`'s second copy of it —
        // `validate_ratifies_limit_gradient` never pins the two together, so a
        // body claiming `"action": "migrates-lineage"` under a different entry
        // action reaches the coordinator unsigned. Keying off the body would
        // project that forgery ACTIVE. See `release_adoption::path_evidence`'s
        // `notarized_lineage_state` for the same rule on the read side.
        "migrates-lineage" | "sunsets-lineage" => {
            parse_lineage_commitment(action, &payload, entry_hash, action_hash)
                .map(CommitmentProjection::Upsert)
        }
        "revokes-commitment" => parse_revokes_commitment(&payload),
        "author-lens" => parse_author_lens(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::UpsertLens),
        "binds-identity" => parse_binds_identity(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::UpsertIdentityHead),
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

/// Parse an `author-lens` payload into a `NewLens` row (lens-market S3).
///
/// A Lens is a Commitment, but it projects into the A-class `lenses` table, not
/// `mishpat_commitments`. `cid = entry_hash` (the read/scope key), `dht_anchor_hash
/// = action_hash` (notarised provenance). `governs_epr` is the EPR slug-id scope
/// key (plan A3). `rule`/`telos` round-trip to compact JSON.
///
/// Fail-closed: required fields error (the signal handler warn-skips that row only,
/// never the whole signal). The S1 coordinator validator already rejects malformed
/// lenses at create-time; this is the defense-in-depth projection-side guard.
/// Parse a `migrates-lineage` / `sunsets-lineage` payload into a
/// `mishpat_commitments` row, `state: "active"`.
///
/// See the dispatch arm above for WHY the state is active on notarization. The
/// row is what `release_adoption::path_evidence::lifecycle` reads; the
/// commitment BODY that `verify_path` checks the crossing against is read from
/// the DHT entry itself, never from here — so this projection deliberately does
/// not try to be a second home for `from_dna_hash`/`to_dna_hash`. It keeps the
/// whole notarized payload in `bounds_json` so an operator reading the
/// commitment ledger sees the path they are looking at.
///
/// Fail-open on the window fields (empty string when absent) rather than
/// refusing the row: `validate_migrates_lineage` in the mishpat coordinator
/// already required them before the entry could be created, and a row this
/// projection dropped would read as "no path notarized" — turning a parse
/// nicety into a statement about someone else's governance.
fn parse_lineage_commitment(
    action: &str,
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    let role = payload
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(action)
        .to_string();
    let window = payload.get("window");
    let window_field = |name: &str| -> String {
        window
            .and_then(|w| w.get(name))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    // `migrates-lineage` carries opens_at/revert_until; `sunsets-lineage`
    // carries sunsets_at and no far edge — a sunset has no revert horizon by
    // construction, which is the whole reason it is a separate commitment.
    let valid_from = if window_field("opens_at").is_empty() {
        window_field("sunsets_at")
    } else {
        window_field("opens_at")
    };
    let valid_until = window_field("revert_until");

    Ok(NewMishpatCommitment {
        cid: entry_hash.to_string(),
        action: action.to_string(),
        scope: role,
        provider: String::new(),
        recipient: String::new(),
        bounds_json: payload.to_string(),
        valid_from,
        valid_until,
        revoked_at: None,
        state: "active".to_string(),
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}

fn parse_author_lens(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewLens, String> {
    let governs_epr = payload
        .get("governs_epr")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "author-lens payload missing non-empty 'governs_epr'".to_string())?
        .to_string();
    let school = payload
        .get("school")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "author-lens payload missing non-empty 'school'".to_string())?
        .to_string();
    let role = payload
        .get("role")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "author-lens payload missing non-empty 'role'".to_string())?;
    // Re-establish the closed role enum at the projection layer (defense-in-depth).
    // The S1 coordinator validator enforces this, but it is a HOT-SWAPPED coordinator
    // gate — a stale-coordinator peer or a replay predating the validator can deliver
    // an unvalidated payload. A bogus constitutional role (e.g. 'dictator') must never
    // reach `lenses.role`, which feeds the floor/ceiling governance seam. Enum mirrors
    // sdk/schemas/v1/commitments/author-lens.schema.json.
    if !matches!(role, "lens" | "floor" | "ceiling") {
        return Err(format!(
            "author-lens payload has out-of-enum 'role' (must be lens|floor|ceiling): {role}"
        ));
    }
    let role = role.to_string();
    // rule (the deterministic predicate) and telos (steering target) are required
    // objects; round-trip to compact JSON for storage.
    let rule_json = payload
        .get("rule")
        .filter(|v| v.is_object())
        .map(|v| v.to_string())
        .ok_or_else(|| "author-lens payload missing object 'rule'".to_string())?;
    let telos_json = payload
        .get("telos")
        .filter(|v| v.is_object())
        .map(|v| v.to_string())
        .ok_or_else(|| "author-lens payload missing object 'telos'".to_string())?;
    // version_parent is optional (None for a first-version lens).
    let version_parent = payload
        .get("version_parent")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Ok(NewLens {
        cid: entry_hash.to_string(),
        governs_epr,
        school,
        role,
        rule_json,
        telos_json,
        version_parent,
        revoked_at: None,
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}

/// Parse a `binds-identity` payload into a `NewIdentityHead` row (Wave C1).
///
/// A `binds-identity` commitment IS a Commitment, but it projects into the A-class
/// `identity_heads` table, not `mishpat_commitments`. `cid = entry_hash` (the read
/// key), `dht_anchor_hash = action_hash` (notarised provenance). The declaration
/// carries `chain_root` (the stable identity-chain id), `head_key` (the current head
/// agent_cid — the did:elohim resolver's join key), `controllers` (a non-empty array
/// of controller id strings), and `controller_policy` (the `{kind, m?, n?}` object).
/// `controllers`/`controller_policy` round-trip to compact JSON.
///
/// Fail-closed, mirroring `parse_author_lens`: required fields error (the signal
/// handler warn-skips that row only, never the whole signal). The B2 coordinator
/// validator (`validate_binds_identity`) already rejects malformed declarations at
/// create-time; this is the defense-in-depth projection-side guard for a stale-
/// coordinator peer or a replay predating the validator.
///
/// Ontology guard (imago-dei, structural): `controllers` MUST be non-empty — a head
/// cannot exist without its controller-set (the recovery quorum is a controller,
/// never an override). An empty set is refused here, never silently projected.
fn parse_binds_identity(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewIdentityHead, String> {
    let chain_root = payload
        .get("chain_root")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "binds-identity payload missing non-empty 'chain_root'".to_string())?
        .to_string();
    let head_key = payload
        .get("head_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "binds-identity payload missing non-empty 'head_key'".to_string())?
        .to_string();
    // controllers: a non-empty array of non-empty strings (the ontology guard —
    // re-established at the projection layer, defense-in-depth against a stale
    // coordinator / pre-validator replay).
    let controllers = payload
        .get("controllers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "binds-identity payload missing array 'controllers'".to_string())?;
    if controllers.is_empty() {
        return Err(
            "binds-identity controllers must be non-empty (a head cannot exist without \
             its controller-set)"
                .to_string(),
        );
    }
    if controllers
        .iter()
        .any(|c| c.as_str().map(str::is_empty).unwrap_or(true))
    {
        return Err("binds-identity controllers entries must be non-empty strings".to_string());
    }
    let controllers_json = serde_json::Value::Array(controllers.clone()).to_string();
    // controller_policy: required object; round-trip to compact JSON.
    let controller_policy_json = payload
        .get("controller_policy")
        .filter(|v| v.is_object())
        .map(|v| v.to_string())
        .ok_or_else(|| "binds-identity payload missing object 'controller_policy'".to_string())?;
    // signed_at is the DHT-canonical head-order key. The AUTHORITATIVE value is the
    // Commitment envelope's `signed_at`, which the signal handler stamps after this
    // parse (envelope-level metadata, not payload data — `parse_commitment_payload`
    // does not receive it). We accept a payload-carried `signed_at` here only as a
    // best-effort fallback so the pure parser yields a complete, orderable row.
    let signed_at = payload
        .get("signed_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // successor_head_key: OPTIONAL, and absent from every declaration today — the
    // coordinator validator (`validate_binds_identity`) does not require the field
    // and `revokes-commitment` carries only {target_cid, reason, signed_at}. Read
    // it optionally anyway so a rotation that starts naming its continuation lands
    // in the column with no further change (correct-but-dormant reader; the
    // producer gap is the named follow-on). An empty string is NOT a successor —
    // absent stays honestly `None`, which the resolver reads as a TERMINAL
    // revocation, never as "unknown successor".
    let successor_head_key = payload
        .get("successor_head_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(NewIdentityHead {
        cid: entry_hash.to_string(),
        chain_root,
        head_key,
        controllers_json,
        controller_policy_json,
        signed_at,
        revoked_at: None,
        successor_head_key,
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

// ============================================================================
// mishpat → REA replication-commitment mirror (the wrong-table-split cure)
// ============================================================================

/// A **per-commitment** `rea_commitments` mirror of a notarized `replicates-*`
/// commitment.
///
/// ## Why this exists (the wrong-table split)
///
/// The live producer (`services::conductor_commitment_author`) authors
/// commitments through the **mishpat** zome; its post-commit
/// `MishpatSignal::CommitmentCommitted` projects into `mishpat_commitments`.
/// But EVERY resilience/distribution reader queries `rea_commitments`, which
/// only the lamad/content_store `ReaCommitmentCommitted` path ever populated.
/// Result: the resilience card's commitment fields read zero forever.
///
/// [`ProvideProjection`] already bridges one narrow slice of this — but it
/// collapses to ONE `(provider, reach)`-keyed `provide` row, deliberately
/// losing per-commitment identity, tier, counterparty and pledged bytes. This
/// descriptor is the faithful, per-commitment mirror the replication lens needs
/// (`elohim_facings::folds::replication_commitment`).
///
/// ## Storage-only — the DHT path is untouched
///
/// No new entry type, no second write, no re-notarization. The commitment is
/// already notarized and already lands in `mishpat_commitments`; this is a
/// second **projection** of that same notarized event into the table the readers
/// actually read.
///
/// ## `id` is the entry_hash, never the action_hash
///
/// `cid` carries the commitment `entry_hash` (memory
/// `project_mishpat_commitment_cid_is_entry_hash`): the `entry_hash` is what
/// `economic_events.bounded_by` and every bounds gate key on; the `action_hash`
/// lives ONLY in `dht_anchor_hash`. Returning the action hash as the CID
/// silently breaks every bounds gate.
///
/// Pure — no DB, no conductor. The caller in `signals.rs` performs the write via
/// [`crate::db::rea_commitments::mirror_replication_commitment`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationMirror {
    /// The commitment `entry_hash` → `rea_commitments.id`.
    pub cid: String,
    /// The action discriminator, carried through verbatim
    /// (`replicates-dwelling` | `replicates-commons` | `replicates-content`).
    pub action: String,
    /// `rea_commitments.provider`. An `agent_cid` for a content provide; a *hub
    /// id* for a dwelling commitment (`provider_dwelling_hub_id`) — the write
    /// site OBSERVES that namespace divergence rather than rejecting it
    /// (`identity_namespace`, enforcement-by-observation).
    pub provider: String,
    /// `rea_commitments.receiver`. The EPR `head_ref` for a commons **content**
    /// commitment (the per-content link), the recipient hub id for a dwelling
    /// commitment, empty for a capacity pledge (no counterparty).
    pub receiver: String,
    /// The action-specific policy envelope, snake_case as the DHT wrote it (the
    /// row's `bounds_json`, plus the identity/validity fields the envelope needs
    /// to stand alone). The replication fold parses `provider_role` /
    /// `capacity_bytes` / `commons_bytes` out of this.
    pub metadata_json: String,
    /// `["content:<reach>"]` for a commons **content** commitment (the same
    /// uniform JSON-list contract the `provide` projection writes, non-commons
    /// spec §11.2 Option A), `None` where the commitment declares no content
    /// classification.
    pub resource_classified_as: Option<String>,
    /// The commitment's `action_hash` — notarized provenance, never the id.
    pub dht_anchor_hash: Option<String>,
}

/// Replication actions that mirror into `rea_commitments`.
///
/// `replicates-content` is the reach-general content action; `replicates-commons`
/// is the migration-window alias the conductor author still emits;
/// `replicates-dwelling` is the household/collective capacity commitment.
/// `delegates-compute`, `custody-blob`, `project-epr`, `author-lens`,
/// `binds-identity`, `acknowledges-reach-change` are NOT replication and are
/// never mirrored.
const MIRRORED_REPLICATION_ACTIONS: [&str; 3] = [
    "replicates-dwelling",
    "replicates-commons",
    "replicates-content",
];

/// Decide whether a just-parsed commitment row should ALSO be mirrored into
/// `rea_commitments` as a per-commitment replication row, and with what shape.
///
/// Returns `None` for any non-replication action. A malformed/absent
/// `bounds_json` does NOT drop the mirror — the commitment is real and its
/// identity fields are known; the envelope degrades to `{}` and the fold's
/// classification simply skips it (measured absence beats a lost row).
///
/// Pure — no DB, no conductor.
pub fn replication_mirror_for(row: &NewMishpatCommitment) -> Option<ReplicationMirror> {
    if !MIRRORED_REPLICATION_ACTIONS.contains(&row.action.as_str()) {
        return None;
    }

    // Start from the notarized policy envelope; degrade a non-object to `{}`.
    let mut envelope = match serde_json::from_str::<serde_json::Value>(&row.bounds_json) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };

    // Identity + validity fields the envelope needs to stand alone once it is
    // divorced from the `mishpat_commitments` row. snake_case throughout — this
    // is the DHT payload's own vocabulary, and the fold reads either casing.
    envelope.insert("action".into(), serde_json::json!(row.action));
    envelope.insert("provider".into(), serde_json::json!(row.provider));
    envelope.insert("recipient".into(), serde_json::json!(row.recipient));
    envelope.insert("valid_from".into(), serde_json::json!(row.valid_from));
    envelope.insert("valid_until".into(), serde_json::json!(row.valid_until));

    let mut resource_classified_as = None;

    if row.action == "replicates-dwelling" {
        // Restore the hub-id field names the dwelling payload uses, so a reader
        // holding only `metadata_json` sees the same shape the DHT payload had.
        envelope.insert(
            "provider_dwelling_hub_id".into(),
            serde_json::json!(row.provider),
        );
        envelope.insert(
            "recipient_dwelling_hub_id".into(),
            serde_json::json!(row.recipient),
        );
    } else {
        // `replicates-commons` / `replicates-content`. The parser distinguishes
        // the two payload variants by counterparty: the `content` variant sets
        // `recipient = head_ref`; the `capacity` pledge leaves it empty.
        let variant = if row.recipient.is_empty() {
            "capacity"
        } else {
            "content"
        };
        envelope.insert("variant".into(), serde_json::json!(variant));
        if variant == "content" {
            envelope.insert("head_ref".into(), serde_json::json!(row.recipient));
            // Classification mirrors the `provide` projection's contract so the
            // existing scope-membership readers (`commitment_backed_collectives`,
            // `peer_selection`) see this row at the content's own reach. The
            // reach is READ THROUGH from the bounds (never re-vocabularized).
            if let Some(reach) = envelope.get("reach").and_then(|v| v.as_str()) {
                resource_classified_as =
                    serde_json::to_string(&vec![format!("content:{reach}")]).ok();
            }
        }
    }

    Some(ReplicationMirror {
        cid: row.cid.clone(),
        action: row.action.clone(),
        provider: row.provider.clone(),
        receiver: row.recipient.clone(),
        metadata_json: serde_json::Value::Object(envelope).to_string(),
        resource_classified_as,
        dht_anchor_hash: row.dht_anchor_hash.clone(),
    })
}

/// The mishpat→REA bridge for a `delegates-compute` commitment (Wave 4.3).
///
/// Sibling to [`ProvideProjection`]: where that descriptor bridges a content
/// provide into the resilience snapshot's `content:<reach>` provide vocabulary,
/// this descriptor bridges a `delegates-compute` commitment into the REA
/// economic facing's relation. The REA facing's `mutual_compute` fold
/// (`elohim_facings::folds::rea`) reads reciprocal `(provider, recipient)`
/// compute-delegation edges; this descriptor carries exactly the fields that
/// fold's `CommitmentRow` input needs to populate a `delegates-compute` edge.
///
/// Unlike `ProvideProjection`, `scope` is read straight off the row column
/// (`NewMishpatCommitment.scope`, set by `parse_delegates_compute`), NOT decoded
/// from `bounds_json` — the delegates-compute parser already promotes `scope` to
/// a first-class column, so there is no bounds round-trip here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatesComputeProjection {
    /// The commitment's `entry_hash` (`NewMishpatCommitment.cid`) — the natural
    /// JOIN KEY the unified `CommitmentRow` relation uses against
    /// `economic_events.bounded_by` to find the OBSERVED `compute-fulfilled`
    /// event for this delegation. This is the `entry_hash`, NOT the
    /// `action_hash`: a `compute-fulfilled` event's `bounded_by` carries the
    /// commitment `entry_hash`, and `action_hash` lives ONLY in
    /// `dht_anchor_hash` (the cid=entry_hash / anchor=action_hash invariant). A
    /// projection that omitted this could not be joined to its fulfillment.
    pub cid: String,
    /// Delegating agent (`agent_cid`, `uhCAk…`) — `mutual_compute`'s edge source.
    pub provider: String,
    /// Receiving agent (`agent_cid`) — `mutual_compute`'s edge target.
    pub recipient: String,
    /// The commitment's compute scope (e.g. `republish-epr`), read through from
    /// the `scope` column the delegates-compute parser already populated.
    pub scope: String,
    /// The commitment's `action_hash` for provenance on the projected edge.
    pub dht_anchor_hash: Option<String>,
}

/// Decide whether a just-projected commitment is a `delegates-compute` agreement
/// that should ALSO surface in the REA economic facing's relation, and with what
/// `(provider, recipient, scope)`.
///
/// Returns `Some` ONLY for `action == "delegates-compute"` (the compute-delegation
/// agreement the `mutual_compute` fold scopes to). Any other action — `provide`,
/// `replicates-content`, `replicates-commons`, `replicates-dwelling`,
/// `custody-blob`, `project-epr`, `acknowledges-reach-change`, … — returns `None`:
/// those are not compute delegations and must not mint a compute edge.
///
/// This is PROJECTION ONLY — no DHT write, no new entry type. The
/// `delegates-compute` row is already notarized and already landed in
/// `mishpat_commitments` (via the `parse_delegates_compute` → `Upsert` path); this
/// bridge does not duplicate that row into a second ledger. It is the pure
/// translation a future unified `CommitmentRow` loader (the REA facing's relation
/// materializer) consumes to add the `delegates-compute` edge — exactly as
/// `signals.rs` consumes [`provide_projection_for`] for the content-provide edge.
///
/// Pure — no DB, no conductor.
pub fn delegates_compute_projection_for(
    row: &NewMishpatCommitment,
) -> Option<DelegatesComputeProjection> {
    if row.action != "delegates-compute" {
        return None;
    }
    Some(DelegatesComputeProjection {
        // `cid` is the commitment entry_hash (the bounded_by join key), NOT the
        // action_hash — `parse_delegates_compute` set `cid = entry_hash`.
        cid: row.cid.clone(),
        provider: row.provider.clone(),
        recipient: row.recipient.clone(),
        scope: row.scope.clone(),
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

    // ── delegates_compute_projection_for (Wave 4.3 mishpat→rea bridge) ─────────

    #[test]
    fn delegates_compute_projection_maps_provider_recipient_scope() {
        // A delegates-compute commitment projects into the REA relation with
        // provider/recipient/scope mapped correctly (the mishpat→rea bridge).
        let row = unwrap_upsert(
            parse_commitment_payload(
                "delegates-compute",
                &delegates_compute_payload(),
                "uhCEk-entry-abc",
                "uhCkk-action-xyz",
            )
            .expect("well-formed delegates-compute must parse"),
        );

        let proj = delegates_compute_projection_for(&row)
            .expect("a delegates-compute row must bridge into the REA relation");

        // `cid` is the entry_hash — the bounded_by join key to the OBSERVED
        // compute-fulfilled event — NOT the action_hash.
        assert_eq!(
            proj.cid, "uhCEk-entry-abc",
            "cid must be the commitment entry_hash (the bounded_by join key)"
        );
        assert_ne!(
            proj.cid, "uhCkk-action-xyz",
            "cid must NOT be the action_hash (that lives only in dht_anchor_hash)"
        );
        assert_eq!(proj.provider, "agent:matthew-steward");
        assert_eq!(proj.recipient, "agent:deploy-svc-matthew");
        assert_eq!(
            proj.scope, "republish-epr",
            "scope is read through from the row column, not bounds_json"
        );
        assert_eq!(
            proj.dht_anchor_hash.as_deref(),
            Some("uhCkk-action-xyz"),
            "provenance anchor carries through for the projected edge"
        );
    }

    #[test]
    fn non_delegates_compute_row_does_not_bridge() {
        // A replicates-dwelling row (also provider/recipient-shaped) is NOT a
        // compute delegation and must return None — only delegates-compute bridges.
        let dwelling = unwrap_upsert(
            parse_commitment_payload(
                "replicates-dwelling",
                &replicates_dwelling_payload(),
                "eh-dwell",
                "ah-dwell",
            )
            .expect("well-formed replicates-dwelling must parse"),
        );
        assert!(
            delegates_compute_projection_for(&dwelling).is_none(),
            "replicates-dwelling is not a compute delegation → None"
        );

        // A content provide row (acknowledges/replicates-commons) is likewise None.
        let acknowledge = unwrap_upsert(
            parse_commitment_payload(
                "acknowledges-reach-change",
                &serde_json::json!({
                    "acknowledger": "agent:a",
                    "target_epr_cid": "epr:x",
                    "signed_at": "2026-05-28T00:00:00Z"
                })
                .to_string(),
                "eh-ack",
                "ah-ack",
            )
            .expect("acknowledges-reach-change must parse"),
        );
        assert!(
            delegates_compute_projection_for(&acknowledge).is_none(),
            "acknowledges-reach-change is not a compute delegation → None"
        );
    }

    // ── binds-identity (identity-head projection, Wave C1) ─────────────────────

    fn binds_identity_payload() -> String {
        serde_json::json!({
            "action": "binds-identity",
            "chain_root": "bafyreichainrootgenesis0000",
            "head_key": "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
            "controllers": [
                "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
                "uhCAkRecoveryQuorumKey"
            ],
            "controller_policy": { "kind": "recovery-quorum", "m": 2, "n": 3 },
            "signed_at": "2026-07-17T12:00:00Z"
        })
        .to_string()
    }

    #[test]
    fn parse_binds_identity_projects_head_row() {
        // A binds-identity commitment projects into the A-class identity_heads table
        // with chain_root/head_key/controllers/policy read through, cid = entry_hash,
        // dht_anchor_hash = action_hash.
        let proj = parse_commitment_payload(
            "binds-identity",
            &binds_identity_payload(),
            "uhCEk-identity-entry",
            "uhCkk-identity-action",
        )
        .expect("well-formed binds-identity must parse");

        let row = match proj {
            CommitmentProjection::UpsertIdentityHead(r) => r,
            other => panic!("expected UpsertIdentityHead, got {other:?}"),
        };
        assert_eq!(row.cid, "uhCEk-identity-entry");
        assert_eq!(row.chain_root, "bafyreichainrootgenesis0000");
        assert_eq!(
            row.head_key,
            "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH"
        );
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-identity-action"),
            "dht_anchor_hash = action_hash (notarised provenance)"
        );
        let controllers: Vec<String> =
            serde_json::from_str(&row.controllers_json).expect("controllers_json is a JSON array");
        assert_eq!(controllers.len(), 2);
        let policy: serde_json::Value =
            serde_json::from_str(&row.controller_policy_json).expect("policy is JSON");
        assert_eq!(policy["kind"], "recovery-quorum");
        assert_eq!(policy["m"], 2);
        // signed_at is read best-effort from the payload at parse time (the signal
        // handler overrides with the authoritative Commitment-envelope value).
        assert_eq!(row.signed_at, "2026-07-17T12:00:00Z");
    }

    #[test]
    fn parse_binds_identity_empty_controllers_rejected() {
        // Ontology guard (defense-in-depth): a head with no controllers must never
        // project — a head cannot exist without its controller-set.
        let mut payload: serde_json::Value =
            serde_json::from_str(&binds_identity_payload()).unwrap();
        payload["controllers"] = serde_json::json!([]);
        let result = parse_commitment_payload(
            "binds-identity",
            &payload.to_string(),
            "eh-empty",
            "ah-empty",
        );
        assert!(result.is_err(), "empty controllers must return Err");
        assert!(result.unwrap_err().contains("controllers"));
    }

    #[test]
    fn parse_binds_identity_missing_chain_root_rejected() {
        let mut payload: serde_json::Value =
            serde_json::from_str(&binds_identity_payload()).unwrap();
        payload.as_object_mut().unwrap().remove("chain_root");
        let result =
            parse_commitment_payload("binds-identity", &payload.to_string(), "eh-nc", "ah-nc");
        assert!(result.is_err(), "missing chain_root must return Err");
        assert!(result.unwrap_err().contains("chain_root"));
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

    // ── author-lens role enum (projection-side defense-in-depth) ───────────────

    fn author_lens_payload(role: &str) -> String {
        serde_json::json!({
            "action": "author-lens",
            "governs_epr": "epr:lamad-spa",
            "school": "georgist",
            "role": role,
            "rule": { "predicate": "land_value_uplift > rent_capture" },
            "telos": { "summary": "tax unimproved land value" }
        })
        .to_string()
    }

    #[test]
    fn parse_author_lens_rejects_out_of_enum_role() {
        // The S1 coordinator validator enforces role ∈ {lens, floor, ceiling}, but
        // it is a HOT-SWAPPED coordinator gate — a stale-coordinator peer or a replay
        // predating the validator can deliver an unvalidated payload. The projection
        // must re-establish the closed enum so a bogus constitutional role
        // (e.g. role='dictator', feeding the floor/ceiling governance seam) never
        // reaches the `lenses` table.
        let payload = author_lens_payload("dictator");
        let result = parse_commitment_payload("author-lens", &payload, "eh1", "ah1");
        assert!(
            result.is_err(),
            "an out-of-enum role must be rejected at the projection layer"
        );
        assert!(
            result.unwrap_err().contains("role"),
            "the error must name the offending 'role' field"
        );
    }

    #[test]
    fn parse_author_lens_accepts_constitutional_roles() {
        for role in ["lens", "floor", "ceiling"] {
            let payload = author_lens_payload(role);
            let result = parse_commitment_payload("author-lens", &payload, "eh1", "ah1");
            assert!(
                matches!(result, Ok(CommitmentProjection::UpsertLens(_))),
                "role '{role}' is a valid constitutional role and must project a lens"
            );
        }
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

#[cfg(test)]
mod lineage_projection_tests {
    use super::*;

    /// The measured Station-2 defect, pinned: a notarized `migrates-lineage`
    /// commitment must project ACTIVE, because `verify_path` establishes a path
    /// only on `"active"` and nothing else in this codebase ever writes that
    /// value. Before this arm existed the action fell to the unknown-action
    /// fallback and landed `"proposed"`, and every correct migration path on
    /// the household mesh was refused `path_not_notarized` ("is proposed, not
    /// active").
    #[test]
    fn migrates_lineage_projects_active_with_its_window() {
        let payload = serde_json::json!({
            "action": "migrates-lineage",
            "role": "node_registry",
            "from_dna_hash": "uhC0kFROM",
            "to_dna_hash": "uhC0kTO",
            "release_cid": "uhCkkRELEASE",
            "constitution_root": "root",
            "roster_cid": "roster",
            "signing_payload_cid": "bafySigning",
            "signatures": [{ "agent": "uhCAkSIGNER", "signature": "c2ln" }],
            "required_signatures": 1,
            "window": { "opens_at": "2026-09-04T00:00:00Z", "revert_until": "2026-09-05T00:00:00Z" }
        });
        let projected = parse_commitment_payload(
            "migrates-lineage",
            &payload.to_string(),
            "uhCEkENTRY",
            "uhCkkACTION",
        )
        .expect("a well-formed migrates-lineage payload projects");
        let CommitmentProjection::Upsert(row) = projected else {
            panic!("migrates-lineage must project into mishpat_commitments");
        };
        assert_eq!(
            row.state, "active",
            "a notarized path is active on notarization"
        );
        assert_eq!(row.cid, "uhCEkENTRY");
        assert_eq!(row.action, "migrates-lineage");
        assert_eq!(row.scope, "node_registry");
        assert_eq!(row.valid_from, "2026-09-04T00:00:00Z");
        assert_eq!(row.valid_until, "2026-09-05T00:00:00Z");
        assert_eq!(row.revoked_at, None);
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkkACTION"));
    }

    /// A sunset has no revert horizon by construction — its window carries
    /// `sunsets_at` alone, and the row must not invent a far edge for it.
    #[test]
    fn sunsets_lineage_projects_active_with_no_far_edge() {
        let payload = serde_json::json!({
            "action": "sunsets-lineage",
            "role": "node_registry",
            "migration_commitment_cid": "uhCEkMIGRATION",
            "window": { "sunsets_at": "2026-09-09T00:00:00Z" }
        });
        let projected = parse_commitment_payload(
            "sunsets-lineage",
            &payload.to_string(),
            "uhCEkSUNSET",
            "uhCkkACTION2",
        )
        .expect("a well-formed sunsets-lineage payload projects");
        let CommitmentProjection::Upsert(row) = projected else {
            panic!("sunsets-lineage must project into mishpat_commitments");
        };
        assert_eq!(row.state, "active");
        assert_eq!(row.valid_from, "2026-09-09T00:00:00Z");
        assert_eq!(row.valid_until, "");
    }

    /// **The forgery this keying closes.** `parse_commitment_payload` dispatches
    /// on the ENTRY's action (`signals.rs` passes `commitment.action`), not on
    /// `payload_json`'s second copy of it. `validate_ratifies_limit_gradient`
    /// never pins the two together, so a commitment created as
    /// `ratifies-limit-gradient` whose body declares `"action":
    /// "migrates-lineage"` is accepted by the coordinator with no signatures at
    /// all — and projecting THAT as `active` would hand it a migration path.
    #[test]
    fn a_forged_body_action_never_projects_active() {
        let forged = serde_json::json!({
            "action": "migrates-lineage",
            "role": "node_registry",
            "signatures": [],
            "window": { "opens_at": "2026-09-04T00:00:00Z", "revert_until": "2026-09-05T00:00:00Z" }
        });
        let projected = parse_commitment_payload(
            // What the ENTRY actually is.
            "ratifies-limit-gradient",
            &forged.to_string(),
            "uhCEkFORGED",
            "uhCkkFORGED",
        )
        .expect("projects (via the unknown-action fallback)");
        let CommitmentProjection::Upsert(row) = projected else {
            panic!("expected a commitment row");
        };
        assert_eq!(
            row.action, "ratifies-limit-gradient",
            "the row must record what the ENTRY is, not what its body claims"
        );
        assert_eq!(
            row.state, "proposed",
            "a body claiming migrates-lineage under another entry action must never project active"
        );
    }

    /// The classes with a real acceptance step are UNTOUCHED — this arm is
    /// about the lineage actions only, never a loosening of the ledger.
    #[test]
    fn delegates_compute_still_projects_proposed() {
        let payload = serde_json::json!({
            "scope": "compute",
            "provider": "p",
            "recipient": "r",
            "valid_from": "2026-09-04T00:00:00Z",
            "valid_until": "2026-09-05T00:00:00Z",
            "bounds": {}
        });
        let projected = parse_commitment_payload(
            "delegates-compute",
            &payload.to_string(),
            "uhCEkDC",
            "uhCkkDC",
        )
        .expect("projects");
        let CommitmentProjection::Upsert(row) = projected else {
            panic!("delegates-compute projects a commitment row");
        };
        assert_eq!(row.state, "proposed");
    }
}
