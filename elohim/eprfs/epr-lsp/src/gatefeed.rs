//! Gate failures as editor diagnostics: the `gate-live` external feed.
//!
//! A pre-push or `just gate` run appends its failure-level lines to a gate-live run log,
//! `<main checkout>/.eprfs/status/gate-live/runs/<run>.jsonl` (`genesis/orchestrator/gate-live.mjs`).
//! The device mux tails that directory and publishes each failure that names a source locus to
//! the root that holds the file, through [`ExternalFeed`], under the source tag [`SOURCE`]. Every
//! session attached to that root then sees it like any other diagnostic: in full on a file it
//! opened or edited, folded into its area otherwise.
//!
//! Nothing is decided from these diagnostics. They are Ephemeral (C): device-local, never
//! notarized or projected, and rebuilt from the run log whenever the mux starts (every run
//! inside the gate-live stale window is replayed). The gate's exit status stays the verdict.
//!
//! The rules this module keeps:
//! - Placement. The run log names a locus as the tool printed it, usually relative to the gate's
//!   own directory. A run is placed in the checkout(s) whose HEAD is the run's candidate commit
//!   (read once, when the run opens), and a relative locus resolves the way `gate-tap.mjs`'s
//!   attributor does: against the gate's directory in that checkout, then the checkout itself. A
//!   locus that resolves nowhere is not shown.
//! - Roots. A placed file is published to the checkout root that contains it (the root an
//!   `epr lsp attach` names by default) and to every other running server root that also
//!   contains it. A root with no running server drops the set rather than queueing it: with no
//!   editor attached nobody is waiting, and the run log still holds the failure. When the set of
//!   running servers changes, every held set is published again, so a root that starts later
//!   still sees the current failures.
//! - Languages. Only `.rs` loci map to a language (`rust`) today; any other locus is counted and
//!   reported in one log line when the run stops being read, never guessed at.
//! - Replacement. A project's diagnostics belong to the newest run (run ids sort by their
//!   timestamp) that started that project's gate. They stay after the run closes, and the next
//!   run's `gate-start` for the same project replaces them, so a green re-run clears them. An
//!   older run never overwrites a newer one's set.
//! - Bounds. At most [`RUN_CAP`] failures are shown per run, at most [`URI_CAP`] diagnostics per
//!   URI and root, at most [`READ_CAP`] bytes are read per poll across every run, and a line
//!   longer than [`LINE_CAP`] is dropped. A run stops being read at its `run-close` or once its
//!   file is older than [`STALE`].

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::daemon::{log, ExternalFeed};
use crate::socket::DaemonRecord;
use crate::uri::path_to_uri;

/// The source tag every gate diagnostic carries (`source` and `data.epr.source`).
pub const SOURCE: &str = "gate-live";
/// How often the runs directory is polled: a metadata read per run file, no inotify.
pub const POLL: Duration = Duration::from_secs(1);
/// A run whose file has not changed for this long is abandoned (`STALE_MS` in gate-live.mjs).
pub const STALE: Duration = Duration::from_secs(6 * 60 * 60);
/// The most failures shown from one run (`MAX_FAILURES` in gate-failure-lines.mjs).
pub const RUN_CAP: usize = 200;
/// The most gate diagnostics published for one URI under one root.
pub const URI_CAP: usize = 50;
/// The most run-log bytes read in one poll, across every run.
pub const READ_CAP: u64 = 1 << 20;
/// The longest run-log line kept; a v1 event's excerpt is bounded at 2048 bytes, so a longer
/// line is damage, not a failure.
pub const LINE_CAP: usize = 64 * 1024;
/// The longest summary taken from a failure's excerpt, in characters.
const SUMMARY_CHARS: usize = 160;

/// One run-log event this feed reads. Every other kind is ignored.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    /// The run opened: the session that started it and its candidate commit.
    Open {
        /// The session, when the writer knew it.
        session: Option<String>,
        /// The candidate commit, when the writer had one.
        sha: Option<String>,
    },
    /// A gate project started in this run.
    GateStart {
        /// The gate project.
        project: String,
    },
    /// A failure that names a file.
    Failure(GateFailure),
    /// The run closed.
    Close,
}

/// A failure line with a file locus, as the run log states it.
#[derive(Debug, Clone, PartialEq)]
pub struct GateFailure {
    /// The run id.
    pub run: String,
    /// The gate project.
    pub project: String,
    /// The file, as the tool printed it (relative or absolute).
    pub file: String,
    /// The 1-based line, when printed.
    pub line: Option<u32>,
    /// The 1-based column, when printed.
    pub col: Option<u32>,
    /// The first line of the excerpt, bounded.
    pub summary: String,
    /// The recognizer's class (`test`, `compile`, `lint`, `format`, `env`).
    pub class: Option<String>,
    /// The recognizer's fingerprint.
    pub fingerprint: Option<String>,
    /// The gate's attribution label (`mine`, `other`, `ambiguous`, `unattributed`).
    pub attribution: String,
    /// The sessions the attribution names.
    pub sessions: Vec<String>,
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn number(value: &Value, pointer: &str) -> Option<u32> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}

/// Parse one run-log line. `None` for a damaged line, a kind this feed does not read, or a
/// failure with no file in its locus (nothing to place it on).
#[must_use]
pub fn parse_line(line: &str) -> Option<RunEvent> {
    let event: Value = serde_json::from_str(line.trim()).ok()?;
    match event.get("kind").and_then(Value::as_str)? {
        "run-open" => Some(RunEvent::Open {
            session: text(&event, "/session"),
            sha: text(&event, "/candidate/sha"),
        }),
        "gate-start" => Some(RunEvent::GateStart {
            project: text(&event, "/project")?,
        }),
        "run-close" => Some(RunEvent::Close),
        "failure" => parse_failure(&event).map(RunEvent::Failure),
        _ => None,
    }
}

fn parse_failure(event: &Value) -> Option<GateFailure> {
    let failure = event.get("failure")?;
    let file = text(failure, "/locus/file")?;
    // gate-tap.mjs nests the attribution in the failure; the v1 schema also allows it beside.
    let attribution = failure
        .get("attribution")
        .or_else(|| event.get("attribution"));
    let excerpt = failure
        .get("excerpt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let first = excerpt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(SUMMARY_CHARS).collect::<String>());
    let summary = first
        .or_else(|| text(failure, "/locus/test"))
        .or_else(|| text(failure, "/class").map(|class| format!("{class} failure")))
        .unwrap_or_else(|| "gate failure".to_owned());
    Some(GateFailure {
        run: text(event, "/run").unwrap_or_default(),
        project: text(event, "/project")?,
        file,
        line: number(failure, "/locus/line"),
        col: number(failure, "/locus/col"),
        summary,
        class: text(failure, "/class"),
        fingerprint: text(failure, "/fingerprint"),
        attribution: attribution
            .and_then(|a| text(a, "/label"))
            .unwrap_or_else(|| "unattributed".to_owned()),
        sessions: attribution
            .and_then(|a| a.get("sessions"))
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The language server a locus belongs to: only `.rs` maps today (`rust`).
#[must_use]
pub fn lang_of(file: &str) -> Option<&'static str> {
    match Path::new(file).extension().and_then(|e| e.to_str()) {
        Some("rs") => Some("rust"),
        _ => None,
    }
}

/// The LSP diagnostic for a failure of the run `session` started. The run log's line and column
/// are 1-based; LSP's are 0-based. With no column the range covers the whole line, and with no
/// line the first.
#[must_use]
pub fn diagnostic(failure: &GateFailure, session: Option<&str>) -> Value {
    let line = failure.line.map_or(0, |l| l - 1);
    let character = failure.col.map_or(0, |c| c - 1);
    json!({
        "range": {
            "start": {"line": line, "character": character},
            "end": {"line": line + 1, "character": 0}
        },
        "severity": 1,
        "source": SOURCE,
        "message": format!("{} [gate {}]", failure.summary, failure.project),
        "data": {"epr": {"gate": {
            "run": failure.run,
            "project": failure.project,
            "attribution": failure.attribution,
            "sessions": failure.sessions,
            "session": session,
            "class": failure.class,
            "fingerprint": failure.fingerprint
        }}}
    })
}

/// Splits appended bytes into complete lines. A torn last line waits for its newline; a line
/// past [`LINE_CAP`] is dropped whole.
#[derive(Debug, Default)]
pub struct RunLines {
    partial: Vec<u8>,
    dropping: bool,
}

impl RunLines {
    /// Feed newly appended bytes; return every line they complete.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for chunk in bytes.split_inclusive(|b| *b == b'\n') {
            let complete = chunk.last() == Some(&b'\n');
            let body = if complete {
                &chunk[..chunk.len() - 1]
            } else {
                chunk
            };
            if !self.dropping {
                if self.partial.len() + body.len() > LINE_CAP {
                    self.partial.clear();
                    self.dropping = true;
                } else {
                    self.partial.extend_from_slice(body);
                }
            }
            if complete {
                if !self.dropping {
                    if let Ok(line) = std::str::from_utf8(&self.partial) {
                        lines.push(line.to_owned());
                    }
                }
                self.partial.clear();
                self.dropping = false;
            }
        }
        lines
    }

    /// Forget a torn line, as when the file is replaced.
    pub fn reset(&mut self) {
        self.partial.clear();
        self.dropping = false;
    }
}

/// Where a run's files are: which checkouts ran it, and where a gate project lives in one.
pub trait Placer: Send + Sync {
    /// The checkouts whose HEAD is `sha`, as canonical paths. Empty when none is, or no sha.
    fn checkouts(&self, sha: Option<&str>) -> Vec<PathBuf>;
    /// `project`'s gate directory relative to `checkout` (`gate.projects.<name>.dir`).
    fn project_dir(&self, checkout: &Path, project: &str) -> Option<PathBuf>;
}

/// What the tailer reads and where it publishes.
pub struct GateFeedConfig {
    /// `<main checkout>/.eprfs/status/gate-live/runs`.
    pub runs_dir: PathBuf,
    /// The device mux's runtime directory, whose server records name the running roots.
    pub runtime_dir: PathBuf,
    /// Places a run's loci.
    pub placer: Arc<dyn Placer>,
    /// The device mux's feed.
    pub feed: ExternalFeed,
}

impl std::fmt::Debug for GateFeedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateFeedConfig")
            .field("runs_dir", &self.runs_dir)
            .finish_non_exhaustive()
    }
}

/// Tail the runs directory until the device mux stops. Spawned by the device mux only.
pub async fn tail(config: GateFeedConfig) {
    let mut tail = Tail::new(config);
    let mut tick = tokio::time::interval(POLL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if !tail.poll().await {
            return;
        }
    }
}

/// `(lang, root, uri)`: one published set.
type Target = (String, PathBuf, String);

/// One run being read.
#[derive(Debug)]
struct RunFile {
    id: String,
    offset: u64,
    lines: RunLines,
    session: Option<String>,
    checkouts: Vec<PathBuf>,
    dirs: HashMap<(PathBuf, String), Option<PathBuf>>,
    shown: usize,
    not_rust: usize,
    unplaced: usize,
    over_cap: usize,
}

/// A failure placed on a file.
#[derive(Debug)]
struct Placed {
    lang: &'static str,
    path: PathBuf,
    diagnostic: Value,
}

/// A project's current set: the newest run that started its gate, and that run's failures.
#[derive(Debug)]
struct Shown {
    run: String,
    placed: Vec<Placed>,
}

struct Tail {
    config: GateFeedConfig,
    files: BTreeMap<PathBuf, RunFile>,
    finished: BTreeSet<PathBuf>,
    shown: BTreeMap<String, Shown>,
    records: BTreeSet<OsString>,
    running: Vec<(String, PathBuf)>,
    published: BTreeMap<Target, Vec<Value>>,
    dirty: bool,
}

impl Tail {
    fn new(config: GateFeedConfig) -> Tail {
        Tail {
            config,
            files: BTreeMap::new(),
            finished: BTreeSet::new(),
            shown: BTreeMap::new(),
            records: BTreeSet::new(),
            running: Vec::new(),
            published: BTreeMap::new(),
            dirty: false,
        }
    }

    /// One poll. `false` once the device mux has stopped.
    async fn poll(&mut self) -> bool {
        let roots_moved = self.refresh_running();
        let mut names: Vec<PathBuf> = match std::fs::read_dir(&self.config.runs_dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|e| e == "jsonl"))
                .collect(),
            Err(_) => Vec::new(),
        };
        names.sort();
        // gate-live prunes old runs at each open; forget what it removed.
        let present: BTreeSet<&PathBuf> = names.iter().collect();
        self.finished.retain(|path| present.contains(path));
        self.files.retain(|path, _| present.contains(path));

        let mut budget = READ_CAP;
        for path in &names {
            if self.finished.contains(path) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(path) else {
                continue;
            };
            let stale = meta
                .modified()
                .ok()
                .and_then(|at| SystemTime::now().duration_since(at).ok())
                .is_some_and(|age| age > STALE);
            if stale {
                // Abandoned: whatever it already showed stays, as for a closed run.
                self.finish(path);
                continue;
            }
            if budget == 0 {
                continue;
            }
            let read = self.read(path, meta.len(), budget).await;
            budget = budget.saturating_sub(read);
        }
        if self.dirty || roots_moved {
            self.dirty = false;
            return self.publish(roots_moved).await;
        }
        true
    }

    /// Read up to `budget` new bytes of one run and apply its events; returns the bytes read.
    async fn read(&mut self, path: &Path, len: u64, budget: u64) -> u64 {
        let file = self
            .files
            .entry(path.to_path_buf())
            .or_insert_with(|| RunFile {
                id: path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                offset: 0,
                lines: RunLines::default(),
                session: None,
                checkouts: Vec::new(),
                dirs: HashMap::new(),
                shown: 0,
                not_rust: 0,
                unplaced: 0,
                over_cap: 0,
            });
        if len < file.offset {
            // Replaced under the same name: read it again from the top.
            file.offset = 0;
            file.lines.reset();
        }
        if len == file.offset {
            return 0;
        }
        let Ok(mut handle) = tokio::fs::File::open(path).await else {
            return 0;
        };
        if handle.seek(SeekFrom::Start(file.offset)).await.is_err() {
            return 0;
        }
        let mut bytes = Vec::new();
        let want = (len - file.offset).min(budget);
        let Ok(read) = (&mut handle).take(want).read_to_end(&mut bytes).await else {
            return 0;
        };
        file.offset += read as u64;
        let events: Vec<RunEvent> = file
            .lines
            .feed(&bytes)
            .iter()
            .filter_map(|line| parse_line(line))
            .collect();
        for event in events {
            if self.apply(path, event).await {
                self.finish(path);
                break;
            }
        }
        read as u64
    }

    /// Apply one event of the run at `path`; `true` when the run closed.
    async fn apply(&mut self, path: &Path, event: RunEvent) -> bool {
        match event {
            RunEvent::Open { session, sha } => {
                let placer = Arc::clone(&self.config.placer);
                let checkouts =
                    tokio::task::spawn_blocking(move || placer.checkouts(sha.as_deref()))
                        .await
                        .unwrap_or_default();
                if let Some(file) = self.files.get_mut(path) {
                    file.session = session;
                    file.checkouts = checkouts;
                }
                false
            }
            RunEvent::GateStart { project } => {
                if let Some(run) = self.files.get(path).map(|f| f.id.clone()) {
                    self.claim(&project, &run);
                }
                false
            }
            RunEvent::Failure(failure) => {
                self.failure(path, failure).await;
                false
            }
            RunEvent::Close => true,
        }
    }

    /// Whether `run` holds `project`'s set, taking it from any older run.
    fn claim(&mut self, project: &str, run: &str) -> bool {
        match self.shown.get(project) {
            Some(shown) if shown.run.as_str() == run => true,
            Some(shown) if shown.run.as_str() > run => false,
            previous => {
                if previous.is_some_and(|s| !s.placed.is_empty()) {
                    self.dirty = true;
                }
                self.shown.insert(
                    project.to_owned(),
                    Shown {
                        run: run.to_owned(),
                        placed: Vec::new(),
                    },
                );
                true
            }
        }
    }

    async fn failure(&mut self, path: &Path, failure: GateFailure) {
        let Some(run) = self.files.get(path).map(|f| f.id.clone()) else {
            return;
        };
        if !self.claim(&failure.project, &run) {
            return;
        }
        let Some(file) = self.files.get_mut(path) else {
            return;
        };
        if file.shown >= RUN_CAP {
            file.over_cap += 1;
            return;
        }
        let Some(lang) = lang_of(&failure.file) else {
            file.not_rust += 1;
            return;
        };
        let paths = self.place(path, &failure).await;
        let Some(file) = self.files.get_mut(path) else {
            return;
        };
        if paths.is_empty() {
            file.unplaced += 1;
            return;
        }
        file.shown += 1;
        let diagnostic = diagnostic(&failure, file.session.as_deref());
        if let Some(shown) = self.shown.get_mut(&failure.project) {
            for placed in paths {
                shown.placed.push(Placed {
                    lang,
                    path: placed,
                    diagnostic: diagnostic.clone(),
                });
            }
            self.dirty = true;
        }
    }

    /// The canonical files a failure's locus names: absolute as printed, else under the gate's
    /// directory in each checkout that ran it, then under the checkout itself.
    async fn place(&mut self, path: &Path, failure: &GateFailure) -> Vec<PathBuf> {
        let printed = PathBuf::from(&failure.file);
        if printed.is_absolute() {
            return printed.canonicalize().into_iter().collect();
        }
        let Some(checkouts) = self.files.get(path).map(|f| f.checkouts.clone()) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for checkout in checkouts {
            let key = (checkout.clone(), failure.project.clone());
            let cached = self.files.get(path).and_then(|f| f.dirs.get(&key).cloned());
            let dir = match cached {
                Some(dir) => dir,
                None => {
                    let placer = Arc::clone(&self.config.placer);
                    let (at, project) = key.clone();
                    let dir =
                        tokio::task::spawn_blocking(move || placer.project_dir(&at, &project))
                            .await
                            .unwrap_or_default();
                    if let Some(file) = self.files.get_mut(path) {
                        file.dirs.insert(key, dir.clone());
                    }
                    dir
                }
            };
            let candidates = dir
                .map(|dir| checkout.join(dir).join(&printed))
                .into_iter()
                .chain(std::iter::once(checkout.join(&printed)));
            if let Some(hit) = candidates
                .filter_map(|candidate| candidate.canonicalize().ok())
                .find(|candidate| candidate.is_file())
            {
                if !found.contains(&hit) {
                    found.push(hit);
                }
            }
        }
        found
    }

    /// Stop reading a run, saying once what it could not show.
    fn finish(&mut self, path: &Path) {
        if let Some(file) = self.files.remove(path) {
            let skipped = file.not_rust + file.unplaced + file.over_cap;
            if skipped > 0 {
                log(&format!(
                    "gate-live run {}: {} shown; not shown: {} outside a rust file, {} placed \
                     nowhere, {} past the {RUN_CAP} cap",
                    file.id, file.shown, file.not_rust, file.unplaced, file.over_cap
                ));
            }
        }
        self.finished.insert(path.to_path_buf());
    }

    /// Re-read the running roots when the server records change; `true` when they did.
    fn refresh_running(&mut self) -> bool {
        let names: BTreeSet<OsString> = std::fs::read_dir(&self.config.runtime_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name())
                    .filter(|name| {
                        let name = name.to_string_lossy();
                        name.ends_with(".json") && !name.starts_with("attrib-")
                    })
                    .collect()
            })
            .unwrap_or_default();
        if names == self.records {
            return false;
        }
        self.running = names
            .iter()
            .filter_map(|name| std::fs::read(self.config.runtime_dir.join(name)).ok())
            .filter_map(|bytes| serde_json::from_slice::<DaemonRecord>(&bytes).ok())
            .map(|record| (record.lang, PathBuf::from(record.root)))
            .collect();
        self.records = names;
        true
    }

    /// The roots a placed file is published under: its checkout root, and every running root
    /// of its language that contains it.
    fn roots_of(&self, lang: &str, path: &Path) -> BTreeSet<PathBuf> {
        let mut roots: BTreeSet<PathBuf> = path
            .ancestors()
            .skip(1)
            .find(|dir| dir.join(".git").exists())
            .map(Path::to_path_buf)
            .into_iter()
            .collect();
        for (running_lang, root) in &self.running {
            if running_lang == lang && path.starts_with(root) {
                roots.insert(root.clone());
            }
        }
        roots
    }

    /// Publish every set that changed (all of them when `all`), and an empty set for each that
    /// is gone. `false` once the device mux has stopped.
    async fn publish(&mut self, all: bool) -> bool {
        let mut next: BTreeMap<Target, Vec<Value>> = BTreeMap::new();
        for shown in self.shown.values() {
            for placed in &shown.placed {
                let uri = path_to_uri(&placed.path);
                for root in self.roots_of(placed.lang, &placed.path) {
                    let list = next
                        .entry((placed.lang.to_owned(), root, uri.clone()))
                        .or_default();
                    if list.len() < URI_CAP {
                        list.push(placed.diagnostic.clone());
                    }
                }
            }
        }
        let mut sends: Vec<(Target, Vec<Value>)> = next
            .iter()
            .filter(|(target, list)| all || self.published.get(*target) != Some(*list))
            .map(|(target, list)| (target.clone(), list.clone()))
            .collect();
        sends.extend(
            self.published
                .keys()
                .filter(|target| !next.contains_key(*target))
                .map(|target| (target.clone(), Vec::new())),
        );
        for ((lang, root, uri), list) in sends {
            if !self
                .config
                .feed
                .publish(&lang, &root, SOURCE, &uri, list)
                .await
            {
                return false;
            }
        }
        self.published = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure_line(locus: &Value) -> String {
        json!({
            "v": 1, "run": "20261026T100000Z-abc-000001", "seq": 3,
            "at": "2026-10-26T10:00:03.000Z", "kind": "failure", "project": "eprfs",
            "failure": {
                "class": "compile", "fingerprint": "0123456789ab", "locus": locus, "nth": 1,
                "excerpt": "error[E0308]: mismatched types\n  --> epr-lsp/src/mux.rs:12:5",
                "attribution": {"label": "other", "sessions": ["bbbbbbbb-2222"]}
            }
        })
        .to_string()
    }

    #[test]
    fn a_failure_with_a_locus_becomes_a_zero_based_error_on_its_line() {
        let line = failure_line(&json!({"file": "epr-lsp/src/mux.rs", "line": 12, "col": 5}));
        let Some(RunEvent::Failure(failure)) = parse_line(&line) else {
            panic!("a failure with a file parses");
        };
        assert_eq!(failure.file, "epr-lsp/src/mux.rs");
        assert_eq!((failure.line, failure.col), (Some(12), Some(5)));
        assert_eq!(lang_of(&failure.file), Some("rust"));
        let d = diagnostic(&failure, Some("aaaaaaaa-1111"));
        assert_eq!(d["range"]["start"], json!({"line": 11, "character": 4}));
        assert_eq!(d["range"]["end"], json!({"line": 12, "character": 0}));
        assert_eq!(d["severity"], 1);
        assert_eq!(d["source"], SOURCE);
        assert_eq!(d["message"], "error[E0308]: mismatched types [gate eprfs]");
        let gate = &d["data"]["epr"]["gate"];
        assert_eq!(gate["run"], "20261026T100000Z-abc-000001");
        assert_eq!(gate["project"], "eprfs");
        assert_eq!(gate["attribution"], "other");
        assert_eq!(gate["sessions"], json!(["bbbbbbbb-2222"]));
        assert_eq!(gate["session"], "aaaaaaaa-1111");
    }

    #[test]
    fn a_failure_without_a_line_covers_the_first_line_and_a_top_level_attribution_is_read() {
        let mut event: Value =
            serde_json::from_str(&failure_line(&json!({"file": "src/lib.rs"}))).unwrap();
        let attribution = event["failure"]
            .as_object_mut()
            .unwrap()
            .remove("attribution")
            .unwrap();
        event["attribution"] = attribution;
        let Some(RunEvent::Failure(failure)) = parse_line(&event.to_string()) else {
            panic!("parses");
        };
        assert_eq!(failure.attribution, "other");
        let d = diagnostic(&failure, None);
        assert_eq!(d["range"]["start"], json!({"line": 0, "character": 0}));
        assert_eq!(d["data"]["epr"]["gate"]["session"], Value::Null);
    }

    #[test]
    fn a_failure_without_a_file_is_nothing_to_show() {
        assert_eq!(parse_line(&failure_line(&json!({}))), None);
        assert_eq!(parse_line(&failure_line(&json!({"test": "a::b"}))), None);
    }

    #[test]
    fn a_locus_outside_rust_is_skipped_not_guessed() {
        assert_eq!(lang_of("app/src/main.ts"), None);
        assert_eq!(lang_of("Cargo.toml"), None);
    }

    #[test]
    fn open_start_and_close_parse_and_other_kinds_are_ignored() {
        let open = json!({"v": 1, "run": "r", "seq": 0, "at": "x", "kind": "run-open",
            "candidate": {"sha": "abc1234", "target": "dev"}, "session": "s1"});
        assert_eq!(
            parse_line(&open.to_string()),
            Some(RunEvent::Open {
                session: Some("s1".into()),
                sha: Some("abc1234".into())
            })
        );
        let start = json!({"v": 1, "run": "r", "seq": 1, "kind": "gate-start", "project": "p"});
        assert_eq!(
            parse_line(&start.to_string()),
            Some(RunEvent::GateStart {
                project: "p".into()
            })
        );
        let close = json!({"v": 1, "run": "r", "seq": 9, "kind": "run-close", "outcome": "failed"});
        assert_eq!(parse_line(&close.to_string()), Some(RunEvent::Close));
        let verdict = json!({"v": 1, "run": "r", "seq": 8, "kind": "gate-verdict", "project": "p"});
        assert_eq!(parse_line(&verdict.to_string()), None);
        assert_eq!(parse_line("{\"kind\": \"run-cl"), None);
    }

    #[test]
    fn a_torn_line_waits_for_its_newline_and_an_oversized_one_is_dropped() {
        let mut lines = RunLines::default();
        let close = "{\"kind\":\"run-close\"}";
        assert!(lines.feed(&close.as_bytes()[..7]).is_empty());
        assert_eq!(lines.feed(&close.as_bytes()[7..]), Vec::<String>::new());
        assert_eq!(lines.feed(b"\n"), vec![close.to_owned()]);
        let huge = vec![b'x'; LINE_CAP + 1];
        assert!(lines.feed(&huge).is_empty());
        assert_eq!(lines.feed(b"tail\nnext\n"), vec!["next".to_owned()]);
    }
}
