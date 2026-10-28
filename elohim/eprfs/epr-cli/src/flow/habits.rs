//! Habits as compositions of the epr-rea primitives — progress toward green, red mass, and the
//! time a habit costs and returns.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-10-22-habit-progress-scalar-design.md`.
//!
//! The register says whether a habit is kept. This module reads how far a red habit has moved
//! toward green, what the work cost and what it gives back — with nothing new beside the fabric:
//!
//! - **The habit is a resource.** `atom_cid(PinnedRef { id: "habit:<id>", version: 1 })`, the same
//!   naming [`super::repository_scope`] uses for the repository: stable across every edit of the
//!   atom, because the atom's filename already pins the id. Its red mass is that resource's
//!   state — a pure fold of `Produce` (fills) and `Consume` (drains) in [`RED_MASS_UNIT`].
//! - **Each red episode is a [`Commitment`]** to make the habit green, dated at the commit that
//!   opened it and carrying the declared budget as its [`Bound`]. The flip to green is the one
//!   event that `fulfills` it, so the commitments ledger counts a flip the way it counts any
//!   other discharge. Progress and cost events never name a promise: annotations never discharge.
//! - **Progress is [`elohim_epr_rea::progress`]**: each declared part read in its own shape and
//!   composed by the habit's declared [`Composition`].
//! - **Time spent is a `Consume` of the time resource** scoped to the habit; time returned is a
//!   gauge part's recurring saving as a `Rate`; payback is [`coverage_time`].
//!
//! Everything is derived from git — the first-parent history of the habit atoms, dated when they
//! arrived on that line — so two clones mint the same records and a historical window reads the
//! same tomorrow.
//! Lane reports never enter as live state: a lane's result becomes a reading when it is recorded
//! in the atom with its run id as evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::Stdio;

use cid::Cid;
use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Period, Quantity};
use elohim_epr_rea::{
    atom_cid, compose, coverage_time, level_evidence, recurring_return, AgentRef,
    AlgedonicEvidence, Bound, Commitment, CommitmentState, Composed, Composition, FlowEvent,
    FlowRecord, FlowStore, Magnitude, Part, PinnedRef, ReaVerb, ResourceSpec, SidecarFlowStore,
    Window,
};
use serde::Serialize;
use serde_yaml::Value;

use super::{FlowError, FlowResult};

/// The unit red mass is counted in: one habit-equivalent is one red habit that has not moved.
pub const RED_MASS_UNIT: &str = "habit-equivalent";
/// The unit time is spent in.
pub const HOUR_UNIT: &str = "hour";
/// The unit a red episode promises: one habit, green.
pub const GREEN_UNIT: &str = "habit-green";
/// Slot-0 tag on every episode commitment.
pub const REGISTER_TAG: &str = "register:habit";
/// Who derives the habit records — the register itself, never an agent's session.
const PROVIDER: &str = "tool:habits-register";
/// Below this a red-mass change is float noise, not a movement.
const EPSILON: f64 = 1e-9;
/// The meters `spend` accepts, and the claim each earns. Self-reported session time is advisory
/// forever (middot Commitment II); a metered pipeline or gate run is an instrument reading.
const METERS: &[(&str, ClaimKind)] = &[
    ("pipeline", ClaimKind::InstrumentMeasured),
    ("gate", ClaimKind::InstrumentMeasured),
    ("session", ClaimKind::Estimated),
    ("other", ClaimKind::Estimated),
];

/// The habit as a resource and a scope.
pub fn habit_resource(id: &str) -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: format!("habit:{id}"),
        version: 1,
    })?)
}

/// The synthetic resource the register's red-mass stock is folded on.
pub fn register_resource() -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: "register:red-mass".to_string(),
        version: 1,
    })?)
}

/// The resource every hour spent is a `Consume` of.
pub fn time_resource() -> FlowResult<Cid> {
    Ok(atom_cid(&PinnedRef {
        id: "resource:time".to_string(),
        version: 1,
    })?)
}

// ---------------------------------------------------------------------------
// The declaration
// ---------------------------------------------------------------------------

/// One declared part of a habit, read and validated.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartReading {
    pub part: String,
    /// `check` when an `@concern:` check names it, `declared` when only `rea.parts` does.
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading: Option<Part>,
    /// `None` is unmeasured — never zero.
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deterioration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<ClaimKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Hours per week this part gives back, when it is a recurring time gauge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns_hours_per_week: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns_claim: Option<ClaimKind>,
    /// Why a declared reading was not admitted. An invalid part is unmeasured, never zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
}

impl PartReading {
    fn unmeasured(part: &str, source: &'static str) -> Self {
        Self {
            part: part.to_string(),
            source,
            reading: None,
            progress: None,
            deterioration: None,
            claim: None,
            at: None,
            evidence: None,
            unit: None,
            returns_hours_per_week: None,
            returns_claim: None,
            invalid: None,
        }
    }

    /// What a reading is comparable against: its denominator, or its baseline→target span. Two
    /// readings of one part with different scopes are a RESCOPE, never progress.
    fn scope(&self) -> Option<String> {
        self.reading.map(|reading| match reading {
            Part::Ratio { declared, .. } => format!("ratio/{declared}"),
            Part::Gauge {
                baseline, target, ..
            } => format!("gauge/{baseline}->{target}"),
            Part::Check { .. } => "check".to_string(),
        })
    }
}

/// One habit at one revision of its atom.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitState {
    pub id: String,
    pub status: String,
    pub composed: Composed,
    /// `0` when green; `1 - floor` otherwise.
    pub red_mass: f64,
    pub parts: Vec<PartReading>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Bound>,
    /// Hours per week the habit's recurring gauges give back, summed; `0` when none do.
    pub returns_hours_per_week: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns_claim: Option<ClaimKind>,
    /// Problems with the `rea:` block that did not stop the habit being read.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub declaration_issues: Vec<String>,
}

/// Read one habit atom's text. `recorded_at` is the commit's history-arrival timestamp
/// (normalized UTC); a reading dated after it is refused as future-dated. `None` skips that check
/// (an uncommitted tree has no recording commit).
pub fn evaluate(text: &str, recorded_at: Option<&str>) -> Result<HabitState, String> {
    let front = frontmatter(text).ok_or("missing or unterminated frontmatter")?;
    let doc: Value = serde_yaml::from_str(front).map_err(|e| format!("frontmatter: {e}"))?;
    let id = str_field(&doc, "id").ok_or("no `id`")?.to_string();
    let status = str_field(&doc, "status").ok_or("no `status`")?.to_string();
    if !matches!(status.as_str(), "green" | "red" | "unwired") {
        return Err(format!("status `{status}` is not green, red or unwired"));
    }
    let checks: Vec<&str> = doc
        .get("checks")
        .and_then(Value::as_sequence)
        .map(|seq| seq.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut issues = Vec::new();
    let rea = doc.get("rea");
    let composition = match rea.and_then(|r| r.get("composition")) {
        None => Composition::Sum,
        Some(value) => match value.as_str().map(str::trim) {
            Some("sum") => Composition::Sum,
            Some("min") => Composition::Min,
            Some("harmonic") => Composition::Harmonic,
            _ => {
                issues.push(format!(
                    "rea.composition `{}` is not sum, min or harmonic — read as sum",
                    yaml_text(value)
                ));
                Composition::Sum
            }
        },
    };
    let budget = match rea.and_then(|r| r.get("budget")) {
        None => None,
        Some(declared) => match budget_of(declared) {
            Ok(bound) => Some(bound),
            Err(why) => {
                issues.push(format!("rea.budget: {why}"));
                None
            }
        },
    };

    // Parts: every @concern check, in order; then every declared part no check names.
    let mut parts: Vec<PartReading> = Vec::new();
    for tag in concern_tags(&checks) {
        parts.push(PartReading::unmeasured(&tag, "check"));
    }
    let mut seen_declared = BTreeSet::new();
    let declared = rea
        .and_then(|r| r.get("parts"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    for (index, entry) in declared.iter().enumerate() {
        let Some(name) = entry.get("part").and_then(Value::as_str).map(str::trim) else {
            issues.push(format!("rea.parts[{index}] has no `part` name — ignored"));
            continue;
        };
        if !seen_declared.insert(name.to_string()) {
            issues.push(format!(
                "rea.parts names `{name}` twice — the second is ignored"
            ));
            continue;
        }
        let read = read_part(name, entry, recorded_at);
        match parts.iter_mut().find(|p| p.part == name) {
            Some(existing) => {
                *existing = PartReading {
                    source: "check",
                    ..read
                }
            }
            None => parts.push(PartReading {
                source: "declared",
                ..read
            }),
        }
    }

    let readings: Vec<Option<f64>> = parts.iter().map(|p| p.progress).collect();
    let composed = compose(composition, &readings).map_err(|e| e.to_string())?;
    let red_mass = if status == "green" {
        0.0
    } else {
        1.0 - composed.floor
    };
    let mut returns_hours_per_week = 0.0;
    let mut returns_claim: Option<ClaimKind> = None;
    for part in &parts {
        if let (Some(hours), Some(claim)) = (part.returns_hours_per_week, part.returns_claim) {
            returns_hours_per_week += hours;
            returns_claim = Some(returns_claim.map_or(claim, |c| c.weaker(claim)));
        }
    }
    Ok(HabitState {
        id,
        status,
        composed,
        red_mass,
        parts,
        budget,
        returns_hours_per_week,
        returns_claim,
        declaration_issues: issues,
    })
}

fn read_part(name: &str, entry: &Value, recorded_at: Option<&str>) -> PartReading {
    let mut out = PartReading::unmeasured(name, "declared");
    out.unit = entry
        .get("unit")
        .and_then(Value::as_str)
        .map(str::to_string);
    let invalid = |mut out: PartReading, why: String| {
        out.invalid = Some(why);
        out.reading = None;
        out.progress = None;
        out
    };
    // CI-harvest routes a habit by every `@concern:` tag anywhere in its block; a reading that
    // carried one would mint a route nobody declared.
    if yaml_text(entry).contains("@concern:") {
        return invalid(out, "a part may not carry an `@concern:` tag".into());
    }
    let Some(raw) = entry.get("reading") else {
        return out; // declared, not yet measured
    };
    let at = entry
        .get("at")
        .map(yaml_text)
        .and_then(|t| normalize_at(&t));
    let evidence = entry
        .get("evidence")
        .map(yaml_text)
        .filter(|e| !e.trim().is_empty());
    let (Some(at), Some(evidence)) = (at, evidence) else {
        return invalid(
            out,
            "a reading needs `at` (YYYY-MM-DD or RFC3339) and `evidence` (a run, build or probe)"
                .into(),
        );
    };
    if recorded_at.is_some_and(|recorded| at.as_str() > recorded) {
        return invalid(
            out,
            format!("`at` {at} is later than the commit that records it"),
        );
    }
    let claim = match entry.get("claim") {
        None => ClaimKind::InstrumentMeasured,
        Some(value) => match serde_yaml::from_value::<ClaimKind>(value.clone()) {
            Ok(claim) => claim,
            Err(_) => {
                return invalid(
                    out,
                    format!("`claim` `{}` is not a ClaimKind", yaml_text(value)),
                )
            }
        },
    };
    let part = match parse_reading(raw, entry) {
        Ok(part) => part,
        Err(why) => return invalid(out, why),
    };
    let progress = match part.progress() {
        Ok(progress) => progress,
        Err(e) => return invalid(out, e.to_string()),
    };
    out.reading = Some(part);
    out.progress = Some(progress);
    out.deterioration = part.deterioration();
    out.claim = Some(claim);
    out.at = Some(at);
    out.evidence = Some(evidence);

    // A recurring time gauge returns hours each week.
    let per_week = entry.get("per_week").and_then(number);
    let scale = out.unit.as_deref().and_then(hours_per_unit);
    if let (Some(per_week), Some(scale)) = (per_week, scale) {
        let frequency_claim = if entry
            .get("per_week_evidence")
            .map(yaml_text)
            .is_some_and(|e| !e.trim().is_empty())
        {
            ClaimKind::InstrumentMeasured
        } else {
            ClaimKind::Estimated
        };
        let claim = claim.weaker(frequency_claim);
        if let Ok(Some(rate)) = recurring_return(
            &part,
            scale,
            per_week,
            Period::Week,
            claim,
            format!("{name}: saved per occurrence x {per_week}/week, in hours"),
        ) {
            out.returns_hours_per_week = Some(rate.value);
            out.returns_claim = Some(claim);
        }
    }
    out
}

fn parse_reading(raw: &Value, entry: &Value) -> Result<Part, String> {
    if let Some(holds) = raw.as_bool() {
        return Ok(Part::Check { holds });
    }
    if let Some(text) = raw.as_str().map(str::trim) {
        if text.eq_ignore_ascii_case("pass") {
            return Ok(Part::Check { holds: true });
        }
        if text.eq_ignore_ascii_case("fail") {
            return Ok(Part::Check { holds: false });
        }
        if let Some((passed, declared)) = text.split_once('/') {
            let (Ok(passed), Ok(declared)) =
                (passed.trim().parse::<f64>(), declared.trim().parse::<f64>())
            else {
                return Err(format!("`{text}` is not passed/declared"));
            };
            return Ok(Part::Ratio { passed, declared });
        }
    }
    let reading = number(raw).ok_or_else(|| {
        format!(
            "reading `{}` is not n/d, pass/fail or a number",
            yaml_text(raw)
        )
    })?;
    let (Some(baseline), Some(target)) = (
        entry.get("baseline").and_then(number),
        entry.get("target").and_then(number),
    ) else {
        return Err("a numeric reading is a gauge: it needs `baseline` and `target`".into());
    };
    Ok(Part::Gauge {
        baseline,
        target,
        reading,
    })
}

fn budget_of(declared: &Value) -> Result<Bound, String> {
    let hours = declared
        .get("hours")
        .and_then(number)
        .ok_or("needs `hours`")?;
    let pct = match declared.get("threshold_pct") {
        None => 85.0,
        Some(value) => number(value).ok_or("`threshold_pct` is not a number")?,
    };
    Bound::new(hours, HOUR_UNIT.to_string(), pct).map_err(|e| e.to_string())
}

/// Hours in one of a gauge's time units, or `None` when the unit is not a time.
fn hours_per_unit(unit: &str) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => Some(1.0 / 3600.0),
        "min" | "mins" | "minute" | "minutes" => Some(1.0 / 60.0),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(1.0),
        _ => None,
    }
}

fn concern_tags(checks: &[&str]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for check in checks {
        let mut rest = *check;
        while let Some(at) = rest.find("@concern:") {
            let tail = &rest[at + "@concern:".len()..];
            let tag: String = tail
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect();
            if tag.starts_with(|c: char| c.is_ascii_alphanumeric()) && !tags.contains(&tag) {
                tags.push(tag);
            }
            rest = tail;
        }
    }
    tags
}

fn frontmatter(text: &str) -> Option<&str> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    Some(&body[..end])
}

fn str_field<'a>(doc: &'a Value, key: &str) -> Option<&'a str> {
    doc.get(key).and_then(Value::as_str).map(str::trim)
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
        .filter(|v: &f64| v.is_finite())
}

fn yaml_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// `YYYY-MM-DD` (midnight UTC) or RFC3339, as uniform second-precision UTC.
fn normalize_at(raw: &str) -> Option<String> {
    let text = raw.trim();
    if let Ok(date) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Some(format!("{date}T00:00:00Z"));
    }
    chrono::DateTime::parse_from_rfc3339(text).ok().map(|dt| {
        dt.with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    })
}

// ---------------------------------------------------------------------------
// The history walk
// ---------------------------------------------------------------------------

/// What one walk of the atoms' history derives.
#[derive(Debug, Default)]
pub struct HabitHistory {
    /// Every red-mass and discharge event, in history order.
    pub events: Vec<FlowEvent>,
    /// Every episode commitment, in history order, with its CID.
    pub episodes: Vec<(Cid, Commitment)>,
    /// The habits standing at HEAD.
    pub current: BTreeMap<String, Standing>,
    /// Transient versions of changed habits, rebuilt from the same first-parent history.
    /// A `None` standing records retirement; no register snapshots or new identities are stored.
    versions: BTreeMap<String, Vec<(String, Option<Standing>)>>,
    /// `<sha12>:<path>: why` for every revision that could not be read. Skipped, never zero.
    pub unreadable: Vec<String>,
}

/// One habit at a revision: its state, path, birth, and open episode if red.
#[derive(Debug, Clone)]
pub struct Standing {
    pub state: HabitState,
    pub path: String,
    /// When the habit first stood on this line — moves keep it, so a window can tell a habit
    /// that did not exist yet from one that existed and was green.
    pub born: String,
    pub episode: Option<(Cid, Commitment)>,
}

/// One first-parent commit that touched a habit atom.
struct Revision {
    sha: String,
    at: String,
    changes: Vec<(char, String)>,
}

/// Walk the first-parent history of every habit atom and derive the habit records.
///
/// First-parent, because a merged side branch's intermediate states never stood on this
/// line — the merge commit is where they arrived. Committer dates are normalized to UTC and
/// clamped to history order. Renames are read as a removal plus an addition in the same commit and
/// reconciled by id, so moving an atom between directories is not a retirement.
pub fn walk_history(root: &Path) -> FlowResult<HabitHistory> {
    let repository = super::repository_agent(root)?;
    let register_scope = super::repository_scope(root)?;
    let revisions = first_parent_revisions(root)?;
    let wanted: Vec<String> = revisions
        .iter()
        .flat_map(|r| {
            r.changes
                .iter()
                .filter(|(kind, _)| *kind != 'D')
                .map(move |(_, path)| format!("{}:{path}", r.sha))
        })
        .collect();
    let mut blobs = cat_file_batch(root, &wanted)?.into_iter();

    let mut history = HabitHistory::default();
    let mut path_to_id: BTreeMap<String, String> = BTreeMap::new();
    for revision in &revisions {
        let sha12 = &revision.sha[..revision.sha.len().min(12)];
        let mut arrived: BTreeMap<String, (HabitState, String)> = BTreeMap::new();
        let mut removed: Vec<String> = Vec::new();
        for (kind, path) in &revision.changes {
            if *kind == 'D' {
                if let Some(id) = path_to_id.remove(path) {
                    removed.push(id);
                }
                continue;
            }
            let Some(text) = blobs.next().flatten() else {
                history
                    .unreadable
                    .push(format!("{sha12}:{path}: blob not readable"));
                continue;
            };
            match evaluate(&text, Some(&revision.at)) {
                Ok(state) => {
                    path_to_id.insert(path.clone(), state.id.clone());
                    arrived.insert(state.id.clone(), (state, path.clone()));
                }
                Err(why) => history.unreadable.push(format!("{sha12}:{path}: {why}")),
            }
        }
        let context = Moment {
            at: &revision.at,
            sha12,
            repository: &repository,
            register_scope: &register_scope,
        };
        for (id, (state, path)) in arrived {
            let prior = history.current.remove(&id);
            let standing = advance(&context, &id, prior, state, path, &mut history)?;
            history
                .versions
                .entry(id.clone())
                .or_default()
                .push((revision.at.clone(), Some(standing.clone())));
            history.current.insert(id, standing);
        }
        for id in removed {
            // A path removed while the same id arrived elsewhere in this commit is a move.
            if path_to_id.values().any(|other| *other == id) {
                continue;
            }
            if let Some(prior) = history.current.remove(&id) {
                retire(&context, &id, prior, &mut history)?;
                history
                    .versions
                    .entry(id)
                    .or_default()
                    .push((revision.at.clone(), None));
            }
        }
    }
    Ok(history)
}

/// The commit an event is derived from, and the names every record of it carries.
struct Moment<'a> {
    at: &'a str,
    sha12: &'a str,
    repository: &'a AgentRef,
    register_scope: &'a Cid,
}

impl Moment<'_> {
    fn event(
        &self,
        id: &str,
        action: ReaVerb,
        quantity: Magnitude,
        tag: &str,
    ) -> FlowResult<FlowEvent> {
        Ok(FlowEvent {
            action,
            provider: AgentRef(PROVIDER.to_string()),
            receiver: self.repository.clone(),
            resource: habit_resource(id)?,
            quantity,
            process: None,
            in_scope_of: *self.register_scope,
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            occurred_at: self.at.to_string(),
            classified_as: vec![
                format!("habit:{tag}"),
                id.to_string(),
                format!("commit:{}", self.sha12),
            ],
        })
    }

    fn red_mass(&self, id: &str, delta: f64, tag: &str) -> FlowResult<FlowEvent> {
        let action = if delta > 0.0 {
            ReaVerb::Produce
        } else {
            ReaVerb::Consume
        };
        self.event(
            id,
            action,
            Magnitude::Count {
                value: delta.abs(),
                unit: RED_MASS_UNIT.to_string(),
            },
            tag,
        )
    }

    fn episode(&self, id: &str, state: &HabitState) -> FlowResult<(Cid, Commitment)> {
        let commitment = Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef(PROVIDER.to_string()),
            receiver: self.repository.clone(),
            resource_spec: ResourceSpec {
                classified_as: vec![REGISTER_TAG.to_string(), id.to_string()],
                quantity: Some(Magnitude::Count {
                    value: 1.0,
                    unit: GREEN_UNIT.to_string(),
                }),
            },
            in_scope_of: *self.register_scope,
            valid_from: Some(self.at.to_string()),
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: Vec::new(),
            bound: state.budget.clone(),
        };
        Ok((atom_cid(&commitment)?, commitment))
    }

    fn discharge(&self, id: &str, episode: Cid, tag: &str) -> FlowResult<FlowEvent> {
        let mut event = self.event(
            id,
            ReaVerb::Produce,
            Magnitude::Count {
                value: 1.0,
                unit: GREEN_UNIT.to_string(),
            },
            tag,
        )?;
        event.fulfills = vec![episode];
        Ok(event)
    }
}

/// How a habit's red mass moved between two readings of it.
fn movement(prior: &HabitState, next: &HabitState) -> &'static str {
    if prior.status != "green" && next.status == "green" {
        return "flip-green";
    }
    if prior.status == "green" && next.status != "green" {
        return "flip-red";
    }
    let names =
        |s: &HabitState| -> BTreeSet<String> { s.parts.iter().map(|p| p.part.clone()).collect() };
    let rescoped = names(prior) != names(next)
        || prior.composed.rule != next.composed.rule
        || prior.parts.iter().any(|before| {
            next.parts.iter().any(|after| {
                after.part == before.part
                    && before.scope().is_some()
                    && after.scope().is_some()
                    && before.scope() != after.scope()
            })
        });
    if rescoped {
        "rescope"
    } else if next.red_mass < prior.red_mass {
        "progress"
    } else {
        "regression"
    }
}

fn advance(
    at: &Moment<'_>,
    id: &str,
    prior: Option<Standing>,
    state: HabitState,
    path: String,
    history: &mut HabitHistory,
) -> FlowResult<Standing> {
    let Some(prior) = prior else {
        // Born. A green birth carries no red mass and opens no promise.
        let episode = if state.status == "green" {
            None
        } else {
            let opened = at.episode(id, &state)?;
            history.episodes.push(opened.clone());
            if state.red_mass > EPSILON {
                history
                    .events
                    .push(at.red_mass(id, state.red_mass, "born")?);
            }
            Some(opened)
        };
        return Ok(Standing {
            state,
            path,
            born: at.at.to_string(),
            episode,
        });
    };
    let delta = state.red_mass - prior.state.red_mass;
    let tag = movement(&prior.state, &state);
    let mut episode = prior.episode;
    match tag {
        "flip-green" => {
            if delta.abs() > EPSILON {
                history.events.push(at.red_mass(id, delta, tag)?);
            }
            if let Some((cid, _)) = episode.take() {
                history.events.push(at.discharge(id, cid, tag)?);
            }
        }
        "flip-red" => {
            let opened = at.episode(id, &state)?;
            history.episodes.push(opened.clone());
            episode = Some(opened);
            if delta.abs() > EPSILON {
                history.events.push(at.red_mass(id, delta, tag)?);
            }
        }
        _ if delta.abs() > EPSILON => history.events.push(at.red_mass(id, delta, tag)?),
        _ => {}
    }
    Ok(Standing {
        state,
        path,
        born: prior.born,
        episode,
    })
}

fn retire(
    at: &Moment<'_>,
    id: &str,
    prior: Standing,
    history: &mut HabitHistory,
) -> FlowResult<()> {
    if prior.state.red_mass > EPSILON {
        history
            .events
            .push(at.red_mass(id, -prior.state.red_mass, "retired")?);
    }
    if let Some((cid, _)) = prior.episode {
        history.events.push(at.discharge(id, cid, "retired")?);
    }
    Ok(())
}

/// Every first-parent commit that touched a habit atom, oldest first, each dated when it ARRIVED
/// on this line.
///
/// Committer date, not author date: a window asks what changed in the register while it was
/// open, and a change authored last week and landed today changed it today. Clamped
/// non-decreasing along the walk, because even committer dates go backwards on a real line
/// (clock skew, a rebase onto an older base) — the walk order IS the causal order, and a drain
/// dated before the fill it drains would let a window count the one without the other. Refuses a
/// shallow clone: its first commit would read as the birth of every habit standing there.
fn first_parent_revisions(root: &Path) -> FlowResult<Vec<Revision>> {
    let shallow =
        crate::process::build_command("git", &["rev-parse", "--is-shallow-repository"], root, &[])
            .output()?;
    if String::from_utf8_lossy(&shallow.stdout).trim() == "true" {
        return Err(FlowError::Unavailable(
            "the clone is shallow — its oldest commit would read as every habit's birth; \
             `git fetch --unshallow` first"
                .into(),
        ));
    }
    let output = crate::process::build_command(
        "git",
        &[
            "log",
            "--first-parent",
            "--diff-merges=first-parent",
            "--reverse",
            "--no-renames",
            "--format=%x1e%H%x1f%cI",
            "--name-status",
            "--",
            ":(glob)**/.epr-meta/*.habit.md",
            ":(glob).epr-meta/*.habit.md",
        ],
        root,
        &[],
    )
    .output()?;
    if !output.status.success() {
        return Err(FlowError::Unavailable(format!(
            "git log over the habit atoms failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut revisions: Vec<Revision> = Vec::new();
    for record in text.split('\u{1e}').filter(|r| !r.trim().is_empty()) {
        let mut lines = record.lines();
        let Some((sha, at)) = lines.next().and_then(|l| l.split_once('\u{1f}')) else {
            continue;
        };
        let changes = lines
            .filter_map(|line| {
                let (kind, path) = line.split_once('\t')?;
                let kind = kind.chars().next()?;
                super::habit_census::keep(Path::new(path)).then(|| (kind, path.to_string()))
            })
            .collect::<Vec<_>>();
        if changes.is_empty() {
            continue;
        }
        let mut at = super::normalize_git_timestamp(at);
        if let Some(previous) = revisions.last() {
            // Uniform `…Z` UTC compares lexicographically as instants.
            if at < previous.at {
                at = previous.at.clone();
            }
        }
        revisions.push(Revision {
            sha: sha.to_string(),
            at,
            changes,
        });
    }
    Ok(revisions)
}

/// Read many `<rev>:<path>` blobs with one `git cat-file --batch`, in order. `None` for any that
/// git reports missing.
fn cat_file_batch(root: &Path, specs: &[String]) -> FlowResult<Vec<Option<String>>> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = crate::process::build_command("git", &["cat-file", "--batch"], root, &[])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| FlowError::Unavailable("git cat-file --batch gave no stdin".into()))?;
    let input = specs.join("\n") + "\n";
    // Written from a thread: a large batch fills the stdout pipe before stdin is drained.
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| FlowError::Unavailable("git cat-file --batch gave no stdout".into()))?;
    let mut reader = BufReader::new(stdout);
    let mut out = Vec::with_capacity(specs.len());
    for _ in specs {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let mut fields = header.split_whitespace();
        let (_, kind, size) = (fields.next(), fields.next(), fields.next());
        match (kind, size.and_then(|s| s.parse::<usize>().ok())) {
            (Some("blob"), Some(size)) => {
                let mut body = vec![0u8; size + 1]; // the content, then git's newline
                reader.read_exact(&mut body)?;
                body.truncate(size);
                out.push(String::from_utf8(body).ok());
            }
            _ => out.push(None), // `<spec> missing`
        }
    }
    writer
        .join()
        .map_err(|_| FlowError::Unavailable("git cat-file writer panicked".into()))??;
    child.wait()?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// The reading over a window
// ---------------------------------------------------------------------------

/// The window's movement in one habit's red mass, by kind.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Movement {
    pub drained: f64,
    pub filled: f64,
    /// `habit:<kind>` -> signed red-mass change (negative drains).
    pub by_kind: BTreeMap<String, f64>,
}

/// Hours spent on one habit.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spent {
    pub in_window: f64,
    pub in_episode: f64,
    pub all_time: f64,
    /// `<meter>/<claim>` -> hours, all time.
    pub by_meter: BTreeMap<String, f64>,
    /// The weakest claim among the hours counted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<ClaimKind>,
}

/// One habit's row in the red-mass reading.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitRow {
    pub id: String,
    pub path: String,
    pub born: String,
    /// The habit as it stands at HEAD — the window bounds its red mass and movement, not this.
    pub state: HabitState,
    pub red_mass_at_start: f64,
    pub red_mass_at_end: f64,
    pub movement: Movement,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_opened: Option<String>,
    /// The budget the open episode promised (may differ from the one the atom now declares).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promised_budget: Option<Bound>,
    pub spent: Spent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_pain: Option<AlgedonicEvidence>,
    /// Weeks of returned time that pay back all hours spent. `None` when nothing is returned or
    /// nothing was spent; NaN never reaches the wire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payback_weeks: Option<f64>,
}

/// The register across one window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitsReading {
    pub habits: usize,
    pub non_green: usize,
    pub red_mass: f64,
    pub hours_spent_in_window: f64,
    pub hours_returned_per_week: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_returned_claim: Option<ClaimKind>,
    pub rows: Vec<HabitRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unreadable: Vec<String>,
}

/// Read every habit over `window` from the derived history plus the time spent in `records`.
pub fn reading(
    history: &HabitHistory,
    records: &[(Cid, FlowRecord)],
    window: &Window,
) -> FlowResult<HabitsReading> {
    let time = time_resource()?;
    let mut rows = Vec::new();
    // Select then-standing state, including habits retired after this window. A retirement
    // before the exclusive end removes the row, while its drain remains in the stock's events.
    // Reverse history order also preserves causality when commits share a clamped timestamp.
    for (id, versions) in &history.versions {
        let Some(standing) = versions
            .iter()
            .rev()
            .find(|(at, _)| at.as_str() < window.end.as_str())
            .and_then(|(_, standing)| standing.as_ref())
        else {
            continue;
        };
        let resource = habit_resource(id)?;
        let mut start = 0.0;
        let mut end = 0.0;
        let mut movement = Movement::default();
        for event in history.events.iter().filter(|e| e.resource == resource) {
            let Magnitude::Count { value, unit } = &event.quantity else {
                continue;
            };
            if unit != RED_MASS_UNIT || event.occurred_at.as_str() >= window.end.as_str() {
                continue;
            }
            let signed = if event.action == ReaVerb::Produce {
                *value
            } else {
                -*value
            };
            end += signed;
            if event.occurred_at.as_str() < window.start.as_str() {
                start += signed;
            } else {
                if signed < 0.0 {
                    movement.drained += -signed;
                } else {
                    movement.filled += signed;
                }
                let kind = event
                    .classified_as
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "habit:unclassified".into());
                *movement.by_kind.entry(kind).or_insert(0.0) += signed;
            }
        }

        let episode_from = standing
            .episode
            .as_ref()
            .and_then(|(_, c)| c.valid_from.clone());
        let mut spent = Spent::default();
        for (_, record) in records {
            let FlowRecord::Event(event) = record else {
                continue;
            };
            if event.resource != time
                || event.in_scope_of != resource
                || event.action != ReaVerb::Consume
                || event.occurred_at.as_str() >= window.end.as_str()
            {
                continue;
            }
            let Magnitude::Count { value, unit } = &event.quantity else {
                continue;
            };
            if unit != HOUR_UNIT {
                continue;
            }
            spent.all_time += value;
            if window.contains(&event.occurred_at) {
                spent.in_window += value;
            }
            if episode_from
                .as_deref()
                .is_some_and(|from| event.occurred_at.as_str() >= from)
            {
                spent.in_episode += value;
            }
            let meter = event
                .classified_as
                .first()
                .and_then(|t| t.strip_prefix("cost:"))
                .unwrap_or("other");
            let claim = event
                .classified_as
                .iter()
                .find_map(|s| s.strip_prefix("claim:"))
                .and_then(|c| serde_yaml::from_str::<ClaimKind>(c).ok())
                .unwrap_or(ClaimKind::Estimated);
            *spent
                .by_meter
                .entry(format!("{meter}/{}", claim_word(claim)))
                .or_insert(0.0) += value;
            spent.claim = Some(spent.claim.map_or(claim, |c| c.weaker(claim)));
        }

        let budget_pain = standing.episode.as_ref().and_then(|(cid, commitment)| {
            commitment
                .bound
                .as_ref()
                .and_then(|bound| level_evidence(cid, bound, spent.in_episode))
        });
        let payback_weeks = payback(&standing.state, &spent);
        rows.push(HabitRow {
            id: id.clone(),
            path: standing.path.clone(),
            born: standing.born.clone(),
            state: standing.state.clone(),
            red_mass_at_start: start,
            red_mass_at_end: end,
            movement,
            episode: standing.episode.as_ref().map(|(cid, _)| cid.to_string()),
            episode_opened: episode_from,
            promised_budget: standing.episode.as_ref().and_then(|(_, c)| c.bound.clone()),
            spent,
            budget_pain,
            payback_weeks,
        });
    }
    rows.sort_by(|a, b| {
        let moved = |r: &HabitRow| r.movement.drained + r.movement.filled;
        moved(b)
            .total_cmp(&moved(a))
            .then(b.state.composed.floor.total_cmp(&a.state.composed.floor))
            .then(a.id.cmp(&b.id))
    });
    let mut returns_claim: Option<ClaimKind> = None;
    for row in &rows {
        if let Some(claim) = row.state.returns_claim {
            returns_claim = Some(returns_claim.map_or(claim, |c| c.weaker(claim)));
        }
    }
    Ok(HabitsReading {
        habits: rows.len(),
        non_green: rows.iter().filter(|r| r.state.status != "green").count(),
        red_mass: rows.iter().map(|r| r.red_mass_at_end).sum(),
        hours_spent_in_window: rows.iter().map(|r| r.spent.in_window).sum(),
        hours_returned_per_week: rows.iter().map(|r| r.state.returns_hours_per_week).sum(),
        hours_returned_claim: returns_claim,
        rows,
        unreadable: history.unreadable.clone(),
    })
}

impl HabitsReading {
    /// The text view: the register line, then every habit that moved in the window or stands
    /// partly won, most moved first. The JSON carries every row.
    pub fn render(&self) {
        let returned = match self.hours_returned_claim {
            Some(claim) => format!(
                "{:.2} h/wk returned ({})",
                self.hours_returned_per_week,
                claim_word(claim)
            ),
            None => "no time returned".to_string(),
        };
        println!(
            "    habits:   {} not green of {} · red mass {:.2} · {:.1} h spent in window · {returned}",
            self.non_green, self.habits, self.red_mass, self.hours_spent_in_window
        );
        let shown: Vec<&HabitRow> = self
            .rows
            .iter()
            .filter(|r| {
                r.movement.drained + r.movement.filled > EPSILON
                    || (r.state.status != "green" && r.state.composed.floor > EPSILON)
                    || r.spent.all_time > 0.0
            })
            .collect();
        if !shown.is_empty() {
            println!("    moved or partly won:");
        }
        for row in shown {
            let composed = &row.state.composed;
            let measured = composed
                .over_measured
                .map(|m| format!(" ({m:.2} on {}/{})", composed.measured, composed.declared))
                .unwrap_or_default();
            let mut line = format!(
                "      {:<36} {:<7} progress {:.2}{measured}",
                row.id, row.state.status, composed.floor
            );
            if row.movement.drained > EPSILON {
                line.push_str(&format!(" · drained {:.2}", row.movement.drained));
            }
            if row.movement.filled > EPSILON {
                line.push_str(&format!(" · filled {:.2}", row.movement.filled));
            }
            if row.spent.all_time > 0.0 {
                line.push_str(&format!(" · spent {:.1} h", row.spent.all_time));
                if let Some(bound) = &row.promised_budget {
                    line.push_str(&format!(
                        " ({:.1}/{:.0} h this episode)",
                        row.spent.in_episode, bound.limit
                    ));
                }
            }
            if row.state.returns_hours_per_week > 0.0 {
                line.push_str(&format!(
                    " · returns {:.2} h/wk",
                    row.state.returns_hours_per_week
                ));
            }
            if let Some(weeks) = row.payback_weeks {
                line.push_str(&format!(" · payback {weeks:.1} wk"));
            }
            if let Some(pain) = &row.budget_pain {
                line.push_str(&format!(" · budget {}", pain.kind().as_str()));
            }
            println!("{line}");
        }
        if let Some(first) = self.unreadable.first() {
            println!(
                "    unreadable revisions: {} (skipped, never zero; first: {first})",
                self.unreadable.len()
            );
        }
    }
}

/// Payback: all hours spent over the hours per week the habit returns — [`coverage_time`], so the
/// weaker claim and the unknown-on-zero discipline are the fabric's, not re-derived here.
fn payback(state: &HabitState, spent: &Spent) -> Option<f64> {
    if spent.all_time <= 0.0 || state.returns_hours_per_week <= 0.0 {
        return None;
    }
    let level = Quantity {
        value: spent.all_time,
        kind: MeasureKind::Level,
        confidence: Confidence {
            claim: spent.claim.unwrap_or(ClaimKind::Estimated),
            ..Confidence::estimated(Interval::exact(spent.all_time), "hours spent")
        },
    };
    let rate = Quantity {
        value: state.returns_hours_per_week,
        kind: MeasureKind::Rate { per: Period::Week },
        confidence: Confidence {
            claim: state.returns_claim.unwrap_or(ClaimKind::Estimated),
            ..Confidence::estimated(
                Interval::exact(state.returns_hours_per_week),
                "hours returned per week",
            )
        },
    };
    coverage_time(&level, &rate)
        .ok()
        .map(|q| q.value)
        .filter(|v| v.is_finite())
}

fn claim_word(claim: ClaimKind) -> String {
    serde_yaml::to_string(&claim)
        .unwrap_or_default()
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Spending time
// ---------------------------------------------------------------------------

/// What `epr flow habit spend` recorded.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendOutcome {
    pub habit: String,
    pub hours: f64,
    pub meter: String,
    pub claim: ClaimKind,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
    pub occurred_at: String,
    pub record_cid: String,
    pub appended: bool,
}

/// Record hours spent on a habit: one `Consume` of the time resource, scoped to the habit.
///
/// It names no promise (`fulfills` stays empty): time spent is not a discharge. Dated at HEAD's
/// author date and attributed by `epr flow note`'s own arms, so the steward stays answerable for
/// time an agent spent in the tree.
pub fn spend(
    root: &Path,
    habit: &str,
    hours: f64,
    meter: &str,
    evidence: &str,
    actor: &super::note::NoteActor,
) -> FlowResult<SpendOutcome> {
    let habit = habit.trim();
    if !super::registers::read_habits(root)?
        .iter()
        .any(|h| h.id == habit)
    {
        return Err(FlowError::InvalidArguments(format!(
            "--habit `{habit}` is not a declared habit"
        )));
    }
    if !hours.is_finite() || hours <= 0.0 {
        return Err(FlowError::InvalidArguments(format!(
            "--hours must be a positive number, got {hours}"
        )));
    }
    let Some((meter, claim)) = METERS.iter().find(|(name, _)| *name == meter.trim()) else {
        let names: Vec<&str> = METERS.iter().map(|(name, _)| *name).collect();
        return Err(FlowError::InvalidArguments(format!(
            "--meter `{meter}` is not one of {}",
            names.join("|")
        )));
    };
    let evidence = super::note::non_empty(evidence, "--evidence")?;
    let named = super::note::named_identity(actor.as_ref.as_deref())?;
    let (author, occurred_at) = super::head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(
            "cannot date time spent: git has no HEAD commit — time is dated by the tree it was \
             spent in, never by wall clock"
                .into(),
        )
    })?;
    let attribution =
        super::note::resolve_attribution(root, named, actor.session.as_deref(), &author);
    let mut classified_as = vec![
        format!("cost:{meter}"),
        habit.to_string(),
        format!("claim:{}", claim_word(*claim)),
        format!("evidence:{evidence}"),
    ];
    attribution.append_slots(&mut classified_as);
    let event = FlowEvent {
        action: ReaVerb::Consume,
        provider: AgentRef(attribution.provider(&author)),
        receiver: super::repository_agent(root)?,
        resource: time_resource()?,
        quantity: Magnitude::Count {
            value: hours,
            unit: HOUR_UNIT.to_string(),
        },
        process: None,
        in_scope_of: habit_resource(habit)?,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: occurred_at.clone(),
        classified_as,
    };
    let record = FlowRecord::Event(event);
    let record_cid = record.cid()?;
    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let appended = !store.records()?.iter().any(|(cid, _)| *cid == record_cid);
    if appended {
        store.append(record)?;
    }
    Ok(SpendOutcome {
        habit: habit.to_string(),
        hours,
        meter: meter.to_string(),
        claim: *claim,
        evidence: evidence.to_string(),
        actor: attribution.actor.clone(),
        steward: attribution.steward.clone(),
        occurred_at,
        record_cid: record_cid.to_string(),
        appended,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(status: &str, extra: &str) -> String {
        format!(
            "---\nepr-habit-version: 1\nid: demo\ninvariant: Works\nstatus: {status}\nchecks:\n  - \"a2o @concern:alpha and @concern:beta\"\nretire-when: never again\n{extra}---\nbody\n"
        )
    }

    #[test]
    fn checks_are_parts_and_unmeasured_parts_hold_the_floor_down() {
        let state = evaluate(&atom("red", ""), None).unwrap();
        let names: Vec<&str> = state.parts.iter().map(|p| p.part.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
        assert_eq!(state.composed.floor, 0.0);
        assert_eq!(state.composed.over_measured, None);
        assert_eq!(state.red_mass, 1.0);
    }

    #[test]
    fn a_declared_reading_merges_into_its_check_and_moves_the_floor() {
        let extra = "rea:\n  parts:\n    - part: alpha\n      reading: 3/4\n      at: 2026-10-18\n      evidence: run-1\n";
        let state = evaluate(&atom("red", extra), Some("2026-10-19T00:00:00Z")).unwrap();
        assert_eq!(state.parts.len(), 2, "merged, not a third part");
        assert_eq!(state.parts[0].source, "check");
        assert_eq!(state.parts[0].progress, Some(0.75));
        assert_eq!(state.parts[0].claim, Some(ClaimKind::InstrumentMeasured));
        assert_eq!(state.composed.floor, 0.375);
        assert_eq!(state.composed.over_measured, Some(0.75));
        assert!((state.red_mass - 0.625).abs() < 1e-12);
    }

    #[test]
    fn a_reading_without_evidence_or_from_the_future_is_unmeasured_not_zero() {
        let missing =
            "rea:\n  parts:\n    - part: alpha\n      reading: 1/1\n      at: 2026-10-18\n";
        let state = evaluate(&atom("red", missing), None).unwrap();
        assert_eq!(state.parts[0].progress, None);
        assert!(state.parts[0].invalid.is_some());

        let future = "rea:\n  parts:\n    - part: alpha\n      reading: 1/1\n      at: 2026-10-20\n      evidence: r\n";
        let state = evaluate(&atom("red", future), Some("2026-10-19T12:00:00Z")).unwrap();
        assert!(state.parts[0].invalid.as_deref().unwrap().contains("later"));

        let routed = "rea:\n  parts:\n    - part: alpha\n      reading: 1/1\n      at: 2026-10-18\n      evidence: \"@concern:gamma\"\n";
        let state = evaluate(&atom("red", routed), None).unwrap();
        assert!(state.parts[0]
            .invalid
            .as_deref()
            .unwrap()
            .contains("@concern"));
    }

    #[test]
    fn green_carries_no_red_mass_and_composition_is_declared() {
        let extra = "rea:\n  composition: min\n  parts:\n    - part: alpha\n      reading: pass\n      at: 2026-10-18\n      evidence: r\n";
        let red = evaluate(&atom("red", extra), None).unwrap();
        assert_eq!(red.composed.rule, Composition::Min);
        assert_eq!(
            red.composed.floor, 0.0,
            "beta unmeasured holds a min floor at zero"
        );
        assert_eq!(evaluate(&atom("green", extra), None).unwrap().red_mass, 0.0);
        let bad = "rea:\n  composition: mean\n";
        let state = evaluate(&atom("red", bad), None).unwrap();
        assert_eq!(state.composed.rule, Composition::Sum);
        assert_eq!(state.declaration_issues.len(), 1);
    }

    #[test]
    fn a_recurring_time_gauge_returns_hours_and_a_budget_is_a_bound_in_hours() {
        let extra = "rea:\n  budget:\n    hours: 40\n  parts:\n    - part: wait\n      unit: s\n      baseline: 43\n      target: 12\n      reading: 10\n      at: 2026-10-18\n      evidence: ab-run\n      per_week: 40\n";
        let state = evaluate(&atom("red", extra), None).unwrap();
        let wait = state.parts.iter().find(|p| p.part == "wait").unwrap();
        assert_eq!(wait.progress, Some(1.0));
        let hours = wait.returns_hours_per_week.unwrap();
        assert!((hours - 33.0 * 40.0 / 3600.0).abs() < 1e-12);
        // An estimated frequency weakens a measured reading.
        assert_eq!(wait.returns_claim, Some(ClaimKind::Estimated));
        let budget = state.budget.unwrap();
        assert_eq!(
            (budget.limit, budget.unit.as_str(), budget.threshold_pct),
            (40.0, "hour", 85.0)
        );
    }

    #[test]
    fn movement_names_rescopes_apart_from_progress() {
        let before = evaluate(&atom("red", "rea:\n  parts:\n    - part: alpha\n      reading: 1/4\n      at: 2026-10-18\n      evidence: r\n"), None).unwrap();
        let progressed = evaluate(&atom("red", "rea:\n  parts:\n    - part: alpha\n      reading: 3/4\n      at: 2026-10-18\n      evidence: r\n"), None).unwrap();
        let rescoped = evaluate(&atom("red", "rea:\n  parts:\n    - part: alpha\n      reading: 3/3\n      at: 2026-10-18\n      evidence: r\n"), None).unwrap();
        let flipped = evaluate(&atom("green", ""), None).unwrap();
        assert_eq!(movement(&before, &progressed), "progress");
        assert_eq!(movement(&progressed, &before), "regression");
        assert_eq!(movement(&before, &rescoped), "rescope");
        assert_eq!(movement(&before, &flipped), "flip-green");
        assert_eq!(movement(&flipped, &before), "flip-red");
    }
}
