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

use super::super::{
    concerns, confine_under, context, context_sections, head_commit_provenance, measures, note,
    rel_to_root, walk,
};
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

mod discovery;
use discovery::{bootstrap_projection, first_feature_path, first_screen, outline};
pub use discovery::{discover, discover_scored};

mod providers;
pub use providers::bounded_process;
use providers::{process_result, providers_for};

mod journey;
use journey::execute;

mod sample;

mod lens;
use lens::LensLevel;

mod questions;
pub use questions::{Question, ReachedWhen};

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

const OPERATIONS: [&str; 18] = [
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
    "sample",
    "judge",
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

    /// The same contract, built from an already-parsed value rather than a file on disk — for a
    /// caller that has the JSON in hand (a future in-process ceremony reuse; today, this crate's
    /// own `#[cfg(test)]` fixtures). `sha256`/`method_cid` still derive from real bytes: this
    /// re-encodes `value` to its canonical JSON rather than inventing a placeholder digest, so a
    /// contract built this way pins exactly what it would pin had it been read from a file whose
    /// bytes happened to serialize the same way. `path` has no file to name, so it carries a
    /// placeholder that says so rather than an empty or invented one.
    pub fn from_value(value: Value) -> FlowResult<Self> {
        let raw = serde_json::to_vec(&value)?;
        let contract = Contract {
            raw,
            value,
            path: PathBuf::from("<inline>"),
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

    /// The contract read as a VF knowledge-level `ProcessSpec` — the same seven-stage
    /// [`COMPOSITION`] this executor already refuses to run any other shape of, now typed rather
    /// than a bespoke JSON array. A contract carrying a declared `process_spec` deserializes it
    /// directly; an older contract (pre-v10) that only declared `composition` gets one minted from
    /// it, so `Contract::load` never fails on a contract this executor already accepts.
    pub fn process_spec(&self) -> FlowResult<elohim_epr_rea::ProcessSpec> {
        match self.value.get("process_spec") {
            // Present but malformed is a hand-edit error, not a shape to silently paper over with
            // a minted default — a caller reading a wrong `ProcessSpec` back would never learn the
            // declared one was broken.
            Some(raw) => serde_json::from_value(raw.clone()).map_err(|e| {
                refused(format!(
                    "recall contract `process_spec` is declared but malformed: {e}"
                ))
            }),
            // Genuinely absent (pre-v10 contract) is the only case the v9 fallback covers.
            None => Ok(elohim_epr_rea::ProcessSpec {
                id: self
                    .value
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("bounded-evidence-recall")
                    .to_string(),
                version: self
                    .value
                    .get("version")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                stages: COMPOSITION
                    .iter()
                    .map(|name| elohim_epr_rea::StageSpec {
                        name: (*name).to_string(),
                        artifact_kind: "Window".to_string(),
                    })
                    .collect(),
                edges: Vec::new(),
            }),
        }
    }

    /// The contract's declared budgets read as VF [`elohim_epr_rea::Bound`]s. A declared `bounds`
    /// array deserializes directly, and a malformed one is REFUSED rather than silently replaced by
    /// the folded fallback. A contract with no `bounds` key at all (pre-v10) gets one `Bound` folded
    /// from every `limits.*` entry [`validate`](Self::validate) already requires be a positive
    /// finite number, so the fallback never invents a limit the contract did not itself declare.
    pub fn bounds(&self) -> FlowResult<Vec<elohim_epr_rea::Bound>> {
        match self.value.get("bounds") {
            Some(raw) => serde_json::from_value(raw.clone()).map_err(|e| {
                refused(format!(
                    "recall contract `bounds` is declared but malformed: {e}"
                ))
            }),
            None => Ok(self
                .value
                .get("limits")
                .and_then(Value::as_object)
                .map(|limits| {
                    limits
                        .iter()
                        .filter_map(|(name, raw)| {
                            raw.as_f64().map(|limit| elohim_epr_rea::Bound {
                                limit,
                                unit: name.clone(),
                                threshold_pct: 100.0,
                                sense: None,
                                source: None,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()),
        }
    }

    /// The fixed question bank a `question_bank`-declaring contract names — each row an
    /// `elohim_epr_rea::Intent` `in_scope_of` THIS contract (`method_cid()`), read from the sibling
    /// file the contract points at (see `questions.rs` for the wire-shape/real-shape split this
    /// needs, since a `Cid` cannot deserialize from a bare JSON string).
    ///
    /// Absent `question_bank` (every pre-v12 contract) is genuinely unwired, not malformed: `Ok`
    /// with an empty bank, the same "absent means nothing to refuse about" reading `process_spec`
    /// and `bounds` give a pre-v10 contract. A DECLARED `question_bank` is different — the
    /// contract asserted a bank exists, so a value that is not a string, a file that cannot be
    /// read, or a bank whose shape or pinned `recipe`/`in_scope_of` does not match this contract's
    /// own `method_cid()` is REFUSED rather than silently treated as no bank at all.
    pub fn question_bank(&self) -> FlowResult<Vec<Question>> {
        let declared = match self.value.get("question_bank") {
            None => return Ok(Vec::new()),
            Some(value) => value,
        };
        let rel = declared.as_str().ok_or_else(|| {
            refused(format!(
                "recall contract `question_bank` is declared but malformed: expected a repository-relative path string, found {declared}"
            ))
        })?;
        // The contract and its question bank are declared as siblings under the same
        // `.epr-meta/elohim/algorithms/` directory — `CONTRACT_REL`'s own directory depth tells us
        // how many ancestors of `self.path` to climb to reach the repository root this contract
        // was loaded relative to. A contract built via `from_value` (no real file; `path` is the
        // `<inline>` placeholder) or loaded from a path that does not mirror `CONTRACT_REL`'s
        // shape cannot resolve a root this way, and gets a clear refusal rather than a panic or a
        // silently wrong join.
        let depth = Path::new(CONTRACT_REL).components().count();
        let root = self.path.ancestors().nth(depth).ok_or_else(|| {
            refused(
                "recall contract `question_bank` is declared, but this contract was not loaded from its usual repository-relative location, so the bank path cannot be resolved",
            )
        })?;
        let path = root.join(rel);
        let raw = std::fs::read(&path).map_err(|source| {
            refused(format!(
                "recall contract `question_bank` names {}, which cannot be read: {source}",
                path.display()
            ))
        })?;
        questions::parse_bank(&raw, &path, &self.method_cid())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Test support — a minimally valid contract for this crate's own isolated unit tests
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Fixture support for this crate's own `#[cfg(test)]` unit tests (`providers.rs`'s
/// `LocalLexical`/`MemPalace` tests, so far). The integration test binaries under `tests/` keep
/// their own `contract_value` builder in `tests/common/mod.rs` rather than depending on a
/// `cfg(test)` module of the library crate — the two are the same shape by construction, not by
/// one calling the other.
#[cfg(test)]
pub mod tests_support {
    use serde_json::{json, Value};

    /// The live algorithm artifact, narrowed to a `source_roots` of `["."]` rather than this
    /// repository's own `docs/`-rooted fixture convention — so a bare `tempfile::tempdir()` is
    /// itself a legal scope, which is what every isolated filesystem unit test wants to hand a
    /// provider. `ceremony.measurements` is stripped the same way the integration fixture strips
    /// it: nothing here exercises the paired footprint lens.
    pub fn minimal_contract() -> Value {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let raw = std::fs::read(repo_root.join(super::CONTRACT_REL)).expect("live contract");
        let mut contract: Value = serde_json::from_slice(&raw).expect("contract parses");
        contract["source_roots"] = json!(["."]);
        contract["ceremony"]["defaults"]["scope"] = json!(".");
        if let Some(ceremony) = contract["ceremony"].as_object_mut() {
            ceremony.remove("measurements");
        }
        contract
    }
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
    /// `--purpose bootstrap`: with no `--need` and no `--intent`, `open` mints the session's own
    /// intent from the register's top red habit instead of the recipe's generic purpose — see
    /// `discovery::bootstrap_projection`. No other value is accepted.
    pub purpose: Option<String>,
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
    pub footprint_lens: Option<PathBuf>,
    /// The reader lens level explicitly requested with `--lens <level>`. `None` means resolve it
    /// from the actor sidecar's claim and this reader's own revealed evidence — see `lens.rs`.
    ///
    /// Module-private (not `pub`), unlike every other `Args` field: `LensLevel` is itself
    /// `pub(super)` (recall-scoped), and every reader of this field is inside `recall`'s own
    /// module tree — `Args` never crosses that boundary today. A `pub` field of a `pub(super)`
    /// type is `private_interfaces`-denied under `-D warnings`.
    lens: Option<LensLevel>,
    /// Exact metadata tags a discovery must carry, repeatable. Every named tag must be present.
    pub tags: Vec<String>,
    /// Whether `--root`/`--contract` were named, so linked next actions repeat only real overrides.
    pub root_explicit: bool,
    pub contract_explicit: bool,
    /// `sample --reader agent:<role>@<model>` — the claimed identity whose journey is sampled.
    pub reader: Option<String>,
    /// `judge --event <cid>` — the sampled `FlowEvent`'s own record CID.
    pub event: Option<String>,
    /// `judge --as <seat>` — the second seat rendering the verdict; refused when it equals the
    /// sampled event's own `provider` (a reader never judges its own journey).
    pub seat: Option<String>,
    /// `judge --mistaken <n>` — the count of assertions the seat found wrong.
    pub mistaken: Option<i64>,
    /// `judge --reason <text>` — the seat's one-line justification, carried as the witness summary.
    pub reason: Option<String>,
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
            purpose: None,
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
            footprint_lens: None,
            lens: None,
            tags: Vec::new(),
            root_explicit: false,
            contract_explicit: false,
            reader: None,
            event: None,
            seat: None,
            mistaken: None,
            reason: None,
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
// CLI
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub fn usage() -> String {
    format!(
        "usage: epr flow memory recall <{}> --session <id> [--need TEXT] [--json] [--root DIR]\n\
         \x20      epr flow memory recall open --session <id> --purpose bootstrap; with no --need, \
mints the session's intent from the register's own top red habit\n\
         \x20      epr flow memory recall --adopt-receipts [--from-dir DIR] [--dry-run] [--json]\n\
         \x20      search|source accept --tag <t> (repeatable, EXACT frontmatter membership, local provider only)\n\
         \x20      --lens <level> widens/narrows the READER lens (minimal|simple|standard|detail|debug|trace); \
resolved and printed on every view when omitted\n\
         \x20      --footprint-lens <path> names an EXTERNAL footprint-measurement lens script; native by default\n\
         \x20      epr flow memory recall sample --question <id> --reader agent:<role>@<model> --session <id>; \
runs the bank question's journey in-process (open, read, finish) and folds it as one FlowEvent\n\
         \x20      epr flow memory recall judge --event <cid> --as <seat> --mistaken <n> --reason <text>; \
a second seat's verdict over a sampled journey, refused when the seat is the journey's own reader\n\
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
            "--footprint-lens" => {
                args.footprint_lens = Some(if Path::new(&value).is_absolute() {
                    PathBuf::from(&value)
                } else {
                    args.root.join(&value)
                })
            }
            "--lens" => {
                args.lens = Some(LensLevel::parse(&value).ok_or_else(|| {
                    refused("--lens must be one of minimal|simple|standard|detail|debug|trace")
                })?)
            }
            "--tag" => args.tags.push(value),
            "--session" => args.session = value,
            "--need" => {
                args.need = value;
                args.need_explicit = true;
            }
            "--scope" => args.scope = Some(value),
            "--intent" => args.intent = Some(value),
            "--purpose" => args.purpose = Some(value),
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
            "--reader" => args.reader = Some(value),
            "--event" => args.event = Some(value),
            "--as" => args.seat = Some(value),
            "--mistaken" => {
                args.mistaken = Some(
                    value
                        .parse()
                        .map_err(|_| refused("--mistaken needs a nonnegative count"))?,
                )
            }
            "--reason" => args.reason = Some(value),
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
    if args.mistaken.is_some_and(|n| n < 0) {
        return Err(refused("--mistaken needs a nonnegative count"));
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
    if args.purpose.as_deref().is_some_and(|p| p != "bootstrap") {
        return Err(refused("--purpose must be bootstrap"));
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
    // `judge` rules on an already-recorded `FlowEvent`; it neither opens nor advances a ceremony
    // continuation, so it never takes the session lock the rest of this function exists to hold
    // — and, unlike every operation below, it names no session of its own (the seat claims
    // nothing), so this intercept sits BEFORE the `--session` requirement, not after it.
    if args.operation == "judge" {
        return sample::judge(&args, &contract);
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
    // `sample` composes `open`/`read`/`finish` in-process over the SAME locked session, so it
    // slots in exactly where a single `execute()` call would — the accounting, honesty floor and
    // encoding below apply to its aggregate view precisely as they do to any other operation's.
    let result = if args.operation == "sample" {
        sample::sample(&args, &contract, &mut execution, &method)
    } else {
        execute(&args, &contract, &mut execution, &method)
    };
    let (mut view, resolved_lens) = match result {
        Ok(pair) => pair,
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
                    journey::orientation(contract.recipe(), &execution.state["ceremony"]);
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
        "measurement_lens": match &args.footprint_lens {
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

    // The honesty floor is a constant, never read from a flag or the contract — one instance
    // covers every rendering this call produces, the narrow fallback below included.
    let render_floor = lens::RenderFloor::declared();
    let mut raw = encode(&view, &resolved_lens, &render_floor, args.json)?;
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
        raw = encode(&narrow, &resolved_lens, &render_floor, args.json)?;
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

fn encode(
    view: &Value,
    lens: &lens::LensView,
    floor: &lens::RenderFloor,
    json_output: bool,
) -> FlowResult<String> {
    Ok(if json_output {
        serde_json::to_string(view)? + "\n"
    } else {
        render(view, lens, floor)
    })
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod shape {
    /// Every `.rs` file in this seam stays under the soft line ceiling.
    ///
    /// Fix round (2026-09-12, F6): the list used to be ELEVEN hardcoded filenames, so a twelfth
    /// module could be added to this directory and never be measured — the one failure mode a
    /// shape test exists to prevent. `read_dir` makes the seam itself the population.
    #[test]
    fn no_recall_module_exceeds_the_soft_line_ceiling() {
        let seam = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/flow/memory/recall"
        ));
        let mut measured = 0usize;
        for entry in std::fs::read_dir(seam).expect("recall seam directory reads") {
            let path = entry.expect("directory entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("utf-8 filename")
                .to_string();
            let text = std::fs::read_to_string(&path).expect("module reads");
            assert!(
                text.lines().count() <= 1800,
                "{name} is over the module ceiling"
            );
            measured += 1;
        }
        // An empty walk would pass vacuously — the same silence the hardcoded list produced.
        assert!(
            measured >= 11,
            "only {measured} recall modules measured — the seam walk found less than the eleven \
             that existed when this test was written; a walk that stops finding modules is a \
             broken measure, not a clean one"
        );
    }
}
