//! `epr flow report placement --coverage` and `--stasis` — the two placement readings the stasis
//! loop and the context ratchet consume.
//!
//! Station six round (b) of the memory-kit replacement. `--ledger`, `--focus` and `--headline`
//! moved natively at station two; these two did not, which is why
//! `.claude/workflows/memory-stasis-loop.js`'s `AUDIT` const still ran `placement-audit.py`. Porting
//! them is what lets that const point at `epr` and the script go.
//!
//! ## Two readings, two consumers, one contract each
//!
//! **`--coverage`** answers *how much of the active surface has been reviewed*. The loop reads
//! `uncaptured` from it (`memory-stasis-loop.js:76`) and dispatches the `capture` goal until it is
//! zero. Its JSON keys are the kit's: `active`, `captured`, `needs_agent`, `undecomposed`,
//! `uncaptured`, `uncaptured_docs[]`.
//!
//! One reading DID change with the substrate, and pretending otherwise would be the lie. The kit
//! asked "does `gap-items/<slug>.json` exist with items?" — capture was a *stored artifact*. Station
//! two made extraction native: `epr flow project` reads `- [ ]` stations out of the document itself,
//! so a plan with stations is captured the moment it is written. The native reading is therefore
//! **captured = the document yields stations**, with the adopted store consulted only for the
//! `needs-agent` case it is still the sole witness of (an agent-decomposed doc whose own bytes carry
//! no checkbox). `uncaptured = undecomposed + needs_agent` is preserved exactly, because that is the
//! number the loop drains.
//!
//! **`--stasis`** answers *how close the whole surface is to equilibrium*. Two consumers, both
//! reading the same JSON: the loop takes `stasis_score` and `at_stasis`
//! (`memory-stasis-loop.js:82`), and `context-ratchet.py` takes `dimensions` and `score` and gates
//! on "no dimension may fall below baseline − epsilon". Its keys are the kit's: `benchmark`,
//! `margin`, `threshold`, `score`, `hard_ok`, `at_stasis`, `dimensions{}`, `unmeasured[]`.
//!
//! ## The tuning surface, and where it lives after the deletion
//!
//! The weights, budgets and margin live in `context-coverage.yaml` — which sits inside the report
//! tier round (b) deletes. So the reader walks a ladder and says which rung it used:
//! `.epr-meta/elohim/lenses/context-coverage.yaml` (the relocated home), then
//! `.claude/memory-kit/context-coverage.yaml` (today's), then [`DEFAULTS`] — constants transcribed
//! from the manifest as it stands, with `tuning: "built-in defaults"` on the payload so a reader is
//! never left assuming a file was found. A tuning surface that vanishes silently is how a score
//! changes without anybody deciding to change it.
//!
//! ## The ratchet baseline is a fold, not a private JSON file
//!
//! `context-ratchet.py` keeps its baseline in `.claude/memory-kit/context-coverage-baseline.json`,
//! which only that script reads or writes — the same unaddressable-history shape the whole
//! replacement exists to end. Here each dimension's ratio is an observation keyed
//! `subject × measure@version × env`: subject is the dimension name, measure is
//! `context-coverage-dimension@1`. `--fold` appends them; the reading compares against the latest
//! fold per dimension and reports regressions. A dimension with no fold is **not** a regression and
//! **not** a pass — it is `unbaselined`, named as such, because "nobody has measured this before"
//! and "this has not fallen" are different claims.
//!
//! ## What is UNMEASURED stays unmeasured
//!
//! The kit carries an explicit `unmeasured` list and excludes those dimensions from the score
//! entirely, on the ground that full stasis cannot be claimed while a dimension is unwired. That
//! discipline is transcribed verbatim, including the list, because it is the same
//! `skipped`-is-not-zero rule the bounds report holds one layer up.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{parse_frontmatter, FlowError, FlowResult};

/// The measure a ratchet baseline fold pins. One row for all nine dimensions.
///
/// The fold key is `subject × measure@version × env`. The SUBJECT is the repository — a coverage
/// ratio is a property of the whole corpus — and the dimension is the ENV the measurement was taken
/// under, which is exactly what env keys are for. The first cut used the dimension name as the
/// subject and was refused by the note plane, correctly: `capture` is not a resource, and a subject
/// that names no addressable thing is a fold nobody can join back to what it measured.
pub const DIMENSION_MEASURE: &str = "context-coverage-dimension@1";

/// The env key carrying which dimension a baseline fold measured.
pub const DIMENSION_ENV_KEY: &str = "dimension";

/// Where the tuning manifest is looked for, in order. The first is its home after the report tier
/// is removed; the second is where it sits today.
const TUNING_PATHS: [&str; 2] = [
    ".epr-meta/elohim/lenses/context-coverage.yaml",
    ".claude/memory-kit/context-coverage.yaml",
];

/// The adopted gap-item store, consulted only for the `needs-agent` case.
const GAP_STORE: [&str; 2] = [".eprfs/status/gap-items", ".claude/memory-kit/gap-items"];

/// Dimensions in the order the kit renders them. The ORDER is a contract: `context-ratchet.py`
/// keys by name, but a human reading two runs side by side compares rows by position.
pub const DIMENSION_KEYS: [&str; 9] = [
    "capture",
    "status",
    "well_formed",
    "memory_linked",
    "claude_md_rightsized",
    "history_bidirectional",
    "traceability",
    "memory_md_budget",
    "epr_meta_coverage",
];

/// The human label for each dimension, as the kit prints it.
const DIMENSION_LABELS: [&str; 9] = [
    "capture: specs/plans decomposed",
    "status: docs carry a state",
    "well-formed: docs linked (non-orphan)",
    "memory: entries cite a system",
    "CLAUDE.md right-sized (<= budget)",
    "history: bidirectional canonical link",
    "traceability: docs carry explained trace links",
    "MEMORY.md within byte budget",
    "epr-meta: substantial dirs owned by an .epr-meta",
];

/// The kit's `unmeasured` list, verbatim (`placement-audit.py`). These are dimensions nobody has
/// wired; they are excluded from the score and named, so "checked ≠ done" stays visible.
const UNMEASURED: [&str; 3] = [
    "code-FILE-level traceability (.ts/.rs → story/scenario) — doc-level traces measured above; per-code-file convention not yet wired",
    "gap-items → testing-infrastructure tagging (NOT wired)",
    "CLAUDE.md recursive coverage: every dir needing a gate has one (run claude-md-audit.py)",
];

/// Directories the `.epr-meta` census never descends into (`_lib/epr_meta.py::DEFAULT_SKIP_DIRS`).
const SKIP_DIRS: [&str; 16] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".angular",
    ".pnpm-store",
    "__pycache__",
    ".cargo",
    ".venv",
    "venv",
    ".next",
    "coverage",
    "worktrees",
    ".worktrees",
    ".pytest_cache",
];

/// Extensions that make a directory "code, not pure data" (`DEFAULT_COMPLEXITY_EXTS`).
const COMPLEXITY_EXTS: [&str; 19] = [
    "ts", "tsx", "js", "jsx", "mjs", "py", "rs", "go", "java", "rb", "html", "scss", "css", "vue",
    "svelte", "feature", "sql", "graphql", "proto",
];

/// The tuning values, transcribed from `context-coverage.yaml` as it stands. Used only when no
/// manifest is found, and the payload says so when they are.
pub struct Defaults;
impl Defaults {
    const BENCHMARK: f64 = 1.0;
    const MARGIN: f64 = 0.15;
    const MEMORY_MD_BUDGET: f64 = 24_000.0;
    const CLAUDE_MD_BUDGET: f64 = 12_000.0;
    const RATCHET_EPSILON: f64 = 0.005;
    const MIN_FILES: usize = 15;
    const MIN_SUBDIRS: usize = 4;
    const MIN_EXTS: usize = 1;
    const EXCLUDE: [&'static str; 4] = [
        "**/generated",
        "**/generated/**",
        "genesis/docs/_state/**",
        "genesis/graphos/design-assets/**",
    ];
}

/// The resolved tuning surface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tuning {
    /// Which rung of the ladder supplied these values — a path, or `built-in defaults`.
    pub source: String,
    pub benchmark: f64,
    pub margin: f64,
    pub memory_md_budget: f64,
    pub claude_md_budget: f64,
    pub ratchet_epsilon: f64,
    pub weights: BTreeMap<String, f64>,
    pub min_files: usize,
    pub min_subdirs: usize,
    pub min_exts: usize,
    pub exclude: Vec<String>,
}

impl Tuning {
    fn built_in() -> Self {
        Self {
            source: "built-in defaults".into(),
            benchmark: Defaults::BENCHMARK,
            margin: Defaults::MARGIN,
            memory_md_budget: Defaults::MEMORY_MD_BUDGET,
            claude_md_budget: Defaults::CLAUDE_MD_BUDGET,
            ratchet_epsilon: Defaults::RATCHET_EPSILON,
            weights: DIMENSION_KEYS
                .iter()
                .map(|k| ((*k).to_string(), 1.0))
                .collect(),
            min_files: Defaults::MIN_FILES,
            min_subdirs: Defaults::MIN_SUBDIRS,
            min_exts: Defaults::MIN_EXTS,
            exclude: Defaults::EXCLUDE.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Read the tuning manifest from the first rung that exists, else the built-in constants.
    pub fn resolve(root: &Path) -> Self {
        for rel in TUNING_PATHS {
            let path = root.join(rel);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&text) else {
                eprintln!("stasis: {rel} is unreadable YAML — using built-in defaults");
                continue;
            };
            let mut tuning = Self::built_in();
            tuning.source = rel.to_string();
            let target =
                |key: &str| -> Option<f64> { doc.get("targets")?.get(key)?.get("value")?.as_f64() };
            if let Some(v) = target("benchmark") {
                tuning.benchmark = v;
            }
            if let Some(v) = target("margin") {
                tuning.margin = v;
            }
            if let Some(v) = target("memory_md_budget") {
                tuning.memory_md_budget = v;
            }
            if let Some(v) = target("claude_md_budget") {
                tuning.claude_md_budget = v;
            }
            if let Some(v) = target("ratchet_epsilon") {
                tuning.ratchet_epsilon = v;
            }
            if let Some(dims) = doc.get("dimensions").and_then(|d| d.as_mapping()) {
                for key in DIMENSION_KEYS {
                    if let Some(w) = dims
                        .get(serde_yaml::Value::from(key))
                        .and_then(|d| d.get("weight"))
                        .and_then(serde_yaml::Value::as_f64)
                    {
                        tuning.weights.insert(key.to_string(), w);
                    }
                }
            }
            if let Some(gov) = doc.get("epr_meta_governance") {
                if let Some(v) = gov.get("min_files").and_then(serde_yaml::Value::as_u64) {
                    tuning.min_files = v as usize;
                }
                if let Some(v) = gov.get("min_subdirs").and_then(serde_yaml::Value::as_u64) {
                    tuning.min_subdirs = v as usize;
                }
                if let Some(v) = gov.get("min_exts").and_then(serde_yaml::Value::as_u64) {
                    tuning.min_exts = v as usize;
                }
                if let Some(seq) = gov.get("exclude").and_then(serde_yaml::Value::as_sequence) {
                    tuning.exclude = seq
                        .iter()
                        .filter_map(serde_yaml::Value::as_str)
                        .map(ToString::to_string)
                        .collect();
                }
            }
            return tuning;
        }
        Self::built_in()
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// --coverage
// ───────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UncapturedDoc {
    pub path: String,
    pub why: String,
}

/// The kit's `--coverage --json` keys, spelled the kit's way (snake_case) because
/// `memory-stasis-loop.js` reads them by name.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    pub active: usize,
    pub captured: usize,
    pub needs_agent: usize,
    pub undecomposed: usize,
    pub uncaptured: usize,
    pub uncaptured_docs: Vec<UncapturedDoc>,
    /// How capture was decided, so the reading's change from the kit's is on the payload rather
    /// than only in this module's doc comment.
    pub method: String,
}

impl CoverageReport {
    pub fn render(&self) {
        println!("DECOMPOSE-COVERAGE — the un-reviewed / un-captured workload");
        println!("{}", "=".repeat(72));
        println!("  active specs+plans          : {}", self.active);
        println!("  ✅ decomposed (captured)     : {}", self.captured);
        println!("  ▶ un-decomposed (never reviewed): {}", self.undecomposed);
        println!(
            "  ▶ needs-agent (prose, no structure): {}",
            self.needs_agent
        );
        println!("  ----");
        println!(
            "  UN-CAPTURED WORKLOAD        : {}   (the loop dispatches until this hits 0 → effort ∝ real backlog)",
            self.uncaptured
        );
        println!("\n  the loop's queue (next un-captured):");
        for doc in self.uncaptured_docs.iter().take(14) {
            println!("    [{:<12}] {}", doc.why, doc.path);
        }
        if self.uncaptured_docs.len() > 14 {
            println!("    … +{} more", self.uncaptured_docs.len() - 14);
        }
        println!("\n  method: {}", self.method);
    }
}

/// Derive the decompose-coverage reading over `root`.
pub fn coverage(root: &Path) -> FlowResult<CoverageReport> {
    let active = super::placement::active_docs(root);
    let store = gap_store(root);
    let mut captured = 0;
    let mut needs_agent = 0;
    let mut undecomposed = 0;
    let mut queue = Vec::new();
    for doc in &active {
        if doc.stations > 0 {
            captured += 1;
            continue;
        }
        // No stations in the document's own bytes. The store is the only witness that distinguishes
        // "an agent decomposed this and found no structure" from "nobody has looked".
        match store.get(&slug_of(&doc.path)) {
            Some(true) => {
                captured += 1;
            }
            Some(false) => {
                needs_agent += 1;
                queue.push(UncapturedDoc {
                    path: doc.path.clone(),
                    why: "needs-agent".into(),
                });
            }
            None => {
                undecomposed += 1;
                queue.push(UncapturedDoc {
                    path: doc.path.clone(),
                    why: "undecomposed".into(),
                });
            }
        }
    }
    Ok(CoverageReport {
        active: active.len(),
        captured,
        needs_agent,
        undecomposed,
        uncaptured: undecomposed + needs_agent,
        uncaptured_docs: queue,
        method: format!(
            "captured = the document yields `- [ ]` stations (epr flow project); the gap store at \
             {} is consulted only to tell needs-agent from undecomposed",
            GAP_STORE[0]
        ),
    })
}

/// `slug -> has items`, from whichever adopted store exists.
fn gap_store(root: &Path) -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    for rel in GAP_STORE {
        let dir = root.join(rel);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let has_items = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .map(|v| {
                    v.get("method").and_then(|m| m.as_str()) != Some("none")
                        && v.get("items")
                            .and_then(|i| i.as_array())
                            .is_some_and(|a| !a.is_empty())
                })
                .unwrap_or(false);
            out.entry(stem.to_string()).or_insert(has_items);
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// `<parent-dir>__<stem>` — the gap-item filename the kit mints and station two adopted.
fn slug_of(path: &str) -> String {
    let p = Path::new(path);
    let parent = p
        .parent()
        .and_then(|d| d.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    format!("{parent}__{stem}")
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// --stasis
// ───────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HardGate {
    pub name: String,
    pub ok: bool,
    /// The count that decided it — a boolean with no number behind it is unauditable.
    pub observed: usize,
}

/// One dimension's ratio against its ratchet baseline.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ratchet {
    pub dimension: String,
    pub current: f64,
    /// `None` when no fold has ever been taken for this dimension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f64>,
    /// `regressed` | `held` | `improved` | `unbaselined` — never a bare boolean, because
    /// "no baseline" is not "did not regress".
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StasisReport {
    pub benchmark: f64,
    pub margin: f64,
    pub threshold: f64,
    pub score: f64,
    pub hard_ok: bool,
    pub at_stasis: bool,
    pub dimensions: BTreeMap<String, f64>,
    pub unmeasured: Vec<String>,
    // ── native additions, all camelCase so the kit's snake_case contract above is untouched ──
    #[serde(rename = "hardGates")]
    pub hard_gates: Vec<HardGate>,
    #[serde(rename = "dimensionLabels")]
    pub dimension_labels: BTreeMap<String, String>,
    pub tuning: Tuning,
    /// The ratchet reading — the native replacement for `context-coverage-baseline.json`.
    pub ratchet: Vec<Ratchet>,
    /// True when some dimension fell below its baseline by more than the epsilon.
    #[serde(rename = "ratchetRegressed")]
    pub ratchet_regressed: bool,
}

impl StasisReport {
    pub fn render(&self) {
        println!(
            "STASIS — composite score vs benchmark {}, ±{}% band (at stasis ≥ {:.2})",
            trim(self.benchmark),
            (self.margin * 100.0) as i64,
            self.threshold
        );
        println!("{}", "=".repeat(72));
        for key in DIMENSION_KEYS {
            let Some(ratio) = self.dimensions.get(key) else {
                continue;
            };
            let label = self.dimension_labels.get(key).cloned().unwrap_or_default();
            println!(
                "  {} {label:<44} {:5.1}%",
                if *ratio >= self.threshold {
                    "✅"
                } else {
                    "❌"
                },
                ratio * 100.0
            );
        }
        for gate in &self.hard_gates {
            println!(
                "  {} {:<44} {}",
                if gate.ok { "✅" } else { "❌" },
                gate.name,
                if gate.ok {
                    "pass".to_string()
                } else {
                    format!("FAIL ({}) — gates the score", gate.observed)
                }
            );
        }
        println!("  {}", "-".repeat(44));
        println!(
            "  STASIS SCORE (measured): {:.3} / {:.3}   →  {}",
            self.score,
            self.benchmark,
            if self.at_stasis {
                "✅ AT STASIS".to_string()
            } else {
                format!("❌ NOT at stasis (need ≥ {:.2})", self.threshold)
            }
        );
        println!(
            "\n  RATCHET — no dimension may fall below its baseline (ε {}):",
            trim(self.tuning.ratchet_epsilon)
        );
        for r in &self.ratchet {
            match r.baseline {
                Some(base) => println!(
                    "    {:<24} {:5.1}%  (baseline {:5.1}%)  {}",
                    r.dimension,
                    r.current * 100.0,
                    base * 100.0,
                    r.state
                ),
                None => println!(
                    "    {:<24} {:5.1}%  {}",
                    r.dimension,
                    r.current * 100.0,
                    r.state
                ),
            }
        }
        println!(
            "\n  ⏳ UNMEASURED — full stasis CANNOT be claimed until these are wired (checked ≠ done):"
        );
        for u in &self.unmeasured {
            println!("    ? {u}");
        }
        println!("\n  tuning: {}", self.tuning.source);
    }
}

fn trim(v: f64) -> String {
    if (v - v.round()).abs() < f64::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn ratio(num: usize, den: usize) -> f64 {
    if den == 0 {
        1.0
    } else {
        num as f64 / den as f64
    }
}

/// Derive the stasis reading over `root`.
pub fn stasis(root: &Path, baselines: &BTreeMap<String, f64>) -> FlowResult<StasisReport> {
    let tuning = Tuning::resolve(root);
    let cov = coverage(root)?;
    let active = super::placement::active_docs(root);
    let surface = super::placement::surface_docs(root);
    let memory = super::placement::memory_entries(root);

    let stated = active.iter().filter(|d| d.bucket != "NONE").count();
    let linked = active.iter().filter(|d| d.linked).count();
    let mem_linked = memory.iter().filter(|m| m.linked).count();
    let traced = active.iter().filter(|d| d.explained_trace).count();

    let history: Vec<_> = surface.iter().filter(|d| d.home == "HISTORY").collect();
    let history_ok = history.iter().filter(|d| d.canonical_link).count();

    let claude_mds = claude_md_files(root);
    let claude_ok = claude_mds
        .iter()
        .filter(|p| file_len(p) <= tuning.claude_md_budget)
        .count();

    let memory_md = file_len(&root.join(".claude/memory/MEMORY.md"));
    let memory_md_ratio = if memory_md <= tuning.memory_md_budget * (1.0 + tuning.margin) {
        1.0
    } else {
        tuning.memory_md_budget / memory_md.max(1.0)
    };

    let census = epr_meta_coverage(root, &tuning);

    let mut dimensions = BTreeMap::new();
    dimensions.insert("capture".into(), ratio(cov.captured, cov.active));
    dimensions.insert("status".into(), ratio(stated, active.len()));
    dimensions.insert("well_formed".into(), ratio(linked, active.len()));
    dimensions.insert("memory_linked".into(), ratio(mem_linked, memory.len()));
    dimensions.insert(
        "claude_md_rightsized".into(),
        ratio(claude_ok, claude_mds.len()),
    );
    dimensions.insert(
        "history_bidirectional".into(),
        if history.is_empty() {
            1.0
        } else {
            ratio(history_ok, history.len())
        },
    );
    dimensions.insert("traceability".into(), ratio(traced, active.len()));
    dimensions.insert("memory_md_budget".into(), memory_md_ratio);
    dimensions.insert("epr_meta_coverage".into(), census);

    let weight_sum: f64 = DIMENSION_KEYS
        .iter()
        .map(|k| tuning.weights.get(*k).copied().unwrap_or(1.0))
        .sum();
    let weight_sum = if weight_sum == 0.0 { 1.0 } else { weight_sum };
    let score = DIMENSION_KEYS
        .iter()
        .map(|k| {
            dimensions.get(*k).copied().unwrap_or(0.0)
                * tuning.weights.get(*k).copied().unwrap_or(1.0)
        })
        .sum::<f64>()
        / weight_sum;

    let dumps = unverified_retired(root);
    let pressure_docs = pressure_dir_docs(root);
    let hard_gates = vec![
        HardGate {
            name: "_retired dumps == 0".into(),
            ok: dumps == 0,
            observed: dumps,
        },
        HardGate {
            name: "pressure dirs empty".into(),
            ok: pressure_docs == 0,
            observed: pressure_docs,
        },
    ];
    let hard_ok = hard_gates.iter().all(|g| g.ok);
    let threshold = tuning.benchmark - tuning.margin;
    let at_stasis = score >= threshold && hard_ok;

    let mut ratchet = Vec::new();
    let mut ratchet_regressed = false;
    for key in DIMENSION_KEYS {
        // Rounded to the same three places the fold carries, so a ratchet compares like with like.
        // An unrounded current against a rounded baseline differs by up to 0.0005 — well inside the
        // 0.005 epsilon, but it would make two identical runs print different numbers.
        let current = (dimensions.get(key).copied().unwrap_or(0.0) * 1000.0).round() / 1000.0;
        let state = match baselines.get(key) {
            None => "unbaselined".to_string(),
            Some(base) if current < base - tuning.ratchet_epsilon => {
                ratchet_regressed = true;
                "regressed".to_string()
            }
            Some(base) if current > base + tuning.ratchet_epsilon => "improved".to_string(),
            Some(_) => "held".to_string(),
        };
        ratchet.push(Ratchet {
            dimension: key.to_string(),
            current,
            baseline: baselines.get(key).copied(),
            state,
        });
    }

    Ok(StasisReport {
        benchmark: tuning.benchmark,
        margin: tuning.margin,
        threshold: (threshold * 1000.0).round() / 1000.0,
        score: (score * 1000.0).round() / 1000.0,
        hard_ok,
        at_stasis,
        dimensions: dimensions
            .into_iter()
            .map(|(k, v)| (k, (v * 1000.0).round() / 1000.0))
            .collect(),
        unmeasured: UNMEASURED.iter().map(|s| (*s).to_string()).collect(),
        hard_gates,
        dimension_labels: DIMENSION_KEYS
            .iter()
            .zip(DIMENSION_LABELS.iter())
            .map(|(k, l)| ((*k).to_string(), (*l).to_string()))
            .collect(),
        tuning,
        ratchet,
        ratchet_regressed,
    })
}

fn file_len(path: &Path) -> f64 {
    std::fs::metadata(path)
        .map(|m| m.len() as f64)
        .unwrap_or(0.0)
}

/// Every `CLAUDE.md` the kit globs: the root one plus those under `genesis/` and `.claude/`,
/// excluding worktrees and vendored trees.
fn claude_md_files(root: &Path) -> Vec<PathBuf> {
    let mut out = BTreeSet::new();
    let top = root.join("CLAUDE.md");
    if top.is_file() {
        out.insert(top);
    }
    for base in ["genesis", ".claude"] {
        collect_named(&root.join(base), "CLAUDE.md", &mut out);
    }
    out.into_iter().collect()
}

fn collect_named(dir: &Path, name: &str, out: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_DIRS.contains(&base) || path.is_symlink() {
                continue;
            }
            collect_named(&path, name, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            out.insert(path);
        }
    }
}

/// `_retired` docs carrying no verification evidence — the kit's anti-dump hard gate.
fn unverified_retired(root: &Path) -> usize {
    let dir = root.join("genesis/docs/_retired");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter(|p| {
            let Ok(text) = std::fs::read_to_string(p) else {
                return false;
            };
            let fm = parse_frontmatter(&text);
            !["verified_by", "verification", "landed_commit"]
                .iter()
                .any(|k| fm.get(k).is_some_and(|v| !v.trim().is_empty()))
        })
        .count()
}

/// Documents parked in a `genesis/docs/_state/<state>/` pressure directory.
fn pressure_dir_docs(root: &Path) -> usize {
    let state = root.join("genesis/docs/_state");
    let Ok(entries) = std::fs::read_dir(&state) else {
        return 0;
    };
    let mut n = 0;
    for entry in entries.flatten() {
        let sub = entry.path();
        if !sub.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&sub) else {
            continue;
        };
        n += files
            .flatten()
            .map(|f| f.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .filter(|p| {
                !matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("CLAUDE.md") | Some("README.md")
                )
            })
            .count();
    }
    n
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The `.epr-meta` subtree census
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `covered / (covered + gaps)`, where a region is COVERED when a VALID `covers: subtree` manifest
/// claims it and a GAP when it is structurally substantial and no ancestor claim reached it.
///
/// A transcription of `_lib/epr_meta.py::subtree_coverage`. Both claim-points and gap-roots
/// terminate descent, so one manifest at the right altitude resolves a whole subtree and nested
/// substantial dirs are never double-counted. The repo root is never itself a gap. A manifest that
/// claims the subtree but fails to parse is NOT credited — a broken manifest the resolver would
/// reject must not inflate the ratio and hide a real gap, which is the one rule in the original
/// whose absence would make the number flattering rather than true. Returns 1.0 when there is
/// nothing governable.
pub fn epr_meta_coverage(root: &Path, tuning: &Tuning) -> f64 {
    let submodules = submodule_paths(root);
    let mut covered = 0usize;
    let mut gaps = 0usize;
    visit_census(
        root,
        root,
        tuning,
        &submodules,
        true,
        &mut covered,
        &mut gaps,
    );
    if covered + gaps == 0 {
        1.0
    } else {
        covered as f64 / (covered + gaps) as f64
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_census(
    root: &Path,
    dir: &Path,
    tuning: &Tuning,
    submodules: &BTreeSet<String>,
    is_root: bool,
    covered: &mut usize,
    gaps: &mut usize,
) {
    if claims_subtree(dir) {
        *covered += 1;
        return; // fully responsible — terminate
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files = 0usize;
    let mut exts: BTreeSet<String> = BTreeSet::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // Never follow a symlinked dir: it would double-count a target reached two ways, and could
        // pull a tree outside the root into the census under an in-root relative path.
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            let rel = rel_of(root, &path);
            if submodules.contains(&rel) || excluded(&rel, &tuning.exclude) {
                continue;
            }
            subdirs.push(path);
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some(".epr-meta") {
            continue;
        }
        files += 1;
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if COMPLEXITY_EXTS.contains(&ext) {
                exts.insert(ext.to_string());
            }
        }
    }
    if !is_root
        && (files >= tuning.min_files || subdirs.len() >= tuning.min_subdirs)
        && exts.len() >= tuning.min_exts
    {
        *gaps += 1;
        return; // gap-root terminates descent too
    }
    subdirs.sort();
    for sub in subdirs {
        visit_census(root, &sub, tuning, submodules, false, covered, gaps);
    }
}

/// Does this directory carry a VALID `covers: subtree` manifest?
///
/// The claim is read from the manifest's own `covers:` key and the VALIDITY from
/// [`eprfs_meta::parse_meta_file`] — the kit's exact pair (`claims_subtree(cfg) and not
/// validate_meta(cfg)`). Reading the claim off the parsed record's `subject` does NOT work and the
/// reason is worth recording: `parse_meta_file` passes `subject_path: None`, and `to_record` maps
/// `covers: subtree` to `Subtree{path}` only when it has a path — with none it falls through to
/// `Projection`. A census built on the parsed subject therefore finds ZERO claims and reports a
/// coverage ratio of 0.0 on a repository with 31 of them, which is what the first cut of this
/// function did.
fn claims_subtree(dir: &Path) -> bool {
    let base = dir.join(eprfs_meta::MANIFEST_NAME);
    let manifest = if base.is_dir() {
        base.join(eprfs_meta::MANIFEST_FILE_NAME)
    } else {
        base.clone()
    };
    if !manifest.is_file() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return false;
    };
    let declares = text.lines().any(|line| {
        let line = line.trim();
        line.strip_prefix("covers:")
            .is_some_and(|v| v.trim().trim_matches('"').trim_matches('\'') == "subtree")
    });
    declares && eprfs_meta::parse_meta_file(&base).is_ok()
}

fn rel_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// A minimal `**`-aware glob match, enough for the four declared exclusions.
fn excluded(rel: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        let pattern = pattern.trim_end_matches('/');
        if let Some(prefix) = pattern.strip_suffix("/**") {
            return rel == prefix || rel.starts_with(&format!("{prefix}/"));
        }
        if let Some(suffix) = pattern.strip_prefix("**/") {
            return rel == suffix || rel.ends_with(&format!("/{suffix}"));
        }
        rel == pattern
    })
}

/// Submodule paths from `.gitmodules` — excluded automatically, per the manifest's own note.
fn submodule_paths(root: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(root.join(".gitmodules")) else {
        return BTreeSet::new();
    };
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("path")?.trim().strip_prefix('='))
        .map(|p| p.trim().to_string())
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Ratchet baselines, read from and written to the fold plane
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The latest fold per dimension for [`DIMENSION_MEASURE`] — the native replacement for
/// `context-coverage-baseline.json`.
pub fn baselines(root: &Path) -> FlowResult<BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    let sidecar = root.join(".eprfs/status/flows.jsonl");
    if !sidecar.exists() {
        // Asking a question must not create a store.
        return Ok(out);
    }
    for fold in super::report::read_folds(root)? {
        if fold.measure.to_string() != DIMENSION_MEASURE {
            continue;
        }
        let Some(dimension) = fold.env.get(DIMENSION_ENV_KEY) else {
            continue;
        };
        // Later folds overwrite earlier ones: `read_folds` is in sidecar append order, so the last
        // one wins and the baseline is the most recent ratchet, which is what a ratchet means.
        out.insert(dimension.clone(), fold.value);
    }
    Ok(out)
}

/// Append one observation per dimension — the ratchet act, as records anyone can re-derive.
pub fn fold_baselines(root: &Path, report: &StasisReport) -> FlowResult<usize> {
    let measures = root.join(".claude/epr-meta/measures.yaml");
    let mut appended = 0;
    for key in DIMENSION_KEYS {
        let Some(value) = report.dimensions.get(key) else {
            continue;
        };
        let mut env = BTreeMap::new();
        env.insert(DIMENSION_ENV_KEY.to_string(), key.to_string());
        let outcome = super::note::observe(
            root,
            "observation",
            DIMENSION_MEASURE,
            ".",
            *value,
            Some("ratio"),
            &env,
            Some(&format!(
                "context-coverage ratchet baseline for `{key}`, taken by `epr flow report placement --stasis --fold`"
            )),
            &super::note::NoteActor::default(),
            &measures,
        )
        .map_err(|err| {
            FlowError::InvalidArguments(format!(
                "cannot fold the ratchet baseline for `{key}`: {err}. The baseline pins \
                 `{DIMENSION_MEASURE}` on the repository with `env:{DIMENSION_ENV_KEY}=<dimension>`; \
                 a baseline that pins no declared measure is the private JSON file this replaces, \
                 wearing a different name"
            ))
        })?;
        if outcome.appended {
            appended += 1;
        }
    }
    Ok(appended)
}
