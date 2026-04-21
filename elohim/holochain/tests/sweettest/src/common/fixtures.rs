//! Shared fixtures — test data and path resolution.

use anyhow::{anyhow, Result};
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

    Err(anyhow!(
        "DNA artifact not found for '{dna_name}'. Run `hc dna pack` (or the \
         holochain build) before invoking sweettest. Searched: {:?}, {:?}",
        per_dna,
        in_happ
    ))
}

/// Standard network seed per DNA, matching `dna.yaml` defaults.
pub fn network_seed(dna_name: &str) -> String {
    format!("elohim_{dna_name}_alpha")
}
