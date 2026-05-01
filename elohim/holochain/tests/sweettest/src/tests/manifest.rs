//! Sweettest — Manifest entry type (EPR Phase 3, Task 7-8).
//!
//! These tests exercise the Manifest integrity entry type via the
//! content_store coordinator zome. They are `#[ignore]` until the DNA is
//! packed upstream by the Jenkins pipeline; remove `#[ignore]` once the
//! pipeline's pack-then-test stage is wired.
//!
//! Scenarios:
//!   1. `manifest_create_round_trip` — create_manifest + get_manifest round-trip
//!      for a valid pillar-projection manifest.
//!   2. `manifest_unknown_kind_rejected` — create_manifest rejects a Manifest
//!      whose manifest_kind is outside the bootstrap whitelist.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";

/// Mirror of `content_store_integrity::Manifest` (coordinator crate is WASM-only).
/// Field names and order MUST match the integrity struct exactly.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct Manifest {
    pub manifest_kind: String,
    pub pillar: Option<String>,
    pub payload_json: String,
    pub schema_ref: Option<String>,
    pub revision: u32,
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn manifest_create_round_trip() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("elohim-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let manifest = Manifest {
        manifest_kind: "pillar-projection".to_string(),
        pillar: Some("lamad".to_string()),
        payload_json: r#"{"pillar":"lamad","kinds":["Content"]}"#.to_string(),
        schema_ref: None,
        revision: 1,
    };
    let action_hash: holo_hash::ActionHash = conductor
        .call(
            &cell.zome("content_store"),
            "create_manifest",
            manifest.clone(),
        )
        .await;
    let fetched: Option<Manifest> = conductor
        .call(&cell.zome("content_store"), "get_manifest", action_hash)
        .await;
    let fetched = fetched.expect("manifest must round-trip");
    assert_eq!(fetched.manifest_kind, manifest.manifest_kind);
    assert_eq!(fetched.pillar, manifest.pillar);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn manifest_unknown_kind_rejected() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("elohim-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let bad = Manifest {
        manifest_kind: "this-is-not-a-valid-kind".to_string(),
        pillar: None,
        payload_json: "{}".to_string(),
        schema_ref: None,
        revision: 1,
    };
    let result = conductor
        .call_fallible::<_, holo_hash::ActionHash>(
            &cell.zome("content_store"),
            "create_manifest",
            bad,
        )
        .await;
    assert!(
        result.is_err(),
        "expected validation rejection for unknown manifest_kind"
    );
    Ok(())
}
