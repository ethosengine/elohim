//! Integration tests for `services::peer_selection` — contract-aware diverse selector.
//!
//! Tests verify the four-branch selection logic:
//! - Ok: enough diverse peers found (distinct households prioritised first)
//! - Short(contracts-short): no REA commitments match the content scope
//! - Short(peers-unavailable): commitments exist but no peer is accepting
//! - Short(under-committed): some peers found but fewer than desired_count

use diesel::prelude::*;
use diesel::RunQueryDsl;

use elohim_storage::db;
use elohim_storage::db::diesel_schema::{peer_statuses, rea_commitments};
use elohim_storage::db::models::NewHuman;
use elohim_storage::services::peer_selection::{PeerSelection, SelectionInput, SelectionOutcome};
use elohim_storage::test_util::test_pool;

// ---------------------------------------------------------------------------
// Seed helpers — use actual schema column names
// ---------------------------------------------------------------------------

/// Insert a Human row. `agent_key` is stored in agent_pub_key (the peer identity).
fn seed_human(
    conn: &mut diesel::SqliteConnection,
    id: &str,
    agent_key: &str,
    hh: Option<&str>,
) {
    diesel::insert_into(db::diesel_schema::humans::table)
        .values(&NewHuman {
            id: id.into(),
            agent_pub_key: Some(agent_key.into()),
            display_name: id.into(),
            bio: None,
            affinities: "[]".into(),
            profile_reach: "commons".into(),
            location: None,
            profile_photo_url: None,
            h_app_id: "lamad".into(),
            household_id: hh.map(str::to_string),
        })
        .execute(conn)
        .unwrap();
}

/// Insert a PeerStatus row.
///
/// `accepting` → status="online", general_pool_member=1, accepting_stewardship_reserves=1
/// `leaving`   → status="leaving", general_pool_member=0, accepting_stewardship_reserves=0
///
/// Note: peer_statuses has NO h_app_id column in the actual schema.
fn seed_peer_status(conn: &mut diesel::SqliteConnection, peer_id: &str, lifecycle: &str) {
    let (status, pool_member, reserves) = match lifecycle {
        "accepting" => ("online", 1i32, 1i32),
        "leaving" => ("leaving", 0i32, 0i32),
        other => panic!("unexpected lifecycle in test: {other}"),
    };
    diesel::insert_into(peer_statuses::table)
        .values((
            peer_statuses::peer_id.eq(peer_id),
            peer_statuses::status.eq(status),
            peer_statuses::general_pool_member.eq(pool_member),
            peer_statuses::accepting_stewardship_reserves.eq(reserves),
            peer_statuses::timestamp.eq(1_700_000_000_000_000i64),
            peer_statuses::dht_anchor_hash.eq("anchor-placeholder"),
            peer_statuses::updated_at.eq(1_700_000_000_000_000i64),
        ))
        .execute(conn)
        .unwrap();
}

/// Insert a REA commitment row with actual column names:
/// - `provider` (not provider_agent)
/// - `resource_classified_as` (not resource_classification)
/// - `state` (not status)
fn seed_rea_commitment(
    conn: &mut diesel::SqliteConnection,
    provider: &str,
    content_scope: &str,
) {
    diesel::insert_into(rea_commitments::table)
        .values((
            rea_commitments::id.eq(format!("cmt-{provider}-{content_scope}")),
            rea_commitments::h_app_id.eq("lamad"),
            rea_commitments::action.eq("provide"),
            rea_commitments::provider.eq(provider),
            rea_commitments::receiver.eq(""),
            rea_commitments::resource_classified_as
                .eq(Some(format!("content:{content_scope}"))),
            rea_commitments::state.eq("active"),
            rea_commitments::finished.eq(0),
            rea_commitments::created_at.eq("2026-04-19T00:00:00Z"),
        ))
        .execute(conn)
        .unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn selects_distinct_households_first() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Three peers, two households: alpha-1 + alpha-2 share "home-alpha";
    // beta-1 has "home-beta". Selection of 2 should pick one from each household.
    seed_human(&mut conn, "alpha1", "agent-alpha-1", Some("home-alpha"));
    seed_human(&mut conn, "alpha2", "agent-alpha-2", Some("home-alpha"));
    seed_human(&mut conn, "beta1", "agent-beta-1", Some("home-beta"));

    for key in ["agent-alpha-1", "agent-alpha-2", "agent-beta-1"] {
        seed_peer_status(&mut conn, key, "accepting");
        seed_rea_commitment(&mut conn, key, "commons");
    }

    let sel = PeerSelection::new(pool.clone());
    let outcome = sel
        .select(&SelectionInput {
            h_app_id: "lamad",
            content_id: "content-x",
            content_reach: "commons",
            desired_count: 2,
        })
        .unwrap();

    match outcome {
        SelectionOutcome::Ok(peers) => {
            assert_eq!(peers.len(), 2);
            let households: std::collections::HashSet<&str> = peers
                .iter()
                .map(|p| p.household_id.as_deref().unwrap_or(""))
                .collect();
            assert_eq!(households.len(), 2, "expected distinct households in selection");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn reports_contracts_short_when_no_commitment_matches() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "alpha1", "agent-alpha-1", Some("home-alpha"));
    seed_peer_status(&mut conn, "agent-alpha-1", "accepting");
    // NO rea_commitment — reach=commons but no provider commitment exists.

    let sel = PeerSelection::new(pool.clone());
    let outcome = sel
        .select(&SelectionInput {
            h_app_id: "lamad",
            content_id: "content-no-contract",
            content_reach: "commons",
            desired_count: 2,
        })
        .unwrap();

    match outcome {
        SelectionOutcome::Short {
            peers, gap_kind, ..
        } => {
            assert_eq!(peers.len(), 0);
            assert_eq!(gap_kind, "contracts-short");
        }
        other => panic!("expected Short(contracts-short), got {other:?}"),
    }
}

#[test]
fn reports_peers_unavailable_when_commitments_exist_but_no_accepting_peer() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "alpha1", "agent-alpha-1", Some("home-alpha"));
    seed_peer_status(&mut conn, "agent-alpha-1", "leaving"); // not accepting
    seed_rea_commitment(&mut conn, "agent-alpha-1", "commons");

    let sel = PeerSelection::new(pool.clone());
    let outcome = sel
        .select(&SelectionInput {
            h_app_id: "lamad",
            content_id: "content-leaving",
            content_reach: "commons",
            desired_count: 1,
        })
        .unwrap();

    match outcome {
        SelectionOutcome::Short {
            peers, gap_kind, ..
        } => {
            assert_eq!(peers.len(), 0);
            assert_eq!(gap_kind, "peers-unavailable");
        }
        other => panic!("expected Short(peers-unavailable), got {other:?}"),
    }
}

#[test]
fn places_what_we_can_and_flags_under_committed_when_desired_exceeds_households() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    seed_human(&mut conn, "alpha1", "agent-alpha-1", Some("home-alpha"));
    seed_peer_status(&mut conn, "agent-alpha-1", "accepting");
    seed_rea_commitment(&mut conn, "agent-alpha-1", "commons");

    let sel = PeerSelection::new(pool.clone());
    let outcome = sel
        .select(&SelectionInput {
            h_app_id: "lamad",
            content_id: "content-one-household",
            content_reach: "commons",
            desired_count: 3, // only 1 household exists
        })
        .unwrap();

    match outcome {
        SelectionOutcome::Short {
            peers,
            gap_kind,
            achieved,
            requested,
        } => {
            assert_eq!(peers.len(), 1);
            assert_eq!(gap_kind, "under-committed");
            assert_eq!(achieved, 1);
            assert_eq!(requested, 3);
        }
        other => panic!("expected Short(under-committed), got {other:?}"),
    }
}
