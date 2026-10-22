//! `epr flow sense` — the measure plane's sensing organ.
//!
//! The registry in `.claude/epr-meta/measures.yaml` DECLARES what is worth measuring; until this
//! module, nothing in it RAN. Every measure carried a prose `procedure:` and the only folds that
//! existed were the ones some unrelated script happened to write, so most declared bounds read
//! `skipped — no fold` forever: a sensor declared and never wired is numbness that looks like
//! calm. A measure row may now carry a `probe:` block (see [`super::measures::Probe`]) and this
//! verb runs it, folding what it reports through the SAME native observation writer
//! (`note::observe`) every other producer uses. There is no second store and no second judge:
//! the report evaluates sensed folds exactly as it evaluates hand-written ones.
//!
//! **Two shapes.** A `count` probe yields one magnitude, folded on subject `.` when it differs
//! from the latest reading. A `candidates` probe yields a list of subjects — places that may
//! bypass a native primitive, duplicate an implementation, or carry a smell. Each NEW candidate is
//! folded with value 1; each candidate that DISAPPEARED is folded with value 0. That transition
//! rule is the whole lifecycle: `distinct-subjects-since-reset` (the derive the cleanup trigger
//! already uses) counts subjects whose latest fold is non-zero, so a finding opens, closes and
//! re-opens with no JSONL ledger, no cursor and no fingerprint file. A candidate whose file has
//! since been deleted cannot be healed with a zero (an observation needs a readable subject), so
//! the run folds the measure's declared `<id>-reset@1` and re-opens the live set after it.
//!
//! **Detection proposes; a ruling decides.** A candidate is a question, not a finding. Its answer
//! is a native `run:ruling` note on the candidate's own file — the owning boundary — whose reason
//! opens `sense <measure@v> <class>:` followed by the contract difference that justifies it
//! (`epr flow sense rule …` writes one). `intentional`, `canonical` and `false-positive` rulings
//! EXCEPT the subject: it is folded closed, counted separately and reported by name, so restraint
//! is visible rather than silent. `duplication` and `unresolved` rulings annotate an open
//! candidate and except nothing — an unresolved class narrows the next look, it does not grant
//! permission to leave the code alone. Rulings live in the fold plane on purpose: an exception
//! list in YAML would need a second reader, and a second reader is the very duplication this
//! organ exists to find.
//!
//! **Bounded.** Every probe runs under its declared `timeout-seconds:` (default
//! [`super::measures::PROBE_TIMEOUT_DEFAULT_SECONDS`]) and is killed past it. A probe that fails,
//! times out or prints something other than the declared JSON writes NOTHING and reports
//! `refused`: a crashed detector is not a measured zero.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::{Deserialize, Serialize};

use super::measures::{DeclaredProbe, Emits, MeasureRef, ProbeKind, Registry};
use super::note::{self, normalize_subject, NoteActor, REASON_SLOT_PREFIX, REPO_SUBJECT};
use super::report::{latest_reset, outstanding, read_folds, Fold};
use super::{FlowError, FlowResult};

/// The slot-0 tag a ruling note carries.
const RULING_TAG: &str = "run:ruling";

/// The word a sense ruling's reason opens with: `sense <measure@v> <class>: <why>`.
pub const RULING_LEAD: &str = "sense";

/// The measure that, when declared, receives one fold per (measure, class) — the learning
/// slot's precision reading, in the plane where every other reading lives.
pub const CLASSES_MEASURE: &str = "sense-classes";

/// The env key that separates accumulation windows. Every candidate fold carries the short CID of
/// the reset its window opened after (`0` before any reset), so a subject re-opened after a reset
/// mints a new record rather than colliding with — and being deduplicated into — the one it
/// carried before the reset.
pub const EPOCH_ENV_KEY: &str = "epoch";

/// The env key carrying a candidate fold's transition ordinal: how many folds this measure already
/// holds on this subject. Without it a subject that opens, heals and re-opens with identical bytes
/// at one HEAD would mint the SAME record as its first opening — deduplicated into history, so
/// the lens would keep reading it healed. Deterministic, so a re-run is still a no-op.
pub const TRANSITION_ENV_KEY: &str = "transition";

/// A contract inspection's answer about one candidate (notes §4's three classes, plus the owner
/// and the detector's own miss). Deliberately NOT the review-seat `verdict` vocabulary
/// (`approved`/`changes-requested`): that judges a delivery, this classifies a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    /// Evidence-supported duplication: a second implementation of a contract an owned primitive
    /// already holds. Stays open until migrated.
    Duplication,
    /// A genuine semantic difference — a trust boundary, an error contract, a deployment
    /// constraint — that justifies the separate code. Excepted; kept as a negative example.
    Intentional,
    /// The owner every other candidate should converge on. Excepted.
    Canonical,
    /// Not yet decided. Stays open and narrows the next investigation.
    Unresolved,
    /// The detector was wrong about this subject. Excepted; counts against its precision.
    FalsePositive,
}

impl Classification {
    pub const ALL: [Classification; 5] = [
        Classification::Duplication,
        Classification::Intentional,
        Classification::Canonical,
        Classification::Unresolved,
        Classification::FalsePositive,
    ];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().trim_end_matches(':') {
            "duplication" => Some(Classification::Duplication),
            "intentional" => Some(Classification::Intentional),
            "canonical" => Some(Classification::Canonical),
            "unresolved" => Some(Classification::Unresolved),
            "false-positive" => Some(Classification::FalsePositive),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Duplication => "duplication",
            Classification::Intentional => "intentional",
            Classification::Canonical => "canonical",
            Classification::Unresolved => "unresolved",
            Classification::FalsePositive => "false-positive",
        }
    }

    /// Whether this class takes the subject out of the open set.
    pub fn excepts(self) -> bool {
        matches!(
            self,
            Classification::Intentional | Classification::Canonical | Classification::FalsePositive
        )
    }
}

/// One sense ruling read back out of the sidecar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ruling {
    pub measure: String,
    pub subject: String,
    pub class: Classification,
    pub why: String,
    pub cid: String,
    #[serde(skip)]
    order: (String, usize),
}

/// Parse a ruling reason of the form `sense <measure@v> <class>: <why>`.
fn parse_ruling_reason(reason: &str) -> Option<(MeasureRef, Classification, String)> {
    let rest = reason.trim().strip_prefix(RULING_LEAD)?.trim_start();
    let (pin, rest) = rest.split_once(char::is_whitespace)?;
    let measure = MeasureRef::parse(pin, Path::new("")).ok()?;
    let rest = rest.trim_start();
    let (word, why) = match rest.split_once(':') {
        Some((word, why)) => (word, why.trim()),
        None => (rest, ""),
    };
    let class = Classification::parse(word)?;
    Some((measure, class, why.to_string()))
}

/// The ruling reason [`rule`] writes — the inverse of [`parse_ruling_reason`].
pub fn ruling_reason(measure: &MeasureRef, class: Classification, why: &str) -> String {
    format!("{RULING_LEAD} {measure} {}: {}", class.as_str(), why.trim())
}

/// Every sense ruling in the sidecar, keyed by `(measure, subject)` — the LATEST one per key, so a
/// re-classification supersedes the earlier answer without deleting it.
pub fn rulings(root: &Path) -> FlowResult<BTreeMap<(String, String), Ruling>> {
    let mut out: BTreeMap<(String, String), Ruling> = BTreeMap::new();
    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Ok(out);
    }
    let records = SidecarFlowStore::open(root)?.records()?;
    for (seq, (cid, record)) in records.into_iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if event.classified_as.first().map(String::as_str) != Some(RULING_TAG) {
            continue;
        }
        let Some(subject) = event.classified_as.get(1) else {
            continue;
        };
        let Some(reason) = event
            .classified_as
            .iter()
            .find_map(|slot| slot.strip_prefix(REASON_SLOT_PREFIX))
        else {
            continue;
        };
        let Some((measure, class, why)) = parse_ruling_reason(reason) else {
            continue;
        };
        let subject = normalize_subject(subject).to_string();
        let key = (measure.to_string(), subject.clone());
        let order = (event.occurred_at.clone(), seq);
        if out.get(&key).is_some_and(|held| held.order >= order) {
            continue;
        }
        out.insert(
            key,
            Ruling {
                measure: measure.to_string(),
                subject,
                class,
                why,
                cid: cid.to_string(),
                order,
            },
        );
    }
    Ok(out)
}

/// Write one sense ruling as a native `ruling` note on the candidate's own file.
pub fn rule(
    root: &Path,
    measures_path: &Path,
    measure_raw: &str,
    subject: &str,
    class: Classification,
    why: &str,
    actor: &NoteActor,
) -> FlowResult<note::NoteOutcome> {
    let measure = MeasureRef::parse(measure_raw, measures_path)?;
    let registry = Registry::open(measures_path)?;
    if !registry.declares(&measure) {
        return Err(FlowError::InvalidArguments(format!(
            "unknown measure `{measure}` — a ruling answers a candidate of a DECLARED measure \
             ({})",
            measures_path.display()
        )));
    }
    if why.trim().is_empty() {
        return Err(FlowError::InvalidArguments(
            "a ruling needs --why: the contract difference (inputs, errors, authority, \
             persistence, trust boundary) that justifies the class"
                .into(),
        ));
    }
    note::note_with_options_closing(
        root,
        subject,
        "ruling",
        &ruling_reason(&measure, class, why),
        None,
        None,
        None,
        actor,
        &super::acceptance::AcceptanceOptions::default(),
    )
}

/// One subject a probe reported.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Candidate {
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// What a probe prints on stdout: one JSON object.
#[derive(Debug, Clone, Default, Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    candidates: Vec<Candidate>,
    /// String-valued env pairs folded onto a `count` reading. Ignored for candidates: their key is
    /// the subject and the window, never the probe's run-to-run context.
    #[serde(default)]
    env: BTreeMap<String, String>,
    /// Lines a person reads; never folded.
    #[serde(default)]
    detail: Vec<String>,
}

/// A candidate the ruling plane excepted, reported by name.
#[derive(Debug, Clone, Serialize)]
pub struct Excepted {
    pub subject: String,
    pub class: Classification,
    pub why: String,
}

/// How one probe run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SenseStatus {
    Sensed,
    DryRun,
    Refused,
}

/// The outcome of sensing one measure.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenseOutcome {
    pub measure: String,
    pub emits: &'static str,
    pub status: SenseStatus,
    /// The count, or the number of OPEN candidates after this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// Candidates newly opened (folded 1).
    pub opened: Vec<String>,
    /// Candidates healed (folded 0): gone from the probe's output, or excepted by a ruling.
    pub closed: Vec<String>,
    /// Every candidate open after this run, with the symbols the probe named.
    pub open: BTreeMap<String, Vec<String>>,
    pub excepted: Vec<Excepted>,
    /// Class counts over the candidates the probe reported this run, plus `unruled`.
    pub classes: BTreeMap<String, usize>,
    /// The reset this run folded: an open candidate's file no longer exists, or a clean first
    /// reading opened the window (see `fold_candidates`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<String>,
    /// Open candidates that could be neither healed nor reset (deleted file, no declared reset).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stuck: Vec<String>,
    /// Every record this run appended.
    pub written: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub detail: Vec<String>,
}

impl SenseOutcome {
    fn new(declared: &DeclaredProbe, status: SenseStatus) -> Self {
        Self {
            measure: declared.measure.to_string(),
            emits: declared.probe.emits.as_str(),
            status,
            value: None,
            refused: None,
            opened: Vec::new(),
            closed: Vec::new(),
            open: BTreeMap::new(),
            excepted: Vec::new(),
            classes: BTreeMap::new(),
            reset: None,
            stuck: Vec::new(),
            written: Vec::new(),
            detail: Vec::new(),
        }
    }

    fn refused(declared: &DeclaredProbe, reason: String) -> Self {
        let mut out = Self::new(declared, SenseStatus::Refused);
        out.refused = Some(reason);
        out
    }

    pub fn render(&self) {
        let value = self.value.map(|v| format!(" = {v}")).unwrap_or_default();
        match self.status {
            SenseStatus::Refused => println!(
                "sense  {}  REFUSED — {}",
                self.measure,
                self.refused.as_deref().unwrap_or("no reason given")
            ),
            status => println!(
                "sense  {}{value}  ({}{})",
                self.measure,
                self.emits,
                if status == SenseStatus::DryRun {
                    ", dry run — nothing written"
                } else {
                    ""
                }
            ),
        }
        for (subject, symbols) in &self.open {
            if symbols.is_empty() {
                println!("  open      {subject}");
            } else {
                println!("  open      {subject}  ({})", symbols.join(", "));
            }
        }
        for subject in &self.opened {
            println!("  +opened   {subject}");
        }
        for subject in &self.closed {
            println!("  -closed   {subject}");
        }
        for excepted in &self.excepted {
            println!(
                "  excepted  {}  [{}] {}",
                excepted.subject,
                excepted.class.as_str(),
                excepted.why
            );
        }
        for subject in &self.stuck {
            println!("  stuck     {subject}  (file gone and no `<id>-reset@1` declared)");
        }
        if let Some(cid) = &self.reset {
            println!("  reset     {}", short(cid));
        }
        if !self.classes.is_empty() {
            let parts: Vec<String> = self
                .classes
                .iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect();
            println!("  classes   {}", parts.join(" · "));
        }
        for line in &self.detail {
            println!("  · {line}");
        }
    }
}

/// Which declared probes a run selects.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub measures: Vec<String>,
    pub family: Option<String>,
    pub due: Option<String>,
}

impl Selection {
    fn admits(&self, declared: &DeclaredProbe) -> bool {
        let by_measure = self.measures.is_empty()
            || self.measures.iter().any(|m| {
                let m = m.trim();
                m == declared.measure.to_string() || m == declared.measure.id
            });
        let by_family = self
            .family
            .as_deref()
            .is_none_or(|f| declared.family.as_deref() == Some(f));
        let by_due = self
            .due
            .as_deref()
            .is_none_or(|d| declared.probe.due.as_deref() == Some(d));
        by_measure && by_family && by_due
    }
}

/// Run every selected probe and fold what each reports.
pub fn sense(
    root: &Path,
    measures_path: &Path,
    selection: &Selection,
    dry_run: bool,
    actor: &NoteActor,
) -> FlowResult<Vec<SenseOutcome>> {
    let registry = Registry::open(measures_path)?;
    let probes: Vec<DeclaredProbe> = registry
        .probes()
        .into_iter()
        .filter(|p| selection.admits(p))
        .collect();
    if probes.is_empty() && !selection.measures.is_empty() {
        return Err(FlowError::InvalidArguments(format!(
            "no declared probe matches {} — a measure is sensed only when its row in {} carries \
             a `probe:` block",
            selection.measures.join(", "),
            measures_path.display()
        )));
    }
    let ctx = Context {
        root,
        measures_path,
        registry: &registry,
        dry_run,
        actor,
    };
    let mut out = Vec::with_capacity(probes.len());
    for declared in &probes {
        let outcome = match run_probe(root, declared) {
            Err(reason) => SenseOutcome::refused(declared, reason),
            Ok(output) => match declared.probe.emits {
                Emits::Count => ctx.fold_count(declared, output)?,
                Emits::Candidates => ctx.fold_candidates(declared, output)?,
            },
        };
        out.push(outcome);
    }
    Ok(out)
}

struct Context<'a> {
    root: &'a Path,
    measures_path: &'a Path,
    registry: &'a Registry,
    dry_run: bool,
    actor: &'a NoteActor,
}

impl Context<'_> {
    fn status(&self) -> SenseStatus {
        if self.dry_run {
            SenseStatus::DryRun
        } else {
            SenseStatus::Sensed
        }
    }

    fn observe(
        &self,
        measure: &MeasureRef,
        subject: &str,
        value: f64,
        env: &BTreeMap<String, String>,
    ) -> FlowResult<Option<String>> {
        if self.dry_run {
            return Ok(None);
        }
        let outcome = note::observe(
            self.root,
            "observation",
            &measure.to_string(),
            subject,
            value,
            None,
            env,
            None,
            self.actor,
            self.measures_path,
        )?;
        Ok(Some(outcome.record_cid))
    }

    /// Fold `value` on `(measure, subject)` unless the latest fold there already says the same
    /// thing under the same env — the store grows on change, not on every run.
    fn observe_if_changed(
        &self,
        folds: &[Fold],
        measure: &MeasureRef,
        subject: &str,
        value: f64,
        env: &BTreeMap<String, String>,
    ) -> FlowResult<Option<String>> {
        let same = folds
            .iter()
            .filter(|f| {
                f.measure == *measure
                    && normalize_subject(&f.subject) == normalize_subject(subject)
                    && env.iter().all(|(k, v)| f.env.get(k) == Some(v))
            })
            .max_by(|a, b| a.order_key().cmp(&b.order_key()))
            .is_some_and(|latest| latest.value == value && latest.env == *env);
        if same {
            return Ok(None);
        }
        self.observe(measure, subject, value, env)
    }

    fn fold_count(
        &self,
        declared: &DeclaredProbe,
        output: ProbeOutput,
    ) -> FlowResult<SenseOutcome> {
        let Some(value) = output.value.filter(|v| v.is_finite()) else {
            return Ok(SenseOutcome::refused(
                declared,
                "a `count` probe must print a finite `value`".into(),
            ));
        };
        let mut out = SenseOutcome::new(declared, self.status());
        out.value = Some(value);
        out.detail = output.detail;
        for candidate in &output.candidates {
            out.detail.push(match &candidate.symbol {
                Some(symbol) => format!("{} ({symbol})", candidate.subject),
                None => candidate.subject.clone(),
            });
        }
        let folds = read_folds(self.root)?;
        if let Some(cid) =
            self.observe_if_changed(&folds, &declared.measure, REPO_SUBJECT, value, &output.env)?
        {
            out.written.push(cid);
        }
        Ok(out)
    }

    fn fold_candidates(
        &self,
        declared: &DeclaredProbe,
        output: ProbeOutput,
    ) -> FlowResult<SenseOutcome> {
        let measure = &declared.measure;
        let pin = measure.to_string();

        // Validate every subject before anything is written: one escaping path refuses the run.
        let mut reported: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for candidate in &output.candidates {
            let subject = match confine_subject(&candidate.subject) {
                Ok(subject) => subject,
                Err(reason) => return Ok(SenseOutcome::refused(declared, reason)),
            };
            let symbols = reported.entry(subject).or_default();
            if let Some(symbol) = &candidate.symbol {
                if !symbols.contains(symbol) {
                    symbols.push(symbol.clone());
                }
            }
        }

        let mut out = SenseOutcome::new(declared, self.status());
        out.detail = output.detail;

        // Rulings decide what is open. A subject the probe did not report this run has nothing to
        // answer, so its ruling is not counted toward this run's classes.
        let rulings = rulings(self.root)?;
        let mut live: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        for (subject, symbols) in reported {
            match rulings.get(&(pin.clone(), subject.clone())) {
                Some(ruling) => {
                    *classes
                        .entry(ruling.class.as_str().to_string())
                        .or_default() += 1;
                    if ruling.class.excepts() {
                        out.excepted.push(Excepted {
                            subject,
                            class: ruling.class,
                            why: ruling.why.clone(),
                        });
                        continue;
                    }
                }
                None => *classes.entry("unruled".to_string()).or_default() += 1,
            }
            live.insert(subject, symbols);
        }

        let reset = MeasureRef {
            id: format!("{}-reset", measure.id),
            version: 1,
        };
        let mut folds = read_folds(self.root)?;
        let mut open_now: BTreeSet<String> = outstanding(&folds, &reset, |f| f.measure == *measure)
            .into_iter()
            .map(|f| normalize_subject(&f.subject).to_string())
            .collect();

        // An open subject whose file is gone cannot take a healing zero. Drain the window with the
        // declared reset and re-open the live set after it; without a declared reset, say so.
        let gone: Vec<String> = open_now
            .iter()
            .filter(|s| !live.contains_key(*s) && !self.root.join(s).is_file())
            .cloned()
            .collect();
        if !gone.is_empty() {
            if self.registry.declares(&reset) {
                let env = BTreeMap::from([("measure".to_string(), pin.clone())]);
                if let Some(cid) = self.observe(&reset, REPO_SUBJECT, 1.0, &env)? {
                    out.written.push(cid.clone());
                    out.reset = Some(cid);
                }
                out.closed.extend(gone.iter().cloned());
                open_now.clear();
                folds = read_folds(self.root)?;
            } else {
                out.stuck = gone.clone();
            }
        }

        // A clean first reading is still a reading. With no fold of this measure and no reset, an
        // empty open set would write nothing and the bound would read `skipped — no fold`: the
        // numbness this organ exists to remove, wearing the face of health. Open the window with
        // the declared reset instead, so the derive reads 0 against its watermark.
        let never_observed =
            latest_reset(&folds, &reset).is_none() && !folds.iter().any(|f| f.measure == *measure);
        if never_observed && live.is_empty() && self.registry.declares(&reset) {
            let env = BTreeMap::from([
                ("measure".to_string(), pin.clone()),
                ("opens".to_string(), "first-reading".to_string()),
            ]);
            if let Some(cid) = self.observe(&reset, REPO_SUBJECT, 1.0, &env)? {
                out.written.push(cid.clone());
                out.reset = Some(cid);
            }
            folds = read_folds(self.root)?;
        }

        let epoch = match latest_reset(&folds, &reset) {
            Some(fold) => short(fold.cid()),
            None if out.reset.is_some() => "pending".to_string(),
            None => "0".to_string(),
        };
        let transition = |subject: &str| -> BTreeMap<String, String> {
            let prior = folds
                .iter()
                .filter(|f| {
                    f.measure == *measure
                        && normalize_subject(&f.subject) == normalize_subject(subject)
                })
                .count();
            BTreeMap::from([
                (EPOCH_ENV_KEY.to_string(), epoch.clone()),
                (TRANSITION_ENV_KEY.to_string(), (prior + 1).to_string()),
            ])
        };

        for subject in live.keys() {
            if open_now.contains(subject) {
                continue;
            }
            if let Some(cid) = self.observe(measure, subject, 1.0, &transition(subject))? {
                out.written.push(cid);
            }
            out.opened.push(subject.clone());
        }
        for subject in &open_now {
            if live.contains_key(subject) || out.stuck.contains(subject) {
                continue;
            }
            if let Some(cid) = self.observe(measure, subject, 0.0, &transition(subject))? {
                out.written.push(cid);
            }
            out.closed.push(subject.clone());
        }

        out.value = Some(live.len() as f64 + out.stuck.len() as f64);
        out.open = live;
        for subject in &out.stuck {
            out.open.entry(subject.clone()).or_default();
        }

        // The learning slot's reading: class counts, folded only when the registry declares the
        // measure that receives them.
        let classes_pin = MeasureRef {
            id: CLASSES_MEASURE.to_string(),
            version: 1,
        };
        if self.registry.declares(&classes_pin) {
            let folds = read_folds(self.root)?;
            let names = Classification::ALL
                .iter()
                .map(|v| v.as_str().to_string())
                .chain(std::iter::once("unruled".to_string()));
            for class in names {
                let count = classes.get(&class).copied().unwrap_or(0) as f64;
                let env = BTreeMap::from([
                    ("measure".to_string(), pin.clone()),
                    ("class".to_string(), class),
                ]);
                if let Some(cid) =
                    self.observe_if_changed(&folds, &classes_pin, REPO_SUBJECT, count, &env)?
                {
                    out.written.push(cid);
                }
            }
        }
        out.classes = classes;
        Ok(out)
    }
}

/// The crate's one short form of a CID, for the window label; an unparsable CID is labelled whole.
fn short(cid: &str) -> String {
    cid.parse::<cid::Cid>()
        .map(|cid| super::short_cid(&cid))
        .unwrap_or_else(|_| cid.to_string())
}

/// A probe-reported subject must be a repository-relative path that stays inside the tree.
fn confine_subject(raw: &str) -> Result<String, String> {
    let subject = raw.trim().trim_start_matches("./");
    if subject.is_empty() || subject == REPO_SUBJECT {
        return Err(format!(
            "candidate subject `{raw}` names the repository itself — a candidate is a file"
        ));
    }
    let path = Path::new(subject);
    if path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "candidate subject `{raw}` escapes the repository — subjects are repo-relative paths"
        ));
    }
    Ok(subject.to_string())
}

/// The probes compiled into `epr`, by the name a `probe: { native: … }` block declares.
///
/// The measure plane's own primitives — the concern ontology first — are native: durable, fast
/// and one implementation. A shell probe is the disposable form, a sketch of a detector while its
/// procedure and its precision are still being learned.
pub const NATIVE_PROBES: [&str; 2] = ["concern-joins", "concern-evidence-unadopted"];

/// Run one native probe in-process.
fn native_probe(root: &Path, name: &str) -> Result<ProbeOutput, String> {
    match name {
        "concern-joins" => {
            let joins = super::concern::unresolved_joins(root).map_err(|e| e.to_string())?;
            Ok(ProbeOutput {
                candidates: joins
                    .into_iter()
                    .map(|join| Candidate {
                        subject: join.subject,
                        symbol: Some(join.references.join(",")),
                        signature: Some("concern-reference-unresolved".into()),
                    })
                    .collect(),
                ..ProbeOutput::default()
            })
        }
        "concern-evidence-unadopted" => {
            let waiting = super::concern::unadopted_stories(root).map_err(|e| e.to_string())?;
            Ok(ProbeOutput {
                value: Some(waiting.len() as f64),
                detail: waiting
                    .iter()
                    .map(|(tag, files)| {
                        let shown: Vec<&str> = files.iter().take(2).map(String::as_str).collect();
                        format!("{tag} ({})", shown.join(", "))
                    })
                    .collect(),
                ..ProbeOutput::default()
            })
        }
        other => Err(format!(
            "unknown native probe `{other}` — this binary carries {}",
            NATIVE_PROBES.join(", ")
        )),
    }
}

/// Run one probe — natively in-process, or as a shell command under its declared budget.
fn run_probe(root: &Path, declared: &DeclaredProbe) -> Result<ProbeOutput, String> {
    let run = match &declared.probe.kind {
        ProbeKind::Native(name) => return native_probe(root, name),
        ProbeKind::Shell(run) => run.as_str(),
    };
    let root_str = root.to_string_lossy().to_string();
    let pin = declared.measure.to_string();
    let mut child = crate::process::build_command(
        "sh",
        &["-c", run],
        root,
        &[
            ("EPR_SENSE_ROOT", root_str.as_str()),
            ("EPR_SENSE_MEASURE", pin.as_str()),
        ],
    )
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("could not start `{run}`: {e}"))?;

    // Drain both pipes on their own threads so a chatty probe cannot fill a pipe and deadlock
    // against the budget loop below.
    let mut stdout = child.stdout.take().ok_or("probe stdout was not captured")?;
    let mut stderr = child.stderr.take().ok_or("probe stderr was not captured")?;
    let out_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stdout.read_to_string(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        buf
    });

    // Bounded by the declared budget: the loop ends at exit or at the deadline, whichever is first.
    let budget = Duration::from_secs_f64(declared.probe.timeout_seconds);
    let deadline = Instant::now() + budget;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "probe exceeded its {}s budget and was killed",
                    declared.probe.timeout_seconds
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(format!("could not wait on the probe: {e}")),
        }
    };
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    if !status.success() {
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
        return Err(format!("probe exited {status}: {tail}"));
    }
    serde_json::from_str::<ProbeOutput>(stdout.trim())
        .map_err(|e| format!("probe printed something other than its declared JSON ({e})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ruling_reason_round_trips() {
        let pin = MeasureRef {
            id: "limit-models".into(),
            version: 1,
        };
        let reason = ruling_reason(
            &pin,
            Classification::Intentional,
            " the YAML declaration reader ",
        );
        assert_eq!(
            reason,
            "sense limit-models@1 intentional: the YAML declaration reader"
        );
        let (measure, class, why) = parse_ruling_reason(&reason).unwrap();
        assert_eq!(measure, pin);
        assert_eq!(class, Classification::Intentional);
        assert_eq!(why, "the YAML declaration reader");
    }

    #[test]
    fn a_reason_that_is_not_a_sense_ruling_is_ignored() {
        assert!(parse_ruling_reason("green->red 2026-10-08: the invariant").is_none());
        assert!(parse_ruling_reason("sense limit-models intentional: no pin").is_none());
        assert!(parse_ruling_reason("sense limit-models@1 lgtm: unknown class").is_none());
    }

    #[test]
    fn only_intentional_canonical_and_false_positive_except() {
        let excepting: Vec<&str> = Classification::ALL
            .iter()
            .filter(|v| v.excepts())
            .map(|v| v.as_str())
            .collect();
        assert_eq!(excepting, ["intentional", "canonical", "false-positive"]);
    }

    #[test]
    fn a_subject_must_stay_inside_the_repository() {
        assert_eq!(confine_subject("./a/b.rs").unwrap(), "a/b.rs");
        assert!(confine_subject("/etc/passwd").is_err());
        assert!(confine_subject("a/../../x").is_err());
        assert!(confine_subject(".").is_err());
    }
}
