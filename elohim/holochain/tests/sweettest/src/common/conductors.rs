//! Conductor setup helpers.
//!
//! Wraps `SweetConductor` lifecycle so each test file doesn't redo the same
//! scaffolding. Expose single-agent and multi-agent constructors, both with
//! the bootstrap-steward modifier applied where a DNA needs one.

use anyhow::Result;
use holochain::sweettest::{
    DynSweetRendezvous, SweetConductor, SweetConductorConfig, SweetDnaFile, SweetLocalRendezvous,
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
///
/// 0.7 — the pair shares ONE rendezvous, for the same reason
/// [`two_agent_conductors_isolated`] does (see its note): `SweetConductor::standard()`
/// mints a fresh `SweetLocalRendezvous` per call, and kitsune2 0.5 dials only
/// peers whose advertised relay matches its own exactly. Two `standard()`
/// conductors therefore never connect — elohim-holochain #1424 (2026-09-03):
/// `infrastructure::doorway_visible_across_agents_and_operator_only_can_update`
/// ("a2 should be able to read a1's doorway within 30s") and
/// `qahal_formation_test::affirm_membership_happy_path_then_replay_rejected`
/// ("DHT consistency timeout") both red on 0.7, both green on 0.6 where
/// `standard()` carried no relay at all. Discovery stays the 0.6 default (mem
/// bootstrap on), so peers still find each other without `exchange_peer_info`.
pub async fn two_agent_conductors() -> Result<[(SweetConductor, AgentPubKey); 2]> {
    let rendezvous = SweetLocalRendezvous::new().await;
    let (c1, a1) = single_agent_conductor_with(rendezvous.clone()).await?;
    let (c2, a2) = single_agent_conductor_with(rendezvous).await?;
    Ok([(c1, a1), (c2, a2)])
}

/// One `standard()` conductor homed to a caller-supplied rendezvous, so a
/// multi-conductor test can put every peer on the same relay.
pub async fn single_agent_conductor_with(
    rendezvous: DynSweetRendezvous,
) -> Result<(SweetConductor, AgentPubKey)> {
    let config = SweetConductorConfig::standard();
    let mut conductor = SweetConductor::from_config_rendezvous(config, rendezvous).await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    Ok((conductor, agent))
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
///
/// 0.7 — WHY THE PAIR SHARES ONE RENDEZVOUS. The isolation this helper provides
/// is DISCOVERY isolation, NOT TRANSPORT isolation. At 0.6 that distinction was
/// free: `standard()` left the network config at `NetworkConfig::default()`, so
/// there was no relay and peers simply connected directly once
/// `exchange_peer_info` had injected each other's agent info. At 0.7,
/// `SweetConductorConfig::standard()` sets `bootstrap_url = relay_url =
/// "rendezvous:"`, and each `SweetLocalRendezvous` instance is a DIFFERENT
/// relay. Giving each conductor its own instance therefore homes the two peers
/// to two different relays: `exchange_peer_info` still injects peer info, but
/// no transport path exists between them and gossip never flows.
///
/// Measured (2026-09-03, holochain 0.7.0, one rendezvous per conductor):
///   rea_commitment_replication.rs:234 `await_consistency_s(60, ..)` and
///   lamad.rs:1024 both failed with "Consistency not reached" — 0 in validation
///   limbo and 0 in integration limbo on BOTH sides (21 integrated vs 7), i.e.
///   not slow validation but no connectivity at all. Both pass discovery
///   isolation and fail transport.
///
/// So: ONE rendezvous is created here and shared by the pair (bootstrap+relay
/// infrastructure in common), while `disable_bootstrap = true` on each
/// conductor keeps discovery partitioned until the explicit
/// `exchange_peer_info`. That restores the 0.6 invariant exactly.
pub async fn two_agent_conductors_isolated() -> Result<[(SweetConductor, AgentPubKey); 2]> {
    // ONE rendezvous, shared: see the transport-vs-discovery note above.
    let rendezvous = SweetLocalRendezvous::new().await;
    let (c1, a1) = single_agent_conductor_isolated(rendezvous.clone()).await?;
    let (c2, a2) = single_agent_conductor_isolated(rendezvous).await?;
    Ok([(c1, a1), (c2, a2)])
}

/// Single conductor with the bootstrap module disabled, joined to the rendezvous
/// the caller supplies. See [`two_agent_conductors_isolated`] for why the
/// rendezvous is a parameter rather than constructed here.
async fn single_agent_conductor_isolated(
    rendezvous: DynSweetRendezvous,
) -> Result<(SweetConductor, AgentPubKey)> {
    let config = SweetConductorConfig::standard().tune_network_config(|nc| {
        nc.disable_bootstrap = true;
    });
    let mut conductor = SweetConductor::from_config_rendezvous(config, rendezvous).await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    Ok((conductor, agent))
}

// Re-export for tests that need direct access to the agent helper.
pub use holochain::sweettest::SweetAgents;

/// Load a DNA bundle from an EXPLICIT path, with caller-supplied properties.
///
/// [`load_dna`] resolves a DNA by name through [`dna_path`] and can only encode
/// the bootstrap-steward property shape. Lineage tests need two DIFFERENT
/// artifacts of the same DNA (a predecessor copied aside before an integrity
/// change, and the freshly built successor) and need to declare arbitrary
/// properties — notably `lineage`, which folds into the successor's DNA hash.
///
/// Properties are passed as already-encoded [`SerializedBytes`] so the caller
/// owns the shape; `None` leaves the bundle's own properties in place.
pub async fn load_dna_from_path(
    path: &std::path::Path,
    network_seed: &str,
    properties: Option<SerializedBytes>,
) -> Result<DnaFile> {
    let bytes = std::fs::read(path)?;
    let bundle = DnaBundle::unpack(bytes.as_slice())?;
    let modifiers = DnaModifiersOpt {
        network_seed: Some(network_seed.to_string()),
        properties,
    };
    let (dna_file, _dna_hash) = bundle.into_dna_file(modifiers).await?;
    Ok(dna_file)
}
