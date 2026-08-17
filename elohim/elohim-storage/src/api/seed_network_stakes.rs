//! Gated seed endpoint for a declared `network-stakes` manifest.
//!
//! `PUT /admin/seed/network-stakes` — the runtime half of the declared-stakes
//! bootstrap (head-plane trust-gradient program plan §3 L5, task T10;
//! minutes-quiesce fixture-trust-swarm plan §3 W2, task Q6). Accepts the
//! artifact the genesis seeder mints (`genesis/seeder/src/corpus-trust.ts`,
//! `STAKES-DECLARATION-SEAM-Q6` contract block) and stores it into the
//! `manifests` table (`manifest_kind = "network-stakes"`).
//!
//! ## Why this table, not a new one
//!
//! `manifests` is already the Category C operational projection for exactly
//! this shape — content-addressed CID, JSON payload, revision counter — and
//! is the table `services::bootstrap_manifests::seed_if_empty` uses for the
//! standing-policy / tending-policy bootstrap rows. A `network-stakes` row is
//! local-only, unsigned, NOT DHT-published, NOT gossiped — same posture as
//! those bootstrap rows (there is no write route for a *peer-authored* policy
//! manifest yet; see the seeder marker comment). Adding a dedicated table for
//! one more manifest kind would duplicate that machinery for no reason.
//!
//! ## Security boundary
//!
//! `ALLOW_SEED_NETWORK_STAKES=1` is the dev/prod honesty boundary, mirroring
//! `PUT /admin/seed/shard-manifest` and `POST /admin/seed/delegates-compute`.
//! The endpoint is deliberately NOT registered in `build_manifest()` — never
//! doorway-proxied.
//!
//! ## Contract
//!
//! ```json
//! {
//!   "schemaVersion": "1",
//!   "manifestKind": "network-stakes",
//!   "manifestCid": "stakes:genesis-lamad:sha256-<64 hex>",
//!   "stakes": {
//!     "stage": "simulacra",
//!     "scope": "genesis-lamad",
//!     "grantor": "human-adam-firstman",
//!     "environment": "preproduction"
//!   },
//!   "corpus": { … },
//!   "provenance": { … },
//!   "operatorConfig": { "ELOHIM_NETWORK_STAKES": "simulacra" }
//! }
//! ```
//!
//! `corpus` / `provenance` / `operatorConfig` are stored verbatim (audit
//! trail) but not independently validated beyond being present-or-absent;
//! `schemaVersion`, `manifestKind`, `manifestCid`, and every `stakes.*` field
//! are validated. `stakes.stage` is parsed via `NetworkStage::from_str` — an
//! EXACT lowercase match, never best-effort. Any validation failure returns
//! 400 naming the offending field; nothing is partially accepted.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use std::sync::Arc;

use crate::db::manifests::{insert_manifest, ManifestRow};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::response;
use crate::trust::stage::NetworkStage;
use crate::trust::ManifestStakesResolver;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Flag gate
// ---------------------------------------------------------------------------

/// Returns `true` when `ALLOW_SEED_NETWORK_STAKES` is `1`/`true` in the
/// environment — the same accepted spellings as the sibling
/// `ALLOW_SEED_SHARD_MANIFEST` gate, because both are stamped from the same
/// deploy-template placeholder vocabulary (`true`/`false`).
/// Exposed for direct test coverage of the flag-gate logic.
pub fn is_seed_allowed() -> bool {
    gate_value_allows(std::env::var("ALLOW_SEED_NETWORK_STAKES").ok().as_deref())
}

/// Pure half of the gate: which env-var spellings count as opt-in.
/// Kept env-free so tests never mutate process globals.
fn gate_value_allows(value: Option<&str>) -> bool {
    matches!(value, Some(v) if v == "1" || v.eq_ignore_ascii_case("true"))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// A validated (but not yet stored) stakes declaration.
struct ValidatedStakes {
    manifest_cid: String,
    stage_raw: String,
    scope: String,
}

fn str_field<'a>(v: &'a serde_json::Value, path: &str) -> Result<&'a str, StorageError> {
    v.as_str()
        .ok_or_else(|| StorageError::InvalidInput(format!("{path}: required, non-empty string")))
        .and_then(|s| {
            if s.is_empty() {
                Err(StorageError::InvalidInput(format!(
                    "{path}: required, non-empty string"
                )))
            } else {
                Ok(s)
            }
        })
}

/// Validates the seeder's stakes-declaration artifact per the contract in
/// the module doc. Never partial-accepts: the first failing field returns a
/// 400 naming it.
fn validate(body: &serde_json::Value) -> Result<ValidatedStakes, StorageError> {
    let schema_version = body.get("schemaVersion").and_then(|v| v.as_str());
    if schema_version != Some("1") {
        return Err(StorageError::InvalidInput(format!(
            "schemaVersion: must be \"1\" (got {schema_version:?})"
        )));
    }

    let manifest_kind = body.get("manifestKind").and_then(|v| v.as_str());
    if manifest_kind != Some("network-stakes") {
        return Err(StorageError::InvalidInput(format!(
            "manifestKind: must be \"network-stakes\" (got {manifest_kind:?})"
        )));
    }

    let manifest_cid = str_field(
        body.get("manifestCid").unwrap_or(&serde_json::Value::Null),
        "manifestCid",
    )?
    .to_string();

    let stakes = body
        .get("stakes")
        .ok_or_else(|| StorageError::InvalidInput("stakes: required object".to_string()))?;

    let stage_raw = str_field(
        stakes.get("stage").unwrap_or(&serde_json::Value::Null),
        "stakes.stage",
    )?
    .to_string();
    let stage: NetworkStage = stage_raw.parse().map_err(|()| {
        StorageError::InvalidInput(format!(
            "stakes.stage: unrecognized NetworkStage {stage_raw:?} — must be exactly one of \
             simulacra|bootstrap|coordinated|enforced (lowercase, no best-effort match)"
        ))
    })?;

    let scope = str_field(
        stakes.get("scope").unwrap_or(&serde_json::Value::Null),
        "stakes.scope",
    )?
    .to_string();

    // grantor: required, non-empty — validated but not retained in
    // ValidatedStakes (it travels through unchanged as part of the stored
    // payload_json; only fields the resolver reads are pulled out here).
    str_field(
        stakes.get("grantor").unwrap_or(&serde_json::Value::Null),
        "stakes.grantor",
    )?;

    let environment = str_field(
        stakes
            .get("environment")
            .unwrap_or(&serde_json::Value::Null),
        "stakes.environment",
    )?;
    if environment != "preproduction" && environment != "production" {
        return Err(StorageError::InvalidInput(format!(
            "stakes.environment: must be \"preproduction\" or \"production\" (got {environment:?})"
        )));
    }

    if stage == NetworkStage::Simulacra && environment != "preproduction" {
        return Err(StorageError::InvalidInput(
            "stakes: stage \"simulacra\" requires environment \"preproduction\" — \
             preproduction trust must never leak into an enforced-reach environment"
                .to_string(),
        ));
    }

    Ok(ValidatedStakes {
        manifest_cid,
        stage_raw,
        scope,
    })
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

/// `PUT /admin/seed/network-stakes`
///
/// Returns 403 when `ALLOW_SEED_NETWORK_STAKES != "1"`. Deliberately NOT
/// registered in `build_manifest()`.
pub async fn handle(
    req: Request<Incoming>,
    pool: &DbPool,
    resolver: Option<Arc<ManifestStakesResolver>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // --- Honesty gate: 403 unless the operator explicitly opts in. ---------
    if !is_seed_allowed() {
        return Ok(response::forbidden(&serde_json::json!({
            "error": "network-stakes seed is disabled",
            "hint": "set ALLOW_SEED_NETWORK_STAKES=1 to enable this operator/seed lever",
            "note": "this endpoint writes the manifests table directly (manifest_kind=\"network-stakes\"); local-only, unsigned, NOT DHT-published — same posture as the bootstrap standing-policy/tending-policy rows",
        })));
    }

    let body: serde_json::Value = parse_body(req).await?;

    let validated = match validate(&body) {
        Ok(v) => v,
        Err(StorageError::InvalidInput(msg)) => return Ok(response::bad_request(&msg)),
        Err(e) => return Err(e),
    };

    let mut conn = get_conn(pool)?;

    let row = ManifestRow {
        cid: validated.manifest_cid.clone(),
        manifest_kind: "network-stakes".to_string(),
        pillar: None,
        // Stored verbatim — the full artifact (stakes + corpus + provenance
        // + operatorConfig) is the audit trail; the resolver only reads
        // stakes.{scope,stage} back out via ManifestRegistry.
        payload_json: body.to_string(),
        schema_ref: None,
        // Local-only bootstrap-style declaration — never DHT-signed. Same
        // all-zero placeholder `services::bootstrap_manifests` uses for its
        // seeded rows.
        signer_pubkey: vec![0u8; 32],
        created_at: chrono::Utc::now().to_rfc3339(),
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        revision: 1,
    };

    insert_manifest(&mut conn, &row)?;

    // Best-effort immediate visibility: refresh the shared ManifestRegistry
    // cache so ManifestStakesResolver::stage_for sees this declaration
    // without a process restart. A missing resolver (e.g. DB pool absent at
    // boot) or a refresh failure does not fail the write — the row is
    // durably stored either way and picks up on the next natural reload.
    if let Some(resolver) = resolver.as_ref() {
        if let Err(e) = resolver.refresh(&mut conn) {
            tracing::warn!(
                error = %e,
                manifest_cid = %validated.manifest_cid,
                "network-stakes seed: registry refresh failed — row is stored but stage_for \
                 may lag until the next natural reload"
            );
        }
    }

    Ok(response::created(&serde_json::json!({
        "manifestCid": validated.manifest_cid,
        "scope": validated.scope,
        "stage": validated.stage_raw,
        "state": "seeded",
    })))
}

#[cfg(test)]
mod gate_tests {
    use super::gate_value_allows;

    #[test]
    fn the_gate_accepts_the_deploy_template_spellings() {
        // The deploy template stamps "true"/"false"; operators hand-set "1".
        // Same accepted vocabulary as the sibling ALLOW_SEED_SHARD_MANIFEST gate.
        assert!(gate_value_allows(Some("1")));
        assert!(gate_value_allows(Some("true")));
        assert!(gate_value_allows(Some("TRUE")));
    }

    #[test]
    fn the_gate_fails_closed_on_everything_else() {
        assert!(!gate_value_allows(None));
        assert!(!gate_value_allows(Some("")));
        assert!(!gate_value_allows(Some("false")));
        assert!(!gate_value_allows(Some("0")));
        assert!(!gate_value_allows(Some("yes")));
    }
}
