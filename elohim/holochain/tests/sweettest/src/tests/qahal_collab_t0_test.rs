//! Sweettest cross-DNA flows for T0 Collab end-to-end.
//! Per plan: 2026-05-23-multi-collective-collaboration-epr-plan.md
//!
//! Scenario: `create_collective_atomic_founder_membership`
//!   - Founding agent calls `create_collective`.
//!   - Returns an ActionHash for the Collective entry.
//!   - `get_collective_by_action` returns the Collective record.
//!   - `list_memberships_for_collective` returns exactly 1 Steward Membership
//!     (the founder, bootstrapped with synthetic sponsor_cid = "founder").
//!
//! Pattern mirrors portal_host_crud.rs / recovery_m4.rs:
//!   - `single_agent_conductor` + `load_dna("imagodei", ...)`.
//!   - Local mirror structs for I/O (no path-dep on WASM coordinator crate).
//!   - DNA artifact loaded from elohim/workdir/imagodei.dna via `load_dna`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holo_hash::ActionHash;

// ============================================================================
// Constants
// ============================================================================

const DNA: &str = "imagodei";
const ZOME: &str = "imagodei";

// ============================================================================
// Local I/O mirrors
//
// Field names must exactly match the coordinator structs so that msgpack
// round-tripping through the conductor is correct. Mirrors the pattern from
// portal_host_crud.rs — no path-dep on the WASM coordinator crate.
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateCollectiveInput {
    charter: String,
    display_name: String,
    salt: String,
}

// ============================================================================
// Tests
// ============================================================================

/// Verify that `create_collective` atomically creates both the Collective entry
/// and a founder Steward Membership in the same coordinator call.
///
/// This is the critical invariant from the M1 plan §2: the chicken-and-egg of
/// "who attests the first steward?" is resolved by bootstrapping the founder via
/// synthetic `sponsor_cid = "founder"` (the integrity validator accepts this).
#[tokio::test(flavor = "multi_thread")]
async fn create_collective_atomic_founder_membership() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    // Create a Collective — expect the ActionHash of the Collective entry back.
    let collective_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_collective",
            CreateCollectiveInput {
                charter: "We steward the watershed.".into(),
                display_name: "Watershed Stewards".into(),
                // 32 hex chars as required by the integrity validator
                salt: "0123456789abcdef0123456789abcdef".into(),
            },
        )
        .await;

    // Read back: Collective record must exist.
    let collective_record: Option<holochain_types::prelude::Record> = conductor
        .call(
            &cell.zome(ZOME),
            "get_collective_by_action",
            collective_hash.clone(),
        )
        .await;
    assert!(collective_record.is_some(), "Collective record must exist after create_collective");

    // Read back: exactly one Membership record (the founder Steward Membership).
    let memberships: Vec<holochain_types::prelude::Record> = conductor
        .call(
            &cell.zome(ZOME),
            "list_memberships_for_collective",
            collective_hash,
        )
        .await;
    assert_eq!(
        memberships.len(),
        1,
        "create_collective must atomically create exactly one founder Steward Membership; got {}",
        memberships.len()
    );

    Ok(())
}
