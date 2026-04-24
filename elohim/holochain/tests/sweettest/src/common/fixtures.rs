//! Shared fixtures — test data and path resolution.

use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Resolve the packaged `.dna` artifact for a given DNA name.
///
/// Looks first in the DNA workdir (per-DNA convention), then in the shared
/// happ workdir. Returns an error if neither exists so tests skip gracefully
/// when artifacts haven't been built.
pub fn dna_path(dna_name: &str) -> Result<PathBuf> {
    // Repo root is 3 parents up from `elohim/holochain/tests/sweettest/`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let holochain_dir = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow!("failed to resolve elohim/holochain root"))?;

    // Per-DNA convention: `dna/<name>/workdir/<name>.dna`
    let per_dna = holochain_dir
        .join("dna")
        .join(dna_name)
        .join("workdir")
        .join(format!("{dna_name}.dna"));
    if per_dna.exists() {
        return Ok(per_dna);
    }

    // Elohim happ workdir (lamad/infrastructure/imagodei/mishpat co-located there).
    let in_happ = holochain_dir
        .join("dna")
        .join("elohim")
        .join("workdir")
        .join(format!("{dna_name}.dna"));
    if in_happ.exists() {
        return Ok(in_happ);
    }

    // Standalone DNAs that pack with hyphenated names alongside their crate
    // (e.g., node-registry → dna/node-registry/node-registry.dna). Normalize
    // the underscore name to its hyphenated dir/file form.
    let hyphen_name = dna_name.replace('_', "-");
    let standalone_flat = holochain_dir
        .join("dna")
        .join(&hyphen_name)
        .join(format!("{hyphen_name}.dna"));
    if standalone_flat.exists() {
        return Ok(standalone_flat);
    }

    Err(anyhow!(
        "DNA artifact not found for '{dna_name}'. Run `hc dna pack` (or the \
         holochain build) before invoking sweettest. Searched: {:?}, {:?}, {:?}",
        per_dna,
        in_happ,
        standalone_flat
    ))
}

/// Standard network seed per DNA, matching `dna.yaml` defaults.
pub fn network_seed(dna_name: &str) -> String {
    format!("elohim_{dna_name}_alpha")
}

// =============================================================================
// node_registry fixtures
// =============================================================================

/// Local mirror of `node_registry_integrity::NodeRegistration`.
///
/// Field names must exactly match the integrity struct so that msgpack
/// round-tripping through the conductor works. No path-dep on the coordinator
/// crate is needed — sweettest serializes the input via serde and the zome
/// deserializes the same bytes.
///
/// Validator state (as of 2026-04-24): `validate_create_entry` returns
/// `ValidateCallbackResult::Valid` for all `NodeRegistration` entries
/// (the `_ => Ok(ValidateCallbackResult::Valid)` arm). The `signature` field
/// is never verified cryptographically — any non-empty string is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    // === IDENTITY ===
    pub node_id: String,
    pub agent_pub_key: String,
    pub display_name: String,

    // === CAPACITY ===
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_tb: f64,
    pub bandwidth_mbps: u32,

    // === LOCATION ===
    pub region: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // === CAPABILITIES ===
    pub zomes_hosted: Vec<String>,
    pub steward_tier: String,

    // === PARTICIPATION ===
    pub custodian_opt_in: bool,
    pub max_custody_gb: Option<f64>,
    pub max_bandwidth_mbps: Option<u32>,
    pub max_cpu_percent: Option<f64>,

    // === HEALTH ===
    pub uptime_percent: f64,
    pub last_heartbeat: String,

    // === METADATA ===
    pub registered_at: String,
    pub updated_at: String,

    // === STEWARDSHIP CLAIM ===
    pub claim_status: String,
    pub context_epr_id: Option<String>,

    // === PROOF ===
    /// Validator does not cryptographically verify this field —
    /// any non-empty placeholder passes.
    pub signature: String,
}

/// Construct a `NodeRegistration` with sane defaults for sweettest.
///
/// `node_id` is the unique identifier; `agent` is the Holochain agent key
/// stored as a string. All capacity/health fields are filled with minimal
/// but valid values that satisfy the coordinator's index links.
pub fn node_registration(node_id: &str, agent: &AgentPubKey) -> NodeRegistration {
    NodeRegistration {
        node_id: node_id.to_string(),
        agent_pub_key: agent.to_string(),
        display_name: format!("sweettest node {node_id}"),

        cpu_cores: 4,
        memory_gb: 8,
        storage_tb: 1.0,
        bandwidth_mbps: 100,

        region: "us-west".to_string(),
        latitude: None,
        longitude: None,

        zomes_hosted: vec!["node_registry_coordinator".to_string()],
        steward_tier: "caretaker".to_string(),

        custodian_opt_in: true,
        max_custody_gb: Some(100.0),
        max_bandwidth_mbps: Some(50),
        max_cpu_percent: Some(25.0),

        uptime_percent: 1.0,
        last_heartbeat: "0".to_string(),

        registered_at: "0".to_string(),
        updated_at: "0".to_string(),

        claim_status: "unclaimed".to_string(),
        context_epr_id: None,

        // Non-empty placeholder — validator does not verify cryptographically.
        signature: "sweettest-fixture-placeholder".to_string(),
    }
}
