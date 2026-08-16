//! Q7 — the head-plane **pricing consumption point**: the one place a resolved
//! [`NetworkStage`] is allowed to change what an adoption sweep actually does.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md`
//! §3 W2 (per-peer fixture trust — ceremony compression), composed with
//! `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L2 + L5. Canonical principle:
//! `genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md`.
//!
//! IMPURE (diesel + metrics), for the same reason
//! [`crate::trust::manifest_resolver`] is: the pure halves (`stage.rs`,
//! `pricer.rs`) must stay free of I/O, so the code that BUYS the pricing inputs
//! lives here and hands them in as values.
//!
//! ## What is priced, and what is NEVER priced
//!
//! The pricer prices **verification depth**. It does not price, widen, or
//! otherwise touch **write authority**. Declared heads move through exactly one
//! entrypoint before and after this task: the own conductor's
//! `declare_canonical_head` extern, whose answer is then stamped by
//! `content_diesel::stamp_declared_head_mode`
//! (`head_adoption::declare_peer_head`). A peer-advertised
//! `(id, head_action_hash)` pair is NEVER stamped directly at any stage — that
//! is the substrate trust contract's "canonical channels alone move declared
//! heads," and `AcceptWithProvenance` does not amend it.
//!
//! What `AcceptWithProvenance` DOES buy is the removal of the two per-id
//! EVIDENCE round-trips that precede that write:
//!
//! 1. the **election probe** (`resolve_canonical_election`, one conductor
//!    round-trip per probe-eligible candidate, batched but still O(ids)), and
//! 2. the **`ContentHeadRecord` verify RPC** to the advertising peer
//!    (`head_adoption::resolve_peer_evidence`, one view-federation round-trip
//!    per candidate, plus up to `evidence_fallback_max_alternates` more).
//!
//! Both are skipped only where the advertised declared head **can be adopted
//! directly** — the `AdoptPeer` corner of `head_adoption::decide_head_action`:
//! a peer advertises a declaration AND the local row declares nothing. Outside
//! that corner the decision rule routes to `Hold` / `ContestPeer` /
//! `ContestDivergent` / `Author`, none of which the skips would serve, so
//! [`price_adopt_slice`] refuses to price them cheaply at all.
//!
//! ## Floor protection, end to end
//!
//! The keystone lives in `pricer.rs` and is proven over the full
//! `NetworkStage × FloorClass × Reach × Standing × provenance` product for every
//! in-crate pricer. This module is what makes it BITE on real rows: the floor
//! class is read from the local projection (`content.reach` →
//! `floor_protections::content_head_floor_class`) on EVERY pricing call at
//! EVERY stage — never hardcoded to `None`, never fabricated for a stage the
//! consumer expects to be full-chain anyway — and an id whose reach does not
//! parse is refused a price entirely.
//! `tests::a_private_reach_row_is_never_priced_cheap_even_at_simulacra` and
//! `tests::an_unparseable_reach_is_refused_a_price` pin the end-to-end path.

use std::collections::HashMap;

use crate::db::{content_diesel, AppContext, DbPool};
use crate::services::floor_protections::content_head_floor_class;
use crate::services::head_adoption::PeerHeadHints;
use crate::services::standing::Standing;
use crate::trust::pricer::{PricedVerification, PricingInput, VerificationDepth};
use crate::trust::stage::{NetworkStage, StakesProvenance};
use crate::trust::TrustGradient;

/// One sweep's priced verdicts plus the counts the sweep summary reports.
///
/// Deliberately NOT `Default`: a plan without a resolved
/// `(scope, stage, provenance)` would be a plan that cannot say WHY it priced
/// what it did, and the sweep summary's whole value is that it names the
/// declaration in force.
#[derive(Debug)]
pub struct AdoptionPricingPlan {
    /// `content_id -> verdict`. An id ABSENT from this map is unpriced and
    /// therefore full-chain — [`Self::for_id`] is the only reader, so the
    /// default cannot drift between consumers.
    verdicts: HashMap<String, PricedVerification>,
    /// The scope this plan was priced for (the `h_app_id` of the sweep).
    pub scope: String,
    /// The stage the scope resolved to, and how it was declared.
    pub stage: NetworkStage,
    pub provenance: StakesProvenance,
    pub accept_with_provenance: usize,
    pub delta_verify: usize,
    pub full_chain: usize,
    /// Candidates the pricer was never offered because the local projection
    /// could not classify them (row absent for this scope, or an unparseable
    /// reach). These are FULL-CHAIN by refusal, which is a different fact from
    /// full-chain by verdict and is counted apart from it.
    pub unpriceable: usize,
}

impl AdoptionPricingPlan {
    /// An empty plan for a resolved scope — the shape every sweep starts from
    /// and the shape an empty slice keeps.
    fn empty(scope: String, stage: NetworkStage, provenance: StakesProvenance) -> Self {
        Self {
            verdicts: HashMap::new(),
            scope,
            stage,
            provenance,
            accept_with_provenance: 0,
            delta_verify: 0,
            full_chain: 0,
            unpriceable: 0,
        }
    }

    /// The verdict for one id — [`PricedVerification::inert`] (full chain) for
    /// anything this plan did not price. The single default.
    pub fn for_id(&self, id: &str) -> PricedVerification {
        self.verdicts
            .get(id)
            .copied()
            .unwrap_or_else(PricedVerification::inert)
    }

    /// `true` iff this id's verdict licenses the ceremony compression.
    pub fn accepts_with_provenance(&self, id: &str) -> bool {
        self.for_id(id).accepts_with_provenance()
    }

    /// Did this plan cheapen anything at all? The sweep summary is only worth
    /// emitting at `info!` when a stage was actually declared; a Bootstrap
    /// fleet would otherwise log one line per sweep forever saying nothing
    /// happened.
    pub fn compressed_anything(&self) -> bool {
        self.accept_with_provenance > 0
    }
}

/// Price one sweep's adopt slice.
///
/// **Cost:** exactly ONE local SQL read for the whole slice — a primary-key
/// lookup of ≤ `WITNESS_MAX_PER_TICK` (200) ids, on a ~300s cadence. No
/// conductor call, no peer call: pricing must never itself cost a round-trip,
/// or it cannot pay for the ones it removes.
///
/// **The pricer is never lied to.** An earlier shape skipped the read at
/// non-Simulacra stages and handed the pricer fabricated inputs
/// (`floor: None`, `reach: Commons` — the MOST permissive of the eight). That
/// was sound only for the pricer wired that day: `pricer` is a `dyn` seam, and
/// a future standing-aware implementor honoring the floor keystone could still
/// have cheapened a `private`-reach, already-declared row because the consumer
/// told it there was no floor. Adversarial review (2026-08-16) produced exactly
/// that counterexample. The read is therefore unconditional: every
/// [`PricingInput`] this function builds is derived from the real projection
/// row, at every stage, and the `provenance_carried` conjunct
/// (`hint present AND the local row declares nothing`) means the same thing on
/// every path. Defense in depth, not instead of it: `VerificationPricer`'s
/// STAGE obligation is now a stated contract pinned registry-wide by
/// `trust::pricer::tests::every_in_crate_pricer_is_full_chain_at_every_non_simulacra_stage`.
///
/// **Behavior-neutrality.** Every DECISION is unchanged outside an explicit
/// Simulacra declaration — the stage obligation forces `FullChain` for every
/// id, so no skip is reachable and no write path differs. What is new at those
/// stages is one read-only `SELECT` per sweep; that is a cost statement, not a
/// behavior change, and it is what buys truthful floor labels and a
/// stage-comparable `unpriceable` count.
pub fn price_adopt_slice(
    pool: &DbPool,
    ctx: &AppContext,
    ids: &[String],
    hints: &PeerHeadHints,
    trust: &TrustGradient<'_>,
) -> AdoptionPricingPlan {
    let (stage, provenance) = trust.stakes.stage_for(&ctx.h_app_id);
    let mut plan = AdoptionPricingPlan::empty(ctx.h_app_id.clone(), stage, provenance);
    if ids.is_empty() {
        return plan;
    }

    // The projection facts, bought ONCE for the whole slice.
    let rows: HashMap<String, content_diesel::ContentHeadPricingRow> = match pool.get() {
        Ok(mut conn) => match content_diesel::content_head_pricing_inputs(&mut conn, ctx, ids) {
            Ok(rows) => rows.into_iter().map(|r| (r.id.clone(), r)).collect(),
            Err(e) => {
                // A DB error is never a licence to cheapen: an empty map makes
                // every id UNPRICEABLE, i.e. full chain by refusal.
                tracing::warn!(
                    error = %e,
                    "trust-pricing: could not read the adopt slice's projection inputs — \
                     every candidate stays FULL CHAIN this sweep"
                );
                HashMap::new()
            }
        },
        Err(e) => {
            tracing::warn!(
                error = %e,
                "trust-pricing: no db connection for the adopt slice — every candidate stays \
                 FULL CHAIN this sweep"
            );
            HashMap::new()
        }
    };

    for id in ids {
        let Some(row) = rows.get(id) else {
            // No local row for this scope — nothing to classify, so nothing to
            // price. Full chain by refusal.
            plan.unpriceable += 1;
            continue;
        };
        let Some(floor) = content_head_floor_class(&row.reach) else {
            // An unparseable reach cannot be shown to be floor-FREE, so it is
            // never shown to be priceable.
            plan.unpriceable += 1;
            continue;
        };
        let Some(reach) = crate::services::epr_kind::parse_reach_key(&row.reach) else {
            plan.unpriceable += 1;
            continue;
        };
        let input = PricingInput {
            stage,
            floor,
            reach,
            // `Unknown` everywhere until T19 lands the standing_projector
            // writer — this seam must not claim gradient behavior it does not
            // implement (program plan §2, evidence correction #1).
            standing: Standing::Unknown,
            // THE gate that keeps the skips inside the AdoptPeer corner: a peer
            // advertises a declaration AND this row declares nothing. Applied
            // on every path, at every stage, so the field cannot mean two
            // different things depending on the declaration in force.
            provenance_carried: hints.get(id).is_some() && row.declared_head_action_hash.is_none(),
        };

        let priced = trust.pricer.price(&input);
        match priced.depth {
            VerificationDepth::AcceptWithProvenance => plan.accept_with_provenance += 1,
            VerificationDepth::DeltaVerify => plan.delta_verify += 1,
            VerificationDepth::FullChain => plan.full_chain += 1,
        }
        crate::metrics::inc_trust_priced_adoption(depth_label(priced.depth));
        plan.verdicts.insert(id.clone(), priced);
    }

    plan
}

/// Map a [`VerificationDepth`] onto the closed metrics label vocabulary.
fn depth_label(depth: VerificationDepth) -> crate::metrics::TrustPricedDepth {
    match depth {
        VerificationDepth::AcceptWithProvenance => {
            crate::metrics::TrustPricedDepth::AcceptWithProvenance
        }
        VerificationDepth::DeltaVerify => crate::metrics::TrustPricedDepth::DeltaVerify,
        VerificationDepth::FullChain => crate::metrics::TrustPricedDepth::FullChain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::content_diesel::create_content;
    use crate::db::diesel_types::CreateContentInput;
    use crate::services::head_adoption::PeerHeadHint;
    use crate::test_util::test_pool;
    use crate::trust::pricer::{DeclaredStakesPricer, InertPricer, PricingReason};
    use crate::trust::stage::FixedStakesResolver;

    fn ctx() -> AppContext {
        AppContext::default_lamad()
    }

    fn seed_row(pool: &DbPool, id: &str, reach: &str, declared: Option<&str>) {
        let mut conn = pool.get().unwrap();
        let ctx = ctx();
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.to_string(),
                created_by: None,
                tags: vec![],
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .expect("seed content row");
        if let Some(head) = declared {
            content_diesel::stamp_declared_head(&mut conn, &ctx, id, head, Some(1), None)
                .expect("stamp declaration");
        }
    }

    fn hints_for(ids: &[&str]) -> PeerHeadHints {
        let mut hints = PeerHeadHints::new();
        for id in ids {
            hints.insert(
                (*id).to_string(),
                PeerHeadHint::sole(format!("uhCkk-head-{id}"), Some(1), "peer-a".into()),
            );
        }
        hints
    }

    fn declared_gradient(stage: NetworkStage) -> (FixedStakesResolver, DeclaredStakesPricer) {
        (
            FixedStakesResolver {
                stage,
                provenance: StakesProvenance::Manifest {
                    cid: "stakes:genesis-lamad:test".into(),
                },
            },
            DeclaredStakesPricer,
        )
    }

    #[test]
    fn a_simulacra_declaration_prices_an_adoptable_hinted_id_accept_with_provenance() {
        let pool = test_pool();
        seed_row(&pool, "c-1", "commons", None);
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-1".to_string()],
            &hints_for(&["c-1"]),
            &trust,
        );
        assert!(plan.accepts_with_provenance("c-1"));
        assert_eq!(plan.accept_with_provenance, 1);
        assert_eq!(plan.full_chain, 0);
        assert_eq!(
            plan.for_id("c-1").reason,
            PricingReason::DeclaredSimulacraProvenance
        );
    }

    /// FLOOR PROTECTION, END TO END. A `private`-reach row is the
    /// `LocalRelationship` floor, read off the real projection — and the sweep's
    /// pricing decision (not just the pricer unit) refuses to cheapen it, at
    /// the very stage and with the very provenance that would otherwise buy the
    /// compression.
    #[test]
    fn a_private_reach_row_is_never_priced_cheap_even_at_simulacra() {
        let pool = test_pool();
        seed_row(&pool, "c-private", "private", None);
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-private".to_string()],
            &hints_for(&["c-private"]),
            &trust,
        );
        assert!(!plan.accepts_with_provenance("c-private"));
        assert_eq!(plan.for_id("c-private").depth, VerificationDepth::FullChain);
        assert_eq!(
            plan.for_id("c-private").reason,
            PricingReason::FloorProtected,
            "a floor save must SAY it was a floor save"
        );
        assert_eq!(plan.accept_with_provenance, 0);
    }

    #[test]
    fn an_unparseable_reach_is_refused_a_price() {
        let pool = test_pool();
        seed_row(&pool, "c-weird", "not-a-reach", None);
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-weird".to_string()],
            &hints_for(&["c-weird"]),
            &trust,
        );
        assert_eq!(plan.unpriceable, 1);
        assert_eq!(plan.accept_with_provenance, 0);
        assert!(!plan.accepts_with_provenance("c-weird"));
        assert_eq!(plan.for_id("c-weird"), PricedVerification::inert());
    }

    #[test]
    fn an_id_with_no_local_row_is_refused_a_price() {
        let pool = test_pool();
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-absent".to_string()],
            &hints_for(&["c-absent"]),
            &trust,
        );
        assert_eq!(plan.unpriceable, 1);
        assert!(!plan.accepts_with_provenance("c-absent"));
    }

    /// The skips are scoped to the `AdoptPeer` corner: a row that ALREADY
    /// declares a head routes to Hold/Contest, so it must never be priced
    /// cheap — the evidence round-trips it would skip are the ones those arms
    /// depend on.
    #[test]
    fn an_already_declared_row_is_never_priced_cheap() {
        let pool = test_pool();
        seed_row(&pool, "c-declared", "commons", Some("uhCkk-existing"));
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-declared".to_string()],
            &hints_for(&["c-declared"]),
            &trust,
        );
        assert!(!plan.accepts_with_provenance("c-declared"));
        assert_eq!(
            plan.for_id("c-declared").reason,
            PricingReason::NoProvenanceCarried
        );
    }

    #[test]
    fn an_unhinted_id_is_never_priced_cheap() {
        let pool = test_pool();
        seed_row(&pool, "c-unhinted", "commons", None);
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-unhinted".to_string()],
            &PeerHeadHints::new(),
            &trust,
        );
        assert!(!plan.accepts_with_provenance("c-unhinted"));
        assert_eq!(
            plan.for_id("c-unhinted").reason,
            PricingReason::NoProvenanceCarried
        );
    }

    /// BEHAVIOR-NEUTRALITY at the consumption point: every non-Simulacra stage
    /// prices the whole slice full-chain, whatever the rows and hints say — the
    /// pin that "everywhere but an explicit Simulacra declaration is today's
    /// behavior."
    #[test]
    fn every_non_simulacra_stage_prices_the_whole_slice_full_chain() {
        let pool = test_pool();
        seed_row(&pool, "c-1", "commons", None);
        for stage in [
            NetworkStage::Bootstrap,
            NetworkStage::Coordinated,
            NetworkStage::Enforced,
        ] {
            let (stakes, pricer) = declared_gradient(stage);
            let trust = TrustGradient {
                stakes: &stakes,
                pricer: &pricer,
                memo: None,
            };
            let plan = price_adopt_slice(
                &pool,
                &ctx(),
                &["c-1".to_string()],
                &hints_for(&["c-1"]),
                &trust,
            );
            assert!(!plan.accepts_with_provenance("c-1"), "stage={stage:?}");
            assert_eq!(plan.full_chain, 1, "stage={stage:?}");
            assert_eq!(
                plan.for_id("c-1").reason,
                PricingReason::StageRequiresFullChain,
                "stage={stage:?}"
            );
            assert!(!plan.compressed_anything());
        }
    }

    /// The INERT gradient — the default every non-Q7 call site still passes —
    /// cannot cheapen anything even if a Simulacra stage somehow reaches it.
    #[test]
    fn the_inert_pricer_never_cheapens_even_under_a_simulacra_declaration() {
        let pool = test_pool();
        seed_row(&pool, "c-1", "commons", None);
        let stakes = FixedStakesResolver {
            stage: NetworkStage::Simulacra,
            provenance: StakesProvenance::OperatorConfig,
        };
        let pricer = InertPricer;
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-1".to_string()],
            &hints_for(&["c-1"]),
            &trust,
        );
        assert!(!plan.accepts_with_provenance("c-1"));
        assert_eq!(plan.for_id("c-1").reason, PricingReason::PricerInert);
    }

    /// THE scope-key reconciliation proof, end to end through the REAL
    /// resolver: a stakes manifest seeded exactly as the genesis seeder mints
    /// it — `stakes.scope = "genesis-lamad"`, the CORPUS id — prices a
    /// `lamad`-scope (h_app_id) adoption at Simulacra once the operator
    /// declares the bridge. Without this the grant would resolve, log, and
    /// price nothing: the silent mismatch this test exists to refuse.
    #[test]
    fn a_seeded_genesis_lamad_grant_prices_lamad_scope_adoption_at_simulacra() {
        use crate::db::manifests::{insert_manifest, ManifestRow};
        use crate::services::manifest_registry::ManifestRegistry;
        use crate::trust::ManifestStakesResolver;
        use std::sync::Arc;

        let pool = test_pool();
        seed_row(&pool, "c-1", "commons", None);
        {
            let mut conn = pool.get().unwrap();
            let payload = serde_json::json!({
                "schemaVersion": "1",
                "manifestKind": "network-stakes",
                "manifestCid": "stakes:genesis-lamad:sha256-deadbeef",
                "stakes": {
                    "stage": "simulacra",
                    "scope": "genesis-lamad",
                    "grantor": "human-adam-firstman",
                    "environment": "preproduction",
                },
                "corpus": { "corpusId": "genesis-lamad" },
            })
            .to_string();
            insert_manifest(
                &mut conn,
                &ManifestRow {
                    cid: "stakes:genesis-lamad:sha256-deadbeef".to_string(),
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
        }
        let registry = Arc::new(ManifestRegistry::new());
        registry
            .load_from_db(&mut pool.get().unwrap())
            .expect("load manifests");
        let stakes = ManifestStakesResolver::with_declarations(
            registry,
            None,
            // The operator-declared bridge: corpus scope -> runtime scope.
            "genesis-lamad=lamad",
        );
        let trust = TrustGradient::declared(&stakes);

        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-1".to_string()],
            &hints_for(&["c-1"]),
            &trust,
        );
        assert_eq!(plan.scope, "lamad", "the sweep prices by h_app_id");
        assert_eq!(plan.stage, NetworkStage::Simulacra);
        assert_eq!(
            plan.provenance,
            StakesProvenance::Manifest {
                cid: "stakes:genesis-lamad:sha256-deadbeef".to_string()
            },
            "the corpus manifest is the provenance the sweep reports"
        );
        assert!(
            plan.accepts_with_provenance("c-1"),
            "a corpus-scoped grant must actually price the traffic it was minted for"
        );
        assert_eq!(plan.accept_with_provenance, 1);
    }

    /// The negative half of the same proof: with no declared reconciliation the
    /// corpus grant does NOT silently price lamad traffic — it fails closed to
    /// Bootstrap, which is a visible non-effect rather than an invisible one.
    #[test]
    fn a_genesis_lamad_grant_without_a_declared_bridge_prices_nothing_for_lamad() {
        use crate::db::manifests::{insert_manifest, ManifestRow};
        use crate::services::manifest_registry::ManifestRegistry;
        use crate::trust::ManifestStakesResolver;
        use std::sync::Arc;

        let pool = test_pool();
        seed_row(&pool, "c-1", "commons", None);
        {
            let mut conn = pool.get().unwrap();
            let payload = serde_json::json!({
                "manifestKind": "network-stakes",
                "stakes": { "stage": "simulacra", "scope": "genesis-lamad",
                            "grantor": "g", "environment": "preproduction" },
            })
            .to_string();
            insert_manifest(
                &mut conn,
                &ManifestRow {
                    cid: "stakes:genesis-lamad:1".to_string(),
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
        }
        let registry = Arc::new(ManifestRegistry::new());
        registry
            .load_from_db(&mut pool.get().unwrap())
            .expect("load manifests");
        let stakes = ManifestStakesResolver::with_declarations(registry, None, "");
        let trust = TrustGradient::declared(&stakes);

        let plan = price_adopt_slice(
            &pool,
            &ctx(),
            &["c-1".to_string()],
            &hints_for(&["c-1"]),
            &trust,
        );
        assert_eq!(plan.stage, NetworkStage::Bootstrap);
        assert_eq!(plan.provenance, StakesProvenance::BootstrapDefault);
        assert!(!plan.accepts_with_provenance("c-1"));
    }

    #[test]
    fn an_empty_slice_prices_nothing_and_reports_the_resolved_stage() {
        let pool = test_pool();
        let (stakes, pricer) = declared_gradient(NetworkStage::Simulacra);
        let trust = TrustGradient {
            stakes: &stakes,
            pricer: &pricer,
            memo: None,
        };
        let plan = price_adopt_slice(&pool, &ctx(), &[], &PeerHeadHints::new(), &trust);
        assert_eq!(plan.stage, NetworkStage::Simulacra);
        assert_eq!(plan.scope, ctx().h_app_id);
        assert_eq!(plan.accept_with_provenance, 0);
        assert_eq!(plan.full_chain, 0);
    }
}
