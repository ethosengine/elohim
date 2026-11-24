//! `epr flow reading append` and `epr flow report reading` — the shadow per-gate reading.
//!
//! Gate callers collect rows and hand one JSONL file to the native append verb. This module owns
//! the record shape, its lock and its bounded device-local retention; callers never append the
//! durable file themselves. The report puts closure movement, outcome and time beside each other
//! without feeding any of them back into selection, reuse or the closure key.
//!
//! # Known limits
//!
//! The numbers are shadow telemetry, read for direction, not as accounting:
//! - `key_changed` compares with the last *recorded* green reading of that step on this device. A
//!   dropped batch (lock contention, a timed-out append, a stale `epr`) can make it wrong either
//!   way, and it is `unknown` on a first run and after retention trims the last green row.
//! - `ran` seconds exclude the cost lookup, the input shadow and final evidence recording, while
//!   `reused` seconds include them (gate-runner.mjs), so ran and reused time are not like for like.
//! - Pre-push leg seconds are whole seconds (bash `SECONDS`).
//! - A torn tail (a write cut off before its newline) is joined by the next append and the joined
//!   line fails to parse, so the next row can be lost with it.
//! - An ordinary skipped row records near-zero time and can still be named the step that set the
//!   finish; only rows synthesized at the hook's exit are excluded.
//! - The on/off decision-equivalence test covers one non-reusable fixture, not every path.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use elohim_device_runtime::flock::Flock;
use serde::{Deserialize, Serialize};

use super::measures::{default_measures, MeasureRef, Registry};
use super::{FlowError, FlowResult};

pub const MEASURE: &str = "gate-step-reading@1";
pub const READINGS_REL: &str = ".eprfs/status/readings/gate-step.jsonl";
const LOCK_FILE: &str = "gate-step.lock";
const DEFAULT_LAST_RUNS: usize = 10;
const TRIM_ABOVE_BYTES: u64 = 2 * 1024 * 1024;
const TRIM_TO_BYTES: usize = 1024 * 1024;
const MAX_ROW_BYTES: usize = 64 * 1024;
const LOCK_RETRY_FOR: Duration = Duration::from_millis(250);
const LOCK_RETRY_EVERY: Duration = Duration::from_millis(10);

/// Printed above every report so nobody reads the numbers as more than they are (see the module's
/// Known limits).
const KNOWN_LIMITS: [&str; 4] = [
    "shadow telemetry: direction, not accounting. key_changed is against the last RECORDED green on this device (a dropped batch can make it wrong either way; unknown on a first run).",
    "ran seconds exclude cost lookup, input shadow and evidence recording; reused seconds include them. Hook legs are whole seconds.",
    "a torn tail can swallow the next row; an ordinary skip can still be named the finish-setter.",
    "decision equivalence is tested on one non-reusable fixture only.",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum KeyChanged {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Ran,
    Reused,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Green,
    Red,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadingRow {
    measure: String,
    value: f64,
    unit: String,
    run_id: String,
    step: String,
    /// The step this row is a part of (a recipe step names its gate project). A child row lies
    /// inside its parent's span, so a run's sums, gaps and finish-setter read parents alone.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    parent: Option<String>,
    key: String,
    key_changed: KeyChanged,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    key_changed_writer: Option<KeyChanged>,
    status: Status,
    outcome: Outcome,
    waited_seconds: f64,
    lease_waited_seconds: f64,
    synthesized: bool,
    started_at: String,
    ended_at: String,
    at: String,
}

impl ReadingRow {
    fn validate(&self, declared_unit: &str) -> Result<(), String> {
        if self.measure != MEASURE {
            return Err(format!("measure must be `{MEASURE}`"));
        }
        if self.unit != declared_unit {
            return Err(format!("unit must be `{declared_unit}`"));
        }
        if self.run_id.trim().is_empty() || self.step.trim().is_empty() {
            return Err("run_id and step must be non-empty".into());
        }
        if let Some(parent) = &self.parent {
            if parent.trim().is_empty() || parent.trim() == self.step.trim() {
                return Err("parent must be non-empty and name another step".into());
            }
        }
        if !self.value.is_finite() || self.value < 0.0 {
            return Err("value must be a finite non-negative number".into());
        }
        if !self.waited_seconds.is_finite() || self.waited_seconds < 0.0 {
            return Err("waited_seconds must be a finite non-negative number".into());
        }
        if !self.lease_waited_seconds.is_finite() || self.lease_waited_seconds < 0.0 {
            return Err("lease_waited_seconds must be a finite non-negative number".into());
        }
        let status_matches = matches!(
            (self.outcome, self.status),
            (Outcome::Ran, Status::Green | Status::Red)
                | (Outcome::Reused, Status::Green)
                | (Outcome::Skipped, Status::None)
        );
        if !status_matches {
            return Err("status must match the row outcome".into());
        }
        if self.synthesized && self.outcome != Outcome::Skipped {
            return Err("only a skipped row may be synthesized".into());
        }
        let started =
            parse_time(&self.started_at).ok_or_else(|| "started_at must be RFC3339".to_string())?;
        let ended =
            parse_time(&self.ended_at).ok_or_else(|| "ended_at must be RFC3339".to_string())?;
        if ended < started {
            return Err("ended_at must not precede started_at".into());
        }
        parse_time(&self.at).ok_or_else(|| "at must be RFC3339".to_string())?;
        Ok(())
    }

    fn observation(&self) -> Option<ReadingObservation> {
        Some(ReadingObservation {
            run_id: self.run_id.trim().to_string(),
            step: self.step.trim().to_string(),
            parent: self
                .parent
                .as_deref()
                .map(|parent| parent.trim().to_string()),
            key: self.key.trim().to_string(),
            key_changed: self.key_changed,
            outcome: self.outcome,
            seconds: nonnegative(self.value),
            waited_seconds: nonnegative(self.waited_seconds),
            lease_waited_seconds: nonnegative(self.lease_waited_seconds),
            synthesized: self.synthesized,
            started_at: parse_time(&self.started_at)?,
            ended_at: parse_time(&self.ended_at)?,
        })
    }
}

#[derive(Debug, Clone)]
struct ReadingObservation {
    run_id: String,
    step: String,
    parent: Option<String>,
    key: String,
    key_changed: KeyChanged,
    outcome: Outcome,
    seconds: f64,
    waited_seconds: f64,
    lease_waited_seconds: f64,
    synthesized: bool,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendSummary {
    pub accepted: usize,
    pub rejected: usize,
    /// Valid rows not written because the device-local store lock stayed busy past its retry.
    pub dropped: usize,
    pub trimmed: bool,
    pub path: PathBuf,
}

/// The device's one reading store: under the main checkout, so every linked worktree a push or
/// gate runs from appends to, and is read from, the same file. A worktree's `.git` file names its
/// private git directory, whose `commondir` names the shared one; the main checkout is that
/// directory's parent. Anything unreadable keeps `root` itself.
fn reading_path(root: &Path) -> PathBuf {
    device_root(root).join(READINGS_REL)
}

fn device_root(root: &Path) -> PathBuf {
    let dot_git = root.join(".git");
    if !dot_git.is_file() {
        return root.to_path_buf();
    }
    let Some(gitdir) = fs::read_to_string(&dot_git).ok().and_then(|text| {
        text.trim()
            .strip_prefix("gitdir:")
            .map(|path| PathBuf::from(path.trim()))
    }) else {
        return root.to_path_buf();
    };
    let gitdir = if gitdir.is_absolute() {
        gitdir
    } else {
        root.join(gitdir)
    };
    let Ok(common) = fs::read_to_string(gitdir.join("commondir")) else {
        return root.to_path_buf();
    };
    let common = PathBuf::from(common.trim());
    let common = if common.is_absolute() {
        common
    } else {
        gitdir.join(common)
    };
    match common.canonicalize() {
        Ok(common) if common.file_name().is_some_and(|name| name == ".git") => common
            .parent()
            .map_or_else(|| root.to_path_buf(), Path::to_path_buf),
        _ => root.to_path_buf(),
    }
}

fn declared_unit(root: &Path) -> FlowResult<String> {
    let path = default_measures(root);
    let registry = Registry::open(&path)?;
    let measure = MeasureRef::parse(MEASURE, &path)?;
    if !registry.declares(&measure) {
        return Err(FlowError::InvalidArguments(format!(
            "{MEASURE} is not declared in {}",
            path.display()
        )));
    }
    registry.declared_unit(&measure).ok_or_else(|| {
        FlowError::InvalidArguments(format!("{MEASURE} declares no unit in {}", path.display()))
    })
}

fn read_rows(path: &Path) -> FlowResult<Vec<ReadingRow>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok().and_then(|text| serde_json::from_str(&text).ok()))
        .collect())
}

fn derive_key_changed(rows: &mut [ReadingRow], previous: Option<&[ReadingRow]>) {
    let mut latest_green = previous.map(|stored| {
        stored
            .iter()
            .filter(|row| row.status == Status::Green && !row.key.starts_with("none:"))
            .map(|row| (row.step.clone(), row.key.clone()))
            .collect::<BTreeMap<_, _>>()
    });
    for row in rows {
        row.key_changed_writer = Some(row.key_changed);
        row.key_changed = if row.key.starts_with("none:") || latest_green.is_none() {
            KeyChanged::Unknown
        } else {
            match latest_green.as_ref().and_then(|green| green.get(&row.step)) {
                Some(key) if key == &row.key => KeyChanged::No,
                Some(_) => KeyChanged::Yes,
                None => KeyChanged::Unknown,
            }
        };
        if row.status == Status::Green && !row.key.starts_with("none:") {
            if let Some(green) = latest_green.as_mut() {
                green.insert(row.step.clone(), row.key.clone());
            }
        }
    }
}

fn trim(path: &Path) -> FlowResult<bool> {
    if fs::metadata(path)?.len() <= TRIM_ABOVE_BYTES {
        return Ok(false);
    }
    let bytes = fs::read(path)?;
    let mut held = 0usize;
    let mut lines = Vec::new();
    for (index, line) in bytes
        .split_inclusive(|byte| *byte == b'\n')
        .rev()
        .enumerate()
    {
        if index > 0 && held + line.len() > TRIM_TO_BYTES {
            break;
        }
        held += line.len();
        lines.push(line);
    }
    lines.reverse();
    let dir = path.parent().ok_or_else(|| {
        FlowError::InvalidArguments(format!("{} has no parent directory", path.display()))
    })?;
    let mut staged = tempfile::NamedTempFile::new_in(dir)?;
    for line in lines {
        staged.write_all(line)?;
    }
    staged.flush()?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(true)
}

pub fn append(root: &Path, from: &Path) -> FlowResult<AppendSummary> {
    let unit = declared_unit(root)?;
    let source = fs::File::open(from).map_err(|source| FlowError::Read {
        path: from.to_path_buf(),
        source,
    })?;
    let mut accepted = Vec::new();
    let mut rejected = 0;
    for line in BufReader::new(source).lines() {
        let row = line.ok().and_then(|text| {
            (text.len() <= MAX_ROW_BYTES)
                .then(|| serde_json::from_str::<ReadingRow>(&text).ok())
                .flatten()
        });
        match row {
            Some(row) if row.validate(&unit).is_ok() => accepted.push(row),
            _ => rejected += 1,
        }
    }

    let path = reading_path(root);
    let dir = path.parent().expect("the reading path has a parent");
    fs::create_dir_all(dir)?;
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))?;
    let deadline = Instant::now() + LOCK_RETRY_FOR;
    let mut available = Some(lock_file);
    let mut held = None;
    while let Some(file) = available.take() {
        match Flock::try_exclusive(file)? {
            Ok(lock) => held = Some(lock),
            Err(file) if Instant::now() < deadline => {
                available = Some(file);
                thread::sleep(LOCK_RETRY_EVERY);
            }
            Err(_) => break,
        }
    }
    let Some(_lock) = held else {
        // The verb reports this drop once and exits non-zero; the library never prints.
        return Ok(AppendSummary {
            accepted: 0,
            rejected,
            dropped: accepted.len(),
            trimmed: false,
            path,
        });
    };

    let previous = read_rows(&path).ok();
    derive_key_changed(&mut accepted, previous.as_deref());
    if !accepted.is_empty() {
        let mut output = OpenOptions::new().create(true).append(true).open(&path)?;
        for row in &accepted {
            serde_json::to_writer(&mut output, row)?;
            output.write_all(b"\n")?;
        }
        output.flush()?;
        output.sync_all()?;
    }
    let trimmed = path.exists() && trim(&path)?;
    Ok(AppendSummary {
        accepted: accepted.len(),
        rejected,
        dropped: 0,
        trimmed,
        path,
    })
}

fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

fn nonnegative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn elapsed_seconds(start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
    nonnegative((end - start).num_milliseconds() as f64 / 1000.0)
}

#[derive(Debug, Clone)]
pub struct ReadingOptions {
    pub since: Option<DateTime<Utc>>,
    pub last_runs: Option<usize>,
}

impl Default for ReadingOptions {
    fn default() -> Self {
        Self {
            since: None,
            last_runs: Some(DEFAULT_LAST_RUNS),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingWindow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_runs: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepSeconds {
    pub step: String,
    pub run_count: usize,
    pub changed_seconds: f64,
    pub unchanged_seconds: f64,
    pub unknown_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunFold {
    pub run_id: String,
    pub steps: usize,
    pub wall_clock_seconds: f64,
    pub sum_step_seconds: f64,
    pub executing_seconds: f64,
    pub observed_waited_seconds: f64,
    pub lease_waited_seconds: f64,
    pub unaccounted_seconds: f64,
    pub waiting_seconds: f64,
    pub finish_setting_step: String,
    pub started_at: String,
    pub ended_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingReport {
    pub command: String,
    pub measure: String,
    pub window: ReadingWindow,
    pub rows: usize,
    pub runs: usize,
    pub seconds_by_step: Vec<StepSeconds>,
    pub ran_with_unchanged_key: Vec<String>,
    pub run_folds: Vec<RunFold>,
    pub gap_accounting: String,
    pub no_key_no_reason: Vec<String>,
}

pub fn report(root: &Path, options: &ReadingOptions) -> FlowResult<ReadingReport> {
    let rows = read_rows(&reading_path(root))?
        .iter()
        .filter_map(ReadingRow::observation)
        .collect::<Vec<_>>();
    Ok(fold(rows, options))
}

fn fold(mut rows: Vec<ReadingObservation>, options: &ReadingOptions) -> ReadingReport {
    if let Some(since) = options.since {
        rows.retain(|row| row.ended_at >= since);
    }

    let mut newest_by_run = BTreeMap::<String, DateTime<Utc>>::new();
    for row in &rows {
        newest_by_run
            .entry(row.run_id.clone())
            .and_modify(|held| *held = (*held).max(row.ended_at))
            .or_insert(row.ended_at);
    }
    if let Some(last) = options.last_runs {
        let mut newest = newest_by_run.into_iter().collect::<Vec<_>>();
        newest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let admitted = newest
            .into_iter()
            .take(last)
            .map(|(run, _)| run)
            .collect::<BTreeSet<_>>();
        rows.retain(|row| admitted.contains(&row.run_id));
    }

    let mut by_step = BTreeMap::<String, StepSeconds>::new();
    let mut step_runs = BTreeMap::<String, BTreeSet<String>>::new();
    let mut ran_unchanged = BTreeSet::new();
    let mut no_key_no_reason = BTreeSet::new();
    let mut by_run = BTreeMap::<String, Vec<&ReadingObservation>>::new();
    for row in &rows {
        let step = by_step
            .entry(row.step.clone())
            .or_insert_with(|| StepSeconds {
                step: row.step.clone(),
                run_count: 0,
                changed_seconds: 0.0,
                unchanged_seconds: 0.0,
                unknown_seconds: 0.0,
            });
        step_runs
            .entry(row.step.clone())
            .or_default()
            .insert(row.run_id.clone());
        match row.key_changed {
            KeyChanged::Yes => step.changed_seconds += row.seconds,
            KeyChanged::No => step.unchanged_seconds += row.seconds,
            KeyChanged::Unknown => step.unknown_seconds += row.seconds,
        }
        if row.outcome == Outcome::Ran && row.key_changed == KeyChanged::No {
            ran_unchanged.insert(row.step.clone());
        }
        let missing_reason = row.key.is_empty()
            || row
                .key
                .strip_prefix("none:")
                .is_some_and(|reason| reason.trim().is_empty());
        if missing_reason {
            no_key_no_reason.insert(row.step.clone());
        }
        // A synthesized fill sets no span, and a child row is already inside its parent's span:
        // counting it would add its seconds twice and could name it the finish-setter.
        if !row.synthesized && row.parent.is_none() {
            by_run.entry(row.run_id.clone()).or_default().push(row);
        }
    }

    for (name, runs) in step_runs {
        if let Some(step) = by_step.get_mut(&name) {
            step.run_count = runs.len();
        }
    }

    let mut seconds_by_step = by_step.into_values().collect::<Vec<_>>();
    seconds_by_step.sort_by(|a, b| {
        b.unchanged_seconds
            .total_cmp(&a.unchanged_seconds)
            .then_with(|| a.step.cmp(&b.step))
    });

    let mut run_folds = by_run
        .into_iter()
        .map(|(run_id, mut run_rows)| {
            run_rows.sort_by_key(|row| (row.started_at, row.ended_at, row.step.clone()));
            let started_at = run_rows
                .iter()
                .map(|row| row.started_at)
                .min()
                .expect("a grouped run has at least one row");
            let ended_at = run_rows
                .iter()
                .map(|row| row.ended_at)
                .max()
                .expect("a grouped run has at least one row");
            let executing_seconds = run_rows.iter().map(|row| row.seconds).sum::<f64>();
            let observed_waited_seconds =
                run_rows.iter().map(|row| row.waited_seconds).sum::<f64>();
            let lease_waited_seconds = run_rows
                .iter()
                .map(|row| row.lease_waited_seconds)
                .sum::<f64>();
            let mut execution_frontier = run_rows[0].ended_at;
            let mut unaccounted_seconds = 0.0;
            for row in run_rows.iter().skip(1) {
                if row.started_at >= execution_frontier {
                    unaccounted_seconds += elapsed_seconds(execution_frontier, row.started_at);
                }
                execution_frontier = execution_frontier.max(row.ended_at);
            }
            let finish_setting_step = run_rows
                .iter()
                .max_by(|a, b| {
                    a.ended_at
                        .cmp(&b.ended_at)
                        .then_with(|| a.seconds.total_cmp(&b.seconds))
                        .then_with(|| b.step.cmp(&a.step))
                })
                .expect("a grouped run has at least one row")
                .step
                .clone();
            RunFold {
                run_id,
                steps: run_rows.len(),
                wall_clock_seconds: elapsed_seconds(started_at, ended_at),
                sum_step_seconds: executing_seconds,
                executing_seconds,
                observed_waited_seconds,
                lease_waited_seconds,
                unaccounted_seconds,
                waiting_seconds: observed_waited_seconds + lease_waited_seconds,
                finish_setting_step,
                started_at: started_at.to_rfc3339(),
                ended_at: ended_at.to_rfc3339(),
            }
        })
        .collect::<Vec<_>>();
    run_folds.sort_by(|a, b| {
        b.ended_at
            .cmp(&a.ended_at)
            .then_with(|| a.run_id.cmp(&b.run_id))
    });

    ReadingReport {
        command: "flow report reading".into(),
        measure: MEASURE.into(),
        window: ReadingWindow {
            since: options.since.map(|time| time.to_rfc3339()),
            last_runs: options.last_runs,
        },
        rows: rows.len(),
        runs: run_folds.len(),
        seconds_by_step,
        ran_with_unchanged_key: ran_unchanged.into_iter().collect(),
        run_folds,
        gap_accounting: "gaps between sequential, non-overlapping steps are unaccounted, never inferred as waiting; an overlapping step extends the execution frontier and contributes no gap".into(),
        no_key_no_reason: no_key_no_reason.into_iter().collect(),
    }
}

impl ReadingReport {
    pub fn render(&self) {
        for line in KNOWN_LIMITS {
            println!("{line}");
        }
        println!(
            "reading  {} rows across {} runs ({})",
            self.rows,
            self.runs,
            self.window.since.as_deref().map_or_else(
                || format!("last {} runs", self.window.last_runs.unwrap_or(0)),
                |since| format!("since {since}")
            )
        );
        println!("seconds by step and key movement:");
        for step in &self.seconds_by_step {
            println!(
                "  {}  runs={}  changed={:.3}  unchanged={:.3}  unknown={:.3}",
                step.step,
                step.run_count,
                step.changed_seconds,
                step.unchanged_seconds,
                step.unknown_seconds
            );
        }
        println!(
            "ran with unchanged key ({}): {}",
            self.ran_with_unchanged_key.len(),
            list_or_none(&self.ran_with_unchanged_key)
        );
        println!(
            "runs — wall vs summed steps; finish-setting step; waiting, unaccounted and executing:"
        );
        println!("  gap accounting: {}", self.gap_accounting);
        for run in &self.run_folds {
            println!(
                "  {}  wall={:.3}  steps={:.3}  finish={}  waiting={:.3} (observed={:.3}, lease={:.3})  unaccounted={:.3}  executing={:.3}",
                run.run_id,
                run.wall_clock_seconds,
                run.sum_step_seconds,
                run.finish_setting_step,
                run.waiting_seconds,
                run.observed_waited_seconds,
                run.lease_waited_seconds,
                run.unaccounted_seconds,
                run.executing_seconds
            );
        }
        println!(
            "steps with no key and no reason ({}): {}",
            self.no_key_no_reason.len(),
            list_or_none(&self.no_key_no_reason)
        );
    }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.join(", ")
    }
}

pub fn parse_since(raw: &str) -> FlowResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| {
            FlowError::InvalidArguments(format!(
                "--since `{raw}` is not RFC3339 (for example 2026-11-24T12:00:00Z)"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared_root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join(".claude/epr-meta");
        fs::create_dir_all(&registry).unwrap();
        fs::write(
            registry.join("measures.yaml"),
            "measures-version: 1\nmeasures:\n  - id: gate-step-reading\n    version: 1\n    unit: seconds\n",
        )
        .unwrap();
        root
    }

    fn stored_row(run_id: &str, step: &str, key: &str) -> ReadingRow {
        ReadingRow {
            measure: MEASURE.into(),
            value: 1.0,
            unit: "seconds".into(),
            run_id: run_id.into(),
            step: step.into(),
            parent: None,
            key: key.into(),
            key_changed: KeyChanged::Unknown,
            key_changed_writer: None,
            status: Status::Green,
            outcome: Outcome::Ran,
            waited_seconds: 0.0,
            lease_waited_seconds: 0.0,
            synthesized: false,
            started_at: "2026-11-24T12:00:00Z".into(),
            ended_at: "2026-11-24T12:00:01Z".into(),
            at: "2026-11-24T12:00:01Z".into(),
        }
    }

    fn write_source(path: &Path, rows: &[ReadingRow]) {
        let mut output = fs::File::create(path).unwrap();
        for row in rows {
            serde_json::to_writer(&mut output, row).unwrap();
            output.write_all(b"\n").unwrap();
        }
    }

    #[test]
    fn append_validates_each_row_and_keeps_the_rest() {
        let root = declared_root();
        let source = root.path().join("rows.jsonl");
        let valid =
            serde_json::to_string(&stored_row("run", "quoted", "say \"hi\" \\ path")).unwrap();
        let bad_outcome = valid.replace("\"outcome\":\"ran\"", "\"outcome\":\"maybe\"");
        let bad_unit = valid.replace("\"unit\":\"seconds\"", "\"unit\":\"minutes\"");
        fs::write(&source, format!("{valid}\n{bad_outcome}\n{bad_unit}\n")).unwrap();

        let summary = append(root.path(), &source).unwrap();
        assert_eq!(summary.accepted, 1);
        assert_eq!(summary.rejected, 2);
        let rows = read_rows(&reading_path(root.path())).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "say \"hi\" \\ path");
        assert_eq!(rows[0].key_changed, KeyChanged::Unknown);
        assert_eq!(rows[0].key_changed_writer, Some(KeyChanged::Unknown));
    }

    fn append_rows(root: &tempfile::TempDir, rows: &[ReadingRow]) -> Vec<ReadingRow> {
        let source = root.path().join("rows.jsonl");
        write_source(&source, rows);
        append(root.path(), &source).unwrap();
        read_rows(&reading_path(root.path())).unwrap()
    }

    #[test]
    fn key_changed_is_changed_from_the_latest_green_key() {
        let root = declared_root();
        append_rows(&root, &[stored_row("green-a", "step", "key-a")]);
        let mut changed = stored_row("green-b", "step", "key-b");
        changed.key_changed = KeyChanged::No;
        let rows = append_rows(&root, &[changed]);
        assert_eq!(rows[1].key_changed, KeyChanged::Yes);
        assert_eq!(rows[1].key_changed_writer, Some(KeyChanged::No));
    }

    #[test]
    fn key_changed_is_unchanged_from_the_latest_green_key() {
        let root = declared_root();
        append_rows(&root, &[stored_row("green-a", "step", "key-a")]);
        let rows = append_rows(&root, &[stored_row("green-a-2", "step", "key-a")]);
        assert_eq!(rows[1].key_changed, KeyChanged::No);
    }

    #[test]
    fn red_in_between_does_not_become_the_key_changed_baseline() {
        let root = declared_root();
        append_rows(&root, &[stored_row("green-a", "step", "key-a")]);
        let mut red = stored_row("red-b", "step", "key-b");
        red.status = Status::Red;
        append_rows(&root, &[red]);
        let rows = append_rows(&root, &[stored_row("green-a-2", "step", "key-a")]);
        assert_eq!(rows[1].key_changed, KeyChanged::Yes);
        assert_eq!(rows[2].key_changed, KeyChanged::No);
    }

    #[test]
    fn first_run_has_unknown_key_changed() {
        let root = declared_root();
        let rows = append_rows(&root, &[stored_row("first", "step", "key-a")]);
        assert_eq!(rows[0].key_changed, KeyChanged::Unknown);
    }

    #[test]
    fn none_key_always_has_unknown_key_changed() {
        let root = declared_root();
        append_rows(&root, &[stored_row("green", "step", "key-a")]);
        let rows = append_rows(
            &root,
            &[stored_row("none", "step", "none: closure unavailable")],
        );
        assert_eq!(rows[1].key_changed, KeyChanged::Unknown);
    }

    #[test]
    fn failed_green_lookup_overrides_writer_with_unknown_key_changed() {
        let mut current = stored_row("current", "step", "key-a");
        current.key_changed = KeyChanged::No;
        derive_key_changed(std::slice::from_mut(&mut current), None);
        assert_eq!(current.key_changed, KeyChanged::Unknown);
        assert_eq!(current.key_changed_writer, Some(KeyChanged::No));
    }

    #[test]
    fn append_rejects_a_row_above_the_size_bound() {
        let root = declared_root();
        let source = root.path().join("rows.jsonl");
        write_source(
            &source,
            &[stored_row("large", "step", &"x".repeat(MAX_ROW_BYTES))],
        );
        let summary = append(root.path(), &source).unwrap();
        assert_eq!(summary.accepted, 0);
        assert_eq!(summary.rejected, 1);
    }

    #[test]
    fn append_retries_then_drops_a_contended_store_lock() {
        let root = declared_root();
        let source = root.path().join("rows.jsonl");
        write_source(&source, &[stored_row("run", "step", "key")]);
        let dir = reading_path(root.path()).parent().unwrap().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(LOCK_FILE))
            .unwrap();
        let _held = Flock::try_exclusive(file).unwrap().expect("free lock");

        let started = Instant::now();
        let summary = append(root.path(), &source).unwrap();
        assert!(started.elapsed() >= LOCK_RETRY_FOR);
        assert_eq!(summary.accepted, 0);
        assert_eq!(
            summary.dropped, 1,
            "the contended batch is counted as dropped"
        );
        assert!(!reading_path(root.path()).exists());
    }

    #[test]
    fn append_atomically_trims_over_two_mib_to_the_newest_one_mib() {
        let root = declared_root();
        let source = root.path().join("rows.jsonl");
        let rows = (0..700)
            .map(|index| {
                stored_row(
                    "trim-run",
                    &format!("step-{index:04}"),
                    &format!("key-{index:04}-{}", "x".repeat(4096)),
                )
            })
            .collect::<Vec<_>>();
        write_source(&source, &rows);

        let summary = append(root.path(), &source).unwrap();
        assert!(summary.trimmed);
        let path = reading_path(root.path());
        assert!(fs::metadata(&path).unwrap().len() <= TRIM_TO_BYTES as u64);
        let kept = read_rows(&path).unwrap();
        assert!(kept.iter().any(|row| row.step == "step-0699"));
        assert!(kept.iter().all(|row| row.step != "step-0000"));
    }

    #[test]
    fn trim_never_removes_the_newest_row() {
        let root = declared_root();
        let path = reading_path(root.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let newest = format!("{}\n", "x".repeat(TRIM_ABOVE_BYTES as usize + 1));
        fs::write(&path, &newest).unwrap();
        assert!(trim(&path).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), newest);
    }

    fn row(
        run_id: &str,
        step: &str,
        key: &str,
        key_changed: KeyChanged,
        outcome: Outcome,
        timing: (f64, f64, &str, &str),
    ) -> ReadingObservation {
        let (seconds, waited_seconds, started_at, ended_at) = timing;
        ReadingObservation {
            run_id: run_id.into(),
            step: step.into(),
            parent: None,
            key: key.into(),
            key_changed,
            outcome,
            seconds,
            waited_seconds,
            lease_waited_seconds: 0.0,
            synthesized: false,
            started_at: parse_time(started_at).unwrap(),
            ended_at: parse_time(ended_at).unwrap(),
        }
    }

    #[test]
    fn fixture_observations_compute_all_three_folds_and_missing_key_reasons() {
        let report = fold(
            vec![
                row(
                    "run-1",
                    "alpha",
                    "key-a",
                    KeyChanged::No,
                    Outcome::Ran,
                    (4.0, 1.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:05Z"),
                ),
                row(
                    "run-1",
                    "beta",
                    "key-b",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (3.0, 0.0, "2026-11-24T10:00:07Z", "2026-11-24T10:00:10Z"),
                ),
                row(
                    "run-1",
                    "broken",
                    "none:",
                    KeyChanged::Unknown,
                    Outcome::Skipped,
                    (0.0, 2.0, "2026-11-24T10:00:10Z", "2026-11-24T10:00:12Z"),
                ),
                row(
                    "run-2",
                    "alpha",
                    "key-c",
                    KeyChanged::Unknown,
                    Outcome::Reused,
                    (2.0, 0.0, "2026-11-24T11:00:00Z", "2026-11-24T11:00:02Z"),
                ),
            ],
            &ReadingOptions::default(),
        );

        assert_eq!(report.ran_with_unchanged_key, ["alpha"]);
        assert_eq!(report.no_key_no_reason, ["broken"]);
        let alpha = report
            .seconds_by_step
            .iter()
            .find(|step| step.step == "alpha")
            .unwrap();
        assert_eq!(alpha.unchanged_seconds, 4.0);
        assert_eq!(alpha.unknown_seconds, 2.0);

        let run = report
            .run_folds
            .iter()
            .find(|run| run.run_id == "run-1")
            .unwrap();
        assert_eq!(run.wall_clock_seconds, 12.0);
        assert_eq!(run.sum_step_seconds, 7.0);
        assert_eq!(run.observed_waited_seconds, 3.0);
        assert_eq!(run.unaccounted_seconds, 2.0);
        assert_eq!(run.waiting_seconds, 3.0);
        assert_eq!(run.executing_seconds, 7.0);
        assert_eq!(run.finish_setting_step, "broken");
    }

    #[test]
    fn seconds_view_ranks_unchanged_cost_descending_and_counts_runs() {
        let report = fold(
            vec![
                row(
                    "run-1",
                    "paid-twice",
                    "key-a",
                    KeyChanged::No,
                    Outcome::Ran,
                    (6.0, 0.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:06Z"),
                ),
                row(
                    "run-2",
                    "paid-twice",
                    "key-a",
                    KeyChanged::No,
                    Outcome::Ran,
                    (3.0, 0.0, "2026-11-24T11:00:00Z", "2026-11-24T11:00:03Z"),
                ),
                row(
                    "run-1",
                    "smaller-repeat",
                    "key-b",
                    KeyChanged::No,
                    Outcome::Ran,
                    (4.0, 0.0, "2026-11-24T10:00:06Z", "2026-11-24T10:00:10Z"),
                ),
                row(
                    "run-1",
                    "unknown-expense",
                    "none: reads device state",
                    KeyChanged::Unknown,
                    Outcome::Ran,
                    (100.0, 0.0, "2026-11-24T10:00:10Z", "2026-11-24T10:01:50Z"),
                ),
            ],
            &ReadingOptions::default(),
        );

        assert_eq!(
            report
                .seconds_by_step
                .iter()
                .map(|step| step.step.as_str())
                .collect::<Vec<_>>(),
            ["paid-twice", "smaller-repeat", "unknown-expense"]
        );
        assert_eq!(report.seconds_by_step[0].unchanged_seconds, 9.0);
        assert_eq!(report.seconds_by_step[0].run_count, 2);
        assert_eq!(report.seconds_by_step[1].run_count, 1);
    }

    #[test]
    fn run_view_names_latest_finish_and_breaks_ties_by_longest_step() {
        let report = fold(
            vec![
                row(
                    "run",
                    "short-finish",
                    "key-a",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (2.0, 0.0, "2026-11-24T10:00:08Z", "2026-11-24T10:00:10Z"),
                ),
                row(
                    "run",
                    "long-finish",
                    "key-b",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (7.0, 0.0, "2026-11-24T10:00:03Z", "2026-11-24T10:00:10Z"),
                ),
                row(
                    "run",
                    "earlier",
                    "key-c",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (8.0, 0.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:08Z"),
                ),
            ],
            &ReadingOptions::default(),
        );

        let run = &report.run_folds[0];
        assert_eq!(run.wall_clock_seconds, 10.0);
        assert_eq!(run.sum_step_seconds, 17.0);
        assert_eq!(run.finish_setting_step, "long-finish");
    }

    #[test]
    fn waiting_view_counts_only_gaps_after_the_execution_frontier() {
        let report = fold(
            vec![
                row(
                    "run",
                    "long",
                    "key-a",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (10.0, 1.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:10Z"),
                ),
                row(
                    "run",
                    "nested",
                    "key-b",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (1.0, 2.0, "2026-11-24T10:00:02Z", "2026-11-24T10:00:03Z"),
                ),
                row(
                    "run",
                    "after-gap",
                    "key-c",
                    KeyChanged::Yes,
                    Outcome::Ran,
                    (2.0, 0.0, "2026-11-24T10:00:12Z", "2026-11-24T10:00:14Z"),
                ),
            ],
            &ReadingOptions::default(),
        );

        let run = &report.run_folds[0];
        assert_eq!(run.observed_waited_seconds, 3.0);
        assert_eq!(run.unaccounted_seconds, 2.0);
        assert_eq!(run.waiting_seconds, 3.0);
        assert!(report.gap_accounting.contains("non-overlapping"));
    }

    #[test]
    fn lease_wait_and_unaccounted_gaps_are_reported_separately() {
        let first = row(
            "run",
            "first",
            "key-a",
            KeyChanged::Yes,
            Outcome::Ran,
            (2.0, 1.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:02Z"),
        );
        let mut second = row(
            "run",
            "second",
            "key-b",
            KeyChanged::Yes,
            Outcome::Ran,
            (2.0, 0.0, "2026-11-24T10:00:05Z", "2026-11-24T10:00:07Z"),
        );
        second.lease_waited_seconds = 2.0;
        let report = fold(vec![first, second], &ReadingOptions::default());
        let run = &report.run_folds[0];
        assert_eq!(run.waiting_seconds, 3.0);
        assert_eq!(run.observed_waited_seconds, 1.0);
        assert_eq!(run.lease_waited_seconds, 2.0);
        assert_eq!(run.unaccounted_seconds, 3.0);
    }

    #[test]
    fn synthesized_skips_do_not_set_run_span_or_finish() {
        let real = row(
            "run",
            "real",
            "key-a",
            KeyChanged::Yes,
            Outcome::Ran,
            (2.0, 0.0, "2026-11-24T10:00:00Z", "2026-11-24T10:00:02Z"),
        );
        let mut synthesized = row(
            "run",
            "filled-at-exit",
            "none: not reached",
            KeyChanged::Unknown,
            Outcome::Skipped,
            (0.0, 0.0, "2026-11-24T10:00:10Z", "2026-11-24T10:00:10Z"),
        );
        synthesized.synthesized = true;
        let report = fold(vec![real, synthesized], &ReadingOptions::default());
        let run = &report.run_folds[0];
        assert_eq!(run.wall_clock_seconds, 2.0);
        assert_eq!(run.finish_setting_step, "real");
    }

    #[test]
    fn default_window_keeps_the_ten_newest_runs() {
        let rows = (0..12)
            .map(|hour| {
                let start = format!("2026-11-24T{hour:02}:00:00Z");
                let end = format!("2026-11-24T{hour:02}:00:01Z");
                row(
                    &format!("run-{hour:02}"),
                    "step",
                    "key",
                    KeyChanged::No,
                    Outcome::Reused,
                    (0.0, 0.0, &start, &end),
                )
            })
            .collect();
        let report = fold(rows, &ReadingOptions::default());
        assert_eq!(report.runs, 10);
        assert!(report.run_folds.iter().all(|run| run.run_id != "run-00"));
        assert!(report.run_folds.iter().all(|run| run.run_id != "run-01"));
    }

    #[test]
    fn child_rows_keep_their_step_seconds_and_never_count_twice_in_a_run() {
        let mut fmt = row(
            "run",
            "eprfs:fmt",
            "key",
            KeyChanged::Unknown,
            Outcome::Ran,
            (4.0, 0.0, "2026-11-24T12:00:00Z", "2026-11-24T12:00:04Z"),
        );
        fmt.parent = Some("eprfs".into());
        let mut test = row(
            "run",
            "eprfs:test",
            "key",
            KeyChanged::Unknown,
            Outcome::Ran,
            (5.0, 0.0, "2026-11-24T12:00:04Z", "2026-11-24T12:00:09Z"),
        );
        test.parent = Some("eprfs".into());
        let project = row(
            "run",
            "eprfs",
            "key",
            KeyChanged::Unknown,
            Outcome::Ran,
            (10.0, 0.0, "2026-11-24T12:00:00Z", "2026-11-24T12:00:10Z"),
        );
        let report = fold(vec![fmt, test, project], &ReadingOptions::default());

        let run = &report.run_folds[0];
        assert_eq!(run.steps, 1, "only the parent row is a run step");
        assert_eq!(
            run.sum_step_seconds, 10.0,
            "child seconds are inside the parent's"
        );
        assert_eq!(run.finish_setting_step, "eprfs");
        let seconds = |name: &str| {
            report
                .seconds_by_step
                .iter()
                .find(|step| step.step == name)
                .map(|step| step.unknown_seconds)
        };
        assert_eq!(seconds("eprfs:fmt"), Some(4.0));
        assert_eq!(seconds("eprfs:test"), Some(5.0));
        assert_eq!(seconds("eprfs"), Some(10.0));
    }

    #[test]
    fn a_parent_must_name_another_nonempty_step() {
        let mut child = stored_row("run", "eprfs:fmt", "key");
        child.parent = Some("eprfs".into());
        assert!(child.validate("seconds").is_ok());
        for bad in ["", "  ", "eprfs:fmt"] {
            child.parent = Some(bad.into());
            assert!(child.validate("seconds").is_err(), "{bad:?}");
        }
        let absent = serde_json::to_string(&stored_row("run", "eprfs", "key")).unwrap();
        assert!(
            !absent.contains("parent"),
            "a row without a parent serializes as before"
        );
    }

    /// A main checkout and one linked worktree, laid out as git lays them out.
    fn main_and_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let base = tempfile::tempdir().unwrap();
        let main = base.path().join("main");
        let private = main.join(".git/worktrees/wt");
        fs::create_dir_all(&private).unwrap();
        fs::write(private.join("commondir"), "../..\n").unwrap();
        let worktree = base.path().join("wt");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", private.display()),
        )
        .unwrap();
        for root in [&main, &worktree] {
            let registry = root.join(".claude/epr-meta");
            fs::create_dir_all(&registry).unwrap();
            fs::write(
                registry.join("measures.yaml"),
                "measures-version: 1\nmeasures:\n  - id: gate-step-reading\n    version: 1\n    unit: seconds\n",
            )
            .unwrap();
        }
        (base, main, worktree)
    }

    #[test]
    fn pushes_from_two_checkouts_fold_into_one_device_report() {
        let (base, main, worktree) = main_and_worktree();
        for (root, run) in [(&main, "push-main"), (&worktree, "push-worktree")] {
            let source = base.path().join(format!("{run}.jsonl"));
            write_source(&source, &[stored_row(run, "eprfs", "key")]);
            assert_eq!(append(root, &source).unwrap().accepted, 1);
        }
        assert!(
            !worktree.join(READINGS_REL).exists(),
            "a linked worktree keeps no store of its own"
        );
        for root in [&main, &worktree] {
            let report = report(root, &ReadingOptions::default()).unwrap();
            assert_eq!(
                report.runs, 2,
                "one device report, read from either checkout"
            );
        }
    }

    #[test]
    fn a_checkout_whose_git_file_cannot_be_followed_keeps_its_own_store() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(".git"),
            "gitdir: /nonexistent/.git/worktrees/x\n",
        )
        .unwrap();
        assert_eq!(reading_path(root.path()), root.path().join(READINGS_REL));
        let plain = tempfile::tempdir().unwrap();
        assert_eq!(reading_path(plain.path()), plain.path().join(READINGS_REL));
    }
}
