//! `epr flow report parity --inventory <path>` — the RETIREMENT ledger, folded against the tree.
//!
//! Station six of the memory-kit replacement
//! (`genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`): *"the parity
//! report shows every inventory row as native, relocated or retired with evidence"*. The inventory
//! (`genesis/docs/superpowers/plans/memory-kit-replacement/parity-inventory-2026-09-10.md`) is a
//! hand-taken ledger of 36 scripts and 17 report-tier artifacts. This module turns each of its rows
//! into one witnessed outcome, so "may the kit be deleted" is a derivation over the repository
//! rather than a sentence in a summary.
//!
//! **Three-valued, and `skipped` is the load-bearing member** — the same discipline
//! `super::report` holds for declared bounds, for the same reason. A row that names no checkable
//! replacement, or names no checkable test, is `skipped` NAMING WHAT IS MISSING. It is never
//! `passed`, because the whole failure mode this station exists to end is a ledger row that reads
//! as fine because nobody could check it.
//!
//! ## The three legs of a row
//!
//! A row is `passed` only when all three legs pass. Any failed leg fails the row; otherwise any
//! skipped leg skips it.
//!
//! 1. **Replacement.** Satisfied when the row's `parity` cell declares a *retirement with a
//!    reason*, or when a backticked claim in its replacement cell resolves: an `epr …` verb chain
//!    the binary recognises (and, when the chain answers `--help`, whose usage names each long flag
//!    the claim spelled), or a relocated path that exists. A claim like
//!    `` `epr flow concerns --stamp <doc>` `` is where this earns its keep: the chain is real and
//!    the flag is not, so the row fails naming the flag rather than passing on the verb's
//!    existence.
//! 2. **Tests.** Every path-shaped backticked token in the row's `tests` cell must resolve. A cell
//!    that names no path — `none`, `n/a`, or prose describing the checks — is `skipped`: prose is
//!    not evidence a report can re-derive, and admitting it would let an assertion pass itself.
//! 3. **References.** No executable reference to the row's kit artifact may remain, across
//!    `.py .js .mjs .json .yaml justfile .sh` under `.claude`, `genesis`, `elohim/sdk` and the root
//!    `justfile`.
//!
//! ## Two scan exclusions that are decisions, not conveniences
//!
//! The station brief names four exclusions (`.claude/memory-kit/gap-items`,
//! `.eprfs/status/gap-items`, the plan's own report directory, and `genesis/data/timeline/`). Three
//! more are added and are worth stating:
//!
//! - **The kit's own trees** (`.claude/scripts/memory-kit/`, `.claude/memory-kit/`) are excluded
//!   from the *gate*. `placement-audit.py` references `focus-baseline.py`; counting that would make
//!   every row fail until the kit is deleted, and the kit may only be deleted once every row
//!   passes — a deadlock in which the ledger can never reach the state that authorises the act it
//!   exists to authorise. Intra-kit hits are still COUNTED and reported per row as
//!   `selfReferences`, so nothing is hidden; they simply do not gate.
//! - **`.claude/worktrees/`** holds separate git worktrees of this same repository at other
//!   commits. They are other trees, not consumers of this one, and station six deletes from this
//!   one.
//!
//! - **`.claude/shifts/`** holds dated shift objectives — records of what a session set out to do on
//!   a given day. Like `genesis/data/timeline/` (which the brief already excludes) they are
//!   evidence, not wiring: a row cannot be cleared by editing history, and one that gated on history
//!   would be unpassable for a reason nobody may act on.
//!
//! A further non-gating class is a LINE shape rather than a path prefix: a `provenance:` value is a
//! citation, not a consumer (see [`CITATION_KEYS`]). It is reported per row as `citedReferences`.
//!
//! Both exclusions, and the extension set's blind spots (`.md`, `.ts`, `.rs` are outside it by the
//! brief's construction — prose surfaces and step files are station six's own rewrite, not this
//! gate's), are emitted as `omissions` on the payload rather than left for a reader to infer.
//!
//! **Method pin.** The payload carries the raw-codec CID of the exact inventory bytes read. Two
//! parity reports over different inventory revisions are different measurements, and the CID is
//! what makes that legible instead of invisible.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use eprfs_core::BlobCid;
use serde::Serialize;

use super::report::OutcomeStatus;
use super::{rel_to_root, FlowError, FlowResult};

/// File extensions the reference scan reads. Exactly the brief's set; `justfile` is matched by
/// file NAME rather than extension.
const SCAN_EXTENSIONS: &[&str] = &["py", "js", "mjs", "json", "yaml", "sh"];

/// Roots the reference scan walks, relative to the repository root. The root `justfile` is added
/// separately as a file.
const SCAN_ROOTS: &[&str] = &[".claude", "genesis", "elohim/sdk"];

/// Path prefixes excluded from the reference GATE (see the module doc for why each is here).
const SCAN_EXCLUDED: &[&str] = &[
    // Named by the station brief.
    ".claude/memory-kit/gap-items",
    ".eprfs/status/gap-items",
    "genesis/docs/superpowers/plans/memory-kit-replacement",
    // Dated records are EVIDENCE, not consumers. A timeline entry or a shift objective is a
    // statement about a tree that existed on the day it was written; editing one to clear a gate
    // would be falsifying a record, and leaving it to gate would make a row unpassable for a reason
    // nobody may act on. `genesis/data/timeline` was named by the brief; `.claude/shifts` is the
    // same kind of thing and is excluded on the same ground (operator ruling, 2026-09-11, after the
    // native seat surfaced the class rather than deciding it).
    "genesis/data/timeline",
    ".claude/shifts",
    // Other trees, not consumers of this one.
    ".claude/worktrees",
    // Gitignored, zero tracked files, regenerated by every run: a2o's own report output. A
    // cucumber receipt naming a script that once ran is a RECORD of a past run, not a consumer of
    // the tree, and it cannot be repaired by deleting anything — leaving it in the gate would make
    // six rows permanently unpassable for a reason station six has no act to address.
    "genesis/a2o/reports",
];

/// Path prefixes whose hits are the kit talking to itself: counted, reported, never gating.
const KIT_TREES: &[&str] = &[".claude/scripts/memory-kit", ".claude/memory-kit"];

/// Declaration keys whose VALUE is a citation rather than a producer: counted, reported, never
/// gating.
///
/// The registry states its own contract at `.claude/epr-meta/measures.yaml:190` — "`provenance:`
/// CITES the constant a row replaces; identity is `<id>@<version>` plus the row's own bytes, never
/// a file path." A citation names *where a number came from*, and the whole point of lifting a
/// hard-coded constant into a declared row is that the row can outlive the script the constant was
/// read out of. Gating on it would make the retirement of a script invalidate the record of what it
/// once taught us, which inverts the reason the lift happened.
///
/// `procedure:` is deliberately NOT here. It names the producer that must exist for the measure to
/// be takeable, so a `procedure:` naming a deleted script is a dangling method pin and a real
/// finding.
const CITATION_KEYS: &[&str] = &["provenance"];

/// Directory names never descended into.
const SKIP_DIRS: &[&str] = &["node_modules", "__pycache__", ".git", "target", "dist"];

/// A file larger than this is not read. Only derived report blobs reach it, and every one is named
/// in `omissions` rather than silently skipped.
const MAX_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// Directories a bare path claim is resolved against, in order. Explicit rather than a global
/// basename search: a hidden search can resolve a claim to an unrelated file with the same name,
/// and a refusal that lists exactly what was tried is actionable where a fuzzy hit is not.
const RESOLVE_ROOTS: &[&str] = &[
    "",
    ".claude/scripts",
    ".claude/scripts/memory-kit",
    ".claude/scripts/_lib",
    ".claude/hooks",
    "genesis/scripts",
    "elohim/sdk/domains/elohim-agent/scripts",
    "elohim/eprfs/epr-cli/tests",
    "elohim/eprfs/epr-cli/src/flow",
    ".epr-meta/elohim/lenses",
    ".epr-meta/elohim/algorithms",
];

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Payload
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// One thing a leg looked for, and what it found. Carried so a non-passed row names the exact
/// residue rather than a status word.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// What the inventory claimed, verbatim.
    pub claim: String,
    /// `verb` | `path` | `retire` — how the claim was read.
    pub kind: String,
    pub resolved: bool,
    /// Where it resolved, or what was tried and missed.
    pub evidence: String,
}

/// One leg's three-valued outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub status: OutcomeStatus,
    pub detail: String,
    pub checks: Vec<Check>,
}

impl Leg {
    fn skipped(detail: impl Into<String>) -> Self {
        Self {
            status: OutcomeStatus::Skipped,
            detail: detail.into(),
            checks: Vec::new(),
        }
    }
}

/// One executable reference that still names a kit artifact.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub path: String,
    pub line: usize,
    /// The kit token that matched.
    pub token: String,
    /// The matching line, trimmed and bounded — enough to judge whether it is an execution or a
    /// mention, without pasting a file into a report.
    pub text: String,
}

/// One inventory row, folded.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityRow {
    /// Which inventory table the row came from.
    pub table: String,
    /// The artifact cell, markdown stripped.
    pub artifact: String,
    /// The capability cell, for a reader who needs to know what is being retired.
    pub capability: String,
    /// The `parity` word the inventory ITSELF declares (`partial`, `missing`, `retire`, …),
    /// carried through uninterpreted. It does not gate — the three legs do — but a reader
    /// comparing a passing leg against a `partial` self-assessment is reading a stale cell, and
    /// that comparison is only possible if the declaration is on the payload.
    pub declared_parity: String,
    /// The path-shaped tokens the reference scan searched for.
    pub kit_tokens: Vec<String>,
    pub outcome: OutcomeStatus,
    pub summary: String,
    pub replacement: Leg,
    pub tests: Leg,
    pub references: Leg,
    /// Gating references — the exact residue a non-passed row must name.
    pub residue: Vec<Reference>,
    /// Intra-kit references. Reported, never gating (module doc).
    pub self_references: Vec<Reference>,
    /// Declaration CITATIONS — `provenance:` values naming this artifact as where a constant came
    /// from. Reported, never gating; see [`CITATION_KEYS`].
    pub cited_references: Vec<Reference>,
    /// Caveats that did not change the outcome but a reader should see.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub rows: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableReport {
    pub table: String,
    pub totals: Totals,
    pub rows: Vec<ParityRow>,
}

/// What the reference scan actually covered — the difference between "no references" and "nothing
/// was read".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub roots: Vec<String>,
    pub extensions: Vec<String>,
    pub excluded: Vec<String>,
    pub files_scanned: usize,
    pub bytes_scanned: u64,
    /// Files skipped for size, named individually.
    pub oversize_skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityPayload {
    /// The inventory this fold was taken against, repo-relative.
    pub inventory: String,
    /// **The method pin** — the raw-codec CID of the exact inventory bytes read.
    pub inventory_cid: String,
    pub inventory_bytes: usize,
    pub totals: Totals,
    pub tables: Vec<TableReport>,
    pub scan: ScanSummary,
    pub omissions: Vec<String>,
}

impl ParityPayload {
    /// The markdown a station-six report is pasted from rather than retyped.
    pub fn render(&self) {
        println!(
            "parity  {}@{}  ({} rows: {} passed · {} failed · {} skipped)",
            self.inventory,
            self.inventory_cid,
            self.totals.rows,
            self.totals.passed,
            self.totals.failed,
            self.totals.skipped
        );
        println!(
            "scan    {} files, {} bytes; roots {}; extensions {}",
            self.scan.files_scanned,
            self.scan.bytes_scanned,
            self.scan.roots.join(" "),
            self.scan.extensions.join(" ")
        );
        for table in &self.tables {
            println!(
                "\n## {} — {} passed · {} failed · {} skipped\n",
                table.table, table.totals.passed, table.totals.failed, table.totals.skipped
            );
            println!("| artifact | outcome | replacement | tests | references | residue |");
            println!("|---|---|---|---|---|---|");
            for row in &table.rows {
                println!(
                    "| `{}` | **{}** | {} | {} | {} | {} |",
                    row.artifact,
                    row.outcome.as_str(),
                    cell(&row.replacement.status, &row.replacement.detail),
                    cell(&row.tests.status, &row.tests.detail),
                    cell(&row.references.status, &row.references.detail),
                    bounded(
                        &row.residue
                            .iter()
                            .map(|r| format!("`{}:{}`", r.path, r.line))
                            .collect::<Vec<_>>(),
                    )
                );
            }
        }
        if !self.omissions.is_empty() {
            println!();
            for omission in &self.omissions {
                println!("Limit: {omission}");
            }
        }
    }
}

fn cell(status: &OutcomeStatus, detail: &str) -> String {
    format!("{} — {}", status.as_str(), detail.replace('|', "\\|"))
}

/// A bounded list for a table cell. The full set is always on the `--json` payload; a markdown
/// table that pasted forty file paths into one cell would be unreadable, and an unreadable
/// residue is one nobody acts on.
fn bounded(items: &[String]) -> String {
    if items.is_empty() {
        return "—".to_string();
    }
    if items.len() <= 5 {
        return items.join(", ");
    }
    format!("{}, …and {} more", items[..5].join(", "), items.len() - 5)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Entry points
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub fn usage() -> String {
    "usage: epr flow report parity --inventory <path> [--json] [--root DIR]\n  \
     (folds a memory-kit parity inventory's markdown tables against the tree: one \
     passed|failed|skipped outcome per row, method-pinned to the inventory bytes' raw CID. \
     A row passes when its replacement resolves, its named tests exist and no executable \
     consumer still references the kit path; a row that names nothing checkable is SKIPPED \
     naming what is missing, never passed)"
        .to_string()
}

/// Fold an inventory against `root`, probing verbs with the binary this process is.
pub fn parity(root: &Path, inventory: &Path) -> FlowResult<ParityPayload> {
    parity_with_binary(root, inventory, None)
}

/// The same fold with an explicit verb-probe binary — how an integration test points the probe at
/// `CARGO_BIN_EXE_epr` instead of at the test harness.
pub fn parity_with_binary(
    root: &Path,
    inventory: &Path,
    binary: Option<&Path>,
) -> FlowResult<ParityPayload> {
    let root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let inventory_abs = if inventory.is_absolute() {
        inventory.to_path_buf()
    } else {
        root.join(inventory)
    };
    let bytes = std::fs::read(&inventory_abs).map_err(|source| FlowError::Read {
        path: inventory_abs.clone(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let tables = parse_tables(&text);
    if tables.is_empty() {
        return Err(FlowError::InvalidArguments(format!(
            "no markdown tables with an `artifact` column found in {} — the parity fold reads the \
             inventory's tables, so an inventory with none is a refusal rather than an empty pass",
            inventory_abs.display()
        )));
    }

    let scan = Scan::build(&root)?;
    let probe = VerbProbe::new(binary);

    let mut table_reports = Vec::new();
    let mut totals = Totals::default();
    for table in &tables {
        let mut rows = Vec::new();
        let mut table_totals = Totals::default();
        for raw in &table.rows {
            let row = fold_row(&root, table, raw, &scan, &probe);
            table_totals.rows += 1;
            match row.outcome {
                OutcomeStatus::Passed => table_totals.passed += 1,
                OutcomeStatus::Failed => table_totals.failed += 1,
                OutcomeStatus::Skipped => table_totals.skipped += 1,
            }
            rows.push(row);
        }
        totals.rows += table_totals.rows;
        totals.passed += table_totals.passed;
        totals.failed += table_totals.failed;
        totals.skipped += table_totals.skipped;
        table_reports.push(TableReport {
            table: table.id.clone(),
            totals: table_totals,
            rows,
        });
    }

    let mut omissions = vec![
        "The reference gate reads only `.py .js .mjs .json .yaml justfile .sh`. Markdown prose \
         (agents, skills, commands, CLAUDE.md), TypeScript step files and Rust sources are OUTSIDE \
         it by construction: they are station six's own rewrite, not this gate's."
            .to_string(),
        "Intra-kit references (`.claude/scripts/memory-kit/`, `.claude/memory-kit/`) are counted \
         as `selfReferences` and never gate — the kit citing itself cannot block the deletion that \
         removes both ends, and gating on it would deadlock the ledger."
            .to_string(),
        "`provenance:` values are counted as `citedReferences` and never gate. The registry \
         declares them a CITATION of the constant a row replaces (.claude/epr-meta/measures.yaml), \
         deliberately allowed to name a line in a deleted script; `procedure:` names the producer \
         that must exist and DOES gate."
            .to_string(),
        "`.claude/worktrees/` is excluded: those are separate git worktrees of this repository at \
         other commits, not consumers of this tree. `genesis/a2o/reports/` is excluded on the same \
         ground: it is gitignored, has zero tracked files, and its receipts record past runs \
         rather than wiring a tree."
            .to_string(),
        "Dated records are excluded from the gate: `genesis/data/timeline/` (named by the station \
         brief) and `.claude/shifts/`. Both are statements about a tree that existed on the day \
         they were written — evidence, not consumers — and a row cannot honestly be cleared by \
         editing history."
            .to_string(),
        "A row is checked against what the inventory SAYS. An inventory cell that is stale with \
         respect to the tree yields a skipped or failed row naming the stale cell — re-taking the \
         inventory is the repair, not re-running this fold."
            .to_string(),
    ];
    for skipped in &scan.oversize_skipped {
        omissions.push(format!(
            "{skipped} exceeds {MAX_SCAN_BYTES} bytes and was not read; a reference inside it \
             would not be seen."
        ));
    }
    if !probe.available {
        omissions.push(format!(
            "No `epr` binary was reachable at `{}`: every native-verb claim is reported unresolved \
             rather than assumed present.",
            probe.binary.display()
        ));
    }

    Ok(ParityPayload {
        inventory: rel_to_root(&root, &inventory_abs),
        inventory_cid: BlobCid::compute_raw(&bytes).to_string(),
        inventory_bytes: bytes.len(),
        totals,
        tables: table_reports,
        scan: ScanSummary {
            roots: SCAN_ROOTS
                .iter()
                .map(ToString::to_string)
                .chain(std::iter::once("justfile".to_string()))
                .collect(),
            extensions: SCAN_EXTENSIONS.iter().map(ToString::to_string).collect(),
            excluded: SCAN_EXCLUDED.iter().map(ToString::to_string).collect(),
            files_scanned: scan.files_scanned,
            bytes_scanned: scan.bytes_scanned,
            oversize_skipped: scan.oversize_skipped.clone(),
        },
        omissions,
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Row folding
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn fold_row(
    root: &Path,
    table: &Table,
    raw: &BTreeMap<String, String>,
    scan: &Scan,
    probe: &VerbProbe,
) -> ParityRow {
    let artifact_cell = raw.get("artifact").cloned().unwrap_or_default();
    let capability = raw.get("capability").cloned().unwrap_or_default();
    let replacement_cell = raw
        .get("native replacement today")
        .or_else(|| raw.get("native replacement"))
        .or_else(|| raw.get("replacement"))
        .cloned()
        .unwrap_or_default();
    let parity_cell = raw.get("parity").cloned().unwrap_or_default();
    let tests_cell = raw.get("tests").cloned().unwrap_or_default();

    let kit_tokens: Vec<String> = backticked(&artifact_cell)
        .into_iter()
        .filter(|t| looks_like_path(t))
        .collect();

    let mut notes = Vec::new();
    let replacement = replacement_leg(root, &replacement_cell, &parity_cell, probe, &mut notes);
    let tests = tests_leg(root, &tests_cell);
    let (references, residue, self_references, cited_references) =
        references_leg(table, &artifact_cell, &kit_tokens, scan);

    let legs = [&replacement.status, &tests.status, &references.status];
    let outcome = if legs.iter().any(|s| **s == OutcomeStatus::Failed) {
        OutcomeStatus::Failed
    } else if legs.iter().any(|s| **s == OutcomeStatus::Skipped) {
        OutcomeStatus::Skipped
    } else {
        OutcomeStatus::Passed
    };
    let summary = match outcome {
        OutcomeStatus::Passed => {
            "replacement resolves, tests exist, no executable reference remains".to_string()
        }
        OutcomeStatus::Failed => [&replacement, &tests, &references]
            .iter()
            .filter(|leg| leg.status == OutcomeStatus::Failed)
            .map(|leg| leg.detail.clone())
            .collect::<Vec<_>>()
            .join("; "),
        OutcomeStatus::Skipped => [&replacement, &tests, &references]
            .iter()
            .filter(|leg| leg.status == OutcomeStatus::Skipped)
            .map(|leg| leg.detail.clone())
            .collect::<Vec<_>>()
            .join("; "),
    };

    ParityRow {
        table: table.id.clone(),
        artifact: plain(&artifact_cell),
        capability: plain(&capability),
        declared_parity: plain(&parity_cell),
        kit_tokens,
        outcome,
        summary,
        replacement,
        tests,
        references,
        residue,
        self_references,
        cited_references,
        notes,
    }
}

/// Leg 1 — does a replacement exist?
fn replacement_leg(
    root: &Path,
    cell: &str,
    parity_cell: &str,
    probe: &VerbProbe,
    notes: &mut Vec<String>,
) -> Leg {
    if let Some(reason) = retirement_reason(parity_cell) {
        return Leg {
            status: OutcomeStatus::Passed,
            detail: format!("retired with a reason: {reason}"),
            checks: vec![Check {
                claim: strip_markdown(parity_cell),
                kind: "retire".into(),
                resolved: true,
                evidence: reason,
            }],
        };
    }

    // A cell that OPENS with a negation declares that nothing native covers this row. Its later
    // backticks are prose about what does NOT cover it — `none (`epr check`/`govern` gate writes,
    // not doc drift)` is the live example — and reading one of them as a satisfied claim would turn
    // an explicit "no replacement" into a pass. The first word decides, before any token is read.
    if let Some(word) = negation_lead(cell) {
        return Leg::skipped(format!(
            "the inventory declares no replacement (cell opens `{word}`: {})",
            summarize(cell)
        ));
    }

    let mut checks = Vec::new();
    for token in backticked(cell) {
        if let Some(chain) = verb_chain(&token) {
            let (resolved, evidence, caveat) = probe.check(&chain, &token);
            if let Some(caveat) = caveat {
                notes.push(caveat);
            }
            checks.push(Check {
                claim: token,
                kind: "verb".into(),
                resolved,
                evidence,
            });
        } else if let Some(candidate) = path_claim(&token) {
            let (resolved, evidence) = resolve_path(root, &candidate, None);
            checks.push(Check {
                claim: token,
                kind: "path".into(),
                resolved,
                evidence,
            });
        }
        // Anything else is prose inside backticks (`SlugIndex`, `meta seal --dir`) — not a claim
        // this fold can check, and counting it as an unresolved claim would fail rows for their
        // authors' punctuation.
    }

    if checks.is_empty() {
        return Leg::skipped(format!(
            "no checkable replacement named (cell: {})",
            summarize(cell)
        ));
    }
    if checks.iter().any(|c| c.resolved) {
        let resolved: Vec<&str> = checks
            .iter()
            .filter(|c| c.resolved)
            .map(|c| c.claim.as_str())
            .collect();
        Leg {
            status: OutcomeStatus::Passed,
            detail: format!("resolves: {}", resolved.join(", ")),
            checks,
        }
    } else {
        let missed: Vec<String> = checks
            .iter()
            .map(|c| format!("{} ({})", c.claim, c.evidence))
            .collect();
        Leg {
            status: OutcomeStatus::Failed,
            detail: format!("no named replacement resolves: {}", missed.join("; ")),
            checks,
        }
    }
}

/// Leg 2 — do the row's named tests exist?
///
/// The in-cell directory carry is the cell grammar the inventory actually uses:
/// `` `_lib/__tests__/a_test.py`, `b_test.py` `` means both live under `_lib/__tests__/`. Without
/// it the second token would be reported missing, which is a defect in the reader rather than in
/// the tree.
fn tests_leg(root: &Path, cell: &str) -> Leg {
    let mut checks = Vec::new();
    let mut prefix: Option<String> = None;
    for token in backticked(cell) {
        let Some(candidate) = path_claim(&token) else {
            continue;
        };
        let carry = if candidate.contains('/') {
            None
        } else {
            prefix.clone()
        };
        let (resolved, evidence) = resolve_path(root, &candidate, carry.as_deref());
        // The carry is the directory a token RESOLVED in, not the directory it was spelled with:
        // `_lib/__tests__/kept_test.py` is written relative to the kit and resolves under
        // `.claude/scripts/`, and a sibling spelled bare in the same cell lives beside the
        // resolved file rather than beside the spelling.
        if resolved {
            prefix = Path::new(&evidence)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .filter(|p| !p.is_empty());
        }
        checks.push(Check {
            claim: token,
            kind: "path".into(),
            resolved,
            evidence,
        });
    }
    if checks.is_empty() {
        return Leg::skipped(format!(
            "the inventory names no checkable test path (cell: {})",
            summarize(cell)
        ));
    }
    let missing: Vec<String> = checks
        .iter()
        .filter(|c| !c.resolved)
        .map(|c| format!("{} ({})", c.claim, c.evidence))
        .collect();
    if missing.is_empty() {
        Leg {
            status: OutcomeStatus::Passed,
            detail: format!("{} named test path(s) exist", checks.len()),
            checks,
        }
    } else {
        Leg {
            status: OutcomeStatus::Failed,
            detail: format!("named test path(s) missing: {}", missing.join("; ")),
            checks,
        }
    }
}

/// Leg 3 — does any executable consumer still name the kit artifact?
fn references_leg(
    table: &Table,
    artifact_cell: &str,
    tokens: &[String],
    scan: &Scan,
) -> (Leg, Vec<Reference>, Vec<Reference>, Vec<Reference>) {
    if tokens.is_empty() {
        return (
            Leg::skipped(format!(
                "the artifact cell names no single kit path to search for (cell: {})",
                summarize(artifact_cell)
            )),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
    }
    let mut gating = Vec::new();
    let mut internal = Vec::new();
    let mut cited = Vec::new();
    for token in tokens {
        for hit in scan.find(token) {
            if KIT_TREES.iter().any(|kit| hit.path.starts_with(kit)) {
                internal.push(hit);
            } else if is_citation_line(&hit.text) {
                cited.push(hit);
            } else {
                gating.push(hit);
            }
        }
    }
    let detail = if gating.is_empty() {
        format!(
            "no executable reference to {} ({} intra-kit, {} cited)",
            tokens.join(", "),
            internal.len(),
            cited.len()
        )
    } else {
        format!(
            "{} executable reference(s) remain: {}",
            gating.len(),
            bounded(
                &gating
                    .iter()
                    .map(|r| format!("{}:{}", r.path, r.line))
                    .collect::<Vec<_>>()
            )
        )
    };
    let leg = Leg {
        status: if gating.is_empty() {
            OutcomeStatus::Passed
        } else {
            OutcomeStatus::Failed
        },
        detail,
        checks: tokens
            .iter()
            .map(|t| Check {
                claim: t.clone(),
                kind: "path".into(),
                resolved: true,
                evidence: format!("searched under {}", table.id),
            })
            .collect(),
    };
    (leg, gating, internal, cited)
}

/// Is this matching line a CITATION of where a constant came from, rather than a producer?
///
/// A line-shape rule, and it is exact for the corpus it governs: all 46 `provenance:` values in
/// `.claude/epr-meta/measures.yaml` are single-line double-quoted scalars, so a key test on the
/// trimmed line cannot miss a continuation. The optional `- ` strip admits the list-item spelling;
/// the quoted form admits the JSON one, so a registry that moves format keeps the rule.
fn is_citation_line(text: &str) -> bool {
    let line = text.trim_start().trim_start_matches("- ").trim_start();
    CITATION_KEYS.iter().any(|key| {
        line.strip_prefix(key)
            .or_else(|| line.strip_prefix(&format!("\"{key}\"")))
            .is_some_and(|rest| rest.trim_start().starts_with(':'))
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Claim parsing
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The contents of every `` `…` `` span in a cell, in order.
pub fn backticked(cell: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let span = after[..close].trim();
        if !span.is_empty() {
            out.push(span.to_string());
        }
        rest = &after[close + 1..];
    }
    out
}

/// Does this token name a file or directory? Extension-or-trailing-slash, no whitespace, at least
/// one letter — so `agent-audit.py`, `gap-items/` and `.prev.json` are paths and `2026-05-10`,
/// `SlugIndex` and `epr flow stocks --check` are not.
pub fn looks_like_path(token: &str) -> bool {
    if token.contains(char::is_whitespace) || !token.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if token.ends_with('/') {
        return true;
    }
    let Some((_, ext)) = token.rsplit_once('.') else {
        return false;
    };
    !ext.is_empty() && ext.len() <= 6 && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

/// A path claim inside a replacement/tests cell: the token itself, or its first word when the
/// token spells an invocation (`package-projections.mjs verify`).
fn path_claim(token: &str) -> Option<String> {
    let bare = strip_locator(token);
    if looks_like_path(bare) {
        return Some(bare.to_string());
    }
    let first = strip_locator(bare.split_whitespace().next()?);
    looks_like_path(first).then(|| first.to_string())
}

/// The verb chain of an `epr …` claim: the bare words after `epr`, stopping at the first flag or
/// `<placeholder>`. `None` for any token that is not an `epr` invocation.
fn verb_chain(token: &str) -> Option<Vec<String>> {
    let mut words = token.split_whitespace();
    if words.next()? != "epr" {
        return None;
    }
    let chain: Vec<String> = words
        .take_while(|w| !w.starts_with('-') && !w.starts_with('<') && !w.contains('|'))
        .map(ToString::to_string)
        .collect();
    (!chain.is_empty()).then_some(chain)
}

/// The long flags a claim spelled, so a claim's FLAG can be checked and not just its verb.
fn claimed_flags(token: &str) -> Vec<String> {
    token
        .split_whitespace()
        .filter(|w| w.starts_with("--") && w.len() > 2)
        .map(|w| w.trim_end_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-'))
        .map(ToString::to_string)
        .collect()
}

/// The retirement reason a `parity` cell declares, or `None` when it declares no retirement.
///
/// The first word decides, which is what keeps `**NOT retirable (measured …)**` out: it declares
/// the OPPOSITE of a retirement, and a substring match on "retir" would read it as one.
fn retirement_reason(cell: &str) -> Option<String> {
    let plain = strip_markdown(cell);
    let first = plain.split_whitespace().next()?.to_ascii_lowercase();
    if !(first == "retire" || first == "retired" || first == "retire," || first == "retired,") {
        return None;
    }
    let reason = plain
        .split_once(['—', ';', ':'])
        .map(|(_, tail)| tail.trim())
        .unwrap_or("")
        .trim_start_matches('-')
        .trim();
    (!reason.is_empty()).then(|| reason.to_string())
}

/// The negation a replacement cell opens with, when it opens with one.
///
/// `none`, `n/a`, `no <x> equivalent`, `missing`, and the inventory's `same as above` back-pointer
/// (which names no claim of its own and would otherwise inherit the previous row's silence).
fn negation_lead(cell: &str) -> Option<String> {
    let plain = plain(cell).to_ascii_lowercase();
    let first: String = plain
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '/')
        .to_string();
    let negations = ["none", "n/a", "no", "missing", "same", "n/a."];
    negations.contains(&first.as_str()).then_some(first)
}

fn strip_markdown(cell: &str) -> String {
    cell.replace("**", "").trim().to_string()
}

/// Display text: markdown emphasis and code fences removed. Used for the `artifact` and
/// `capability` fields, which a reader reads as prose; `summarize` keeps the fences because a
/// refusal that quotes a cell should quote what the cell says.
fn plain(cell: &str) -> String {
    strip_markdown(cell).replace('`', "").trim().to_string()
}

/// A token with any trailing `:<line>` / `:<line>-<line>` locator removed.
///
/// The inventory writes `` `_lib/__tests__/cluster_state_test.py:55` `` — a path AND the line the
/// reference sits on. Shape-testing the whole token reads the locator as part of the extension and
/// reports an existing file as unnameable, which is a defect in the reader rather than in the tree.
fn strip_locator(token: &str) -> &str {
    let Some((head, tail)) = token.rsplit_once(':') else {
        return token;
    };
    let numeric = !tail.is_empty()
        && tail.chars().all(|c| c.is_ascii_digit() || c == '-')
        && tail.chars().next().is_some_and(|c| c.is_ascii_digit());
    if numeric {
        head
    } else {
        token
    }
}

/// A cell, bounded for a refusal message. The whole cell in a refusal buries the refusal.
fn summarize(cell: &str) -> String {
    let plain = strip_markdown(cell);
    if plain.chars().count() <= 90 {
        return plain;
    }
    let head: String = plain.chars().take(87).collect();
    format!("{head}…")
}

/// Resolve a path claim against the ladder, returning what resolved or everything that was tried.
fn resolve_path(root: &Path, claim: &str, carry: Option<&str>) -> (bool, String) {
    let mut tried = Vec::new();
    let mut candidates: Vec<String> = Vec::new();
    if let Some(prefix) = carry {
        candidates.push(format!("{prefix}/{claim}"));
    }
    for base in RESOLVE_ROOTS {
        candidates.push(if base.is_empty() {
            claim.to_string()
        } else {
            format!("{base}/{claim}")
        });
    }
    for candidate in candidates {
        let path = root.join(candidate.trim_end_matches('/'));
        if path.exists() {
            return (true, candidate);
        }
        tried.push(candidate);
    }
    (false, format!("not found; tried {}", tried.join(", ")))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Verb probe
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Probes whether the binary recognises a verb chain, by asking it for help.
struct VerbProbe {
    binary: PathBuf,
    available: bool,
}

impl VerbProbe {
    fn new(explicit: Option<&Path>) -> Self {
        let binary = explicit.map(Path::to_path_buf).unwrap_or_else(|| {
            // The binary this process IS, when it is the CLI; otherwise whatever `epr` PATH names.
            // The distinction matters under `cargo test`, where `current_exe` is a test harness —
            // hence `parity_with_binary`, which a test uses to point at `CARGO_BIN_EXE_epr`.
            std::env::current_exe()
                .ok()
                .filter(|p| p.file_stem().map(|s| s == "epr").unwrap_or(false))
                .unwrap_or_else(|| PathBuf::from("epr"))
        });
        let available = crate::process::build_command(
            &binary.to_string_lossy(),
            &["--help"],
            Path::new("."),
            &[],
        )
        .output()
        .is_ok();
        Self { binary, available }
    }

    /// `(resolved, evidence, caveat)` for one `epr …` claim.
    fn check(&self, chain: &[String], token: &str) -> (bool, String, Option<String>) {
        if !self.available {
            return (
                false,
                format!("no `epr` binary at {}", self.binary.display()),
                None,
            );
        }
        let mut args: Vec<&str> = chain.iter().map(String::as_str).collect();
        args.push("--help");
        let output = crate::process::build_command(
            &self.binary.to_string_lossy(),
            &args,
            Path::new("."),
            &[],
        )
        .output();
        let Ok(output) = output else {
            return (false, "the probe could not be spawned".into(), None);
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // "unknown … subcommand `x`" is the CLI's own shape for a verb it does not have. Anything
        // else — help on stdout, or an argument complaint about `--help` — means the chain was
        // reached and parsed by the verb it names.
        let unknown = text.contains("unknown") && text.contains("subcommand");
        if unknown {
            return (
                false,
                format!("`epr {}` is not a verb this binary has", chain.join(" ")),
                None,
            );
        }
        // A chain that answered with its own usage can also be asked about its FLAGS.
        let answers_help = output.status.success() || text.contains("usage: epr");
        let flags = claimed_flags(token);
        if flags.is_empty() {
            return (
                true,
                format!("`epr {} --help` answers", chain.join(" ")),
                None,
            );
        }
        if !answers_help {
            return (
                true,
                format!(
                    "`epr {}` is a verb; it does not answer --help, so {} is unverified",
                    chain.join(" "),
                    flags.join(" ")
                ),
                Some(format!(
                    "`{token}`: the verb exists but does not answer `--help`, so the claimed \
                     flag(s) {} could not be verified",
                    flags.join(" ")
                )),
            );
        }
        let missing: Vec<&String> = flags
            .iter()
            .filter(|f| !text.contains(f.as_str()))
            .collect();
        if missing.is_empty() {
            (
                true,
                format!("`epr {} --help` names {}", chain.join(" "), flags.join(" ")),
                None,
            )
        } else {
            (
                false,
                format!(
                    "`epr {}` exists but its usage does not name {}",
                    chain.join(" "),
                    missing
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                None,
            )
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Reference scan
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The executable corpus, read once. Reading every candidate file once and searching it for every
/// token beats re-walking the tree per row by two orders of magnitude, and it makes the scan
/// summary a single honest number.
struct Scan {
    files: Vec<(String, String)>,
    files_scanned: usize,
    bytes_scanned: u64,
    oversize_skipped: Vec<String>,
}

impl Scan {
    fn build(root: &Path) -> FlowResult<Self> {
        let mut scan = Scan {
            files: Vec::new(),
            files_scanned: 0,
            bytes_scanned: 0,
            oversize_skipped: Vec::new(),
        };
        for rel in SCAN_ROOTS {
            scan.walk(root, &root.join(rel));
        }
        let justfile = root.join("justfile");
        if justfile.is_file() {
            scan.read(root, &justfile);
        }
        scan.files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(scan)
    }

    fn walk(&mut self, root: &Path, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let rel = rel_to_root(root, &path);
            if SCAN_EXCLUDED
                .iter()
                .any(|ex| rel == *ex || rel.starts_with(&format!("{ex}/")))
            {
                continue;
            }
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            if path.is_dir() {
                if name.as_deref().is_some_and(|n| SKIP_DIRS.contains(&n)) {
                    continue;
                }
                self.walk(root, &path);
                continue;
            }
            let interesting = name.as_deref() == Some("justfile")
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| SCAN_EXTENSIONS.contains(&e));
            if interesting {
                self.read(root, &path);
            }
        }
    }

    fn read(&mut self, root: &Path, path: &Path) {
        let rel = rel_to_root(root, path);
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_SCAN_BYTES {
            self.oversize_skipped.push(rel);
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            // A non-UTF-8 file under these extensions is not source anybody executes; skipping it
            // silently is safe in a way skipping a readable one would not be.
            return;
        };
        self.files_scanned += 1;
        self.bytes_scanned += size;
        self.files.push((rel, text));
    }

    /// Every line naming `token`, deduped by (path, line).
    fn find(&self, token: &str) -> Vec<Reference> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for (path, text) in &self.files {
            if !text.contains(token) {
                continue;
            }
            for (index, line) in text.lines().enumerate() {
                if !line.contains(token) {
                    continue;
                }
                if !seen.insert((path.clone(), index + 1)) {
                    continue;
                }
                let trimmed = line.trim();
                let text: String = trimmed.chars().take(160).collect();
                out.push(Reference {
                    path: path.clone(),
                    line: index + 1,
                    token: token.to_string(),
                    text,
                });
            }
        }
        out
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Markdown table reading
// ───────────────────────────────────────────────────────────────────────────────────────────────

struct Table {
    id: String,
    rows: Vec<BTreeMap<String, String>>,
}

/// Read every markdown table that has an `artifact` column, keyed by its enclosing `##` heading.
///
/// The cell splitter is backtick-aware because the inventory contains at least one cell whose
/// prose includes a pipe inside code (`` `Edit|Write` ``). A naive `split('|')` shifts that row's
/// columns by one and silently reads the wrong cell as the replacement — the class of defect this
/// whole station exists to stop shipping.
fn parse_tables(text: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut heading = String::from("table");
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(rest) = line.strip_prefix("## ") {
            heading = slug(rest);
            i += 1;
            continue;
        }
        if !line.trim_start().starts_with('|') {
            i += 1;
            continue;
        }
        let header = split_row(line);
        let is_table = lines
            .get(i + 1)
            .is_some_and(|next| next.trim_start().starts_with('|') && next.contains("---"));
        if !is_table || !header.iter().any(|h| h.eq_ignore_ascii_case("artifact")) {
            i += 1;
            continue;
        }
        let keys: Vec<String> = header.iter().map(|h| h.to_ascii_lowercase()).collect();
        let mut rows = Vec::new();
        let mut j = i + 2;
        while j < lines.len() && lines[j].trim_start().starts_with('|') {
            let cells = split_row(lines[j]);
            let mut row = BTreeMap::new();
            for (key, cell) in keys.iter().zip(cells.iter()) {
                row.insert(key.clone(), cell.clone());
            }
            if row.get("artifact").is_some_and(|a| !a.is_empty()) {
                rows.push(row);
            }
            j += 1;
        }
        tables.push(Table {
            id: heading.clone(),
            rows,
        });
        i = j;
    }
    tables
}

/// Split one markdown table row into trimmed cells, treating a `|` inside a backtick span as text.
fn split_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    for ch in line.trim().chars() {
        match ch {
            '`' => {
                in_code = !in_code;
                current.push(ch);
            }
            '|' if !in_code => {
                cells.push(current.trim().to_string());
                current.clear();
            }
            other => current.push(other),
        }
    }
    cells.push(current.trim().to_string());
    if cells.first().is_some_and(String::is_empty) {
        cells.remove(0);
    }
    if cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

fn slug(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pipe_inside_backticks_does_not_shift_a_rows_columns() {
        let row = "| `memory-index-projector.py` | Project the index | `.claude/settings.json:206` — PostToolUse `Edit|Write` hook | rewrites `MEMORY.md` | none | missing | `__tests__/x_test.py` |";
        let cells = split_row(row);
        assert_eq!(cells.len(), 7, "seven columns, not eight: {cells:?}");
        assert_eq!(cells[4], "none", "the replacement cell must stay in place");
        assert_eq!(cells[5], "missing");
    }

    #[test]
    fn path_shape_admits_files_and_dirs_and_refuses_dates_and_prose() {
        assert!(looks_like_path("agent-audit.py"));
        assert!(looks_like_path("gap-items/"));
        assert!(looks_like_path(".prev.json"));
        assert!(!looks_like_path("2026-05-10"));
        assert!(!looks_like_path("SlugIndex"));
        assert!(!looks_like_path("epr flow stocks --check"));
    }

    #[test]
    fn a_retirement_needs_a_reason_and_not_is_never_one() {
        assert_eq!(
            retirement_reason("retire — apply leg disabled; proposals lack custody evidence")
                .as_deref(),
            Some("apply leg disabled; proposals lack custody evidence")
        );
        assert!(retirement_reason("**retired 2026-09-10** — script deleted").is_some());
        assert!(
            retirement_reason("**NOT retirable (measured 2026-09-10)** — dry run proposes 225")
                .is_none(),
            "a cell that declares the opposite of a retirement must not read as one"
        );
        assert!(
            retirement_reason("retire").is_none(),
            "a bare word is not a reason"
        );
        assert!(retirement_reason("partial — read-side converges").is_none());
    }

    #[test]
    fn a_verb_chain_stops_at_the_first_flag_or_placeholder() {
        assert_eq!(
            verb_chain("epr flow concerns --stamp <doc>"),
            Some(vec!["flow".into(), "concerns".into()])
        );
        assert_eq!(
            claimed_flags("epr flow concerns --stamp <doc>"),
            vec!["--stamp".to_string()]
        );
        assert_eq!(
            verb_chain("epr flow memory collective|pin"),
            Some(vec!["flow".into(), "memory".into()])
        );
        assert!(verb_chain("package-projections.mjs verify").is_none());
    }

    #[test]
    fn an_invocation_token_yields_its_program_as_the_path_claim() {
        assert_eq!(
            path_claim("package-projections.mjs verify").as_deref(),
            Some("package-projections.mjs")
        );
        assert_eq!(path_claim("none"), None);
    }
}
