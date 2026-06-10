//! Manifest-driven constitutional ratio registry.
//!
//! Reads the elohim domain manifest at startup (lazy via OnceLock) and exposes
//! `effective_ratios()` — the per-tier percentages clamped to DNA floor/ceiling
//! walls. Used by bounds_validator + replicates_dwelling_validator at
//! commitment-author time per spec §5.4.
//!
//! Source of truth: `elohim/sdk/domains/elohim/manifest.json` `constitutionalRatios` block.
//! Override the manifest path via `ELOHIM_MANIFEST_PATH` (tests).

use serde::Deserialize;
use std::sync::OnceLock;

// Mirror of DNA constants. WHY duplicate: bounds_validator runs in storage
// (native target); DNA constants live in WASM zone. Keep these synced with
// `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`.
pub const COMMONS_MIN_FLOOR_PCT: u8 = 10;
pub const COMMONS_MAX_CEILING_PCT: u8 = 60;
pub const DWELLING_MIN_FLOOR_PCT: u8 = 10;
pub const DWELLING_MAX_CEILING_PCT: u8 = 80;
pub const FREE_MIN_FLOOR_PCT: u8 = 5;
pub const FREE_MAX_CEILING_PCT: u8 = 70;

#[derive(Debug, Clone, Copy)]
pub struct EffectiveRatios {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
}

#[derive(Debug, Clone)]
pub struct EffectiveRatiosWithProvenance {
    pub ratios: EffectiveRatios,
    pub manifest_cid: String,
}

#[derive(Deserialize)]
struct RawConstitutionalRatios {
    #[serde(default = "default_commons")]
    commons_pct: u8,
    #[serde(default = "default_dwelling")]
    dwelling_pct: u8,
    // Deserialized for manifest-shape completeness but intentionally NOT read:
    // `collective` is always recomputed as the residual (100 − commons − dwelling
    // − free) so the tiers sum to 100 even when the manifest's collective_pct
    // disagrees. Kept as a field to document the manifest contract.
    #[serde(default = "default_collective")]
    #[allow(dead_code)]
    collective_pct: u8,
    #[serde(default = "default_free")]
    free_pct: u8,
}

fn default_commons() -> u8 {
    20
}
fn default_dwelling() -> u8 {
    40
}
fn default_collective() -> u8 {
    25
}
fn default_free() -> u8 {
    15
}

#[derive(Deserialize)]
struct ElohimManifest {
    #[serde(rename = "constitutionalRatios", default)]
    constitutional_ratios: Option<RawConstitutionalRatios>,
}

static REGISTRY: OnceLock<EffectiveRatiosWithProvenance> = OnceLock::new();

pub fn effective_ratios() -> EffectiveRatiosWithProvenance {
    REGISTRY.get_or_init(load_from_manifest).clone()
}

fn load_from_manifest() -> EffectiveRatiosWithProvenance {
    let manifest_path = std::env::var("ELOHIM_MANIFEST_PATH").unwrap_or_else(|_| {
        format!(
            "{}/../sdk/domains/elohim/manifest.json",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let manifest_cid = compute_manifest_cid(&manifest_path);
    let raw = match std::fs::read(&manifest_path) {
        Ok(b) => b,
        Err(_) => {
            return EffectiveRatiosWithProvenance {
                ratios: dna_default_ratios(),
                manifest_cid,
            };
        }
    };
    let parsed: ElohimManifest = serde_json::from_slice(&raw).unwrap_or(ElohimManifest {
        constitutional_ratios: None,
    });
    let raw = parsed
        .constitutional_ratios
        .unwrap_or(RawConstitutionalRatios {
            commons_pct: default_commons(),
            dwelling_pct: default_dwelling(),
            collective_pct: default_collective(),
            free_pct: default_free(),
        });
    let commons = raw
        .commons_pct
        .clamp(COMMONS_MIN_FLOOR_PCT, COMMONS_MAX_CEILING_PCT);
    let dwelling = raw
        .dwelling_pct
        .clamp(DWELLING_MIN_FLOOR_PCT, DWELLING_MAX_CEILING_PCT);
    let free = raw.free_pct.clamp(FREE_MIN_FLOOR_PCT, FREE_MAX_CEILING_PCT);
    // collective is the residual to make percentages sum to 100; if manifest's
    // collective_pct disagrees, the residual wins (substrate-correct).
    let collective = 100u8
        .saturating_sub(commons)
        .saturating_sub(dwelling)
        .saturating_sub(free);
    EffectiveRatiosWithProvenance {
        ratios: EffectiveRatios {
            commons_pct: commons,
            dwelling_pct: dwelling,
            collective_pct: collective,
            free_pct: free,
        },
        manifest_cid,
    }
}

fn dna_default_ratios() -> EffectiveRatios {
    EffectiveRatios {
        commons_pct: 20,
        dwelling_pct: 40,
        collective_pct: 25,
        free_pct: 15,
    }
}

fn compute_manifest_cid(path: &str) -> String {
    // Substrate-correct CID: hash the manifest bytes via the EPR cid module
    // (spec per-substrate-limitarian-governor-design §6.2 — the governed EPR
    // must be content-addressed-in-fact, not fingerprinted-by-path).
    match std::fs::read(path) {
        Ok(bytes) => elohim_epr::cid::compute_cid(&bytes).to_string(),
        Err(_) => format!("manifest-missing:{path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_ratios_sums_to_100() {
        let r = effective_ratios().ratios;
        assert_eq!(
            r.commons_pct as u16
                + r.dwelling_pct as u16
                + r.collective_pct as u16
                + r.free_pct as u16,
            100u16
        );
    }

    #[test]
    fn effective_ratios_within_dna_walls() {
        let r = effective_ratios().ratios;
        assert!(r.commons_pct >= COMMONS_MIN_FLOOR_PCT && r.commons_pct <= COMMONS_MAX_CEILING_PCT);
        assert!(
            r.dwelling_pct >= DWELLING_MIN_FLOOR_PCT && r.dwelling_pct <= DWELLING_MAX_CEILING_PCT
        );
        assert!(r.free_pct >= FREE_MIN_FLOOR_PCT && r.free_pct <= FREE_MAX_CEILING_PCT);
    }

    #[test]
    fn provenance_field_is_populated() {
        let p = effective_ratios();
        assert!(!p.manifest_cid.is_empty());
    }
}
