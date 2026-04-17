//! End-to-end smoke test for Peer-Stewarded Availability Phase 1 (Task 17).
//!
//! This test is the capstone for Phase 1. It documents the full contract
//! from the in-process signal envelope (as emitted by the infrastructure
//! DNA's post-commit hook, projected onto the wire as JSON) all the way
//! into the SQLite `peer_statuses` table that the doorway forwarder reads.
//!
//! ## What this file covers today
//!
//! 1. **In-process projection contract (runs by default).** Constructs an
//!    `InfrastructureSignal::PeerStatusRecorded` value with exactly the wire
//!    shape the DNA emits (see Task 6), feeds it through `signals::handle_signal`
//!    against a fresh SQLite connection, and asserts the `peer_statuses`
//!    projection row is upserted with the expected flags. This is the
//!    narrowest honest E2E we can run without booting a conductor — it
//!    still exercises real Diesel, real schema, real `(de)serialize_with`
//!    attribute drift.
//!
//! 2. **Full live-conductor flow (marked `#[ignore]`).** Documents the
//!    shape of the full flow — conductor up, heartbeat task firing,
//!    signal subscription pulling the post-commit payload off the app
//!    WebSocket, signal handler upserting into SQLite. Not runnable
//!    today because:
//!
//!    - No `SweetConductor`-based harness exists in this crate's `tests/`
//!      directory. All existing integration tests (`sync_integration.rs`,
//!      `provenance_gate_integration.rs`, `resilience_integration.rs`,
//!      `schema_contract.rs`) are pure-Rust, no-conductor tests.
//!    - `elohim_storage::hc_client::HcClient` does not currently expose
//!      a signal receiver. The underlying `AppWebsocket` supports
//!      `on_signal(...)` (see doorway's `projection/subscriber.rs` for
//!      the reference implementation), but that API is not plumbed
//!      through `HcClient` yet, and `main.rs` does not spawn a
//!      subscriber task. Phase 2 should:
//!        a) Add `HcClient::subscribe_signals(handler)` that wraps
//!           `AppWebsocket::on_signal` and msgpack-decodes the payload
//!           into `InfrastructureSignal` (doorway does the same thing
//!           for `ProjectionSignal`).
//!        b) In `main.rs`, after the heartbeat `HcClient` connects on
//!           the `infrastructure` role, spawn a task that calls
//!           `subscribe_signals` and routes each `InfrastructureSignal`
//!           into `signals::handle_signal` against the pool.
//!        c) Delete the `#[ignore]` below and wire it up to a real
//!           harness (likely: bring up a tmp conductor dir + happ, as
//!           done by `elohim-import`'s integration tests, and assert on
//!           the SQLite projection after one 60s heartbeat tick).
//!
//! Running:
//! ```text
//! cargo test --test peer_status_e2e                       # default: runs the in-process projection case
//! cargo test --test peer_status_e2e -- --ignored --nocapture   # full live-conductor placeholder (doesn't run)
//! ```

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::peer_statuses::get_by_peer;
use elohim_storage::signals::{handle_signal, InfrastructureSignal};

/// Create an in-memory SQLite connection pre-populated with the
/// `peer_statuses` schema that the production migration emits.
/// Kept byte-identical to the schema shape in `signals.rs::peer_status_tests`
/// and `db/peer_statuses.rs::tests` so this file tests the *integration*
/// between `handle_signal` and `db::peer_statuses::get_by_peer`, not a
/// rewritten table.
fn setup_projection_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");
    conn.batch_execute(
        r#"
        CREATE TABLE peer_statuses (
            peer_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            general_pool_member INTEGER NOT NULL,
            accepting_stewardship_reserves INTEGER NOT NULL,
            archetype_class TEXT,
            timestamp BIGINT NOT NULL,
            dht_anchor_hash TEXT NOT NULL,
            updated_at BIGINT NOT NULL
        );
        CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
        CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
        "#,
    )
    .expect("Failed to create projection schema");
    conn
}

/// Narrowest honest E2E: deserialize the exact on-wire shape the DNA's
/// `post_commit` hook emits (see Task 6 — `PeerStatusRecorded` tagged
/// as `{"type": "PeerStatusRecorded", "payload": { ... }}`), run it
/// through `handle_signal`, assert SQLite has the row.
///
/// This is the contract the doorway forwarder relies on to find
/// general-pool peers at request time; if any of the three hops drifts
/// (DNA wire shape, `InfrastructureSignal` serde tagging, projection
/// upsert), this test catches it.
#[tokio::test]
async fn peer_status_signal_projects_into_sqlite_row() {
    let mut conn = setup_projection_db();

    // Build the wire shape exactly as the infrastructure DNA emits it.
    // Byte-identical to the fixture in `signals.rs::serde_tag_matches_dna_wire_format`.
    let wire = serde_json::json!({
        "type": "PeerStatusRecorded",
        "payload": {
            "peer_id": "uhCAkSMOKE",
            "status": "online",
            "general_pool_member": true,
            "accepting_stewardship_reserves": true,
            "archetype_class": "home-nuc",
            "timestamp": 1_700_000_000_000_000_i64,
            "action_hash": "uhCkkSMOKE",
        }
    });

    let signal: InfrastructureSignal =
        serde_json::from_value(wire).expect("wire shape must deserialize into InfrastructureSignal");

    handle_signal(&mut conn, signal).expect("handle_signal must project into SQLite");

    // This is the exact query doorway uses to resolve a specific peer.
    let row = get_by_peer(&mut conn, "uhCAkSMOKE")
        .expect("db query must succeed")
        .expect("peer_status row must exist after signal projection");

    assert_eq!(row.status, "online");
    assert_eq!(row.general_pool_member, 1);
    assert_eq!(row.accepting_stewardship_reserves, 1);
    assert_eq!(row.archetype_class.as_deref(), Some("home-nuc"));
    assert_eq!(row.dht_anchor_hash, "uhCkkSMOKE");
    assert_eq!(row.timestamp, 1_700_000_000_000_000);
}

/// Full live-conductor smoke. Not runnable yet — see module docs for
/// the Phase 2 plan that unblocks this. Kept in-tree so the shape of
/// the eventual test is visible and reviewable.
///
/// Run manually once Phase 2 lands:
/// ```text
/// cargo test --test peer_status_e2e -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "requires SweetConductor harness + HcClient::subscribe_signals (Phase 2)"]
async fn peer_publishes_status_within_one_minute() {
    // Expected Phase 2 shape:
    //
    //   let harness = PeerStatusHarness::spawn().await;  // conductor + storage + policy
    //
    //   // Heartbeat cadence is 60s; HeartbeatTask::run does an initial
    //   // tick_once before entering the interval loop (Task 12), so the
    //   // first publish fires at startup. Allow a short grace for the
    //   // signal subscription task to msgpack-decode and upsert.
    //   tokio::time::sleep(Duration::from_secs(10)).await;
    //
    //   let mut conn = harness.db_conn();
    //   let row = get_by_peer(&mut conn, &harness.agent_pubkey_base64())
    //       .expect("db query")
    //       .expect("peer_status row must exist after one heartbeat cycle");
    //
    //   assert_eq!(row.status, "online");
    //   assert_eq!(row.general_pool_member, 1);
    //
    //   harness.shutdown().await;
    //
    //   // And a final `leaving` snapshot published at shutdown (Task 13).
    //   let mut conn = harness.db_conn();
    //   let row = get_by_peer(&mut conn, &harness.agent_pubkey_base64())
    //       .unwrap()
    //       .unwrap();
    //   assert_eq!(row.status, "leaving");
    panic!("placeholder — see module docs for Phase 2 plan");
}
