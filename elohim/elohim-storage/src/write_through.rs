//! Write-through flag state — EPR Phase 2B Task C.4.
//!
//! Implements the 4-layer override stack for per-pillar write-through to EPR
//! storage. When write-through is ON for a (pillar, kind), signals composed via
//! `/api/v1/signal/emit` are signed, ingested as EPR atoms, and projected into
//! the relational view tables. When OFF, the signal-emit endpoint returns 503
//! for non-integrity kinds (per memory pin
//! `project_epr2b_recovery_m4_convergence` — Recovery M4 integrity always
//! writes through, even on cold pillars).
//!
//! ## The 4-layer override stack
//!
//! Per memory pin `project_cadence_archetype_tunable_with_dev_overrides`,
//! every operator-tunable knob in the protocol exposes the same 4 layers:
//!
//! | Layer | Source | Set in |
//! |-------|--------|--------|
//! | 1 | Manifest default | App manifest's `writeThrough` block (per pillar) |
//! | 2 | Policy.toml override | Operator-level override file (Task C.6) |
//! | 3 | Env/CLI override | `--write-through-pillars=shefa` etc. (Task C.6) |
//! | 4 | Admin override | Live admin trigger via API (Task C.6) |
//!
//! Effective state = highest-precedence layer that has an opinion. Layers 2-4
//! are stored as `Option<WriteThroughOverride>` so an empty layer is silent.
//!
//! ## Integrity exception
//!
//! Per memory pin `project_epr2b_recovery_m4_convergence`, integrity-bearing
//! kinds (key rotation, key revocation, peer binding, revocation attestation)
//! must write through unconditionally so DNA-attested membership/key state
//! never lags the projection. `is_integrity_kind()` returns true for these
//! and `effective_for()` short-circuits to `OnIntegrityException`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/// Per-pillar write-through declaration as it appears in the app manifest's
/// `writeThrough` block (Task C.4 schema). Mirrors the JSON Schema in
/// `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` ($defs.WriteThrough).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteThroughConfig {
    /// Whether this pillar's signals are written through by default.
    pub enabled: bool,
    /// Optional restriction to a subset of EPR kinds. None or empty = all kinds.
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
}

impl WriteThroughConfig {
    /// Apply this config to a specific kind. Returns true iff write-through is on
    /// AND the kind is in scope (or scope is unrestricted).
    pub fn applies_to(&self, kind: &str) -> bool {
        if !self.enabled {
            return false;
        }
        match &self.kinds {
            None => true,
            Some(list) if list.is_empty() => true,
            Some(list) => list.iter().any(|k| k == kind),
        }
    }
}

/// An override at layers 2-4 (policy.toml, env/CLI, admin). When set, it
/// supersedes lower layers. Mirrors the manifest config shape but per-pillar
/// and dynamically composed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteThroughOverride {
    /// Per-pillar overrides. Pillars not listed inherit from lower layers.
    pub pillars: HashMap<String, WriteThroughConfig>,
}

impl WriteThroughOverride {
    pub fn empty() -> Self {
        Self {
            pillars: HashMap::new(),
        }
    }

    pub fn for_pillar(&self, pillar: &str) -> Option<&WriteThroughConfig> {
        self.pillars.get(pillar)
    }
}

/// The layer that produced an effective decision. Reported by the
/// `/api/v1/status/write-through` endpoint so operators can see which override
/// is in force.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteThroughSource {
    /// Layer 4 — live admin trigger via API.
    AdminOverride,
    /// Layer 3 — env/CLI flag.
    EnvOverride,
    /// Layer 2 — policy.toml.
    PolicyOverride,
    /// Layer 1 — pillar manifest's `writeThrough` block.
    ManifestDefault,
    /// No layer expressed an opinion; effective state is OFF by default for
    /// non-integrity kinds.
    ImplicitDefault,
    /// Hardcoded integrity exception — bypasses all layers.
    IntegrityException,
}

/// The effective write-through state for a (pillar, kind). Returned from
/// `WriteThroughState::effective_for`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum EffectiveWriteThrough {
    /// Write-through is on. The signal will be signed + ingested + projected.
    On { source: WriteThroughSource },
    /// Write-through is off. The signal-emit endpoint returns 503.
    Off { source: WriteThroughSource },
    /// Write-through is on by integrity exception, regardless of layers.
    OnIntegrityException,
}

impl EffectiveWriteThrough {
    /// True if this effective state writes through.
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On { .. } | Self::OnIntegrityException)
    }

    /// The source layer that produced this decision.
    pub fn source(&self) -> WriteThroughSource {
        match self {
            Self::On { source } | Self::Off { source } => *source,
            Self::OnIntegrityException => WriteThroughSource::IntegrityException,
        }
    }
}

/// Hardcoded list of integrity-bearing EPR kinds that always write through,
/// independent of the 4-layer flag stack.
///
/// Per memory pin `project_epr2b_recovery_m4_convergence`, these kinds carry
/// DNA-attested membership/key state that must be projected eagerly — the
/// reconcile sweep cannot tolerate cold-pillar lag for these.
pub fn is_integrity_kind(kind: &str) -> bool {
    matches!(
        kind,
        "KeyRotation" | "KeyRevocation" | "RevocationAttestation" | "AgentPeerBinding"
    )
}

// ---------------------------------------------------------------------------
// Composed state
// ---------------------------------------------------------------------------

/// Composed write-through state across all 4 override layers.
///
/// Constructed at startup with the manifest defaults (layer 1) and any policy
/// or env overrides resolved at boot (layers 2-3). Layer 4 (admin overrides)
/// is mutable via `set_admin_override` and protected by an `RwLock` so the
/// admin endpoint can mutate without taking a service-wide write lock.
pub struct WriteThroughState {
    /// Layer 1 — manifest defaults, keyed by pillar name.
    manifest: HashMap<String, WriteThroughConfig>,
    /// Layer 2 — policy.toml overrides.
    policy: Option<WriteThroughOverride>,
    /// Layer 3 — env/CLI overrides.
    env: Option<WriteThroughOverride>,
    /// Layer 4 — live admin overrides (mutable).
    admin: Arc<RwLock<Option<WriteThroughOverride>>>,
}

impl WriteThroughState {
    /// Build state from manifest defaults only. Layers 2-4 start empty.
    pub fn from_manifest(manifest: HashMap<String, WriteThroughConfig>) -> Self {
        Self {
            manifest,
            policy: None,
            env: None,
            admin: Arc::new(RwLock::new(None)),
        }
    }

    /// Build empty state (no manifest, no overrides). Useful for tests and as a
    /// null object when no manifest is loaded — every kind resolves to
    /// `Off { source: ImplicitDefault }` except integrity kinds.
    pub fn empty() -> Self {
        Self::from_manifest(HashMap::new())
    }

    /// Set the policy.toml override layer (layer 2).
    pub fn with_policy_override(mut self, ov: WriteThroughOverride) -> Self {
        self.policy = Some(ov);
        self
    }

    /// Set the env/CLI override layer (layer 3).
    pub fn with_env_override(mut self, ov: WriteThroughOverride) -> Self {
        self.env = Some(ov);
        self
    }

    /// Replace the live admin override (layer 4). Pass `None` to clear.
    pub fn set_admin_override(&self, ov: Option<WriteThroughOverride>) {
        // Poisoned-lock recovery: a poisoned RwLock here would mean a previous
        // panic mid-write. We accept the inner state and continue rather than
        // wedging the admin endpoint.
        let mut guard = self
            .admin
            .write()
            .unwrap_or_else(|poison| poison.into_inner());
        *guard = ov;
    }

    /// Return a snapshot of the current admin override (layer 4) for status views.
    pub fn admin_snapshot(&self) -> Option<WriteThroughOverride> {
        self.admin
            .read()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    /// Compute the effective write-through state for a (pillar, kind).
    ///
    /// Resolution order:
    ///   0. Integrity-kind short-circuit → `OnIntegrityException`.
    ///   1. Layer 4 (admin) — if it has a per-pillar entry, it wins.
    ///   2. Layer 3 (env/CLI) — same.
    ///   3. Layer 2 (policy.toml) — same.
    ///   4. Layer 1 (manifest) — same.
    ///   5. Implicit default → `Off { source: ImplicitDefault }`.
    pub fn effective_for(&self, pillar: &str, kind: &str) -> EffectiveWriteThrough {
        if is_integrity_kind(kind) {
            return EffectiveWriteThrough::OnIntegrityException;
        }

        // Layer 4 — admin override.
        let admin_guard = self
            .admin
            .read()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(ov) = admin_guard.as_ref() {
            if let Some(cfg) = ov.for_pillar(pillar) {
                return decision(cfg, kind, WriteThroughSource::AdminOverride);
            }
        }
        drop(admin_guard);

        // Layer 3 — env override.
        if let Some(ov) = &self.env {
            if let Some(cfg) = ov.for_pillar(pillar) {
                return decision(cfg, kind, WriteThroughSource::EnvOverride);
            }
        }

        // Layer 2 — policy.toml override.
        if let Some(ov) = &self.policy {
            if let Some(cfg) = ov.for_pillar(pillar) {
                return decision(cfg, kind, WriteThroughSource::PolicyOverride);
            }
        }

        // Layer 1 — manifest default.
        if let Some(cfg) = self.manifest.get(pillar) {
            return decision(cfg, kind, WriteThroughSource::ManifestDefault);
        }

        EffectiveWriteThrough::Off {
            source: WriteThroughSource::ImplicitDefault,
        }
    }
}

/// Translate a `(WriteThroughConfig, kind)` pair into an effective decision
/// stamped with its layer source.
fn decision(
    cfg: &WriteThroughConfig,
    kind: &str,
    source: WriteThroughSource,
) -> EffectiveWriteThrough {
    if cfg.applies_to(kind) {
        EffectiveWriteThrough::On { source }
    } else {
        EffectiveWriteThrough::Off { source }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn shefa_on() -> WriteThroughConfig {
        WriteThroughConfig {
            enabled: true,
            kinds: None,
        }
    }

    fn shefa_off() -> WriteThroughConfig {
        WriteThroughConfig {
            enabled: false,
            kinds: None,
        }
    }

    fn shefa_only_economic_event() -> WriteThroughConfig {
        WriteThroughConfig {
            enabled: true,
            kinds: Some(vec!["EconomicEvent".to_string()]),
        }
    }

    // --- WriteThroughConfig::applies_to -------------------------------------

    #[test]
    fn config_disabled_never_applies() {
        let cfg = shefa_off();
        assert!(!cfg.applies_to("EconomicEvent"));
        assert!(!cfg.applies_to("Whatever"));
    }

    #[test]
    fn config_enabled_with_no_kinds_applies_to_all() {
        let cfg = shefa_on();
        assert!(cfg.applies_to("EconomicEvent"));
        assert!(cfg.applies_to("Observation"));
    }

    #[test]
    fn config_enabled_with_empty_kinds_applies_to_all() {
        let cfg = WriteThroughConfig {
            enabled: true,
            kinds: Some(vec![]),
        };
        assert!(cfg.applies_to("EconomicEvent"));
    }

    #[test]
    fn config_enabled_with_kinds_restricts_scope() {
        let cfg = shefa_only_economic_event();
        assert!(cfg.applies_to("EconomicEvent"));
        assert!(!cfg.applies_to("Observation"));
    }

    // --- is_integrity_kind --------------------------------------------------

    #[test]
    fn integrity_kinds_are_recognized() {
        assert!(is_integrity_kind("KeyRotation"));
        assert!(is_integrity_kind("KeyRevocation"));
        assert!(is_integrity_kind("RevocationAttestation"));
        assert!(is_integrity_kind("AgentPeerBinding"));
    }

    #[test]
    fn non_integrity_kinds_are_not_recognized() {
        assert!(!is_integrity_kind("EconomicEvent"));
        assert!(!is_integrity_kind("Observation"));
        assert!(!is_integrity_kind("Content"));
        assert!(!is_integrity_kind(""));
    }

    // --- WriteThroughState::effective_for -----------------------------------

    #[test]
    fn empty_state_returns_implicit_off_for_non_integrity() {
        let state = WriteThroughState::empty();
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(
            eff,
            EffectiveWriteThrough::Off {
                source: WriteThroughSource::ImplicitDefault
            }
        );
        assert!(!eff.is_on());
    }

    #[test]
    fn integrity_kind_always_on_even_with_empty_state() {
        let state = WriteThroughState::empty();
        let eff = state.effective_for("imagodei", "KeyRotation");
        assert_eq!(eff, EffectiveWriteThrough::OnIntegrityException);
        assert!(eff.is_on());
        assert_eq!(eff.source(), WriteThroughSource::IntegrityException);
    }

    #[test]
    fn integrity_kind_overrides_explicit_off() {
        let mut manifest = HashMap::new();
        manifest.insert("imagodei".to_string(), shefa_off());
        let state = WriteThroughState::from_manifest(manifest);
        let eff = state.effective_for("imagodei", "KeyRotation");
        assert_eq!(eff, EffectiveWriteThrough::OnIntegrityException);
    }

    #[test]
    fn manifest_layer_resolves_when_no_overrides() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_on());
        let state = WriteThroughState::from_manifest(manifest);
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(
            eff,
            EffectiveWriteThrough::On {
                source: WriteThroughSource::ManifestDefault
            }
        );
    }

    #[test]
    fn policy_override_supersedes_manifest() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_off());

        let mut policy = WriteThroughOverride::empty();
        policy.pillars.insert("shefa".to_string(), shefa_on());

        let state = WriteThroughState::from_manifest(manifest).with_policy_override(policy);
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(
            eff,
            EffectiveWriteThrough::On {
                source: WriteThroughSource::PolicyOverride
            }
        );
    }

    #[test]
    fn env_override_supersedes_policy() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_off());

        let mut policy = WriteThroughOverride::empty();
        policy.pillars.insert("shefa".to_string(), shefa_on());

        let mut env = WriteThroughOverride::empty();
        env.pillars.insert("shefa".to_string(), shefa_off());

        let state = WriteThroughState::from_manifest(manifest)
            .with_policy_override(policy)
            .with_env_override(env);
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(
            eff,
            EffectiveWriteThrough::Off {
                source: WriteThroughSource::EnvOverride
            }
        );
    }

    #[test]
    fn admin_override_supersedes_env() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_off());

        let mut env = WriteThroughOverride::empty();
        env.pillars.insert("shefa".to_string(), shefa_off());

        let state = WriteThroughState::from_manifest(manifest).with_env_override(env);

        let mut admin = WriteThroughOverride::empty();
        admin.pillars.insert("shefa".to_string(), shefa_on());
        state.set_admin_override(Some(admin));

        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(
            eff,
            EffectiveWriteThrough::On {
                source: WriteThroughSource::AdminOverride
            }
        );
    }

    #[test]
    fn admin_override_can_be_cleared() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_on());
        let state = WriteThroughState::from_manifest(manifest);

        let mut admin = WriteThroughOverride::empty();
        admin.pillars.insert("shefa".to_string(), shefa_off());
        state.set_admin_override(Some(admin));

        assert_eq!(
            state.effective_for("shefa", "EconomicEvent").source(),
            WriteThroughSource::AdminOverride
        );

        state.set_admin_override(None);

        assert_eq!(
            state.effective_for("shefa", "EconomicEvent").source(),
            WriteThroughSource::ManifestDefault
        );
    }

    #[test]
    fn override_for_other_pillar_does_not_leak() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_on());
        let mut env = WriteThroughOverride::empty();
        // Override for lamad, not shefa.
        env.pillars.insert("lamad".to_string(), shefa_off());
        let state = WriteThroughState::from_manifest(manifest).with_env_override(env);

        // shefa still resolves from manifest (env has no opinion on shefa).
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert_eq!(eff.source(), WriteThroughSource::ManifestDefault);
        assert!(eff.is_on());
    }

    #[test]
    fn kind_restriction_at_manifest_layer_works() {
        let mut manifest = HashMap::new();
        manifest.insert("shefa".to_string(), shefa_only_economic_event());
        let state = WriteThroughState::from_manifest(manifest);

        // In-scope kind: ON.
        let eff = state.effective_for("shefa", "EconomicEvent");
        assert!(eff.is_on());
        assert_eq!(eff.source(), WriteThroughSource::ManifestDefault);

        // Out-of-scope kind: OFF (still attributed to manifest, since manifest spoke).
        let eff = state.effective_for("shefa", "Observation");
        assert!(!eff.is_on());
        assert_eq!(eff.source(), WriteThroughSource::ManifestDefault);
    }

    #[test]
    fn admin_snapshot_returns_current_value() {
        let state = WriteThroughState::empty();
        assert!(state.admin_snapshot().is_none());

        let mut admin = WriteThroughOverride::empty();
        admin.pillars.insert("shefa".to_string(), shefa_on());
        state.set_admin_override(Some(admin.clone()));

        let snap = state.admin_snapshot().expect("admin should be set");
        assert_eq!(snap, admin);
    }
}
