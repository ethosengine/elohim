//! Gated seed+revoke endpoint for `delegates-compute` Mishpat commitments.
//!
//! `POST /admin/seed/delegates-compute` — writes directly into `mishpat_commitments`
//! with a synthesized dev anchor.  The revoke variant `{cid, revoke:true}` calls
//! `set_revoked_at`.
//!
//! ## Why this exists
//!
//! `POST /api/v1/commitments` writes into `rea_commitments` (NULL anchor) and is
//! NOT consumed by the op-gate.  The op-gate reads `mishpat_commitments`, which
//! is normally populated by the Holochain `CommitmentCommitted` post-commit signal.
//! In Che / dev environments there is no live conductor, so this endpoint seeds
//! the table directly — the same role that `PUT /admin/seed/shard-manifest` plays
//! for distribution rows.
//!
//! ## Security boundary
//!
//! `ALLOW_SEED_DELEGATES_COMPUTE=1` is the dev/prod honesty boundary.  The flag
//! is never set in production; the 403 is the guard.  The endpoint is DELIBERATELY
//! NOT in `build_manifest()` — that function auto-promotes routes to the public
//! doorway proxy, which must never happen here (writes commitments without
//! Holochain notarisation).
//!
//! Spec: Che op-gate Slice 1, §14 (task-2-brief.md).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};

use crate::db::models::NewMishpatCommitment;
use crate::db::{self, DbPool};
use crate::error::StorageError;
use crate::services::response;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Flag gate
// ---------------------------------------------------------------------------

/// Returns `true` when `ALLOW_SEED_DELEGATES_COMPUTE=1` is set in the environment.
///
/// Exposed for direct test coverage of the flag-gate logic.
pub fn is_seed_allowed() -> bool {
    std::env::var("ALLOW_SEED_DELEGATES_COMPUTE").as_deref() == Ok("1")
}

// ---------------------------------------------------------------------------
// Input type
// ---------------------------------------------------------------------------

/// All fields required to upsert one `delegates-compute` commitment row.
///
/// Bundled into a struct to avoid clippy's `too_many_arguments` lint and to
/// make the `perform_seed` signature stable across refactors.
pub struct SeedDelegatesInput<'a> {
    pub cid: &'a str,
    pub scope: &'a str,
    pub provider: &'a str,
    pub recipient: &'a str,
    /// Pre-serialised JSON string (may contain `_provenance:"dev-seed"` key).
    pub bounds_json: &'a str,
    pub valid_from: &'a str,
    pub valid_until: &'a str,
}

// ---------------------------------------------------------------------------
// Logic layer (directly testable)
// ---------------------------------------------------------------------------

/// Upsert a `delegates-compute` commitment row into `mishpat_commitments`.
///
/// This is the thin logic layer extracted for unit-test coverage without
/// needing a full hyper `Request<Incoming>`.  The HTTP handler (`handle`)
/// checks the flag gate and calls this after parsing the request body.
///
/// ### Contract
///
/// - `input.bounds_json` is stored **verbatim** — must already be serialised
///   by the caller (carries `_provenance:"dev-seed"` from Task 1; the
///   `bounds_validator` reads it as an untyped `serde_json::Value` so the
///   extra key is silently ignored there; it doubles as an audit marker).
/// - `dht_anchor_hash` is synthesised from `(recipient, cid)` via
///   `p2p::identity_handshake::synthesise_dht_anchor_hash` so the
///   `ProjectionCommitmentFetcher` (Slice-2a T6) accepts the row
///   (it fails-closed on NULL anchors).
/// - `state` is forced to `"active"` (dev-seeded rows are pre-graduated;
///   no DHT-signal graduation step required).
pub fn perform_seed(
    conn: &mut diesel::SqliteConnection,
    input: &SeedDelegatesInput<'_>,
) -> Result<crate::db::models::MishpatCommitment, StorageError> {
    // Revoke is TERMINAL for dev-seed.  `upsert_with_anchor` is the SHARED
    // DHT-projection upsert and correctly mirrors a re-activation when real DHT
    // truth says so — but the dev seeder is advertised idempotent and its CID is
    // deterministic, so a re-run of `seed:delegates` AFTER `{revoke:true}` would
    // silently un-revoke the grant (do_update resets revoked_at=None,
    // state="active").  For a gate whose revocation story is "deny the next
    // request," that is a footgun.  Refuse to reactivate a revoked row here, at
    // the dev-seed layer, without touching the shared projection upsert.
    if let Some(existing) = db::mishpat_commitments::get_by_cid(conn, input.cid)
        .map_err(|e| StorageError::Internal(format!("get_by_cid: {e}")))?
    {
        if existing.revoked_at.is_some() {
            return Err(StorageError::InvalidInput(format!(
                "delegates-compute {} is revoked; refusing to reactivate via re-seed \
                 (revoke is terminal for dev-seed)",
                input.cid
            )));
        }
    }

    let anchor =
        crate::p2p::identity_handshake::synthesise_dht_anchor_hash(input.recipient, input.cid);

    db::mishpat_commitments::upsert_with_anchor(
        conn,
        NewMishpatCommitment {
            cid: input.cid.to_string(),
            action: "delegates-compute".to_string(),
            scope: input.scope.to_string(),
            provider: input.provider.to_string(),
            recipient: input.recipient.to_string(),
            bounds_json: input.bounds_json.to_string(),
            valid_from: input.valid_from.to_string(),
            valid_until: input.valid_until.to_string(),
            revoked_at: None,
            dht_anchor_hash: Some(anchor),
            state: "active".to_string(),
        },
    )
    .map_err(|e| StorageError::Internal(format!("upsert_with_anchor: {e}")))
}

// ---------------------------------------------------------------------------
// Field extraction helper
// ---------------------------------------------------------------------------

fn str_field(v: &serde_json::Value, key: &str) -> Result<String, StorageError> {
    v.get(key)
        .and_then(|f| f.as_str())
        .map(str::to_string)
        .ok_or_else(|| StorageError::InvalidInput(format!("missing or non-string field: {key}")))
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

/// `POST /admin/seed/delegates-compute`
///
/// Body variants:
///
/// **Seed** (insert / upsert):
/// ```json
/// {
///   "cid": "commitment:…",
///   "scope": "republish-epr",
///   "provider": "agent:…",
///   "recipient": "agent:…",
///   "bounds": { … , "_provenance": "dev-seed" },
///   "validFrom": "2026-06-01T00:00:00Z",
///   "validUntil": "2026-09-01T00:00:00Z"
/// }
/// ```
///
/// **Revoke**:
/// ```json
/// { "cid": "commitment:…", "revoke": true }
/// ```
///
/// Returns 403 when `ALLOW_SEED_DELEGATES_COMPUTE != "1"`.
/// Deliberately **NOT** registered in `build_manifest()`.
pub async fn handle(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // --- Honesty gate: 403 unless operator explicitly opts in. ----------------
    if !is_seed_allowed() {
        return Ok(response::forbidden(&serde_json::json!({
            "error": "delegates-compute dev-seed is disabled",
            "hint": "set ALLOW_SEED_DELEGATES_COMPUTE=1 to enable this operator/seed lever",
            "note": "this endpoint writes mishpat_commitments directly; only for dev/Che environments",
        })));
    }

    let body: serde_json::Value = parse_body(req).await?;
    let mut conn = get_conn(pool)?;

    // --- Revoke variant: { cid, revoke: true } --------------------------------
    if body
        .get("revoke")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let cid = str_field(&body, "cid")?;
        let now = chrono::Utc::now().to_rfc3339();
        db::mishpat_commitments::set_revoked_at(&mut conn, &cid, &now)
            .map_err(|e| StorageError::Internal(format!("set_revoked_at: {e}")))?;
        return Ok(response::ok(
            &serde_json::json!({"cid": cid, "revoked": true}),
        ));
    }

    // --- Seed variant ---------------------------------------------------------
    let cid = str_field(&body, "cid")?;
    let scope = str_field(&body, "scope")?;
    let provider = str_field(&body, "provider")?;
    let recipient = str_field(&body, "recipient")?;
    let valid_from = str_field(&body, "validFrom")?;
    let valid_until = str_field(&body, "validUntil")?;

    // Store bounds verbatim — carries `_provenance:"dev-seed"` from Task 1
    // seeder; the bounds_validator reads it as an untyped serde_json::Value
    // so the extra key is ignored at validation time (audit marker only here).
    let bounds_val = body
        .get("bounds")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let bounds_json = serde_json::to_string(&bounds_val)
        .map_err(|e| StorageError::Internal(format!("bounds serialise: {e}")))?;

    perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid: &cid,
            scope: &scope,
            provider: &provider,
            recipient: &recipient,
            bounds_json: &bounds_json,
            valid_from: &valid_from,
            valid_until: &valid_until,
        },
    )?;

    Ok(response::ok(&serde_json::json!({
        "cid": cid,
        "state": "active",
        "provenance": "dev-seed",
    })))
}
