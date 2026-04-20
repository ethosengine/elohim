//! Test utilities shared across integration tests.
//!
//! Exposed as a public module so `tests/*.rs` integration test binaries can
//! reference `elohim_storage::test_util::test_pool`.  Not intended for
//! production use.

use crate::db::{init_pool, run_migrations, DbPool};

/// Create an in-memory SQLite pool with all migrations applied.
///
/// Each call returns a fresh, isolated database — safe to use concurrently
/// across test threads because SQLite in-memory databases are not shared.
pub fn test_pool() -> DbPool {
    // Use a unique URI so concurrent tests don't share the same in-memory DB.
    // The `?mode=memory&cache=private` prevents connection sharing across pool
    // connections (each checkout is a fresh empty DB), so we use a named DB
    // instead and allow the pool to keep one connection alive.
    let url = format!(
        "file:testdb_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = init_pool(&url).expect("test pool init failed");
    run_migrations(&pool).expect("test migrations failed");
    pool
}

// ---------------------------------------------------------------------------
// P2P test harness — only compiled when the `p2p` feature is enabled
// ---------------------------------------------------------------------------

/// Harness returned by `spawn_p2p_with_peers`.
///
/// Holds a stub [`crate::p2p::P2PHandle`] (no real libp2p swarm — all push/fetch
/// calls return stub errors) and the pool that was seeded with the given peers.
///
/// **Limitation:** because the stub handle does not run a real libp2p swarm,
/// `push_shard` always returns `Err("stub: no P2P swarm in test")`.  This
/// means `distribute_shards` will always report `distributed == 0` and
/// `shard_locations` will never be populated.  Tests that need to verify
/// household diversity via `shard_locations` must use a live harness — see
/// Task 17 for live integration coverage.
///
/// Tests that only need to verify `placement_gaps` recording (gap written on
/// short placement; cleared on full placement) work correctly with this stub
/// because the gap logic runs before any push attempt.
#[cfg(feature = "p2p")]
pub struct P2PTestHarness {
    pub p2p: crate::p2p::P2PHandle,
    pub pool: DbPool,
}

/// Seed the DB with the given peers and return a stub P2P harness.
///
/// Each entry in `peers` is `(agent_key, household_id, lifecycle)` where
/// `lifecycle` is `"accepting"` or `"leaving"` (matches peer_statuses semantics
/// used in peer_selection tests).
///
/// For each peer an active REA commitment is inserted for `content:commons` so
/// that `PeerSelection` can find them.
///
/// Returns a `P2PTestHarness` with:
/// - `pool`: the seeded database
/// - `p2p`:  a stub handle (see `P2PHandle::for_testing()`)
#[cfg(feature = "p2p")]
pub async fn spawn_p2p_with_peers(
    pool: DbPool,
    peers: &[(&str, &str, &str)],
) -> P2PTestHarness {
    use diesel::prelude::*;
    use diesel::RunQueryDsl;
    use crate::db::diesel_schema::{peer_statuses, rea_commitments};
    use crate::db::models::NewHuman;

    let mut conn = pool.get().expect("test pool connection");

    for (agent_key, household_id, lifecycle) in peers {
        // Insert human row
        let human_id = format!("human-{agent_key}");
        diesel::insert_or_ignore_into(crate::db::diesel_schema::humans::table)
            .values(&NewHuman {
                id: human_id.clone(),
                agent_pub_key: Some((*agent_key).to_string()),
                display_name: human_id.clone(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "lamad".to_string(),
                household_id: Some((*household_id).to_string()),
            })
            .execute(&mut conn)
            .expect("insert human");

        // Insert peer status
        let (status, pool_member, reserves) = match *lifecycle {
            "accepting" => ("online", 1i32, 1i32),
            "leaving"   => ("leaving", 0i32, 0i32),
            other       => panic!("unexpected lifecycle in spawn_p2p_with_peers: {other}"),
        };
        diesel::insert_or_ignore_into(peer_statuses::table)
            .values((
                peer_statuses::peer_id.eq(*agent_key),
                peer_statuses::status.eq(status),
                peer_statuses::general_pool_member.eq(pool_member),
                peer_statuses::accepting_stewardship_reserves.eq(reserves),
                peer_statuses::timestamp.eq(1_700_000_000_000_000i64),
                peer_statuses::dht_anchor_hash.eq("anchor-placeholder"),
                peer_statuses::updated_at.eq(1_700_000_000_000_000i64),
            ))
            .execute(&mut conn)
            .expect("insert peer_status");

        // Insert REA commitment for content:commons
        let cmt_id = format!("cmt-{agent_key}-commons");
        diesel::insert_or_ignore_into(rea_commitments::table)
            .values((
                rea_commitments::id.eq(&cmt_id),
                rea_commitments::h_app_id.eq("lamad"),
                rea_commitments::action.eq("provide"),
                rea_commitments::provider.eq(*agent_key),
                rea_commitments::receiver.eq(""),
                rea_commitments::resource_classified_as.eq(Some("content:commons")),
                rea_commitments::state.eq("active"),
                rea_commitments::finished.eq(0),
                rea_commitments::created_at.eq("2026-04-19T00:00:00Z"),
            ))
            .execute(&mut conn)
            .expect("insert rea_commitment");
    }

    let p2p = crate::p2p::P2PHandle::for_testing();
    P2PTestHarness { p2p, pool }
}
