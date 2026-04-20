//! Sweettest — diverse household placement (Task 18).
//!
//! ## Canonical file location note
//!
//! The plan specifies `elohim/holochain/tests/diverse_placement.rs`.  There is
//! no Rust crate at `elohim/holochain/` (no `Cargo.toml` there — that tree is
//! documentation + DNA build manifests).  Per the plan's own fallback — "or
//! extend existing tests file" + "mirror existing sweettest patterns" — this
//! file lives in the established sweettest home:
//! `elohim/holochain/dna/infrastructure/zomes/infrastructure/tests/`.
//! The intent and assertions are identical to the plan spec.
//!
//! ## What this file covers
//!
//! 1. **Type-shape smoke test (runs by default).** Exercises the
//!    `infrastructure_integrity` types relevant to placement: `PeerStatus`,
//!    `PeerCapabilityFlags`, and `PeerLifecycleState`.  Confirms the fields
//!    and serialisation contract that the distribute_shards diversity selector
//!    depends on are stable.  No conductor required.
//!
//! 2. **Full live multi-conductor flow (marked `#[ignore]`).** Documents the
//!    intended sweettest shape: three conductors across two synthetic households
//!    (`home-alpha` × 2 agents, `home-beta` × 1 agent), ingest of a content
//!    item, and an assertion that `resilience_snapshot.stewarding_collectives ≥ 2`
//!    with no `contracts-short` gaps.
//!
//!    **Why `#[ignore]`:** The following infrastructure helpers are not yet
//!    available in any crate in this workspace:
//!
//!    - `common::spawn_cluster_with_humans(&[(agent_id, household_id)])` —
//!      spins N `SweetConductor` instances, each running the full `imagodei` +
//!      `infrastructure` app bundle, and registers the named humans with the
//!      given household groupings.  Requires the `holochain` crate
//!      (`SweetConductor`) as a dev-dependency; that crate is ~200 crates /
//!      several hundred MB and is deliberately excluded from WASM-targeted
//!      zome crates to keep build times manageable.
//!
//!    - `common::provide_commitment(agent_id, collective_id)` — calls the
//!      `imagodei` zome's `begin_stewardship` function on behalf of an agent
//!      to register an REA commitment for the given collective, satisfying the
//!      contract-aware selector's preconditions.
//!
//!    - `storage_clients[n].get_resilience_snapshot(content_id)` — HTTP GET
//!      against the running elohim-storage sidecar for the given conductor,
//!      returning a `ResilienceSnapshotView` (defined in `elohim-storage`
//!      `views.rs`).  Requires an HTTP client bound to the sidecar port
//!      allocated per conductor.
//!
//!    Until these are provided, the `distribute_shards` diversity coverage
//!    lives in `elohim/elohim-storage/tests/distribute_shards_diversity.rs`
//!    (pure-Rust, no conductor, covers gap recording) and
//!    `elohim/elohim-storage/tests/resilience_integration.rs` (covers the
//!    resilience snapshot HTTP response shape).
//!
//!    The Plan 5 chaos suite is the designated home for full live multi-
//!    conductor placement verification.  At that point:
//!    1. Add `holochain` + `tokio` to `[dev-dependencies]` in this crate's
//!       `Cargo.toml`.
//!    2. Add `common/mod.rs` with the three helper functions listed above.
//!    3. Delete `#[ignore]` from `diverse_household_placement_after_ingest`.
//!
//! Running:
//! ```text
//! # From elohim/holochain/dna/infrastructure/:
//! RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test diverse_placement
//! RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test diverse_placement -- --ignored --nocapture
//! ```

use infrastructure_integrity::{PeerCapabilityFlags, PeerLifecycleState, PeerStatus};

use hdk::prelude::{AgentPubKey, Timestamp};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_accepting_status(household_suffix: &str) -> PeerStatus {
    // Zeroed AgentPubKey — valid for serialisation tests, matches the
    // peer_status.rs pattern in this same tests/ directory.
    let raw_bytes = vec![0u8; 36];
    let peer_id = AgentPubKey::from_raw_36(raw_bytes);
    PeerStatus {
        peer_id,
        status: PeerLifecycleState::Online,
        flags: PeerCapabilityFlags {
            general_pool_member: true,
            accepting_stewardship_reserves: true,
        },
        archetype_class: Some(format!("home-{household_suffix}")),
        timestamp: Timestamp::from_micros(1_700_000_000_000_000),
    }
}

// ---------------------------------------------------------------------------
// Running test: type-shape smoke for placement-relevant fields
// ---------------------------------------------------------------------------

/// Verify that the PeerStatus fields used by the diverse-placement selector
/// are present, serialisable, and correctly typed.
///
/// The selector in `elohim-storage/src/services/peer_selection.rs` reads:
/// - `flags.accepting_stewardship_reserves` — gate for eligible peers
/// - `archetype_class` — used to group by household (household_id is stored
///   on the humans table; archetype_class carries the human-readable label
///   that the diversity score algorithm uses for bucket identification)
/// - `status` — `Online` / `Degraded` are both considered eligible;
///   `Leaving` is excluded
///
/// This test pins the serialisation contract for all three fields so that
/// elohim-storage can safely hardcode the strings in its projection and
/// selector logic.
#[test]
fn placement_relevant_fields_shape_and_serde() {
    let alpha1 = make_accepting_status("alpha");
    let alpha2 = make_accepting_status("alpha");
    let beta1  = make_accepting_status("beta");

    // accepting_stewardship_reserves must survive a round-trip — the storage
    // projection reads this from the JSON signal envelope.
    for peer in [&alpha1, &alpha2, &beta1] {
        let json = serde_json::to_string(peer).expect("serialise PeerStatus");
        let back: PeerStatus = serde_json::from_str(&json).expect("deserialise PeerStatus");
        assert!(
            back.flags.accepting_stewardship_reserves,
            "accepting_stewardship_reserves lost across serde round-trip"
        );
        assert_eq!(
            back.status.to_string(),
            "online",
            "PeerLifecycleState::Online must serialise to 'online'"
        );
    }

    // archetype_class distinguishes households in the diversity algorithm.
    assert_eq!(alpha1.archetype_class.as_deref(), Some("home-alpha"));
    assert_eq!(beta1.archetype_class.as_deref(), Some("home-beta"));

    // Two peers with the same archetype count as one household bucket.
    assert_eq!(
        alpha1.archetype_class,
        alpha2.archetype_class,
        "both alpha peers must share the same archetype_class (same household bucket)"
    );

    // Leaving peers are excluded from selection — verify the string.
    let mut leaving = make_accepting_status("gamma");
    leaving.status = PeerLifecycleState::Leaving;
    leaving.flags.accepting_stewardship_reserves = false;
    assert_eq!(leaving.status.to_string(), "leaving");
    assert!(!leaving.flags.accepting_stewardship_reserves);
}

/// Verify PeerLifecycleState variants relevant to the placement eligibility
/// check: Online and Degraded are eligible; Leaving is not.
#[test]
fn placement_eligibility_lifecycle_variants() {
    assert_eq!(PeerLifecycleState::Online.to_string(), "online");
    assert_eq!(PeerLifecycleState::Degraded.to_string(), "degraded");
    assert_eq!(PeerLifecycleState::Leaving.to_string(), "leaving");
    assert_eq!(PeerLifecycleState::Maintenance.to_string(), "maintenance");
    assert_eq!(PeerLifecycleState::Starting.to_string(), "starting");
}

// ---------------------------------------------------------------------------
// Ignored test: full live multi-conductor sweettest
// ---------------------------------------------------------------------------

/// Full sweettest: three conductors across two synthetic households.
///
/// **Not runnable today — see module-level docs for the three missing helpers.**
///
/// Run once Plan 5 adds the SweetConductor harness:
/// ```text
/// RUSTFLAGS='--cfg getrandom_backend="custom"' \
///   cargo test --test diverse_placement -- \
///   diverse_household_placement_after_ingest --ignored --nocapture
/// ```
///
/// ## Missing infrastructure (tracked for Plan 5)
///
/// 1. `common::spawn_cluster_with_humans` — SweetConductor-based multi-node
///    setup with household grouping.  Requires `holochain` dev-dependency.
/// 2. `common::provide_commitment` — calls `imagodei::begin_stewardship` per
///    agent to satisfy the contract-aware selector.
/// 3. `storage_clients[n].get_resilience_snapshot(content_id)` — HTTP client
///    bound to the sidecar port for the nth conductor.
#[test]
#[ignore = "requires SweetConductor harness + three missing helpers — see module docs; tracked for Plan 5 chaos suite"]
fn diverse_household_placement_after_ingest() {
    // Intended async body once the harness is available:
    //
    // #[tokio::test(flavor = "multi_thread")]
    // async fn diverse_household_placement_after_ingest() {
    //     let (_conductors, _apps, storage_clients) =
    //         common::spawn_cluster_with_humans(&[
    //             ("agent-alpha-1", "home-alpha"),
    //             ("agent-alpha-2", "home-alpha"),
    //             ("agent-beta-1",  "home-beta"),
    //         ]).await;
    //
    //     for agent in ["agent-alpha-1", "agent-alpha-2", "agent-beta-1"] {
    //         common::provide_commitment(agent, "commons").await;
    //     }
    //
    //     let content_id = "content-diverse-x";
    //     storage_clients[0]
    //         .ingest(&common::sample_content(content_id, "commons"))
    //         .await
    //         .unwrap();
    //
    //     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    //
    //     let snap = storage_clients[0]
    //         .get_resilience_snapshot(content_id)
    //         .await
    //         .unwrap();
    //     assert!(
    //         snap.stewarding_collectives >= 2,
    //         "expected ≥ 2 stewarding collectives, got {}",
    //         snap.stewarding_collectives
    //     );
    //     assert!(
    //         snap.placement_gaps.is_empty()
    //             || snap.placement_gaps.iter().all(|g| g.gap_kind != "contracts-short"),
    //         "unexpected contracts-short gap after successful diverse placement"
    //     );
    // }

    panic!("placeholder — see module docs for Plan 5 SweetConductor harness plan");
}
