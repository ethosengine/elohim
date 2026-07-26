//! Shard location CRUD operations using Diesel.
//!
//! Shard locations track which peers hold which shards, with verification
//! timestamps. This is per-peer local state (Category C), rebuilt from
//! shard protocol ack events — not DHT-notarized.

use diesel::prelude::*;

use super::diesel_schema::shard_locations;
use super::models::{NewShardLocation, ShardLocationRow};
use crate::StorageError;

/// `shard_locations.status` for a row projected from a PEER's custody
/// announcement (gossip hot path or view-federation catch-up). Distinct from the
/// locally-witnessed vocabulary (`self-held`, `announced`, `verified`, `seeded`,
/// `lost`) SO a self-asserted peer claim is never mistaken for a witnessed fact.
/// A resilience fold may count these (the card's purpose IS the network
/// footprint) but the status rides through so a future fold can weight by
/// evidence class. Column is free-form TEXT; nothing filters on it except the
/// `ne("lost")` liveness guard and the (filter-free) resilience join, so a new
/// value flows through without shadowing an existing read.
pub const PEER_ANNOUNCED_STATUS: &str = "peer-announced";

/// The set of statuses that count as LOCALLY-WITNESSED — stronger evidence than
/// any peer announcement. A `peer-announced` apply NEVER overwrites a row in one
/// of these states (a self-held / push-acked / verified / seeded record, or a
/// witnessed `lost`). Only absence, or another `peer-announced` row, is fillable.
fn is_locally_witnessed(status: &str) -> bool {
    status != PEER_ANNOUNCED_STATUS
}

/// Outcome of one [`apply_peer_announced`] attempt. Maps to the
/// `elohim_custody_announce_total{direction}` metric at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerAnnounceOutcome {
    /// No prior row for `(shard_hash, holder)` — a `peer-announced` row was
    /// inserted (absence filled).
    Inserted,
    /// An existing `peer-announced` row was strictly older — refreshed to the
    /// newer announcement timestamp.
    Refreshed,
    /// A locally-witnessed row (`self-held`/`announced`/`verified`/`seeded`/`lost`)
    /// exists — left untouched (a peer claim is weaker evidence). Metric:
    /// `dropped_weaker`.
    SkippedStronger,
    /// An existing `peer-announced` row is as-fresh-or-fresher — skipped (monotone
    /// guard; never regress, never churn on a replay). Metric: `dropped_weaker`.
    SkippedStale,
    /// The holder is THIS node's own `agent_cid` — a peer telling us about
    /// ourselves is weaker than our own records. Dropped. Metric: `dropped_self`.
    DroppedSelf,
    /// The holder is not a well-formed `agent_cid` (`uhCAk…`) — refused, so a
    /// cross-namespace value can never land in the join-key column. Metric:
    /// `dropped_weaker`.
    NotAgentCid,
}

/// Render an epoch-millis announcement time as the RFC3339 string the
/// `last_verified` column carries (consistent with [`update_verified`]).
fn announced_ms_to_rfc3339(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

/// Parse a stored `last_verified` RFC3339 back to epoch millis for the monotone
/// freshness compare. `None` when absent/malformed (treated as refreshable).
fn rfc3339_to_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}

/// Project a PEER's custody announcement into `shard_locations` as a
/// `peer-announced` row, honoring the never-overwrite-local + monotone-newer
/// contract. This is the SINGLE projection entry point shared by the gossip
/// receive arm and the view-federation catch-up arm.
///
/// Rules (in order):
/// 1. `holder` not an `agent_cid` → [`PeerAnnounceOutcome::NotAgentCid`] (refused).
/// 2. `local_agent_cid == Some(holder)` → [`PeerAnnounceOutcome::DroppedSelf`]
///    (a peer telling us about ourselves — weaker than our own records).
/// 3. No existing row → INSERT `peer-announced` ([`PeerAnnounceOutcome::Inserted`]).
/// 4. Existing LOCALLY-WITNESSED row → [`PeerAnnounceOutcome::SkippedStronger`]
///    (never overwrite a self-held / push-acked / verified / seeded / lost row).
/// 5. Existing `peer-announced` row, strictly older → refresh
///    ([`PeerAnnounceOutcome::Refreshed`]); else [`PeerAnnounceOutcome::SkippedStale`].
///
/// Read-and-decide runs inside a single transaction on one pooled connection, so
/// a concurrent writer cannot interleave between the freshness check and the
/// write (same race-tolerance contract as `peer_statuses::upsert_if_newer`).
pub fn apply_peer_announced(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    holder_agent_cid: &str,
    h_app_id: &str,
    announced_at_ms: i64,
    local_agent_cid: Option<&str>,
) -> Result<PeerAnnounceOutcome, StorageError> {
    // Normalize at the wire boundary: a producer (or, until Stage-2 signature
    // verification, any gossiping peer) may send an `agent:`-prefixed cid.
    // Written raw, a prefixed peer_id never matches humans.agent_pub_key (bare
    // form) — the join-emptying identity-namespace class — and slips past the
    // bare-form self-drop compare below. Resolve to bare form or refuse.
    let holder_agent_cid =
        match crate::identity_namespace::resolve_agent_cid_write(&[Some(holder_agent_cid)]) {
            Some(bare) => bare,
            None => return Ok(PeerAnnounceOutcome::NotAgentCid),
        };
    let holder_agent_cid = holder_agent_cid.as_str();
    if local_agent_cid == Some(holder_agent_cid) {
        return Ok(PeerAnnounceOutcome::DroppedSelf);
    }

    let announced_rfc3339 = announced_ms_to_rfc3339(announced_at_ms);

    conn.transaction(|conn| {
        let existing: Option<(String, Option<String>)> = shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(holder_agent_cid))
            .select((shard_locations::status, shard_locations::last_verified))
            .first::<(String, Option<String>)>(conn)
            .optional()?;

        match existing {
            None => {
                diesel::insert_into(shard_locations::table)
                    .values((
                        shard_locations::shard_hash.eq(shard_hash),
                        shard_locations::peer_id.eq(holder_agent_cid),
                        shard_locations::h_app_id.eq(h_app_id),
                        shard_locations::status.eq(PEER_ANNOUNCED_STATUS),
                        shard_locations::last_verified.eq(&announced_rfc3339),
                    ))
                    .execute(conn)?;
                Ok(PeerAnnounceOutcome::Inserted)
            }
            Some((status, _)) if is_locally_witnessed(&status) => {
                Ok(PeerAnnounceOutcome::SkippedStronger)
            }
            Some((_, last_verified)) => {
                // Existing peer-announced row: refresh only if strictly newer.
                let existing_ms = last_verified.as_deref().and_then(rfc3339_to_ms);
                let is_newer = match existing_ms {
                    Some(prev) => announced_at_ms > prev,
                    None => true,
                };
                if is_newer {
                    diesel::update(
                        shard_locations::table
                            .filter(shard_locations::shard_hash.eq(shard_hash))
                            .filter(shard_locations::peer_id.eq(holder_agent_cid)),
                    )
                    .set((
                        shard_locations::status.eq(PEER_ANNOUNCED_STATUS),
                        shard_locations::last_verified.eq(&announced_rfc3339),
                    ))
                    .execute(conn)?;
                    Ok(PeerAnnounceOutcome::Refreshed)
                } else {
                    Ok(PeerAnnounceOutcome::SkippedStale)
                }
            }
        }
    })
}

/// Tally of applying a whole [`custody inventory`](crate::p2p::custody_announce::CustodyInventoryPayload)
/// page (the view-federation catch-up arm). Each field counts one
/// [`PeerAnnounceOutcome`] class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryApplyStats {
    pub inserted: usize,
    pub refreshed: usize,
    pub dropped_weaker: usize,
    pub dropped_self: usize,
}

impl InventoryApplyStats {
    /// Rows that landed a write (absence filled or refreshed).
    pub fn applied(&self) -> usize {
        self.inserted + self.refreshed
    }
}

/// Apply every custody claim in a catch-up inventory page via
/// [`apply_peer_announced`], returning per-class counts. A single decode/DB error
/// on one entry is logged and skipped (best-effort projection — one poisoned row
/// never fails the whole page).
pub fn apply_custody_inventory(
    conn: &mut SqliteConnection,
    entries: &[crate::p2p::custody_announce::CustodyAnnouncement],
    local_agent_cid: Option<&str>,
) -> InventoryApplyStats {
    let mut stats = InventoryApplyStats::default();
    for ann in entries {
        match apply_peer_announced(
            conn,
            &ann.shard_hash,
            &ann.holder_agent_cid,
            &ann.h_app_id,
            ann.announced_at_ms,
            local_agent_cid,
        ) {
            Ok(PeerAnnounceOutcome::Inserted) => stats.inserted += 1,
            Ok(PeerAnnounceOutcome::Refreshed) => stats.refreshed += 1,
            Ok(PeerAnnounceOutcome::DroppedSelf) => stats.dropped_self += 1,
            Ok(
                PeerAnnounceOutcome::SkippedStronger
                | PeerAnnounceOutcome::SkippedStale
                | PeerAnnounceOutcome::NotAgentCid,
            ) => stats.dropped_weaker += 1,
            Err(e) => {
                tracing::warn!(
                    shard_hash = %ann.shard_hash,
                    holder = %ann.holder_agent_cid,
                    error = %e,
                    "custody catch-up: apply of one inventory entry failed; skipping"
                );
            }
        }
    }
    stats
}

pub fn upsert_location(
    conn: &mut SqliteConnection,
    location: &NewShardLocation,
) -> Result<(), StorageError> {
    // Try insert first — if row exists, preserve first_seen and update status only
    let inserted = diesel::insert_or_ignore_into(shard_locations::table)
        .values(location)
        .execute(conn)?;

    if inserted == 0 {
        // Row exists — just update status
        diesel::update(
            shard_locations::table
                .filter(shard_locations::shard_hash.eq(location.shard_hash))
                .filter(shard_locations::peer_id.eq(location.peer_id)),
        )
        .set(shard_locations::status.eq(location.status))
        .execute(conn)?;
    }

    Ok(())
}

pub fn get_locations_for_shard(
    conn: &mut SqliteConnection,
    shard_hash: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::shard_hash.eq(shard_hash))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::peer_id.eq(peer_id))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_content(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    use super::diesel_schema::shard_manifests;

    let manifest = shard_manifests::table
        .filter(shard_manifests::content_id.eq(content_id))
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .first::<super::models::ShardManifestRow>(conn)
        .optional()?;

    let Some(manifest) = manifest else {
        return Ok(vec![]);
    };

    let shard_hashes: Vec<String> =
        serde_json::from_str(&manifest.shard_hashes_json).unwrap_or_default();

    if shard_hashes.is_empty() {
        return Ok(vec![]);
    }

    shard_locations::table
        .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
        .load(conn)
        .map_err(StorageError::from)
}

/// Delete shard-location rows for hashes that NO shard manifest names any more.
///
/// # Why this exists (blob-rotation orphans)
///
/// `shard_locations` is keyed by `shard_hash`; every per-content reader
/// ([`get_locations_for_content`], `household_resilience::load_holder_relation`)
/// reaches it THROUGH the stored `shard_manifests.shard_hashes_json`. When a
/// content's blob rotates (an SSR bundle rebuild changes `content.blob_hash`) and
/// the manifest is re-stamped from the new bytes, the previous manifest's shard
/// hashes stop being named by anything — the rows keyed under them become
/// unreachable through every manifest-keyed read.
///
/// They are not, however, harmless: `operational_weave_facing::load_all_holder_relation`
/// scans `shard_locations` with NO manifest key, so an orphan keeps inflating the
/// region-occupancy fold with a holder of bytes nobody references. And because a
/// bundle rotates on every deploy, orphans accumulate without bound.
///
/// # Safety (why this cannot delete a live join)
///
/// `candidates` are hashes we believe we just orphaned, but a shard hash is
/// CONTENT-ADDRESSED: byte-identical content elsewhere legitimately names the same
/// hash (with `encoding = "none"` the shard hash IS the blob hash, so two content
/// rows over one blob share it outright). We therefore delete a candidate only when
/// it appears in NO manifest at all — and the reference scan is deliberately
/// **cross-app** (no `h_app_id` filter on `shard_manifests`), so a manifest in
/// another app scope still protects the hash. The DELETE itself stays scoped to
/// `h_app_id`. Conservative in both directions: a hash referenced anywhere is never
/// pruned.
///
/// Returns the number of rows deleted.
pub fn prune_orphaned_locations(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    candidates: &std::collections::HashSet<String>,
) -> Result<usize, StorageError> {
    use super::diesel_schema::shard_manifests;

    if candidates.is_empty() {
        return Ok(0);
    }

    // Every shard hash named by ANY manifest, across ALL app scopes.
    let all_json: Vec<String> = shard_manifests::table
        .select(shard_manifests::shard_hashes_json)
        .load(conn)
        .map_err(StorageError::from)?;
    let mut referenced: std::collections::HashSet<String> = std::collections::HashSet::new();
    for json in &all_json {
        if let Ok(hashes) = serde_json::from_str::<Vec<String>>(json) {
            referenced.extend(hashes);
        }
    }

    let doomed: Vec<String> = candidates
        .iter()
        .filter(|h| !referenced.contains(*h))
        .cloned()
        .collect();
    if doomed.is_empty() {
        return Ok(0);
    }

    let deleted = diesel::delete(
        shard_locations::table
            .filter(shard_locations::h_app_id.eq(h_app_id))
            .filter(shard_locations::shard_hash.eq_any(&doomed)),
    )
    .execute(conn)
    .map_err(StorageError::from)?;

    Ok(deleted)
}

pub fn mark_lost(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set(shard_locations::status.eq("lost"))
    .execute(conn)?;
    Ok(())
}

pub fn update_verified(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set((
        shard_locations::status.eq("verified"),
        shard_locations::last_verified.eq(&now),
    ))
    .execute(conn)?;
    Ok(())
}

/// REKEY CASCADE (membership-truth identity supersede): re-attribute every
/// shard-holder row from a stale `old_peer_id` (an `agent_cid`) to `new_peer_id`
/// within an app scope. Called INSIDE the supersede transaction
/// (`services::membership_identity_reconcile`) alongside the human-row supersede,
/// so the resilience holder join (`shard_locations.peer_id ==
/// humans.agent_pub_key`, both `agent_cid`) stays aligned after the human's key
/// moves — healing the human alone would otherwise strand its shard rows under
/// the dead key.
///
/// `peer_id` is part of the `(shard_hash, peer_id)` primary key, so a blind
/// `UPDATE peer_id = new` can violate the UNIQUE PK when the SAME shard is already
/// recorded under `new_peer_id` (the peer re-distributed under its live key while
/// the fossil row lingered). We therefore move only non-colliding rows and DROP
/// the colliding remainder — the new row already covers that shard, so the two
/// keys collapse to one physical holder (no coverage lost).
///
/// Returns the number of rows re-attributed (moved). Colliding rows that were
/// dropped are not counted as moves. `h_app_id` scopes to the dataplane the
/// resilience card reads (`lamad`); a key belongs to exactly one agent, so
/// matching by key within that scope never touches another agent's rows.
pub fn rekey_peer_id(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    old_peer_id: &str,
    new_peer_id: &str,
) -> Result<usize, StorageError> {
    if new_peer_id.is_empty() || old_peer_id == new_peer_id {
        return Ok(0);
    }

    // Shards already held under the NEW key — a move onto these would collide.
    //
    // The collision pre-check MUST match the actual PK scope, which is
    // `(shard_hash, peer_id)` with NO `h_app_id` (see the initial migration).
    // A same-shard row already held under `new_peer_id` in a DIFFERENT app scope
    // is still a PK collision for the `SET peer_id = new_peer_id` UPDATE below —
    // scoping this probe by `h_app_id` (as an earlier version did) hid such rows,
    // so the UPDATE hit a UNIQUE violation and aborted the whole supersede
    // transaction. Probe h_app_id-agnostically. (The move itself stays scoped to
    // `h_app_id` — a key belongs to exactly one agent, so we only ever re-attribute
    // this scope's rows; a cross-scope collision just means the shard is already
    // covered under the new key somewhere and the stale old-scope row is dropped.)
    let existing_new: std::collections::HashSet<String> = shard_locations::table
        .filter(shard_locations::peer_id.eq(new_peer_id))
        .select(shard_locations::shard_hash)
        .load::<String>(conn)?
        .into_iter()
        .collect();

    let old_shards: Vec<String> = shard_locations::table
        .filter(shard_locations::h_app_id.eq(h_app_id))
        .filter(shard_locations::peer_id.eq(old_peer_id))
        .select(shard_locations::shard_hash)
        .load::<String>(conn)?;

    // Per-row (N+1): one UPDATE/DELETE per shard the old key held. This runs at
    // boot, once per superseded human, over a single household's shard set — a
    // handful of rows — so the N+1 is acceptable here. If this ever moves to a
    // hot/bulk path, replace the loop with two set statements: an
    // `UPDATE … SET peer_id=new WHERE peer_id=old AND shard_hash NOT IN (<new's shards>)`
    // followed by a `DELETE … WHERE peer_id=old` to collapse the colliding remainder.
    let mut moved = 0usize;
    for shard_hash in old_shards {
        if existing_new.contains(&shard_hash) {
            // Collision: the shard is already covered under the new key — drop
            // the stale duplicate rather than violate the PK.
            diesel::delete(
                shard_locations::table
                    .filter(shard_locations::h_app_id.eq(h_app_id))
                    .filter(shard_locations::shard_hash.eq(&shard_hash))
                    .filter(shard_locations::peer_id.eq(old_peer_id)),
            )
            .execute(conn)?;
        } else {
            moved += diesel::update(
                shard_locations::table
                    .filter(shard_locations::h_app_id.eq(h_app_id))
                    .filter(shard_locations::shard_hash.eq(&shard_hash))
                    .filter(shard_locations::peer_id.eq(old_peer_id)),
            )
            .set(shard_locations::peer_id.eq(new_peer_id))
            .execute(conn)?;
        }
    }
    Ok(moved)
}

#[cfg(test)]
mod rekey_tests {
    use super::*;
    use crate::db::models::NewShardLocation;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn seed(conn: &mut SqliteConnection, shard: &str, peer: &str, app: &str) {
        upsert_location(
            conn,
            &NewShardLocation {
                shard_hash: shard,
                peer_id: peer,
                h_app_id: app,
                status: "verified",
            },
        )
        .expect("seed shard location");
    }

    #[test]
    fn rekey_moves_all_holder_rows_in_scope() {
        let mut conn = setup();
        seed(&mut conn, "shard-a", "uhCAkOLD", "lamad");
        seed(&mut conn, "shard-b", "uhCAkOLD", "lamad");
        // A different-scope row (distinct shard — the PK is (shard_hash, peer_id),
        // no h_app_id, so a same-shard row would collide) must be untouched by a
        // lamad-scoped rekey. A different peer likewise.
        seed(&mut conn, "shard-q", "uhCAkOLD", "qahal");
        seed(&mut conn, "shard-c", "uhCAkOTHER", "lamad");

        let moved = rekey_peer_id(&mut conn, "lamad", "uhCAkOLD", "uhCAkNEW").expect("rekey");
        assert_eq!(moved, 2, "both lamad rows under the old key moved");

        assert_eq!(
            get_locations_for_peer(&mut conn, "uhCAkNEW").unwrap().len(),
            2
        );
        // Out-of-scope + other-peer rows untouched.
        let old = get_locations_for_peer(&mut conn, "uhCAkOLD").unwrap();
        assert_eq!(old.len(), 1, "the qahal-scope row stays under the old key");
        assert_eq!(old[0].h_app_id, "qahal");
        assert_eq!(
            get_locations_for_peer(&mut conn, "uhCAkOTHER")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn rekey_collapses_colliding_shard_rows() {
        let mut conn = setup();
        // shard-a is held under BOTH keys (a re-distribute after the re-key).
        seed(&mut conn, "shard-a", "uhCAkOLD", "lamad");
        seed(&mut conn, "shard-a", "uhCAkNEW", "lamad");
        // shard-b only under the old key.
        seed(&mut conn, "shard-b", "uhCAkOLD", "lamad");

        let moved = rekey_peer_id(&mut conn, "lamad", "uhCAkOLD", "uhCAkNEW").expect("rekey");
        assert_eq!(
            moved, 1,
            "only shard-b moved; shard-a collided and collapsed"
        );

        // No rows left under the old key.
        assert!(get_locations_for_peer(&mut conn, "uhCAkOLD")
            .unwrap()
            .is_empty());
        // New key now covers both shards, exactly once each.
        let new = get_locations_for_peer(&mut conn, "uhCAkNEW").unwrap();
        assert_eq!(new.len(), 2);
    }

    #[test]
    fn rekey_survives_cross_scope_collision_on_new_key() {
        // The PK is (shard_hash, peer_id) with NO h_app_id: a same-shard row held
        // under the NEW key in a DIFFERENT app scope is still a PK collision for a
        // move onto the new key. An earlier h_app_id-scoped collision probe missed
        // it, so the UPDATE hit a UNIQUE violation and aborted the whole supersede
        // transaction. This proves the probe now sees the cross-scope row.
        let mut conn = setup();
        // shard-x already held by the NEW key, but in the `qahal` scope.
        seed(&mut conn, "shard-x", "uhCAkNEW", "qahal");
        // The same shard held by the OLD key in the `lamad` scope (the fossil row
        // the cascade wants to re-attribute).
        seed(&mut conn, "shard-x", "uhCAkOLD", "lamad");
        // A genuinely-movable lamad row (new key holds it nowhere) — must still move.
        seed(&mut conn, "shard-y", "uhCAkOLD", "lamad");

        let moved = rekey_peer_id(&mut conn, "lamad", "uhCAkOLD", "uhCAkNEW")
            .expect("rekey must not violate the cross-scope PK");
        assert_eq!(
            moved, 1,
            "only shard-y moved; shard-x collided cross-scope and its stale lamad row was dropped"
        );

        // The cross-scope new-key row is untouched.
        let qahal = get_locations_for_shard(&mut conn, "shard-x").unwrap();
        assert_eq!(qahal.len(), 1, "shard-x collapses to one physical holder");
        assert_eq!(qahal[0].peer_id, "uhCAkNEW");
        assert_eq!(qahal[0].h_app_id, "qahal");

        // No lamad row lingers under the old key.
        assert!(get_locations_for_peer(&mut conn, "uhCAkOLD")
            .unwrap()
            .is_empty());
        // shard-y now lives under the new key in lamad scope.
        let new_rows = get_locations_for_peer(&mut conn, "uhCAkNEW").unwrap();
        assert_eq!(new_rows.len(), 2, "qahal shard-x + moved lamad shard-y");
    }

    #[test]
    fn rekey_noop_when_old_equals_new_or_empty() {
        let mut conn = setup();
        seed(&mut conn, "shard-a", "uhCAkOLD", "lamad");
        assert_eq!(
            rekey_peer_id(&mut conn, "lamad", "uhCAkOLD", "uhCAkOLD").unwrap(),
            0
        );
        assert_eq!(
            rekey_peer_id(&mut conn, "lamad", "uhCAkOLD", "").unwrap(),
            0
        );
        assert_eq!(
            get_locations_for_peer(&mut conn, "uhCAkOLD").unwrap().len(),
            1
        );
    }
}

#[cfg(test)]
mod prune_orphaned_tests {
    use super::*;
    use crate::db::models::{NewShardLocation, NewShardManifest};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn seed_loc(conn: &mut SqliteConnection, shard: &str, app: &str) {
        upsert_location(
            conn,
            &NewShardLocation {
                shard_hash: shard,
                peer_id: "uhCAkHolder",
                h_app_id: app,
                status: "announced",
            },
        )
        .expect("seed location");
    }

    fn seed_manifest(conn: &mut SqliteConnection, content_id: &str, app: &str, shards: &[&str]) {
        let json = serde_json::to_string(shards).expect("encode");
        crate::db::shard_manifests::upsert_manifest(
            conn,
            &NewShardManifest {
                content_id,
                h_app_id: app,
                blob_hash: "sha256-x",
                blob_cid: None,
                encoding: "rs",
                data_shard_count: shards.len() as i32,
                parity_shard_count: 0,
                shard_hashes_json: &json,
                total_size_bytes: 0,
                shard_size_bytes: 0,
                mime_type: "html5-app",
                reach: "commons",
            },
        )
        .expect("seed manifest");
    }

    /// Only hashes that NO manifest names are deleted, and the delete is scoped to
    /// the caller's app.
    #[test]
    fn prunes_only_unreferenced_hashes_in_scope() {
        let mut conn = setup();
        seed_loc(&mut conn, "s-orphan", "lamad");
        seed_loc(&mut conn, "s-live", "lamad");
        seed_manifest(&mut conn, "live-content", "lamad", &["s-live"]);

        let candidates: std::collections::HashSet<String> =
            ["s-orphan".to_string(), "s-live".to_string()]
                .into_iter()
                .collect();
        let n = prune_orphaned_locations(&mut conn, "lamad", &candidates).expect("prune");

        assert_eq!(n, 1, "only the unreferenced hash may be deleted");
        assert!(get_locations_for_shard(&mut conn, "s-orphan")
            .expect("query")
            .is_empty());
        assert_eq!(
            get_locations_for_shard(&mut conn, "s-live")
                .expect("query")
                .len(),
            1,
            "a hash a manifest still names must survive"
        );
    }

    /// SAFETY: the reference scan is deliberately CROSS-APP. A shard hash is
    /// content-addressed, so a manifest in another app scope legitimately names the
    /// same bytes — pruning the lamad-scoped holder row would break that manifest's
    /// join. A hash referenced anywhere is never pruned.
    #[test]
    fn a_hash_referenced_by_another_app_is_never_pruned() {
        let mut conn = setup();
        seed_loc(&mut conn, "s-shared", "lamad");
        seed_manifest(&mut conn, "qahal-content", "qahal", &["s-shared"]);

        let candidates: std::collections::HashSet<String> =
            ["s-shared".to_string()].into_iter().collect();
        let n = prune_orphaned_locations(&mut conn, "lamad", &candidates).expect("prune");

        assert_eq!(n, 0, "a cross-app manifest reference must protect the hash");
        assert_eq!(
            get_locations_for_shard(&mut conn, "s-shared")
                .expect("query")
                .len(),
            1
        );
    }

    /// A hash NOT in the candidate set is never touched, even when unreferenced —
    /// the prune only ever collects what a re-stamp in the same sweep superseded.
    #[test]
    fn unreferenced_hashes_outside_the_candidate_set_are_untouched() {
        let mut conn = setup();
        seed_loc(&mut conn, "s-unrelated", "lamad");

        let empty: std::collections::HashSet<String> = std::collections::HashSet::new();
        assert_eq!(
            prune_orphaned_locations(&mut conn, "lamad", &empty).expect("prune"),
            0
        );
        assert_eq!(
            get_locations_for_shard(&mut conn, "s-unrelated")
                .expect("query")
                .len(),
            1,
            "the prune is candidate-scoped, never a global sweep"
        );
    }
}

#[cfg(test)]
mod apply_peer_announced_tests {
    use super::*;
    use crate::db::models::NewShardLocation;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn seed_local(conn: &mut SqliteConnection, shard: &str, peer: &str, status: &str) {
        upsert_location(
            conn,
            &NewShardLocation {
                shard_hash: shard,
                peer_id: peer,
                h_app_id: "lamad",
                status,
            },
        )
        .expect("seed local row");
    }

    fn row(conn: &mut SqliteConnection, shard: &str, peer: &str) -> Option<ShardLocationRow> {
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard))
            .filter(shard_locations::peer_id.eq(peer))
            .first::<ShardLocationRow>(conn)
            .optional()
            .expect("load row")
    }

    #[test]
    fn fills_absence_with_a_peer_announced_row() {
        let mut conn = setup();
        let out = apply_peer_announced(
            &mut conn,
            "shard-a",
            "uhCAkHolder",
            "lamad",
            1_700_000_000_000,
            Some("uhCAkSelf"),
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::Inserted);

        let r = row(&mut conn, "shard-a", "uhCAkHolder").expect("row present");
        assert_eq!(r.status, PEER_ANNOUNCED_STATUS);
        assert!(
            r.last_verified.is_some(),
            "peer-announced rows carry the announcement timestamp in last_verified"
        );
    }

    #[test]
    fn normalizes_agent_prefixed_holder_to_bare_form() {
        let mut conn = setup();
        // A prefixed holder cid must land as the BARE join key, and a prefixed
        // self-claim must still be caught by the bare-form self-drop compare.
        let out = apply_peer_announced(
            &mut conn,
            "shard-p",
            "agent:uhCAkHolder",
            "lamad",
            1_700_000_000_000,
            Some("uhCAkSelf"),
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::Inserted);
        assert!(
            row(&mut conn, "shard-p", "uhCAkHolder").is_some(),
            "row stored under the bare cid (never the agent:-prefixed wire form)"
        );
        assert!(row(&mut conn, "shard-p", "agent:uhCAkHolder").is_none());

        let self_out = apply_peer_announced(
            &mut conn,
            "shard-p",
            "agent:uhCAkSelf",
            "lamad",
            1_700_000_000_001,
            Some("uhCAkSelf"),
        )
        .expect("apply");
        assert_eq!(self_out, PeerAnnounceOutcome::DroppedSelf);
    }

    #[test]
    fn never_overwrites_a_self_held_row() {
        let mut conn = setup();
        // A locally-witnessed self-held row for the SAME (shard, holder).
        seed_local(&mut conn, "shard-a", "uhCAkHolder", "self-held");

        let out = apply_peer_announced(
            &mut conn,
            "shard-a",
            "uhCAkHolder",
            "lamad",
            1_700_000_000_000,
            Some("uhCAkSelf"),
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::SkippedStronger);

        let r = row(&mut conn, "shard-a", "uhCAkHolder").expect("row present");
        assert_eq!(
            r.status, "self-held",
            "a witnessed self-held row must survive a peer announcement"
        );
    }

    #[test]
    fn never_overwrites_a_push_ack_announced_row() {
        let mut conn = setup();
        // "announced" = push-acked (locally witnessed by the pushing node).
        seed_local(&mut conn, "shard-a", "uhCAkHolder", "announced");

        let out = apply_peer_announced(
            &mut conn,
            "shard-a",
            "uhCAkHolder",
            "lamad",
            1_700_000_000_000,
            None,
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::SkippedStronger);
        assert_eq!(
            row(&mut conn, "shard-a", "uhCAkHolder").unwrap().status,
            "announced"
        );
    }

    #[test]
    fn newer_peer_announced_refreshes_older_but_older_is_skipped() {
        let mut conn = setup();
        apply_peer_announced(&mut conn, "s", "uhCAkH", "lamad", 1_000, None).expect("first");

        // Strictly newer → refresh.
        let out =
            apply_peer_announced(&mut conn, "s", "uhCAkH", "lamad", 2_000, None).expect("2nd");
        assert_eq!(out, PeerAnnounceOutcome::Refreshed);

        // Older-or-equal → skipped (monotone guard).
        let older =
            apply_peer_announced(&mut conn, "s", "uhCAkH", "lamad", 1_500, None).expect("older");
        assert_eq!(older, PeerAnnounceOutcome::SkippedStale);
        let equal =
            apply_peer_announced(&mut conn, "s", "uhCAkH", "lamad", 2_000, None).expect("equal");
        assert_eq!(equal, PeerAnnounceOutcome::SkippedStale);
    }

    #[test]
    fn drops_a_self_announcement() {
        let mut conn = setup();
        // A peer telling us WE hold a shard — weaker than our own records; drop.
        let out = apply_peer_announced(
            &mut conn,
            "shard-a",
            "uhCAkSelf",
            "lamad",
            1_700_000_000_000,
            Some("uhCAkSelf"),
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::DroppedSelf);
        assert!(
            row(&mut conn, "shard-a", "uhCAkSelf").is_none(),
            "a self-claim from a peer must never produce a row"
        );
    }

    #[test]
    fn refuses_a_non_agent_cid_holder() {
        let mut conn = setup();
        let out = apply_peer_announced(
            &mut conn,
            "shard-a",
            "12D3KooWTransportId",
            "lamad",
            1_700_000_000_000,
            None,
        )
        .expect("apply");
        assert_eq!(out, PeerAnnounceOutcome::NotAgentCid);
        assert!(
            row(&mut conn, "shard-a", "12D3KooWTransportId").is_none(),
            "a cross-namespace holder must never land in the join-key column"
        );
    }

    #[test]
    fn inventory_apply_tallies_each_class() {
        use crate::p2p::custody_announce::CustodyAnnouncement;
        let mut conn = setup();
        // Pre-seed a self-held row that a peer claim must not overwrite.
        seed_local(&mut conn, "s-strong", "uhCAkH1", "self-held");

        let mk = |shard: &str, holder: &str, ms: i64| CustodyAnnouncement {
            shard_hash: shard.to_string(),
            holder_agent_cid: holder.to_string(),
            h_app_id: "lamad".to_string(),
            status: "announced".to_string(),
            announced_at_ms: ms,
            signature: vec![0x00],
        };
        let entries = vec![
            mk("s-new", "uhCAkH2", 1_000),    // inserted
            mk("s-strong", "uhCAkH1", 9_999), // dropped_weaker (self-held wins)
            mk("s-self", "uhCAkSelf", 1_000), // dropped_self
        ];

        let stats = apply_custody_inventory(&mut conn, &entries, Some("uhCAkSelf"));
        assert_eq!(stats.inserted, 1);
        assert_eq!(stats.dropped_weaker, 1);
        assert_eq!(stats.dropped_self, 1);
        assert_eq!(stats.applied(), 1);
    }
}
