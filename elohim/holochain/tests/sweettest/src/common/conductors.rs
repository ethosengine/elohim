//! Conductor setup helpers.
//!
//! Wraps `SweetConductor` lifecycle so each test file doesn't redo the same
//! scaffolding. Expose single-agent and multi-agent constructors, both with
//! the bootstrap-steward modifier applied where a DNA needs one.

use anyhow::Result;
use holochain::sweettest::{SweetConductor, SweetConductorConfig, SweetDnaFile};
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
    let bundle = DnaBundle::read_from_file(&path).await?;

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
pub async fn single_agent_conductor() -> Result<(SweetConductor, AgentPubKey)> {
    let mut conductor = SweetConductor::from_config(SweetConductorConfig::standard()).await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    Ok((conductor, agent))
}

/// Spin up two conductors so tests can exercise cross-agent DHT behavior.
pub async fn two_agent_conductors() -> Result<[(SweetConductor, AgentPubKey); 2]> {
    let (c1, a1) = single_agent_conductor().await?;
    let (c2, a2) = single_agent_conductor().await?;
    Ok([(c1, a1), (c2, a2)])
}

// Re-export for tests that need direct access to the agent helper.
pub use holochain::sweettest::SweetAgents;
