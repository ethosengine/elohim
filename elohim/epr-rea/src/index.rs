//! An index is a measure — the kinds that make a searchable shard a middot fold.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md`
//! §2 (the declaration and its bounds) and §7 (the invariants these types make structural).
//! Research: `genesis/research/local-first-to-council-memory-search-seams-2026-09-12.md` §1.5.
//!
//! Nothing here is a new DHT entry type. An [`IndexMeasure`] declaration is a Manifest EPR; a
//! [`FoldAttestation`] is an Attestation EPR; a shard is a local projection rebuilt from atoms.
//! The types are protocol vocabulary so that the recall executor, the storage service, and a
//! pool custodian all name the same thing: the procedure a candidate was produced by, printed
//! as a CID on every result.
//!
//! Three invariants are refused by construction rather than documented:
//!
//! 1. **The private chain never enters a fold.** [`SurfaceRule::new`] refuses every kind in
//!    [`PRIVATE_CHAIN_KINDS`]; there is no constructor that admits one.
//! 2. **Demotion, never deletion.** [`Retention`] has no delete variant.
//! 3. **A fold reports its own completion honestly.** [`FoldAttestation::is_complete`] is true
//!    only for [`FoldState::Complete`]; a degraded or failed fold is never a complete one.
//!
//! What this module does NOT decide: which engine holds a shard (SQLite with FTS5 and a vector
//! table is the recommendation, not a type), how a lens narrows a query (the carrier's seam), or
//! who may replicate what (seam 2). Those compose with these kinds; they are not restated here.
//!
//! **Admission (the requisite-variety guidestar §3a).** A primitive is admitted when a SECOND
//! independent framework needs the same missing distinction. "An index is a declared procedure
//! whose every custodian proves its own completion" is needed by middot (the measure as a fold)
//! and, independently, by the closest prior art's projection owners and completion manifests
//! (SuperLocalMemory 4.0, arXiv 2608.08253) and by the incumbent's offline index build — so
//! [`IndexMeasure`] and [`FoldAttestation`] are admitted. A *band* of reach rings (floor and
//! ceiling on one quantity) is one framework and therefore a **hold** under the same rule;
//! [`ReachBound`] is deliberately a single bound with a [`Sense`], mirroring `Commitment.bound`
//! being singular: a ceiling on one machine, a floor at the pool.

use cid::Cid;
use elohim_epr::kind::EprKind;
use elohim_epr::measure::Period;
use elohim_epr::reach::Reach;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{atom_cid, AgentRef, Bound, PinnedRef, Sense};

/// Kinds that live on an agent's private source chain and are excluded from every fold that
/// could leave the device. Adding a kind here widens the refusal; it never narrows it.
pub const PRIVATE_CHAIN_KINDS: &[EprKind] = &[EprKind::AttentionTending];

/// Errors specific to index declarations. Kept local so the crate's [`crate::error::FabricError`]
/// vocabulary is not widened by a slice that mints no new record kind.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    #[error("surface rule admits a private-chain kind: {0:?}")]
    PrivateChainKind(EprKind),
    #[error("surface rule names no kinds and no paths")]
    EmptySurface,
    #[error("ranking needs an embedding but the measure pins no model")]
    ModelPinMissing,
    #[error("the measure pins a model that no ranking uses")]
    ModelPinUnused,
}

/// The one embedding a semantic index runs under. The model's bytes are content-addressed so a
/// candidate can name what it was embedded by; a changed pin is a new measure version, never a
/// silent re-embed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPin {
    pub model_bytes: Cid,
    /// SPDX identifier of the model's license — a pin without a license is not admissible.
    pub license: String,
    pub dims: u32,
}

/// Distance for a vector ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorMetric {
    Cosine,
    Dot,
    L2,
}

/// How a fold orders candidates. Every variant is a *known* method — an `IndexMeasure` is by
/// definition a declared procedure, which is why [`IndexMeasure::ranking_known`] is constant.
/// An opaque outside provider is not an `IndexMeasure`; it is declared `ranking_known: false`
/// at the carrier's `Provider` seam instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum RankingMethod {
    /// Lexical, BM25 over an inverted index (SQLite FTS5 is the recommended engine).
    Bm25,
    /// Dense similarity under the measure's [`ModelPin`].
    Vector { metric: VectorMetric },
    /// Adjacency and standing edges from the native content-graph resolver, named by id.
    Graph { resolver: String },
    /// Rank fusion FOR ORDER under a recipe with a CID. Each producer's rank is printed beside
    /// the fused order. Standing never enters the fusion — that is the sum the search epic
    /// forbids, and it is refused at the carrier's render floor, not here.
    Fused {
        recipe: Cid,
        producers: Vec<RankingMethod>,
    },
}

impl RankingMethod {
    /// Does this method, or any producer it fuses, rank under an embedding?
    pub fn needs_model(&self) -> bool {
        match self {
            RankingMethod::Vector { .. } => true,
            RankingMethod::Fused { producers, .. } => producers.iter().any(Self::needs_model),
            RankingMethod::Bm25 | RankingMethod::Graph { .. } => false,
        }
    }
}

/// Which atoms a fold may contain. Constructed only through [`SurfaceRule::new`], which refuses
/// the private chain, so a hand-built value cannot admit what a declared one refuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRule {
    kinds: Vec<EprKind>,
    /// Path or glob patterns over an EPRFS scope; empty when the scope is a reach ring.
    paths: Vec<String>,
}

impl SurfaceRule {
    /// Refuses any private-chain kind and an empty rule.
    pub fn new(kinds: Vec<EprKind>, paths: Vec<String>) -> std::result::Result<Self, IndexError> {
        if let Some(k) = kinds.iter().find(|k| PRIVATE_CHAIN_KINDS.contains(k)) {
            return Err(IndexError::PrivateChainKind(*k));
        }
        if kinds.is_empty() && paths.is_empty() {
            return Err(IndexError::EmptySurface);
        }
        Ok(Self { kinds, paths })
    }

    pub fn kinds(&self) -> &[EprKind] {
        &self.kinds
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Re-check a deserialized value against the same refusal the constructor applies.
    pub fn validate(&self) -> std::result::Result<(), IndexError> {
        Self::new(self.kinds.clone(), self.paths.clone()).map(|_| ())
    }
}

/// One bound on the reach ring a fold may contain, with the side that is safe. On one machine
/// the bound is a **ceiling** (`SelfScope`: nothing more open enters); at a pool it is a
/// **floor** (`Commons`: nothing less open enters). One bound, one [`Sense`] — the same
/// singular shape `Commitment.bound` has. A band (floor *and* ceiling) is a hold under the
/// guidestar's admission rule until a second framework asks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachBound {
    pub ring: Reach,
    pub sense: Sense,
}

impl ReachBound {
    pub const fn ceiling(ring: Reach) -> Self {
        Self {
            ring,
            sense: Sense::Ceiling,
        }
    }

    pub const fn floor(ring: Reach) -> Self {
        Self {
            ring,
            sense: Sense::Floor,
        }
    }

    /// May an atom at `reach` enter a fold declared with this bound? Comparison is by
    /// [`Reach::openness`], never by the enum's declaration order (see the note on `Reach`).
    pub fn admits(&self, reach: Reach) -> bool {
        match self.sense {
            Sense::Ceiling => reach.openness() <= self.ring.openness(),
            Sense::Floor => reach.openness() >= self.ring.openness(),
        }
    }
}

/// What a fold does with a fact that stopped being true. There is no delete variant: a fact is
/// demoted with a timestamp and stays findable `as_of` the day it held. This is the Living
/// Memory epic's forgetting made mechanical, and the shape a revoke-reach-everywhere lands as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "kebab-case")]
pub enum Retention {
    /// Demote a superseded fact after it has been superseded for this long.
    DemoteAfter { count: u32, per: Period },
    /// Never demote; superseded facts stay ranked under the temporal channel's own validity.
    Keep,
}

/// A measure whose fold is a searchable shard. Declared once; a change is a new version of the
/// pinned measure, and the declaration's CID (see [`IndexMeasure::cid`]) is what every candidate
/// prints as its method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMeasure {
    /// The middot declaration this index instantiates, as `id@version`.
    pub measure: PinnedRef,
    /// The chunking procedure, content-addressed.
    pub chunk_rule: Cid,
    /// `None` for a lexical-only or graph-only index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<ModelPin>,
    pub ranking: RankingMethod,
    /// Which rings may enter the fold: a ceiling on one machine, a floor at a pool.
    pub reach: ReachBound,
    /// Which atoms may enter the fold (the private chain never can).
    pub surfaces: SurfaceRule,
    pub retention: Retention,
    /// How far behind the heads a fold may lag, as a bound the controller reconciles against —
    /// the replacement for a manual re-index gate.
    pub fold_lag: Bound,
}

impl IndexMeasure {
    /// The declaration's content address — the method CID printed on every candidate.
    pub fn cid(&self) -> Result<Cid> {
        atom_cid(self)
    }

    /// Always true: an `IndexMeasure` is a declared procedure. Opaque providers are declared
    /// `ranking_known: false` at the carrier's `Provider` seam and are never index measures.
    pub const fn ranking_known(&self) -> bool {
        true
    }

    /// May an atom at `reach` enter this measure's fold?
    pub fn admits(&self, reach: Reach) -> bool {
        self.reach.admits(reach)
    }

    /// A semantic ranking without a model pin is malformed; a lexical ranking with one is a
    /// declaration of a model that nothing uses. Both are refused, as is a surface that admits
    /// the private chain, so a deserialized value is re-checkable against the same rules.
    pub fn validate(&self) -> std::result::Result<(), IndexError> {
        self.surfaces.validate()?;
        match (self.ranking.needs_model(), self.embedding.is_some()) {
            (true, false) => Err(IndexError::ModelPinMissing),
            (false, true) => Err(IndexError::ModelPinUnused),
            _ => Ok(()),
        }
    }
}

/// The half-open range of the DHT keyspace a custodian holds, when a fold is a pool shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcRange {
    pub start: u32,
    pub end: u32,
}

/// What a fold holds, by count and by content address of its manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShardManifest {
    /// `None` on one machine; `Some` when the fold is one arc of a pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc: Option<ArcRange>,
    pub atoms: u64,
    pub bytes: u64,
    pub manifest: Cid,
}

/// The state a custodian reports for its own fold. Mirrors the shape the closest prior art
/// converged on (each projection owner proves its own completion): complete, degraded after a
/// retry, or failed with the reason — never a silent absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum FoldState {
    Complete,
    Degraded { retried: u32 },
    Failed { why: String },
}

/// Every custodian of a fold proves its own completion. This is the fold receipt: which measure,
/// which shard, which heads it was folded at, in what state, by whom, when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldAttestation {
    /// [`IndexMeasure::cid`] of the declaration this fold executed.
    pub measure: Cid,
    pub shard: ShardManifest,
    /// `(atom, head)` pairs the fold was taken at; fold lag is measured against these.
    pub heads_at: Vec<(Cid, Cid)>,
    pub state: FoldState,
    pub attested_by: AgentRef,
    /// Unix seconds.
    pub at: i64,
}

impl FoldAttestation {
    /// The attestation's own content address.
    pub fn cid(&self) -> Result<Cid> {
        atom_cid(self)
    }

    /// True only for [`FoldState::Complete`]. A degraded fold is a fold that had to retry, and a
    /// failed one is a failure; neither is reported as complete.
    pub fn is_complete(&self) -> bool {
        matches!(self.state, FoldState::Complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The canonical mint, never a second one.
    use elohim_epr::cid::compute_cid;

    fn cid(tag: &str) -> Cid {
        compute_cid(tag.as_bytes())
    }

    fn lexical_surface() -> SurfaceRule {
        SurfaceRule::new(vec![EprKind::Content, EprKind::Manifest], vec![]).expect("admissible")
    }

    fn local_measure() -> IndexMeasure {
        IndexMeasure {
            measure: PinnedRef {
                id: "recall-lexical".into(),
                version: 1,
            },
            chunk_rule: cid("chunk:heading-window"),
            embedding: None,
            ranking: RankingMethod::Bm25,
            reach: ReachBound::ceiling(Reach::SelfScope),
            surfaces: lexical_surface(),
            retention: Retention::DemoteAfter {
                count: 90,
                per: Period::Day,
            },
            fold_lag: Bound::new(8.0, "heads".into(), 50.0).expect("a bound"),
        }
    }

    #[test]
    fn private_chain_kinds_are_refused_by_construction() {
        let err = SurfaceRule::new(vec![EprKind::Content, EprKind::AttentionTending], vec![])
            .expect_err("the private chain never enters a fold");
        assert_eq!(err, IndexError::PrivateChainKind(EprKind::AttentionTending));
        assert_eq!(
            SurfaceRule::new(vec![], vec![]).expect_err("empty"),
            IndexError::EmptySurface
        );
    }

    #[test]
    fn reach_bound_compares_by_openness_not_declaration_order() {
        let local = ReachBound::ceiling(Reach::SelfScope);
        assert!(local.admits(Reach::Private));
        assert!(local.admits(Reach::SelfScope));
        assert!(!local.admits(Reach::Intimate));
        assert!(!local.admits(Reach::Commons));

        let pool = ReachBound::floor(Reach::Commons);
        assert!(pool.admits(Reach::Commons));
        assert!(!pool.admits(Reach::Public));
        assert!(!pool.admits(Reach::Private));

        let household = ReachBound::ceiling(Reach::Trusted);
        assert!(household.admits(Reach::Intimate));
        assert!(!household.admits(Reach::Familiar));
    }

    #[test]
    fn a_semantic_ranking_needs_its_model_pin_and_a_lexical_one_refuses_an_unused_pin() {
        let mut m = local_measure();
        m.validate().expect("lexical, unpinned");

        m.ranking = RankingMethod::Vector {
            metric: VectorMetric::Cosine,
        };
        assert_eq!(m.validate(), Err(IndexError::ModelPinMissing));

        m.embedding = Some(ModelPin {
            model_bytes: cid("model:v1"),
            license: "Apache-2.0".into(),
            dims: 384,
        });
        m.validate().expect("vector, pinned");

        m.ranking = RankingMethod::Bm25;
        assert_eq!(m.validate(), Err(IndexError::ModelPinUnused));

        m.ranking = RankingMethod::Fused {
            recipe: cid("recipe:rrf"),
            producers: vec![
                RankingMethod::Bm25,
                RankingMethod::Vector {
                    metric: VectorMetric::Cosine,
                },
            ],
        };
        m.validate()
            .expect("a fusion that includes a vector producer uses the pin");
    }

    #[test]
    fn measure_cid_is_stable_across_roundtrip_and_moves_on_a_model_pin() {
        let m = local_measure();
        let a = m.cid().unwrap();
        let json = serde_json::to_string(&m).unwrap();
        let back: IndexMeasure = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert_eq!(
            back.cid().unwrap(),
            a,
            "a roundtrip does not re-address the declaration"
        );
        back.validate()
            .expect("a deserialized declaration re-validates");

        let mut pinned = m.clone();
        pinned.embedding = Some(ModelPin {
            model_bytes: cid("model:v1"),
            license: "Apache-2.0".into(),
            dims: 384,
        });
        pinned.ranking = RankingMethod::Vector {
            metric: VectorMetric::Cosine,
        };
        let b = pinned.cid().unwrap();
        assert_ne!(
            a, b,
            "a changed pin is a new measure, never a silent re-embed"
        );

        let mut repinned = pinned.clone();
        repinned.embedding.as_mut().unwrap().model_bytes = cid("model:v2");
        assert_ne!(
            repinned.cid().unwrap(),
            b,
            "new model bytes, new method CID"
        );
    }

    #[test]
    fn ranking_is_always_a_known_method_and_fusion_names_its_producers() {
        let m = local_measure();
        assert!(m.ranking_known());
        let fused = RankingMethod::Fused {
            recipe: cid("recipe:rrf"),
            producers: vec![
                RankingMethod::Bm25,
                RankingMethod::Vector {
                    metric: VectorMetric::Cosine,
                },
            ],
        };
        let json = serde_json::to_value(&fused).unwrap();
        assert_eq!(json["method"], "fused");
        assert_eq!(json["producers"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn retention_serializes_only_demotion_or_keep() {
        let d = serde_json::to_value(Retention::DemoteAfter {
            count: 90,
            per: Period::Day,
        })
        .unwrap();
        assert_eq!(d["policy"], "demote-after");
        let k = serde_json::to_value(Retention::Keep).unwrap();
        assert_eq!(k["policy"], "keep");
        assert!(serde_json::from_str::<Retention>(r#"{"policy":"delete"}"#).is_err());
    }

    #[test]
    fn a_fold_is_complete_only_when_it_says_complete() {
        let m = local_measure();
        let shard = ShardManifest {
            arc: Some(ArcRange {
                start: 0,
                end: 1 << 20,
            }),
            atoms: 3,
            bytes: 4096,
            manifest: cid("shard:0"),
        };
        let mut att = FoldAttestation {
            measure: m.cid().unwrap(),
            shard,
            heads_at: vec![(cid("atom:a"), cid("head:a1"))],
            state: FoldState::Complete,
            attested_by: AgentRef("agent:custodian@household".into()),
            at: 1_757_000_000,
        };
        assert!(att.is_complete());
        let complete = att.cid().unwrap();

        att.state = FoldState::Degraded { retried: 1 };
        assert!(!att.is_complete());
        assert_ne!(
            att.cid().unwrap(),
            complete,
            "state is inside the attested bytes"
        );

        att.state = FoldState::Failed {
            why: "head moved mid-fold".into(),
        };
        assert!(!att.is_complete());
    }
}
