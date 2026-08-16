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
//!
//! ## Scope-key reconciliation (Q7 — the decision, stated)
//!
//! A stakes declaration is scoped by **corpus id** (`genesis-lamad` — the
//! seeder enforces `trustBootstrap.scope == corpus.id`). The runtime resolves
//! stakes by **`h_app_id`** (`lamad`). Left unreconciled these two keyings never
//! meet, and the grant silently prices nothing — the failure mode this section
//! exists to refuse.
//!
//! **Decision: `stage_for` accepts BOTH keyings, and takes the STRICTEST stage
//! across every leg that matches** — the same fail-toward-scrutiny rule the
//! manifest/env disagreement above already uses, extended to a third and fourth
//! leg rather than given its own precedence order. Every leg is an EXPLICIT
//! declaration; none is inferred:
//!
//! 1. **Exact key.** A manifest whose `stakes.scope` equals the requested key.
//! 2. **Declared `h_app_id`.** A manifest that names `stakes.hAppId`; the
//!    registry indexes it under that key too (`ManifestRegistry::load_from_db`).
//!    Correct-but-dormant: the seeder does not emit the field yet.
//! 3. **Operator-declared alias.** `ELOHIM_NETWORK_STAKES_SCOPE_ALIASES`, e.g.
//!    `genesis-lamad=lamad`, read EXACTLY ONCE at construction. This is the
//!    bridge that makes a corpus-keyed grant bite on h_app_id traffic today.
//! 4. **Operator config.** `ELOHIM_NETWORK_STAKES` — scope-independent, so it
//!    matches every key by construction and needs no reconciliation.
//!
//! What is deliberately NOT done: stripping a `genesis-` prefix, parsing
//! `provenance.declaredBy` for a directory name, or any other derivation of one
//! key from the other. `genesis-humans` does not become `humans` (its h_app_id
//! is `imagodei`), so prefix arithmetic is wrong in general — and inference is
//! exactly the door `Simulacra is never a default` closes.

use std::sync::Arc;

use diesel::sqlite::SqliteConnection;

use crate::error::StorageError;
use crate::services::manifest_registry::ManifestRegistry;
use crate::trust::stage::{NetworkStage, StakesProvenance, StakesResolver};

/// The env var read once at construction — the operator-config leg (b).
const ELOHIM_NETWORK_STAKES_ENV: &str = "ELOHIM_NETWORK_STAKES";

/// The scope-alias env var read once at construction — leg 3 of the
/// scope-key reconciliation (see the module doc).
///
/// Format: comma-separated `corpus-scope=runtime-scope` pairs, e.g.
/// `genesis-lamad=lamad,genesis-humans=imagodei`. Whitespace around either side
/// is trimmed; a malformed or empty entry is `warn!`-logged and DROPPED (never a
/// partial or guessed mapping).
const ELOHIM_NETWORK_STAKES_SCOPE_ALIASES_ENV: &str = "ELOHIM_NETWORK_STAKES_SCOPE_ALIASES";

/// Parse the alias declaration into `runtime-scope -> [corpus-scope, ...]`.
///
/// Keyed by the RUNTIME scope (the argument `stage_for` receives) because that
/// is the lookup direction; several corpora may legitimately alias onto one
/// h_app_id (`genesis-lamad` and `genesis-lamad-extra` both onto `lamad`), and
/// the strictest-wins fold below then covers all of them.
///
/// PURE — takes the raw string, so the parse is testable without touching the
/// process environment (the parallel-test flake this codebase has already paid
/// for once).
fn parse_scope_aliases(raw: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        match entry.split_once('=') {
            Some((corpus, runtime)) => {
                let (corpus, runtime) = (corpus.trim(), runtime.trim());
                if corpus.is_empty() || runtime.is_empty() {
                    tracing::warn!(
                        entry = %entry,
                        "ELOHIM_NETWORK_STAKES_SCOPE_ALIASES entry has an empty side \
                         (expected `corpus-scope=runtime-scope`) — dropping it"
                    );
                    continue;
                }
                out.entry(runtime.to_string())
                    .or_default()
                    .push(corpus.to_string());
            }
            None => {
                tracing::warn!(
                    entry = %entry,
                    "ELOHIM_NETWORK_STAKES_SCOPE_ALIASES entry is not `corpus-scope=\
                     runtime-scope` — dropping it (never a guessed mapping)"
                );
            }
        }
    }
    out
}

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
    /// Scope-key reconciliation leg 3, resolved ONCE at construction:
    /// `runtime-scope -> [corpus-scope, ...]`. Empty when
    /// `ELOHIM_NETWORK_STAKES_SCOPE_ALIASES` is unset — which is the
    /// behavior-neutral default (no alias, no extra lookup).
    scope_aliases: std::collections::HashMap<String, Vec<String>>,
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
        let scope_aliases = std::env::var(ELOHIM_NETWORK_STAKES_SCOPE_ALIASES_ENV)
            .ok()
            .map(|raw| parse_scope_aliases(&raw))
            .unwrap_or_default();
        if !scope_aliases.is_empty() {
            tracing::info!(
                aliases = ?scope_aliases,
                "declared-stakes scope aliases loaded (corpus-scope -> runtime-scope); a \
                 corpus-keyed grant now also answers for the runtime scope it names"
            );
        }
        Self {
            registry,
            env_stage,
            scope_aliases,
        }
    }

    /// Test-only constructor that takes the two operator declarations as
    /// VALUES instead of reading them from the process environment.
    ///
    /// Exists so cross-module tests (`trust::adoption_pricing`) can exercise
    /// the REAL resolver end-to-end without `set_var` — the parallel-test flake
    /// this codebase has already paid for once.
    #[cfg(test)]
    pub(crate) fn with_declarations(
        registry: Arc<ManifestRegistry>,
        env_stage: Option<NetworkStage>,
        scope_aliases_raw: &str,
    ) -> Self {
        Self {
            registry,
            env_stage,
            scope_aliases: parse_scope_aliases(scope_aliases_raw),
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

impl ManifestStakesResolver {
    /// One manifest lookup + parse for ONE key, `None` on absence or on an
    /// unrecognized stage string (rejected, `warn!`-logged, never guessed).
    fn manifest_leg_for_key(&self, key: &str) -> Option<(NetworkStage, String)> {
        self.registry
            .network_stakes_for_scope(key)
            .and_then(|(stage_raw, cid)| match stage_raw.parse::<NetworkStage>() {
                Ok(stage) => Some((stage, cid)),
                Err(()) => {
                    tracing::warn!(
                        scope = %key,
                        raw = %stage_raw,
                        cid = %cid,
                        "network-stakes manifest declares an unrecognized NetworkStage \
                         string — rejecting (treated as absent for this scope), never a guess"
                    );
                    None
                }
            })
    }

    /// The manifest leg across EVERY key that reconciles onto `scope` — the
    /// exact key (legs 1+2, since the registry indexes a declared `hAppId`
    /// under that key too) plus every operator-declared alias (leg 3).
    ///
    /// Returns the STRICTEST match, so a scope reachable by two keys can only
    /// ever be resolved toward MORE scrutiny by the extra reach, never less —
    /// the same rule the manifest/env disagreement below uses. Ties keep the
    /// first (exact-key) cid, so an exact declaration keeps its provenance.
    fn manifest_leg(&self, scope: &str) -> Option<(NetworkStage, String)> {
        let aliases = self
            .scope_aliases
            .get(scope)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        std::iter::once(scope)
            .chain(aliases.iter().map(String::as_str))
            .filter_map(|key| self.manifest_leg_for_key(key))
            .reduce(|best, next| if next.0 > best.0 { next } else { best })
    }
}

impl StakesResolver for ManifestStakesResolver {
    fn stage_for(&self, scope: &str) -> (NetworkStage, StakesProvenance) {
        let manifest_leg = self.manifest_leg(scope);

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
            scope_aliases: Default::default(),
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
            scope_aliases: Default::default(),
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
            scope_aliases: Default::default(),
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
            scope_aliases: Default::default(),
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
            scope_aliases: Default::default(),
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
            scope_aliases: Default::default(),
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

    // -----------------------------------------------------------------------
    // Scope-key reconciliation (Q7)
    // -----------------------------------------------------------------------

    #[test]
    fn scope_aliases_parse_into_runtime_keyed_corpus_lists() {
        let aliases = parse_scope_aliases(
            "genesis-lamad=lamad, genesis-extra = lamad ,genesis-humans=imagodei",
        );
        assert_eq!(
            aliases.get("lamad"),
            Some(&vec![
                "genesis-lamad".to_string(),
                "genesis-extra".to_string()
            ])
        );
        assert_eq!(
            aliases.get("imagodei"),
            Some(&vec!["genesis-humans".to_string()])
        );
    }

    #[test]
    fn malformed_scope_alias_entries_are_dropped_never_guessed() {
        let aliases = parse_scope_aliases(",  ,genesis-lamad,=lamad,genesis-x=,genesis-ok=lamad");
        assert_eq!(aliases.len(), 1, "only the well-formed entry survives");
        assert_eq!(aliases.get("lamad"), Some(&vec!["genesis-ok".to_string()]));
    }

    /// THE reconciliation test the plan asks for: a grant seeded exactly as the
    /// genesis seeder mints it — scoped to the CORPUS id `genesis-lamad` — is
    /// what prices `lamad` (the runtime `h_app_id`) traffic, once the operator
    /// declares the bridge.
    #[test]
    fn a_corpus_scoped_grant_answers_for_h_app_id_traffic_via_a_declared_alias() {
        let mut resolver = resolver_with_manifest("genesis-lamad", "simulacra");
        resolver.scope_aliases = parse_scope_aliases("genesis-lamad=lamad");
        let (stage, provenance) = resolver.stage_for("lamad");
        assert_eq!(stage, NetworkStage::Simulacra);
        assert_eq!(
            provenance,
            StakesProvenance::Manifest {
                cid: "stakes:test:1".to_string()
            },
            "the corpus manifest is the provenance, not the alias"
        );
    }

    #[test]
    fn a_corpus_scoped_grant_never_reaches_h_app_id_traffic_without_a_declaration() {
        // No alias, no hAppId on the artifact — the two keyings do not meet,
        // and the resolver says so by failing closed rather than by inferring
        // "lamad" out of "genesis-lamad".
        let resolver = resolver_with_manifest("genesis-lamad", "simulacra");
        let (stage, provenance) = resolver.stage_for("lamad");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    /// Leg 2 — the forward-compatible one. A declaration that names its
    /// `h_app_id` answers for that key with no operator alias at all.
    #[test]
    fn a_manifest_declaring_its_h_app_id_answers_for_that_key() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let payload = serde_json::json!({
            "schemaVersion": "1",
            "manifestKind": "network-stakes",
            "manifestCid": "stakes:happ:1",
            "stakes": {
                "stage": "simulacra",
                "scope": "genesis-lamad",
                "hAppId": "lamad",
                "grantor": "human-adam-firstman",
                "environment": "preproduction",
            },
        })
        .to_string();
        insert_manifest(
            &mut conn,
            &ManifestRow {
                cid: "stakes:happ:1".to_string(),
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
            scope_aliases: Default::default(),
        };
        assert_eq!(resolver.stage_for("lamad").0, NetworkStage::Simulacra);
        assert_eq!(
            resolver.stage_for("genesis-lamad").0,
            NetworkStage::Simulacra,
            "the corpus key still answers too — indexed under BOTH"
        );
    }

    /// Two corpora alias onto one runtime scope and DISAGREE — the stricter one
    /// wins, exactly as the manifest/env disagreement does. Extra reach can only
    /// ever resolve toward MORE scrutiny.
    #[test]
    fn two_aliased_corpora_that_disagree_resolve_to_the_stricter_stage() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(
            &mut conn,
            &stakes_row("stakes:a:1", "genesis-cheap", "simulacra", 1),
        )
        .unwrap();
        insert_manifest(
            &mut conn,
            &stakes_row("stakes:b:1", "genesis-strict", "enforced", 1),
        )
        .unwrap();
        let registry = Arc::new(ManifestRegistry::new());
        registry.load_from_db(&mut conn).unwrap();
        let resolver = ManifestStakesResolver {
            registry,
            env_stage: None,
            scope_aliases: parse_scope_aliases("genesis-cheap=lamad,genesis-strict=lamad"),
        };
        assert_eq!(resolver.stage_for("lamad").0, NetworkStage::Enforced);
    }

    #[test]
    fn an_alias_to_a_scope_with_no_manifest_changes_nothing() {
        let mut resolver = resolver_with_manifest("lamad", "coordinated");
        resolver.scope_aliases = parse_scope_aliases("genesis-nothing=lamad");
        let (stage, provenance) = resolver.stage_for("lamad");
        assert_eq!(stage, NetworkStage::Coordinated);
        assert_eq!(
            provenance,
            StakesProvenance::Manifest {
                cid: "stakes:test:1".to_string()
            }
        );
    }
}
