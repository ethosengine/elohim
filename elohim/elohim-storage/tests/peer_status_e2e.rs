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
//!    - The signal-subscription plumbing (Phase 1 Gap 1) IS now in place:
//!        a) `HcClient::subscribe_infrastructure_signals(handler)` wraps
//!           `AppWebsocket::on_signal` and msgpack-decodes the payload into
//!           `InfrastructureSignal` — same pattern doorway uses for its
//!           `ProjectionSignal` in `projection/subscriber.rs`.
//!        b) `main.rs` spawns a subscriber task right after the heartbeat
//!           `HcClient` connects on the `infrastructure` role; each decoded
//!           `InfrastructureSignal` is routed into `signals::handle_signal`
//!           against a dedicated Diesel pool.
//!      The only remaining blocker for deleting `#[ignore]` is the harness:
//!      bring up a tmp conductor dir + happ (as `elohim-import`'s integration
//!      tests do), hold the HTTP server up, and assert the SQLite projection
//!      after one 60s heartbeat tick. That harness is its own project-scale
//!      concern — tracked separately from Phase 1.
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
use elohim_storage::{
    build_peer_status_view, load_elohim_capability_from_env, ElohimCapabilityProfile,
};

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

    let signal: InfrastructureSignal = serde_json::from_value(wire)
        .expect("wire shape must deserialize into InfrastructureSignal");

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

// =============================================================================
// Task 10.1 — operator-config hydration for elohim_capability (Phase 10)
// =============================================================================

/// Helper: build a minimal PeerStatusRow for view construction tests.
fn smoke_row() -> elohim_storage::db::peer_statuses::PeerStatusRow {
    elohim_storage::db::peer_statuses::PeerStatusRow {
        peer_id: "uhCAkSMOKE".to_string(),
        status: "online".to_string(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 1,
        archetype_class: Some("home-nuc".to_string()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "uhCkkSMOKE".to_string(),
        updated_at: 1_700_000_000_000_000,
    }
}

/// Helper: build a minimal ElohimCapabilityProfile for hydration tests.
fn sample_capability() -> ElohimCapabilityProfile {
    ElohimCapabilityProfile {
        model_name: "claude-opus-4-7".to_string(),
        model_family: "claude".to_string(),
        context_window_tokens: 200_000,
        constitution_cid: Some("epr:constitutions:family-stewardship-v1".to_string()),
        quantization_spec: None,
        deployment_context: Some("elohim-node-linux-x86_64".to_string()),
        specialties: vec!["child-safety".to_string()],
        skills: vec!["discernment-evaluation".to_string()],
        strengths: vec![],
        active_since: "2026-04-19T00:00:00Z".to_string(),
        reach_level: None,
    }
}

/// `build_peer_status_view` with Some(capability) layers the profile onto the view.
#[test]
fn build_peer_status_view_with_capability() {
    let cap = sample_capability();
    let view = build_peer_status_view(smoke_row(), Some(&cap));

    assert_eq!(view.peer_id, "uhCAkSMOKE");
    assert_eq!(view.status, "online");
    let hydrated = view.elohim_capability.expect("capability must be present");
    assert_eq!(hydrated.model_name, "claude-opus-4-7");
    assert_eq!(hydrated.model_family, "claude");
    assert_eq!(hydrated.context_window_tokens, 200_000);
}

/// `build_peer_status_view` with None leaves elohim_capability as None.
#[test]
fn build_peer_status_view_without_capability() {
    let view = build_peer_status_view(smoke_row(), None);
    assert!(
        view.elohim_capability.is_none(),
        "no capability for storage/relay nodes"
    );
}

/// `load_elohim_capability_from_env` returns None when env var is unset.
#[test]
fn load_capability_unset_env_returns_none() {
    // Guard: remove the env var for this test (may be set in CI).
    // Safety: single-threaded test; env mutation is local to this process.
    std::env::remove_var("ELOHIM_CAPABILITY_CONFIG_FILE");
    assert!(load_elohim_capability_from_env().is_none());
}

/// `load_elohim_capability_from_env` returns None when the file does not exist.
#[test]
fn load_capability_missing_file_returns_none() {
    std::env::set_var(
        "ELOHIM_CAPABILITY_CONFIG_FILE",
        "/tmp/elohim-capability-does-not-exist-xyzzy.json",
    );
    assert!(load_elohim_capability_from_env().is_none());
    std::env::remove_var("ELOHIM_CAPABILITY_CONFIG_FILE");
}

/// `load_elohim_capability_from_env` returns None when the file contains malformed JSON.
#[test]
fn load_capability_malformed_json_returns_none() {
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    use std::io::Write;
    write!(tmp, "{{ this is not valid json }}").unwrap();
    std::env::set_var("ELOHIM_CAPABILITY_CONFIG_FILE", tmp.path());
    assert!(load_elohim_capability_from_env().is_none());
    std::env::remove_var("ELOHIM_CAPABILITY_CONFIG_FILE");
}

/// `load_elohim_capability_from_env` returns Some(profile) when pointed at a valid JSON file.
#[test]
fn load_capability_valid_file_returns_some() {
    let profile_json = serde_json::json!({
        "modelName": "claude-opus-4-7",
        "modelFamily": "claude",
        "contextWindowTokens": 200000,
        "constitutionCid": "epr:constitutions:family-stewardship-v1",
        "quantizationSpec": null,
        "deploymentContext": "elohim-node-linux-x86_64",
        "specialties": ["child-safety"],
        "skills": ["discernment-evaluation"],
        "strengths": [],
        "activeSince": "2026-04-19T00:00:00Z",
        "reachLevel": null
    });
    let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
    use std::io::Write;
    write!(tmp, "{}", profile_json).unwrap();
    std::env::set_var("ELOHIM_CAPABILITY_CONFIG_FILE", tmp.path());

    let profile = load_elohim_capability_from_env().expect("must return Some for valid file");
    assert_eq!(profile.model_name, "claude-opus-4-7");
    assert_eq!(profile.context_window_tokens, 200_000);
    assert_eq!(profile.specialties, vec!["child-safety".to_string()]);

    std::env::remove_var("ELOHIM_CAPABILITY_CONFIG_FILE");
}

/// Integration: when AppState holds Some(capability), build_peer_status_view reflects it.
/// When AppState holds None, elohim_capability is None.
#[test]
fn capability_hydration_mirrors_app_state() {
    // Simulates the pattern used in HTTP route handlers:
    // let view = build_peer_status_view(row, self.elohim_capability.as_ref());
    let cap = sample_capability();

    // AppState-with-capability path
    let state_capability: Option<ElohimCapabilityProfile> = Some(cap.clone());
    let view_with = build_peer_status_view(smoke_row(), state_capability.as_ref());
    assert!(view_with.elohim_capability.is_some());
    assert_eq!(
        view_with.elohim_capability.as_ref().unwrap().model_name,
        "claude-opus-4-7"
    );

    // AppState-without-capability path
    let state_none: Option<ElohimCapabilityProfile> = None;
    let view_without = build_peer_status_view(smoke_row(), state_none.as_ref());
    assert!(view_without.elohim_capability.is_none());
}
