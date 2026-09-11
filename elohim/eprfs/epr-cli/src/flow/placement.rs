//! `epr flow report placement` — every file's placement state, derived rather than filed.
//!
//! The native port of `placement-audit.py --ledger`. The kit's ledger answers one question well:
//! *where does every document sit, what state is it in, and what advances it*. This module answers
//! the same question from the same inputs — the doc surfaces, frontmatter status, `.claude/memory`
//! link state, and `genesis/manifests/cluster-state.yaml` — with two differences that matter.
//!
//! **No state file.** The kit writes `.claude/memory-kit/state-ledger.json` on every run and reads
//! `.claude/memory-kit/gap-items/*.json` for its implementation budget. Neither is a source: the
//! ledger is a rendering of the tree, and the gap budget is a rendering of the plans' own checkbox
//! stations. Both are derived here, live, from the documents — so there is nothing to go stale and
//! nothing to disagree with.
//!
//! **Absence is `unknown`, never zero.** Where the kit would have read a memory-kit JSON accumulator
//! and rendered a missing one as a default, this report says `unknown`. A gap roll-up over a corpus
//! with no extractable stations reports `unknown`, not `0 OPEN` — "nobody measured" and "measured,
//! nothing there" are the two states this replacement exists to keep apart.
//!
//! The state vocabulary is the kit's, transcribed exactly: `ACTIVE`, `LINKED`, `MEM-UNLINKED`,
//! `NEEDS-TRIAGE`, `CLAIMED-ONLY`, `SETTLED`, `SUPERSEDED`, `VERIFIED-STABLE`, `UNKNOWN-STATUS`,
//! plus the two the kit derives from placement rather than status — `REGRESSED` and
//! `BLOCKED-BY-ENV`. All eleven are ported: dropping the two would silently re-bucket every
//! pressure-dir doc and every env-held doc into a state they are not in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::env_scope;
use super::gaps;
use super::scope;
use super::{parse_frontmatter, FlowError, FlowResult};

/// The doc surfaces, in the order the kit scans them. The order is the ledger's row order, so it is
/// a rendering contract rather than a preference.
const SURFACES: [(&str, &str); 7] = [
    ("ACTIVE:specs", "genesis/docs/superpowers/specs"),
    ("ACTIVE:plans", "genesis/docs/superpowers/plans"),
    ("ACTIVE:plans(legacy)", "genesis/docs/plans"),
    (
        "CANONICAL",
        "genesis/docs/content/elohim-protocol/architecture",
    ),
    ("HISTORY", "genesis/docs/content/elohim-protocol/history"),
    ("NOTES", "genesis/docs/superpowers/notes"),
    ("RESEARCH", "genesis/docs/research"),
];

const MEMORY_DIR: &str = ".claude/memory";
const STATE_ROOT: &str = "genesis/docs/_state";

/// Filenames that are structure, not content, in every surface.
const SKIP_NAMES: [&str; 3] = ["INDEX.md", "CLAUDE.md", "README.md"];

/// Status words, by bucket. Transcribed from the kit; a word in none of them is `UNKNOWN`.
const LANDED_WORDS: [&str; 8] = [
    "landed",
    "stable",
    "done",
    "complete",
    "completed",
    "shipped",
    "accepted",
    "latest-stable",
];
const CLAIMED_WORDS: [&str; 1] = ["claimed-not-verified"];
const REGRESSED_WORDS: [&str; 2] = ["regressed", "regression"];
const DEAD_WORDS: [&str; 6] = [
    "superseded",
    "abandoned",
    "cancelled",
    "canceled",
    "deprecated",
    "retired",
];
const ACTIVE_WORDS: [&str; 11] = [
    "draft",
    "design",
    "brainstorm",
    "proposal",
    "proposed",
    "in-flight",
    "inflight",
    "wip",
    "vision",
    "approved",
    "accepted-draft",
];
/// PLACEMENT.md §16: a CANONICAL seed's status leads with a descriptive permanence word. Recognized
/// only when the doc's home IS canonical — a "living" status on an active-home plan is a smell.
const LIVING_WORDS: [&str; 4] = ["living", "architecture", "principle", "pattern"];

/// The pressure states, in the kit's queue order. This ordering is what makes the queue a queue.
const PRESSURE_ORDER: [&str; 6] = [
    "NEEDS-TRIAGE",
    "MEM-UNLINKED",
    "CLAIMED-ONLY",
    "REGRESSED",
    "SUPERSEDED",
    "UNKNOWN-STATUS",
];

/// States that are a resting place rather than a queue.
const SETTLED_STATES: [&str; 4] = ["VERIFIED-STABLE", "ACTIVE", "LINKED", "CANONICAL"];

/// One ledger row: a file, where it sits, what state it is in, and what advances it.
#[derive(Debug, Clone, Serialize)]
pub struct PlacementRow {
    pub path: String,
    /// The surface home, or `MEMORY` for a `.claude/memory` entry.
    pub position: String,
    pub state: String,
    pub next: String,
}

/// The native gap roll-up — the implementation budget, derived from the plans themselves.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GapRollup {
    /// `unknown` when no document in the corpus yielded an extractable station. Never a zero.
    pub known: bool,
    pub docs_with_items: usize,
    pub docs_scanned: usize,
    pub open: usize,
    pub claimed: usize,
    pub blocked: usize,
    /// `(doc, open, claimed, blocked)` for every doc with items, in corpus order.
    pub per_doc: Vec<GapDocRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GapDocRow {
    pub doc: String,
    pub open: usize,
    pub claimed: usize,
    pub blocked: usize,
}

/// The whole placement reading.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementReport {
    pub command: String,
    pub total_files: usize,
    pub rows: Vec<PlacementRow>,
    /// state → count. Sums to `total_files`; nothing hides.
    pub balance: BTreeMap<String, usize>,
    pub pressure: usize,
    pub held: usize,
    pub settled: usize,
    pub gaps: GapRollup,
}

/// One scanned document, before its state is derived.
struct DocRecord {
    home: String,
    path: String,
    abs: PathBuf,
    bucket: &'static str,
    verified: bool,
    requires_env: Vec<String>,
}

/// Options resolved by the CLI shell.
#[derive(Debug, Default)]
pub struct PlacementOptions {
    pub ledger: bool,
    /// `--coverage`: the un-reviewed workload the stasis loop drains.
    pub coverage: bool,
    /// `--stasis`: the composite context-coverage readout, with the ratchet.
    pub stasis: bool,
    /// `--fold`: append this run's dimension ratios as the ratchet baseline. Only meaningful with
    /// `--stasis`, and refused elsewhere rather than ignored.
    pub fold: bool,
    /// `Some(None)` = the whole focus baseline; `Some(Some(subject))` = one subject.
    pub focus: Option<Option<String>>,
    /// `--brief`: the narrowed subject names and a drill-in pointer, nothing more.
    pub brief: bool,
}

/// Derive the placement reading over `root`.
pub fn placement(
    root: &Path,
    cluster_state_override: Option<&Path>,
) -> FlowResult<PlacementReport> {
    let cluster_path = cluster_state_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("genesis/manifests/cluster-state.yaml"));
    let state = super::cluster_state::load(&cluster_path);
    // The `known` set is the DECLARED one: a role-only block makes no runtime claim and must not
    // silently bench work (decision 2 of the cluster-state parser).
    let known = state.declared_names();
    let available = state.available_names();

    let docs = scan_docs(root);
    let mut rows: Vec<PlacementRow> = Vec::with_capacity(docs.len());
    for doc in &docs {
        let (st, next) = budget_state(doc, &available);
        rows.push(PlacementRow {
            path: doc.path.clone(),
            position: doc.home.clone(),
            state: st,
            next,
        });
    }
    for entry in scan_memory(root) {
        let missing: Vec<&String> = entry
            .requires_env
            .iter()
            .filter(|e| !available.contains(*e))
            .collect();
        let (state, next) = if !missing.is_empty() {
            (
                "BLOCKED-BY-ENV".to_string(),
                format!(
                    "restore {}",
                    missing
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            )
        } else if entry.linked {
            ("LINKED".to_string(), "—".to_string())
        } else {
            (
                "MEM-UNLINKED".to_string(),
                "add cites: to a system".to_string(),
            )
        };
        rows.push(PlacementRow {
            path: entry.path,
            position: "MEMORY".to_string(),
            state,
            next,
        });
    }

    let mut balance: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
        *balance.entry(row.state.clone()).or_insert(0) += 1;
    }
    let pressure: usize = PRESSURE_ORDER
        .iter()
        .map(|s| balance.get(*s).copied().unwrap_or(0))
        .sum();
    let held = balance.get("BLOCKED-BY-ENV").copied().unwrap_or(0);
    let total = rows.len();
    let settled = total.saturating_sub(pressure + held);
    let gaps = roll_up_gaps(&docs, &available, &known);

    Ok(PlacementReport {
        command: "flow report placement".into(),
        total_files: total,
        rows,
        balance,
        pressure,
        held,
        settled,
        gaps,
    })
}

/// A doc's instantly-auditable STATE plus the action that advances it.
///
/// **The directory a doc physically sits in IS its state.** A doc parked in a `_state/<state>/`
/// pressure dir is classified by its home, which is authoritative over its frontmatter — the
/// state-machine invariant the kit's pressure dirs encode. Env-blocking is checked next, before any
/// status reading, because a doc whose canvas is absent is HELD, not judged.
fn budget_state(
    rec: &DocRecord,
    available: &std::collections::BTreeSet<String>,
) -> (String, String) {
    match rec.home.as_str() {
        "_state:regression" => {
            return (
                "REGRESSED".into(),
                "fix and re-verify on an available env".into(),
            )
        }
        "_state:blockers" => {
            return (
                "BLOCKED-BY-ENV".into(),
                "restore env (cluster-state.yaml)".into(),
            )
        }
        "_state:unverified" => return ("CLAIMED-ONLY".into(), "VERIFY (ci-investigator)".into()),
        "_state:needs-triage" => {
            return (
                "NEEDS-TRIAGE".into(),
                "classify: add status + place in graph".into(),
            )
        }
        _ => {}
    }
    let missing: Vec<&str> = rec
        .requires_env
        .iter()
        .filter(|e| !available.contains(*e))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return (
            "BLOCKED-BY-ENV".into(),
            format!("restore env: {} (cluster-state.yaml)", missing.join(",")),
        );
    }
    match rec.bucket {
        "NONE" => (
            "NEEDS-TRIAGE".into(),
            "classify: add status + place in graph".into(),
        ),
        "DEAD" => ("SUPERSEDED".into(), "distill → history, retire body".into()),
        "REGRESSED" => (
            "REGRESSED".into(),
            "fix and re-verify on an available env".into(),
        ),
        "CLAIMED" => ("CLAIMED-ONLY".into(), "VERIFY (ci-investigator)".into()),
        "LANDED" => {
            // CANONICAL / HISTORY are the settled destinations: an `accepted`/`landed` status there
            // is the steady state, not a deliverable awaiting CI. Only an ACTIVE-home
            // landed-without-evidence is the over-claim the verification gate exists for.
            if rec.home == "CANONICAL" || rec.home == "HISTORY" {
                (
                    "SETTLED".into(),
                    "settled canon/history — no verification queue".into(),
                )
            } else if rec.verified {
                ("VERIFIED-STABLE".into(), "retire-eligible → history".into())
            } else {
                ("CLAIMED-ONLY".into(), "VERIFY (ci-investigator)".into())
            }
        }
        "ACTIVE" => ("ACTIVE".into(), "in-flight".into()),
        "LIVING" => {
            if rec.home == "CANONICAL" {
                (
                    "SETTLED".into(),
                    "living canonical seed — no verification queue".into(),
                )
            } else {
                (
                    "UNKNOWN-STATUS".into(),
                    "living status outside canonical home — re-place or re-status".into(),
                )
            }
        }
        _ => (
            "UNKNOWN-STATUS".into(),
            "normalize the status string".into(),
        ),
    }
}

/// The status bucket for a raw status string: first whitespace token, lowercased, with `:` and `*`
/// stripped off both ends.
fn status_bucket(raw: &str) -> &'static str {
    let first = raw.trim().to_lowercase();
    let Some(word) = first.split_whitespace().next() else {
        return "NONE";
    };
    let word = word.trim_matches([':', '*']).trim();
    if word.is_empty() {
        return "NONE";
    }
    if LANDED_WORDS.contains(&word) {
        "LANDED"
    } else if CLAIMED_WORDS.contains(&word) {
        "CLAIMED"
    } else if REGRESSED_WORDS.contains(&word) {
        "REGRESSED"
    } else if DEAD_WORDS.contains(&word) {
        "DEAD"
    } else if ACTIVE_WORDS.contains(&word) || word == "active" {
        "ACTIVE"
    } else if LIVING_WORDS.contains(&word) {
        "LIVING"
    } else {
        "UNKNOWN"
    }
}

/// `**Status:** <value>` — the markdown status line, read only when the frontmatter has none.
fn markdown_status(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim_end().strip_prefix("**Status:**") {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Every doc across the surfaces plus the `_state/` pressure dirs. Non-recursive per home, matching
/// the kit: a surface's subdirectories are a different concern (the `held/` tree is sequestered
/// precisely by not being globbed).
fn scan_docs(root: &Path) -> Vec<DocRecord> {
    let mut homes: Vec<(String, PathBuf)> = SURFACES
        .iter()
        .map(|(home, rel)| (home.to_string(), root.join(rel)))
        .collect();
    let state_root = root.join(STATE_ROOT);
    if let Ok(entries) = std::fs::read_dir(&state_root) {
        let mut subs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subs.sort();
        for sub in subs {
            let name = sub
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            homes.push((format!("_state:{name}"), sub));
        }
    }

    let mut docs = Vec::new();
    for (home, dir) in homes {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
            .collect();
        files.sort();
        for file in files {
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if SKIP_NAMES.contains(&name.as_str()) {
                continue;
            }
            let Some(text) = read_lossy(&file) else {
                continue;
            };
            let fm = parse_frontmatter(&text);
            let raw = fm
                .get("status")
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .or_else(|| markdown_status(&text))
                .unwrap_or_default();
            docs.push(DocRecord {
                home: home.clone(),
                path: rel_string(root, &file),
                abs: file.clone(),
                bucket: status_bucket(&raw),
                verified: declares_value(&fm, "verified_by")
                    || declares_value(&fm, "landed_commit"),
                requires_env: requires_env_of(&fm),
            });
        }
    }
    docs
}

/// A `.claude/memory` entry and whether it is linked into a system.
struct MemoryEntry {
    path: String,
    linked: bool,
    requires_env: Vec<String>,
}

fn scan_memory(root: &Path) -> Vec<MemoryEntry> {
    let dir = root.join(MEMORY_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        if file.file_name().is_some_and(|n| n == "MEMORY.md") {
            continue;
        }
        let Some(text) = read_lossy(&file) else {
            continue;
        };
        let fm = parse_frontmatter(&text);
        let linked = declares_value(&fm, "cites");
        out.push(MemoryEntry {
            path: rel_string(root, &file),
            linked,
            requires_env: requires_env_of(&fm),
        });
    }
    out
}

/// Whether a frontmatter key carries an actual VALUE, in any of the three spellings a document may
/// use it in.
///
/// The inline-list arm is why this exists. The native frontmatter parser has no inline-list handling,
/// so `verified_by: []` arrives as the non-empty STRING `"[]"` — and a naive "is it non-empty"
/// reading of that turns an explicitly empty evidence list into evidence. That moved
/// `status: landed` documents from `CLAIMED-ONLY` to `VERIFIED-STABLE`, i.e. out of the pressure
/// queue and out of the over-claim count `delivery-gate.py` steers on: the one number this station
/// rewired. The kit parses the inline list and reads `[]` as falsy, and so does this, by normalizing
/// through the same list reader `requires_env` uses. `[ ]` and `[,]` are empty too — the check is on
/// the parsed items, never on the literal spelling.
pub fn declares_value(fm: &super::Frontmatter, key: &str) -> bool {
    if !fm.list(key).is_empty() {
        return true;
    }
    fm.get(key)
        .is_some_and(|scalar| !env_scope::parse_requires_env_scalar(scalar).is_empty())
}

/// Read a document the way every kit reader reads one: `errors="replace"`.
///
/// A non-UTF-8 byte in one document must not remove that document from the ledger. Dropping it
/// would silently change the BALANCE denominator — a file that exists, is placed somewhere, and is
/// in some state would simply not be counted, which is the opposite of "sums to total, nothing
/// hides". Only an unreadable FILE (permissions, a race, a directory) is skipped.
fn read_lossy(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// `requires_env` / `requires-env`, in either the block-list or the inline/scalar spelling.
pub fn requires_env_of(fm: &super::Frontmatter) -> Vec<String> {
    for key in ["requires_env", "requires-env"] {
        let list = fm.list(key);
        if !list.is_empty() {
            return env_scope::parse_requires_env_list(list);
        }
        if let Some(scalar) = fm.get(key) {
            let parsed = env_scope::parse_requires_env_scalar(scalar);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn rel_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The projection `super::stasis` reads
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// One document, reduced to the facts the coverage/stasis dimensions are ratios over.
///
/// A projection rather than a re-scan: `--coverage` and `--stasis` must agree with `--ledger` about
/// which documents exist and what state they are in, and two scans with two filters is how they
/// would come to disagree.
#[derive(Debug, Clone)]
pub struct SurfaceDoc {
    pub home: String,
    pub path: String,
    /// The status bucket, `NONE` when the doc declares no state.
    pub bucket: &'static str,
    /// How many `- [ ]`/`- [x]` stations the document's own bytes yield.
    pub stations: usize,
    /// Whether the doc carries any outbound link (`cites:` or `traces:`) — the native reading of
    /// "non-orphan". The kit also counted INBOUND links; a doc with neither is an orphan under
    /// both readings, and a doc with only inbound links is rarer than the cost of a second index.
    pub linked: bool,
    /// Whether at least one `traces:` entry is EXPLAINED — a path plus a separator plus prose.
    /// A bare path does not count; that is the link rule the kit enforces.
    pub explained_trace: bool,
    /// HISTORY only: whether the doc names a canonical seed, the bidirectional-link dimension.
    pub canonical_link: bool,
}

/// Homes whose documents are ACTIVE work — the denominator of most dimensions.
const ACTIVE_HOMES: [&str; 3] = ["ACTIVE:specs", "ACTIVE:plans", "ACTIVE:plans(legacy)"];

/// Frontmatter keys that carry a real anchor (`placement-audit.py::LINK_KEYS`). A doc that names
/// its lineage is not an orphan, which is what removes the false-orphan class the kit documents.
const LINK_KEYS: [&str; 15] = [
    "canonical",
    "distills",
    "cites",
    "verified_by",
    "informed-by",
    "informs",
    "supersedes",
    "superseded-by",
    "supersedes-or-conforms-to",
    "parent",
    "related",
    "spec",
    "source-spec",
    "roadmap",
    "traces",
];

/// Every surface document, `_state/` pressure dirs excluded (they are their own hard gate).
///
/// TWO PASSES, because `linked` is a graph property. The kit builds a `link_targets` set of every
/// basename any surface doc points at, then marks a doc non-orphan when it has an outbound link OR
/// appears in that set. A one-pass reading that only counted outbound links scores this repository's
/// `well_formed` dimension at 0.698 where the kit scores 0.927 — inbound-only docs are real and
/// numerous, and dropping them would have silently moved a ratcheted dimension down by 23 points.
pub fn surface_docs(root: &Path) -> Vec<SurfaceDoc> {
    let scanned: Vec<_> = scan_docs(root)
        .into_iter()
        .filter(|d| !d.home.starts_with("_state"))
        .collect();
    let basenames: std::collections::BTreeSet<String> = scanned
        .iter()
        .filter_map(|d| Path::new(&d.path).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    // (doc, own basename, outbound basenames, whether any raw link names a canonical seed)
    let mut parsed = Vec::with_capacity(scanned.len());
    let mut link_targets: std::collections::BTreeSet<String> = Default::default();
    for d in &scanned {
        let text = read_lossy(&d.abs).unwrap_or_default();
        let fm = parse_frontmatter(&text);
        let mut raw_links: Vec<String> = markdown_links(&text);
        for key in LINK_KEYS {
            if let Some(one) = fm.get(key) {
                if !one.trim().is_empty() {
                    raw_links.push(one.to_string());
                }
            }
            raw_links.extend(fm.list(key).iter().map(ToString::to_string));
        }
        let own = Path::new(&d.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        // Every `*.md` TOKEN inside each raw value, not only values that are themselves a path.
        // The kit parses frontmatter as full YAML, so `related: [a.md, b.md]` and a cite envelope's
        // `path: x.md` both arrive as real targets; this parser is a documented subset that hands
        // back the inline list as one scalar, and requiring the whole value to end in `.md` dropped
        // every one of them.
        let outbound: std::collections::BTreeSet<String> = raw_links
            .iter()
            .flat_map(|value| md_tokens(value))
            .filter_map(|x| Path::new(x.trim()).file_name().map(|n| n.to_owned()))
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        for target in &outbound {
            if basenames.contains(target) && target != &own {
                link_targets.insert(target.clone());
            }
        }
        // `"canonical" in fm` is a KEY-PRESENCE test in the kit, true for a scalar OR a list.
        // Asking only `fm.get()` (scalars) misses every doc that declares `canonical:` as a list
        // and moved this dimension from 0.755 to 0.396 in one edit — a reminder that the two
        // frontmatter accessors here answer different questions.
        let canonical_link = fm.get("canonical").is_some()
            || !fm.list("canonical").is_empty()
            || raw_links.iter().any(|x| x.contains("/architecture/"));
        let traces = fm.list("traces");
        let stations = gaps::decompose(
            &gaps::slug_for(Path::new(&d.path)),
            &text,
            d.requires_env.clone(),
        )
        .items
        .len();
        parsed.push((
            d,
            own,
            outbound,
            canonical_link,
            stations,
            traces
                .iter()
                .any(|t| [" — ", " -- ", " | "].iter().any(|sep| t.contains(sep))),
        ));
    }

    parsed
        .into_iter()
        .map(
            |(d, own, outbound, canonical_link, stations, explained_trace)| SurfaceDoc {
                home: d.home.clone(),
                path: d.path.clone(),
                bucket: d.bucket,
                stations,
                linked: !outbound.is_empty() || link_targets.contains(&own),
                explained_trace,
                canonical_link,
            },
        )
        .collect()
}

/// Every whitespace/punctuation-delimited `*.md` token inside one frontmatter value.
fn md_tokens(value: &str) -> Vec<String> {
    value
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '[' | ']' | '|' | '"' | '\'' | '`'))
        .filter(|t| t.ends_with(".md"))
        .map(ToString::to_string)
        .collect()
}

/// Every `](path.md…)` target in a markdown body — `placement-audit.py::MD_LINK_RE`.
fn markdown_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("](") {
        let after = &rest[open + 2..];
        let Some(close) = after.find(')') else { break };
        let target = &after[..close];
        if let Some(md) = target.split_whitespace().next() {
            if md.ends_with(".md") {
                out.push(md.to_string());
            } else if let Some(cut) = md.find(".md") {
                out.push(md[..cut + 3].to_string());
            }
        }
        rest = &after[close + 1..];
    }
    out
}

/// Surface documents in an ACTIVE home.
pub fn active_docs(root: &Path) -> Vec<SurfaceDoc> {
    surface_docs(root)
        .into_iter()
        .filter(|d| ACTIVE_HOMES.contains(&d.home.as_str()))
        .collect()
}

/// One `.claude/memory` entry and whether it cites a system.
#[derive(Debug, Clone)]
pub struct MemoryRow {
    pub path: String,
    pub linked: bool,
}

pub fn memory_entries(root: &Path) -> Vec<MemoryRow> {
    scan_memory(root)
        .into_iter()
        .map(|m| MemoryRow {
            path: m.path,
            linked: m.linked,
        })
        .collect()
}

/// The implementation budget, derived from the documents' own checkbox stations.
///
/// `known: false` when the corpus yielded nothing extractable — which is `unknown`, not `0 OPEN`.
/// The distinction is the whole point: a corpus nobody has decomposed and a corpus with no remaining
/// work render the same zero in the kit, and they are opposite facts.
fn roll_up_gaps(
    docs: &[DocRecord],
    available: &std::collections::BTreeSet<String>,
    known_caps: &std::collections::BTreeSet<String>,
) -> GapRollup {
    let mut rollup = GapRollup {
        known: false,
        docs_with_items: 0,
        docs_scanned: docs.len(),
        open: 0,
        claimed: 0,
        blocked: 0,
        per_doc: Vec::new(),
    };
    for doc in docs {
        let Some(text) = read_lossy(&doc.abs) else {
            continue;
        };
        let slug = gaps::slug_for(Path::new(&doc.path));
        let decomposition = gaps::decompose(&slug, &text, doc.requires_env.clone());
        if decomposition.items.is_empty() {
            continue;
        }
        rollup.known = true;
        let (mut open, mut claimed, mut blocked) = (0usize, 0usize, 0usize);
        for item in &decomposition.items {
            let resolved = env_scope::resolved_requires_env(
                &item.requires_env,
                &decomposition.doc_requires_env,
            );
            if env_scope::gap_blocked(&resolved, available, known_caps) {
                blocked += 1;
            } else if item.state == gaps::GapState::Open {
                open += 1;
            } else {
                claimed += 1;
            }
        }
        rollup.docs_with_items += 1;
        rollup.open += open;
        rollup.claimed += claimed;
        rollup.blocked += blocked;
        rollup.per_doc.push(GapDocRow {
            doc: doc.path.clone(),
            open,
            claimed,
            blocked,
        });
    }
    rollup
}

impl PlacementReport {
    /// The BALANCE section — every state, sorted by descending count then name, exactly as the kit
    /// sorts it. A `·` marks a resting state, a `▶` a queue.
    pub fn render_balance(&self) {
        println!(
            "BUDGET LEDGER — every file accounted (position + state)   total files: {}",
            self.total_files
        );
        println!("{}", "=".repeat(72));
        println!("  BALANCE (sums to total — nothing hides):");
        let mut states: Vec<(&String, &usize)> = self.balance.iter().collect();
        states.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (state, count) in states {
            let mark = if SETTLED_STATES.contains(&state.as_str()) {
                "·"
            } else {
                "▶"
            };
            // `checked_div` rather than a guarded divide: an empty ledger has no percentages to
            // render, and 0 is the only honest filler for a ratio with no denominator.
            let pct = (100 * count).checked_div(self.total_files).unwrap_or(0);
            println!("    {mark} {state:<16} {count:>4}  ({pct:>2}%)");
        }
        println!("  ----");
        println!(
            "    PRESSURE (needs action): {}   HELD (not pressure): {}   SETTLED: {}",
            self.pressure, self.held, self.settled
        );
    }

    /// The PRESSURE QUEUE — what to do, by state, in the kit's declared order.
    pub fn render_pressure_queue(&self) {
        println!("\n  PRESSURE QUEUE (what to do, by state):");
        for state in PRESSURE_ORDER {
            let items: Vec<&PlacementRow> = self.rows.iter().filter(|r| r.state == state).collect();
            if items.is_empty() {
                continue;
            }
            println!("    ▶ {state} ({}) — {}", items.len(), items[0].next);
            for row in items.iter().take(4) {
                println!("        {:<18} {}", row.position, row.path);
            }
            if items.len() > 4 {
                println!("        … +{} more", items.len() - 4);
            }
        }
    }

    /// The gap roll-up. An unmeasured corpus prints `unknown`, never a zero.
    pub fn render_gaps(&self) {
        if !self.gaps.known {
            println!(
                "\n  GAPS: unknown — no document in the {} scanned yielded an extractable station",
                self.gaps.docs_scanned
            );
            return;
        }
        let blocked = if self.gaps.blocked > 0 {
            format!(
                ", {} BLOCKED-BY-ENV (held — gap needs an unavailable cap)",
                self.gaps.blocked
            )
        } else {
            String::new()
        };
        println!(
            "\n  GAPS ({} docs with stations, {} scanned): {} OPEN to implement, \
             {} CLAIMED to verify (checked ≠ done){blocked}",
            self.gaps.docs_with_items, self.gaps.docs_scanned, self.gaps.open, self.gaps.claimed
        );
        for row in self.gaps.per_doc.iter().take(8) {
            let held = if row.blocked > 0 {
                format!(" / {:>2} held", row.blocked)
            } else {
                String::new()
            };
            println!(
                "      {:>3} open / {:>3} claimed{held}   {}",
                row.open, row.claimed, row.doc
            );
        }
        if self.gaps.per_doc.len() > 8 {
            println!("      … +{} more", self.gaps.per_doc.len() - 8);
        }
    }

    pub fn render_ledger(&self) {
        self.render_balance();
        self.render_pressure_queue();
        self.render_gaps();
    }
}

/// The CLI arm. `--focus` short-circuits to the focus baseline, which is a different reading of the
/// same substrate and prints on its own.
pub fn run(
    root: &Path,
    json: bool,
    options: &PlacementOptions,
    cluster_state_override: Option<&Path>,
) -> FlowResult<()> {
    if let Some(subject) = &options.focus {
        let baseline = scope::focus_baseline(root, cluster_state_override)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&baseline)?);
        } else if options.brief && subject.is_none() {
            print!("{}", baseline.render_brief());
        } else {
            print!("{}", baseline.render(subject.as_deref()));
        }
        return Ok(());
    }
    if options.coverage {
        let report = super::stasis::coverage(root)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            report.render();
        }
        return Ok(());
    }
    if options.stasis {
        let baselines = super::stasis::baselines(root)?;
        let report = super::stasis::stasis(root, &baselines)?;
        if options.fold {
            let appended = super::stasis::fold_baselines(root, &report)?;
            eprintln!(
                "stasis: ratchet baseline folded — {appended} new observation(s) on {}",
                super::stasis::DIMENSION_MEASURE
            );
        }
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            report.render();
        }
        return Ok(());
    }
    let report = placement(root, cluster_state_override)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if options.ledger {
        report.render_ledger();
    } else {
        report.render_balance();
    }
    Ok(())
}

/// Resolve a `--cluster-state PATH` override.
pub fn cluster_state_arg(root: &Path, value: Option<&str>) -> FlowResult<Option<PathBuf>> {
    let Some(value) = value else { return Ok(None) };
    let path = if Path::new(value).is_absolute() {
        PathBuf::from(value)
    } else {
        root.join(value)
    };
    if !path.is_file() {
        return Err(FlowError::InvalidArguments(format!(
            "--cluster-state names no file: {}",
            path.display()
        )));
    }
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_buckets_are_the_kits() {
        assert_eq!(status_bucket("landed"), "LANDED");
        assert_eq!(status_bucket("**Landed** on 2026-01-01"), "LANDED");
        assert_eq!(status_bucket("claimed-not-verified"), "CLAIMED");
        assert_eq!(status_bucket("regressed"), "REGRESSED");
        assert_eq!(status_bucket("superseded"), "DEAD");
        assert_eq!(status_bucket("proposed"), "ACTIVE");
        assert_eq!(status_bucket("active"), "ACTIVE");
        assert_eq!(status_bucket("living document"), "LIVING");
        assert_eq!(status_bucket(""), "NONE");
        assert_eq!(status_bucket("   "), "NONE");
        assert_eq!(status_bucket("whatever"), "UNKNOWN");
    }

    #[test]
    fn the_markdown_status_line_is_the_fallback_reader() {
        assert_eq!(
            markdown_status("# T\n\n**Status:** landed\n").as_deref(),
            Some("landed")
        );
        assert_eq!(markdown_status("# T\n\nno status\n"), None);
    }

    #[test]
    fn the_pressure_order_is_the_kits_queue_order() {
        assert_eq!(
            PRESSURE_ORDER,
            [
                "NEEDS-TRIAGE",
                "MEM-UNLINKED",
                "CLAIMED-ONLY",
                "REGRESSED",
                "SUPERSEDED",
                "UNKNOWN-STATUS"
            ]
        );
    }
}
