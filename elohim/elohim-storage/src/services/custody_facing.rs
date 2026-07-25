//! Custody-observation facing — the impure loader behind the typed custody folds
//! (`elohim_facings::folds::operational_weave::{observed_custody_class_counts,
//! fresh_evidence_counts}`), plus the Prometheus projection of the v1 counts.
//!
//! The folds are pure and DB-free; THIS file is where the two independent
//! lookups happen and where honesty is won or lost. The folds cannot tell a
//! measured absence from a fabricated one — the distinction is made here.
//!
//! # The two planes
//!
//! A custody observation is the join of two *independently failing* lookups:
//!
//! | Plane | Source | Answers |
//! |-------|--------|---------|
//! | Evidence | `shard_locations.status` | *why do I believe this holder has these bytes?* |
//! | Commitment | `rea_commitments` (`action='custody-blob'`, `state='active'`) | *what did the holder promise?* |
//!
//! Evidence classification (`shard_locations.status` → [`CustodyEvidence`]):
//!
//! | status | evidence | written by |
//! |--------|----------|-----------|
//! | `self-held` | [`CustodyEvidence::DirectPossession`] | `services::self_stewardship` — only when the local blob store actually holds the bytes |
//! | `verified` | [`CustodyEvidence::VerifiedDraw`] | `db::shard_locations::update_verified` |
//! | `announced` | [`CustodyEvidence::AcceptedPush`] | push + remote ack |
//! | `peer-announced` | [`CustodyEvidence::PeerAsserted`] | `p2p::custody_announce` → `apply_peer_announced` (never overwrites a locally-witnessed row) |
//! | `lost` | [`CustodyEvidence::ObservedLost`] | `db::shard_locations::mark_lost` |
//! | anything else (`seeded`, future values) | [`CustodyEvidence::NotObserved`] | we DID look and found a row carrying no possession evidence |
//!
//! Row-absent is also `NotObserved` — but it is the CALLER's to state, not this
//! loader's: a shard/holder pair with no row produces no row here. We never
//! fabricate a holder we have no record of.
//!
//! # Custody-class mapping (what a commitment pledges)
//!
//! Custody-blob commitments **do not record a tier today** — every authoring
//! path in-tree writes `metadata_json: None` or `{"origin":"salvage", ...}`
//! (`services::salvage_commitment_author`, `reconcile::custody`). So the mapping
//! is:
//!
//! 1. **Recorded tier wins.** If `metadata_json` parses to an object carrying
//!    `custodyClass` (or `tier`) as a string, that string maps to the class:
//!    `none` / `shelved` / `stocked` / `stocked-warm` (`_`, `-` and case are
//!    normalized away). This is the ONLY way [`CustodyClass::StockedWarm`] is
//!    ever produced — a warm pantry is a pledge, never an inference.
//! 2. **No recorded tier → derive from evidence of holding.** A present active
//!    commitment plus *locally-witnessed* possession (`DirectPossession` /
//!    `VerifiedDraw` / `AcceptedPush`) = [`CustodyClass::Stocked`]. A present
//!    active commitment with no locally-witnessed possession (`PeerAsserted`,
//!    `NotObserved`, `ObservedLost`) = [`CustodyClass::Shelved`] — the promise
//!    stands, the bytes are unwitnessed. `PeerAsserted` deliberately does NOT
//!    reach `Stocked`: a received peer claim never becomes stronger by being
//!    folded (the evidence bucket keeps carrying it truthfully).
//! 3. An unrecognized tier string falls to rule 2 (and logs at debug) rather
//!    than inventing a class.
//!
//! # Binding honesty rules
//!
//! - **The two lookups fail independently; neither erases the other.** A
//!   commitment-plane ERROR (query failure — *not* an empty result) yields
//!   `observed_custody_class: None` (honest unknown) while the evidence found
//!   rides through untouched.
//! - **`Some(CustodyClass::None)` requires a COMPLETE commitment lookup that
//!   measured no active promise.** If the shard's blob key could not be resolved
//!   (no `shard_manifests` row), the commitment plane is only partially
//!   searchable for that shard — a blob-level promise could exist that we cannot
//!   see — so the class stays `None` (unknown), never `Some(None)`.
//! - **An evidence-query error excludes the affected rows entirely** with a
//!   distinct `tracing::warn` (target `custody_facing`) — "couldn't look" is
//!   never rendered as `NotObserved`. If EVERY evidence query fails, the loader
//!   returns `Err`, never `Ok(vec![])`: an empty return would silently drop
//!   every row from the fold, which is dishonest by omission. (This is
//!   deliberately NOT the warn-and-return-empty shape of the neighbouring
//!   `operational_weave_facing::load_custodian_relation`.)
//! - **A missing or unparseable timestamp is stale** (`is_fresh = false`), never
//!   fresh.
//!
//! Charter: `genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md`

use std::collections::{BTreeMap, HashMap};

use diesel::prelude::*;
use elohim_facings::folds::operational_weave::{
    observed_custody_class_counts, CustodyClass, CustodyClassCounts, CustodyEvidence,
    CustodyObservationRow,
};

use crate::db::models::ReaCommitment;
use crate::StorageError;

/// `eq_any` chunk size — stays well under SQLite's bound-variable ceiling
/// (`SQLITE_MAX_VARIABLE_NUMBER`, historically 999) so a wide shard set never
/// fails as one oversized statement.
const IN_CHUNK: usize = 400;

/// Upper bound on the shard set [`compute_and_emit_custody_counts`] folds in one
/// pass. The enumerated set is the **observed** set — distinct `shard_hash`
/// values already present in `shard_locations` for the scope, ordered by hash so
/// the truncation point is deterministic. A shard with no location row
/// contributes no fabricated observation either way (see the module doc), so
/// bounding the observed set costs no honesty, only breadth.
pub const MAX_EMIT_SHARDS: i64 = 5_000;

/// Freshness window the gauge emitter uses when a caller has no better policy:
/// 24h, comfortably wider than the ~20min restart-churn window the substrate
/// trust contract documents, so a healthy node's observations are not counted
/// stale between verification passes.
pub const DEFAULT_CUSTODY_STALENESS_SECS: i64 = 86_400;

// ---------------------------------------------------------------------------
// Evidence plane
// ---------------------------------------------------------------------------

/// Classify one `shard_locations.status` into the fold's evidence vocabulary.
/// Unknown statuses (e.g. the fixture-authored `seeded`) are `NotObserved`: we
/// looked, and the row carries no possession evidence.
pub fn classify_evidence(status: &str) -> CustodyEvidence {
    match status {
        crate::services::self_stewardship::SELF_HELD_STATUS => CustodyEvidence::DirectPossession,
        "verified" => CustodyEvidence::VerifiedDraw,
        "announced" => CustodyEvidence::AcceptedPush,
        crate::db::shard_locations::PEER_ANNOUNCED_STATUS => CustodyEvidence::PeerAsserted,
        "lost" => CustodyEvidence::ObservedLost,
        _ => CustodyEvidence::NotObserved,
    }
}

/// Is this evidence a LOCALLY-WITNESSED holding (bytes we or our own push/draw
/// path observed), as opposed to a received assertion or an absence?
fn is_witnessed_holding(evidence: CustodyEvidence) -> bool {
    matches!(
        evidence,
        CustodyEvidence::DirectPossession
            | CustodyEvidence::VerifiedDraw
            | CustodyEvidence::AcceptedPush
    )
}

/// Parse a `shard_locations` timestamp to epoch micros. Accepts RFC3339 (what
/// `update_verified` / `apply_peer_announced` stamp into `last_verified`) and
/// SQLite's `datetime('now')` form `YYYY-MM-DD HH:MM:SS` (the `first_seen`
/// default), with or without fractional seconds. `None` on absent/unparseable —
/// which the caller renders as STALE, never fresh.
fn parse_observed_micros(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_micros());
    }
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Some(naive.and_utc().timestamp_micros());
        }
    }
    None
}

/// Freshness cutoff in epoch micros. `staleness_secs <= 0` disables the window
/// (`i64::MIN` makes the compare a no-op) — mirrors the precedent in
/// `household_resilience::count_online_peers_in_households`.
fn freshness_cutoff(now_micros: i64, staleness_secs: i64) -> i64 {
    if staleness_secs > 0 {
        now_micros.saturating_sub(staleness_secs.saturating_mul(1_000_000))
    } else {
        i64::MIN
    }
}

// ---------------------------------------------------------------------------
// Commitment plane
// ---------------------------------------------------------------------------

/// The commitment-side lookup state. `Unavailable` is the honest-unknown carrier:
/// a query error on this plane must never erase the evidence plane's finding.
#[derive(Debug, Clone)]
enum CommitmentPlane {
    Complete {
        /// `shard_hash -> blob_hash`, resolved from `shard_manifests`. A shard
        /// missing here has no resolvable blob key (see rule 2 in the module doc).
        shard_to_blob: HashMap<String, String>,
        /// `(provider, classification) -> pledged tier`, where the inner
        /// `Option` is the tier RECORDED in `metadata_json` (`None` = a present
        /// commitment that pledges no explicit tier).
        pledges: HashMap<(String, String), Option<CustodyClass>>,
    },
    /// A manifest or commitment query errored — every class is unknown, all
    /// evidence preserved.
    Unavailable,
}

/// Normalize a recorded tier string onto [`CustodyClass`]. Case, `-` and `_` are
/// insignificant. Unrecognized strings return `None` so the caller falls back to
/// the evidence-derived default rather than inventing a class.
fn parse_recorded_class(raw: &str) -> Option<CustodyClass> {
    let norm: String = raw
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect();
    match norm.as_str() {
        "none" => Some(CustodyClass::None),
        "shelved" => Some(CustodyClass::Shelved),
        "stocked" => Some(CustodyClass::Stocked),
        "stockedwarm" | "warm" => Some(CustodyClass::StockedWarm),
        _ => None,
    }
}

/// Read the pledged tier out of a commitment's `metadata_json` (`custodyClass`
/// or `tier`). Returns `None` when no tier is recorded — the common case today.
fn recorded_class(commitment: &ReaCommitment) -> Option<CustodyClass> {
    let raw = commitment.metadata_json.as_deref()?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let field = value
        .get("custodyClass")
        .or_else(|| value.get("custody_class"))
        .or_else(|| value.get("tier"))?
        .as_str()?;
    let parsed = parse_recorded_class(field);
    if parsed.is_none() {
        tracing::debug!(
            target: "custody_facing",
            commitment = %commitment.id,
            tier = %field,
            "custody-blob commitment records an unrecognized tier — falling back to \
             evidence-derived class rather than inventing one"
        );
    }
    parsed
}

/// Build the commitment plane for the requested shards/holders. Either lookup
/// erroring collapses to [`CommitmentPlane::Unavailable`] — never to an empty
/// `Complete`, which would read as "no promises exist."
fn load_commitment_plane(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    shard_hashes: &[String],
    holders: &[String],
) -> CommitmentPlane {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;
    use crate::db::diesel_schema::shard_manifests::dsl as sm;

    // 1. shard -> blob, via the manifests' shard_hashes_json. The column is JSON
    //    TEXT (unindexable), so the scope's manifests are read once and inverted
    //    in memory rather than issuing one LIKE query per shard.
    let manifests: Vec<(String, String)> = match sm::shard_manifests
        .filter(sm::h_app_id.eq(h_app_id))
        .select((sm::blob_hash, sm::shard_hashes_json))
        .load::<(String, String)>(conn)
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "custody_facing",
                error = %e,
                "commitment plane: shard_manifests query failed — custody class is an \
                 honest unknown for this pass (evidence preserved)"
            );
            return CommitmentPlane::Unavailable;
        }
    };

    let wanted: std::collections::HashSet<&str> = shard_hashes.iter().map(String::as_str).collect();
    let mut shard_to_blob: HashMap<String, String> = HashMap::new();
    for (blob_hash, shard_hashes_json) in manifests {
        let Ok(hashes) = serde_json::from_str::<Vec<String>>(&shard_hashes_json) else {
            continue;
        };
        for h in hashes {
            if wanted.contains(h.as_str()) {
                shard_to_blob.insert(h, blob_hash.clone());
            }
        }
    }

    // 2. Active custody-blob commitments naming any observed holder as provider.
    let mut pledges: HashMap<(String, String), Option<CustodyClass>> = HashMap::new();
    for chunk in holders.chunks(IN_CHUNK) {
        let rows: Vec<ReaCommitment> = match rc::rea_commitments
            .filter(rc::h_app_id.eq(h_app_id))
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::state.eq("active"))
            .filter(rc::provider.eq_any(chunk))
            .load::<ReaCommitment>(conn)
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(
                    target: "custody_facing",
                    error = %e,
                    "commitment plane: rea_commitments query failed — custody class is an \
                     honest unknown for this pass (evidence preserved)"
                );
                return CommitmentPlane::Unavailable;
            }
        };
        for commitment in rows {
            let tier = recorded_class(&commitment);
            for classification in commitment.classifications() {
                pledges
                    .entry((commitment.provider.clone(), classification))
                    // An explicit tier beats a tier-less duplicate; otherwise
                    // first-write wins (deterministic given a sorted load).
                    .and_modify(|existing| {
                        if existing.is_none() {
                            *existing = tier;
                        }
                    })
                    .or_insert(tier);
            }
        }
    }

    CommitmentPlane::Complete {
        shard_to_blob,
        pledges,
    }
}

/// The evidence-derived class for a present active commitment that records no
/// tier. See rule 2 in the module doc.
fn class_from_evidence(evidence: CustodyEvidence) -> CustodyClass {
    if is_witnessed_holding(evidence) {
        CustodyClass::Stocked
    } else {
        CustodyClass::Shelved
    }
}

/// Derive `observed_custody_class` for one (shard, holder) pair. `None` is an
/// honest unknown; `Some(CustodyClass::None)` is a COMPLETE lookup that measured
/// no active promise.
fn derive_class(
    plane: &CommitmentPlane,
    shard_cid: &str,
    holder: &str,
    evidence: CustodyEvidence,
) -> Option<CustodyClass> {
    let CommitmentPlane::Complete {
        shard_to_blob,
        pledges,
    } = plane
    else {
        // Commitment lookup did not complete → unknown, evidence untouched.
        return None;
    };

    let blob = shard_to_blob.get(shard_cid);
    // A commitment may classify the shard directly or (the common shape) the
    // blob the shard belongs to.
    let hit = pledges
        .get(&(holder.to_string(), shard_cid.to_string()))
        .or_else(|| blob.and_then(|b| pledges.get(&(holder.to_string(), b.clone()))));

    match hit {
        Some(tier) => Some(tier.unwrap_or_else(|| class_from_evidence(evidence))),
        // Lookup completed AND the blob key was resolvable → a measured absence.
        None if blob.is_some() => Some(CustodyClass::None),
        // No manifest row → the blob-keyed half of the lookup was unsearchable.
        None => None,
    }
}

// ---------------------------------------------------------------------------
// The loader
// ---------------------------------------------------------------------------

/// Materialize one observer's current custody facts for `shard_hashes`, ready
/// for `observed_custody_class_counts` / `fresh_evidence_counts`.
///
/// Rows are latest-wins per `(observer, shard, holder)` — greatest parsed
/// timestamp, ties broken lexicographically on the raw timestamp then the status
/// — and the returned vector is sorted by `(shard_cid, holder_agent_cid)` so the
/// fold input is deterministic.
///
/// Honesty contract: see the module doc. In short — a commitment-plane error
/// yields unknown classes with evidence intact; an evidence-plane error excludes
/// the affected rows (warn), and a TOTAL evidence-plane failure returns `Err`
/// rather than a silently-empty `Ok`.
pub fn load_custody_observation_relation(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    observer_agent_cid: &str,
    shard_hashes: &[String],
    now_micros: i64,
    staleness_secs: i64,
) -> Result<Vec<CustodyObservationRow>, StorageError> {
    use crate::db::diesel_schema::shard_locations::dsl as sl;

    if shard_hashes.is_empty() {
        return Ok(Vec::new());
    }

    // --- Evidence plane ----------------------------------------------------
    // Latest-wins per (shard, holder). Key ordering is deterministic via BTreeMap.
    let mut latest: BTreeMap<(String, String), (i64, String, String)> = BTreeMap::new();
    let mut chunks_total = 0usize;
    let mut chunks_failed = 0usize;
    let mut last_err: Option<String> = None;

    for chunk in shard_hashes.chunks(IN_CHUNK) {
        chunks_total += 1;
        let rows: Vec<(String, String, String, String, Option<String>)> = match sl::shard_locations
            .filter(sl::h_app_id.eq(h_app_id))
            .filter(sl::shard_hash.eq_any(chunk))
            .select((
                sl::shard_hash,
                sl::peer_id,
                sl::status,
                sl::first_seen,
                sl::last_verified,
            ))
            .load::<(String, String, String, String, Option<String>)>(conn)
        {
            Ok(rows) => rows,
            Err(e) => {
                chunks_failed += 1;
                last_err = Some(e.to_string());
                tracing::warn!(
                    target: "custody_facing",
                    error = %e,
                    shards_in_chunk = chunk.len(),
                    "evidence plane: shard_locations query failed — EXCLUDING these shards \
                     from the relation (a failed look is not an observation; never rendered \
                     as NotObserved)"
                );
                continue;
            }
        };

        for (shard_hash, peer_id, status, first_seen, last_verified) in rows {
            // `last_verified` is the freshest stamp when present (verified draws
            // and peer announcements write it); `first_seen` is the floor.
            let raw_ts = last_verified.unwrap_or(first_seen);
            let micros = parse_observed_micros(&raw_ts).unwrap_or(i64::MIN);
            let key = (shard_hash, peer_id);
            let candidate = (micros, raw_ts, status);
            match latest.get(&key) {
                Some(existing) if *existing >= candidate => {}
                _ => {
                    latest.insert(key, candidate);
                }
            }
        }
    }

    if chunks_failed > 0 && chunks_failed == chunks_total {
        // Never a silently-empty Ok: the caller must be able to tell "no custody
        // anywhere" from "the evidence plane is unreadable".
        return Err(StorageError::Database(format!(
            "custody_facing: every shard_locations query failed ({chunks_failed}/{chunks_total} \
             chunks); last error: {}",
            last_err.unwrap_or_else(|| "unknown".to_string())
        )));
    }

    // --- Commitment plane --------------------------------------------------
    let observed_shards: Vec<String> = latest.keys().map(|(s, _)| s.clone()).collect();
    let mut holders: Vec<String> = latest.keys().map(|(_, h)| h.clone()).collect();
    holders.sort();
    holders.dedup();
    let plane = load_commitment_plane(conn, h_app_id, &observed_shards, &holders);

    // --- Materialize -------------------------------------------------------
    let cutoff = freshness_cutoff(now_micros, staleness_secs);
    let mut out: Vec<CustodyObservationRow> = latest
        .into_iter()
        .map(|((shard_cid, holder), (micros, observed_at, status))| {
            let evidence = classify_evidence(&status);
            // Unparseable/missing timestamps carry `i64::MIN` → never fresh, even
            // when the staleness window is disabled.
            let is_fresh = micros != i64::MIN && micros >= cutoff;
            CustodyObservationRow {
                observer_agent_cid: observer_agent_cid.to_string(),
                observed_custody_class: derive_class(&plane, &shard_cid, &holder, evidence),
                shard_cid,
                holder_agent_cid: holder,
                evidence,
                observed_at,
                is_fresh,
            }
        })
        .collect();

    out.sort_by(|a, b| {
        (a.shard_cid.as_str(), a.holder_agent_cid.as_str())
            .cmp(&(b.shard_cid.as_str(), b.holder_agent_cid.as_str()))
    });
    Ok(out)
}

// ---------------------------------------------------------------------------
// Prometheus projection (v1 surface)
// ---------------------------------------------------------------------------

/// Fold the observed shard set into [`CustodyClassCounts`] and publish it as
/// `elohim_custody_class_count{class=…}`.
///
/// **Scope choice:** the shard set is the *observed* set — distinct
/// `shard_hash` in `shard_locations` for `h_app_id`, ordered by hash, capped at
/// [`MAX_EMIT_SHARDS`]. Enumerating `shard_manifests` instead would add shards
/// with no location rows, and those contribute no observation row by contract
/// (the loader never fabricates a holder), so the counts would be identical at
/// strictly higher cost.
///
/// Returns the counts it published (useful to callers that also want the numbers).
///
// WIRE: call from a periodic task or the /api/v1/weave handler. Currently wired
// at the `/api/v1/weave` route in `api/mod.rs` (after `build_weave_view`).
pub fn compute_and_emit_custody_counts(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    observer_agent_cid: &str,
    now_micros: i64,
    staleness_secs: i64,
) -> Result<CustodyClassCounts, StorageError> {
    use crate::db::diesel_schema::shard_locations::dsl as sl;

    let shard_hashes: Vec<String> = sl::shard_locations
        .filter(sl::h_app_id.eq(h_app_id))
        .select(sl::shard_hash)
        .distinct()
        .order(sl::shard_hash.asc())
        .limit(MAX_EMIT_SHARDS)
        .load::<String>(conn)
        .map_err(|e| {
            StorageError::Database(format!("custody_facing: enumerate observed shards: {e}"))
        })?;

    let rows = load_custody_observation_relation(
        conn,
        h_app_id,
        observer_agent_cid,
        &shard_hashes,
        now_micros,
        staleness_secs,
    )?;
    let counts = observed_custody_class_counts(&rows);
    crate::metrics::set_custody_class_counts(&counts);
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::{rea_commitments, shard_locations};
    use crate::db::models::{NewReaCommitment, NewShardLocation};
    use crate::test_util::test_pool;

    const HOLDER_A: &str = "uhCAkholderaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HOLDER_B: &str = "uhCAkholderbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const OBSERVER: &str = "uhCAkobserveraaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn insert_location(conn: &mut SqliteConnection, shard: &str, holder: &str, status: &str) {
        diesel::replace_into(shard_locations::table)
            .values(&NewShardLocation {
                shard_hash: shard,
                peer_id: holder,
                h_app_id: "lamad",
                status,
            })
            .execute(conn)
            .expect("insert shard_location");
    }

    /// Stamp an explicit timestamp pair on an existing location row.
    fn set_timestamps(
        conn: &mut SqliteConnection,
        shard: &str,
        holder: &str,
        first_seen: &str,
        last_verified: Option<&str>,
    ) {
        diesel::update(
            shard_locations::table
                .filter(shard_locations::shard_hash.eq(shard))
                .filter(shard_locations::peer_id.eq(holder)),
        )
        .set((
            shard_locations::first_seen.eq(first_seen),
            shard_locations::last_verified.eq(last_verified),
        ))
        .execute(conn)
        .expect("stamp timestamps");
    }

    fn insert_manifest(conn: &mut SqliteConnection, content_id: &str, blob: &str, shards: &[&str]) {
        use crate::db::diesel_schema::shard_manifests;
        diesel::sql_query(format!(
            "INSERT INTO shard_manifests (content_id, h_app_id, blob_hash, blob_cid, encoding, \
             data_shard_count, parity_shard_count, shard_hashes_json, total_size_bytes, \
             shard_size_bytes, mime_type, reach, created_at) VALUES ('{content_id}', 'lamad', \
             '{blob}', NULL, 'rs', {}, 0, '{}', 100, 10, 'application/octet-stream', 'public', \
             '2026-07-25T00:00:00Z')",
            shards.len(),
            serde_json::to_string(shards).unwrap()
        ))
        .execute(conn)
        .expect("insert shard_manifest");
        let _ = shard_manifests::table;
    }

    fn insert_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        classified: &str,
        state: &str,
        metadata_json: Option<&str>,
    ) {
        let classified_json = serde_json::to_string(&vec![classified]).unwrap();
        diesel::insert_into(rea_commitments::table)
            .values(&NewReaCommitment {
                id,
                h_app_id: "lamad",
                action: "custody-blob",
                provider,
                receiver: "uhCAkreceiver",
                resource_conforms_to: None,
                resource_classified_as: Some(&classified_json),
                resource_quantity_value: Some(1024.0),
                resource_quantity_unit: Some("B"),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_beginning: None,
                has_end: None,
                due: None,
                clause_of: None,
                in_scope_of: None,
                medium_of_exchange_id: None,
                state,
                finished: 0,
                note: None,
                metadata_json,
                dht_anchor_hash: None,
            })
            .execute(conn)
            .expect("insert commitment");
    }

    // -- evidence classification -------------------------------------------

    #[test]
    fn classify_evidence_maps_every_known_status() {
        assert_eq!(
            classify_evidence("self-held"),
            CustodyEvidence::DirectPossession
        );
        assert_eq!(classify_evidence("verified"), CustodyEvidence::VerifiedDraw);
        assert_eq!(
            classify_evidence("announced"),
            CustodyEvidence::AcceptedPush
        );
        assert_eq!(
            classify_evidence("peer-announced"),
            CustodyEvidence::PeerAsserted
        );
        assert_eq!(classify_evidence("lost"), CustodyEvidence::ObservedLost);
        // We DID look; the row simply carries no possession evidence.
        assert_eq!(classify_evidence("seeded"), CustodyEvidence::NotObserved);
        assert_eq!(
            classify_evidence("some-future-status"),
            CustodyEvidence::NotObserved
        );
    }

    #[test]
    fn loader_classifies_evidence_from_status_end_to_end() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for (i, status) in [
            "self-held",
            "verified",
            "announced",
            "peer-announced",
            "lost",
            "seeded",
        ]
        .iter()
        .enumerate()
        {
            insert_location(&mut conn, &format!("shard-{i}"), HOLDER_A, status);
        }
        let shards: Vec<String> = (0..6).map(|i| format!("shard-{i}")).collect();
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, 0, 0).unwrap();
        let evidence: Vec<CustodyEvidence> = rows.iter().map(|r| r.evidence).collect();
        assert_eq!(
            evidence,
            vec![
                CustodyEvidence::DirectPossession,
                CustodyEvidence::VerifiedDraw,
                CustodyEvidence::AcceptedPush,
                CustodyEvidence::PeerAsserted,
                CustodyEvidence::ObservedLost,
                CustodyEvidence::NotObserved,
            ],
            "rows sort by shard_cid; shard-0..shard-5 map to the status order"
        );
        assert!(rows.iter().all(|r| r.observer_agent_cid == OBSERVER));
    }

    // -- the Some(None) invariant ------------------------------------------

    #[test]
    fn some_none_only_when_lookup_completed_and_measured_no_promise() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // shard-a IS in a manifest → the blob key is resolvable → a completed
        // lookup that finds no active commitment is a MEASURED absence.
        insert_location(&mut conn, "shard-a", HOLDER_A, "verified");
        insert_manifest(&mut conn, "content-1", "sha256-blob1", &["shard-a"]);
        // shard-b is in NO manifest → the blob-keyed half is unsearchable →
        // honest unknown, never Some(None).
        insert_location(&mut conn, "shard-b", HOLDER_A, "verified");

        let shards = vec!["shard-a".to_string(), "shard-b".to_string()];
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, 0, 0).unwrap();

        assert_eq!(
            rows[0].observed_custody_class,
            Some(CustodyClass::None),
            "manifest-resolvable shard with no active commitment = measured absence"
        );
        assert_eq!(
            rows[1].observed_custody_class, None,
            "unresolvable blob key = honest unknown, NOT a measured absence"
        );
    }

    #[test]
    fn inactive_commitment_is_a_measured_absence_not_a_promise() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_location(&mut conn, "shard-a", HOLDER_A, "self-held");
        insert_manifest(&mut conn, "content-1", "sha256-blob1", &["shard-a"]);
        insert_commitment(
            &mut conn,
            "c-cancelled",
            HOLDER_A,
            "sha256-blob1",
            "cancelled",
            None,
        );

        let rows = load_custody_observation_relation(
            &mut conn,
            "lamad",
            OBSERVER,
            &["shard-a".to_string()],
            0,
            0,
        )
        .unwrap();
        assert_eq!(rows[0].observed_custody_class, Some(CustodyClass::None));
    }

    // -- custody-class mapping ---------------------------------------------

    #[test]
    fn active_commitment_with_witnessed_bytes_is_stocked_otherwise_shelved() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(
            &mut conn,
            "content-1",
            "sha256-blob1",
            &["shard-a", "shard-b"],
        );
        insert_location(&mut conn, "shard-a", HOLDER_A, "self-held"); // witnessed
        insert_location(&mut conn, "shard-b", HOLDER_B, "peer-announced"); // asserted only
        insert_commitment(&mut conn, "c-a", HOLDER_A, "sha256-blob1", "active", None);
        insert_commitment(&mut conn, "c-b", HOLDER_B, "sha256-blob1", "active", None);

        let shards = vec!["shard-a".to_string(), "shard-b".to_string()];
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, 0, 0).unwrap();
        assert_eq!(rows[0].observed_custody_class, Some(CustodyClass::Stocked));
        assert_eq!(
            rows[1].observed_custody_class,
            Some(CustodyClass::Shelved),
            "a peer assertion never promotes to Stocked"
        );
    }

    #[test]
    fn recorded_tier_wins_and_is_the_only_path_to_stocked_warm() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, "content-1", "sha256-blob1", &["shard-a"]);
        insert_location(&mut conn, "shard-a", HOLDER_A, "peer-announced");
        insert_commitment(
            &mut conn,
            "c-warm",
            HOLDER_A,
            "sha256-blob1",
            "active",
            Some(r#"{"origin":"salvage","custodyClass":"stocked-warm"}"#),
        );

        let rows = load_custody_observation_relation(
            &mut conn,
            "lamad",
            OBSERVER,
            &["shard-a".to_string()],
            0,
            0,
        )
        .unwrap();
        assert_eq!(
            rows[0].observed_custody_class,
            Some(CustodyClass::StockedWarm),
            "the pledged tier wins over the evidence-derived default"
        );
        assert_eq!(rows[0].evidence, CustodyEvidence::PeerAsserted);
    }

    #[test]
    fn unrecognized_tier_falls_back_to_evidence_default() {
        assert_eq!(
            parse_recorded_class("Stocked_Warm"),
            Some(CustodyClass::StockedWarm)
        );
        assert_eq!(parse_recorded_class("SHELVED"), Some(CustodyClass::Shelved));
        assert_eq!(parse_recorded_class("frozen"), None);
        assert_eq!(
            class_from_evidence(CustodyEvidence::VerifiedDraw),
            CustodyClass::Stocked
        );
        assert_eq!(
            class_from_evidence(CustodyEvidence::NotObserved),
            CustodyClass::Shelved
        );
    }

    // -- independent failure of the two planes ------------------------------

    #[test]
    fn commitment_query_error_yields_unknown_but_preserves_evidence() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_location(&mut conn, "shard-a", HOLDER_A, "self-held");
        insert_manifest(&mut conn, "content-1", "sha256-blob1", &["shard-a"]);
        // Make the commitment plane unreadable while the evidence plane stays fine.
        diesel::sql_query("DROP TABLE rea_commitments")
            .execute(&mut conn)
            .unwrap();

        let rows = load_custody_observation_relation(
            &mut conn,
            "lamad",
            OBSERVER,
            &["shard-a".to_string()],
            0,
            0,
        )
        .expect("a commitment-plane failure must NOT fail the whole load");
        assert_eq!(rows.len(), 1, "the evidence row survives");
        assert_eq!(
            rows[0].observed_custody_class, None,
            "commitment lookup errored → honest unknown, never Some(None)"
        );
        assert_eq!(
            rows[0].evidence,
            CustodyEvidence::DirectPossession,
            "the evidence found must not be erased by the other plane's failure"
        );
    }

    #[test]
    fn manifest_query_error_yields_unknown_but_preserves_evidence() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_location(&mut conn, "shard-a", HOLDER_A, "verified");
        diesel::sql_query("DROP TABLE shard_manifests")
            .execute(&mut conn)
            .unwrap();

        let rows = load_custody_observation_relation(
            &mut conn,
            "lamad",
            OBSERVER,
            &["shard-a".to_string()],
            0,
            0,
        )
        .expect("a commitment-plane failure must NOT fail the whole load");
        assert_eq!(rows[0].observed_custody_class, None);
        assert_eq!(rows[0].evidence, CustodyEvidence::VerifiedDraw);
    }

    #[test]
    fn evidence_query_error_errors_rather_than_returning_empty() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        diesel::sql_query("DROP TABLE shard_locations")
            .execute(&mut conn)
            .unwrap();

        let result = load_custody_observation_relation(
            &mut conn,
            "lamad",
            OBSERVER,
            &["shard-a".to_string()],
            0,
            0,
        );
        assert!(
            result.is_err(),
            "a total evidence-plane failure must be an Err, never a silently-empty Ok \
             (dishonest by omission)"
        );
    }

    // -- freshness ----------------------------------------------------------

    #[test]
    fn missing_or_unparseable_timestamp_is_never_fresh() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_location(&mut conn, "shard-a", HOLDER_A, "verified");
        set_timestamps(&mut conn, "shard-a", HOLDER_A, "", None);
        insert_location(&mut conn, "shard-b", HOLDER_A, "verified");
        set_timestamps(&mut conn, "shard-b", HOLDER_A, "not-a-timestamp", None);

        let shards = vec!["shard-a".to_string(), "shard-b".to_string()];
        // staleness_secs = 0 DISABLES the window — even so, an unreadable stamp
        // must not read as fresh.
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, 0, 0).unwrap();
        assert!(rows.iter().all(|r| !r.is_fresh));
    }

    #[test]
    fn freshness_window_uses_the_injected_clock_and_both_stamp_formats() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-25T12:00:00Z")
            .unwrap()
            .timestamp_micros();

        insert_location(&mut conn, "shard-fresh", HOLDER_A, "verified");
        set_timestamps(
            &mut conn,
            "shard-fresh",
            HOLDER_A,
            "2020-01-01 00:00:00",
            Some("2026-07-25T11:59:00Z"),
        );
        // SQLite `datetime('now')` form on first_seen, no last_verified.
        insert_location(&mut conn, "shard-sqlite", HOLDER_A, "announced");
        set_timestamps(
            &mut conn,
            "shard-sqlite",
            HOLDER_A,
            "2026-07-25 11:58:00",
            None,
        );
        insert_location(&mut conn, "shard-stale", HOLDER_A, "verified");
        set_timestamps(
            &mut conn,
            "shard-stale",
            HOLDER_A,
            "2020-01-01 00:00:00",
            Some("2026-07-24T00:00:00Z"),
        );

        let shards = vec![
            "shard-fresh".to_string(),
            "shard-sqlite".to_string(),
            "shard-stale".to_string(),
        ];
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, now, 3600)
                .unwrap();
        let by_shard: HashMap<&str, bool> = rows
            .iter()
            .map(|r| (r.shard_cid.as_str(), r.is_fresh))
            .collect();
        assert_eq!(by_shard["shard-fresh"], true);
        assert_eq!(
            by_shard["shard-sqlite"], true,
            "the SQLite datetime('now') first_seen form must parse"
        );
        assert_eq!(by_shard["shard-stale"], false);
    }

    // -- dedup determinism --------------------------------------------------

    #[test]
    fn latest_wins_dedup_is_deterministic_per_observer_shard_holder() {
        // The (shard_hash, peer_id) PK means the DB cannot hold two rows for one
        // pair, so the dedup rule is exercised directly on the comparison key.
        let older = (
            1_000i64,
            "2026-07-01T00:00:00Z".to_string(),
            "announced".to_string(),
        );
        let newer = (
            2_000i64,
            "2026-07-02T00:00:00Z".to_string(),
            "verified".to_string(),
        );
        assert!(newer > older, "greatest timestamp wins");

        // Tie on timestamp → lexicographic tie-break on the raw stamp, then status.
        let tie_a = (
            2_000i64,
            "2026-07-02T00:00:00Z".to_string(),
            "announced".to_string(),
        );
        let tie_b = (
            2_000i64,
            "2026-07-02T00:00:00Z".to_string(),
            "verified".to_string(),
        );
        assert!(tie_b > tie_a, "deterministic lexicographic tie-break");

        // Unparseable stamps sort below every parseable one (i64::MIN floor).
        let unparseable = (i64::MIN, "garbage".to_string(), "verified".to_string());
        assert!(older > unparseable);
    }

    #[test]
    fn output_order_is_deterministic_by_shard_then_holder() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_location(&mut conn, "shard-b", HOLDER_B, "verified");
        insert_location(&mut conn, "shard-a", HOLDER_B, "verified");
        insert_location(&mut conn, "shard-a", HOLDER_A, "verified");

        let shards = vec!["shard-a".to_string(), "shard-b".to_string()];
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &shards, 0, 0).unwrap();
        let keys: Vec<(&str, &str)> = rows
            .iter()
            .map(|r| (r.shard_cid.as_str(), r.holder_agent_cid.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("shard-a", HOLDER_A),
                ("shard-a", HOLDER_B),
                ("shard-b", HOLDER_B),
            ]
        );
    }

    #[test]
    fn empty_shard_set_is_an_empty_relation_not_an_error() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let rows =
            load_custody_observation_relation(&mut conn, "lamad", OBSERVER, &[], 0, 0).unwrap();
        assert!(rows.is_empty());
    }

    // -- gauge emission -----------------------------------------------------

    #[test]
    fn compute_and_emit_publishes_the_fold_counts() {
        crate::metrics::register_all(); // idempotent (Once-guarded)
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-25T12:00:00Z")
            .unwrap()
            .timestamp_micros();

        insert_manifest(
            &mut conn,
            "content-1",
            "sha256-blob1",
            &["shard-a", "shard-b"],
        );
        insert_location(&mut conn, "shard-a", HOLDER_A, "self-held");
        set_timestamps(&mut conn, "shard-a", HOLDER_A, "2026-07-25 11:00:00", None);
        insert_location(&mut conn, "shard-b", HOLDER_B, "lost");
        set_timestamps(&mut conn, "shard-b", HOLDER_B, "2026-07-25 11:00:00", None);
        insert_commitment(&mut conn, "c-a", HOLDER_A, "sha256-blob1", "active", None);

        let counts =
            compute_and_emit_custody_counts(&mut conn, "lamad", OBSERVER, now, 86_400).unwrap();
        assert_eq!(counts.stocked, 1, "witnessed bytes + active promise");
        assert_eq!(
            counts.observed_lost, 1,
            "fresh negative evidence stays visible"
        );

        let text = crate::metrics::gather_text();
        assert!(
            text.contains("elohim_custody_class_count"),
            "custody class gauge missing:\n{text}"
        );
        assert!(text.contains("class=\"stocked\""), "{text}");
        assert!(
            text.contains("class=\"observed_lost\""),
            "every bucket series must materialise, including the zeros:\n{text}"
        );
    }
}
