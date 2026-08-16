//! `ManifestStakesResolver` — T10's IMPURE `StakesResolver` production
//! implementor (head-plane trust-gradient program plan §3 L5; minutes-quiesce
//! fixture-trust-swarm plan §3 W2, task Q6).
//!
//! Lives OUTSIDE `stage.rs` on purpose: `stage.rs`'s `pure-half-stays-pure-stage`
//! `.epr-meta` rule forbids diesel/tokio/a wall clock/an env read in that file,
//! so the production resolver — which touches both the `manifests` projection
//! table (via [`ManifestRegistry`]) and `ELOHIM_NETWORK_STAKES` — is authored
//! here and passed to pure call sites as `&dyn StakesResolver`, exactly as
//! `stage.rs`'s own module doc anticipates.
//!
//! ## Precedence (fail-closed; never toward [`NetworkStage::Simulacra`])
//!
//! 1. **(a) Manifest** — a `network-stakes` manifest row for `scope`, read from
//!    the shared [`ManifestRegistry`] cache. Provenance: `Manifest { cid }`.
//! 2. **(b) Operator config** — `ELOHIM_NETWORK_STAKES`, read EXACTLY ONCE at
//!    [`ManifestStakesResolver::new`] (never on the `stage_for` hot path — the
//!    same boot-time-env-read discipline `main.rs` uses for its other
//!    `ELOHIM_*` flags). Provenance: `OperatorConfig`.
//! 3. **(c) Neither present/parseable** — [`NetworkStage::Bootstrap`] /
//!    [`StakesProvenance::BootstrapDefault`]. Fail-closed default.
//!
//! When both (a) and (b) are present and PARSE but DISAGREE, the STRICTER
//! stage (the greater one under `NetworkStage`'s semantic `Ord`) wins, and a
//! `warn!` names both — fail toward scrutiny, never toward `Simulacra`. An
//! unrecognized stage string on EITHER leg is a parse REJECTION (treated as
//! that leg being absent, `warn!`-logged), never a best-effort guess — the
//! same rule the seeder's `STAKES-DECLARATION-SEAM-Q6` contract states for
//! the artifact it mints (`genesis/seeder/src/corpus-trust.ts`).

use std::sync::Arc;

use diesel::sqlite::SqliteConnection;

use crate::error::StorageError;
use crate::services::manifest_registry::ManifestRegistry;
use crate::trust::stage::{NetworkStage, StakesProvenance, StakesResolver};

/// The env var read once at construction — the operator-config leg (b).
const ELOHIM_NETWORK_STAKES_ENV: &str = "ELOHIM_NETWORK_STAKES";

/// Reads a declared [`NetworkStage`] for a scope from the manifest registry
/// and/or `ELOHIM_NETWORK_STAKES`. Constructed once at boot (`main.rs`),
/// shared as `Arc`, borrowed as `&dyn StakesResolver` at call sites — the
/// impure counterpart [`crate::trust::stage::FixedStakesResolver`] stands in
/// for in tests and the still-inert decision paths.
pub struct ManifestStakesResolver {
    registry: Arc<ManifestRegistry>,
    /// Resolved ONCE at construction — see the module doc's precedence leg
    /// (b). `None` covers both "unset" and "set but unparseable" (the latter
    /// is `warn!`-logged at construction time, not per-lookup).
    env_stage: Option<NetworkStage>,
}

impl ManifestStakesResolver {
    /// Reads `ELOHIM_NETWORK_STAKES` exactly once. Mirrors the boot-time
    /// env-read pattern in `main.rs` (parse + `warn!` on an unrecognized
    /// value, never a silent guess or a DEV_MODE-derived fallback).
    pub fn new(registry: Arc<ManifestRegistry>) -> Self {
        let env_stage = std::env::var(ELOHIM_NETWORK_STAKES_ENV)
            .ok()
            .and_then(|raw| match raw.trim().parse::<NetworkStage>() {
                Ok(stage) => Some(stage),
                Err(()) => {
                    tracing::warn!(
                        raw = %raw,
                        "ELOHIM_NETWORK_STAKES is set to an unrecognized NetworkStage string \
                         (must be exactly one of simulacra|bootstrap|coordinated|enforced, \
                         lowercase) — ignoring; resolution falls through to the manifest leg \
                         or the Bootstrap fail-closed default"
                    );
                    None
                }
            });
        Self {
            registry,
            env_stage,
        }
    }

    /// Refreshes the shared [`ManifestRegistry`] cache from the `manifests`
    /// table. Called by `PUT /admin/seed/network-stakes` right after an
    /// insert so a freshly-declared stakes manifest is visible to
    /// `stage_for` without a process restart — the same
    /// project-then-reload pairing `EprStore::put` already does for the
    /// live DHT-projected manifest path (`services/epr_store.rs`).
    pub fn refresh(&self, conn: &mut SqliteConnection) -> Result<usize, StorageError> {
        self.registry.load_from_db(conn)
    }
}

impl StakesResolver for ManifestStakesResolver {
    fn stage_for(&self, scope: &str) -> (NetworkStage, StakesProvenance) {
        let manifest_leg =
            self.registry
                .network_stakes_for_scope(scope)
                .and_then(|(stage_raw, cid)| match stage_raw.parse::<NetworkStage>() {
                    Ok(stage) => Some((stage, cid)),
                    Err(()) => {
                        tracing::warn!(
                            scope = %scope,
                            raw = %stage_raw,
                            cid = %cid,
                            "network-stakes manifest declares an unrecognized NetworkStage \
                             string — rejecting (treated as absent for this scope), never a guess"
                        );
                        None
                    }
                });

        match (manifest_leg, self.env_stage) {
            (Some((m_stage, cid)), Some(e_stage)) if m_stage == e_stage => {
                (m_stage, StakesProvenance::Manifest { cid })
            }
            (Some((m_stage, cid)), Some(e_stage)) => {
                let stricter = std::cmp::max(m_stage, e_stage);
                tracing::warn!(
                    scope = %scope,
                    manifest_stage = ?m_stage,
                    env_stage = ?e_stage,
                    resolved = ?stricter,
                    "declared stakes DISAGREE between the network-stakes manifest and \
                     ELOHIM_NETWORK_STAKES — resolving to the STRICTER stage (fail toward \
                     scrutiny, never toward Simulacra)"
                );
                if stricter == m_stage {
                    (stricter, StakesProvenance::Manifest { cid })
                } else {
                    (stricter, StakesProvenance::OperatorConfig)
                }
            }
            (Some((m_stage, cid)), None) => (m_stage, StakesProvenance::Manifest { cid }),
            (None, Some(e_stage)) => (e_stage, StakesProvenance::OperatorConfig),
            (None, None) => (NetworkStage::Bootstrap, StakesProvenance::BootstrapDefault),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manifests::{insert_manifest, ManifestRow};
    use crate::test_util::test_pool;

    fn stakes_row(cid: &str, scope: &str, stage: &str, revision: i32) -> ManifestRow {
        let payload = serde_json::json!({
            "schemaVersion": "1",
            "manifestKind": "network-stakes",
            "manifestCid": cid,
            "stakes": {
                "stage": stage,
                "scope": scope,
                "grantor": "human-adam-firstman",
                "environment": if stage == "simulacra" { "preproduction" } else { "production" },
            },
        })
        .to_string();
        ManifestRow {
            cid: cid.to_string(),
            manifest_kind: "network-stakes".to_string(),
            pillar: None,
            payload_json: payload,
            schema_ref: None,
            signer_pubkey: vec![0u8; 32],
            created_at: "2026-08-16T00:00:00Z".to_string(),
            verified_at: Some("2026-08-16T00:00:00Z".to_string()),
            revision,
        }
    }

    fn resolver_with_manifest(scope: &str, stage: &str) -> ManifestStakesResolver {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &stakes_row("stakes:test:1", scope, stage, 1)).unwrap();
        let registry = Arc::new(ManifestRegistry::new());
        registry.load_from_db(&mut conn).unwrap();
        ManifestStakesResolver {
            registry,
            env_stage: None,
        }
    }

    #[test]
    fn manifest_only_resolves_declared_stage_with_manifest_provenance() {
        let resolver = resolver_with_manifest("genesis-lamad", "coordinated");
        let (stage, provenance) = resolver.stage_for("genesis-lamad");
        assert_eq!(stage, NetworkStage::Coordinated);
        assert_eq!(
            provenance,
            StakesProvenance::Manifest {
                cid: "stakes:test:1".to_string()
            }
        );
    }

    #[test]
    fn env_only_resolves_declared_stage_with_operator_config_provenance() {
        let registry = Arc::new(ManifestRegistry::new());
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: Some(NetworkStage::Enforced),
        };
        let (stage, provenance) = resolver.stage_for("any-scope");
        assert_eq!(stage, NetworkStage::Enforced);
        assert_eq!(provenance, StakesProvenance::OperatorConfig);
    }

    #[test]
    fn both_agree_resolves_that_stage_with_manifest_provenance() {
        let mut resolver = resolver_with_manifest("genesis-lamad", "simulacra");
        resolver.env_stage = Some(NetworkStage::Simulacra);
        let (stage, provenance) = resolver.stage_for("genesis-lamad");
        assert_eq!(stage, NetworkStage::Simulacra);
        assert_eq!(
            provenance,
            StakesProvenance::Manifest {
                cid: "stakes:test:1".to_string()
            }
        );
    }

    #[test]
    fn both_disagree_resolves_to_the_stricter_stage_manifest_stricter() {
        let mut resolver = resolver_with_manifest("genesis-lamad", "enforced");
        resolver.env_stage = Some(NetworkStage::Simulacra);
        let (stage, provenance) = resolver.stage_for("genesis-lamad");
        assert_eq!(
            stage,
            NetworkStage::Enforced,
            "manifest was stricter — must win"
        );
        assert_eq!(
            provenance,
            StakesProvenance::Manifest {
                cid: "stakes:test:1".to_string()
            }
        );
    }

    #[test]
    fn both_disagree_resolves_to_the_stricter_stage_env_stricter() {
        let mut resolver = resolver_with_manifest("genesis-lamad", "simulacra");
        resolver.env_stage = Some(NetworkStage::Enforced);
        let (stage, provenance) = resolver.stage_for("genesis-lamad");
        assert_eq!(stage, NetworkStage::Enforced, "env was stricter — must win");
        assert_eq!(provenance, StakesProvenance::OperatorConfig);
    }

    #[test]
    fn absent_both_resolves_bootstrap_default() {
        let registry = Arc::new(ManifestRegistry::new());
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: None,
        };
        let (stage, provenance) = resolver.stage_for("no-such-scope");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn unknown_manifest_stage_string_is_rejected_not_guessed_falls_to_bootstrap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Hand-insert a row whose stakes.stage is NOT one of the four
        // recognized variants — must be rejected, not "best effort" parsed.
        let payload = serde_json::json!({
            "manifestKind": "network-stakes",
            "stakes": { "stage": "dev-mode", "scope": "genesis-lamad", "grantor": "g", "environment": "preproduction" },
        })
        .to_string();
        insert_manifest(
            &mut conn,
            &ManifestRow {
                cid: "stakes:bad:1".to_string(),
                manifest_kind: "network-stakes".to_string(),
                pillar: None,
                payload_json: payload,
                schema_ref: None,
                signer_pubkey: vec![0u8; 32],
                created_at: "2026-08-16T00:00:00Z".to_string(),
                verified_at: None,
                revision: 1,
            },
        )
        .unwrap();
        let registry = Arc::new(ManifestRegistry::new());
        registry.load_from_db(&mut conn).unwrap();
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: None,
        };
        let (stage, provenance) = resolver.stage_for("genesis-lamad");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn unrecognized_env_value_is_rejected_at_construction_falls_to_bootstrap() {
        // SAFETY: single-threaded test env var mutation, scoped to this test
        // only, restored before returning.
        std::env::set_var(ELOHIM_NETWORK_STAKES_ENV, "dev");
        let registry = Arc::new(ManifestRegistry::new());
        let resolver = ManifestStakesResolver::new(registry);
        std::env::remove_var(ELOHIM_NETWORK_STAKES_ENV);
        assert!(
            resolver.env_stage.is_none(),
            "an unrecognized env string must resolve to None, never a guessed stage"
        );
        let (stage, provenance) = resolver.stage_for("any-scope");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn simulacra_is_never_produced_without_an_explicit_declaration() {
        // No manifest row, no env var — the resolver must never default to
        // Simulacra under any absence path.
        let registry = Arc::new(ManifestRegistry::new());
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: None,
        };
        for scope in ["genesis-lamad", "lamad", "", "anything"] {
            let (stage, _) = resolver.stage_for(scope);
            assert_ne!(stage, NetworkStage::Simulacra);
        }
    }

    #[test]
    fn refresh_makes_a_newly_inserted_manifest_visible_without_reconstruction() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let registry = Arc::new(ManifestRegistry::new());
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: None,
        };
        // Nothing declared yet.
        assert_eq!(
            resolver.stage_for("genesis-lamad").0,
            NetworkStage::Bootstrap
        );

        insert_manifest(
            &mut conn,
            &stakes_row("stakes:test:2", "genesis-lamad", "coordinated", 1),
        )
        .unwrap();
        resolver.refresh(&mut conn).expect("refresh should succeed");

        assert_eq!(
            resolver.stage_for("genesis-lamad").0,
            NetworkStage::Coordinated
        );
    }
}
