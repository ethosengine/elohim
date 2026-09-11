//! The bounded footprint lens, native.
//!
//! A transcription of `genesis/scripts/memory_balance.py` — the deterministic measurement the
//! recall ceremony's paired baseline/close observation is taken with. It counts files, bytes and
//! lines per declared cohort over an explicit, confined, bounded traversal, and it reports what it
//! could NOT reach as omissions rather than folding a gap into a total.
//!
//! **Why it is native rather than a path to a script.** The lens is a REPOSITORY tool, and the
//! first cut resolved it by searching the filesystem for `genesis/scripts/memory_balance.py`. That
//! ladder is the wrong shape: a cucumber fixture runs with its cwd under `/tmp` and a fixture
//! contract, so every rung misses and the operation that needs a measurement refuses. A measurement
//! primitive the ceremony depends on cannot be conditional on where the process happens to be
//! standing. `--footprint-lens <script>` survives as an EXPLICIT override for an external lens, and only as
//! that.
//!
//! **What is preserved exactly**, because these are the promises the measurement makes:
//!
//! * `complete` is false the moment ANYTHING was omitted, and a delta is refused on an incomplete
//!   pair — unknown is never zero.
//! * A file that changed while it was being read is recorded `stable: false` AND omitted, so the
//!   bytes stay in the total while the total stops claiming to be complete.
//! * Symlinks are refused, including on intermediate components, and a refused path is named.
//! * `same_content_relocations` exists so a MOVE cannot read as a shrink.
//! * The scope digest keys the pair: two snapshots taken over different cohorts or different
//!   budgets are not comparable, and `compare` says which field disagreed rather than returning a
//!   number.
//!
//! **What differs from the Python, stated rather than hidden.** The Python walks with fd-relative
//! `openat(..., O_NOFOLLOW)`, which closes the TOCTOU window on intermediate components. This
//! implementation builds each path component by component from the root and refuses any component
//! whose `symlink_metadata` says it is a symlink, using only `std::fs` — the same refusal, checked
//! one instant earlier. The sample was already documented as "sequential, not an atomic filesystem
//! snapshot"; this widens that existing window on ancestors and does not create a new class of
//! claim. `method.sha256` is this module's own bytes, so a native snapshot and a Python snapshot
//! are correctly NOT comparable: they are different methods, and `compare` refuses the pair.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::super::{FlowError, FlowResult};

/// The four declared cohorts. Closed: a category outside this set is a claim about a partition the
/// totals do not carry.
pub const CATEGORIES: [&str; 4] = ["authored", "projected", "retained", "operational"];

/// Directories that are implementation or cache rather than authored footprint. They are reported
/// as `exclusions` — declared and visible — never silently skipped.
pub const EXCLUDED: [&str; 5] = [
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".cargo-target-pool",
];

/// The measurement's identity string, versioned with the algorithm rather than with the crate.
pub const METHOD_VERSION: &str = "bounded-memory-balance-v1";

/// The three phases a sample may be taken in. `observation` is unpaired by construction.
const PHASES: [&str; 3] = ["baseline", "close", "observation"];

/// The traversal budgets. Every one of them, when it binds, produces a named omission.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_files: usize,
    pub max_bytes: usize,
    pub max_entries: usize,
    pub max_seconds: f64,
    pub max_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_files: 2000,
            max_bytes: 20_000_000,
            max_entries: 10_000,
            max_seconds: 10.0,
            max_depth: 64,
        }
    }
}

impl Limits {
    /// Read the declared limits out of a contract's `ceremony.measurements.limits`, keeping the
    /// defaults for anything it does not name.
    pub fn from_declared(declared: Option<&Value>) -> Self {
        let mut limits = Limits::default();
        let Some(map) = declared else {
            return limits;
        };
        let count = |key: &str, current: usize| {
            map.get(key)
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .unwrap_or(current)
        };
        limits.max_files = count("max_files", limits.max_files);
        limits.max_bytes = count("max_bytes", limits.max_bytes);
        limits.max_entries = count("max_entries", limits.max_entries);
        limits.max_depth = count("max_depth", limits.max_depth);
        limits.max_seconds = map
            .get("max_seconds")
            .and_then(Value::as_f64)
            .unwrap_or(limits.max_seconds);
        limits
    }

    /// The `limits` block a snapshot publishes, and half of what the scope digest is taken over.
    fn published(&self) -> Value {
        let mut excluded: Vec<&str> = EXCLUDED.to_vec();
        excluded.sort_unstable();
        json!({
            "max_files": self.max_files,
            "max_bytes": self.max_bytes,
            "max_entries": self.max_entries,
            "max_seconds": whole_or_float(self.max_seconds),
            "max_depth": self.max_depth,
            "excluded": excluded,
        })
    }

    fn validate(&self) -> FlowResult<()> {
        if !(self.max_seconds > 0.0 && self.max_seconds <= 120.0)
            || self.max_depth == 0
            || self.max_depth > 128
        {
            return Err(refused("max_seconds must be 0..120 and max_depth 1..128"));
        }
        if self.max_files == 0 || self.max_bytes == 0 || self.max_entries == 0 {
            return Err(refused("positive integer limits required"));
        }
        Ok(())
    }
}

fn refused(message: impl Into<String>) -> FlowError {
    FlowError::InvalidArguments(message.into())
}

/// Keep an integral budget spelled as an integer, because the scope DIGEST is taken over these
/// bytes: `5` and `5.0` would be two cohorts with one name.
fn whole_or_float(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The lens's own identity: what measured this, and by which rule lines were counted.
pub fn method() -> Value {
    json!({
        "version": METHOD_VERSION,
        "sha256": hex(&Sha256::digest(include_str!("footprint.rs").as_bytes())),
        "lines": "LF count plus one for nonempty unterminated final line",
        "implementation": "elohim-epr-cli flow/memory/footprint.rs",
    })
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
}

/// sha256 over the canonical (sorted-key, separator-tight) JSON encoding, matching the oracle.
fn digest_of(value: &Value) -> String {
    hex(&Sha256::digest(
        serde_json::to_string(value).unwrap_or_default().as_bytes(),
    ))
}

/// One cohort member: an explicit, confined, relative path and the partition it counts toward.
fn normalize_scope(scope: &[Value], max_entries: usize) -> FlowResult<Vec<Value>> {
    let mut normalized = Vec::new();
    for entry in scope {
        // A bare string is an authored path — the shorthand the CLI and the recipe both use.
        let (path, category) = match entry {
            Value::String(path) => (path.clone(), "authored".to_string()),
            Value::Object(map) => {
                let keys: BTreeSet<&str> = map.keys().map(String::as_str).collect();
                if keys != BTreeSet::from(["path", "category"]) {
                    return Err(refused("scope requires path and known category only"));
                }
                let category = map["category"].as_str().unwrap_or_default().to_string();
                if !CATEGORIES.contains(&category.as_str()) {
                    return Err(refused("scope requires path and known category only"));
                }
                match map["path"].as_str() {
                    Some(path) => (path.to_string(), category),
                    None => {
                        return Err(refused(
                            "scope paths must be explicit confined relative paths",
                        ))
                    }
                }
            }
            _ => return Err(refused("scope requires path and known category only")),
        };
        // Confinement is checked on the SPELLING, before anything is opened: no absolute path, no
        // `.`/`..`/empty component, no NUL. A path that has to be canonicalized to be judged safe
        // has already been resolved once by something else.
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\0')
            || path.split('/').any(|part| matches!(part, ".." | "." | ""))
        {
            return Err(refused(
                "scope paths must be explicit confined relative paths",
            ));
        }
        normalized.push(json!({"path": path, "category": category}));
    }
    if normalized.is_empty() || normalized.len() > max_entries {
        return Err(refused("scope must be nonempty and bounded by max_entries"));
    }
    Ok(normalized)
}

/// The traversal's running state, so the walk reads as one thing rather than six threaded locals.
struct Walk {
    root: PathBuf,
    limits: Limits,
    began: Instant,
    rows: Vec<Value>,
    omissions: Vec<Value>,
    exclusions: Vec<Value>,
    visited: BTreeSet<String>,
    entries: usize,
    read_bytes: usize,
}

impl Walk {
    fn omit(&mut self, path: &str, reason: &str) {
        self.omissions.push(json!({"path": path, "reason": reason}));
    }

    fn budget_spent(&self) -> bool {
        self.entries >= self.limits.max_entries
            || self.rows.len() >= self.limits.max_files
            || self.read_bytes >= self.limits.max_bytes
    }

    fn elapsed_over(&self) -> bool {
        self.began.elapsed().as_secs_f64() > self.limits.max_seconds
    }

    /// Visit one path. `relative` is the cohort-relative spelling; `absolute` is where it lives.
    fn visit(&mut self, absolute: &Path, relative: &str, category: &str) {
        // A path named twice — the same directory in two categories — is measured ONCE, under the
        // category that reached it first. Counting it twice would inflate the total by the caller's
        // spelling rather than by anything on disk.
        if !self.visited.insert(relative.to_string()) {
            return;
        }
        self.entries += 1;
        if relative.matches('/').count() > self.limits.max_depth || self.elapsed_over() {
            self.omit(relative, "depth or elapsed budget exhausted");
            return;
        }
        if self.entries > self.limits.max_entries || self.budget_spent() {
            self.omit(relative, "budget exhausted; subtree unmeasured");
            return;
        }
        let Ok(before) = absolute.symlink_metadata() else {
            self.omit(relative, "unavailable or refused: FileNotFoundError");
            return;
        };
        // A symlink is REFUSED, never followed — the oracle's `O_NOFOLLOW` open raises `OSError`
        // (ELOOP) here, and the reason string says the same thing it says.
        if before.file_type().is_symlink() {
            self.omit(relative, "unavailable or refused: OSError");
            return;
        }
        if before.is_dir() {
            self.visit_directory(absolute, relative, category, &before);
        } else if before.is_file() {
            self.visit_file(absolute, relative, category, &before);
        } else {
            self.omit(relative, "not a regular file or directory");
        }
    }

    fn visit_directory(
        &mut self,
        absolute: &Path,
        relative: &str,
        category: &str,
        before: &std::fs::Metadata,
    ) {
        let mut children: Vec<String> = Vec::new();
        let mut enumeration_cut = false;
        match std::fs::read_dir(absolute) {
            Ok(reader) => {
                for child in reader.flatten() {
                    if children.len() + self.entries >= self.limits.max_entries {
                        enumeration_cut = true;
                        break;
                    }
                    children.push(child.file_name().to_string_lossy().to_string());
                }
            }
            Err(_) => {
                self.omit(relative, "sampling failed: OSError");
                return;
            }
        }
        if enumeration_cut {
            self.omit(
                relative,
                "directory enumeration budget; remaining entries unknown",
            );
        }
        children.sort();
        for child in children {
            let child_relative = format!("{relative}/{child}");
            if EXCLUDED.contains(&child.as_str()) {
                self.exclusions.push(json!({
                    "path": child_relative,
                    "reason": "declared implementation/cache exclusion",
                }));
            } else {
                self.visit(&absolute.join(&child), &child_relative, category);
            }
            if self.budget_spent() {
                self.omit(
                    relative,
                    "remaining directory entries unmeasured after budget",
                );
                break;
            }
        }
        // A directory whose mtime moved under the walk was being written to while it was read.
        if let Ok(after) = absolute.symlink_metadata() {
            if modified_nanos(before) != modified_nanos(&after) {
                self.omit(relative, "directory changed during sampling");
            }
        }
    }

    fn visit_file(
        &mut self,
        absolute: &Path,
        relative: &str,
        category: &str,
        before: &std::fs::Metadata,
    ) {
        let remaining = self.limits.max_bytes - self.read_bytes;
        if before.len() as usize > remaining {
            self.omit(relative, "file exceeds remaining byte budget");
            return;
        }
        let mut body = Vec::new();
        match std::fs::File::open(absolute) {
            Ok(mut handle) => {
                let mut chunk = vec![0u8; 65536];
                let mut available = remaining;
                while available > 0 {
                    if self.elapsed_over() {
                        self.omit(relative, "elapsed budget exhausted while reading");
                        break;
                    }
                    let want = available.min(chunk.len());
                    match handle.read(&mut chunk[..want]) {
                        Ok(0) => break,
                        Ok(read) => {
                            body.extend_from_slice(&chunk[..read]);
                            available -= read;
                        }
                        Err(_) => {
                            self.omit(relative, "sampling failed: OSError");
                            break;
                        }
                    }
                }
            }
            Err(_) => {
                self.omit(relative, "unavailable or refused: PermissionError");
                return;
            }
        }
        self.read_bytes += body.len();
        // Stability is three facts agreeing: the size, the timestamps, and the count of bytes we
        // actually got. An unstable read keeps its bytes in the total and costs the snapshot its
        // `complete` flag — the number stays honest by admitting it might be wrong.
        let after = absolute.symlink_metadata().ok();
        let stable = after.as_ref().is_some_and(|after| {
            before.len() == after.len()
                && modified_nanos(before) == modified_nanos(after)
                && changed_nanos(before) == changed_nanos(after)
                && body.len() as u64 == after.len()
        });
        if !stable {
            self.omit(relative, "changed during sampling; observed bytes only");
        }
        let lines = body.iter().filter(|b| **b == b'\n').count()
            + usize::from(!body.is_empty() && !body.ends_with(b"\n"));
        self.rows.push(json!({
            "path": relative,
            "category": category,
            "bytes": body.len(),
            "lines": lines,
            "sha256": hex(&Sha256::digest(&body)),
            "stable": stable,
        }));
    }
}

fn modified_nanos(meta: &std::fs::Metadata) -> i128 {
    use std::os::unix::fs::MetadataExt;
    i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec())
}

fn changed_nanos(meta: &std::fs::Metadata) -> i128 {
    use std::os::unix::fs::MetadataExt;
    i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec())
}

/// Read an explicit cohort without writing anything.
///
/// Missing, refused, unstable and budget-excluded inputs make every total a LOWER BOUND, and say so
/// through `complete: false` plus a named omission per cause.
pub fn snapshot(
    root: &Path,
    run_id: &str,
    phase: &str,
    scope: &[Value],
    limits: Limits,
) -> FlowResult<Value> {
    limits.validate()?;
    if run_id.is_empty() || !PHASES.contains(&phase) {
        return Err(refused(
            "run_id and baseline/close/observation phase required",
        ));
    }
    let normalized = normalize_scope(scope, limits.max_entries)?;
    let root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;

    let started = now();
    let mut walk = Walk {
        root: root.clone(),
        limits,
        began: Instant::now(),
        rows: Vec::new(),
        omissions: Vec::new(),
        exclusions: Vec::new(),
        visited: BTreeSet::new(),
        entries: 0,
        read_bytes: 0,
    };

    for entry in &normalized {
        let relative = entry["path"].as_str().unwrap_or_default().to_string();
        let category = entry["category"].as_str().unwrap_or("authored").to_string();
        // Walk the ancestors ourselves, refusing a symlinked or non-directory component, so no
        // symlink is ever traversed on the way in either.
        let mut absolute = walk.root.clone();
        let parts: Vec<&str> = relative.split('/').collect();
        let mut ancestors_ok = true;
        for component in &parts[..parts.len().saturating_sub(1)] {
            absolute = absolute.join(component);
            match absolute.symlink_metadata() {
                Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
                Ok(_) => {
                    walk.omit(&relative, "ancestor unavailable or refused: OSError");
                    ancestors_ok = false;
                    break;
                }
                Err(_) => {
                    walk.omit(
                        &relative,
                        "ancestor unavailable or refused: FileNotFoundError",
                    );
                    ancestors_ok = false;
                    break;
                }
            }
        }
        if !ancestors_ok {
            continue;
        }
        let last = parts.last().copied().unwrap_or_default();
        walk.visit(&absolute.join(last), &relative, &category);
    }

    let mut totals = Map::new();
    for category in CATEGORIES {
        totals.insert(category.into(), json!({"files": 0, "bytes": 0, "lines": 0}));
    }
    for row in &walk.rows {
        let category = row["category"].as_str().unwrap_or("authored");
        let bucket = totals.get_mut(category).expect("declared category");
        bucket["files"] = json!(bucket["files"].as_u64().unwrap_or(0) + 1);
        bucket["bytes"] =
            json!(bucket["bytes"].as_u64().unwrap_or(0) + row["bytes"].as_u64().unwrap_or(0));
        bucket["lines"] =
            json!(bucket["lines"].as_u64().unwrap_or(0) + row["lines"].as_u64().unwrap_or(0));
    }
    let observed: Map<String, Value> = ["files", "bytes", "lines"]
        .into_iter()
        .map(|key| {
            let sum: u64 = totals.values().map(|v| v[key].as_u64().unwrap_or(0)).sum();
            (key.to_string(), json!(sum))
        })
        .collect();

    // Unique CONTENT, not unique storage: identical bytes at two paths are one payload, and that is
    // still not a claim about disk allocation.
    let mut unique: BTreeMap<String, u64> = BTreeMap::new();
    for row in &walk.rows {
        unique.insert(
            row["sha256"].as_str().unwrap_or_default().to_string(),
            row["bytes"].as_u64().unwrap_or(0),
        );
    }

    let published_limits = limits.published();
    let mut files = walk.rows.clone();
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));

    Ok(json!({
        "schema": "memory-balance-v1",
        "method": method(),
        "run_id": run_id,
        "phase": phase,
        "root": root.to_string_lossy(),
        "scope": normalized,
        "limits": published_limits,
        "scope_digest": digest_of(&json!({"scope": normalized, "limits": limits.published()})),
        "sampling_started": started,
        "sampling_finished": now(),
        "complete": walk.omissions.is_empty(),
        "omissions": walk.omissions,
        "exclusions": walk.exclusions,
        "files": files,
        "unique_content_bytes": unique.values().sum::<u64>(),
        "totals": Value::Object(totals),
        "observed": Value::Object(observed),
        "overhead": {
            "source_bytes_read": walk.read_bytes,
            "entries_examined": walk.entries,
            "elapsed_seconds": walk.began.elapsed().as_secs_f64(),
            "snapshot_serialization_bytes": Value::Null,
            "tokens": Value::Null,
        },
        "limitations": [
            "Sequential sample, not an atomic filesystem snapshot.",
            "Hard-linked physical storage is counted per path; unique content is not disk allocation.",
            "Closing artifacts written after sampling require separate accounting.",
            "Private stores and providers outside scope are unmeasured.",
            "Legacy .claude/archive is not selected by default; select an actual archive explicitly.",
        ],
    }))
}

/// Compare only explicit, complete, compatible, ordered baseline/close samples.
///
/// Every refusal here is a refusal to produce a NUMBER from evidence that cannot support one: a
/// different method, cohort or budget; an incomplete sample on either side; or windows that
/// overlap, which would let work done during the baseline read as work done between the two.
pub fn compare(baseline: &Value, close: &Value) -> Value {
    let mut errors: Vec<String> = Vec::new();
    for key in [
        "schema",
        "method",
        "run_id",
        "root",
        "scope",
        "limits",
        "scope_digest",
    ] {
        if baseline.get(key) != close.get(key) {
            errors.push(format!("{key} mismatch"));
        }
    }
    if baseline.get("phase").and_then(Value::as_str) != Some("baseline")
        || close.get("phase").and_then(Value::as_str) != Some("close")
    {
        errors.push("baseline/close phases required".into());
    }
    if !baseline["complete"].as_bool().unwrap_or(false)
        || !close["complete"].as_bool().unwrap_or(false)
    {
        errors.push("incomplete sampling; unknown is not zero".into());
    }
    if close["sampling_started"].as_str().unwrap_or("")
        < baseline["sampling_finished"].as_str().unwrap_or("")
    {
        errors.push("overlapping or reversed sampling windows".into());
    }
    if !errors.is_empty() {
        return json!({"comparable": false, "reasons": errors, "delta": Value::Null});
    }
    let rows = |sample: &Value| -> BTreeMap<String, Value> {
        sample["files"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|r| r["path"].as_str().map(|p| (p.to_string(), r.clone())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (before, after) = (rows(baseline), rows(close));
    let removed: Vec<Value> = before
        .iter()
        .filter(|(path, _)| !after.contains_key(*path))
        .map(|(_, row)| row.clone())
        .collect();
    let added: Vec<Value> = after
        .iter()
        .filter(|(path, _)| !before.contains_key(*path))
        .map(|(_, row)| row.clone())
        .collect();
    // A relocation is the same CONTENT at a new path. Naming them is what stops an archival move
    // from reading as a net shrink — the one inference this measurement exists to refuse.
    let mut candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in &removed {
        candidates
            .entry(row["sha256"].as_str().unwrap_or_default().to_string())
            .or_default()
            .push(row["path"].as_str().unwrap_or_default().to_string());
    }
    let mut relocations = Vec::new();
    for row in &added {
        let key = row["sha256"].as_str().unwrap_or_default().to_string();
        if let Some(list) = candidates.get_mut(&key) {
            if !list.is_empty() {
                relocations.push(
                    json!({"from": list.remove(0), "to": row["path"], "bytes": row["bytes"]}),
                );
            }
        }
    }
    let delta = |key: &str| {
        close["observed"][key].as_i64().unwrap_or(0)
            - baseline["observed"][key].as_i64().unwrap_or(0)
    };
    let mut partitions = Map::new();
    for category in CATEGORIES {
        let mut row = Map::new();
        for key in ["files", "bytes", "lines"] {
            row.insert(
                key.into(),
                json!(
                    close["totals"][category][key].as_i64().unwrap_or(0)
                        - baseline["totals"][category][key].as_i64().unwrap_or(0)
                ),
            );
        }
        partitions.insert(category.into(), Value::Object(row));
    }
    json!({
        "comparable": true,
        "reasons": [],
        "delta": {"files": delta("files"), "bytes": delta("bytes"), "lines": delta("lines")},
        "partitions": partitions,
        "added_paths": added,
        "removed_paths": removed,
        "same_content_relocations": relocations,
        "interpretation": "Relocation is not net shrink. Shared-tree attribution and token savings are unmeasured.",
    })
}
