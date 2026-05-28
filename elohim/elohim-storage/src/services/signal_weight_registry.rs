//! Manifest-driven signal-weight registry.
//!
//! Reads the elohim domain manifest at startup (lazy via `OnceLock`) and
//! exposes `weight_for(signal_kind) -> Option<SignalWeight>`. The
//! `standing_projector` consumes weights from here when projecting
//! FeedbackSignals into the `standing_view` table.
//!
//! Source of truth: `elohim/sdk/domains/elohim/manifest.json` (the elohim
//! domain manifest's `signalKinds` map). Override the manifest path via the
//! `ELOHIM_MANIFEST_PATH` environment variable (tests; deployments where the
//! manifest lives outside the default path).
//!
//! Signal kinds that lack BOTH `debit_weight` and `decay_days` fields (e.g.,
//! `forget-request`, which is advisory-only) return None from `weight_for`.
//! The standing projector treats None as "no standing impact" and logs a warn
//! (per Sprint 2 Task 8).

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Per-signal-kind weight policy.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalWeight {
    pub debit_weight: i32,
    pub decay_days: u32,
}

/// Raw manifest signal_kind entry. Only the weight-related fields are
/// captured here; the projector doesn't need description / target_kinds /
/// standing_impact_allowed at runtime.
#[derive(Debug, Clone, Deserialize)]
struct RawSignalKindEntry {
    #[serde(default)]
    debit_weight: Option<i32>,
    #[serde(default)]
    decay_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ElohimManifest {
    #[serde(rename = "signalKinds", default)]
    signal_kinds: HashMap<String, RawSignalKindEntry>,
}

static REGISTRY: OnceLock<HashMap<String, SignalWeight>> = OnceLock::new();

/// Return the weight policy for a signal_kind, or None if the manifest does
/// not declare both `debit_weight` and `decay_days` for that kind.
pub fn weight_for(signal_kind: &str) -> Option<SignalWeight> {
    REGISTRY
        .get_or_init(load_weights_from_manifest)
        .get(signal_kind)
        .cloned()
}

fn load_weights_from_manifest() -> HashMap<String, SignalWeight> {
    // CARGO_MANIFEST_DIR resolves to the elohim-storage crate root at compile
    // time; the manifest lives one level up under sdk/domains/elohim/.
    let default_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sdk/domains/elohim/manifest.json"
    );
    let manifest_path =
        std::env::var("ELOHIM_MANIFEST_PATH").unwrap_or_else(|_| default_path.to_owned());

    let bytes = match std::fs::read(&manifest_path) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::signal_weight_registry",
                path = %manifest_path,
                error = %e,
                "failed to read elohim manifest; signal_weight_registry will return None for all signal_kinds"
            );
            return HashMap::new();
        }
    };

    let manifest: ElohimManifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::signal_weight_registry",
                error = %e,
                "failed to parse elohim manifest signalKinds; signal_weight_registry will return None for all signal_kinds"
            );
            return HashMap::new();
        }
    };

    manifest
        .signal_kinds
        .into_iter()
        .filter_map(|(name, raw)| {
            let (debit_weight, decay_days) = (raw.debit_weight?, raw.decay_days?);
            Some((name, SignalWeight { debit_weight, decay_days }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_weight_for_known_signal() {
        let w = weight_for("rate-limit-exceeded")
            .expect("known signal_kind has weight");
        assert!(
            w.debit_weight > 0,
            "debit_weight should be positive for rate-limit-exceeded"
        );
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        assert!(weight_for("nonexistent-signal").is_none());
    }

    #[test]
    fn forget_request_has_no_weight() {
        // forget-request is advisory-only; it must not carry standing impact.
        assert!(
            weight_for("forget-request").is_none(),
            "forget-request should have no weight in the registry"
        );
    }

    #[test]
    fn bad_custody_has_highest_weight() {
        let w = weight_for("bad-custody").expect("bad-custody has weight");
        assert_eq!(w.debit_weight, 20, "bad-custody debit_weight should be 20");
        assert_eq!(w.decay_days, 90, "bad-custody decay_days should be 90");
    }

    #[test]
    fn compute_breach_weight_and_decay() {
        let w = weight_for("compute-breach").expect("compute-breach has weight");
        assert_eq!(w.debit_weight, 10);
        assert_eq!(w.decay_days, 60);
    }
}
