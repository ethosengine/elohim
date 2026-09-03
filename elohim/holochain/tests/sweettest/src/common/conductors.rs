//! Conductor setup helpers.
//!
//! Wraps `SweetConductor` lifecycle so each test file doesn't redo the same
//! scaffolding. Expose single-agent and multi-agent constructors, both with
//! the bootstrap-steward modifier applied where a DNA needs one.

use anyhow::Result;
use holochain::sweettest::{
    SweetConductor, SweetConductorConfig, SweetDnaFile, SweetLocalRendezvous,
};
use holochain_types::prelude::*;
use std::path::PathBuf;

use crate::common::fixtures::dna_path;

/// Load a DNA bundle from disk, applying a network seed and optional
/// bootstrap-steward pubkey as modifier overrides.
///
/// Callers pass the DNA name (e.g., "imagodei") and the workspace resolves
/// the packaged `.dna` path via [`dna_path`]. If the file is missing, this
/// returns an error — tests should mark themselves ignored when the DNA
/// artifact isn't built.
pub async fn load_dna(
    dna_name: &str,
    network_seed: &str,
    bootstrap_steward: Option<AgentPubKey>,
) -> Result<DnaFile> {
    let path: PathBuf = dna_path(dna_name)?;
    let bytes = std::fs::read(&path)?;
    let bundle = DnaBundle::unpack(bytes.as_slice())?;

    let mut properties: Option<SerializedBytes> = None;
    if let Some(pubkey) = bootstrap_steward {
        // The DnaProperties shape is `{ progenitor_pubkey: String }` in every
        // bootstrap-steward DNA. Encode it here so tests don't need to know
        // the field name.
        #[derive(serde::Serialize, serde::Deserialize, SerializedBytes, Debug)]
        struct Props {
            progenitor_pubkey: String,
        }
        let props = Props {
            progenitor_pubkey: pubkey.to_string(),
        };
        properties = Some(SerializedBytes::try_from(props)?);
    }

    let modifiers = DnaModifiersOpt {
        network_seed: Some(network_seed.to_string()),
        properties,
    };

    let (dna_file, _dna_hash) = bundle.into_dna_file(modifiers).await?;
    Ok(dna_file)
}

/// Spin up a single-conductor/single-agent test harness.
///
/// Returns a conductor plus the freshly-generated agent pubkey. The caller
/// can then install any DNA with that agent, using [`load_dna`] to build
/// the DnaFile first.
// 0.7 MIGRATION NOTE — `SweetConductor::from_config` was REMOVED, and this is
// not a drop-in rename. At 0.6, `SweetConductorConfig::standard()` left
// `bootstrap_url`/`signal_url` at `NetworkConfig::default()` and carried no
// rendezvous, so `from_config` resolved `get_rendezvous() == None` and built a
// conductor with NO rendezvous server. At 0.7, `standard()` itself sets
// `bootstrap_url = relay_url = "rendezvous:"` and `get_rendezvous()` is gone,
// so a rendezvous instance MUST be supplied or those URLs never resolve.
// The 0.7 shape below is therefore a behaviour change, not a transcription:
// discovery now runs through a real local rendezvous server.
pub async fn single_agent_conductor() -> Result<(SweetConductor, AgentPubKey)> {
    let mut conductor = SweetConductor::standard().await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    Ok((conductor, agent))
}

/// Spin up two conductors so tests can exercise cross-agent DHT behavior.
pub async fn two_agent_conductors() -> Result<[(SweetConductor, AgentPubKey); 2]> {
    let (c1, a1) = single_agent_conductor().await?;
    let (c2, a2) = single_agent_conductor().await?;
    Ok([(c1, a1), (c2, a2)])
}

/// Spin up two conductors with the bootstrap module DISABLED, for partition
/// tests that must author under genuine isolation before an explicit heal.
///
/// Why this exists: `SweetConductorConfig::standard()` leaves `mem_bootstrap:
/// true, disable_bootstrap: false`. Kitsune2's in-memory bootstrap store is a
/// process-global `HashMap<(test_id, space_id), _>` where `test_id` defaults to
/// the *thread id at conductor-construction time* and `space_id` is the DNA
/// hash (network-seed-derived). Under `#[tokio::test(flavor = "multi_thread")]`
/// two conductors built in the same test frequently land on the same worker
/// thread and thus SHARE that store for their common space — so the second
/// conductor's bootstrap poll (which runs one iteration immediately at startup)
/// discovers the first's agent info and begins gossiping BEFORE the test's
/// author phase. The intended "pre-exchange partition" never exists: the second
/// peer sees the first's content/links (duplicate-id collisions, premature
/// earned-head visibility) even though `exchange_peer_info` has not been called.
///
/// Setting `disable_bootstrap = true` turns the bootstrap module off entirely,
/// so the conductors can ONLY learn about each other through an explicit
/// `SweetConductor::exchange_peer_info` (direct peer-store injection). Gossip
/// and publish stay enabled (separate flags), so a post-exchange
/// `await_consistency` still converges — the canonical partition-then-heal
/// idiom. See kitsune2_core `factories/mem_bootstrap.rs` for the shared-store
/// keying that makes the default non-isolating.
pub async fn two_agent_conductors_isolated() -> Result<[(SweetConductor, AgentPubKey); 2]> {
    let (c1, a1) = single_agent_conductor_isolated().await?;
    let (c2, a2) = single_agent_conductor_isolated().await?;
    Ok([(c1, a1), (c2, a2)])
}

/// Single conductor with the bootstrap module disabled. See
/// [`two_agent_conductors_isolated`] for the isolation rationale.
async fn single_agent_conductor_isolated() -> Result<(SweetConductor, AgentPubKey)> {
    let config = SweetConductorConfig::standard().tune_network_config(|nc| {
        nc.disable_bootstrap = true;
    });
    // 0.7: see the migration note on `single_agent_conductor`. `standard()` now
    // points bootstrap/relay at `rendezvous:`, so a rendezvous server must be
    // supplied even here; `disable_bootstrap = true` is what still keeps the
    // pair partitioned until an explicit `exchange_peer_info`. Each isolated
    // conductor gets its OWN rendezvous instance, so there is no shared
    // discovery surface between them at all.
    let mut conductor =
        SweetConductor::from_config_rendezvous(config, SweetLocalRendezvous::new().await).await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    Ok((conductor, agent))
}

// Re-export for tests that need direct access to the agent helper.
pub use holochain::sweettest::SweetAgents;
