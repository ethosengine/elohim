//! `epr flow report` — the READ leg that turns declared bounds into witnessed outcomes.
//!
//! This is the native replacement for the memory kit's `placement-audit.py --headline` gate
//! dispatcher (`genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`,
//! station one). The kit computes each headline line by shelling out to a script that owns both
//! its threshold and its history; nothing else can read either, and nothing verifies either. The
//! replacement inverts that: the **threshold is a declared row** in `.claude/epr-meta`, the
//! **history is a fold** in the flows sidecar, and this module is only the arithmetic between
//! them plus a rendering.
//!
//! **Three-valued, never two.** Each bound resolves to `passed | failed | skipped`, as the
//! verification-as-memoized-derivation guidestar requires. `skipped` is the load-bearing member:
//! a bound with no fold means *nobody measured this*, and the kit's habit of rendering that as a
//! zero is exactly the false reassurance the replacement exists to end. A skipped outcome names
//! the measure whose fold is missing, so the reader learns what to run rather than that something
//! was fine.
//!
//! **Keyed subject × measure@version × env.** A fold satisfies a bound only when it names the
//! same subject, the same pinned measure, and at least the env the bound declares. A fold taken
//! under a different env is evidence of a different question and is not admitted as evidence of
//! this one.
//!
//! **Method pinning.** The `--json` payload carries the raw-codec CIDs of the exact registry bytes
//! read. Two reports over the same folds but different registry bytes are different measurements,
//! and the method CID is what makes that legible instead of invisible.
//!
//! **This report has no teeth, by construction.** It always exits successfully, even with a failed
//! bound, because enforcement class is declared on the row (`deny`/`ask`/`inject`/`measure`) and
//! belongs to whichever context consumes the outcome. A report that also blocked would be a second
//! authority over the same rows — the substrate-floor discipline says the floor projects truth and
//! the ceiling decides what to do about it.
//!
//! Sibling module note: `crate::report` (no `flow::` prefix) is the *check/doctor* finding
//! vocabulary — five-valued (`pass|info|warn|refer|fail`), reach-blocking, gate-shaped. Its
//! [`Finding`] type is reused here through [`BoundOutcome::to_finding`] so a gate can absorb a
//! bound outcome without inventing a second vocabulary; the three-valued `OutcomeStatus` below
//! stays distinct because `skipped` has no honest spelling in a five-valued gate enum, and
//! collapsing it onto `info` is precisely the "unmeasured reads as fine" failure.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::Serialize;

use super::measures::{
    default_measures, default_policies, Bound, Derive, MeasureRef, Registry, SurfaceWalk,
};
use super::note::{
    normalize_subject, ENV_SLOT_PREFIX, MEASURE_SLOT_PREFIX, REPO_SUBJECT, UNIT_SLOT_PREFIX,
    VALUE_SLOT_PREFIX,
};
use super::{short_cid, FlowError, FlowResult};
use crate::report::{Finding, FindingStatus};

/// The slot-0 tag every structured observation carries.
const OBSERVATION_TAG: &str = "run:observation";

/// The headline slots, in the order the SessionStart reader expects them.
///
/// Order and prefix are a compatibility contract, not a preference: the root `CLAUDE.md` declares
/// `cleanup:` and `scope:` as triggers an agent reads at session start. Renaming or reordering
/// them would silently retire two gospel-declared triggers, which is a worse outcome than any
/// tidier vocabulary is worth.
/// Slot 0 was `memkit` until 2026-09-11: the kit it measured was deleted, and the question that
/// slot answered — "is the memory layer paying for itself?" — is now asked of the governed recall
/// journey, whose honesty reading is the first thing a session should see.
const HEADLINE_ORDER: [&str; 5] = ["recall", "mempalace", "cleanup", "scope", "budget"];

/// The line prefix each slot prints under.
fn slot_prefix(slot: &str) -> &'static str {
    match slot {
        "recall" => "recall",
        // `memkit` left HEADLINE_ORDER on 2026-09-11 but stays in the VOCABULARY: its retired bound
        // still declares that slot, and a reader asking for it by name must get its own word back
        // rather than the fallback's.
        "memkit" => "memkit",
        "mempalace" => "mempalace",
        "cleanup" => "cleanup",
        "scope" => "scope",
        _ => "memory-budget",
    }
}

/// Which headline slot a bound belongs to: its declared `headline:` key, else derived from the
/// measure id.
///
/// The derivation exists because the rows are authored by another owner. Requiring an explicit
/// `headline:` on every row would mean a correctly-declared bound silently vanishing from the
/// headline for want of a key nobody knew to write; deriving from the measure id means the
/// conventionally-named measures land in their slot with no extra declaration, and anything
/// unconventional can still say so outright.
fn headline_slot(bound: &Bound) -> Option<&'static str> {
    if let Some(declared) = &bound.headline {
        let declared = declared.trim().to_ascii_lowercase();
        return match declared.as_str() {
            "recall" => Some("recall"),
            // The retired kit rows still declare their old slot; naming it here keeps them out of
            // the stderr "unknown slot" path without putting them back in the headline.
            "memkit" => Some("memkit"),
            "mempalace" => Some("mempalace"),
            "cleanup" => Some("cleanup"),
            "scope" => Some("scope"),
            "budget" | "memory-budget" => Some("budget"),
            other => {
                // A typo in a file this crate does not own must not kill the headline: say so on
                // stderr and fall through to derivation.
                eprintln!(
                    "report: bound `{}` declares unknown headline slot `{other}` — \
                     falling back to the measure-id derivation",
                    bound.id
                );
                derived_slot(&bound.measure)
            }
        };
    }
    derived_slot(&bound.measure)
}

fn derived_slot(measure: &MeasureRef) -> Option<&'static str> {
    let id = measure.id.as_str();
    // ONE of the four recall-journey middot owns the slot. The other three are measured against
    // their own ceilings and reported by `--bound`; folding them all into the headline would make
    // the line mean "whichever recall row the registry happens to list first".
    if id.starts_with("recall-unmetered-bytes") {
        Some("recall")
    } else if id.starts_with("memkit") {
        Some("memkit")
    } else if id.starts_with("mempalace") {
        Some("mempalace")
    } else if id.starts_with("cleanup") {
        Some("cleanup")
    } else if id.starts_with("scope") {
        Some("scope")
    } else if id.starts_with("memory-index") {
        Some("budget")
    } else {
        None
    }
}

/// The three-valued outcome of one bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Passed,
    Failed,
    /// Nobody measured this. Never a zero, never a pass.
    Skipped,
}

impl OutcomeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OutcomeStatus::Passed => "passed",
            OutcomeStatus::Failed => "failed",
            OutcomeStatus::Skipped => "skipped",
        }
    }
}

/// The watermarks a bound was evaluated against. Both keys are always present — a null soft
/// watermark is a declaration ("this row has one threshold"), and omitting the key would make it
/// indistinguishable from a reader that forgot to look.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Watermarks {
    pub soft: Option<f64>,
    pub hard: Option<f64>,
}

/// One witnessed outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundOutcome {
    /// The declared bound's pinned id.
    pub bound: String,
    /// The measure it consumes.
    pub measure: String,
    /// The subject evaluated — the bound's declared subject, or the fold's when the bound declared
    /// none, or `*` when neither exists.
    pub subject: String,
    pub outcome: OutcomeStatus,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub watermarks: Watermarks,
    /// The CID of the fold this outcome was derived from; absent exactly when `skipped`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fold_cid: Option<String>,
    /// Which registry home declared the bound (`lens` | `ceiling`).
    pub source: String,
    /// The declaring row's rung on the Precedent ladder, carried through uninterpreted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    /// The comparator the watermarks were read under. On the payload because "250 past 120" and
    /// "1 having reached 1" are different claims, and a consumer that could not tell them apart
    /// would re-derive the inversion this key exists to end.
    pub compare: String,
    /// How the value was computed, when it was an accumulation rather than a reading. Absent on an
    /// ordinary bound, so every pre-derive payload stays byte-identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derive: Option<String>,
    /// The measure whose observation drains this accumulation — the reset a person would append.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_measure: Option<String>,
    /// The reset the count is measured FROM, absent when counting since the beginning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_fold_cid: Option<String>,
    /// How many folds the count was computed over — the difference between a witnessed zero and
    /// an unmeasured one, in a number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributing_folds: Option<usize>,
    /// Which policy recipe produced this outcome. Present on the outcome as well as on its group
    /// so a flattening consumer never loses the provenance.
    pub recipe: RecipeRef,
}

impl BoundOutcome {
    /// Project onto the gate-shaped [`Finding`] vocabulary so `check`/`doctor` can absorb a bound
    /// outcome without a second enum.
    ///
    /// The projection is lossy in one direction on purpose: `skipped` becomes a non-blocking
    /// `Warn` rather than `Info`, because an unmeasured bound is a gap in evidence and a gate that
    /// filed it as information would be reporting silence as health.
    pub fn to_finding(&self) -> Finding {
        let status = match self.outcome {
            OutcomeStatus::Passed => FindingStatus::Pass,
            OutcomeStatus::Failed => FindingStatus::Fail,
            OutcomeStatus::Skipped => FindingStatus::Warn,
        };
        Finding::new(
            format!("bound:{}", self.bound),
            status,
            self.summary.clone(),
        )
        .detail(format!(
            "subject {} · measure {}",
            self.subject, self.measure
        ))
    }
}

/// The registry bytes one recipe was derived from, plus the recipe's name.
///
/// Carried on every outcome as well as on its group. The duplication is deliberate: an outcome
/// lifted out of its group by a flattening consumer must still be able to say which policy set
/// produced it, and a provenance that survives only in the container is a provenance that survives
/// only until someone iterates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRef {
    pub name: String,
    pub measures_cid: Option<String>,
    pub policies_cid: Option<String>,
}

impl RecipeRef {
    /// `<name>@<short-cid>` — the orientation handle printed under the headline.
    ///
    /// The short is the MEASURES pin, not a composite of both files: a combined address would need
    /// an encoding this crate would have to invent, and inventing an identity encoding to save four
    /// characters of output is exactly the re-derivation the addressing homes exist to prevent. The
    /// full pair stays in the payload, which is where a verifier reads it.
    pub fn handle(&self) -> String {
        let short = self
            .measures_cid
            .as_deref()
            .and_then(|raw| raw.parse::<Cid>().ok())
            .map(|cid| short_cid(&cid))
            .or_else(|| {
                self.policies_cid
                    .as_deref()
                    .and_then(|raw| raw.parse::<Cid>().ok())
                    .map(|cid| short_cid(&cid))
            })
            .unwrap_or_else(|| "no-registry".to_string());
        format!("{}@{short}", self.name)
    }
}

/// Counts, so a consumer does not have to fold the outcome list to learn the shape.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Totals {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl Totals {
    fn of(outcomes: &[BoundOutcome]) -> Self {
        Self {
            passed: count(outcomes, OutcomeStatus::Passed),
            failed: count(outcomes, OutcomeStatus::Failed),
            skipped: count(outcomes, OutcomeStatus::Skipped),
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            passed: self.passed + other.passed,
            failed: self.failed + other.failed,
            skipped: self.skipped + other.skipped,
        }
    }
}

/// One policy recipe's outcomes: a measures+policies pair evaluated over the same folds.
///
/// The policy set is ONE LENS among several possible ones (operator ruling, 2026-09-10). The same
/// records read through a different declared recipe are a different reading, not a correction of
/// the first — so recipes are grouped side by side rather than merged, and the primary is named
/// rather than assumed.
/// One bound the registry has RETIRED by supersession.
///
/// Not an outcome. A superseded bound is not `skipped` — `skipped` means nobody measured something
/// they should have, and rendering a deliberate retirement that way turns a decision into an
/// accusation. It leaves evaluation and is listed here with the lineage the registry's never-delete
/// rule preserves, so a reader can still ask what happened to a bound they remember.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetiredBound {
    pub bound: String,
    pub measure: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub reason: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeReport {
    pub recipe: RecipeRef,
    /// Whether this recipe's lines are the ones the headline prints.
    pub primary: bool,
    pub totals: Totals,
    pub outcomes: Vec<BoundOutcome>,
    /// Bounds excluded from evaluation because the registry superseded them. Counted separately
    /// from `totals`, which stays a count of MEASUREMENTS.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub retired: Vec<RetiredBound>,
}

/// The machine-facing report: recipes side by side, never merged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPayload {
    pub command: String,
    pub recipes: Vec<RecipeReport>,
    /// The `scope:` slot's line, derived rather than folded.
    ///
    /// Scope is the one headline slot whose fact is NOT a magnitude. "3 to hold (local-conductor,
    /// owned-substrate)" carries which capabilities and in which direction; a watermark over a
    /// count cannot say either, and the root `CLAUDE.md` declares those exact spellings as session
    /// triggers. So the slot prints the derivation `epr flow report scope` owns, while the declared
    /// `scope-pending-moves@1` bound still evaluates and appears in the outcomes — a watermark over
    /// the same number, in the plane where watermarks live.
    ///
    /// `None` when the derivation could not be taken — no `genesis/manifests/cluster-state.yaml`, or
    /// an unreadable tree — in which case the slot falls back to the bound outcome and, failing
    /// that, says no bound is declared. A tree with no substrate manifest has not been found to
    /// match the substrate; it has not been asked, and "aligned" there would be a false all-clear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_line: Option<String>,
    /// The same derivation's pending-move count — the `scope-pending-moves@1` magnitude, carried on
    /// the payload so a caller folds it without taking the whole zone walk a second time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_pending_moves: Option<usize>,
    /// Across every recipe. A convenience roll-up, NOT an authority: two recipes disagreeing about
    /// one subject is a real disagreement, and this number does not adjudicate it.
    pub totals: Totals,
}

impl ReportPayload {
    /// The recipe whose lines the headline prints.
    pub fn primary(&self) -> Option<&RecipeReport> {
        self.recipes
            .iter()
            .find(|r| r.primary)
            .or_else(|| self.recipes.first())
    }

    /// The primary recipe's outcomes — the flattened read most callers want.
    pub fn outcomes(&self) -> &[BoundOutcome] {
        self.primary().map(|r| r.outcomes.as_slice()).unwrap_or(&[])
    }
}

/// One measures+policies pair, resolved by the CLI shell.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub measures: PathBuf,
    pub policies: PathBuf,
}

impl Recipe {
    /// The `.claude/epr-meta` pair — the fallback when no recipe is declared anywhere.
    pub fn claude_default(root: &Path) -> Self {
        Self {
            name: "claude-epr-meta".into(),
            measures: default_measures(root),
            policies: default_policies(root),
        }
    }

    /// A recipe rooted at a directory holding `measures.yaml` and `policies.yaml`.
    pub fn at_dir(root: &Path, dir: &Path) -> Self {
        let dir = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            root.join(dir)
        };
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "recipe".to_string());
        Self {
            name,
            measures: dir.join("measures.yaml"),
            policies: dir.join("policies.yaml"),
        }
    }
}

/// Every recipe the repository declares, primary first.
///
/// The root manifest is read as a **named set**, not a single path. `policy-recipes:` is a map of
/// declared names to recipes; `policy-recipe:` names which of those keys is the default. That
/// makes plurality the resting state rather than something a caller has to ask for with a flag:
/// adding a second lens to the map is enough for it to be evaluated over the same records on every
/// run, which is what "the policy set is one lens among several" means in practice. It also makes
/// the headline label a *declared name* (`default`) rather than an incidental directory basename.
///
/// Resolution, in order:
///   1. `policy-recipes:` map present and `policy-recipe:` names a key in it → that entry is
///      primary, every other entry is an alt, in declaration order.
///   2. `policy-recipes:` map present but the scalar names no key in it → the scalar is read as a
///      DIRECTORY (the pre-map behaviour) and the map's entries follow as alts, with a notice. A
///      manifest typo narrows the reading; it never empties it.
///   3. No map → the scalar as a directory. This fallback is retained deliberately: a repository
///      that has not adopted the map still gets its declared default.
///   4. No manifest, or no keys at all → the `.claude/epr-meta` pair.
pub fn declared_recipes(root: &Path) -> Vec<Recipe> {
    let manifest = root.join(".epr-meta/manifest.md");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return vec![Recipe::claude_default(root)];
    };
    let Some(front) = frontmatter_block(&text) else {
        return vec![Recipe::claude_default(root)];
    };
    let doc: serde_yaml::Value = match serde_yaml::from_str(front) {
        Ok(doc) => doc,
        Err(err) => {
            // The manifest belongs to the governance plane, not to this crate. An unparseable
            // frontmatter is reported and stepped around rather than propagated: a report that
            // refuses because someone else's document is mid-edit is a report nobody can rely on.
            eprintln!(
                "report: {} frontmatter is unparseable ({err}) — using the .claude/epr-meta pair",
                manifest.display()
            );
            return vec![Recipe::claude_default(root)];
        }
    };

    let default_key = doc
        .get("policy-recipe")
        .and_then(serde_yaml::Value::as_str)
        .map(|s| s.trim().to_string());
    let map = doc.get("policy-recipes").and_then(|v| v.as_mapping());

    let Some(map) = map else {
        return match default_key {
            Some(dir) if !dir.is_empty() => vec![Recipe::at_dir(root, Path::new(&dir))],
            _ => vec![Recipe::claude_default(root)],
        };
    };

    let named: Vec<(String, Recipe)> = map
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?.trim().to_string();
            recipe_from_entry(root, &key, value).map(|recipe| (key, recipe))
        })
        .collect();
    if named.is_empty() {
        return vec![Recipe::claude_default(root)];
    }

    // The default first, then every other declared recipe in declaration order.
    let mut recipes = Vec::with_capacity(named.len() + 1);
    match default_key.as_deref() {
        Some(key) if named.iter().any(|(name, _)| name == key) => {
            recipes.extend(
                named
                    .iter()
                    .filter(|(name, _)| name == key)
                    .map(|(_, r)| r.clone()),
            );
            recipes.extend(
                named
                    .iter()
                    .filter(|(name, _)| name != key)
                    .map(|(_, r)| r.clone()),
            );
        }
        Some(dir) if !dir.is_empty() => {
            let as_dir = Recipe::at_dir(root, Path::new(dir));
            // A scalar still spelled as a path, while the map declares the same pair under a name,
            // is ONE recipe written two ways — the ordinary state of a manifest mid-migration. Emit
            // the NAMED entry (so the headline carries the declared name) and drop the duplicate,
            // rather than evaluating the same lens twice and printing it against itself as an alt.
            if let Some(pos) = named
                .iter()
                .position(|(_, r)| r.measures == as_dir.measures && r.policies == as_dir.policies)
            {
                recipes.push(named[pos].1.clone());
                recipes.extend(
                    named
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != pos)
                        .map(|(_, (_, r))| r.clone()),
                );
            } else {
                eprintln!(
                    "report: policy-recipe `{dir}` names no key in policy-recipes — \
                     reading it as a directory and keeping the map's entries as alternates"
                );
                recipes.push(as_dir);
                recipes.extend(named.into_iter().map(|(_, r)| r));
            }
        }
        _ => recipes.extend(named.into_iter().map(|(_, r)| r)),
    }
    recipes
}

/// One `policy-recipes:` entry: `dir:` alone, or explicit `measures:`/`policies:` paths, or a bare
/// string naming the directory.
fn recipe_from_entry(root: &Path, name: &str, value: &serde_yaml::Value) -> Option<Recipe> {
    if let Some(dir) = value.as_str() {
        let mut recipe = Recipe::at_dir(root, Path::new(dir.trim()));
        recipe.name = name.to_string();
        return Some(recipe);
    }
    let map = value.as_mapping()?;
    let get = |key: &str| {
        map.get(serde_yaml::Value::String(key.into()))
            .and_then(serde_yaml::Value::as_str)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    };
    // `dir:` supplies the defaults; explicit `measures:`/`policies:` override either half, so a
    // recipe may compose two files that do not sit side by side.
    let base = get("dir").map(|dir| Recipe::at_dir(root, Path::new(dir)));
    let measures = get("measures").map(|p| resolve_recipe_path(root, p));
    let policies = get("policies").map(|p| resolve_recipe_path(root, p));
    let (measures, policies) = match (&base, measures, policies) {
        (_, Some(m), Some(p)) => (m, p),
        (Some(b), Some(m), None) => (m, b.policies.clone()),
        (Some(b), None, Some(p)) => (b.measures.clone(), p),
        (Some(b), None, None) => (b.measures.clone(), b.policies.clone()),
        (None, Some(m), None) => (m.clone(), m),
        (None, None, Some(p)) => (p.clone(), p),
        (None, None, None) => return None,
    };
    Some(Recipe {
        name: name.to_string(),
        measures,
        policies,
    })
}

fn resolve_recipe_path(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

/// The primary declared recipe — the one whose lines the headline prints.
pub fn declared_default(root: &Path) -> Recipe {
    declared_recipes(root)
        .into_iter()
        .next()
        .unwrap_or_else(|| Recipe::claude_default(root))
}

/// The `---`-fenced YAML frontmatter block, without its fences.
fn frontmatter_block(text: &str) -> Option<&str> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Options resolved by the CLI shell.
pub struct ReportOptions {
    pub headline: bool,
    pub bound: Option<String>,
    /// The recipes to evaluate, in order. The FIRST is the primary — the one whose lines the
    /// headline prints.
    pub recipes: Vec<Recipe>,
}

impl ReportOptions {
    /// The default reading: every recipe the root manifest declares, primary first.
    ///
    /// Plural by default rather than on request — a second lens declared in `policy-recipes:` is
    /// evaluated on every run with no flag, which is what makes "one lens among several" the
    /// resting state instead of an opt-in.
    pub fn new(root: &Path) -> Self {
        Self {
            headline: false,
            bound: None,
            recipes: declared_recipes(root),
        }
    }

    /// Point the single default recipe at an explicit measures file (test and `--measures` shim).
    pub fn with_measures(mut self, path: PathBuf) -> Self {
        if let Some(first) = self.recipes.first_mut() {
            first.measures = path;
        }
        self
    }

    /// Point the single default recipe at an explicit policies file.
    pub fn with_policies(mut self, path: PathBuf) -> Self {
        if let Some(first) = self.recipes.first_mut() {
            first.policies = path;
        }
        self
    }
}

/// One structured observation read back out of the sidecar.
#[derive(Debug, Clone)]
pub(super) struct Fold {
    cid: String,
    pub(super) measure: MeasureRef,
    pub(super) subject: String,
    pub(super) value: f64,
    unit: Option<String>,
    pub(super) env: BTreeMap<String, String>,
    occurred_at: String,
    /// Sidecar append position — the tie-break when two folds share a HEAD-derived timestamp,
    /// which is the common case rather than the exceptional one (notes are dated by the tree they
    /// were written against, so a whole session's folds share one date).
    seq: usize,
}

/// Evaluate every declared bound of every declared recipe against the folds in the sidecar.
///
/// The folds are read ONCE and every recipe reads the same set. That is what makes two recipes a
/// genuine comparison rather than two runs that happened at different moments: the records are
/// held fixed and only the lens over them changes.
pub fn report(root: &Path, options: &ReportOptions) -> FlowResult<ReportPayload> {
    if options.recipes.is_empty() {
        return Err(FlowError::InvalidArguments(
            "no policy recipe to report over — a report needs at least one declared recipe".into(),
        ));
    }

    let folds = read_folds(root)?;
    let mut recipes = Vec::with_capacity(options.recipes.len());
    for (index, recipe) in options.recipes.iter().enumerate() {
        recipes.push(evaluate_recipe(
            root,
            recipe,
            &folds,
            options.bound.as_deref(),
            index == 0,
        )?);
    }

    let totals = recipes
        .iter()
        .fold(Totals::of(&[]), |acc, r| acc.add(r.totals));

    // Derived, not folded, and never fatal: a scope derivation that fails must not take the whole
    // headline with it — the other four slots are still true.
    // The derivation is only taken when the substrate manifest EXISTS. Without it, "aligned" would
    // be a false all-clear: a tree with no `cluster-state.yaml` has not been found to match the
    // substrate, it has not been asked. In that case the slot falls back to the declared bound, and
    // failing that says no bound is declared — both of which are honest absences.
    let scope = root
        .join("genesis/manifests/cluster-state.yaml")
        .is_file()
        .then(|| super::scope::scope_report(root, None).ok())
        .flatten();
    let scope_line = scope.as_ref().map(|report| report.headline());
    let scope_pending_moves = scope.as_ref().map(|report| report.pending_moves);

    Ok(ReportPayload {
        command: "flow report".into(),
        recipes,
        scope_line,
        scope_pending_moves,
        totals,
    })
}

fn evaluate_recipe(
    root: &Path,
    recipe: &Recipe,
    folds: &[Fold],
    filter: Option<&str>,
    primary: bool,
) -> FlowResult<RecipeReport> {
    let measures = Registry::open_optional(&recipe.measures)?;
    let policies = Registry::open_optional(&recipe.policies)?;
    if measures.is_none() && policies.is_none() {
        return Err(FlowError::InvalidArguments(format!(
            "recipe `{}` has no bound registry to report over: neither {} nor {} exists",
            recipe.name,
            recipe.measures.display(),
            recipe.policies.display()
        )));
    }

    let mut bounds: Vec<Bound> = Vec::new();
    if let Some(reg) = &measures {
        bounds.extend(reg.lens_bounds());
    }
    if let Some(reg) = &policies {
        bounds.extend(reg.ceiling_bounds());
    }
    if let Some(filter) = filter {
        bounds.retain(|bound| bound.matches(filter));
        if bounds.is_empty() {
            return Err(FlowError::InvalidArguments(format!(
                "no declared bound named `{filter}` in recipe `{}` ({} or {})",
                recipe.name,
                recipe.measures.display(),
                recipe.policies.display()
            )));
        }
    }

    let recipe_ref = RecipeRef {
        name: recipe.name.clone(),
        measures_cid: measures.map(|reg| reg.bytes_cid),
        policies_cid: policies.map(|reg| reg.bytes_cid),
    };
    // A superseded bound never reaches `evaluate`. Filtering here rather than inside the evaluator
    // is what keeps `Totals` a count of measurements: a retired row contributes to no bucket,
    // because it is not a measurement that passed, failed, or went untaken.
    let (retired_bounds, live): (Vec<&Bound>, Vec<&Bound>) =
        bounds.iter().partition(|bound| bound.retired());
    let outcomes: Vec<BoundOutcome> = live
        .iter()
        .map(|bound| evaluate(root, bound, folds, &recipe_ref))
        .collect();
    let retired: Vec<RetiredBound> = retired_bounds
        .iter()
        .map(|bound| RetiredBound {
            bound: bound.id.clone(),
            measure: bound.measure.to_string(),
            headline: headline_slot_of(bound),
            superseded_by: bound.superseded_by.clone(),
            reason: bound
                .superseded_reason
                .clone()
                .unwrap_or_else(|| "superseded; the registry records no reason".to_string()),
            source: bound.source.as_str().to_string(),
        })
        .collect();

    Ok(RecipeReport {
        recipe: recipe_ref,
        primary,
        totals: Totals::of(&outcomes),
        outcomes,
        retired,
    })
}

fn count(outcomes: &[BoundOutcome], want: OutcomeStatus) -> usize {
    outcomes.iter().filter(|o| o.outcome == want).count()
}

/// Evaluate one bound against the latest admissible fold.
fn evaluate(root: &Path, bound: &Bound, folds: &[Fold], recipe: &RecipeRef) -> BoundOutcome {
    let watermarks = Watermarks {
        soft: bound.soft,
        hard: bound.hard,
    };
    if let Some(derive) = bound.derive {
        if derive.reads_tree() {
            return evaluate_surface_walk(root, bound, derive, recipe, watermarks);
        }
        if derive.reads_window() {
            return evaluate_rate_over_window(bound, derive, folds, recipe, watermarks, Utc::now());
        }
        return evaluate_derived(bound, derive, folds, recipe, watermarks);
    }
    let latest = folds
        .iter()
        .filter(|fold| admissible(bound, fold))
        .max_by(|a, b| {
            a.occurred_at
                .cmp(&b.occurred_at)
                .then_with(|| a.seq.cmp(&b.seq))
        });

    let Some(fold) = latest else {
        return BoundOutcome {
            bound: bound.id.clone(),
            measure: bound.measure.to_string(),
            subject: bound
                .subject
                .as_deref()
                .map(|s| normalize_subject(s).to_string())
                .unwrap_or_else(|| "*".to_string()),
            outcome: OutcomeStatus::Skipped,
            summary: format!("no fold for {}", bound.measure),
            observed: None,
            unit: None,
            watermarks,
            fold_cid: None,
            source: bound.source.as_str().to_string(),
            binding: bound.binding.clone(),
            compare: bound.compare.as_str().to_string(),
            derive: None,
            reset_measure: None,
            reset_fold_cid: None,
            contributing_folds: None,
            recipe: recipe.clone(),
        };
    };

    let unit_suffix = fold
        .unit
        .as_ref()
        .map(|u| format!(" {u}"))
        .unwrap_or_default();
    let observed = fold.value;
    // The comparator is the ROW's, not this function's. Under the default `above` a watermark is
    // the last acceptable value (24,000 is within a 24,000 bound; 24,001 is not); under
    // `at-or-above` the watermark itself is the breach, which is what a trigger count declared as
    // "fires at N" actually means. Both watermarks use the same comparator: a row that fires AT its
    // hard number would be lying if its soft number quietly needed exceeding.
    let compare = bound.compare;
    let crossed = compare.crossed_phrase();
    let (outcome, summary) = match (bound.soft, bound.hard) {
        (_, Some(hard)) if compare.crossed(observed, hard) => (
            OutcomeStatus::Failed,
            format!(
                "{}{unit_suffix} {crossed} the hard watermark {}",
                trim_number(observed),
                trim_number(hard)
            ),
        ),
        (Some(soft), _) if compare.crossed(observed, soft) => (
            OutcomeStatus::Passed,
            format!(
                "warn: {}{unit_suffix} {crossed} the soft watermark {}{}",
                trim_number(observed),
                trim_number(soft),
                bound
                    .hard
                    .map(|h| format!(" (hard {})", trim_number(h)))
                    .unwrap_or_default()
            ),
        ),
        _ => (
            OutcomeStatus::Passed,
            format!(
                "{}{unit_suffix} {} {}",
                trim_number(observed),
                compare.within_phrase(),
                describe_watermarks(bound)
            ),
        ),
    };

    BoundOutcome {
        bound: bound.id.clone(),
        measure: bound.measure.to_string(),
        subject: fold.subject.clone(),
        outcome,
        summary,
        observed: Some(observed),
        unit: fold.unit.clone(),
        watermarks,
        fold_cid: Some(fold.cid.clone()),
        source: bound.source.as_str().to_string(),
        binding: bound.binding.clone(),
        compare: bound.compare.as_str().to_string(),
        derive: None,
        reset_measure: None,
        reset_fold_cid: None,
        contributing_folds: None,
        recipe: recipe.clone(),
    }
}

/// Evaluate a bound that measures a standing condition of the TREE: how many files under the
/// declared surfaces are newer than a marker's recorded epoch.
///
/// The native replacement for `mempalace-currency.py --status`'s count. Three refusals, and each is
/// the difference between a measurement and a number:
///
/// - **No marker, no fallback → `skipped`.** Nobody knows when the index was built, so nothing was
///   measured. The kit's epoch-0 fallback makes every file newer, i.e. reports maximum staleness
///   from an absence of evidence; the config-mtime fallback is a real timestamp and is kept.
/// - **An unreadable/unparseable marker → `skipped`**, naming the marker. A marker whose contents
///   are not an epoch is a broken stamp, not a stamp reading zero.
/// - **A surface directory that does not exist is SKIPPED IN THE WALK and named** in the summary,
///   because the kit does the same (`if not os.path.isdir(base): continue`) and because a missing
///   surface contributes no files either way. A walk where NO declared surface exists is `skipped`.
fn evaluate_surface_walk(
    root: &Path,
    bound: &Bound,
    derive: Derive,
    recipe: &RecipeRef,
    watermarks: Watermarks,
) -> BoundOutcome {
    let subject = bound
        .subject
        .as_deref()
        .map(|s| normalize_subject(s).to_string())
        .unwrap_or_else(|| REPO_SUBJECT.to_string());
    let skipped = |summary: String| BoundOutcome {
        bound: bound.id.clone(),
        measure: bound.measure.to_string(),
        subject: subject.clone(),
        outcome: OutcomeStatus::Skipped,
        summary,
        observed: None,
        unit: None,
        watermarks,
        fold_cid: None,
        source: bound.source.as_str().to_string(),
        binding: bound.binding.clone(),
        compare: bound.compare.as_str().to_string(),
        derive: Some(derive.as_str().to_string()),
        reset_measure: None,
        reset_fold_cid: None,
        contributing_folds: None,
        recipe: recipe.clone(),
    };
    let Some(walk) = &bound.walk else {
        return skipped(format!(
            "bound {} declares `files-newer-than` with no surfaces to walk",
            bound.id
        ));
    };
    let Some((built_at, stamp)) = marker_epoch(root, walk) else {
        return skipped(format!(
            "no readable build stamp at {} (nor {}) — nothing recorded when the surface was last \
             indexed, so staleness is unmeasured rather than zero",
            walk.marker,
            walk.marker_fallback.as_deref().unwrap_or("(no fallback)")
        ));
    };

    let cutoff = built_at + walk.grace_seconds;
    let mut changed = 0usize;
    let mut walked = 0usize;
    let mut present = Vec::new();
    let mut absent = Vec::new();
    for surface in &walk.surfaces {
        let dir = root.join(surface);
        if !dir.is_dir() {
            absent.push(surface.clone());
            continue;
        }
        present.push(surface.clone());
        count_newer(&dir, &walk.suffix, cutoff, &mut changed, &mut walked);
    }
    if present.is_empty() {
        return skipped(format!(
            "none of the declared surfaces exist ({})",
            walk.surfaces.join(", ")
        ));
    }

    let mut summary = format!(
        "{changed} of {walked} {} file(s) newer than {} (+{}s grace)",
        walk.suffix, stamp, walk.grace_seconds as i64
    );
    if !absent.is_empty() {
        summary.push_str(&format!("; {} not present", absent.join(", ")));
    }
    let observed = changed as f64;
    let (outcome, summary) = judge(bound, observed, &summary, watermarks);
    BoundOutcome {
        bound: bound.id.clone(),
        measure: bound.measure.to_string(),
        subject,
        outcome,
        summary,
        observed: Some(observed),
        unit: bound.unit.clone(),
        watermarks,
        fold_cid: None,
        source: bound.source.as_str().to_string(),
        binding: bound.binding.clone(),
        compare: bound.compare.as_str().to_string(),
        derive: Some(derive.as_str().to_string()),
        reset_measure: None,
        reset_fold_cid: None,
        contributing_folds: Some(walked),
        recipe: recipe.clone(),
    }
}

/// Compare an observed magnitude against a bound's watermarks, in the vocabulary every other
/// outcome uses.
///
/// Factored out rather than copied so a tree-reading bound and a fold-reading bound can never
/// disagree about what "crossed the hard watermark" means. `basis` is the measurement's own
/// sentence; the watermark clause is appended to it.
fn judge(
    bound: &Bound,
    observed: f64,
    basis: &str,
    _watermarks: Watermarks,
) -> (OutcomeStatus, String) {
    let compare = bound.compare;
    let crossed = compare.crossed_phrase();
    match (bound.soft, bound.hard) {
        (_, Some(hard)) if compare.crossed(observed, hard) => (
            OutcomeStatus::Failed,
            format!(
                "{basis} — {crossed} the hard watermark {}",
                trim_number(hard)
            ),
        ),
        (Some(soft), _) if compare.crossed(observed, soft) => (
            OutcomeStatus::Passed,
            format!(
                "warn: {basis} — {crossed} the soft watermark {}{}",
                trim_number(soft),
                bound
                    .hard
                    .map(|h| format!(" (hard {})", trim_number(h)))
                    .unwrap_or_default()
            ),
        ),
        _ => (
            OutcomeStatus::Passed,
            format!(
                "{basis} — {} {}",
                compare.within_phrase(),
                describe_watermarks(bound)
            ),
        ),
    }
}

/// `(epoch, human stamp)` from the marker, else from its declared fallback's mtime, else `None`.
fn marker_epoch(root: &Path, walk: &SurfaceWalk) -> Option<(f64, String)> {
    let marker = root.join(&walk.marker);
    if let Ok(text) = std::fs::read_to_string(&marker) {
        if let Ok(epoch) = text.trim().parse::<f64>() {
            return Some((epoch, walk.marker.clone()));
        }
    }
    let fallback = walk.marker_fallback.as_ref()?;
    let meta = std::fs::metadata(root.join(fallback)).ok()?;
    let epoch = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs_f64();
    Some((epoch, format!("{fallback} (mtime; marker absent)")))
}

/// Walk `dir`, counting files whose suffix matches and whose mtime is past `cutoff`.
///
/// `/.git` is skipped, matching the kit. Symlinked directories are not followed: a symlink could
/// pull a tree outside the surface into the count, or count one twice.
fn count_newer(dir: &Path, suffix: &str, cutoff: f64, changed: &mut usize, walked: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if path.file_name().is_some_and(|n| n == ".git") || path.is_symlink() {
                continue;
            }
            count_newer(&path, suffix, cutoff, changed, walked);
            continue;
        }
        if !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(suffix))
        {
            continue;
        }
        *walked += 1;
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH) else {
            continue;
        };
        if since.as_secs_f64() > cutoff {
            *changed += 1;
        }
    }
}

/// Evaluate a bound whose value is an ACCUMULATION since its last reset.
///
/// The kit kept these counts in private JSON files that only their own script could read or drain
/// (`placement-drift.json` and its four siblings, summed by `cleanup-pressure.py::activity()`).
/// Here the events are already content-addressed observations and the reset is one more
/// observation, so the count is arithmetic over records anyone can re-derive — and `--reset`
/// becomes `epr flow note --kind observation --measure <id>-reset@1 --subject . --value 1` rather
/// than a script mutating a file nobody else can read.
///
/// **Zero is only reported when a reset witnesses it.** No reset and no contributing folds means
/// nobody has measured anything, which is `skipped` — the same discipline every other bound holds.
/// A reset with nothing since IS a witnessed zero: someone drained the accumulator, and that act is
/// on the record.
fn evaluate_derived(
    bound: &Bound,
    derive: Derive,
    folds: &[Fold],
    recipe: &RecipeRef,
    watermarks: Watermarks,
) -> BoundOutcome {
    // Ordering is (occurred_at, seq): notes share a HEAD-derived date, so append position is the
    // real discriminator within a session and the date orders across them.
    let reset = folds
        .iter()
        .filter(|fold| fold.measure == bound.reset)
        .max_by(|a, b| {
            a.occurred_at
                .cmp(&b.occurred_at)
                .then_with(|| a.seq.cmp(&b.seq))
        });
    let after = |fold: &Fold| match reset {
        Some(reset) => (&fold.occurred_at, fold.seq) > (&reset.occurred_at, reset.seq),
        None => true,
    };

    let contributing: Vec<&Fold> = folds
        .iter()
        .filter(|fold| fold.measure != bound.reset && admissible(bound, fold) && after(fold))
        .collect();

    let subject = bound
        .subject
        .as_deref()
        .map(|s| normalize_subject(s).to_string())
        .unwrap_or_else(|| REPO_SUBJECT.to_string());

    if contributing.is_empty() && reset.is_none() {
        return BoundOutcome {
            bound: bound.id.clone(),
            measure: bound.measure.to_string(),
            subject,
            outcome: OutcomeStatus::Skipped,
            summary: format!(
                "no fold for {} since no reset (derived {})",
                bound.measure,
                derive.as_str()
            ),
            observed: None,
            unit: None,
            watermarks,
            fold_cid: None,
            source: bound.source.as_str().to_string(),
            binding: bound.binding.clone(),
            compare: bound.compare.as_str().to_string(),
            derive: Some(derive.as_str().to_string()),
            reset_measure: Some(bound.reset.to_string()),
            reset_fold_cid: None,
            contributing_folds: Some(0),
            recipe: recipe.clone(),
        };
    }

    let observed = match derive {
        // EVENTS, including heals: a zero-valued fold is still a fold that landed. This is exactly
        // where the two derives part company — `count-since-reset` measures how much happened,
        // `distinct-subjects-since-reset` measures how much is still outstanding.
        Derive::CountSinceReset => contributing.len() as f64,
        // Per consumed measure, distinct subjects, summed — `cleanup-pressure.py::activity()`'s
        // arithmetic exactly (see `Derive::DistinctSubjectsSinceReset`) — and RETIRING any subject
        // whose latest fold is a zero.
        //
        // The zero is the producers' self-heal signal, not a measurement of nothing:
        // `placement-drift-signal.py:167-170` pops a re-opened doc out of the accumulator and folds
        // value 0 for it; the kit's collection shrinks accordingly. Counting the subject anyway
        // makes the bound MONOTONE — it can only ever rise — which turns the `cleanup:` trigger
        // into a false alarm that never clears. Reading the latest value per subject is what keeps
        // the fold plane's cardinality equal to the accumulator's.
        Derive::DistinctSubjectsSinceReset => {
            // (measure, subject) -> the latest fold for that pair, by the SAME (occurred_at, seq)
            // ordering the reset uses. Explicit rather than last-write-wins on iteration order, so
            // "latest" means one thing everywhere in this function.
            let mut latest: BTreeMap<(String, &str), &Fold> = BTreeMap::new();
            for fold in &contributing {
                let key = (fold.measure.to_string(), normalize_subject(&fold.subject));
                match latest.get(&key) {
                    Some(held)
                        if (&held.occurred_at, held.seq) >= (&fold.occurred_at, fold.seq) => {}
                    _ => {
                        latest.insert(key, fold);
                    }
                }
            }
            latest.values().filter(|fold| fold.value != 0.0).count() as f64
        }
        // Unreachable by construction: `evaluate` routes every tree-reading derive to
        // `evaluate_surface_walk` before this function is entered. Answering 0.0 rather than
        // panicking keeps a future third tree-derive from taking the whole headline down, and the
        // `reads_tree` guard is the thing that must stay true.
        Derive::FilesNewerThan => 0.0,
        // Unreachable by construction: `evaluate` routes every window-reading derive to
        // `evaluate_rate_over_window` before this function is entered — the `reads_window` guard's
        // sibling promise to `reads_tree`'s above.
        Derive::RateOverWindow => 0.0,
    };

    // The LENS's unit, not a contributing fold's: an accumulation over `documents` and `seeds` and
    // `edits` is denominated in the thing the bound measures, never in whichever source sorted first.
    let unit = bound
        .unit
        .clone()
        .or_else(|| contributing.iter().find_map(|f| f.unit.clone()));
    let unit_suffix = unit.as_ref().map(|u| format!(" {u}")).unwrap_or_default();
    let compare = bound.compare;
    let crossed = compare.crossed_phrase();
    let since = if reset.is_some() {
        "since the last reset"
    } else {
        "since the beginning"
    };
    let (outcome, summary) = match (bound.soft, bound.hard) {
        (_, Some(hard)) if compare.crossed(observed, hard) => (
            OutcomeStatus::Failed,
            format!(
                "{}{unit_suffix} {since} {crossed} the hard watermark {}",
                trim_number(observed),
                trim_number(hard)
            ),
        ),
        (Some(soft), _) if compare.crossed(observed, soft) => (
            OutcomeStatus::Passed,
            format!(
                "warn: {}{unit_suffix} {since} {crossed} the soft watermark {}{}",
                trim_number(observed),
                trim_number(soft),
                bound
                    .hard
                    .map(|h| format!(" (hard {})", trim_number(h)))
                    .unwrap_or_default()
            ),
        ),
        _ => (
            OutcomeStatus::Passed,
            format!(
                "{}{unit_suffix} {since} {} {}",
                trim_number(observed),
                compare.within_phrase(),
                describe_watermarks(bound)
            ),
        ),
    };

    BoundOutcome {
        bound: bound.id.clone(),
        measure: bound.measure.to_string(),
        subject,
        outcome,
        summary,
        observed: Some(observed),
        unit,
        watermarks,
        // The newest contributing fold — the most recent evidence behind the count. The count
        // itself is reproducible from `derive` + `resetFoldCid` + the sidecar.
        fold_cid: contributing.last().map(|f| f.cid.clone()),
        source: bound.source.as_str().to_string(),
        binding: bound.binding.clone(),
        compare: bound.compare.as_str().to_string(),
        derive: Some(derive.as_str().to_string()),
        reset_measure: Some(bound.reset.to_string()),
        reset_fold_cid: reset.map(|f| f.cid.clone()),
        contributing_folds: Some(contributing.len()),
        recipe: recipe.clone(),
    }
}

/// How far back a `rate-over-window` bound looks when the row declares no `window_days:` — a
/// quarter, matching the recall habit's own "the last quarter's journeys" framing.
const DEFAULT_WINDOW_DAYS: f64 = 91.0;

/// Evaluate a bound whose value is a RATE OVER A ROLLING TIME WINDOW — never an accumulation
/// since a reset, because a window has no reset: folds age out on their own as they fall past the
/// cutoff, so the population re-derives itself on every read.
///
/// The population is every admissible fold (drawn from every measure the row `consumes:`, exactly
/// like the reset-accumulation derives) whose `occurred_at` is within `window_days` of `now`. The
/// observed value is the fraction of that population whose value is positive — the same
/// "something happened" reading `count-since-reset` gives a single measure, generalized to a rate
/// over several.
///
/// **Fewer than 3 folds in the window is `skipped`, never a rate.** A fraction over one or two
/// journeys is not evidence of a trend; it is that trend's number wearing more confidence than the
/// population earns. This is the same three-valued discipline the module doc opens with, applied
/// to a population size rather than to a reset's presence.
fn evaluate_rate_over_window(
    bound: &Bound,
    derive: Derive,
    folds: &[Fold],
    recipe: &RecipeRef,
    watermarks: Watermarks,
    now: DateTime<Utc>,
) -> BoundOutcome {
    let window_days = bound.window_days.unwrap_or(DEFAULT_WINDOW_DAYS);
    let cutoff = now - chrono::Duration::seconds((window_days * 86_400.0) as i64);

    let subject = bound
        .subject
        .as_deref()
        .map(|s| normalize_subject(s).to_string())
        .unwrap_or_else(|| REPO_SUBJECT.to_string());

    // Ordering is (occurred_at, seq), the same discriminator every other derive uses: notes share
    // a HEAD-derived date, so append position is the real tie-break within one dated commit.
    let mut windowed: Vec<&Fold> = folds
        .iter()
        .filter(|fold| admissible(bound, fold) && fold_within_window(fold, cutoff))
        .collect();
    windowed.sort_by(|a, b| {
        a.occurred_at
            .cmp(&b.occurred_at)
            .then_with(|| a.seq.cmp(&b.seq))
    });

    if windowed.len() < 3 {
        return BoundOutcome {
            bound: bound.id.clone(),
            measure: bound.measure.to_string(),
            subject,
            outcome: OutcomeStatus::Skipped,
            summary: format!(
                "fewer than 3 journeys in window — {} fold(s) in the last {} days",
                windowed.len(),
                trim_number(window_days)
            ),
            observed: None,
            unit: None,
            watermarks,
            fold_cid: None,
            source: bound.source.as_str().to_string(),
            binding: bound.binding.clone(),
            compare: bound.compare.as_str().to_string(),
            derive: Some(derive.as_str().to_string()),
            reset_measure: None,
            reset_fold_cid: None,
            contributing_folds: Some(windowed.len()),
            recipe: recipe.clone(),
        };
    }

    let positive = windowed.iter().filter(|fold| fold.value > 0.0).count();
    let observed = positive as f64 / windowed.len() as f64;

    let basis = format!(
        "{positive} of {} journeys in the last {} days",
        windowed.len(),
        trim_number(window_days)
    );
    let (outcome, summary) = judge(bound, observed, &basis, watermarks);

    BoundOutcome {
        bound: bound.id.clone(),
        measure: bound.measure.to_string(),
        subject,
        outcome,
        summary,
        observed: Some(observed),
        unit: bound.unit.clone(),
        watermarks,
        // The newest fold in the window — the most recent evidence behind the rate. The rate
        // itself is reproducible from `derive` + `window_days` + the sidecar.
        fold_cid: windowed.last().map(|f| f.cid.clone()),
        source: bound.source.as_str().to_string(),
        binding: bound.binding.clone(),
        compare: bound.compare.as_str().to_string(),
        derive: Some(derive.as_str().to_string()),
        reset_measure: None,
        reset_fold_cid: None,
        contributing_folds: Some(windowed.len()),
        recipe: recipe.clone(),
    }
}

/// Whether a fold's `occurred_at` falls on or after `cutoff` — an unparseable timestamp is
/// excluded rather than guessed into the window, the same refusal-over-guess discipline
/// `admissible` and the reset derives hold.
fn fold_within_window(fold: &Fold, cutoff: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(&fold.occurred_at)
        .map(|dt| dt.with_timezone(&Utc) >= cutoff)
        .unwrap_or(false)
}

fn describe_watermarks(bound: &Bound) -> String {
    match (bound.soft, bound.hard) {
        (Some(soft), Some(hard)) => {
            format!("soft {} / hard {}", trim_number(soft), trim_number(hard))
        }
        (Some(soft), None) => format!("soft {}", trim_number(soft)),
        (None, Some(hard)) => format!("hard {}", trim_number(hard)),
        (None, None) => "no declared watermark".to_string(),
    }
}

/// Whether a fold is evidence of THIS bound: same measure pin, same subject when one is declared,
/// and carrying at least the declared env.
fn admissible(bound: &Bound, fold: &Fold) -> bool {
    if !bound.admits_measure(&fold.measure) {
        return false;
    }
    // Normalized on BOTH sides, so a row declaring `subject: .` (or `./`) and a fold recorded
    // against the repository root are one subject rather than two. A row with NO subject already
    // matches any fold, which is the other half of "repository-wide" — a bound that names no file
    // is a bound about the tree.
    if let Some(subject) = &bound.subject {
        if normalize_subject(subject) != normalize_subject(&fold.subject) {
            return false;
        }
    }
    bound
        .env
        .iter()
        .all(|(key, value)| fold.env.get(key) == Some(value))
}

/// Read every structured observation out of the sidecar, oldest first.
///
/// A repository with no sidecar has no folds — honest absence, not an error. That is what makes a
/// fresh clone's report a wall of `skipped` rather than a crash, which is the correct first answer:
/// nothing has been measured here yet.
pub(super) fn read_folds(root: &Path) -> FlowResult<Vec<Fold>> {
    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Ok(Vec::new());
    }
    let records = SidecarFlowStore::open(root)?.records()?;
    let mut folds = Vec::new();
    for (seq, (cid, record)) in records.into_iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if event.classified_as.first().map(String::as_str) != Some(OBSERVATION_TAG) {
            continue;
        }
        let Some(fold) = fold_from_slots(
            &cid.to_string(),
            &event.classified_as,
            &event.occurred_at,
            seq,
        ) else {
            continue;
        };
        folds.push(fold);
    }
    Ok(folds)
}

/// Project one event's slot vector back onto a [`Fold`], or `None` when it carries no measurement.
///
/// Slot-prefix keyed rather than positional: the prose arms of `note` emit a variable number of
/// leading slots (`switched-to:`, `verdict:`, acceptance slots, attribution slots), so reading by
/// position here would break the first time an unrelated slot was added upstream.
fn fold_from_slots(cid: &str, slots: &[String], occurred_at: &str, seq: usize) -> Option<Fold> {
    let subject = slots.get(1)?.clone();
    let mut measure = None;
    let mut value = None;
    let mut unit = None;
    let mut env = BTreeMap::new();
    for slot in slots.iter().skip(2) {
        if let Some(raw) = slot.strip_prefix(MEASURE_SLOT_PREFIX) {
            // A fold whose pin is unparseable is not a fold. Dropping it is correct: the report
            // would otherwise have to guess which measure it belongs to, and a guessed key is the
            // one thing a memoization key must never be.
            measure = MeasureRef::parse(raw, Path::new("")).ok();
        } else if let Some(raw) = slot.strip_prefix(VALUE_SLOT_PREFIX) {
            value = raw.trim().parse::<f64>().ok();
        } else if let Some(raw) = slot.strip_prefix(UNIT_SLOT_PREFIX) {
            unit = Some(raw.trim().to_string());
        } else if let Some(raw) = slot.strip_prefix(ENV_SLOT_PREFIX) {
            if let Some((key, val)) = raw.split_once('=') {
                env.insert(key.trim().to_string(), val.trim().to_string());
            }
        }
    }
    Some(Fold {
        cid: cid.to_string(),
        measure: measure?,
        subject,
        value: value?,
        unit,
        env,
        occurred_at: occurred_at.to_string(),
        seq,
    })
}

/// Render a number without a trailing `.0` on integral values, matching how the fold spells it.
fn trim_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

impl ReportPayload {
    /// The full human rendering — one section per recipe, one line per bound.
    pub fn render(&self) {
        for recipe in &self.recipes {
            println!(
                "flow report  {}{}  ({} bound{})",
                recipe.recipe.handle(),
                if recipe.primary { "" } else { "  [alt]" },
                recipe.outcomes.len(),
                if recipe.outcomes.len() == 1 { "" } else { "s" }
            );
            for outcome in &recipe.outcomes {
                println!(
                    "  {:<8} {} · {} — {}",
                    outcome.outcome.as_str(),
                    outcome.bound,
                    outcome.subject,
                    outcome.summary
                );
            }
            println!(
                "  totals: {} passed · {} failed · {} skipped",
                recipe.totals.passed, recipe.totals.failed, recipe.totals.skipped
            );
        }
    }

    /// The SessionStart headline: the primary recipe's five slot lines in their declared order, the
    /// recipe that produced them, then one `alt:` count line per additional recipe.
    ///
    /// Every slot prints, always. A slot with no declared bound says so rather than vanishing,
    /// because a line that disappears reads as "nothing to report" to a human and to a trigger
    /// alike — and "no bound declared" and "measured, fine" are the two states this replacement
    /// exists to keep apart.
    ///
    /// Alternate recipes get COUNTS, never lines. The headline is a fixed-shape surface a session
    /// reads every time; letting a second recipe double its length would make the shape depend on
    /// how many lenses happen to be declared, and a trigger cannot rely on a surface that grows.
    pub fn render_headline(&self) {
        for slot in HEADLINE_ORDER {
            println!("  {}", self.headline_line(slot));
        }
        if let Some(primary) = self.primary() {
            println!("  recipe: {}", primary.recipe.handle());
        }
        for line in self.alt_lines() {
            println!("  {line}");
        }
    }

    /// One `alt:` summary line per non-primary recipe.
    pub fn alt_lines(&self) -> Vec<String> {
        self.recipes
            .iter()
            .filter(|r| !r.primary)
            .map(|r| {
                format!(
                    "alt: {} — {} passed · {} failed · {} skipped",
                    r.recipe.handle(),
                    r.totals.passed,
                    r.totals.failed,
                    r.totals.skipped
                )
            })
            .collect()
    }

    /// The rendered line for one slot, exposed so a test can assert order and vocabulary without
    /// capturing stdout.
    pub fn headline_line(&self, slot: &str) -> String {
        let prefix = slot_prefix(slot);
        if slot == "scope" {
            if let Some(line) = &self.scope_line {
                return line.clone();
            }
        }
        let Some(outcome) = self.slot_outcome(slot) else {
            // A RETIRED slot is not an unmeasured one. Checking supersession before falling through
            // to "no bound declared" is the difference between "this was retired on 2026-09-11 and
            // here is why" and a reader opening a ticket about a missing measurement.
            if let Some(retired) = self.slot_retired(slot) {
                return format!("{prefix}: retired — {}", retired.reason);
            }
            return format!("{prefix}: skipped (no bound declared)");
        };
        match outcome.outcome {
            // The recall slot's absence has one meaning and deserves its own words: no journey has
            // been folded yet. "no fold for recall-unmetered-bytes@1" names the row; a session
            // reader needs to be told that nobody has walked the entry since the last reset.
            OutcomeStatus::Skipped
                if slot == "recall" && outcome.summary.starts_with("no fold") =>
            {
                format!("{prefix}: skipped — no journey fold")
            }
            OutcomeStatus::Skipped => format!("{prefix}: skipped ({})", outcome.summary),
            OutcomeStatus::Failed => format!("{prefix}: ⚠ failed — {}", outcome.summary),
            OutcomeStatus::Passed if outcome.summary.starts_with("warn: ") => {
                format!(
                    "{prefix}: ⚠ {}",
                    outcome.summary.trim_start_matches("warn: ")
                )
            }
            OutcomeStatus::Passed => format!("{prefix}: {} ✅", outcome.summary),
        }
    }

    /// The first PRIMARY-recipe outcome whose bound belongs to `slot`.
    fn slot_outcome(&self, slot: &str) -> Option<&BoundOutcome> {
        self.outcomes()
            .iter()
            .find(|outcome| slot_of_outcome(outcome) == Some(slot))
    }

    /// The first PRIMARY-recipe RETIRED bound belonging to `slot`.
    fn slot_retired(&self, slot: &str) -> Option<&RetiredBound> {
        self.recipes
            .iter()
            .find(|r| r.primary)
            .into_iter()
            .flat_map(|r| r.retired.iter())
            .find(|retired| retired.headline.as_deref() == Some(slot))
    }

    /// Every retired bound across the primary recipe — the `--json` reader's list.
    pub fn retired(&self) -> &[RetiredBound] {
        self.recipes
            .iter()
            .find(|r| r.primary)
            .map(|r| r.retired.as_slice())
            .unwrap_or_default()
    }
}

/// The headline slot a BOUND belongs to — the row's explicit `headline:` when it declares one,
/// else the same id-derived default [`slot_of_outcome`] applies to an outcome.
///
/// Needed because a retired bound never becomes an outcome, and its slot still has to render.
fn headline_slot_of(bound: &Bound) -> Option<String> {
    if let Some(slot) = &bound.headline {
        return Some(slot.trim().to_string());
    }
    slot_of_measure(&bound.measure.id).map(ToString::to_string)
}

/// The slot a bare measure id implies.
fn slot_of_measure(id: &str) -> Option<&'static str> {
    if id.starts_with("recall-unmetered-bytes") {
        Some("recall")
    } else if id.starts_with("memkit") {
        Some("memkit")
    } else if id.starts_with("mempalace") {
        Some("mempalace")
    } else if id.starts_with("cleanup") {
        Some("cleanup")
    } else if id.starts_with("scope") {
        Some("scope")
    } else if id.starts_with("memory-index") {
        Some("budget")
    } else {
        None
    }
}

/// Recover a rendered outcome's slot from its measure id.
///
/// The outcome payload deliberately carries no slot field: the headline is a *projection* of the
/// outcomes, not a property of them, and storing the slot would make the same fact true in two
/// places that could disagree.
fn slot_of_outcome(outcome: &BoundOutcome) -> Option<&'static str> {
    slot_of_measure(outcome.measure.split('@').next().unwrap_or_default())
}

/// Fold the scope slot's magnitude onto its declared bound. Re-exported from [`super::scope`] so
/// the report's callers reach one module for the whole headline.
pub fn fold_scope(root: &Path, pending_moves: usize) -> Option<String> {
    super::scope::fold_pending_moves(root, pending_moves)
}

/// Resolve a bound's headline slot at declaration time; re-exported for the CLI's `--bound`
/// diagnostics and asserted by the headline tests.
pub fn slot_for(bound: &Bound) -> Option<&'static str> {
    headline_slot(bound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_headline_order_is_the_gospel_declared_order() {
        assert_eq!(
            HEADLINE_ORDER,
            ["recall", "mempalace", "cleanup", "scope", "budget"]
        );
        assert_eq!(slot_prefix("recall"), "recall");
        assert_eq!(slot_prefix("cleanup"), "cleanup");
        assert_eq!(slot_prefix("scope"), "scope");
        assert_eq!(slot_prefix("budget"), "memory-budget");
    }

    #[test]
    fn a_measure_id_derives_its_slot_without_an_explicit_declaration() {
        let derive = |id: &str| {
            derived_slot(&MeasureRef {
                id: id.into(),
                version: 1,
            })
        };
        assert_eq!(derive("recall-unmetered-bytes"), Some("recall"));
        // The other three journey middot are measured, never headlined.
        assert_eq!(derive("recall-metered-bytes"), None);
        assert_eq!(derive("recall-screens-to-shape"), None);
        assert_eq!(derive("memkit-report-tier-mb"), Some("memkit"));
        assert_eq!(derive("mempalace-currency-days"), Some("mempalace"));
        assert_eq!(derive("cleanup-pressure"), Some("cleanup"));
        assert_eq!(derive("scope-drift"), Some("scope"));
        assert_eq!(derive("memory-index-bytes"), Some("budget"));
        assert_eq!(derive("decompose-threshold"), None);
    }

    #[test]
    fn trimming_matches_the_folds_own_spelling() {
        assert_eq!(trim_number(8.0), "8");
        assert_eq!(trim_number(24000.0), "24000");
        assert_eq!(trim_number(0.5), "0.5");
    }
}
