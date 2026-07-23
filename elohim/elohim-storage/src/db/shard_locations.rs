//! Shard location CRUD operations using Diesel.
//!
//! Shard locations track which peers hold which shards, with verification
//! timestamps. This is per-peer local state (Category C), rebuilt from
//! shard protocol ack events — not DHT-notarized.

use diesel::prelude::*;

use super::diesel_schema::shard_locations;
use super::models::{NewShardLocation, ShardLocationRow};
use crate::StorageError;

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
