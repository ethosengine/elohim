//! `epr flow report reading` — the shadow per-gate change-detection reading.
//!
//! `gate-runner.mjs` writes one `gate-step-reading@1` observation after a project finishes. This
//! report puts closure movement, outcome and time beside each other without feeding any of them
//! back into selection, reuse or the closure key. The sidecar remains the one observation store;
//! this module is only its native fold and renderer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::report::{read_folds, Fold};
use super::{FlowError, FlowResult};

pub const MEASURE: &str = "gate-step-reading@1";
const DEFAULT_LAST_RUNS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum KeyChanged {
    Yes,
    No,
    Unknown,
}

impl KeyChanged {
    fn parse(raw: Option<&String>) -> Self {
        match raw.map(String::as_str) {
            Some("yes") => Self::Yes,
            Some("no") => Self::No,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Ran,
    Reused,
    Skipped,
}

impl Outcome {
    fn parse(raw: Option<&String>) -> Option<Self> {
        match raw.map(String::as_str) {
            Some("ran") => Some(Self::Ran),
            Some("reused") => Some(Self::Reused),
            Some("skipped") => Some(Self::Skipped),
            _ => None,
        }
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

impl ReadingObservation {
    fn from_fold(fold: &Fold) -> Option<Self> {
        if fold.measure.to_string() != MEASURE {
            return None;
        }
        let env = &fold.env;
        let started_at = parse_time(env.get("started_at")?)?;
        let ended_at = parse_time(env.get("ended_at")?)?;
        Some(Self {
            run_id: env.get("run_id")?.trim().to_string(),
            step: env.get("step")?.trim().to_string(),
            key: env
                .get("key")
                .map_or_else(String::new, |value| value.trim().to_string()),
            key_changed: KeyChanged::parse(env.get("key_changed")),
            outcome: Outcome::parse(env.get("outcome"))?,
            seconds: nonnegative(fold.value),
            waited_seconds: env
                .get("waited_seconds")
                .and_then(|raw| raw.parse::<f64>().ok())
                .map_or(0.0, nonnegative),
            started_at,
            ended_at,
        })
    }
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
    let rows = read_folds(root)?
        .iter()
        .filter_map(ReadingObservation::from_fold)
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
