//! The governed operations: `observe` appends, `drift` reads, `offer mint` proposes.
//!
//! The refuse-without-mutation shape of `bridges/k8s/src/observe.rs`: every input is parsed,
//! validated and translated BEFORE the sidecar is opened for writing, and every governance check
//! runs before the one transaction that appends. A malformed input, an unknown build, a missing
//! verdict or an unprojected recipe leaves `.eprfs/status/flows.jsonl` byte-identical.
//!
//! Governance refusals (exit 3) name what is missing: the offer is not minted, is Proposed (no
//! distinct Steward's approval) or was withdrawn, or the network has not projected the recipe the
//! observations would pin.

use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_cli::flow::memory::offer::{
    load_offer, offer_cid, offer_intent, offer_standing, OfferStanding,
};
use elohim_epr_cli::flow::registry::{Recipe, Registry};
use elohim_epr_rea::{FlowRecord, FlowStore, OfferDocument, SidecarFlowStore};
use serde::Serialize;

use crate::drift::{drift, DeclaredStage, DriftReport};
use crate::translate::{parse_graph, parse_wfapi, translate, Translation};

pub const DEFAULT_OFFER: &str = "bridges/.epr-meta/offers/jenkins-edge-pipeline.offer.md";
pub const DEFAULT_RECIPES: &str = ".claude/epr-meta/recipes.yaml";
const FLOWS: &str = ".eprfs/status/flows.jsonl";

/// Why an operation refused, and the exit code it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    /// Malformed or unknown input (exit 65). Nothing was written.
    Malformed(String),
    /// A governance precondition is unmet (exit 3). Nothing was written.
    Governance(String),
    /// The sidecar could not be read or written (exit 74).
    Io(String),
}

impl BridgeError {
    pub fn exit_code(&self) -> u8 {
        match self {
            BridgeError::Malformed(_) => 65,
            BridgeError::Governance(_) => 3,
            BridgeError::Io(_) => 74,
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Malformed(m) => {
                write!(f, "refused (malformed input, nothing written): {m}")
            }
            BridgeError::Governance(m) => write!(f, "refused (governance, nothing written): {m}"),
            BridgeError::Io(m) => write!(f, "sidecar: {m}"),
        }
    }
}
impl std::error::Error for BridgeError {}

impl From<crate::translate::Refusal> for BridgeError {
    fn from(r: crate::translate::Refusal) -> Self {
        BridgeError::Malformed(r.0)
    }
}

pub type Result<T> = std::result::Result<T, BridgeError>;

/// Where an operation runs: the repository root, the offer and the recipe registry.
#[derive(Debug, Clone)]
pub struct Context {
    pub root: PathBuf,
    pub offer: OfferDocument,
    pub recipe: Recipe,
}

/// The inputs `observe` / `drift` read: Jenkins' own archive for one build.
#[derive(Debug, Clone)]
pub struct BuildInputs {
    pub stages: String,
    pub graph: Option<String>,
    pub jenkins_base: String,
}

/// The offer's standing, including the state before it exists in the sidecar at all.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "minted")]
pub enum Standing {
    #[serde(rename = "no")]
    Unminted { offer: String },
    #[serde(rename = "yes")]
    Minted(OfferStanding),
}

impl Standing {
    pub fn line(&self) -> String {
        match self {
            Standing::Unminted { offer } => {
                format!("offer {offer} is not minted — `jenkins-bridge offer mint` proposes it")
            }
            Standing::Minted(s) => s.stakes_line(),
        }
    }
}

impl Context {
    pub fn load(root: &Path, offer_path: &str, recipes_path: &str) -> Result<Self> {
        let offer =
            load_offer(root, offer_path).map_err(|e| BridgeError::Malformed(e.to_string()))?;
        let registry = Registry::load(&root.join(recipes_path))
            .map_err(|e| BridgeError::Malformed(e.to_string()))?;
        let pin = &offer.declaration().recipe;
        let recipe = registry
            .recipes
            .into_iter()
            .find(|r| r.id == pin.id && r.version == pin.version)
            .ok_or_else(|| {
                BridgeError::Malformed(format!(
                    "{recipes_path} declares no recipe {}@{} (the offer's pin)",
                    pin.id, pin.version
                ))
            })?;
        Ok(Self {
            root: root.to_path_buf(),
            offer,
            recipe,
        })
    }

    pub fn offer_cid(&self) -> Result<Cid> {
        offer_cid(&self.offer).map_err(|e| BridgeError::Malformed(e.to_string()))
    }

    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>> {
        if !self.root.join(FLOWS).is_file() {
            return Ok(Vec::new());
        }
        SidecarFlowStore::open(&self.root)
            .and_then(|s| s.records())
            .map_err(|e| BridgeError::Io(e.to_string()))
    }

    pub fn declared(&self) -> Vec<DeclaredStage> {
        self.recipe
            .stages
            .iter()
            .map(|s| DeclaredStage {
                name: s.name.clone(),
                exercises: s.exercises.clone(),
            })
            .collect()
    }

    /// The offer's standing, read through the distinct-Steward path. Never a grant.
    pub fn standing(&self) -> Result<Standing> {
        let cid = self.offer_cid()?;
        let minted = self
            .records()?
            .iter()
            .any(|(c, r)| c == &cid && matches!(r, FlowRecord::Intent(_)));
        if !minted {
            return Ok(Standing::Unminted {
                offer: cid.to_string(),
            });
        }
        let standing = offer_standing(&self.root, self.offer.declaration(), &cid)
            .map_err(|e| BridgeError::Governance(e.to_string()))?;
        Ok(Standing::Minted(standing))
    }

    /// Refuse (exit 3) unless the offer is Active, naming the missing verdict.
    pub fn require_active(&self) -> Result<OfferStanding> {
        match self.standing()? {
            Standing::Minted(s) if s.is_active() => Ok(s),
            Standing::Minted(s) => Err(BridgeError::Governance(format!(
                "{} — missing: {}",
                s.stakes_line(),
                s.missing
                    .as_deref()
                    .unwrap_or("a distinct Steward's approving verdict")
            ))),
            unminted => Err(BridgeError::Governance(format!(
                "{} — then a distinct Steward's approving verdict",
                unminted.line()
            ))),
        }
    }

    /// Refuse (exit 3) unless the network has projected the recipe the observations pin: the
    /// sidecar must hold the exact `ProcessSpec` this registry mints (`epr flow project`).
    fn require_projected(&self, records: &[(Cid, FlowRecord)]) -> Result<Cid> {
        let spec = self.recipe.to_process_spec();
        records
            .iter()
            .find_map(|(cid, r)| match r {
                FlowRecord::Spec(s) if s == &spec => Some(*cid),
                _ => None,
            })
            .ok_or_else(|| {
                BridgeError::Governance(format!(
                    "the network has not projected recipe {}@{} — run `epr flow project` first; \
                     the recipe is declared by the network, never by the consumer",
                    spec.id, spec.version
                ))
            })
    }

    /// Parse and translate one build. Pure with respect to the sidecar.
    pub fn translate(&self, inputs: &BuildInputs) -> Result<Translation> {
        let run = parse_wfapi(&inputs.stages)?;
        let graph = inputs.graph.as_deref().map(parse_graph).transpose()?;
        Ok(translate(
            &run,
            graph.as_ref(),
            &self.offer.declaration().spec(),
            self.offer_cid()?,
            &inputs.jenkins_base,
        )?)
    }

    pub fn drift_of(&self, translation: &Translation) -> DriftReport {
        drift(
            &self.declared(),
            &translation.observed,
            &self.offer.declaration().held(),
        )
    }
}

/// What `observe` did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserveOutcome {
    pub build: u64,
    pub basis: &'static str,
    pub process: String,
    pub spec: String,
    pub appended: usize,
    pub present: usize,
    pub stakes: String,
}

/// Translate, check governance, then append the new observations — idempotently, deduped by CID
/// under one sidecar transaction.
pub fn observe(ctx: &Context, inputs: &BuildInputs) -> Result<ObserveOutcome> {
    // 1. Inputs first: a malformed archive refuses before governance is even read.
    let translation = ctx.translate(inputs)?;
    // 2. Governance: an active offer and a network-projected recipe.
    let standing = ctx.require_active()?;
    let spec_cid = ctx.require_projected(&ctx.records()?)?;
    // 3. The one write: under the exclusive lock, append only what is not already there.
    let store = SidecarFlowStore::open(&ctx.root).map_err(|e| BridgeError::Io(e.to_string()))?;
    let mut tx = store
        .transaction()
        .map_err(|e| BridgeError::Io(e.to_string()))?;
    let existing: std::collections::HashSet<Cid> = tx
        .records()
        .map_err(|e| BridgeError::Io(e.to_string()))?
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    let (mut appended, mut present) = (0, 0);
    for observation in translation.observations.iter().cloned() {
        if existing.contains(&observation.cid()?) {
            present += 1;
            continue;
        }
        tx.append(observation.into_record())
            .map_err(|e| BridgeError::Io(e.to_string()))?;
        appended += 1;
    }
    Ok(ObserveOutcome {
        build: translation.build,
        basis: translation.basis,
        process: translation.process_cid.to_string(),
        spec: spec_cid.to_string(),
        appended,
        present,
        stakes: standing.stakes_line(),
    })
}

/// Mint (propose) the offer's Intent. Idempotent; activation is a Steward's verdict, never this.
pub fn mint_offer(ctx: &Context) -> Result<(Cid, bool)> {
    let intent = offer_intent(&ctx.offer);
    let cid = ctx.offer_cid()?;
    let store = SidecarFlowStore::open(&ctx.root).map_err(|e| BridgeError::Io(e.to_string()))?;
    let mut tx = store
        .transaction()
        .map_err(|e| BridgeError::Io(e.to_string()))?;
    let present = tx
        .records()
        .map_err(|e| BridgeError::Io(e.to_string()))?
        .iter()
        .any(|(c, _)| c == &cid);
    if !present {
        tx.append(FlowRecord::Intent(intent))
            .map_err(|e| BridgeError::Io(e.to_string()))?;
    }
    Ok((cid, !present))
}
