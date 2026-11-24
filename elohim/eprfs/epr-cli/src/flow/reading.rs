//! `epr flow reading append` and `epr flow report reading` — the shadow per-gate reading.
//!
//! Gate callers collect rows and hand one JSONL file to the native append verb. This module owns
//! the record shape, its lock and its bounded device-local retention; callers never append the
//! durable file themselves. The report puts closure movement, outcome and time beside each other
//! without feeding any of them back into selection, reuse or the closure key.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadingRow {
    measure: String,
    value: f64,
    unit: String,
    run_id: String,
    step: String,
    key: String,
    key_changed: KeyChanged,
    outcome: Outcome,
    waited_seconds: f64,
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
        if !self.value.is_finite() || self.value < 0.0 {
            return Err("value must be a finite non-negative number".into());
        }
        if !self.waited_seconds.is_finite() || self.waited_seconds < 0.0 {
            return Err("waited_seconds must be a finite non-negative number".into());
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
            key: self.key.trim().to_string(),
            key_changed: self.key_changed,
            outcome: self.outcome,
            seconds: nonnegative(self.value),
            waited_seconds: nonnegative(self.waited_seconds),
            started_at: parse_time(&self.started_at)?,
            ended_at: parse_time(&self.ended_at)?,
        })
    }
}

#[derive(Debug, Clone)]
struct ReadingObservation {
    run_id: String,
    step: String,
    key: String,
    key_changed: KeyChanged,
    outcome: Outcome,
    seconds: f64,
    waited_seconds: f64,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendSummary {
    pub accepted: usize,
    pub rejected: usize,
    pub trimmed: bool,
    pub path: PathBuf,
}

fn reading_path(root: &Path) -> PathBuf {
    root.join(READINGS_REL)
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

fn derive_key_changed(rows: &mut [ReadingRow], previous: &[ReadingRow]) {
    let mut latest = previous
        .iter()
        .map(|row| (row.step.clone(), row.key.clone()))
        .collect::<BTreeMap<_, _>>();
    for row in rows {
        if row.key_changed == KeyChanged::Unknown && !row.key.starts_with("none:") {
            row.key_changed = match latest.get(&row.step) {
                Some(key) if key == &row.key => KeyChanged::No,
                _ => KeyChanged::Yes,
            };
        }
        latest.insert(row.step.clone(), row.key.clone());
    }
}

fn trim(path: &Path) -> FlowResult<bool> {
    if fs::metadata(path)?.len() <= TRIM_ABOVE_BYTES {
        return Ok(false);
    }
    let bytes = fs::read(path)?;
    let mut lines = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .rev()
        .scan(0usize, |held, line| {
            if *held + line.len() > TRIM_TO_BYTES {
                None
            } else {
                *held += line.len();
                Some(line)
            }
        })
        .collect::<Vec<_>>();
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
        let row = line
            .ok()
            .and_then(|text| serde_json::from_str::<ReadingRow>(&text).ok());
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
    let _lock = Flock::try_exclusive(lock_file)?.map_err(|_| {
        FlowError::Unavailable(
            "gate-step reading append is busy under its device-local lock".into(),
        )
    })?;

    let previous = read_rows(&path)?;
    derive_key_changed(&mut accepted, &previous);
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
    pub gap_seconds: f64,
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
        by_run.entry(row.run_id.clone()).or_default().push(row);
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
            let mut execution_frontier = run_rows[0].ended_at;
            let mut gap_seconds = 0.0;
            for row in run_rows.iter().skip(1) {
                if row.started_at >= execution_frontier {
                    gap_seconds += elapsed_seconds(execution_frontier, row.started_at);
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
                gap_seconds,
                waiting_seconds: observed_waited_seconds + gap_seconds,
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
        gap_accounting: "gaps count only between sequential, non-overlapping steps; an overlapping step extends the execution frontier and contributes no gap".into(),
        no_key_no_reason: no_key_no_reason.into_iter().collect(),
    }
}

impl ReadingReport {
    pub fn render(&self) {
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
        println!("runs — wall vs summed steps; finish-setting step; waiting vs executing:");
        println!("  gap accounting: {}", self.gap_accounting);
        for run in &self.run_folds {
            println!(
                "  {}  wall={:.3}  steps={:.3}  finish={}  waiting={:.3} (observed={:.3}, gaps={:.3})  executing={:.3}",
                run.run_id,
                run.wall_clock_seconds,
                run.sum_step_seconds,
                run.finish_setting_step,
                run.waiting_seconds,
                run.observed_waited_seconds,
                run.gap_seconds,
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
            key: key.into(),
            key_changed: KeyChanged::Unknown,
            outcome: Outcome::Ran,
            waited_seconds: 0.0,
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
        assert_eq!(rows[0].key_changed, KeyChanged::Yes);
    }

    #[test]
    fn append_refuses_a_contended_store_lock() {
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

        let error = append(root.path(), &source).unwrap_err().to_string();
        assert!(error.contains("busy under its device-local lock"));
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
            key: key.into(),
            key_changed,
            outcome,
            seconds,
            waited_seconds,
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
        assert_eq!(run.gap_seconds, 2.0);
        assert_eq!(run.waiting_seconds, 5.0);
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
        assert_eq!(run.gap_seconds, 2.0);
        assert_eq!(run.waiting_seconds, 5.0);
        assert!(report.gap_accounting.contains("non-overlapping"));
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
}
