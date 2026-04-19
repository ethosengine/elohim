//! Integration test for PeerStatus coordinator surface — Task B1.
//!
//! ## What this file covers
//!
//! 1. **Type-shape verification (runs by default).** Constructs a `PeerStatus`
//!    using the real `infrastructure_integrity` types, round-trips it through
//!    `serde_json`, and asserts field values. This is the narrowest honest
//!    integration test we can run without booting a conductor — it still
//!    exercises the real struct definitions, derives, and serialization
//!    contract that the DHT entry validation depends on.
//!
//! 2. **Full live-conductor flow (marked `#[ignore]`).** Documents the shape
//!    of the full `record_peer_status` + `get_latest_peer_status_for_agent`
//!    round-trip. Not runnable today because the `holochain` crate (which
//!    contains `SweetConductor` / `sweettest`) is not available in this
//!    crate's dependency graph — the infrastructure DNA workspace only pulls
//!    in `hdi`/`hdk` for the WASM guest layer. Adding `holochain` as a
//!    dev-dependency would pull in the full conductor (hundreds of crates,
//!    several hundred MB). The established repo pattern for this situation:
//!    see `elohim/elohim-storage/tests/peer_status_e2e.rs` which documents
//!    the same gap and marks its conductor test `#[ignore]`.
//!
//!    When a SweetConductor harness is added to this workspace, delete
//!    `#[ignore]` from `record_peer_status_and_query_latest` and add
//!    `holochain` + `tokio` to `[dev-dependencies]` in
//!    `zomes/infrastructure/Cargo.toml`.
//!
//! Running:
//! ```text
//! # From elohim/holochain/dna/infrastructure/:
//! cargo test --test peer_status                          # runs type-shape test
//! cargo test --test peer_status -- --ignored --nocapture # shows conductor stub
//! ```

use infrastructure_integrity::{PeerCapabilityFlags, PeerLifecycleState, PeerStatus};

use hdk::prelude::{AgentPubKey, Timestamp};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Build a canonical test PeerStatus.
///
/// Uses a zeroed agent pubkey (39 zero bytes + version prefix) which is valid
/// for serialization tests but would be rejected by integrity validation in a
/// real conductor (peer_id must equal the author).
fn make_test_status() -> PeerStatus {
    // AgentPubKey is a HoloHash<Agent>; the 36-byte zeroed form is serialisable
    // without requiring a live keystore.  We use the same pattern as the upstream
    // Holochain test suites for offline struct-shape tests (from_raw_36 is used
    // in holochain_integrity_types' own unit tests).
    let raw_bytes = vec![0u8; 36];
    let peer_id = AgentPubKey::from_raw_36(raw_bytes);

    PeerStatus {
        peer_id: peer_id.clone(),
        status: PeerLifecycleState::Online,
        flags: PeerCapabilityFlags {
            general_pool_member: true,
            accepting_stewardship_reserves: true,
        },
        archetype_class: Some("home-nuc".into()),
        timestamp: Timestamp::from_micros(1_700_000_000_000_000),
    }
}

// ---------------------------------------------------------------------------
// Running test: type-shape + serde round-trip
// ---------------------------------------------------------------------------

/// Verify the PeerStatus struct is correctly defined and serialises as
/// expected.
///
/// This test validates:
/// - All fields from the task spec are present in `infrastructure_integrity`
/// - `PeerLifecycleState::Online` serialises to `"online"` (Display impl)
/// - `PeerCapabilityFlags` fields survive a JSON round-trip
/// - `archetype_class` is `Option<String>` and survives Some/None round-trips
/// - The struct is `Clone + PartialEq` (used by coordinator query logic)
#[test]
fn peer_status_struct_shape_and_serde_round_trip() {
    let original = make_test_status();

    // Verify field accessors
    assert_eq!(original.archetype_class.as_deref(), Some("home-nuc"));
    assert!(original.flags.general_pool_member);
    assert!(original.flags.accepting_stewardship_reserves);
    assert_eq!(original.status.to_string(), "online");
    assert_eq!(original.timestamp.as_micros(), 1_700_000_000_000_000);

    // JSON round-trip — exercises the `#[hdk_entry_helper]` derive chain
    let serialised = serde_json::to_string(&original).expect("serialise PeerStatus");
    let deserialised: PeerStatus =
        serde_json::from_str(&serialised).expect("deserialise PeerStatus");

    assert_eq!(original, deserialised);
    assert_eq!(deserialised.archetype_class.as_deref(), Some("home-nuc"));
    assert!(deserialised.flags.general_pool_member);
    assert_eq!(deserialised.status.to_string(), "online");

    // None archetype_class also survives
    let mut ps_none = original.clone();
    ps_none.archetype_class = None;
    let s = serde_json::to_string(&ps_none).expect("serialise None archetype");
    let d: PeerStatus = serde_json::from_str(&s).expect("deserialise None archetype");
    assert_eq!(d.archetype_class, None);

    // Clone + PartialEq (used by coordinator logic)
    let cloned = deserialised.clone();
    assert_eq!(cloned, deserialised);
}

/// Verify all PeerLifecycleState variants serialise to the expected strings.
/// The `post_commit` signal handler emits `ps.status.to_string()` — this
/// test pins that contract so elohim-storage can safely hardcode the strings
/// in its signal projection.
#[test]
fn peer_lifecycle_state_display_strings() {
    assert_eq!(PeerLifecycleState::Starting.to_string(), "starting");
    assert_eq!(PeerLifecycleState::Online.to_string(), "online");
    assert_eq!(PeerLifecycleState::Degraded.to_string(), "degraded");
    assert_eq!(PeerLifecycleState::Maintenance.to_string(), "maintenance");
    assert_eq!(PeerLifecycleState::Leaving.to_string(), "leaving");
}

// ---------------------------------------------------------------------------
// Ignored test: full live-conductor flow
// ---------------------------------------------------------------------------

/// Full live-conductor round-trip: `record_peer_status` then
/// `get_latest_peer_status_for_agent`.
///
/// Not runnable today — see module docs for the conductor harness gap.
/// Kept in-tree so the intended test shape is visible and reviewable.
///
/// Run once Phase 2 adds a SweetConductor harness:
/// ```text
/// cargo test --test peer_status -- record_peer_status_and_query_latest --ignored --nocapture
/// ```
#[test]
#[ignore = "requires SweetConductor harness (holochain crate not in dev-dependencies — see module docs)"]
fn record_peer_status_and_query_latest() {
    // Expected shape once SweetConductor is wired:
    //
    //   #[tokio::test(flavor = "multi_thread")]
    //   async fn record_peer_status_and_query_latest() {
    //       let mut conductor = SweetConductor::from_standard_config().await;
    //       let dna_path = std::env::var("INFRASTRUCTURE_DNA_PATH")
    //           .expect("INFRASTRUCTURE_DNA_PATH env must point to packed infrastructure.dna");
    //       let dna = SweetDnaFile::from_bundle(
    //           &std::path::PathBuf::from(dna_path)
    //       ).await.unwrap();
    //       let app = conductor.setup_app("test", &[dna]).await.unwrap();
    //       let (alice,) = app.into_tuple();
    //       let zome = alice.zome("infrastructure");
    //
    //       let status = PeerStatus {
    //           peer_id: alice.agent_pubkey().clone(),
    //           status: PeerLifecycleState::Online,
    //           flags: PeerCapabilityFlags {
    //               general_pool_member: true,
    //               accepting_stewardship_reserves: true,
    //           },
    //           archetype_class: Some("home-nuc".into()),
    //           timestamp: Timestamp::now(),
    //       };
    //
    //       let action_hash: ActionHash = conductor
    //           .call(&zome, "record_peer_status", status.clone())
    //           .await;
    //       assert!(!action_hash.as_hash().as_ref().is_empty());
    //
    //       let latest: Option<PeerStatus> = conductor
    //           .call(&zome, "get_latest_peer_status_for_agent", alice.agent_pubkey().clone())
    //           .await;
    //       assert_eq!(latest.unwrap().archetype_class.as_deref(), Some("home-nuc"));
    //   }

    panic!("placeholder — see module docs for SweetConductor harness plan");
}
