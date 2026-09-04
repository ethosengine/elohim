//! Mishpat Commitment coordinator — Z.D substrate-correct deploy primitive.
//!
//! Authors the `delegates-compute` and `acknowledges-reach-change` Commitment
//! actions per genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md.
//!
//! See `bounds-validator-pattern` memory: per-instance validators consume the
//! Commitment via `services::commitment_fetcher::CommitmentFetcher` and
//! `services::bounds_validator::validate` in elohim-storage.

use hdk::prelude::*;
use mishpat_integrity::{Commitment, LinkTypes};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentInput {
    pub action: String,
    pub payload_json: String,
    /// Caller-supplied ISO-8601 (or epoch-seconds) timestamp. Replaces the
    /// in-zome `sys_time()` call so the notarized commitment carries a
    /// deterministic, caller-controlled signing time (Slice 2b T1). The
    /// projection writes this onto the `mishpat_commitments` row; the bounds
    /// validator still reads `valid_from`/`valid_until` from `payload_json`.
    pub signed_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
}

#[hdk_extern]
pub fn create_commitment(input: CreateCommitmentInput) -> ExternResult<CommitmentOutput> {
    validate_commitment_payload(&input).map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    let entry = Commitment {
        action: input.action.clone(),
        payload_json: input.payload_json.clone(),
        signed_at: input.signed_at.clone(),
    };

    let action_hash = create_entry(&mishpat_integrity::EntryTypes::Commitment(entry.clone()))?;
    let entry_hash = hash_entry(&entry)?;
    Ok(CommitmentOutput {
        action_hash,
        entry_hash,
    })
}

// =============================================================================
// CommitmentByState link author (Slice-2b T11)
// =============================================================================
//
// Records a Commitment's lifecycle transition (proposed → active → …) as an
// immutable `CommitmentByState` link on the Mishpat DHT. The SQL `state` column
// in elohim-storage becomes a write-through cache: `graduate_to_active` writes
// the cache; this link is the truth. Peers verify lifecycle by reading the link
// off the commitment anchor — no need to replay every EconomicEvent.

/// Input for `create_commitment_state_link`. `commitment_cid` is the base64
/// `EntryHash` of the Commitment (the same value elohim-storage stores as
/// `mishpat_commitments.cid` and `get_commitment` takes). `event_hash` is the
/// base64 `ActionHash` of the EconomicEvent that justifies the transition.
/// `signed_at` is the deterministic, caller-supplied signing time (Category-A —
/// never `sys_time()` in-zome).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentStateLinkInput {
    pub commitment_cid: String,
    pub state: String,
    pub event_hash: String,
    pub signed_at: String,
}

/// Output of `create_commitment_state_link` — the new link's `ActionHash`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentStateLinkOutput {
    pub link_action_hash: ActionHash,
}

/// Author a `CommitmentByState` link recording a commitment's state transition.
///
/// - **base** = the Commitment's `EntryHash` (resolved from `commitment_cid`) —
///   so the link is readable off the commitment anchor by any peer.
/// - **target** = the graduating event's `ActionHash` (resolved from
///   `event_hash`) — the proof a verifier can replay.
/// - **tag** = `"<state>|<signed_at>"` — the new lifecycle state + signing time;
///   the integrity zome enforces both segments are non-empty.
///
/// The link is immutable (links never update). Called by the elohim-storage
/// graduation projection right after `graduate_to_active` flips the SQL cache.
#[hdk_extern]
pub fn create_commitment_state_link(
    input: CreateCommitmentStateLinkInput,
) -> ExternResult<CommitmentStateLinkOutput> {
    if input.state.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_commitment_state_link: state must be non-empty".to_string()
        )));
    }
    if input.signed_at.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_commitment_state_link: signed_at must be non-empty".to_string()
        )));
    }

    // The commitment anchor: base64 EntryHash → EntryHash (the link base).
    let base = EntryHash::try_from(input.commitment_cid.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "create_commitment_state_link: invalid commitment_cid (expected base64 EntryHash): {}",
            input.commitment_cid
        )))
    })?;

    // The transition proof: base64 ActionHash → ActionHash (the link target).
    let target = ActionHash::try_from(input.event_hash.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "create_commitment_state_link: invalid event_hash (expected base64 ActionHash): {}",
            input.event_hash
        )))
    })?;

    // Tag carries the new state + signing time: "<state>|<signed_at>".
    let tag_str = format!("{}|{}", input.state, input.signed_at);
    let link_action_hash = create_link(
        base,
        target,
        LinkTypes::CommitmentByState,
        LinkTag::new(tag_str.as_bytes().to_vec()),
    )?;

    Ok(CommitmentStateLinkOutput { link_action_hash })
}

/// A `CommitmentByState` link projected to a wire shape. `state`/`signed_at`
/// are parsed from the LinkTag (`"<state>|<signed_at>"`); `event_hash` is the
/// link target (the graduating event's ActionHash) as base64.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentStateLink {
    pub state: String,
    pub signed_at: String,
    pub event_hash: String,
}

/// Read all `CommitmentByState` links off a commitment anchor (base64
/// `EntryHash`). Returns the lifecycle transitions notarized on the DHT —
/// the peer-observable lifecycle without replaying every EconomicEvent.
///
/// Used by the Slice-2b T11 sweettest to verify cross-conductor replication,
/// and available to any storage observer that wants to project lifecycle from
/// DHT-truth rather than the SQL write-through cache.
#[hdk_extern]
pub fn get_commitment_state_links(
    commitment_cid: String,
) -> ExternResult<Vec<CommitmentStateLink>> {
    let base = EntryHash::try_from(commitment_cid.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "get_commitment_state_links: invalid commitment_cid (expected base64 EntryHash): {commitment_cid}"
        )))
    })?;

    let query = LinkQuery::try_new(base, LinkTypes::CommitmentByState)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut out = Vec::with_capacity(links.len());
    for link in links {
        let raw = String::from_utf8(link.tag.0.clone()).unwrap_or_default();
        let mut parts = raw.splitn(2, '|');
        let state = parts.next().unwrap_or("").to_string();
        let signed_at = parts.next().unwrap_or("").to_string();
        // Target is the graduating event's ActionHash; render as base64.
        let event_hash = ActionHash::try_from(link.target.clone())
            .map(|h| h.to_string())
            .unwrap_or_default();
        out.push(CommitmentStateLink {
            state,
            signed_at,
            event_hash,
        });
    }
    Ok(out)
}

/// Validate the commitment payload against the action-specific schema.
/// For `delegates-compute` action, validates against delegates-compute.schema.json
/// (hand-rolled — HDK WASM size constraints prohibit a full JSON Schema library).
pub fn validate_commitment_payload(input: &CreateCommitmentInput) -> Result<(), String> {
    let payload: serde_json::Value = serde_json::from_str(&input.payload_json)
        .map_err(|e| format!("payload_json not parseable: {e}"))?;

    match input.action.as_str() {
        "delegates-compute" => validate_delegates_compute(&payload),
        // TODO(sprint1-task3): implement acknowledges-reach-change validation
        "acknowledges-reach-change" => validate_acknowledges_reach_change(&payload),
        "replicates-dwelling" => validate_replicates_dwelling(&payload),
        // `replicates-content` is the reach-general content-provide action;
        // `replicates-commons` is the migration-window alias (Stage B rename).
        // Both route to the same reach-general validator for one window.
        "replicates-content" | "replicates-commons" => validate_replicates_content(&payload),
        "revokes-commitment" => validate_revokes_commitment(&payload),
        "ratifies-limit-gradient" => validate_ratifies_limit_gradient(&payload),
        "sets-authority-arc" => validate_sets_authority_arc(&payload),
        "author-lens" => validate_author_lens(&payload),
        "binds-identity" => validate_binds_identity(&payload),
        "migrates-lineage" => validate_migrates_lineage(&payload),
        "sunsets-lineage" => validate_sunsets_lineage(&payload),
        other => Err(format!(
            "commitments::validate_commitment_payload unhandled action: {other}"
        )),
    }
}

/// Validator for the `replicates-content` action (EPR provide loop; Stage B
/// generalization of the former commons-only `replicates-commons`).
/// Variant-dispatch on `variant` ("content" | "capacity"). The **content**
/// reach is now reach-general (any non-empty string — the projection floor),
/// NOT pinned to "commons"; the `bounds.reach_ceiling == "commons"` invariant is
/// UNCHANGED (it is what keeps the mishpat *integrity* zome — which gates only
/// `reach_ceiling == "commons"` — passing unmodified, so this coordinator edit
/// is DNA-hash-neutral). Capacity variant carries ratio_attestation sum-to-100 +
/// effective_ratio_cid present. Mirrors `validate_replicates_dwelling`'s style.
///
/// One-window action alias: accepts BOTH `"replicates-content"` (the new name)
/// and `"replicates-commons"` (the migration alias the author still emits until
/// the rename fully lands), structurally identical.
///
/// Note: the author does NOT supply `epr_scope`. For a content-scoped
/// commitment the effective `bounds.epr_scope` is derived as `[head_ref]` at
/// projection time (the storage `parse_replicates_commons` projection), so the
/// bounds-validator's epr_scope check is satisfied downstream — never required
/// in the author-facing payload here.
fn validate_replicates_content(payload: &serde_json::Value) -> Result<(), String> {
    // One-window action alias: accept both the renamed action and the alias.
    let action = payload["action"].as_str().unwrap_or("");
    if action != "replicates-content" && action != "replicates-commons" {
        return Err(
            "action field must equal 'replicates-content' (or the 'replicates-commons' alias)"
                .into(),
        );
    }

    // bounds: required object with rate_per_minute and reach_ceiling="commons".
    let bounds = payload
        .get("bounds")
        .and_then(|b| b.as_object())
        .ok_or_else(|| "replicates-content bounds must be object".to_string())?;
    for field in ["rate_per_minute", "reach_ceiling"] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // rate_per_minute must be a positive rate — a zero rate is a no-op
    // commitment (mirrors replicates-dwelling's positive-quantity guards).
    if bounds["rate_per_minute"].as_u64().unwrap_or(0) == 0 {
        return Err("replicates-content bounds.rate_per_minute must be > 0".into());
    }
    // KEPT UNCHANGED (load-bearing for hash-neutrality): the commitment's
    // reach_ceiling must be "commons". The mishpat *integrity* zome gates ONLY
    // `reach_ceiling == "commons"` (never the top-level content `reach`), so a
    // non-commons content provide carrying `reach: <content-reach>` +
    // `reach_ceiling: "commons"` passes integrity UNMODIFIED → no DNA hash move.
    if bounds["reach_ceiling"].as_str().unwrap_or("") != "commons" {
        return Err("bounds.reach_ceiling must equal 'commons'".into());
    }

    // variant dispatch.
    let variant = payload["variant"].as_str().unwrap_or("");
    match variant {
        "content" => {
            for field in ["head_ref", "reach"] {
                if payload.get(field).is_none() {
                    return Err(format!(
                        "replicates-content content variant missing field: {field}"
                    ));
                }
            }
            if payload["head_ref"].as_str().unwrap_or("").is_empty() {
                return Err("replicates-content head_ref must be non-empty".into());
            }
            // Stage B: the content `reach` is now reach-GENERAL — any non-empty
            // string is admissible (this is the projection floor only). We do
            // NOT enforce schema-8 membership here: production content carries
            // reaches outside the schema-8 DNA vocabulary (`local` (~11.5k rows),
            // `household`, `neighborhood`, …) and a membership gate would reject
            // those exact rows (the reach-vocab drift, roadmap-13, deferred).
            // The `reach_ceiling == "commons"` invariant above is what bounds the
            // offer (and keeps integrity hash-neutral); the content's own reach
            // is read through faithfully by the storage projection.
            if payload["reach"].as_str().unwrap_or("").is_empty() {
                return Err("replicates-content content reach must be non-empty".into());
            }
            // content variant carries NO ratio_attestation.
            if payload.get("ratio_attestation").is_some() {
                return Err(
                    "replicates-content content variant must not carry ratio_attestation".into(),
                );
            }
            Ok(())
        }
        "capacity" => {
            // commons_bytes > 0.
            let bytes = payload["commons_bytes"].as_u64().unwrap_or(0);
            if bytes == 0 {
                return Err("replicates-content commons_bytes must be > 0".into());
            }
            // ratio_attestation: required sub-fields + sum-to-100 (mirrors replicates-dwelling).
            let attestation = payload
                .get("ratio_attestation")
                .and_then(|v| v.as_object())
                .ok_or("replicates-content capacity variant requires ratio_attestation object")?;
            for f in [
                "commons_pct",
                "dwelling_pct",
                "collective_pct",
                "free_pct",
                "effective_ratio_cid",
            ] {
                if !attestation.contains_key(f) {
                    return Err(format!("ratio_attestation missing field: {f}"));
                }
            }
            if attestation["effective_ratio_cid"]
                .as_str()
                .unwrap_or("")
                .is_empty()
            {
                return Err("ratio_attestation effective_ratio_cid must be non-empty".into());
            }
            let commons = attestation["commons_pct"].as_u64().unwrap_or(0);
            let dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0);
            let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
            let free = attestation["free_pct"].as_u64().unwrap_or(0);
            if commons + dwelling + collective + free != 100 {
                return Err(format!(
                    "ratio_attestation pct sum {} != 100",
                    commons + dwelling + collective + free
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "replicates-content variant '{other}' not in enum (content|capacity)"
        )),
    }
}

fn validate_revokes_commitment(payload: &serde_json::Value) -> Result<(), String> {
    if payload["action"] != "revokes-commitment" {
        return Err("action field must equal 'revokes-commitment'".into());
    }
    let target = payload
        .get("target_cid")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if target.is_empty() {
        return Err("revokes-commitment target_cid must be non-empty".into());
    }
    let signed = payload
        .get("signed_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if signed.is_empty() {
        return Err("revokes-commitment signed_at must be present".into());
    }
    // Quorum on revocation (Task 2): when the payload names the target
    // commitment's action (`target_action`), the target is a lineage
    // commitment (`migrates-lineage` | `sunsets-lineage` are the only actions
    // that carry `target_action` in this MVP) — the SAME signature-quorum rule
    // that authored the lineage commitment gates pulling it back. A lineage
    // commitment cannot be revoked by one signer's say-so alone.
    if payload
        .get("target_action")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .is_some()
    {
        validate_lineage_signatures(payload)?;
    }
    Ok(())
}

// =============================================================================
// DNA walls for the limitarian gradient (spec §5.2 — reject-at-write; a config
// that exists is, by construction, in-wall). Widths are TBD-operator (spec
// §Decision 2); the SHAPE (that a wall exists, that α cannot blind the tail,
// that loosening is witnessed) is decided. Mirror: storage
// limit_gradient_registry.rs — keep in lockstep.
// =============================================================================
const ALPHA_WALL: (f64, f64) = (1.0, 2.0);
const C_TARGET_WALL: (f64, f64) = (0.05, 0.30);
const K_MAX_WALL: (f64, f64) = (0.01, 0.10);
const BASE_RATE_WALL: (f64, f64) = (0.0005, 0.005);
const GAMMA_WALL: (f64, f64) = (0.5, 2.0);

fn wall_check(
    payload: &serde_json::Value,
    path: &[&str],
    wall: (f64, f64),
    name: &str,
) -> Result<(), String> {
    let mut v = payload;
    for key in path {
        v = v
            .get(key)
            .ok_or_else(|| format!("ratifies-limit-gradient missing field: {}", path.join(".")))?;
    }
    let x = v
        .as_f64()
        .ok_or_else(|| format!("{name} must be a number"))?;
    if x < wall.0 || x > wall.1 {
        return Err(format!(
            "{name}={x} outside DNA wall [{}, {}] — out-of-wall values cannot be ratified (reject-at-write)",
            wall.0, wall.1
        ));
    }
    Ok(())
}

fn validate_ratifies_limit_gradient(payload: &serde_json::Value) -> Result<(), String> {
    for field in [
        "substrate_signal",
        "governance_layer",
        "measure",
        "shape",
        "base_rate",
        "k_max",
        "dignity_floor",
        "valid_from",
        "valid_until",
        "ratified_by_governance_action_cid",
    ] {
        if payload.get(field).is_none() {
            return Err(format!(
                "ratifies-limit-gradient missing required field: {field}"
            ));
        }
    }
    wall_check(payload, &["measure", "alpha"], ALPHA_WALL, "measure.alpha")?;
    wall_check(
        payload,
        &["shape", "C_target"],
        C_TARGET_WALL,
        "shape.C_target",
    )?;
    wall_check(payload, &["shape", "gamma"], GAMMA_WALL, "shape.gamma")?;
    wall_check(payload, &["base_rate"], BASE_RATE_WALL, "base_rate")?;
    wall_check(payload, &["k_max"], K_MAX_WALL, "k_max")?;

    let floor = payload
        .get("dignity_floor")
        .and_then(|v| v.as_f64())
        .unwrap_or(-1.0);
    if floor < 0.0 {
        return Err("dignity_floor must be >= 0".into());
    }

    // Loosening witness (spec §5.4 v1-minimal): any param looser than the core
    // default requires loosening_acknowledged=true. Core defaults inline (the
    // WASM cannot read the storage registry; lockstep with GradientConfig::default):
    //   c_target default = 0.15, k_max default = 0.05, dignity_floor default = 100.0
    // Whole-arc review W2: the floor check covers the ENTIRE below-default
    // range (a floor of 1.0 guts sufficientarian protection as surely as 0.0).
    // NAMED GAP (TBD-operator, spec §Decision 2): k_s (shape gain) and S_target
    // are not loosening-checked in v1 — a k_s=0 ratification would near-
    // extinguish the shape term unwitnessed; wall them when the operator
    // derives the wall widths.
    let loosens = payload["shape"]["C_target"]
        .as_f64()
        .map(|v| v > 0.15)
        .unwrap_or(false)
        || payload["k_max"].as_f64().map(|v| v < 0.05).unwrap_or(false)
        || floor < 100.0;
    if loosens {
        let acked = payload
            .get("loosening_acknowledged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !acked {
            return Err(
                "loosening override requires loosening_acknowledged=true (witnessed loosening, spec §5.4)"
                    .into(),
            );
        }
    }
    Ok(())
}

/// Validator for the `sets-authority-arc` action — a bounded, revocable grant of
/// authority to set a node's conductor authority-arc factor (the memory↔resilience
/// trade; spec `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md` §5).
///
/// COORDINATOR-side validation (create-time), like every commitment action — so
/// adding it is **DNA-hash-NEUTRAL**: the `mishpat_integrity` zome is untouched
/// and this validator hot-swaps via `update_coordinators`. The deployed arc
/// lever is `{0,1}` (full anchor vs accountable leecher; no fractional arc until
/// kitsune2 sharding lands — spec §2), so the granted factor range is constrained
/// to `{0,1}`. The grant's validity window is the commitment's top-level
/// `valid_from`/`valid_until`; the storage-side actuator
/// (`services::arc_actuator`) enforces expiry + the LIVE coverage gate at
/// actuation time (the zome cannot see the live peer count).
fn validate_sets_authority_arc(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "scope",
        "provider",
        "recipient",
        "bounds",
        "valid_from",
        "valid_until",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!(
                "sets-authority-arc missing required field: {field}"
            ));
        }
    }
    if payload["action"] != "sets-authority-arc" {
        return Err("action field must equal 'sets-authority-arc'".into());
    }
    let bounds = payload
        .get("bounds")
        .and_then(|b| b.as_object())
        .ok_or_else(|| "sets-authority-arc bounds must be object".to_string())?;
    for field in ["knob", "min_factor", "max_factor", "coverage_floor"] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // Only the conductor authority-arc factor is grantable through this action.
    if bounds["knob"].as_str().unwrap_or("") != "conductor.target_arc_factor" {
        return Err("bounds.knob must equal 'conductor.target_arc_factor'".into());
    }
    // The deployed lever is {0,1} (spike §2: no fractional arc; holochain_p2p
    // hard-clamps any factor > 1). Grants must stay within that domain.
    let min = bounds["min_factor"]
        .as_u64()
        .ok_or_else(|| "bounds.min_factor must be an integer".to_string())?;
    let max = bounds["max_factor"]
        .as_u64()
        .ok_or_else(|| "bounds.max_factor must be an integer".to_string())?;
    if min > 1 || max > 1 {
        return Err(
            "bounds.min_factor/max_factor must be in {0,1} (the deployed arc lever; \
             no fractional arc — spec §2)"
                .into(),
        );
    }
    if min > max {
        return Err("bounds.min_factor must be <= max_factor".into());
    }
    // coverage_floor: the per-key redundancy floor the grant pins. Must be > 0 —
    // a zero floor would permit a coverage-destroying leecher.
    if bounds["coverage_floor"].as_u64().unwrap_or(0) == 0 {
        return Err("sets-authority-arc bounds.coverage_floor must be > 0".into());
    }
    Ok(())
}

/// Validator for the `author-lens` action — the lens-market "teeth" (plan S1 of
/// `genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-service-layer-plan.md`,
/// payload contract I1; charter §8).
///
/// A **Lens** is a `Mishpat::Commitment` with `action="author-lens"`; the whole
/// concept lives in `payload_json` — zero integrity-struct change. This is the
/// closed-coordinator gate (a malformed lens is rejected at create-time), so
/// adding it is **DNA-hash-NEUTRAL**: the `mishpat_integrity` zome is untouched
/// and this validator hot-swaps via `update_coordinators`. The storage projection
/// (plan S3) keys the `lenses` table on `cid == entry_hash`; `governs_epr` is the
/// EPR **slug-id** scope key (plan A3), NOT the dag-cbor CID — a forward index
/// reuses the existing SQL scope projection, so no new DHT entry/link type.
///
/// `role` ∈ {lens, floor, ceiling}: a plain lens is one school's reading; floor
/// and ceiling are constitutional bounds (the wisdom-layer floor/ceiling seam).
/// `rule` is the deterministic predicate (the teeth) and `telos` is what the lens
/// steers toward (viability and/or justice) — both required objects.
fn validate_author_lens(payload: &serde_json::Value) -> Result<(), String> {
    let required = ["action", "governs_epr", "school", "role", "rule", "telos"];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("author-lens missing required field: {field}"));
        }
    }
    if payload["action"] != "author-lens" {
        return Err("action field must equal 'author-lens'".into());
    }
    // governs_epr is the scope key (slug-id, plan A3) — must be a non-empty
    // string, else the lens binds to no scope row.
    if payload["governs_epr"].as_str().unwrap_or("").is_empty() {
        return Err("author-lens governs_epr must be a non-empty slug-id".into());
    }
    if payload["school"].as_str().unwrap_or("").is_empty() {
        return Err("author-lens school must be a non-empty string".into());
    }
    // role enum: a plain lens, or a constitutional floor/ceiling bound.
    let role = payload["role"].as_str().unwrap_or("");
    if !matches!(role, "lens" | "floor" | "ceiling") {
        return Err(format!(
            "author-lens role '{role}' not in enum (lens|floor|ceiling)"
        ));
    }
    // rule (the deterministic predicate — the teeth) and telos (what it steers
    // toward) must both be objects.
    if !payload["rule"].is_object() {
        return Err("author-lens rule must be an object (the deterministic predicate)".into());
    }
    if !payload["telos"].is_object() {
        return Err("author-lens telos must be an object".into());
    }
    Ok(())
}

/// Validator for the `binds-identity` action — the identity-head declaration
/// (Wave B of the identity-head-key-lineage plan; design §2.2 / §3).
///
/// A **binds-identity** commitment IS a `Mishpat::Commitment` with
/// `action="binds-identity"`; the whole declaration lives in `payload_json`
/// (zero integrity-struct change → **DNA-hash-NEUTRAL**, the `author-lens` /
/// `binds-policy` precedent: the mishpat integrity zome's action dispatch ends
/// `_ => None`, so a new discriminator passes integrity unmodified and this
/// validator hot-swaps via `update_coordinators`).
///
/// It declares, for one identity chain: *"chain-root C's current head is key K;
/// controllers = {set}; controller-policy = self | steward-set |
/// recovery-quorum(M,N)."* The **B0 architecture decision**: lineage lives on
/// the imagodei `KeyRotation` DAG + chain-root derivation; mishpat owns THIS
/// declaration referencing the imagodei chain-root. `chain_root` is a content
/// reference (the imagodei genesis-key / CID string) — NOT re-derived here; the
/// coordinator carries it through faithfully (the imagodei↔mishpat cross-DNA
/// link is a CID reference, not a runtime bridge).
///
/// Ontology guard (imago-dei, structural): `controllers` MUST be non-empty — a
/// head cannot exist without its controller-set; the recovery quorum is a
/// *controller*, named in the same `controller_policy` field that names self,
/// never an override bolted on.
fn validate_binds_identity(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "chain_root",
        "head_key",
        "controllers",
        "controller_policy",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("binds-identity missing required field: {field}"));
        }
    }
    if payload["action"] != "binds-identity" {
        return Err("action field must equal 'binds-identity'".into());
    }
    // chain_root: the stable identity-chain identifier (imagodei genesis-key /
    // CID). Non-empty string — an empty root binds to no chain.
    if payload["chain_root"].as_str().unwrap_or("").is_empty() {
        return Err("binds-identity chain_root must be a non-empty reference".into());
    }
    // head_key: the current head of the chain. Non-empty string.
    let head_key = payload["head_key"].as_str().unwrap_or("");
    if head_key.is_empty() {
        return Err("binds-identity head_key must be a non-empty key reference".into());
    }
    // controllers: non-empty array of non-empty strings (ontology guard — the
    // head cannot exist without its controller-set).
    let controllers = payload
        .get("controllers")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "binds-identity controllers must be an array".to_string())?;
    if controllers.is_empty() {
        return Err(
            "binds-identity controllers must be non-empty (a head cannot exist without \
             its controller-set)"
                .into(),
        );
    }
    if controllers
        .iter()
        .any(|c| c.as_str().unwrap_or("").is_empty())
    {
        return Err("binds-identity controllers entries must be non-empty strings".into());
    }
    // controller_policy: object with `kind` ∈ {self, steward-set, recovery-quorum}.
    let policy = payload
        .get("controller_policy")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "binds-identity controller_policy must be an object".to_string())?;
    let kind = policy.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    match kind {
        "self" => {
            // self-policy: the current head IS its own controller — the head_key
            // must appear in the controller-set (structural, not a bolt-on).
            let head_is_controller = controllers.iter().any(|c| c.as_str() == Some(head_key));
            if !head_is_controller {
                return Err(
                    "binds-identity self-policy requires head_key to be listed in controllers"
                        .into(),
                );
            }
            Ok(())
        }
        "steward-set" => {
            // steward-set: the named controller stewards authorize. The set is
            // already validated non-empty above; nothing further at declare-time.
            //
            // Declare/authorize asymmetry (INTENTIONAL): accepting the policy HERE
            // records the declared intent — it does NOT imply a rotation under it is
            // authorizable yet. imagodei `authorize_rotation` (identity_lineage.rs,
            // the `ControllerPolicy::StewardSet` arm) REFUSES steward-set rotations
            // until Wave C resolves the notarized controller set. Declaring is safe;
            // authorizing is gated.
            Ok(())
        }
        "recovery-quorum" => {
            // recovery-quorum(M,N): reuses the wired imagodei
            // RecoveryAuthority/RecoveryRequest semantics at rotation time; here
            // we validate the declared threshold shape only.
            let m = policy.get("m").and_then(|v| v.as_u64());
            let n = policy.get("n").and_then(|v| v.as_u64());
            match (m, n) {
                (Some(m), Some(n)) => {
                    if m == 0 || m > n {
                        return Err(format!(
                            "binds-identity recovery-quorum requires 1 <= m <= n; got m={m} n={n}"
                        ));
                    }
                    Ok(())
                }
                _ => Err(
                    "binds-identity recovery-quorum controller_policy requires integer m and n"
                        .into(),
                ),
            }
        }
        other => Err(format!(
            "binds-identity controller_policy.kind '{other}' not in enum \
             (self|steward-set|recovery-quorum)"
        )),
    }
}

// =============================================================================
// Lineage arms (Holochain Evolution Epic Task 2) — migrates-lineage /
// sunsets-lineage commitments, and the signature-quorum rule they share with
// revokes-commitment when the revoked target is a lineage commitment.
//
// Quorum rule (MVP, epic spec §3): `signatures` non-empty, unique agents, and
// every signature verifies over the literal UTF-8 bytes of
// `signing_payload_cid` (NOT the field re-serialized — `verify_signature_raw`
// is required here; the msgpack-serializing `verify_signature` would check
// the signature against the msgpack encoding of the byte vector instead of
// the literal bytes the caller actually signed, and would never verify).
// `required_signatures` (default 1) is k in k-of-n; roster-chain verification
// against `roster_cid` is deferred to Task 2b (needs the elohim-DNA bridge) —
// this MVP declares the 1-of-1 progenitor roster.
// =============================================================================

fn validate_lineage_signatures(payload: &serde_json::Value) -> Result<(), String> {
    let cid = payload
        .get("signing_payload_cid")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("signing_payload_cid must be a non-empty string")?;
    let sigs = payload
        .get("signatures")
        .and_then(|v| v.as_array())
        .filter(|a| !a.is_empty())
        .ok_or("signatures must be a non-empty array")?;
    let required = payload
        .get("required_signatures")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;
    let mut seen = std::collections::BTreeSet::new();
    for s in sigs {
        let agent = s
            .get("agent")
            .and_then(|v| v.as_str())
            .ok_or("signature.agent missing")?;
        if !seen.insert(agent.to_string()) {
            return Err(format!("duplicate signer {agent}"));
        }
        let key = AgentPubKey::try_from(agent)
            .map_err(|e| format!("signature.agent invalid: {e:?}"))?;
        let raw = {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            STANDARD
                .decode(
                    s.get("signature")
                        .and_then(|v| v.as_str())
                        .ok_or("signature.signature missing")?,
                )
                .map_err(|e| format!("signature not base64: {e}"))?
        };
        let bytes: [u8; 64] = raw
            .try_into()
            .map_err(|_| "signature must be 64 bytes".to_string())?;
        // `verify_signature_raw` verifies the LITERAL bytes (no re-serialization) —
        // the counterpart to `keystore.sign(agent, bytes)`, which signs the
        // literal bytes too. See the module note above.
        let ok = verify_signature_raw(key, Signature(bytes), cid.as_bytes().to_vec())
            .map_err(|e| format!("verify_signature: {e:?}"))?;
        if !ok {
            return Err(format!(
                "signature by {agent} does not verify over signing_payload_cid"
            ));
        }
    }
    if seen.len() < required {
        return Err(format!(
            "quorum unmet: {} of {required} signatures",
            seen.len()
        ));
    }
    Ok(())
}

fn validate_migrates_lineage(payload: &serde_json::Value) -> Result<(), String> {
    for field in [
        "action",
        "role",
        "from_dna_hash",
        "to_dna_hash",
        "release_cid",
        "constitution_root",
        "roster_cid",
        "signing_payload_cid",
        "signatures",
        "evidence",
        "window",
    ] {
        if payload.get(field).is_none() {
            return Err(format!("migrates-lineage missing required field: {field}"));
        }
    }
    if payload["action"] != "migrates-lineage" {
        return Err("action field must equal 'migrates-lineage'".into());
    }
    for f in ["from_dna_hash", "to_dna_hash"] {
        let h = payload[f].as_str().unwrap_or("");
        if !h.starts_with("uhC0k") {
            return Err(format!("{f} must be a DNA hash (uhC0k…)"));
        }
    }
    if payload["from_dna_hash"] == payload["to_dna_hash"] {
        return Err("from_dna_hash and to_dna_hash must differ".into());
    }
    let w = payload["window"].as_object().ok_or("window must be object")?;
    let (opens_at, revert_until) = (
        w.get("opens_at")
            .and_then(|v| v.as_str())
            .ok_or("window.opens_at missing")?,
        w.get("revert_until")
            .and_then(|v| v.as_str())
            .ok_or("window.revert_until missing")?,
    );
    // RFC3339 UTC strings compare lexicographically ONLY when both are
    // `Z`-suffixed (fixed-offset forms sort out of chronological order).
    if !opens_at.ends_with('Z') || !revert_until.ends_with('Z') {
        return Err("window.opens_at and window.revert_until must be RFC3339 UTC ('Z'-suffixed)"
            .into());
    }
    if opens_at >= revert_until {
        return Err("window.opens_at must precede window.revert_until".into());
    }
    validate_lineage_signatures(payload)
}

fn validate_sunsets_lineage(payload: &serde_json::Value) -> Result<(), String> {
    for field in [
        "action",
        "role",
        "from_dna_hash",
        "to_dna_hash",
        "migration_commitment_cid",
        "signing_payload_cid",
        "signatures",
        "evidence",
        "window",
    ] {
        if payload.get(field).is_none() {
            return Err(format!("sunsets-lineage missing required field: {field}"));
        }
    }
    if payload["action"] != "sunsets-lineage" {
        return Err("action field must equal 'sunsets-lineage'".into());
    }
    let sunsets_at = payload["window"]
        .get("sunsets_at")
        .and_then(|v| v.as_str())
        .ok_or("window.sunsets_at missing")?;
    if !sunsets_at.ends_with('Z') {
        return Err("window.sunsets_at must be RFC3339 UTC ('Z'-suffixed)".into());
    }
    validate_lineage_signatures(payload)
}

fn validate_delegates_compute(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "scope",
        "provider",
        "recipient",
        "bounds",
        "valid_from",
        "valid_until",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("delegates-compute missing required field: {field}"));
        }
    }
    if payload["action"] != "delegates-compute" {
        return Err("action field must equal 'delegates-compute'".into());
    }
    let bounds = payload
        .get("bounds")
        .and_then(|b| b.as_object())
        .ok_or_else(|| "bounds must be object".to_string())?;
    for field in [
        "epr_scope",
        "reach_ceiling",
        "rate_per_hour",
        "rotation_ttl_days",
    ] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // reach_ceiling above commons/community requires reach_elevation_acknowledged=true.
    // commons/community are the default-allowed ceilings; anything more permissive
    // (public, or higher in the protocol's reach hierarchy) is an escalation.
    let ceiling = bounds["reach_ceiling"].as_str().unwrap_or("");
    if !matches!(ceiling, "commons" | "community") {
        let acked = bounds
            .get("reach_elevation_acknowledged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !acked {
            return Err(format!(
                "reach_ceiling='{}' requires reach_elevation_acknowledged=true",
                ceiling
            ));
        }
    }
    Ok(())
}

fn validate_acknowledges_reach_change(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "acknowledger",
        "target_epr_cid",
        "new_reach",
        "signed_at",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!(
                "acknowledges-reach-change missing required field: {field}"
            ));
        }
    }
    if payload["action"] != "acknowledges-reach-change" {
        return Err("action field must equal 'acknowledges-reach-change'".into());
    }
    let valid_reach = [
        "private",
        "self",
        "intimate",
        "trusted",
        "familiar",
        "community",
        "public",
        "commons",
    ];
    let new_reach = payload["new_reach"].as_str().unwrap_or("");
    if !valid_reach.contains(&new_reach) {
        return Err(format!("new_reach '{}' not a known reach value", new_reach));
    }
    Ok(())
}

fn validate_replicates_dwelling(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "provider_dwelling_hub_id",
        "recipient_dwelling_hub_id",
        "provider_role",
        "capacity_bytes",
        "scope_filter",
        "valid_from",
        "valid_until",
        "grace_period_days",
        "rotation_ttl_days",
        "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!(
                "replicates-dwelling missing required field: {field}"
            ));
        }
    }
    if payload["action"] != "replicates-dwelling" {
        return Err("action field must equal 'replicates-dwelling'".into());
    }

    // provider_role enum
    let provider_role = payload["provider_role"].as_str().unwrap_or("");
    if provider_role != "steward_mutual" && provider_role != "collective_steward" {
        return Err(format!("provider_role '{provider_role}' not in enum"));
    }
    if provider_role == "collective_steward" {
        let via = payload
            .get("via_collective_hub_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if via.is_empty() {
            return Err("collective_steward requires non-empty via_collective_hub_id".into());
        }
    }

    // capacity_bytes positive
    let capacity = payload["capacity_bytes"].as_u64().unwrap_or(0);
    if capacity == 0 {
        return Err("capacity_bytes must be > 0".into());
    }

    // ratio_attestation: required sub-fields + sum-to-100
    let attestation = payload
        .get("ratio_attestation")
        .and_then(|v| v.as_object())
        .ok_or("ratio_attestation must be object")?;
    for f in [
        "commons_pct",
        "dwelling_pct",
        "collective_pct",
        "free_pct",
        "effective_ratio_cid",
    ] {
        if !attestation.contains_key(f) {
            return Err(format!("ratio_attestation missing field: {f}"));
        }
    }
    let commons = attestation["commons_pct"].as_u64().unwrap_or(0);
    let dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0);
    let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
    let free = attestation["free_pct"].as_u64().unwrap_or(0);
    if commons + dwelling + collective + free != 100 {
        return Err(format!(
            "ratio_attestation pct sum {} != 100",
            commons + dwelling + collective + free
        ));
    }

    // scope_filter must be object (curation policy; can be empty)
    if !payload
        .get("scope_filter")
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        return Err("scope_filter must be object".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_delegates_compute_payload() -> serde_json::Value {
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
    }

    #[test]
    fn delegates_compute_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: well_formed_delegates_compute_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "well-formed delegates-compute payload must validate"
        );
    }

    #[test]
    fn delegates_compute_missing_fields_rejected() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: serde_json::json!({"action": "delegates-compute"}).to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "incomplete payload must fail validation"
        );
    }

    #[test]
    fn delegates_compute_wrong_action_rejected() {
        let mut payload = well_formed_delegates_compute_payload();
        payload["action"] = serde_json::json!("not-delegates-compute");
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "action field must equal action discriminator"
        );
    }

    #[test]
    fn delegates_compute_unacknowledged_reach_elevation_rejected() {
        let mut payload = well_formed_delegates_compute_payload();
        payload["bounds"]["reach_ceiling"] = serde_json::json!("public");
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "reach_ceiling above commons/community requires reach_elevation_acknowledged=true"
        );
    }

    #[test]
    fn unhandled_action_rejected() {
        let input = CreateCommitmentInput {
            action: "totally-bogus-action".to_string(),
            payload_json: serde_json::json!({"action": "totally-bogus-action"}).to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn malformed_json_rejected() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: "{not valid json".to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    fn well_formed_acknowledges_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "acknowledges-reach-change",
            "acknowledger": "agent:matthew-steward",
            "target_epr_cid": "bafy-new-epr-head-cid",
            "new_reach": "community",
            "signed_at": "2026-05-29T00:00:00Z"
        })
    }

    #[test]
    fn acknowledges_reach_change_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: well_formed_acknowledges_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn acknowledges_reach_change_missing_target_epr_cid_rejected() {
        let mut payload = well_formed_acknowledges_payload();
        payload.as_object_mut().unwrap().remove("target_epr_cid");
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn acknowledges_reach_change_unknown_reach_value_rejected() {
        let mut payload = well_formed_acknowledges_payload();
        payload["new_reach"] = serde_json::json!("totally-bogus-reach");
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    fn well_formed_replicates_dwelling_payload() -> serde_json::Value {
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
    }

    #[test]
    fn replicates_dwelling_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: well_formed_replicates_dwelling_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_dwelling_unknown_role_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("totally-bogus");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_collective_steward_requires_via_collective() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("collective_steward");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_collective_steward_with_via_validates() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("collective_steward");
        payload["via_collective_hub_id"] = serde_json::json!("collective:church");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_dwelling_zero_capacity_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["capacity_bytes"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_ratio_sum_not_100_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(30); // sum becomes 110
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // replicates-commons tests (content + capacity variants)
    // =========================================================================

    fn well_formed_commons_content_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "bafyhead-lamad-spa",
            "closure_rule": "transitive-1",
            "reach": "commons",
            "bounds": { "rate_per_minute": 30, "reach_ceiling": "commons" }
        })
    }

    fn well_formed_commons_capacity_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            "commons_bytes": 50_000_000_000u64,
            "bounds": { "rate_per_minute": 30, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-x"
            }
        })
    }

    #[test]
    fn replicates_commons_content_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: well_formed_commons_content_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_commons_capacity_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: well_formed_commons_capacity_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_content_non_commons_reach_now_accepted() {
        // Stage B INVERSION: the content `reach` is now reach-general. A
        // household/community/local reach with reach_ceiling="commons" VALIDATES
        // (the integrity zome gates only reach_ceiling, so this is hash-neutral).
        for reach in ["community", "household", "local", "neighborhood"] {
            let mut payload = well_formed_commons_content_payload();
            payload["reach"] = serde_json::json!(reach);
            let input = CreateCommitmentInput {
                action: "replicates-commons".to_string(),
                payload_json: payload.to_string(),
                signed_at: "2026-06-10T00:00:00Z".to_string(),
            };
            assert!(
                validate_commitment_payload(&input).is_ok(),
                "content reach '{reach}' (with reach_ceiling=commons) must validate post-Stage-B"
            );
        }
    }

    #[test]
    fn replicates_content_empty_reach_rejected() {
        // Reach is still REQUIRED and must be non-empty (structural floor) even
        // though it is no longer pinned to "commons".
        let mut payload = well_formed_commons_content_payload();
        payload["reach"] = serde_json::json!("");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_content_action_alias_validates() {
        // The renamed `replicates-content` action validates identically to the
        // `replicates-commons` migration alias (one-window dispatch).
        let payload = {
            let mut p = well_formed_commons_content_payload();
            p["action"] = serde_json::json!("replicates-content");
            p["reach"] = serde_json::json!("household");
            p
        };
        let input = CreateCommitmentInput {
            action: "replicates-content".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "the replicates-content action (household reach) must validate"
        );
    }

    #[test]
    fn replicates_content_reach_ceiling_not_commons_still_rejected() {
        // The reach_ceiling invariant is UNCHANGED and load-bearing for
        // hash-neutrality: a non-commons reach_ceiling is still rejected.
        let mut payload = well_formed_commons_content_payload();
        payload["reach"] = serde_json::json!("household");
        payload["bounds"]["reach_ceiling"] = serde_json::json!("household");
        let input = CreateCommitmentInput {
            action: "replicates-content".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "reach_ceiling != commons must still reject (hash-neutrality invariant)"
        );
    }

    #[test]
    fn replicates_commons_content_missing_head_ref_rejected() {
        let mut payload = well_formed_commons_content_payload();
        payload.as_object_mut().unwrap().remove("head_ref");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_content_carrying_ratio_attestation_rejected() {
        // The content variant carries no ratio_attestation; a stray one is rejected.
        let mut payload = well_formed_commons_content_payload();
        payload["ratio_attestation"] = serde_json::json!({
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-x"
        });
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_zero_rate_rejected() {
        // bounds.rate_per_minute == 0 is a no-op commitment and is rejected.
        let mut payload = well_formed_commons_content_payload();
        payload["bounds"]["rate_per_minute"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_zero_bytes_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["commons_bytes"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_ratio_sum_not_100_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(30); // sum 110
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_missing_effective_ratio_cid_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["ratio_attestation"]
            .as_object_mut()
            .unwrap()
            .remove("effective_ratio_cid");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_unknown_variant_rejected() {
        let mut payload = well_formed_commons_content_payload();
        payload["variant"] = serde_json::json!("bogus");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // revokes-commitment tests
    // =========================================================================

    fn well_formed_revokes_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "revokes-commitment",
            "target_cid": "bafyhead-target-commitment",
            "reason": "pin removed",
            "signed_at": "2026-06-10T00:00:00Z"
        })
    }

    #[test]
    fn revokes_commitment_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: well_formed_revokes_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn revokes_commitment_empty_target_cid_rejected() {
        let mut payload = well_formed_revokes_payload();
        payload["target_cid"] = serde_json::json!("");
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn revokes_commitment_missing_signed_at_rejected() {
        let mut payload = well_formed_revokes_payload();
        payload.as_object_mut().unwrap().remove("signed_at");
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // ratifies-limit-gradient tests (spec §5.2 DNA walls + §5.4 loosening)
    // =========================================================================

    fn lg_payload(c_target: f64, k_max: f64, acked: bool) -> serde_json::Value {
        serde_json::json!({
            "substrate_signal": "attention", "governance_layer": "community",
            "measure": {"alpha": 2.0, "q": 0.01, "w_e": 0.6, "w_s": 0.4},
            "shape": {"C_target": c_target, "k_s": 0.5, "gamma": 1.0},
            "base_rate": 0.001, "k_max": k_max, "dignity_floor": 100.0,
            "valid_from": "2026-06-10T00:00:00Z", "valid_until": "2026-09-10T00:00:00Z",
            "loosening_acknowledged": acked,
            "ratified_by_governance_action_cid": "uhCEk-test"
        })
    }

    #[test]
    fn in_wall_config_validates() {
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.15, 0.05, false)).is_ok());
    }

    #[test]
    fn out_of_wall_c_target_rejected_at_write() {
        let err = validate_ratifies_limit_gradient(&lg_payload(0.9, 0.05, true)).unwrap_err();
        assert!(err.contains("DNA wall"), "must name the wall: {err}");
    }

    #[test]
    fn confiscatory_k_max_rejected() {
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.15, 1.0, true)).is_err());
    }

    #[test]
    fn floor_below_default_is_loosening_and_requires_acknowledgement() {
        // Review W2: ANY floor below the 100.0 core default guts sufficientarian
        // protection and must be witnessed — not only the pathological zero.
        let mut payload = lg_payload(0.15, 0.05, false);
        payload["dignity_floor"] = serde_json::json!(50.0);
        let err = validate_ratifies_limit_gradient(&payload).unwrap_err();
        assert!(err.contains("loosening_acknowledged"), "{err}");
        payload["loosening_acknowledged"] = serde_json::json!(true);
        assert!(validate_ratifies_limit_gradient(&payload).is_ok());
    }

    #[test]
    fn loosening_requires_acknowledgement() {
        let err = validate_ratifies_limit_gradient(&lg_payload(0.25, 0.05, false)).unwrap_err();
        assert!(err.contains("loosening_acknowledged"), "{err}");
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.25, 0.05, true)).is_ok());
    }

    // =========================================================================
    // CommitmentByState link author input (Slice-2b T11)
    // =========================================================================

    /// The state-link author input must survive a serde round-trip — the wire
    /// contract with the elohim-storage `call_create_commitment_state_link`
    /// caller. A dropped field would fail the zome call at runtime.
    #[test]
    fn create_commitment_state_link_input_serde_roundtrip() {
        let original = CreateCommitmentStateLinkInput {
            commitment_cid: "uhCEk-commitment-1".to_string(),
            state: "active".to_string(),
            event_hash: "uhCkk-graduating-event".to_string(),
            signed_at: "2026-06-11T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: CreateCommitmentStateLinkInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.commitment_cid, original.commitment_cid);
        assert_eq!(decoded.state, original.state);
        assert_eq!(decoded.event_hash, original.event_hash);
        assert_eq!(decoded.signed_at, original.signed_at);
    }

    // =========================================================================
    // author-lens tests (S1 — the lens-market "teeth"; plan I1)
    //
    // A Lens IS a Mishpat::Commitment with action="author-lens"; the whole lens
    // concept lives in payload_json (zero struct change, coordinator-only →
    // DNA-hash-neutral). The validator is the closed-coordinator gate: a malformed
    // lens payload must be rejected at create-time. `governs_epr` is the EPR
    // SLUG-ID scope key (plan A3), NOT the dag-cbor CID.
    // =========================================================================

    fn well_formed_author_lens_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "author-lens",
            "governs_epr": "epr:lamad-spa",
            "school": "georgist",
            "role": "lens",
            "rule": { "predicate": "land_value_uplift > rent_capture", "emits": "contention-vote" },
            "telos": { "steers_toward": "justice", "summary": "tax unimproved land value, not labor" },
            "version_parent": serde_json::Value::Null
        })
    }

    #[test]
    fn author_lens_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "author-lens".to_string(),
            payload_json: well_formed_author_lens_payload().to_string(),
            signed_at: "2026-06-27T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "well-formed author-lens payload must validate"
        );
    }

    #[test]
    fn author_lens_missing_field_rejected() {
        // Each required field is load-bearing: drop one at a time, expect reject.
        for drop_field in ["governs_epr", "school", "role", "rule", "telos"] {
            let mut payload = well_formed_author_lens_payload();
            payload.as_object_mut().unwrap().remove(drop_field);
            let input = CreateCommitmentInput {
                action: "author-lens".to_string(),
                payload_json: payload.to_string(),
                signed_at: "2026-06-27T00:00:00Z".to_string(),
            };
            assert!(
                validate_commitment_payload(&input).is_err(),
                "author-lens missing '{drop_field}' must fail validation"
            );
        }
    }

    #[test]
    fn author_lens_wrong_action_discriminator_rejected() {
        let mut payload = well_formed_author_lens_payload();
        payload["action"] = serde_json::json!("not-author-lens");
        let input = CreateCommitmentInput {
            action: "author-lens".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-27T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "author-lens action field must equal the discriminator"
        );
    }

    #[test]
    fn author_lens_empty_governs_epr_rejected() {
        // governs_epr is the scope key (slug-id); an empty key would bind to no
        // scope row (plan A3) — reject at write.
        let mut payload = well_formed_author_lens_payload();
        payload["governs_epr"] = serde_json::json!("");
        let input = CreateCommitmentInput {
            action: "author-lens".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-27T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "author-lens empty governs_epr must fail validation"
        );
    }

    #[test]
    fn author_lens_unknown_role_rejected() {
        // role enum: lens | floor | ceiling.
        let mut payload = well_formed_author_lens_payload();
        payload["role"] = serde_json::json!("dictator");
        let input = CreateCommitmentInput {
            action: "author-lens".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-27T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "author-lens role outside {{lens,floor,ceiling}} must fail validation"
        );
    }

    #[test]
    fn author_lens_floor_and_ceiling_roles_validate() {
        // floor and ceiling are valid governance roles (constitutional bounds).
        for role in ["floor", "ceiling"] {
            let mut payload = well_formed_author_lens_payload();
            payload["role"] = serde_json::json!(role);
            let input = CreateCommitmentInput {
                action: "author-lens".to_string(),
                payload_json: payload.to_string(),
                signed_at: "2026-06-27T00:00:00Z".to_string(),
            };
            assert!(
                validate_commitment_payload(&input).is_ok(),
                "author-lens role '{role}' must validate"
            );
        }
    }

    #[test]
    fn author_lens_non_object_rule_rejected() {
        // rule is the deterministic predicate (the teeth) — must be an object.
        let mut payload = well_formed_author_lens_payload();
        payload["rule"] = serde_json::json!("just a string");
        let input = CreateCommitmentInput {
            action: "author-lens".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-27T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "author-lens rule must be an object"
        );
    }

    // =========================================================================
    // binds-identity tests (Wave B — the identity-head declaration; design §2.2)
    //
    // A binds-identity IS a Mishpat::Commitment with action="binds-identity";
    // the whole declaration lives in payload_json (zero integrity change →
    // DNA-hash-neutral, the author-lens precedent). `chain_root` is a content
    // reference to the imagodei chain-root (genesis-key/CID) — the imagodei↔
    // mishpat cross-DNA link is a CID reference, never re-derived here.
    // =========================================================================

    fn well_formed_binds_identity_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "binds-identity",
            "chain_root": "uhCAk-genesis-agent-key",
            "head_key": "uhCAk-genesis-agent-key",
            "controllers": ["uhCAk-genesis-agent-key"],
            "controller_policy": { "kind": "self" }
        })
    }

    fn well_formed_binds_identity_recovery_quorum() -> serde_json::Value {
        serde_json::json!({
            "action": "binds-identity",
            "chain_root": "uhCAk-grandma-genesis-key",
            "head_key": "uhCAk-grandma-current-key",
            "controllers": ["uhCAk-friend-a", "uhCAk-friend-b", "uhCAk-friend-c"],
            "controller_policy": { "kind": "recovery-quorum", "m": 2, "n": 3 }
        })
    }

    #[test]
    fn binds_identity_well_formed_self_validates() {
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: well_formed_binds_identity_payload().to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "well-formed self-policy binds-identity must validate"
        );
    }

    #[test]
    fn binds_identity_well_formed_recovery_quorum_validates() {
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: well_formed_binds_identity_recovery_quorum().to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "well-formed recovery-quorum(2,3) binds-identity must validate"
        );
    }

    #[test]
    fn binds_identity_steward_set_validates() {
        let mut payload = well_formed_binds_identity_payload();
        payload["controller_policy"] = serde_json::json!({ "kind": "steward-set" });
        payload["controllers"] = serde_json::json!(["uhCAk-steward-a", "uhCAk-steward-b"]);
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "steward-set binds-identity must validate"
        );
    }

    #[test]
    fn binds_identity_missing_field_rejected() {
        for drop_field in ["chain_root", "head_key", "controllers", "controller_policy"] {
            let mut payload = well_formed_binds_identity_payload();
            payload.as_object_mut().unwrap().remove(drop_field);
            let input = CreateCommitmentInput {
                action: "binds-identity".to_string(),
                payload_json: payload.to_string(),
                signed_at: "2026-07-17T00:00:00Z".to_string(),
            };
            assert!(
                validate_commitment_payload(&input).is_err(),
                "binds-identity missing '{drop_field}' must fail validation"
            );
        }
    }

    #[test]
    fn binds_identity_wrong_action_discriminator_rejected() {
        let mut payload = well_formed_binds_identity_payload();
        payload["action"] = serde_json::json!("not-binds-identity");
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "binds-identity action field must equal the discriminator"
        );
    }

    #[test]
    fn binds_identity_empty_chain_root_rejected() {
        let mut payload = well_formed_binds_identity_payload();
        payload["chain_root"] = serde_json::json!("");
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "binds-identity empty chain_root must fail validation"
        );
    }

    #[test]
    fn binds_identity_empty_controllers_rejected() {
        // Ontology guard: a head cannot exist without its controller-set.
        let mut payload = well_formed_binds_identity_payload();
        payload["controllers"] = serde_json::json!([]);
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "binds-identity empty controllers must fail validation (ontology guard)"
        );
    }

    #[test]
    fn binds_identity_self_policy_head_not_in_controllers_rejected() {
        // self-policy: the head must be its own controller (structural).
        let mut payload = well_formed_binds_identity_payload();
        payload["controllers"] = serde_json::json!(["uhCAk-someone-else"]);
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "self-policy with head_key absent from controllers must fail"
        );
    }

    #[test]
    fn binds_identity_unknown_policy_kind_rejected() {
        let mut payload = well_formed_binds_identity_payload();
        payload["controller_policy"] = serde_json::json!({ "kind": "dictator" });
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "controller_policy.kind outside {{self,steward-set,recovery-quorum}} must fail"
        );
    }

    #[test]
    fn binds_identity_recovery_quorum_m_greater_than_n_rejected() {
        let mut payload = well_formed_binds_identity_recovery_quorum();
        payload["controller_policy"] =
            serde_json::json!({ "kind": "recovery-quorum", "m": 4, "n": 3 });
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "recovery-quorum with m > n must fail validation"
        );
    }

    #[test]
    fn binds_identity_recovery_quorum_missing_threshold_rejected() {
        let mut payload = well_formed_binds_identity_recovery_quorum();
        payload["controller_policy"] = serde_json::json!({ "kind": "recovery-quorum" });
        let input = CreateCommitmentInput {
            action: "binds-identity".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-07-17T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "recovery-quorum without integer m and n must fail validation"
        );
    }
}
