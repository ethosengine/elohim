//! `epr flow memory recall <op> --session <id>` — the native bounded-evidence recall executor.
//!
//! This is the native owner of what `.claude/scripts/memory-kit/recall-ceremony.py` and its three
//! libraries did: `recall_runtime.py` (session locking, budget counters, bounded reads with receipt
//! keys, method pinning), `recall_lenses.py` (projection receipts and the paired deterministic
//! footprint lens) and `recall_providers.py` (recipe-selected providers, deterministic local
//! traversal by default, MemPalace optional and honest about unknown ranking/freshness).
//!
//! **The privacy line is structural here, not a convention.** Receipts and continuations are
//! private records under `genesis/docs/architecture/private-thought-governed-fruit.md` §2: a
//! participant's reasoning trace is never fruit. So this executor
//!
//! - writes every receipt and continuation under `.eprfs/status/recall/<session>/`, a directory the
//!   root `.gitignore` excludes terminally (the directory itself is named, so no later `!` rung can
//!   re-include anything inside it without first re-including the directory);
//! - exposes only *what was read* (bounded excerpts with their own fingerprints) and *what was
//!   concluded* (findings, frontier, outcome) — never the deliberation that produced them;
//! - has **no verb that imports, contributes, projects or feeds back a receipt.** `contribute`,
//!   `project`, `feedback` and `graduate` remain the collective-memory verbs one directory up, and
//!   they take an authored request file. A receipt is not an admissible input to any of them, and
//!   [`refuse_private_import`] is the single gate that says so out loud.
//!
//! **Method pinning.** The algorithm artifact is `.epr-meta/elohim/algorithms/recall-contract.json`
//! — relocated under its EPRFS owner rather than sitting beside an executor. Every session and
//! every receipt pins that file's **raw CID** as `method`. A session whose stored method differs
//! from the contract's current CID is refused rather than silently continued: the counters would be
//! accounting for two different algorithms under one name.
//!
//! **Native projections are in-process.** The Python executor shelled out to `epr flow …` and
//! charged the bytes it read back. Here `concerns`, `context --section` and `walk` are function
//! calls, so `native_raw_bytes` is charged as the serialized size of the projection actually
//! consumed, and there is no `native_stderr_bytes` slot because there is no second process to have
//! a stderr. That is the one usage-key divergence from the Python view, and it is a divergence of
//! honesty rather than of shape.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use eprfs_core::BlobCid;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::super::{concerns, confine_under, context, context_sections, note, rel_to_root, walk};
use super::super::{FlowError, FlowResult};
use super::footprint;

mod receipts;
pub use receipts::{adopt_receipts, excerpt, refuse_private_import, save_receipt};
use receipts::{contained, load_receipt};

mod refusal;
use refusal::{accepted_flags, one_line, print_refusal, refusal_lines, remedy_for};
mod render;
use render::{clip, render, render_hits};
mod measure;
use measure::measure;

/// Where the algorithm artifact lives, relative to the repository root.
pub const CONTRACT_REL: &str = ".epr-meta/elohim/algorithms/recall-contract.json";

/// The private per-session store. Terminal in the ignore ladder — see the module doc.
pub const RECALL_DIR_REL: &str = ".eprfs/status/recall";

/// The kit's receipt store, relocated by `--adopt-receipts`.
pub const LEGACY_RECEIPTS_REL: &str = ".claude/memory-kit/recall-executions";

/// The declared foreign footprint lens the `measure` op invokes.
pub const BALANCE_LENS_REL: &str = "genesis/scripts/memory_balance.py";

/// The stage composition this executor implements. A contract declaring anything else is refused
/// rather than partially honoured — an executor that runs six of seven declared stages is a lie
/// about which algorithm produced the receipt.
const COMPOSITION: [&str; 7] = [
    "scope", "discover", "filter", "group", "select", "read", "judge",
];

/// Every budget the contract must declare, positive and finite.
const REQUIRED_LIMITS: [&str; 13] = [
    "source_files",
    "source_bytes",
    "search_results",
    "scan_bytes",
    "scan_files",
    "scan_entries",
    "metadata_bytes",
    "scan_seconds",
    "output_bytes",
    "provider_bytes",
    "provider_seconds",
    "native_raw_bytes",
    "native_timeout_seconds",
];

/// The three budgets that may be fractional seconds; every other one counts bytes or items.
const SECOND_LIMITS: [&str; 3] = ["scan_seconds", "provider_seconds", "native_timeout_seconds"];

/// The ceremony operations this executor answers.
///
/// The five collective-memory operations the Python entry also carried (`collective`,
/// `memory-project`, `memory-contribute`, `memory-feedback`, `memory-graduate`) are NOT here: they
/// are already native verbs (`epr flow memory collective|project|contribute|feedback|graduate`) and
/// re-exposing them through a second front door would give one act two addresses.
/// Subtrees that are INSIDE a declared source root and still refused: a sibling checkout is not
/// this repository's source, however reachable its path happens to be.
const FOREIGN_TREES: [&str; 1] = [".claude/worktrees"];

const OPERATIONS: [&str; 16] = [
    "open",
    "select",
    "read",
    "remember",
    "context",
    "recipe",
    "resume",
    "adopt",
    "search",
    "prepare",
    "reconcile",
    "finish",
    "history",
    "compare",
    "source",
    "measure",
];

fn refused(message: impl Into<String>) -> FlowError {
    FlowError::InvalidArguments(message.into())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Contract
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The algorithm artifact: its exact bytes, its parsed value, and the CID every receipt pins.
#[derive(Debug)]
pub struct Contract {
    pub raw: Vec<u8>,
    pub value: Value,
    pub path: PathBuf,
}

impl Contract {
    pub fn load(path: &Path) -> FlowResult<Self> {
        let raw = std::fs::read(path).map_err(|source| FlowError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let value: Value = serde_json::from_slice(&raw)?;
        let contract = Contract {
            raw,
            value,
            path: path.to_path_buf(),
        };
        contract.validate()?;
        Ok(contract)
    }

    /// The raw CID of the contract bytes — the `method` pin on every session and every receipt.
    pub fn method_cid(&self) -> String {
        BlobCid::compute_raw(&self.raw).to_string()
    }

    pub fn sha256(&self) -> String {
        hex(&Sha256::digest(&self.raw))
    }

    fn validate(&self) -> FlowResult<()> {
        if self.value.get("artifact_type").and_then(Value::as_str) != Some("algorithm-content") {
            return Err(refused(
                "unsupported algorithm composition; executor must implement the declared stages",
            ));
        }
        let composition: Vec<&str> = self
            .value
            .get("composition")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if composition != COMPOSITION {
            return Err(refused(
                "unsupported algorithm composition; executor must implement the declared stages",
            ));
        }
        for name in REQUIRED_LIMITS {
            let raw = self
                .value
                .pointer(&format!("/limits/{name}"))
                .ok_or_else(|| refused(format!("invalid positive budget: {name}")))?;
            let value = raw
                .as_f64()
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or_else(|| refused(format!("invalid positive budget: {name}")))?;
            if !SECOND_LIMITS.contains(&name) && !raw.is_i64() && !raw.is_u64() {
                return Err(refused(format!(
                    "byte/count budgets must be integers: {name}"
                )));
            }
            let _ = value;
        }
        let roots = self.source_roots();
        if roots.is_empty()
            || roots.iter().any(|p| {
                p.starts_with('/') || Path::new(p).components().any(|c| c.as_os_str() == "..")
            })
        {
            return Err(refused("source roots must be repository-relative"));
        }
        Ok(())
    }

    pub fn source_roots(&self) -> Vec<String> {
        self.value
            .get("source_roots")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn limit_usize(&self, name: &str) -> usize {
        self.value
            .pointer(&format!("/limits/{name}"))
            .and_then(Value::as_f64)
            .map(|v| v.max(0.0) as usize)
            .unwrap_or(0)
    }

    fn limit_secs(&self, name: &str) -> f64 {
        self.value
            .pointer(&format!("/limits/{name}"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    }

    fn recipe(&self) -> &Value {
        self.value.get("ceremony").unwrap_or(&Value::Null)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Execution — lock, pin and charge one session before evidence is emitted
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// One locked session: its continuation file, its accumulated counters, and the method it is
/// accounting for.
pub struct Execution {
    dir: PathBuf,
    path: PathBuf,
    file: File,
    pub state: Value,
    state_limit: usize,
}

/// A session label is a simple LOCAL name: it becomes a directory, so anything that could traverse
/// or hide is refused before a directory is created rather than after.
fn valid_session(session: &str) -> bool {
    let bytes = session.as_bytes();
    if bytes.is_empty() || bytes.len() > 96 {
        return false;
    }
    if !bytes[0].is_ascii_alphanumeric() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}

impl Execution {
    pub fn open(
        root: &Path,
        session: &str,
        need: &str,
        method: &str,
        executor: Option<&str>,
        state_limit: usize,
    ) -> FlowResult<Self> {
        if !valid_session(session) {
            return Err(refused(
                "session must be a simple local label (1–96 characters)",
            ));
        }
        if need.trim().is_empty() {
            return Err(refused(
                "every execution needs an explicit evidence question (--need)",
            ));
        }
        let dir = root.join(RECALL_DIR_REL).join(session);
        if !dir.starts_with(root.join(RECALL_DIR_REL)) {
            return Err(refused("session directory escapes repository"));
        }
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("continuation.json");
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(refused("session state must not be a symlink"));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(libc_nofollow())
            .mode(0o600)
            .open(&path)
            .map_err(|source| FlowError::Read {
                path: path.clone(),
                source,
            })?;
        rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(|_| refused("session is executing another packet; retry after it finishes"))?;
        let mut execution = Execution {
            dir,
            path,
            file,
            state: Value::Null,
            state_limit,
        };
        execution.hydrate(session, need, method, executor)?;
        Ok(execution)
    }

    /// Read, PIN and charge the session — in that order, and nothing is written before the pins hold.
    ///
    /// Two pins, both refuse-then-adopt. The contract's raw CID is the algorithm's identity; the
    /// executor's digest is the identity of the thing that ran it. A session that continued across
    /// either change would be accumulating one set of counters over two different methods and
    /// calling the sum a measurement of one. The refusal names the exact adopt command, and it
    /// happens BEFORE the attempt counter is incremented and before anything is saved, so the prior
    /// receipt — including its recorded digest — survives the refusal untouched and is still there
    /// for `adopt` to carry forward.
    fn hydrate(
        &mut self,
        session: &str,
        need: &str,
        method: &str,
        executor: Option<&str>,
    ) -> FlowResult<()> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut raw = Vec::new();
        (&self.file)
            .take(self.state_limit as u64 + 1)
            .read_to_end(&mut raw)?;
        if raw.len() > self.state_limit {
            return Err(refused(
                "continuation exceeds state budget; start a new bounded session retaining this receipt",
            ));
        }
        let text =
            String::from_utf8(raw).map_err(|_| refused("continuation is not valid UTF-8"))?;
        self.state = if text.trim().is_empty() {
            json!({"method": method, "attempts": 0, "totals": {}})
        } else {
            serde_json::from_str(&text)?
        };
        let adopt_pointer = format!(
            "epr flow memory recall adopt --from-session {session} --session <new-session>"
        );
        if self.state.get("method").and_then(Value::as_str) != Some(method) {
            return Err(refused(format!(
                "algorithm bytes changed; explicitly start a new session and retain prior receipt — {adopt_pointer}"
            )));
        }
        match (
            self.state.get("executor_digest").and_then(Value::as_str),
            executor,
        ) {
            (Some(stored), Some(running)) if stored != running => {
                let short = |digest: &str| digest[..digest.len().min(12)].to_string();
                return Err(refused(format!(
                    concat!(
                        "executor bytes changed since this session began ",
                        "(recorded {}…, running {}…); a distinct executor carries a distinct ",
                        "method pin, so silent continuation is refused. The prior receipt and ",
                        "its recorded digest are intact — {}"
                    ),
                    short(stored),
                    short(running),
                    adopt_pointer
                )));
            }
            // A session that has not yet recorded one adopts the running executor. That includes a
            // continuation relocated from the Python store, which has no such field: it is pinned on
            // first native use rather than being refused for a field it could not have carried.
            (None, Some(running)) => {
                self.state["executor_digest"] = json!(running);
            }
            _ => {}
        }
        let attempts = self.state["attempts"].as_u64().unwrap_or(0) + 1;
        self.state["attempts"] = json!(attempts);
        let unmetered = self.state["unmetered_attempts"].as_i64().unwrap_or(0) + 1;
        self.state["unmetered_attempts"] = json!(unmetered);
        self.state["last_need"] = json!(need);
        // Even an interrupted operation remains an attempted execution.
        self.save()
    }

    pub fn save(&mut self) -> FlowResult<()> {
        let encoded = serde_json::to_string(&self.state)?;
        if encoded.len() > self.state_limit {
            return Err(refused(
                "continuation exceeds state budget; prior receipt preserved, start a bounded continuation",
            ));
        }
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(encoded.as_bytes())?;
        self.file.flush()?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Fold one operation's usage into the session totals and stop counting it as unmetered.
    pub fn charge(&mut self, usage: &Value) -> FlowResult<Value> {
        let unmetered = self.state["unmetered_attempts"].as_i64().unwrap_or(0) - 1;
        self.state["unmetered_attempts"] = json!(unmetered);
        if let Some(map) = usage.as_object() {
            let totals = self.state["totals"]
                .as_object_mut()
                .ok_or_else(|| refused("continuation totals are not an object"))?;
            for (key, value) in map {
                if let Some(number) = numeric(value) {
                    let previous = totals.get(key).and_then(numeric).unwrap_or(0.0);
                    totals.insert(key.clone(), number_value(previous + number));
                }
            }
        }
        self.save()?;
        Ok(json!({
            "attempts": self.state["attempts"],
            "totals": self.state["totals"],
            "unmetered_attempts": self.state["unmetered_attempts"],
            "accounting_scope": "this executor/session only; outside-tool reads and total model tokens unknown",
        }))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn libc_nofollow() -> i32 {
    // O_NOFOLLOW. Named here rather than pulling `libc` in for one constant.
    0o400000
}

/// A JSON number that is not a bool, mirroring the Python `isinstance(value, (int, float)) and not
/// isinstance(value, bool)` guard the counters have always used.
fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Render a fold back into JSON, keeping integral values integral so `1` never becomes `1.0`.
fn number_value(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

fn add_usage(total: &mut Value, more: &Value) {
    let Some(source) = more.as_object() else {
        return;
    };
    let Some(target) = total.as_object_mut() else {
        return;
    };
    for (key, value) in source {
        if let Some(number) = numeric(value) {
            let previous = target.get(key).and_then(numeric).unwrap_or(0.0);
            target.insert(key.clone(), number_value(previous + number));
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Discovery — deterministic local traversal, the default provider
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Metadata-only bounded traversal: never a whole-repository filename list, never a relevance
/// ranking, and never a claim about the corpus outside the window it actually inspected.
pub fn discover(
    root: &Path,
    contract: &Contract,
    scope: &str,
    query: &str,
    tags: &[String],
    group_by: &str,
    name: &str,
) -> FlowResult<Value> {
    discover_scored(root, contract, scope, query, &[], tags, group_by, name)
}

/// The same bounded traversal, ranked by how many of `terms` a row's declared metadata carries.
///
/// One traversal serves both doors. `search` and `source --tag` pass no terms and get today's
/// deterministic window; the focused first screen passes the question's terms and gets the rows
/// that mention the most of them first. Ranking is over DECLARED metadata only — it is candidate
/// discovery, never authority, which is why every row still arrives with a `read` command rather
/// than an excerpt.
#[allow(clippy::too_many_arguments)]
pub fn discover_scored(
    root: &Path,
    contract: &Contract,
    scope: &str,
    query: &str,
    terms: &[String],
    tags: &[String],
    group_by: &str,
    name: &str,
) -> FlowResult<Value> {
    let base = contained(root, scope, &contract.source_roots())?;
    if !base.is_dir() {
        return Err(refused("discovery scope must be a directory"));
    }
    let pattern = glob::Pattern::new(name).map_err(|_| refused("--name is not a valid glob"))?;
    let excluded: BTreeSet<String> = contract
        .value
        .pointer("/discovery/exclude_directories")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let (scan_bytes_limit, scan_files_limit, scan_entries_limit) = (
        contract.limit_usize("scan_bytes"),
        contract.limit_usize("scan_files"),
        contract.limit_usize("scan_entries"),
    );
    let metadata_bytes = contract.limit_usize("metadata_bytes");
    // The per-file window term matching may read. It is NOT the frontmatter window: membership is
    // still established from `metadata_bytes` of byte-exact header, and a wider body window never
    // widens what qualifies a document. It is spent only when there is something to match — a pure
    // tag filter reads no more than it ever did, so tag discovery keeps its file coverage.
    let body_scan_bytes = contract
        .value
        .pointer("/limits/body_scan_bytes")
        .and_then(Value::as_u64)
        .map_or(metadata_bytes, |value| value as usize)
        .max(metadata_bytes);
    let matching = !terms.is_empty() || !query.is_empty();
    let per_file = if matching {
        body_scan_bytes
    } else {
        metadata_bytes
    };
    let scan_seconds = contract.limit_secs("scan_seconds");
    let search_results = contract.limit_usize("search_results");
    // With terms the window is not closed early AT ALL: every row the scan budget allows is
    // collected, ranked, and only then truncated to the declared result count. Stopping at a small
    // multiple of `search_results` made the ranking a ranking of whatever the traversal happened to
    // reach first — which, once body text became searchable and nearly every document matched
    // SOMETHING, was indistinguishable from no ranking. The traversal is still bounded by
    // scan_bytes/scan_files/scan_entries/scan_seconds exactly as before; only the RESULT window
    // moved, and the frontier still names the budget that stopped the walk.
    let collect_cap = if terms.is_empty() {
        search_results
    } else {
        scan_files_limit.max(search_results)
    };

    let began = Instant::now();
    let mut stack = vec![base];
    let mut candidates: Vec<Value> = Vec::new();
    let mut frontier: Vec<String> = Vec::new();
    // Candidates whose frontmatter would not parse. They are SKIPPED, COUNTED and NAMED rather
    // than aborting the traversal (2026-09-11 ruling): the honesty rule exists so a window never
    // hides what it could not read, and naming each one satisfies that, while stopping on the first
    // one made the whole screen useless because of a handful of documents nobody chose.
    let mut unreadable: Vec<String> = Vec::new();
    let mut budget_cut = false;
    // Eligible documents bigger than the window they were read with. A term past the window is
    // invisible, and invisible is not absent — so the count is reported rather than implied.
    let mut partially_scanned = 0usize;
    let (mut scan_bytes, mut scanned_files, mut scanned_entries) = (0usize, 0usize, 0usize);

    'outer: while let Some(directory) = stack.pop() {
        if !frontier.is_empty() {
            break;
        }
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                frontier.push(format!("metadata read failed: {}", error.kind()));
                break;
            }
        };
        for entry in entries.flatten() {
            scanned_entries += 1;
            if scanned_entries > scan_entries_limit
                || scanned_files >= scan_files_limit
                || scan_bytes >= scan_bytes_limit
                || began.elapsed().as_secs_f64() > scan_seconds
            {
                frontier.push(
                    "discovery budget exhausted; narrow the scope or continue with a named question"
                        .into(),
                );
                break 'outer;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let Ok(meta) = entry.path().symlink_metadata() else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                if !excluded.contains(&file_name) {
                    stack.push(entry.path());
                }
                continue;
            }
            if !pattern.matches(&file_name) || !file_name.ends_with(".md") || !meta.is_file() {
                continue;
            }
            scanned_files += 1;
            let budget = per_file.min(scan_bytes_limit.saturating_sub(scan_bytes));
            let mut data = Vec::new();
            let opened = File::open(entry.path())
                .and_then(|f| f.take(budget as u64).read_to_end(&mut data).map(|_| ()));
            if opened.is_err() {
                frontier.push("metadata read failed: OSError".into());
                break 'outer;
            }
            scan_bytes += data.len();
            let relative = rel_to_root(root, &entry.path());
            let truncated = meta.len() as usize > data.len();
            // A BOUNDED read ends wherever the budget says, which is very often mid-character. The
            // cut is an artifact of this reader, not a fault in the document, so the tail is
            // trimmed back to the last complete character rather than condemning the file: on
            // 2026-09-11 that mistake reported `.claude/skills/converge/SKILL.md` — 708 bytes of
            // perfectly valid frontmatter — as unparseable, because an em dash sat across byte
            // 8192. The frontmatter itself is still EXACT bytes: if the trim lands before the
            // closing delimiter the header simply cannot be proven and the row is skipped as
            // incomplete, which is true.
            // Membership is read from the METADATA window only, whatever the body window read.
            let head_len = metadata_bytes.min(data.len());
            let Some(head) = trim_to_character_boundary(&data[..head_len]) else {
                unreadable.push(relative);
                continue;
            };
            let Some(header) = frontmatter_header(&head, head.len(), meta.len() as usize) else {
                // A document that opens `---` and whose boundary this read could not prove is a
                // candidate we could not read. A file with no frontmatter at all is not a candidate
                // in the first place, and is passed over as it always was.
                if head.starts_with("---\n") {
                    // Only a read that actually hit its cap can be blamed on the budget; anything
                    // shorter simply has no closing delimiter.
                    budget_cut |= head_len >= metadata_bytes;
                    unreadable.push(relative);
                }
                continue;
            };
            if truncated {
                partially_scanned += 1;
            }
            // The body window is the whole read, trimmed the same way; the header slice above is a
            // prefix of it, so `header.len()` still indexes correctly.
            let text = if head_len == data.len() {
                head
            } else {
                match trim_to_character_boundary(&data) {
                    Some(text) => text,
                    None => head,
                }
            };
            let Ok(meta_value) = serde_yaml::from_str::<serde_yaml::Value>(&header[4..]) else {
                unreadable.push(relative);
                continue;
            };
            let Some(mapping) = meta_value.as_mapping() else {
                continue;
            };
            // Field lookup by iteration rather than `Mapping::get`, so the reader does not depend
            // on which `Index` impls a given serde_yaml minor happens to expose.
            let field = |key: &str| {
                mapping
                    .iter()
                    .find(|(k, _)| k.as_str() == Some(key))
                    .map(|(_, v)| v)
            };
            let actual_tags = match field("tags") {
                None => Some(Vec::new()),
                Some(serde_yaml::Value::Sequence(items)) => items
                    .iter()
                    .map(|item| item.as_str().map(str::to_string))
                    .collect::<Option<Vec<String>>>(),
                Some(_) => None,
            };
            // A `tags:` field that is not a list of strings is not category membership; the row is
            // skipped rather than coerced, exactly as the oracle skips it.
            let Some(actual_tags) = actual_tags else {
                continue;
            };
            let title = field("title")
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| file_name.clone());
            let description = field("description")
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            // The bytes after the closing delimiter were ALREADY READ under `metadata_bytes`.
            // Matching them costs no second read and no wider budget, and it is where an agent's
            // own words actually live: a fresh reader asking "re-mine", "marker", "stamp" on
            // 2026-09-11 got zero candidates over a corpus whose bodies say all three, because
            // discovery only ever looked at declared metadata.
            let body_prefix = text
                .get(header.len()..)
                .and_then(|rest| rest.split_once('\n'))
                .map(|(_, body)| body)
                .unwrap_or_default();
            let declared = [
                relative.as_str(),
                title.as_str(),
                description.as_str(),
                &actual_tags.join(" "),
            ]
            .join(" ")
            .to_lowercase();
            let body = body_prefix.to_lowercase();
            if !tags.iter().all(|t| actual_tags.contains(t))
                || (!query.is_empty() && {
                    let needle = query.to_lowercase();
                    !declared.contains(&needle) && !body.contains(&needle)
                })
            {
                continue;
            }
            // Where a term was found is evidence about how strong the hit is, so it is reported
            // rather than folded into one number. A declared hit outranks a body-only hit; both
            // remain candidate discovery, never authority.
            let (mut declared_hits, mut body_only_hits) = (0usize, 0usize);
            let mut kinds: BTreeSet<&str> = BTreeSet::new();
            let mut matched_terms: Map<String, Value> = Map::new();
            for term in terms {
                let needle = term.to_lowercase();
                let in_path = relative.to_lowercase().contains(&needle);
                let in_title = title.to_lowercase().contains(&needle);
                let in_description = description.to_lowercase().contains(&needle);
                let in_tag = actual_tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&needle));
                let in_body = body.contains(&needle);
                if in_path {
                    kinds.insert("path");
                }
                if in_title {
                    kinds.insert("title");
                }
                if in_description {
                    kinds.insert("description");
                }
                if in_tag {
                    kinds.insert("tag");
                }
                if in_body {
                    kinds.insert("body");
                }
                // How OFTEN, not merely whether: a document that names the concern once in
                // passing and one that is about it are not equal evidence, and the occurrences are
                // already inside the bytes this read covered.
                let occurrences = declared.matches(&needle).count() + body.matches(&needle).count();
                if in_path || in_title || in_description || in_tag {
                    declared_hits += 1;
                    matched_terms.insert(
                        needle,
                        json!({"found_in": "declared", "occurrences": occurrences}),
                    );
                } else if in_body {
                    body_only_hits += 1;
                    matched_terms.insert(
                        needle,
                        json!({"found_in": "body", "occurrences": occurrences}),
                    );
                }
            }
            if !terms.is_empty() && declared_hits + body_only_hits == 0 {
                continue;
            }
            candidates.push(json!({
                "path": relative,
                "title": title,
                "tags": actual_tags,
                "term_hits": declared_hits + body_only_hits,
                "declared_hits": declared_hits,
                "match": kinds.iter().collect::<Vec<_>>(),
                "matched_terms": matched_terms,
                "match_scope": "declared frontmatter and the bounded body prefix this read already \
                                covered; a body hit beyond the metadata budget is not visible here",
                "metadata_fingerprint": hex(&Sha256::digest(header.as_bytes())),
            }));
            if candidates.len() >= collect_cap {
                frontier.push("result window reached; remaining corpus not inspected".into());
                break 'outer;
            }
        }
    }

    // The account of what was skipped, said once and never silently: a named list bounded the same
    // way every other list in this view is, plus a count on the frontier so a reader scanning only
    // the unresolved lines still learns that candidates went unread.
    let mut omissions: Vec<Value> = Vec::new();
    if partially_scanned > 0 {
        omissions.push(json!(format!(
            "{partially_scanned} candidate(s) scanned to the body-scan window only"
        )));
    }
    if !unreadable.is_empty() {
        unreadable.sort();
        let shown = unreadable.len().min(8);
        let mut named = unreadable[..shown].join(", ");
        if unreadable.len() > shown {
            named.push_str(&format!(", +{} more", unreadable.len() - shown));
        }
        omissions.push(json!(format!(
            "{} candidate(s) unreadable (frontmatter did not parse) and were skipped: {named}",
            unreadable.len()
        )));
        frontier.push(format!("unreadable candidates: {}", unreadable.len()));
        if budget_cut {
            frontier.push(
                "some candidate frontmatter exceeded the metadata budget; raise                  limits.metadata_bytes to read it"
                    .into(),
            );
        }
    }

    candidates.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    if !terms.is_empty() {
        // Two rules, and the second is what makes the first useful.
        //
        // A declared hit outranks a body hit ON THE SAME TERM — weighted per term (declared ×2)
        // rather than as a lexicographic override, so one hit on a word every document shares can
        // never outrank several hits on the words that name the concern.
        //
        // And a term is worth what it distinguishes. Counting matched terms equally made
        // `commands` and `index` — carried by forty of forty-four skills — worth as much as
        // `mempalace` and `re-mine`, carried by two, which is how the document that answered the
        // reader's question came fifth with the window already wide enough to hold the answer. The
        // weight is the term's rarity IN THE SCANNED WINDOW, so it is derived from what this call
        // actually read and needs no corpus statistics kept anywhere.
        let total = candidates.len().max(1) as f64;
        let mut frequency: BTreeMap<String, usize> = BTreeMap::new();
        for row in &candidates {
            for term in row["matched_terms"].as_object().into_iter().flatten() {
                *frequency.entry(term.0.clone()).or_insert(0) += 1;
            }
        }
        let score = |row: &Value| -> f64 {
            row["matched_terms"]
                .as_object()
                .map(|matched| {
                    matched
                        .iter()
                        .map(|(term, hit)| {
                            let occurrences =
                                hit["occurrences"].as_u64().unwrap_or(1).max(1) as f64;
                            // Sublinear in the count: the tenth mention says less than the second.
                            let weight = 1.0 + occurrences.ln();
                            let shared = *frequency.get(term).unwrap_or(&1) as f64;
                            // A term nearly every candidate carries distinguishes nothing. The
                            // floor keeps it contributing a little rather than nothing, so a
                            // reader's ordinary words are not simply discarded.
                            let rarity = (total / shared).ln().max(0.01);
                            let placement = if hit["found_in"] == "declared" {
                                2.0
                            } else {
                                1.0
                            };
                            weight * rarity * placement
                        })
                        .sum()
                })
                .unwrap_or(0.0)
        };
        candidates.sort_by(|a, b| {
            score(b)
                .partial_cmp(&score(a))
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    b["declared_hits"]
                        .as_u64()
                        .cmp(&a["declared_hits"].as_u64())
                })
                .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
        });
        candidates.truncate(search_results);
    }
    let mut groups: Map<String, Value> = Map::new();
    for row in &candidates {
        let keys: Vec<String> = if group_by == "tag" {
            row["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            let path = row["path"].as_str().unwrap_or_default();
            vec![Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| ".".into())]
        };
        for key in keys {
            let previous = groups.get(&key).and_then(Value::as_u64).unwrap_or(0);
            groups.insert(key, json!(previous + 1));
        }
    }
    Ok(json!({
        "candidates": candidates,
        "groups": groups,
        "omissions": omissions,
        "unreadable_candidates": unreadable.len(),
        "partially_scanned_candidates": partially_scanned,
        "aggregation_scope": "returned candidates only",
        "selection": if terms.is_empty() {
            "bounded filesystem traversal; sorted returned window, no relevance ranking"
        } else {
            "bounded filesystem traversal ranked by term overlap over declared metadata and the bounded body window already read, each term weighted by how often it occurs and how rare it is in that window, a declared hit worth twice a body hit; candidate discovery, not authority"
        },
        "usage": {"scan_bytes": scan_bytes, "scanned_files": scanned_files,
                  "scanned_entries": scanned_entries, "search_queries": 1},
        "unresolved": frontier,
        "next": "Select a path and explicit --source/--lines; membership does not establish authority.",
    }))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The focused door — the area's habits, its last delta, and the competing sources
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Where the habit register is projected. Read, never written, by this executor.
pub const HABITS_REL: &str = "genesis/manifests/habits.yaml";

/// Words that carry no area, so they never select a habit or a source.
const STOPWORDS: [&str; 40] = [
    "about", "after", "again", "against", "because", "before", "being", "between", "could", "does",
    "doing", "down", "from", "does", "have", "here", "how", "into", "just", "like", "make", "more",
    "most", "much", "must", "only", "other", "over", "same", "should", "some", "such", "than",
    "that", "them", "then", "there", "this", "were", "what",
];

/// The question's distinctive terms: what an area match is made of.
fn question_terms(need: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for raw in need.split(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
    }) {
        let term = raw
            .trim_matches(|c| c == '.' || c == '/')
            .to_ascii_lowercase();
        if term.len() < 4 || STOPWORDS.contains(&term.as_str()) || seen.contains(&term) {
            continue;
        }
        seen.push(term);
        if seen.len() >= 12 {
            break;
        }
    }
    seen
}

/// One habit as the first screen needs it: what it promises, whether it holds, and what moved last.
fn habit_row(id: &str, status: &str, invariant: &str, evidence: &str, score: usize) -> Value {
    // The FIRST line of the ledger is the newest entry — the atoms are written newest-first — so
    // "what moved last" is one line, never the whole ledger.
    let delta = evidence
        .split("\n\n")
        .map(one_line)
        .find(|paragraph| !paragraph.is_empty())
        .map(|paragraph| clip(&paragraph, 240))
        .unwrap_or_default();
    json!({
        "id": id,
        "status": status,
        "delta": delta,
        "invariant": truncate(invariant, 400),
        "match_score": score,
    })
}

/// The habits whose id or invariant words the question's terms touch.
///
/// The register is a GENERATED projection of the habit atoms, read here by a bounded line scan
/// rather than a whole-document YAML load: it is 400 KB of evidence ledgers, and the first screen
/// needs four fields per habit. Its bytes are charged under their own declared budget, because
/// they are neither source evidence the reader quoted nor a native projection.
fn matching_habits(
    root: &Path,
    contract: &Contract,
    terms: &[String],
    usage: &mut Value,
) -> FlowResult<(Vec<Value>, Vec<String>)> {
    let mut unresolved: Vec<String> = Vec::new();
    let path = match contained(root, HABITS_REL, &contract.source_roots()) {
        Ok(path) => path,
        Err(_) => return Ok((Vec::new(), unresolved)),
    };
    if !path.is_file() {
        return Ok((Vec::new(), unresolved));
    }
    let budget = contract
        .value
        .pointer("/limits/habit_register_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let mut data = Vec::new();
    File::open(&path)
        .and_then(|file| {
            file.take(budget as u64 + 1)
                .read_to_end(&mut data)
                .map(|_| ())
        })
        .map_err(|source| FlowError::Read { path, source })?;
    if data.len() > budget {
        data.truncate(budget);
        unresolved.push(
            "habit register exceeds its declared budget; only the leading rows were inspected"
                .into(),
        );
    }
    add_usage(usage, &json!({"habit_register_bytes": data.len()}));
    let text = String::from_utf8_lossy(&data).into_owned();

    let (mut id, mut status, mut invariant, mut evidence) =
        (String::new(), String::new(), String::new(), String::new());
    let mut folding: Option<&'static str> = None;
    let mut rows: Vec<Value> = Vec::new();
    let mut flush = |id: &str, status: &str, invariant: &str, evidence: &str| {
        if id.is_empty() {
            return;
        }
        let lowered = id.to_ascii_lowercase();
        let parts: Vec<&str> = lowered.split('-').collect();
        let in_id = terms
            .iter()
            .filter(|term| lowered.contains(term.as_str()) || parts.contains(&term.as_str()))
            .count();
        let in_invariant = terms
            .iter()
            .filter(|term| invariant.to_ascii_lowercase().contains(term.as_str()))
            .count();
        let score = in_id * 3 + in_invariant;
        if score >= 3 {
            rows.push(habit_row(id, status, invariant, evidence, score));
        }
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("  - id: ") {
            flush(&id, &status, &invariant, &evidence);
            id = rest.trim().to_string();
            status.clear();
            invariant.clear();
            evidence.clear();
            folding = None;
            continue;
        }
        if folding.is_some() && line.trim().is_empty() {
            let target = match folding {
                Some("invariant") => &mut invariant,
                _ => &mut evidence,
            };
            target.push_str("\n\n");
            continue;
        }
        if let Some(rest) = line.strip_prefix("    ") {
            if !rest.starts_with(' ') && !rest.starts_with('-') {
                folding = None;
                if let Some(value) = rest.strip_prefix("status:") {
                    status = value.trim().to_string();
                } else if rest.starts_with("invariant:") {
                    folding = Some("invariant");
                } else if rest.starts_with("evidence:") {
                    folding = Some("evidence");
                }
                continue;
            }
            // The register folds these scalars with `>`, so a wrapped line is a continuation and
            // a BLANK line is the paragraph break. Reassembling that way is what makes "the first
            // line of the newest delta" a sentence rather than the first 100 columns of one.
            let target = match folding {
                Some("invariant") => &mut invariant,
                Some("evidence") => &mut evidence,
                _ => continue,
            };
            if rest.trim().is_empty() {
                target.push_str("\n\n");
            } else {
                target.push_str(rest.trim());
                target.push(' ');
            }
        }
    }
    flush(&id, &status, &invariant, &evidence);
    rows.sort_by(|a, b| b["match_score"].as_u64().cmp(&a["match_score"].as_u64()));
    rows.truncate(3);
    Ok((rows, unresolved))
}

/// The directory this question is about, or `None` when it is about the whole tree.
fn focus_area(root: &Path, contract: &Contract, scope: &str, terms: &[String]) -> Option<String> {
    let roots = contract.source_roots();
    let named = |candidate: &str| {
        contained(root, candidate, &roots)
            .ok()
            .filter(|path| path.is_dir())
            .map(|_| candidate.trim_end_matches('/').to_string())
    };
    if scope != "." {
        if let Some(area) = named(scope) {
            return Some(area);
        }
    }
    terms
        .iter()
        .filter(|term| term.contains('/'))
        .find_map(|term| named(term))
}

/// The first screen a focused question gets: the area's habits and the sources competing to answer
/// it, before any stale-edge group.
///
/// Whole-scope opens get `None` — the convergence question is answered by the grouped stale-edge
/// view, and putting a ranked source list in front of it would answer a question nobody asked.
fn first_screen(
    args: &Args,
    contract: &Contract,
    state: &Value,
    usage: &mut Value,
) -> FlowResult<Option<Value>> {
    if !args.need_explicit {
        return Ok(None);
    }
    let terms = question_terms(args.need.trim());
    if terms.is_empty() {
        return Ok(None);
    }
    let scope = state["scope"].as_str().unwrap_or(".");
    let (habits, mut unresolved) = matching_habits(&args.root, contract, &terms, usage)?;
    let area = focus_area(&args.root, contract, scope, &terms);
    if area.is_none() && habits.is_empty() {
        return Ok(None);
    }
    let mut candidates: Vec<Value> = Vec::new();
    let mut omissions: Vec<String> = Vec::new();
    let mut ranking = Value::Null;
    if let Some(area) = &area {
        // A term that names the area itself matches every file under it, so it ranks nothing. The
        // discriminating terms are the ones the area does NOT already carry.
        let lowered = area.to_ascii_lowercase();
        let ranking_terms: Vec<String> = terms
            .iter()
            .filter(|term| !term.contains('/') && !lowered.contains(term.as_str()))
            .cloned()
            .collect();
        let mut found = discover_scored(
            &args.root,
            contract,
            area,
            "",
            &ranking_terms,
            &[],
            "directory",
            "*.md",
        )?;
        let charged = found["usage"].take();
        add_usage(usage, &charged);
        candidates = found["candidates"].as_array().cloned().unwrap_or_default();
        // Each candidate is located, not merely named: the section its terms land in, and a read
        // range bounded to one excerpt. The outline scan is charged to the scan counters.
        for candidate in candidates.iter_mut() {
            let Some(path) = candidate["path"].as_str().map(str::to_string) else {
                continue;
            };
            if let Some(section) = best_section(args, contract, &path, &terms) {
                add_usage(usage, &section["usage"]);
                candidate["best_section"] = json!({
                    "title": section["title"],
                    "lines": section["read_lines"],
                    "hits": section["hits"],
                    "window_complete": section["window_complete"],
                });
            }
        }
        ranking = found["selection"].take();
        for message in found["omissions"].as_array().cloned().unwrap_or_default() {
            omissions.push(message.as_str().unwrap_or_default().to_string());
        }
        for message in found["unresolved"].as_array().cloned().unwrap_or_default() {
            unresolved.push(message.as_str().unwrap_or_default().to_string());
        }
    } else {
        unresolved.push(
            "no directory scope in this question; name --scope <dir> for ranked candidate sources"
                .into(),
        );
    }
    Ok(Some(json!({
        "area": area.clone().unwrap_or_else(|| scope.to_string()),
        "terms": terms,
        "habits": habits,
        "candidates": candidates,
        "provider": "local",
        "ranking": ranking,
        "authority": "Candidate discovery over declared metadata. Membership and rank establish \
                      neither authority nor currency; read the passage.",
        "omissions": omissions,
        "unresolved": unresolved,
    })))
}

/// Decode a bounded read, discarding a character the budget cut in half.
///
/// A byte budget ends wherever it ends, very often mid-character; that is a property of the reader,
/// not a fault in the document. `None` means not even the first character survived.
fn trim_to_character_boundary(data: &[u8]) -> Option<String> {
    match std::str::from_utf8(data) {
        Ok(text) => Some(text.to_string()),
        Err(error) => match error.valid_up_to() {
            0 => None,
            boundary => Some(String::from_utf8_lossy(&data[..boundary]).into_owned()),
        },
    }
}

/// The complete `---` fenced header, or `None` when the budget could not prove one is complete.
///
/// The subtle case is the last one: a delimiter that lands exactly at the end of the buffer with no
/// trailing newline may be a `---` prefix the budget cut in half. When more file remains unread,
/// that is not a proven document boundary, and treating it as one would let a truncated read
/// establish category membership.
fn frontmatter_header(text: &str, read_bytes: usize, file_bytes: usize) -> Option<String> {
    if !text.starts_with("---\n") {
        return None;
    }
    let body = &text[4..];
    let (start, end) = find_close(body)?;
    if end == body.len() && !text.ends_with('\n') && read_bytes < file_bytes {
        return None;
    }
    Some(text[..4 + start].to_string())
}

/// Locate a `^---[ \t]*\r?$` line in `body`, returning its (start, end) byte offsets.
fn find_close(body: &str) -> Option<(usize, usize)> {
    let mut position = 0usize;
    loop {
        let line_end = body[position..]
            .find('\n')
            .map(|i| position + i)
            .unwrap_or(body.len());
        let line = &body[position..line_end];
        if let Some(rest) = line.strip_prefix("---") {
            let rest = rest.strip_suffix('\r').unwrap_or(rest);
            if rest.chars().all(|c| c == ' ' || c == '\t') {
                return Some((position, line_end));
            }
        }
        if line_end >= body.len() {
            return None;
        }
        position = line_end + 1;
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Bounded subprocess — the only shape in which a foreign provider or lens may speak
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub struct ProcessOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
    pub error: Option<String>,
}

/// Bound bytes and seconds before buffering foreign output, including stderr; kill on excess.
pub fn bounded_process(
    program: &str,
    args: &[String],
    output_limit: usize,
    seconds: f64,
) -> std::io::Result<ProcessOutcome> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let cap = output_limit + 1;
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let out_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = out.by_ref().take(cap as u64).read_to_end(&mut buffer);
        buffer
    });
    let err_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = err.by_ref().take(cap as u64).read_to_end(&mut buffer);
        buffer
    });
    let began = Instant::now();
    let mut reason = None;
    let mut status = None;
    loop {
        match child.try_wait()? {
            Some(exit) => {
                status = exit.code();
                break;
            }
            None => {
                if began.elapsed().as_secs_f64() >= seconds {
                    reason = Some("provider timed out".to_string());
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    if reason.is_none() && stdout.len() + stderr.len() > output_limit {
        reason = Some("provider output exceeds budget; results withheld".into());
    }
    Ok(ProcessOutcome {
        stdout,
        stderr,
        status,
        error: reason,
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Providers — recipe-selected, never a source of authority
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Retrieve candidates through a provider the pinned recipe declares.
///
/// The default is the deterministic local traversal, which knows exactly what it inspected.
/// MemPalace is optional and reports its unknowns as unknowns: the declaration carries `ranking`,
/// `version` and `freshness` as nulls, and this function copies that declaration into the result
/// rather than papering over it. A candidate is a route to verify, never an answer.
#[allow(clippy::too_many_arguments)]
pub fn retrieve(
    root: &Path,
    contract: &Contract,
    provider: &str,
    query: &str,
    scope: &str,
    name: &str,
    tags: &[String],
) -> FlowResult<Value> {
    let declaration = contract
        .value
        .pointer(&format!("/ceremony/providers/{provider}"))
        .cloned()
        .filter(|v| !v.is_null())
        .ok_or_else(|| refused("provider is not declared by the pinned recipe"))?;
    let kind = declaration
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut result = match kind {
        // EXACT tag membership, and only the deterministic local traversal can honour it: a tag is
        // read out of a document's own frontmatter, which a semantic provider never sees. A tag
        // filter that silently did nothing on another provider would be a claim about a corpus
        // nobody filtered.
        "local" => discover(root, contract, scope, query, tags, "directory", name)?,
        "mempalace" => {
            let palace = root.join(".mempalace/palace");
            let args = vec![
                "--palace".to_string(),
                palace.to_string_lossy().to_string(),
                "search".to_string(),
                query.to_string(),
                "--results".to_string(),
                contract.limit_usize("search_results").to_string(),
            ];
            process_result("mempalace", &args, contract)
        }
        "fixture" => {
            // An authored local fixture demonstrates interchange, not live provider fitness.
            let source = declaration
                .get("source")
                .and_then(Value::as_str)
                .ok_or_else(|| refused("fixture provider declares no source"))?;
            let path = contained(root, source, &contract.source_roots())?;
            let limit = contract.limit_usize("provider_bytes");
            let mut data = Vec::new();
            File::open(&path)
                .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut data).map(|_| ()))
                .map_err(|source| FlowError::Read { path, source })?;
            let over = data.len() > limit;
            json!({
                "candidate_text": if over { String::new() } else { String::from_utf8_lossy(&data).to_string() },
                "usage": {"provider_bytes": data.len(), "search_queries": 1},
                "unresolved": if over { json!(["provider output exceeds budget"]) } else { json!([]) },
                "fitness": "local fixture only; no live provider fitness established",
            })
        }
        _ => return Err(refused("unsupported declared provider kind")),
    };
    if !tags.is_empty() && kind != "local" {
        return Err(refused(format!(
            "--tag is exact frontmatter membership and only the local provider reads frontmatter; \
             `{provider}` is a `{kind}` provider and would ignore it"
        )));
    }
    result["tags"] = json!(tags);
    result["provider"] = json!(provider);
    result["query"] = json!(query);
    result["declaration"] = declaration;
    result["authority"] = json!(
        "Unverified candidates; verify source evidence. Ranking/version/freshness unknown unless explicitly disclosed by the provider."
    );
    Ok(result)
}

fn process_result(program: &str, args: &[String], contract: &Contract) -> Value {
    let outcome = match bounded_process(
        program,
        args,
        contract.limit_usize("provider_bytes"),
        contract.limit_secs("provider_seconds"),
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            return json!({
                "unresolved": [format!("provider unavailable: {error}")],
                "usage": {"search_queries": 1},
                "candidate_text": "",
            })
        }
    };
    let failure = outcome
        .error
        .clone()
        .or_else(|| (outcome.status != Some(0)).then(|| "provider failed".to_string()));
    json!({
        "usage": {"search_queries": 1, "provider_bytes": outcome.stdout.len() + outcome.stderr.len()},
        "unresolved": failure.clone().map(|f| vec![f]).unwrap_or_default(),
        "candidate_text": if failure.is_some() { String::new() }
                          else { String::from_utf8_lossy(&outcome.stdout).to_string() },
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Arguments
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Everything one recall invocation may be told.
#[derive(Debug, Clone)]
pub struct Args {
    pub root: PathBuf,
    pub contract_path: PathBuf,
    pub session: String,
    pub need: String,
    /// Whether `--need` was actually typed, as opposed to the standing default below.
    pub need_explicit: bool,
    pub operation: String,
    pub scope: Option<String>,
    pub intent: Option<String>,
    pub path: Option<String>,
    pub lines: Option<String>,
    pub finding: Option<String>,
    pub question: Option<String>,
    pub next_action: Option<String>,
    pub from_session: Option<String>,
    pub provider: Option<String>,
    pub query: Option<String>,
    pub outcome: Option<String>,
    pub section: Option<String>,
    pub context_pin: Option<String>,
    pub search_scope: String,
    pub name: String,
    pub evidence: Vec<String>,
    pub offset: usize,
    pub limit: usize,
    pub edge: Option<usize>,
    pub evidence_offset: usize,
    pub classification: String,
    pub kind: String,
    pub json: bool,
    pub actor_session: Option<String>,
    pub measure_scope: Vec<String>,
    pub phase: String,
    /// An explicit external footprint-lens override. Absent means the native footprint sampler runs.
    pub lens: Option<PathBuf>,
    /// Exact metadata tags a discovery must carry, repeatable. Every named tag must be present.
    pub tags: Vec<String>,
    /// Whether `--root`/`--contract` were named, so linked next actions repeat only real overrides.
    pub root_explicit: bool,
    pub contract_explicit: bool,
}

impl Args {
    fn new(root: PathBuf) -> Self {
        let contract_path = root.join(CONTRACT_REL);
        Args {
            root,
            contract_path,
            session: String::new(),
            need: "Orient and choose the next justified reconciliation action".into(),
            need_explicit: false,
            operation: String::new(),
            scope: None,
            intent: None,
            path: None,
            lines: None,
            finding: None,
            question: None,
            next_action: None,
            from_session: None,
            provider: None,
            query: None,
            outcome: None,
            section: None,
            context_pin: None,
            search_scope: "genesis".into(),
            name: "*.md".into(),
            evidence: Vec::new(),
            offset: 0,
            limit: 12,
            edge: None,
            evidence_offset: 0,
            classification: "unreviewed".into(),
            kind: "observation".into(),
            json: false,
            actor_session: None,
            measure_scope: Vec::new(),
            phase: "baseline".into(),
            lens: None,
            tags: Vec::new(),
            root_explicit: false,
            contract_explicit: false,
        }
    }
}

/// The argv of a linked next action — a real command, not a description of one.
fn command(args: &Args, operation: &str, options: &[(&str, Value)]) -> Vec<String> {
    let mut argv = vec![
        "epr".into(),
        "flow".into(),
        "memory".into(),
        "recall".into(),
        operation.to_string(),
        "--session".into(),
        args.session.clone(),
    ];
    if args.root_explicit {
        argv.push("--root".into());
        argv.push(args.root.to_string_lossy().to_string());
    }
    if args.contract_explicit {
        argv.push("--contract".into());
        argv.push(args.contract_path.to_string_lossy().to_string());
    }
    if let Some(actor) = &args.actor_session {
        argv.push("--actor-session".into());
        argv.push(actor.clone());
    }
    for (key, value) in options {
        let flag = format!("--{}", key.replace('_', "-"));
        match value {
            Value::Null => continue,
            Value::Array(items) => {
                for item in items {
                    argv.push(flag.clone());
                    argv.push(scalar(item));
                }
            }
            other => {
                argv.push(flag);
                argv.push(scalar(other));
            }
        }
    }
    argv
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|word| {
            if !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"@%+=:,./-_".contains(&b))
            {
                word.clone()
            } else {
                format!("'{}'", word.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn action(args: &Args, label: &str, operation: &str, options: &[(&str, Value)]) -> Value {
    let argv = command(args, operation, options);
    json!({"label": label, "command": shell_join(&argv), "argv": argv})
}

fn push_action(view: &mut Value, item: Value) {
    if let Some(list) = view["actions"].as_array_mut() {
        list.push(item);
    }
}

fn push_unresolved(view: &mut Value, message: impl Into<String>) {
    if let Some(list) = view["unresolved"].as_array_mut() {
        list.push(json!(message.into()));
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Ceremony state helpers
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn orientation(recipe: &Value, state: &Value) -> Value {
    json!({
        "intent": state["intent"],
        "intent_source": recipe["intent_source"],
        "guiding_context": recipe["guiding_context"],
        "scope": state["scope"],
        "worthwhile_finish": recipe["finish"],
        "constraints": [
            "Evidence is not acceptance; existing governors decide effects.",
            "Substantive conflicts remain unresolved without judgment; holds need operator confirmation."
        ],
        "recipe_version": recipe["version"],
        "provider": state["provider"],
    })
}

/// The sidecar's native slot is from/to; attributes are not identity.
fn edge_identity(slot: &Value) -> String {
    let plane = slot["plane"].as_str().unwrap_or_default();
    let base = format!(
        "{plane}\u{1}{}\u{1}{}",
        slot["from"].as_str().unwrap_or_default(),
        slot["to"].as_str().unwrap_or_default()
    );
    if plane == "sidecar" {
        base
    } else {
        format!("{base}\u{1}{}", slot["description"])
    }
}

fn edges(projection: &Value) -> Vec<Value> {
    projection["groups"]
        .as_array()
        .map(|groups| {
            groups
                .iter()
                .flat_map(|group| {
                    group["edges"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn selected_edge(state: &Value) -> FlowResult<Value> {
    match state.get("selected") {
        Some(value) if !value.is_null() => Ok(value.clone()),
        _ => Err(refused("choose a concern from open first")),
    }
}

fn evidence_key(source: &Value) -> String {
    format!(
        "{}:{}",
        source["path"].as_str().unwrap_or_default(),
        source["lines"].as_str().unwrap_or_default()
    )
}

fn retain_evidence(state: &mut Value, result: &Value, need: &str) {
    let sources = result["sources"].as_array().cloned().unwrap_or_default();
    for source in sources {
        let key = evidence_key(&source);
        let previous_fingerprint = state["evidence"][&key]["fingerprint"].clone();
        let mut stored = source.clone();
        stored["need"] = json!(need);
        stored["valid"] = json!(true);
        if previous_fingerprint == stored["fingerprint"] {
            let repeats = state["repeated_reads"].as_u64().unwrap_or(0) + 1;
            state["repeated_reads"] = json!(repeats);
        }
        state["evidence"][key] = stored;
    }
}

/// Findings the current selection makes relevant, each annotated with whether its pinned receipts
/// still hold. `evidence_snapshots` never leaves this function: it is the private working copy.
fn relevant_findings(state: &Value, all_findings: bool) -> Vec<Value> {
    let slot = state
        .get("selected")
        .filter(|v| !v.is_null())
        .map(|s| s["slot"].clone());
    let mut findings = Vec::new();
    for item in state["findings"].as_array().cloned().unwrap_or_default() {
        if !all_findings {
            match &slot {
                Some(slot) => {
                    let edge = &item["edge"];
                    if edge.is_null() || edge_identity(edge) != edge_identity(slot) {
                        continue;
                    }
                }
                // The shared-projection arm is retired: `epr flow memory project` is its own verb,
                // so an unselected session has no relevant findings rather than a second home.
                None => continue,
            }
        }
        let references = item["evidence"].as_array().cloned().unwrap_or_default();
        let checked = references.iter().all(|key| {
            let key = key.as_str().unwrap_or_default();
            state["evidence"][key]["valid"] == json!(true)
                && state["evidence"][key]["fingerprint"] == item["evidence_pins"][key]
        });
        let mut public = item.clone();
        if let Some(map) = public.as_object_mut() {
            map.remove("evidence_snapshots");
        }
        public["evidence_state"] = json!(if checked && !references.is_empty() {
            "receipt-valid"
        } else {
            "revalidation-or-evidence-required"
        });
        findings.push(public);
    }
    findings
}

fn last_one(items: &[Value]) -> Vec<Value> {
    items.iter().rev().take(1).cloned().collect()
}

fn selected_evidence(state: &Value) -> Vec<String> {
    last_one(&relevant_findings(state, false))
        .first()
        .and_then(|f| f["evidence"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

/// Change detector for previously fingerprint-verified bytes, not acceptance evidence.
fn receipt_stamp(
    args: &Args,
    contract: &Contract,
    source: &Value,
    usage: &mut Value,
) -> Option<Value> {
    add_usage(usage, &json!({"source_stat_checks": 1}));
    let relative = source["path"].as_str()?;
    let path = contained(&args.root, relative, &contract.source_roots()).ok()?;
    let meta = path.metadata().ok()?;
    use std::os::unix::fs::MetadataExt;
    Some(json!([
        meta.dev(),
        meta.ino(),
        meta.size(),
        meta.mtime_nsec() + meta.mtime() * 1_000_000_000,
        meta.ctime_nsec() + meta.ctime() * 1_000_000_000,
    ]))
}

/// Re-read every named receipt within ONE shared read budget, and say what it could not reach.
///
/// The `pending` frontier is the load-bearing part: when the budget runs out the remaining receipts
/// are marked unknown (`valid: null`) rather than left looking checked, and the caller is handed an
/// executable continuation offset. A revalidation that quietly stopped early would report "receipts
/// valid" about receipts it never opened.
fn revalidate(
    args: &Args,
    contract: &Contract,
    state: &mut Value,
    usage: &mut Value,
    keys: Option<&[String]>,
    reuse_valid: bool,
    start_offset: Option<usize>,
) -> FlowResult<Value> {
    let all: Vec<String> = state["evidence"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let selected_keys: Vec<String> = match keys {
        None => all.clone(),
        Some(list) => {
            let mut seen = BTreeSet::new();
            list.iter()
                .filter(|k| seen.insert((*k).clone()))
                .cloned()
                .collect()
        }
    };
    if selected_keys.iter().any(|k| !all.contains(k)) {
        return Err(refused("revalidation names an unknown inspected receipt"));
    }
    let mut remaining_scan = contract.limit_usize("scan_bytes") as i64;
    let mut remaining_bytes = contract.limit_usize("source_bytes") as i64;
    let file_limit = contract.limit_usize("source_files");
    let start = start_offset.unwrap_or(args.evidence_offset);

    let (mut changed, mut unchanged, mut reused, mut pending): (
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
    ) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());

    for (index, key) in selected_keys.iter().enumerate() {
        if index < start {
            continue;
        }
        let source = state["evidence"][key].clone();
        let before = receipt_stamp(args, contract, &source, usage);
        if reuse_valid
            && source["valid"] == json!(true)
            && before.is_some()
            && source.get("validated_stamp") == before.as_ref()
        {
            reused.push(key.clone());
            continue;
        }
        if changed.len() + unchanged.len() >= file_limit
            || remaining_scan <= 0
            || remaining_bytes <= 0
        {
            state["evidence"][key]["valid"] = Value::Null;
            pending.push(key.clone());
            continue;
        }
        let (valid, after) = match excerpt(
            &args.root,
            contract,
            source["path"].as_str().unwrap_or_default(),
            source["lines"].as_str().unwrap_or_default(),
        ) {
            Ok(check) => {
                add_usage(usage, &check["usage"]);
                remaining_scan -= check["usage"]["scan_bytes"].as_i64().unwrap_or(0);
                remaining_bytes -= check["usage"]["source_bytes"].as_i64().unwrap_or(0);
                let after = receipt_stamp(args, contract, &source, usage);
                let matched = check["sources"]
                    .as_array()
                    .and_then(|a| a.first())
                    .is_some_and(|s| s["fingerprint"] == source["fingerprint"]);
                (before.is_some() && before == after && matched, after)
            }
            Err(_) => (false, None),
        };
        state["evidence"][key]["valid"] = json!(valid);
        state["evidence"][key]["validated_stamp"] = if valid {
            after.unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        if valid {
            unchanged.push(key.clone());
        } else {
            changed.push(key.clone());
        }
    }
    let next_offset = pending
        .first()
        .and_then(|first| selected_keys.iter().position(|k| k == first));
    Ok(json!({
        "unchanged_receipts": unchanged,
        "reused_receipts": reused,
        "revalidation_required": changed,
        "pending": pending.iter().take(args.limit).collect::<Vec<_>>(),
        "pending_count": pending.len(),
        "scope_count": selected_keys.len(),
        "next_offset": next_offset,
        "meaning": "Only the declared receipt scope is checked. Reused receipts retain an earlier fingerprint check with unchanged file identity/size/mtime/ctime; this is not acceptance. Findings tied to changed or pending receipts require renewed evidence.",
    }))
}

fn evidence_continuation(args: &Args, view: &mut Value, keys: Option<&[String]>) {
    let next = view["evidence_check"]["next_offset"].clone();
    if !next.is_null() {
        let evidence = keys.map(|k| json!(k)).unwrap_or(Value::Null);
        push_action(
            view,
            action(
                args,
                "Continue bounded evidence revalidation",
                "resume",
                &[
                    ("evidence_offset", next),
                    ("evidence", evidence),
                    ("limit", json!(args.limit)),
                ],
            ),
        );
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Native projections — in-process, charged by the bytes actually consumed
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Serialize an in-process projection and charge it, so `native_raw_bytes` still means "the bytes
/// this operation actually took in".
fn charge_native<T: serde::Serialize>(usage: &mut Value, value: &T) -> FlowResult<Value> {
    let bytes = serde_json::to_vec(value)?;
    add_usage(usage, &json!({"native_raw_bytes": bytes.len()}));
    Ok(serde_json::from_slice(&bytes)?)
}

fn native_concerns(
    args: &Args,
    state: &Value,
    offset: usize,
    usage: &mut Value,
) -> FlowResult<Value> {
    let scope = state["scope"].as_str().unwrap_or(".").to_string();
    let projection = concerns::concerns_with(&args.root, &scope, offset, args.limit, false)?;
    charge_native(usage, &projection)
}

/// Re-find the EXACT selected slot in the complete scoped projection, paging until it is resolved
/// or the native budget runs out.
///
/// Display offsets and limits never control this: a selection is an identity, and answering "is it
/// still there" from one displayed page would let a page boundary read as an absent edge.
fn refresh_selected(
    args: &Args,
    contract: &Contract,
    state: &mut Value,
    usage: &mut Value,
) -> FlowResult<Value> {
    let edge = selected_edge(state)?;
    let from = edge["slot"]["from"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let identity = edge_identity(&edge["slot"]);
    let mut offset = 0usize;
    let mut matches: Vec<Value> = Vec::new();
    let mut unresolved: Vec<String> = Vec::new();
    let mut remaining_bytes = contract.limit_usize("native_raw_bytes") as i64;
    let deadline =
        Instant::now() + Duration::from_secs_f64(contract.limit_secs("native_timeout_seconds"));
    let mut next_offset: Option<usize> = None;
    loop {
        if Instant::now() >= deadline {
            unresolved.push(
                "Exact selected-edge lookup exceeded its total native time budget; current standing is unknown."
                    .into(),
            );
            break;
        }
        let mut consumed = json!({});
        let projection = concerns::concerns_with(&args.root, &from, offset, 100, true)?;
        let projection = charge_native(&mut consumed, &projection)?;
        add_usage(usage, &consumed);
        remaining_bytes -= consumed["native_raw_bytes"].as_i64().unwrap_or(0);
        matches.extend(
            edges(&projection)
                .into_iter()
                .filter(|row| edge_identity(&row["slot"]) == identity),
        );
        next_offset = projection["page"]["next_offset"]
            .as_u64()
            .map(|v| v as usize);
        if !unresolved.is_empty() || next_offset.is_none() || matches.len() > 1 {
            break;
        }
        let next = next_offset.unwrap_or(offset);
        if remaining_bytes <= 0 || next <= offset {
            unresolved.push(
                "Exact selected-edge lookup is incomplete within its native byte budget; no missing-edge conclusion is warranted."
                    .into(),
            );
            break;
        }
        offset = next;
    }
    if unresolved.is_empty() && matches.len() != 1 {
        unresolved.push(if matches.is_empty() {
            "Selected native slot is absent from the complete scoped projection.".into()
        } else {
            "Selected native slot is ambiguous; choose a unique current assertion before acting."
                .to_string()
        });
    }
    let current = (matches.len() == 1 && unresolved.is_empty()).then(|| matches[0].clone());
    let mut changed_fields: Vec<String> = Vec::new();
    if let Some(current) = &current {
        for key in [
            "current_evidence",
            "consumer_evidence",
            "sealed_evidence",
            "verdict",
        ] {
            if current.get(key) != edge.get(key) {
                changed_fields.push(key.into());
            }
        }
        for key in ["description", "governor"] {
            if current["slot"].get(key) != edge["slot"].get(key) {
                changed_fields.push(format!("slot.{key}"));
            }
        }
    }
    // Unknown is not false: with no resolved current edge, "did it change" has no answer, and
    // reporting `false` there would read as "unchanged" about an edge nobody could find.
    let changed_since_selection = match &current {
        Some(_) => json!(!changed_fields.is_empty()),
        None => Value::Null,
    };
    let result = json!({
        "matching_edges": matches,
        "current_edge": current,
        "changed_since_selection": changed_since_selection,
        "changed_fields": changed_fields,
        "unresolved": unresolved,
        "next_offset": next_offset,
        "meaning": "Current exact native slots; an absent, ambiguous or incompletely searched selection must be resolved again. Cached selection is a locator and original observation only.",
    });
    state["selected_current"] = result.clone();
    Ok(result)
}

/// Open one bounded native context section, refusing an indexed navigation choice taken against a
/// projection that has since changed.
fn native_context(args: &Args, edge: &Value, view: &mut Value, section: &str) -> FlowResult<Value> {
    let from = edge["slot"]["from"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut usage = json!({});
    let projected = context_sections::context_section(
        &args.root,
        &from,
        section,
        context::DEFAULT_NOTES,
        args.offset,
        args.limit.max(1),
    )?;
    let context_value = charge_native(&mut usage, &projected)?;
    add_usage(&mut view["usage"], &usage);
    if context_value["found"] == json!(false) {
        push_unresolved(view, "Requested native section is absent; reopen current sections instead of treating it as empty history.");
    }
    let pin = hex(&Sha256::digest(
        serde_json::to_string(&context_value["projection_identity"])?.as_bytes(),
    ));
    if let Some(expected) = &args.context_pin {
        if expected != &pin {
            push_unresolved(view, "Native context changed since this navigation choice; reopen its sections before following an indexed record.");
            push_action(
                view,
                action(
                    args,
                    "Reopen changed native context",
                    "context",
                    &[("section", json!(".")), ("offset", json!(0))],
                ),
            );
            return Ok(json!({
                "unresolved": ["stale native section navigation refused"],
                "projection_identity": context_value["projection_identity"],
            }));
        }
    }
    for item in context_value["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let kind = item["kind"].as_str().unwrap_or_default();
        if kind == "object" || kind == "array" || item["omitted_items"].as_u64().unwrap_or(0) > 0 {
            push_action(
                view,
                action(
                    args,
                    &format!(
                        "Expand native {}",
                        item["section"].as_str().unwrap_or_default()
                    ),
                    "context",
                    &[
                        ("section", item["section"].clone()),
                        ("context_pin", json!(pin)),
                        ("offset", json!(0)),
                        (
                            "limit",
                            json!(if kind == "string" { 100 } else { args.limit }),
                        ),
                    ],
                ),
            );
        }
    }
    if let Some(next) = context_value["page"]["next_offset"].as_u64() {
        push_action(
            view,
            action(
                args,
                "Continue this native section",
                "context",
                &[
                    ("section", json!(section)),
                    ("context_pin", json!(pin)),
                    ("offset", json!(next)),
                    ("limit", json!(args.limit)),
                ],
            ),
        );
    }
    if section != "." {
        push_action(
            view,
            action(
                args,
                "Return to native context sections",
                "context",
                &[("section", json!(".")), ("offset", json!(0))],
            ),
        );
    }
    Ok(context_value)
}

/// Bounded table of contents, so the agent chooses a passage before loading it.
/// One section of an outline, with where the question's terms actually land inside it.
///
/// Naming a document is half an answer. A reader that opens a 24,000-byte skill and is handed six
/// headings, none of which mentions its question, closes the file — which is exactly what happened
/// on 2026-09-11: the answer was under "Phase 4 — verify the experience, reconcile and retain
/// learning", a heading that says nothing about re-mining an index.
fn outline(args: &Args, contract: &Contract, path: &str) -> FlowResult<Value> {
    outline_with_terms(args, contract, path, &question_terms(args.need.trim()))
}

fn outline_with_terms(
    args: &Args,
    contract: &Contract,
    path: &str,
    terms: &[String],
) -> FlowResult<Value> {
    let source = contained(&args.root, path, &contract.source_roots())?;
    let bound = contract.limit_usize("scan_bytes");
    let mut raw = Vec::new();
    File::open(&source)
        .and_then(|f| f.take(bound as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|error| FlowError::Read {
            path: source.clone(),
            source: error,
        })?;
    let complete = raw.len() <= bound;
    let text = String::from_utf8_lossy(&raw[..raw.len().min(bound)]).to_string();
    let lines: Vec<&str> = text.lines().collect();
    let is_code = source.extension().and_then(|e| e.to_str()) == Some("py");
    let mut headings: Vec<Value> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('#')
            || (is_code && (line.starts_with("def ") || line.starts_with("class ")))
        {
            headings.push(json!({
                "line": index + 1,
                "title": line.trim_start_matches('#').trim(),
            }));
        }
    }
    for index in 0..headings.len() {
        let end = if index + 1 < headings.len() {
            headings[index + 1]["line"].as_u64().unwrap_or(1) - 1
        } else {
            lines.len() as u64
        };
        headings[index]["end_line"] = json!(end);
    }
    // Where the question's terms land, section by section, plus a read range bounded to what one
    // excerpt may return. The bytes counted here are scan bytes; only `read` charges source_bytes.
    let source_bytes = contract.limit_usize("source_bytes");
    let mut oversized_sections = 0usize;
    for heading in headings.iter_mut() {
        let start = heading["line"].as_u64().unwrap_or(1) as usize;
        let end = (heading["end_line"].as_u64().unwrap_or(1) as usize).max(start);
        let section: String = lines[start.saturating_sub(1)..end.min(lines.len())]
            .join("\n")
            .to_lowercase();
        let mut hits: Map<String, Value> = Map::new();
        let mut total = 0usize;
        for term in terms {
            let count = section.matches(&term.to_lowercase()).count();
            if count > 0 {
                hits.insert(term.clone(), json!(count));
                total += count;
            }
        }
        // The emitted range never promises more than one excerpt can carry: lines are taken from
        // the section's start until the byte budget is spent, and a section that does not fit says
        // so rather than handing over a range `read` would refuse.
        let (mut window_end, mut bytes) = (start, 0usize);
        for (offset, line) in lines[start.saturating_sub(1)..end.min(lines.len())]
            .iter()
            .enumerate()
        {
            let next = bytes + line.len() + 1;
            if next > source_bytes && window_end > start {
                break;
            }
            bytes = next;
            window_end = start + offset;
        }
        heading["hits"] = json!(hits);
        heading["hit_total"] = json!(total);
        heading["read_lines"] = json!(format!("{start}:{window_end}"));
        heading["window_complete"] = json!(window_end >= end);
        if window_end < end {
            oversized_sections += 1;
        }
    }
    let truncated = !complete || headings.len() > 40;
    headings.truncate(40);
    let mut omissions: Vec<Value> = Vec::new();
    if truncated {
        omissions.push(json!("outline incomplete; use an explicit range"));
    }
    if oversized_sections > 0 {
        omissions.push(json!(format!(
            "{oversized_sections} section(s) exceed the per-excerpt byte budget; the offered range \
             is the first window of each — continue with an explicit later range"
        )));
    }
    Ok(json!({
        "path": path,
        "headings": headings,
        "line_count": lines.len(),
        "terms": terms,
        "hit_scope": "term occurrences inside each section of this outline; scan bytes, not evidence",
        "omissions": omissions,
        "usage": {"scan_bytes": raw.len(), "scanned_files": 1},
    }))
}

/// The section of `path` whose text carries most of the question's terms, if any does.
fn best_section(args: &Args, contract: &Contract, path: &str, terms: &[String]) -> Option<Value> {
    let outline = outline_with_terms(args, contract, path, terms).ok()?;
    let mut best: Option<Value> = None;
    for heading in outline["headings"].as_array()? {
        if heading["hit_total"].as_u64().unwrap_or(0) == 0 {
            continue;
        }
        let better = best
            .as_ref()
            .is_none_or(|current| heading["hit_total"].as_u64() > current["hit_total"].as_u64());
        if better {
            best = Some(heading.clone());
        }
    }
    let mut section = best?;
    section["usage"] = outline["usage"].clone();
    Some(section)
}

fn continuation_page(args: &Args, state: &Value) -> Value {
    let all_items = relevant_findings(state, true);
    let page: Vec<Value> = all_items
        .iter()
        .skip(args.offset)
        .take(args.limit)
        .cloned()
        .collect();
    let receipts: Vec<Value> = state["evidence"]
        .as_object()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    let questions = state["frontier"].as_array().cloned().unwrap_or_default();
    let total = all_items.len().max(receipts.len()).max(questions.len());
    json!({
        "findings": page,
        "evidence": receipts.iter().skip(args.offset).take(args.limit).map(without_content).collect::<Vec<_>>(),
        "unresolved_questions": questions.iter().skip(args.offset).take(args.limit).cloned().collect::<Vec<_>>(),
        "offset": args.offset,
        "limit": args.limit,
        "counts": {"findings": all_items.len(), "evidence": receipts.len(), "questions": questions.len()},
        "next_offset": (args.offset + args.limit < total).then_some(args.offset + args.limit),
    })
}

fn without_content(item: &Value) -> Value {
    let mut copy = item.clone();
    if let Some(map) = copy.as_object_mut() {
        map.remove("content");
    }
    copy
}

/// The cite entry's human description — the second `|`-separated field of a `cites:` row.
fn cite_desc(entry: &str) -> Option<String> {
    let parts: Vec<&str> = entry.split('|').collect();
    (parts.len() >= 2).then(|| parts[1].trim().to_string())
}

/// Route a doc-plane repair to the cite writer, and only when the slot is unambiguous.
fn doc_repair(args: &Args, contract: &Contract, edge: &Value) -> FlowResult<Vec<String>> {
    let from = edge["slot"]["from"].as_str().unwrap_or_default();
    let to = edge["slot"]["to"].as_str().unwrap_or_default();
    let description = edge["slot"]["description"].as_str();
    let path = contained(&args.root, from, &contract.source_roots())?;
    let limit = contract.limit_usize("source_bytes");
    let mut raw = Vec::new();
    File::open(&path)
        .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut raw).map(|_| ()))
        .map_err(|error| FlowError::Read {
            path: path.clone(),
            source: error,
        })?;
    if raw.len() > limit {
        return Err(refused(
            "document exceeds repair inspection budget; use a separately reviewed source edit",
        ));
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let declarations = super::super::parse_frontmatter(&text);
    let entries = declarations.list("cites").to_vec();
    let matches: Vec<&String> = entries
        .iter()
        .filter(|entry| {
            super::super::cite_path(entry).as_deref() == Some(to)
                && cite_desc(entry).as_deref() == description
        })
        .collect();
    let reference = matches
        .first()
        .and_then(|entry| super::super::cite_slug(entry));
    let ambiguous = matches.len() != 1
        || reference.is_none()
        || entries
            .iter()
            .filter(|entry| super::super::cite_slug(entry) == reference)
            .count()
            != 1;
    if ambiguous {
        return Err(refused(
            "citation slot is ambiguous; a cites refresh would affect neighboring assertions. Review a source-specific edit instead.",
        ));
    }
    Ok(vec![
        "epr".into(),
        "flow".into(),
        "cites".into(),
        "refresh".into(),
        from.into(),
        reference.unwrap_or_default(),
    ])
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The ceremony
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn prior_state(args: &Args, state_limit: usize) -> FlowResult<Value> {
    let label = args
        .from_session
        .as_deref()
        .filter(|l| valid_session(l))
        .ok_or_else(|| {
            refused("adopt requires --from-session with an existing local session label")
        })?;
    let path = args
        .root
        .join(RECALL_DIR_REL)
        .join(label)
        .join("continuation.json");
    let meta = path.symlink_metadata().map_err(|source| FlowError::Read {
        path: path.clone(),
        source,
    })?;
    if meta.file_type().is_symlink() {
        return Err(refused("prior receipt must not be a symlink"));
    }
    let mut raw = Vec::new();
    File::open(&path)
        .and_then(|f| {
            f.take(state_limit as u64 + 1)
                .read_to_end(&mut raw)
                .map(|_| ())
        })
        .map_err(|source| FlowError::Read { path, source })?;
    if raw.len() > state_limit {
        return Err(refused("prior continuation exceeds state budget"));
    }
    Ok(serde_json::from_slice(&raw)?)
}

#[allow(clippy::too_many_lines)]
fn execute(
    args: &Args,
    contract: &Contract,
    execution: &mut Execution,
    method: &str,
) -> FlowResult<Value> {
    let recipe = contract.recipe().clone();
    let session_limit = contract
        .value
        .pointer("/limits/session_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let mut state = execution.state["ceremony"].clone();

    if args.operation == "adopt" {
        if !state.is_null() {
            return Err(refused(
                "adopt needs a new session; existing continuation cannot be overwritten",
            ));
        }
        let prior = prior_state(args, session_limit)?;
        state = prior["ceremony"].clone();
        // BOTH pins are carried forward, not just the algorithm's. Adoption is how a method change
        // is continued explicitly, so the receipt has to say which method and which executor the
        // inherited counters were accumulated under — otherwise the adopted totals are a sum with
        // no stated provenance.
        execution.state["prior_receipt"] = json!({
            "session": args.from_session,
            "method": prior["method"],
            "executor_digest": prior["executor_digest"],
            "totals": prior["totals"],
            "attempts": prior["attempts"],
            "parent_session": prior["prior_receipt"]["session"],
        });
    }
    let new_ceremony = state.is_null();
    if new_ceremony {
        if args.operation != "open" {
            return Err(refused("start with open; no ceremony continuation exists"));
        }
        let scope = args.scope.clone().unwrap_or_else(|| {
            recipe["defaults"]["scope"]
                .as_str()
                .unwrap_or(".")
                .to_string()
        });
        if Path::new(&scope).is_absolute()
            || confine_under(&args.root, &args.root.join(&scope)).is_err()
        {
            return Err(refused("scope must remain inside the repository"));
        }
        // The agent's OWN question is the session's intent whenever it did not name one. The
        // documented entry is `open --need '<question>'`; before this, that question was recorded
        // nowhere and every later receipt, finish and measurement was accounted against the
        // recipe's generic purpose — an intent nobody carried.
        let intent = args
            .intent
            .clone()
            .or_else(|| {
                args.need_explicit
                    .then(|| args.need.trim().to_string())
                    .filter(|need| !need.is_empty())
            })
            .unwrap_or_else(|| {
                contract.value["purpose"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            });
        state = json!({
            "intent": intent,
            "scope": scope,
            "provider": recipe["defaults"]["provider"],
            "evidence": {}, "findings": [], "frontier": [], "selected": Value::Null,
            "repeated_reads": 0, "next_action": "choose a concern",
        });
    }
    if args
        .scope
        .as_deref()
        .is_some_and(|s| json!(s) != state["scope"])
        || args
            .intent
            .as_deref()
            .is_some_and(|i| json!(i) != state["intent"])
    {
        return Err(refused(
            "intent/scope changed; open a new session and explicitly retain the prior receipt",
        ));
    }

    let mut view = json!({
        "orientation": orientation(&recipe, &state),
        "operation": args.operation,
        "actions": [],
        "usage": {},
        "unresolved": [],
    });

    let has_measurements = !recipe["measurements"].is_null();
    if new_ceremony && has_measurements {
        view["measurement"] = measure(args, contract, &mut state, "baseline", method)?;
    } else if matches!(args.operation.as_str(), "open" | "resume" | "adopt")
        && !state["measurements"]["baseline"].is_null()
    {
        view["measurement"] = json!({
            "baseline": state["measurements"]["baseline"],
            "meaning": "Original baseline retained; resuming does not resample it.",
        });
    }

    match args.operation.as_str() {
        "open" | "resume" | "adopt" => {
            let evidence_keys: Option<Vec<String>> =
                (!args.evidence.is_empty()).then(|| args.evidence.clone());
            if args.operation != "open" {
                let mut usage = view["usage"].take();
                view["evidence_check"] = revalidate(
                    args,
                    contract,
                    &mut state,
                    &mut usage,
                    evidence_keys.as_deref(),
                    false,
                    None,
                )?;
                if !state["selected"].is_null() {
                    view["selected_current"] =
                        refresh_selected(args, contract, &mut state, &mut usage)?;
                }
                view["usage"] = usage;
                evidence_continuation(args, &mut view, evidence_keys.as_deref());
            }
            if args.operation != "open" && !state["selected"].is_null() {
                push_action(
                    &mut view,
                    action(
                        args,
                        "Open remaining concerns in the preserved scope",
                        "open",
                        &[
                            ("offset", json!(0)),
                            ("limit", json!(args.limit)),
                            ("need", json!("Choose another justified concern")),
                        ],
                    ),
                );
            } else {
                let mut usage = view["usage"].take();
                // The focused door first. A reader who named an area or asked a question whose
                // terms touch a habit gets that habit, its last delta and the sources competing to
                // answer, BEFORE the stale-edge groups — the two doors are the same journey with
                // different first screens.
                if args.operation == "open" {
                    if let Some(screen) = first_screen(args, contract, &state, &mut usage)? {
                        for candidate in
                            screen["candidates"].as_array().cloned().unwrap_or_default()
                        {
                            let path = candidate["path"].as_str().unwrap_or_default().to_string();
                            // Straight to the passage its terms land in when one was located; the
                            // outline only when nothing was.
                            match (
                                candidate["best_section"]["lines"].as_str(),
                                candidate["best_section"]["title"].as_str(),
                            ) {
                                (Some(lines), Some(title)) => push_action(
                                    &mut view,
                                    action(
                                        args,
                                        &format!("Read {path} — {title} ({lines})"),
                                        "read",
                                        &[("path", json!(path)), ("lines", json!(lines))],
                                    ),
                                ),
                                _ => push_action(
                                    &mut view,
                                    action(
                                        args,
                                        &format!("Outline {path}"),
                                        "source",
                                        &[("path", json!(path))],
                                    ),
                                ),
                            }
                        }
                        view["first_screen"] = screen;
                    }
                }
                let projection = native_concerns(args, &state, args.offset, &mut usage)?;
                view["usage"] = usage;
                if !projection["groups"].is_array() || projection["counts"].is_null() {
                    return Err(refused(
                        "native concern projection is unavailable; rebuild the verified CLI",
                    ));
                }
                state["page"] = projection.clone();
                let mut annotated = projection.clone();
                let history = relevant_findings(&state, true);
                if let Some(groups) = annotated["groups"].as_array_mut() {
                    for group in groups.iter_mut() {
                        if let Some(rows) = group["edges"].as_array_mut() {
                            for edge in rows.iter_mut() {
                                let identity = edge_identity(&edge["slot"]);
                                let assessments: Vec<Value> = history
                                    .iter()
                                    .filter(|item| {
                                        !item["edge"].is_null()
                                            && edge_identity(&item["edge"]) == identity
                                    })
                                    .cloned()
                                    .collect();
                                edge["assessment_count"] = json!(assessments.len());
                                edge["investigator_assessments"] = json!(last_one(&assessments));
                                edge["assessment_scope"] = json!("latest investigator observation only; history exposes every judgment");
                            }
                        }
                    }
                }
                view["concerns"] = annotated.clone();
                for (number, edge) in edges(&annotated).iter().enumerate() {
                    push_action(
                        &mut view,
                        action(
                            args,
                            &format!(
                                "{}. Inspect {} → {}",
                                number + 1,
                                edge["slot"]["from"].as_str().unwrap_or_default(),
                                edge["slot"]["to"].as_str().unwrap_or_default()
                            ),
                            "select",
                            &[
                                ("edge", json!(number + 1)),
                                (
                                    "need",
                                    json!("Understand this assertion and its changed evidence"),
                                ),
                            ],
                        ),
                    );
                }
                if let Some(next) = annotated["page"]["next_offset"].as_u64() {
                    push_action(
                        &mut view,
                        action(
                            args,
                            "Continue the next page; preserve scope",
                            "open",
                            &[
                                ("offset", json!(next)),
                                ("need", json!("Inspect remaining concerns")),
                            ],
                        ),
                    );
                }
            }
            let receipts: Vec<Value> = state["evidence"]
                .as_object()
                .map(|m| m.values().cloned().collect())
                .unwrap_or_default();
            let questions = state["frontier"].as_array().cloned().unwrap_or_default();
            view["continuation"] = json!({
                "selected": state["selected"],
                "findings": last_one(&relevant_findings(&state, false)),
                "evidence": last_one(&receipts).iter().map(without_content).collect::<Vec<_>>(),
                "unresolved_questions": last_one(&questions),
                "next_action": state["next_action"],
                "counts": {
                    "findings": state["findings"].as_array().map(Vec::len).unwrap_or(0),
                    "evidence": receipts.len(),
                    "questions": questions.len(),
                },
                "omissions": "Only the most recent selected finding, receipt and question are summarized; history expands every retained item.",
                "repeated_reads": state["repeated_reads"],
                "prior_receipt": execution.state["prior_receipt"],
            });
        }
        "measure" => {
            view["measurement"] = measure(args, contract, &mut state, &args.phase, method)?;
        }
        "history" => {
            view["history"] = continuation_page(args, &state);
            if let Some(next) = view["history"]["next_offset"].as_u64() {
                push_action(
                    &mut view,
                    action(
                        args,
                        "Continue retained investigation",
                        "history",
                        &[("offset", json!(next)), ("limit", json!(args.limit))],
                    ),
                );
            }
        }
        "compare" => {
            if args.evidence.len() != 1 {
                return Err(refused(
                    "compare requires one --evidence path:START:END receipt key",
                ));
            }
            let key = args.evidence[0].clone();
            let previous: Vec<Value> = state["findings"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|item| item["evidence_snapshots"].get(&key).cloned())
                .collect();
            view["comparison"] = json!({
                "key": key,
                "current_inspected_receipt": state["evidence"].get(&key),
                "previous_judgment_receipts": previous.iter().skip(args.offset).take(args.limit).cloned().collect::<Vec<_>>(),
                "total_previous": previous.len(),
                "offset": args.offset,
                "meaning": "Inspected versions, not an assertion that the source is currently unchanged.",
            });
        }
        "recipe" => {
            view["recipe"] = recipe.clone();
            view["contract"] = json!({
                "id": contract.value["id"],
                "version": contract.value["version"],
                "limits": contract.value["limits"],
                "source_roots": contract.value["source_roots"],
                "governance": contract.value["governance"],
            });
        }
        "select" | "context" => {
            if args.operation == "select" {
                let available = edges(&state["page"]);
                let index = args
                    .edge
                    .filter(|n| *n >= 1 && *n <= available.len())
                    .ok_or_else(|| refused("--edge must name a row in the last displayed page"))?;
                state["selected"] = available[index - 1].clone();
                state["selection_reason"] = json!(args.need);
            }
            let edge = selected_edge(&state)?;
            let mut usage = view["usage"].take();
            view["selected_current"] = refresh_selected(args, contract, &mut state, &mut usage)?;
            view["usage"] = usage;
            let current = view["selected_current"]["current_edge"].clone();
            let from = edge["slot"]["from"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let to = edge["slot"]["to"].as_str().unwrap_or_default().to_string();
            let node_evidence: Vec<Value> = state["evidence"]
                .as_object()
                .map(|m| {
                    m.values()
                        .filter(|item| {
                            let path = item["path"].as_str().unwrap_or_default();
                            path == from || path == to
                        })
                        .map(without_content)
                        .collect()
                })
                .unwrap_or_default();
            view["node"] = json!({
                "edge": current,
                "original_edge": edge,
                "why_followed": state["selection_reason"],
                "original_purpose": "See source intent and ruling records; absent history remains unknown.",
                "original_claim": edge["slot"]["description"],
                "current_claim": match &view["selected_current"]["current_edge"] {
                    Value::Null => json!("Current exact edge unavailable; original wording is historical only."),
                    current => match current["slot"]["description"].as_str() {
                        Some(text) => json!(text),
                        None => json!("Not stated on the current edge; inspect the source."),
                    },
                },
                "classification": "Native standing is separate from investigator judgment; evidence-ready/conflict requires source-backed judgment",
                "evidence": node_evidence,
                "findings": last_one(&relevant_findings(&state, false)),
                "omissions": "Latest selected finding shown; history expands retained judgments.",
            });
            view["input_choices"] = json!([
                {"label": "Retain a source-backed finding and uncertainty", "operation": "remember",
                 "required_inputs": ["--finding", "--question", "--next-action", "--evidence path:START:END"],
                 "optional_input": "--classification unreviewed|evidence-ready|conflict|missing-evidence",
                 "command_prefix": shell_join(&command(args, "remember", &[]))},
                {"label": "Prepare a scoped repair or request judgment", "operation": "prepare",
                 "required_inputs": ["--kind repair|judgment|observation", "--finding"],
                 "prerequisite": "Repair requires current inspected evidence; preparation does not execute or approve it.",
                 "command_prefix": shell_join(&command(args, "prepare", &[]))},
                {"label": "Finish with a bounded outcome or responsible stop", "operation": "finish",
                 "required_inputs": ["--outcome", "--question (unresolved frontier)"],
                 "command_prefix": shell_join(&command(args, "finish", &[]))}
            ]);
            if args.operation == "context" {
                let section = args.section.clone().unwrap_or_else(|| ".".into());
                view["native_context"] = native_context(args, &edge, &mut view, &section)?;
            } else {
                for side in ["from", "to"] {
                    let path = edge["slot"][side].as_str().unwrap_or_default().to_string();
                    match outline(args, contract, &path) {
                        Ok(mut contents) => {
                            let usage = contents["usage"].take();
                            add_usage(&mut view["usage"], &usage);
                            let headings =
                                contents["headings"].as_array().cloned().unwrap_or_default();
                            view["node"][format!("{side}_contents")] = contents;
                            for heading in headings.iter().take(8) {
                                push_action(&mut view, action(args,
                                    &format!("Open {side}: {}", heading["title"].as_str().unwrap_or_default()),
                                    "read",
                                    &[("path", json!(path)),
                                      ("lines", json!(format!("{}:{}", heading["line"], heading["end_line"]))),
                                      ("need", json!("Inspect this section for the selected assertion"))]));
                            }
                        }
                        Err(error) => push_unresolved(&mut view, format!("{path}: {error}")),
                    }
                }
            }
            push_action(
                &mut view,
                action(
                    args,
                    "Inspect native intent, review and acceptance evidence",
                    "context",
                    &[("need", json!("Inspect native standing"))],
                ),
            );
            if args.operation == "select" {
                state["next_action"] = json!(
                    "Inspect relevant passages; record evidence-backed findings and uncertainty"
                );
            }
        }
        "source" => {
            // `--tag` without `--path` is the "which sources carry this" question the kit's
            // `--tag <category>` answered: exact frontmatter membership over a bounded traversal,
            // grouped by tag, no ranking. It ANSWERS with candidates and linked outline actions
            // rather than opening anything, so membership still never establishes authority.
            if args.path.is_none() && !args.tags.is_empty() {
                let mut found = discover(
                    &args.root,
                    contract,
                    &args.search_scope,
                    args.query.as_deref().unwrap_or_default(),
                    &args.tags,
                    "tag",
                    &args.name,
                )?;
                let usage = found["usage"].take();
                add_usage(&mut view["usage"], &usage);
                for message in found["unresolved"].as_array().cloned().unwrap_or_default() {
                    push_unresolved(&mut view, message.as_str().unwrap_or_default().to_string());
                }
                for candidate in found["candidates"].as_array().cloned().unwrap_or_default() {
                    let candidate_path = candidate["path"].as_str().unwrap_or_default().to_string();
                    push_action(
                        &mut view,
                        action(
                            args,
                            &format!("Outline {candidate_path}"),
                            "source",
                            &[("path", json!(candidate_path))],
                        ),
                    );
                }
                found["tags"] = json!(args.tags);
                view["source_candidates"] = found;
            } else {
                let path = args.path.clone().ok_or_else(|| {
                refused(
                    "source needs --path from a shared evidence reference, or --tag to discover one",
                )
            })?;
                let mut contents = outline(args, contract, &path)?;
                let usage = contents["usage"].take();
                add_usage(&mut view["usage"], &usage);
                let headings = contents["headings"].as_array().cloned().unwrap_or_default();
                let line_count = contents["line_count"].as_u64().unwrap_or(0);
                view["source_outline"] = contents;
                // Sections the question's terms actually land in come FIRST, with a range bounded
                // to what one excerpt may return. A reader handed six headings in document order,
                // none of which names its question, closes the file.
                let mut ordered: Vec<(usize, Value)> =
                    headings.iter().cloned().enumerate().collect();
                ordered.sort_by(|a, b| {
                    b.1["hit_total"]
                        .as_u64()
                        .cmp(&a.1["hit_total"].as_u64())
                        .then_with(|| a.0.cmp(&b.0))
                });
                for (_, heading) in ordered.iter().take(8) {
                    let hits = heading["hit_total"].as_u64().unwrap_or(0);
                    let title = heading["title"].as_str().unwrap_or_default();
                    let label = if hits > 0 {
                        format!("Read {title} — {}", render_hits(&heading["hits"]))
                    } else {
                        format!("Inspect {title}")
                    };
                    let lines = heading["read_lines"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("{}:{}", heading["line"], heading["end_line"]));
                    push_action(
                        &mut view,
                        action(
                            args,
                            &label,
                            "read",
                            &[("path", json!(path)), ("lines", json!(lines))],
                        ),
                    );
                }
                if headings.is_empty() && line_count > 0 {
                    push_action(
                        &mut view,
                        action(
                            args,
                            "Inspect the opening passage",
                            "read",
                            &[
                                ("path", json!(path)),
                                ("lines", json!(format!("1:{}", line_count.min(20)))),
                            ],
                        ),
                    );
                }
            }
        }
        "read" => {
            let (path, lines) = match (&args.path, &args.lines) {
                (Some(path), Some(lines)) => (path.clone(), lines.clone()),
                _ => return Err(refused("read requires --path and --lines START:END")),
            };
            let mut result = excerpt(&args.root, contract, &path, &lines)?;
            let usage = result["usage"].take();
            add_usage(&mut view["usage"], &usage);
            retain_evidence(&mut state, &result, &args.need);
            let sources = result["sources"].as_array().cloned().unwrap_or_default();
            view["receipt_keys"] = json!(sources.iter().map(evidence_key).collect::<Vec<_>>());
            for message in result["unresolved"].as_array().cloned().unwrap_or_default() {
                push_unresolved(&mut view, message.as_str().unwrap_or_default().to_string());
            }
            view["evidence"] = result;
            state["next_action"] =
                json!("Compare evidence; record a finding or keep an unresolved question");
            let destination = if state["selected"].is_null() {
                "open"
            } else {
                "context"
            };
            push_action(
                &mut view,
                action(
                    args,
                    "Return to this concern and its available actions",
                    destination,
                    &[(
                        "need",
                        json!("Compare inspected evidence with native standing"),
                    )],
                ),
            );
        }
        "remember" => {
            let (finding, question, next_action) = match (
                &args.finding,
                &args.question,
                &args.next_action,
            ) {
                (Some(f), Some(q), Some(n)) => (f.clone(), q.clone(), n.clone()),
                _ => return Err(refused(
                    "remember takes --finding --question --next-action --evidence path:START:END; \
                     --finding, --question and --next-action are all required and uncertainty \
                     cannot be omitted",
                )),
            };
            let references = args.evidence.clone();
            if references.iter().any(|key| {
                state["evidence"].get(key).is_none()
                    || state["evidence"][key]["valid"] != json!(true)
            }) {
                return Err(refused(
                    "finding evidence must reference valid inspected receipts",
                ));
            }
            if args.classification == "evidence-ready" && references.is_empty() {
                return Err(refused(
                    "evidence-ready requires inspected evidence references",
                ));
            }
            if state["selected"].is_null() {
                return Err(refused("choose an assertion before retaining a finding"));
            }
            let mut pins = Map::new();
            let mut snapshots = Map::new();
            for key in &references {
                pins.insert(key.clone(), state["evidence"][key]["fingerprint"].clone());
                snapshots.insert(key.clone(), state["evidence"][key].clone());
            }
            let finding_record = json!({
                "claim": finding,
                "evidence": references,
                "evidence_pins": pins,
                "evidence_snapshots": snapshots,
                "classification": args.classification,
                "standing": "investigator observation, not technical review or acceptance",
                "edge": selected_edge(&state)?["slot"],
                "memory_request": Value::Null,
            });
            if let Some(list) = state["findings"].as_array_mut() {
                list.push(finding_record);
            }
            if let Some(list) = state["frontier"].as_array_mut() {
                list.push(json!(question));
            }
            state["next_action"] = json!(next_action);
            view["retained"] = last_one(&relevant_findings(&state, false))
                .into_iter()
                .next()
                .unwrap_or(Value::Null);
        }
        "search" => {
            let provider = args
                .provider
                .clone()
                .unwrap_or_else(|| state["provider"].as_str().unwrap_or("local").to_string());
            // The evidence question stands in for a missing `--query` — EXCEPT when tags were
            // named, because then the tags are the filter and folding the need's prose in as a
            // substring match would silently empty the result for a caller who filtered correctly.
            let query = args.query.clone().unwrap_or_else(|| {
                if args.tags.is_empty() {
                    args.need.clone()
                } else {
                    String::new()
                }
            });
            let mut result = retrieve(
                &args.root,
                contract,
                &provider,
                &query,
                &args.search_scope,
                &args.name,
                &args.tags,
            )?;
            state["provider"] = json!(provider);
            if state["provider_history"].is_null() {
                state["provider_history"] = json!([]);
            }
            if let Some(list) = state["provider_history"].as_array_mut() {
                list.push(json!({"provider": provider, "need": args.need,
                                 "unresolved": result["unresolved"]}));
            }
            let usage = result["usage"].take();
            add_usage(&mut view["usage"], &usage);
            for message in result["unresolved"].as_array().cloned().unwrap_or_default() {
                push_unresolved(&mut view, message.as_str().unwrap_or_default().to_string());
            }
            view["retrieval"] = result;
        }
        "prepare" => {
            let edge = selected_edge(&state)?;
            let keys = selected_evidence(&state);
            let mut usage = view["usage"].take();
            view["evidence_check"] = revalidate(
                args,
                contract,
                &mut state,
                &mut usage,
                Some(&keys),
                true,
                Some(0),
            )?;
            view["selected_current"] = refresh_selected(args, contract, &mut state, &mut usage)?;
            view["usage"] = usage;
            evidence_continuation(args, &mut view, Some(&keys));
            let finding = args.finding.clone().ok_or_else(|| {
                refused("prepare needs --finding explaining the evidenced action")
            })?;
            let from = edge["slot"]["from"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let to = edge["slot"]["to"].as_str().unwrap_or_default().to_string();
            let mut words: Vec<String> = vec![
                "note".into(),
                "--on".into(),
                from.clone(),
                "--kind".into(),
                "observation".into(),
                "--reason".into(),
                finding.clone(),
            ];
            let mut issue: Option<String> = None;
            if args.kind == "repair" {
                let latest = last_one(&relevant_findings(&state, false));
                let ready = latest.iter().any(|item| {
                    item["classification"] == json!("evidence-ready")
                        && item["evidence_state"] == json!("receipt-valid")
                });
                let current = view["selected_current"]["current_edge"].clone();
                if !current.is_null() && current["verdict"] != json!("stale") {
                    issue = Some("selected edge is no longer stale; reconcile its current outcome instead of resealing".into());
                } else if !ready
                    || view["evidence_check"]["pending_count"]
                        .as_u64()
                        .unwrap_or(0)
                        > 0
                    || !view["selected_current"]["unresolved"]
                        .as_array()
                        .map(Vec::is_empty)
                        .unwrap_or(true)
                    || current.is_null()
                    || view["selected_current"]["changed_since_selection"] == json!(true)
                {
                    issue = Some("repair preparation requires a current evidence-ready judgment, valid inspected evidence and current native edge observations".into());
                }
                words = vec!["reseal".into(), from.clone(), "--on".into(), to.clone()];
            } else if args.kind == "judgment" {
                words = vec![
                    "note".into(),
                    "--on".into(),
                    from.clone(),
                    "--kind".into(),
                    "correction".into(),
                    "--reason".into(),
                    format!("STALE: {finding}"),
                ];
            }
            match issue {
                Some(message) => push_unresolved(&mut view, message),
                None => {
                    let mut argv: Vec<String> = vec!["epr".into(), "flow".into()];
                    argv.extend(words);
                    argv.extend([
                        "--root".into(),
                        args.root.to_string_lossy().to_string(),
                        "--json".into(),
                    ]);
                    if args.kind == "repair" && edge["slot"]["plane"] == json!("doc") {
                        argv = doc_repair(args, contract, &edge)?;
                    }
                    view["prepared_action"] = json!({
                        "argv": argv, "command": shell_join(&argv),
                        "standing": "Proposed action, not executed or approved by this view.",
                        "prerequisites": "Review the source-backed finding; apply existing source/hold governance. Independently review the effect before acceptance.",
                    });
                    state["next_action"] = json!("Review and execute the prepared native action if authorized, then reconcile");
                }
            }
        }
        "reconcile" => {
            let edge = selected_edge(&state)?;
            let from = edge["slot"]["from"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mut usage = view["usage"].take();
            let outcome = walk::walk(&args.root, &from)?;
            let outcome = charge_native(&mut usage, &outcome)?;
            view["usage"] = usage;
            state["last_reconciliation"] = outcome["edges"].clone();
            view["native_walk"] = outcome;
            view["meaning"] = json!("Current dependent edges are re-read. A matching seal alone does not establish review or acceptance.");
            state["next_action"] =
                json!("Inspect each affected edge and native acceptance before finishing");
        }
        "finish" => {
            let (outcome_text, question) = match (&args.outcome, &args.question) {
                (Some(o), Some(q)) => (o.clone(), q.clone()),
                _ => return Err(refused(
                    "finish takes --outcome --question; both are required, and --question names \
                     the unresolved frontier or its evidenced absence",
                )),
            };
            // A FOCUSED journey has no concern edge to select — it answered a question from
            // passages. Refusing it left the reader unable to close a session it had done the work
            // of, so the standing requirement is now: an outcome, a question, and either a selected
            // concern OR at least one inspected passage.
            let all_receipts: Vec<String> = state["evidence"]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            let focused = state["selected"].is_null();
            let keys = if focused {
                all_receipts.clone()
            } else {
                selected_evidence(&state)
            };
            if focused && keys.is_empty() {
                return Err(refused(
                    "finish needs --outcome and --question (both given) and either a selected \
                     concern or at least one inspected passage; this session has 0 receipts — \
                     read a passage first: recall read --path <p> --lines START:END",
                ));
            }
            let mut usage = view["usage"].take();
            view["evidence_check"] = revalidate(
                args,
                contract,
                &mut state,
                &mut usage,
                Some(&keys),
                true,
                Some(0),
            )?;
            view["usage"] = usage;
            evidence_continuation(args, &mut view, Some(&keys));
            if !focused {
                let edge = selected_edge(&state)?;
                view["reconciliation"] = native_context(args, &edge, &mut view, "reconciliation")?;
                let mut usage = view["usage"].take();
                view["current_edges"] = refresh_selected(args, contract, &mut state, &mut usage)?;
                view["usage"] = usage;
            }
            if has_measurements {
                view["measurement"] = measure(args, contract, &mut state, "close", method)?;
            }
            view["outcome"] = json!({
                "report": outcome_text,
                "standing": "operator/investigator report; consult native evidence for acceptance",
                "unresolved_frontier": question,
                "intent": state["intent"],
                "findings": last_one(&relevant_findings(&state, false)),
                "next_action": state["next_action"],
                "receipts": keys,
                "reconciled_edges": u64::from(!focused),
                "reconciliation_scope": if focused {
                    "No concern edge was selected or reconciled: this journey answered a question \
                     from inspected passages. The receipts above are what it stands on."
                } else {
                    "One selected concern edge was reconciled; its native standing is reported \
                     above."
                },
            });
            state["last_outcome"] = view["outcome"].clone();
            if let Some(list) = state["frontier"].as_array_mut() {
                list.push(json!(question));
            }
        }
        other => return Err(refused(format!("unknown recall operation `{other}`"))),
    }

    if view.get("measurement").is_some() {
        let usage = view["measurement"]["usage"].take();
        if let Some(map) = view["measurement"].as_object_mut() {
            map.remove("usage");
        }
        add_usage(&mut view["usage"], &usage);
    }
    if has_measurements {
        push_action(
            &mut view,
            action(
                args,
                "Inspect the original burden baseline",
                "measure",
                &[("phase", json!("baseline"))],
            ),
        );
    }
    view["orientation"] = orientation(&recipe, &state);
    push_action(
        &mut view,
        action(
            args,
            "Inspect governing recipe and alternatives",
            "recipe",
            &[("need", json!("Understand this view and its omissions"))],
        ),
    );
    push_action(
        &mut view,
        action(
            args,
            "Expand retained findings, receipts and questions",
            "history",
            &[("limit", json!(1))],
        ),
    );
    push_action(
        &mut view,
        action(
            args,
            "Resume orientation and remaining work",
            "resume",
            &[("need", json!("Continue the same intent"))],
        ),
    );
    view["next_action"] = state["next_action"].clone();
    let questions = state["frontier"].as_array().cloned().unwrap_or_default();
    view["frontier"] =
        json!({"latest": last_one(&questions), "total": questions.len(), "expand": "history"});
    execution.state["ceremony"] = state;
    Ok(view)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// CLI
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub fn usage() -> String {
    format!(
        "usage: epr flow memory recall <{}> --session <id> [--need TEXT] [--json] [--root DIR]\n\
         \x20      epr flow memory recall --adopt-receipts [--from-dir DIR] [--dry-run] [--json]\n\
         \x20      search|source accept --tag <t> (repeatable, EXACT frontmatter membership, local provider only)\n\
         \x20      --lens <path> names an EXTERNAL footprint lens script; the lens is NATIVE by default\n\
         Receipts and continuations are PRIVATE session records under {RECALL_DIR_REL}/<session>/; \
         they are never imported, projected, witnessed or targeted by feedback.",
        OPERATIONS.join("|")
    )
}

#[allow(clippy::too_many_lines)]
fn parse_args(argv: &[String]) -> FlowResult<(Args, bool, bool, Option<String>)> {
    let mut root = PathBuf::from(".");
    // `--root` is read first so every default derived from it (contract, lens) is right.
    let mut i = 0;
    let mut root_explicit = false;
    while i < argv.len() {
        if argv[i] == "--root" {
            root = PathBuf::from(
                argv.get(i + 1)
                    .ok_or_else(|| refused("--root needs a value"))?,
            );
            root_explicit = true;
        }
        i += 1;
    }
    let root =
        std::fs::canonicalize(&root).map_err(|source| FlowError::Read { path: root, source })?;
    let mut args = Args::new(root);
    args.root_explicit = root_explicit;
    let mut adopt = false;
    let mut dry_run = false;
    let mut from_dir: Option<String> = None;

    let mut i = 0;
    while i < argv.len() {
        let key = argv[i].as_str();
        if !key.starts_with("--") {
            if !args.operation.is_empty() {
                return Err(refused(format!("unexpected second operation `{key}`")));
            }
            args.operation = key.to_string();
            i += 1;
            continue;
        }
        match key {
            "--json" => {
                args.json = true;
                i += 1;
                continue;
            }
            "--adopt-receipts" => {
                adopt = true;
                i += 1;
                continue;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
                continue;
            }
            "--help" | "-h" => {
                println!("{}", usage());
                i += 1;
                continue;
            }
            _ => {}
        }
        let value = argv
            .get(i + 1)
            .ok_or_else(|| refused(format!("{key} needs a value")))?
            .clone();
        match key {
            "--root" => {}
            "--contract" => {
                args.contract_path = if Path::new(&value).is_absolute() {
                    PathBuf::from(&value)
                } else {
                    args.root.join(&value)
                };
                args.contract_explicit = true;
            }
            "--lens" => {
                args.lens = Some(if Path::new(&value).is_absolute() {
                    PathBuf::from(&value)
                } else {
                    args.root.join(&value)
                })
            }
            "--tag" => args.tags.push(value),
            "--session" => args.session = value,
            "--need" => {
                args.need = value;
                args.need_explicit = true;
            }
            "--scope" => args.scope = Some(value),
            "--intent" => args.intent = Some(value),
            "--path" => args.path = Some(value),
            "--lines" => args.lines = Some(value),
            "--finding" => args.finding = Some(value),
            "--question" => args.question = Some(value),
            "--next-action" => args.next_action = Some(value),
            "--from-session" => args.from_session = Some(value),
            "--from-dir" => from_dir = Some(value),
            "--provider" => args.provider = Some(value),
            "--query" => args.query = Some(value),
            "--outcome" => args.outcome = Some(value),
            "--section" => args.section = Some(value),
            "--context-pin" => args.context_pin = Some(value),
            "--search-scope" => args.search_scope = value,
            "--name" => args.name = value,
            "--evidence" => args.evidence.push(value),
            "--measure-scope" => args.measure_scope.push(value),
            "--actor-session" => args.actor_session = Some(value),
            "--phase" => args.phase = value,
            "--classification" => args.classification = value,
            "--kind" => args.kind = value,
            "--edge" => {
                args.edge = Some(
                    value
                        .parse()
                        .map_err(|_| refused("--edge needs a row number"))?,
                )
            }
            "--offset" => {
                args.offset = value
                    .parse()
                    .map_err(|_| refused("offsets must be nonnegative"))?
            }
            "--evidence-offset" => {
                args.evidence_offset = value
                    .parse()
                    .map_err(|_| refused("offsets must be nonnegative"))?
            }
            "--limit" => {
                args.limit = value
                    .parse()
                    .map_err(|_| refused("--limit needs a count"))?
            }
            other => {
                return Err(refused(format!(
                    "unknown flag {other}; {} takes {}",
                    if args.operation.is_empty() {
                        "recall"
                    } else {
                        &args.operation
                    },
                    accepted_flags(&args.operation)
                )))
            }
        }
        i += 2;
    }
    if !(1..=100).contains(&args.limit) {
        return Err(refused(
            "offsets must be nonnegative and limit between 1 and 100",
        ));
    }
    if !["baseline", "close"].contains(&args.phase.as_str()) {
        return Err(refused("--phase must be baseline|close"));
    }
    if ![
        "unreviewed",
        "evidence-ready",
        "conflict",
        "missing-evidence",
    ]
    .contains(&args.classification.as_str())
    {
        return Err(refused(
            "--classification must be unreviewed|evidence-ready|conflict|missing-evidence",
        ));
    }
    if !["repair", "judgment", "observation"].contains(&args.kind.as_str()) {
        return Err(refused("--kind must be repair|judgment|observation"));
    }
    for text in [
        &args.need,
        args.intent.as_deref().unwrap_or_default(),
        args.finding.as_deref().unwrap_or_default(),
        args.question.as_deref().unwrap_or_default(),
        args.outcome.as_deref().unwrap_or_default(),
    ] {
        if text.len() > 4000 {
            return Err(refused(
                "text exceeds compact context budget (4000 bytes); reference a source passage",
            ));
        }
    }
    // Nothing a recall verb accepts may name a private record: the gate lives on every path-shaped
    // input, not only on the ones that happen to be read today.
    for candidate in [
        args.path.as_deref(),
        args.scope.as_deref(),
        Some(args.search_scope.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        refuse_private_import(&args.root, candidate)?;
    }
    Ok((args, adopt, dry_run, from_dir))
}

/// Read a `--flag value` out of a raw argv without parsing the rest.
///
/// Used only to name the session in a refusal that happened BEFORE the arguments could be parsed —
/// so the remedy can still say `--from-session <this one>` rather than a placeholder.
fn take_flag(argv: &[String], flag: &str) -> Option<String> {
    argv.iter()
        .position(|a| a == flag)
        .and_then(|i| argv.get(i + 1))
        .cloned()
}

/// `epr flow memory recall …`.
pub fn run(argv: &[String]) -> FlowResult<ExitCode> {
    if argv.iter().any(|a| a == "--help" || a == "-h") && argv.len() == 1 {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    // Argument and contract refusals get the SAME structured envelope as every later refusal.
    // A caller reading `--json` should not have to parse one shape when the executor rejects a flag
    // and another when it rejects a source, and a refusal that arrives as bare stderr carries no
    // `next` line at all — which is precisely where a caller most needs one.
    let named_session = take_flag(argv, "--session").unwrap_or_default();
    // Read before parsing, because the rendering mode has to be known for a refusal the parser
    // itself produces.
    let wants_json = argv.iter().any(|a| a == "--json");
    let (args, adopt, dry_run, from_dir) = match parse_args(argv) {
        Ok(parsed) => parsed,
        Err(error) => return print_refusal(&error.to_string(), &named_session, None, wants_json),
    };
    let contract = match Contract::load(&args.contract_path) {
        Ok(contract) => contract,
        Err(error) => return print_refusal(&error.to_string(), &args.session, None, args.json),
    };
    let method = contract.method_cid();

    if adopt || args.operation == "adopt-receipts" {
        let from = match from_dir {
            Some(dir) => args.root.join(dir),
            None => args.root.join(LEGACY_RECEIPTS_REL),
        };
        let report = adopt_receipts(&args.root, &from, &method, dry_run)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(ExitCode::SUCCESS);
    }
    if args.operation.is_empty() {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    if !OPERATIONS.contains(&args.operation.as_str()) {
        let message = format!(
            "unknown recall operation `{}` — the set is {}",
            args.operation,
            OPERATIONS.join("|")
        );
        return print_refusal(&message, &args.session, None, args.json);
    }
    if args.session.is_empty() {
        return print_refusal(
            "recall needs --session <id>",
            &args.session,
            None,
            args.json,
        );
    }

    let session_limit = contract
        .value
        .pointer("/limits/session_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let output_limit = contract.limit_usize("output_bytes");

    // Resolved BEFORE the session is opened, because it is one of the two pins the session refuses
    // on. An executor that cannot read its own bytes reports no digest rather than a guess, and a
    // session with no recorded digest adopts whatever ran it first.
    let executor_digest = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| hex(&Sha256::digest(&bytes)));

    let mut execution = match Execution::open(
        &args.root,
        &args.session,
        &args.need,
        &method,
        executor_digest.as_deref(),
        session_limit,
    ) {
        Ok(execution) => execution,
        Err(error) => {
            return print_refusal(
                &error.to_string(),
                &args.session,
                Some(output_limit),
                args.json,
            );
        }
    };

    let began = Instant::now();
    let result = execute(&args, &contract, &mut execution, &method);
    let mut view = match result {
        Ok(view) => view,
        Err(error) => {
            let message = error.to_string();
            execution.state["last_error"] = json!(message);
            let _ = execution.save();
            if !args.json {
                print!(
                    "{}",
                    refusal_lines(&message, &remedy_for(&message, &args.session))
                );
                return Ok(ExitCode::from(2));
            }
            let mut failure = json!({
                "unresolved": [truncate(&message, 1000)],
                "session": args.session,
                "next": remedy_for(&message, &args.session),
                "accounting": {
                    "attempts": execution.state["attempts"],
                    "unmetered_attempts": execution.state["unmetered_attempts"],
                    "meaning": "Refused attempt retained; unmetered work is not zero cost.",
                },
            });
            if !execution.state["ceremony"].is_null() {
                failure["orientation"] =
                    orientation(contract.recipe(), &execution.state["ceremony"]);
            }
            let mut encoded = serde_json::to_string(&failure)?;
            if encoded.len() > output_limit {
                encoded = serde_json::to_string(
                    &json!({"unresolved": ["request refused; context exceeds output budget"]}),
                )?;
            }
            println!("{encoded}");
            return Ok(ExitCode::from(2));
        }
    };
    view["execution_method"] = json!({
        "method": method,
        "recall-contract.json": contract.sha256(),
        "contract_path": rel_to_root(&args.root, &contract.path),
        "executor": "epr flow memory recall",
        "native_executable": executor_digest,
        "measurement_lens": match &args.lens {
            Some(path) => json!(rel_to_root(&args.root, path)),
            None => json!(footprint::method()),
        },
    });
    add_usage(
        &mut view["usage"],
        &json!({"elapsed_seconds": (began.elapsed().as_secs_f64() * 1e6).round() / 1e6}),
    );
    let usage = view["usage"].clone();
    view["cumulative"] = execution.charge(&usage)?;

    let mut raw = encode(&view, args.json)?;
    if raw.len() > output_limit {
        let retry = match args.operation.as_str() {
            "context" | "history" | "open" => args.operation.clone(),
            _ => "open".into(),
        };
        let mut narrow = json!({
            "orientation": view["orientation"],
            "operation": view["operation"],
            "cumulative": view["cumulative"],
            "execution_method": view["execution_method"],
            "unresolved": ["view exceeds output budget; narrow page or explicit memory input selection; continuation retained"],
            "actions": [action(&args, "Open a smaller page of this view", &retry, &[
                ("limit", json!(1)), ("offset", json!(args.offset)),
                ("section", if retry == "context" { json!(args.section) } else { Value::Null }),
                ("context_pin", if retry == "context" { json!(args.context_pin) } else { Value::Null }),
            ])],
        });
        narrow["usage"] = json!({});
        raw = encode(&narrow, args.json)?;
        view = narrow;
    }
    if raw.len() > output_limit {
        raw = serde_json::to_string(&json!({
            "unresolved": ["even orientation exceeds output budget; increase declared output budget or narrow authored intent"],
            "session": args.session,
        }))? + "\n";
    }
    if raw.len() > output_limit {
        return Ok(ExitCode::from(2));
    }
    print!("{raw}");
    Ok(
        if view["unresolved"]
            .as_array()
            .map(Vec::is_empty)
            .unwrap_or(true)
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(2)
        },
    )
}

fn encode(view: &Value, json_output: bool) -> FlowResult<String> {
    Ok(if json_output {
        serde_json::to_string(view)? + "\n"
    } else {
        render(view)
    })
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}
