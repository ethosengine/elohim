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

mod discovery;
pub use discovery::{discover, discover_scored};
use discovery::{first_screen, outline};

mod providers;
pub use providers::bounded_process;
use providers::{process_result, providers_for};

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
            let provider = match args
                .provider
                .clone()
                .or_else(|| state["provider"].as_str().map(str::to_string))
            {
                Some(provider) => provider,
                // Neither the CLI nor a resumed session named one: consult the pinned recipe's
                // own declared providers, in their own order, and settle on the first that
                // actually returns something — the recipe has always declared `local` first and
                // it has always been what this repository has installed, so this is the same
                // default as before, now read from the declaration instead of hard-coded. Each
                // provider actually asked is charged for real, same as any other probe of source
                // bytes; the loop stops asking as soon as one answers, so an unconsulted provider
                // (mempalace, once local already found something) is never charged at all.
                None => {
                    let mut chosen: Option<String> = None;
                    for candidate in providers_for(contract) {
                        let Ok(found) = candidate.candidates(
                            &[],
                            Path::new(&args.search_scope),
                            contract,
                            &args.root,
                        ) else {
                            continue;
                        };
                        add_usage(&mut view["usage"], &found.usage);
                        if !found.ranked.is_empty() {
                            chosen = Some(candidate.id());
                            break;
                        }
                    }
                    chosen.unwrap_or_else(|| "local".to_string())
                }
            };
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
