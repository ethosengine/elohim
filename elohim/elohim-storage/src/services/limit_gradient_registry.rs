//! LimitGradientRegistry — the effective gradient for (substrate, layer):
//! core value-laden defaults, DNA-wall-clamped (spec §6.2). The registry clamps
//! its OWN DEFAULT OUTPUT only; ratified values are wall-checked at
//! create_commitment by the DNA validator (§5.2 — reject-at-write, never
//! silently clamp a ratified truth). Ratified overrides arrive via the
//! responsibility_demand_configs projection once the writeback lands (Task 8);
//! v1's effective lookup: ratified row if ratified_by is set, else core default.
//!
//! WALL WIDTHS ARE TBD-OPERATOR (spec §Decision 2): the SHAPE of each wall is
//! decided; the numerics below are asserted defaults awaiting derivation.

use crate::services::token_decay_service::GradientConfig;

// DNA-wall mirror (native side). Keep in lockstep with mishpat wall constants
// (Task 7) — the validator there is authoritative at write time.
pub const ALPHA_WALL: (f32, f32) = (1.0, 2.0); // cannot blind the tail (α=0 forbidden)
pub const C_TARGET_WALL: (f32, f32) = (0.05, 0.30); // TBD-operator
pub const K_MAX_WALL: (f32, f32) = (0.01, 0.10); // TBD-operator
pub const BASE_RATE_WALL: (f32, f32) = (0.0005, 0.005); // TBD-operator
pub const GAMMA_WALL: (f32, f32) = (0.5, 2.0); // TBD-operator

pub struct LimitGradientRegistry;

impl LimitGradientRegistry {
    /// Core value-laden default for a substrate/layer, wall-clamped.
    /// v1: layer-defaulted alpha (small-N household → 1.0; community+ → 2.0).
    pub fn core_default(_substrate_signal: &str, governance_layer: &str) -> GradientConfig {
        let mut g = GradientConfig::default();
        g.alpha = match governance_layer {
            "individual" | "household" => 1.0,
            _ => 2.0,
        };
        Self::clamp_to_walls(g)
    }

    pub fn clamp_to_walls(mut g: GradientConfig) -> GradientConfig {
        g.alpha = g.alpha.clamp(ALPHA_WALL.0, ALPHA_WALL.1);
        g.c_target = g.c_target.clamp(C_TARGET_WALL.0, C_TARGET_WALL.1);
        g.k_max = g.k_max.clamp(K_MAX_WALL.0, K_MAX_WALL.1);
        g.base_rate = g.base_rate.clamp(BASE_RATE_WALL.0, BASE_RATE_WALL.1);
        g.gamma = g.gamma.clamp(GAMMA_WALL.0, GAMMA_WALL.1);
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_in_wall() {
        for layer in ["household", "community", "bioregional"] {
            let g = LimitGradientRegistry::core_default("attention", layer);
            assert!(g.alpha >= ALPHA_WALL.0 && g.alpha <= ALPHA_WALL.1);
            assert!(g.c_target >= C_TARGET_WALL.0 && g.c_target <= C_TARGET_WALL.1);
            assert!(g.k_max >= K_MAX_WALL.0 && g.k_max <= K_MAX_WALL.1);
            assert!(g.base_rate >= BASE_RATE_WALL.0 && g.base_rate <= BASE_RATE_WALL.1);
            assert!(g.gamma >= GAMMA_WALL.0 && g.gamma <= GAMMA_WALL.1);
        }
    }

    #[test]
    fn layer_defaulted_alpha() {
        assert_eq!(
            LimitGradientRegistry::core_default("attention", "household").alpha,
            1.0
        );
        assert_eq!(
            LimitGradientRegistry::core_default("attention", "community").alpha,
            2.0
        );
    }

    #[test]
    fn clamp_pulls_out_of_wall_values_in() {
        let mut wild = GradientConfig::default();
        wild.alpha = 0.0; // tail-blinding attempt
        wild.k_max = 1.0; // confiscate-everything attempt
        let clamped = LimitGradientRegistry::clamp_to_walls(wild);
        assert_eq!(clamped.alpha, ALPHA_WALL.0);
        assert_eq!(clamped.k_max, K_MAX_WALL.1);
    }
}
