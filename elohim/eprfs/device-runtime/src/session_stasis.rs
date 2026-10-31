//! The dialogue-stasis fold: journal events in, incidents and measure readings out.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md` (Incident
//! model, Budgets, Folding and measures, Native CLI contract). This slice is the shadow
//! regulator: it reads, classifies and records `would-have`; it executes nothing, and a
//! consultation's recorded outcome never closes an incident here (a parked or proceeded stall
//! would otherwise stop counting waiting the human is still doing).
//!
//! **The unmeasured rule.** Missing, retained-away or unfolded evidence is reported as `None`,
//! a flag, or a named gap, never as a measured zero.

pub mod classify;
pub mod event;
pub mod journal;

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use elohim_epr::algedonic::{Hysteresis, LatchConfig, LatchSense};
use elohim_epr::verdict::Decision;
use elohim_epr_rea::Bound;
use serde::{Serialize, Serializer};
use serde_json::{json, Value};

use self::classify::{CallInput, Classifier, ConcernClass, EffectClass, StallClass};
use self::event::{
    elapsed, CloseAttribution, Confidence, ConsultationEvent, Disposition, Event, EventKind, Level,
    Moment, Phase, Pivot, Presence, Question,
};
use self::journal::{Replay, StasisError};

/// What `fold` and `decide` report before slice 6 creates the policy file.
pub const POLICY_ABSENT: &str = "absent (defaults, shadow)";

/// Starting budget bounds (policy `budgets`), not measured baselines.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Budgets {
    /// Keeper calls per incident.
    pub incident_keeper_calls: u32,
    /// Elapsed seconds per incident.
    pub incident_elapsed_s: f64,
    /// Tokens per incident.
    pub incident_tokens: u64,
    /// Keeper seconds per shift.
    pub shift_keeper_s: f64,
    /// Tokens per shift.
    pub shift_tokens: u64,
}

/// The built-in policy defaults (every value the Budgets section names).
#[derive(Debug, Clone)]
pub struct StasisPolicy {
    /// Budget bounds.
    pub budgets: Budgets,
    /// The collaborative window T_c, seconds.
    pub collaborative_window_s: f64,
    /// The incident stall bound: Breach at the limit, Approach at `threshold_pct` of it.
    pub stall: Bound,
    /// The cumulative blocked-seconds ceiling per shift.
    pub shift_ceiling: Bound,
    /// Latch dwell for dialogue keys, seconds.
    pub dwell_s: u64,
}

impl StasisPolicy {
    /// The pre-policy defaults: 4 calls, 1800 s, 200k tokens per incident; 7200 s and 2M tokens
    /// per shift; T_c 600 s; Approach 180 s; Breach 300 s; dwell 300 s; ceiling 1800 s.
    pub fn builtin() -> Self {
        Self {
            budgets: Budgets {
                incident_keeper_calls: 4,
                incident_elapsed_s: 1800.0,
                incident_tokens: 200_000,
                shift_keeper_s: 7200.0,
                shift_tokens: 2_000_000,
            },
            collaborative_window_s: 600.0,
            stall: Bound::new(300.0, "seconds".into(), 60.0)
                .expect("300 s at 60% is a valid bound"),
            shift_ceiling: Bound::new(1800.0, "seconds".into(), 100.0)
                .expect("1800 s at 100% is a valid bound"),
            dwell_s: 300,
        }
    }

    /// The latch hysteresis a dialogue incident key uses: entry at the stall's Approach edge,
    /// reset when the stall is closed (stock 0), dwell as declared.
    pub fn hysteresis(&self) -> Hysteresis {
        Hysteresis {
            threshold_pct: self.stall.threshold_pct,
            reset_pct: 0.0,
            dwell_s: self.dwell_s,
        }
    }

    /// A latch declaration for one dialogue key; the coordinator (slice 8) owns the latch file.
    pub fn latch_config(&self, declarer: &str, target: &str, bound_ref: &str) -> LatchConfig {
        LatchConfig {
            declarer: declarer.into(),
            target: target.into(),
            bound_ref: bound_ref.into(),
            limit: self.stall.limit,
            sense: LatchSense::Ceiling,
            hysteresis: self.hysteresis(),
        }
    }

    /// The Approach edge in seconds, derived from the bound.
    pub fn approach_s(&self) -> f64 {
        self.stall.limit * self.stall.threshold_pct / 100.0
    }
}

/// An away ruling and when it lapses. The journal records only a ruling reference, so the caller
/// supplies what it can resolve; a reference it cannot resolve stays in force (unverifiable).
#[derive(Debug, Clone)]
pub struct AwayRuling {
    /// The ruling reference an event's presence basis names.
    pub id: String,
    /// When the ruling expires; `None` does not expire.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Incident lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IncidentState {
    /// Within the first 180 s.
    Open,
    /// At or past 180 s.
    Approach,
    /// At or past 300 s.
    Breach,
    /// Rung 5 reached; awaiting the steward.
    Pending,
    /// The stall is resolved.
    Closed,
}

/// How a stall was resolved. `Reshaped`, `Proceeded` and `Parked` are reserved for live evidence
/// (slices 8-9); a shadow consultation never produces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClosedBy {
    /// The steward approved.
    Approved,
    /// A denial inferred from the next event.
    DeniedInferred,
    /// The steward answered.
    Answered,
    /// A reshape replaced the call.
    Reshaped,
    /// A question was resolved under a validated standing ruling.
    Proceeded,
    /// The step was parked.
    Parked,
    /// The request was withdrawn.
    Withdrawn,
    /// The request expired.
    Expired,
    /// The session ended.
    SessionEnded,
}

/// A blocked interval; `to: null` is open until observation closes it.
#[derive(Debug, Clone, Serialize)]
pub struct Interval {
    /// Start.
    pub from: DateTime<Utc>,
    /// End, or open.
    pub to: Option<DateTime<Utc>>,
}

fn decision_word<S: Serializer>(d: &Option<Decision>, s: S) -> Result<S::Ok, S::Error> {
    match d {
        Some(Decision::Permit) => s.serialize_str("Permit"),
        Some(Decision::Refer(_)) => s.serialize_str("Refer"),
        Some(Decision::Refuse) => s.serialize_str("Refuse"),
        None => s.serialize_none(),
    }
}

/// One consultation, folded from its start, completion and expiry records.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Consultation {
    /// The rung (1-5).
    pub rung: u8,
    /// Capability level.
    pub level: Option<Level>,
    /// Runtime binding.
    pub binding: Option<String>,
    /// `"Permit"` or `"Refer"`, once recorded.
    #[serde(serialize_with = "decision_word")]
    pub decision: Option<Decision>,
    /// The pivot, once recorded.
    pub pivot: Option<Pivot>,
    /// The recorded reason.
    pub reason: Option<String>,
    /// The seq of the start record that claimed it.
    pub claim_seq: Option<u64>,
    /// Claim start.
    pub started_at: Option<Moment>,
    /// Result time (the deadline when it expired).
    pub finished_at: Option<Moment>,
    /// Claim deadline.
    pub deadline_at: Option<DateTime<Utc>>,
    /// Claim holder, `pid@bootId`.
    pub claimed_by: Option<String>,
    /// Measured elapsed seconds reported with the result; `None` is unmeasured.
    pub reported_elapsed_s: Option<f64>,
    /// Reported tokens; `None` is unmeasured.
    pub tokens: Option<u64>,
    /// The deadline passed without a result.
    pub expired: bool,
    /// Duplicate start records folded into this consultation.
    pub duplicate_starts: u32,
}

impl Consultation {
    fn new(rung: u8) -> Self {
        Self {
            rung,
            level: Level::for_rung(rung),
            binding: None,
            decision: None,
            pivot: None,
            reason: None,
            claim_seq: None,
            started_at: None,
            finished_at: None,
            deadline_at: None,
            claimed_by: None,
            reported_elapsed_s: None,
            tokens: None,
            expired: false,
            duplicate_starts: 0,
        }
    }

    fn in_flight(&self) -> bool {
        self.started_at.is_some() && self.finished_at.is_none() && !self.expired
    }

    /// Seconds this consultation took: the reported usage, else its clocks; `None` when neither
    /// is known (a missing start is unmeasured, not zero).
    fn seconds(&self, now: &Moment) -> Option<f64> {
        if let Some(e) = self.reported_elapsed_s {
            return Some(e);
        }
        let start = self.started_at.as_ref()?;
        Some(elapsed(start, self.finished_at.as_ref().unwrap_or(now)))
    }
}

/// Budget consumption. Totals are `None` when any contributing consultation is unmeasured; the
/// `*_known` fields are then a lower bound.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetUsed {
    /// Distinct keeper calls.
    pub keeper_calls: u32,
    /// Seconds since the incident opened (to its close; a parked incident keeps running).
    pub elapsed_s: f64,
    /// Keeper seconds, `None` when any consultation's duration is unmeasured.
    pub keeper_seconds: Option<f64>,
    /// Keeper seconds that are known.
    pub keeper_seconds_known: f64,
    /// Consultations whose duration is unmeasured.
    pub keeper_seconds_unmeasured: u32,
    /// Combined tokens, `None` when any finished consultation reported no usage.
    pub tokens: Option<u64>,
    /// Tokens that were reported.
    pub tokens_known: u64,
    /// Finished consultations that reported no usage.
    pub tokens_unreported: u32,
}

/// The folded incident.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    /// Session incarnation plus the opening event's seq.
    pub id: String,
    /// The session id.
    pub session: String,
    /// The agent whose call or turn opened it.
    pub agent_id: String,
    /// What is waiting.
    pub stall_class: StallClass,
    /// The native classification.
    pub concern_class: ConcernClass,
    /// Why it classified so.
    pub concern_reason: String,
    /// The rule that decided it, when one did.
    pub matched_rule: Option<String>,
    /// The normalized effect.
    pub effect_class: EffectClass,
    /// Opening event time.
    pub opened_at: Moment,
    /// Presence at open, with absence precedence applied.
    pub presence_at_open: Presence,
    /// Current state.
    pub state: IncidentState,
    /// Current rung (0-5), folded from the recorded consultations.
    pub rung: u8,
    /// Folded consultations.
    pub consultations: Vec<Consultation>,
    /// Blocked intervals.
    pub blocked_intervals: Vec<Interval>,
    /// Confidence in the interval's start: low when no dialog notification was seen.
    pub interval_confidence: Confidence,
    /// How it closed.
    pub closed_by: Option<ClosedBy>,
    /// When it closed.
    pub closed_at: Option<DateTime<Utc>>,
    /// True when the close could not be established (never a confident close).
    pub close_unknown: bool,
    /// True when the close is known but its time is not (an approval with no duration).
    pub end_unmeasured: bool,
    /// Budget consumption.
    pub budget_used: BudgetUsed,
    /// Per-incident bounds reached.
    pub budget_exhausted: Vec<String>,
    #[serde(skip)]
    closed_moment: Option<Moment>,
    #[serde(skip)]
    dialog_seen: bool,
    #[serde(skip)]
    reshape_offered: bool,
    #[serde(skip)]
    witness_at_open: Option<DateTime<Utc>>,
}

fn secs(a: DateTime<Utc>, b: DateTime<Utc>) -> f64 {
    (b - a).num_milliseconds() as f64 / 1000.0
}

/// A gap in the evidence, named rather than zeroed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldGap {
    /// The reason (`retention`, `unknown-close`, `backwards-seq`, ...).
    pub reason: String,
    /// The session.
    pub session: String,
    /// The event that recorded or revealed it.
    pub seq: Option<u64>,
    /// Its time; `None` means it cannot be placed in any window.
    pub at: Option<DateTime<Utc>>,
    /// The incident concerned.
    pub incident: Option<String>,
}

/// Which accounting window the caller wants read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// The whole journal as one window.
    All,
    /// From a time to now.
    Since(DateTime<Utc>),
    /// The latest open shift-marker window, else the current session window.
    ShiftCurrent,
}

/// Fold parameters.
#[derive(Debug, Clone, Copy)]
pub struct FoldOptions<'a> {
    /// The clock: open intervals and in-flight consultations are read against it.
    pub now: DateTime<Utc>,
    /// Boot id and uptime for `now`, when known; monotonic time is preferred within one boot.
    pub now_mono: Option<(&'a str, f64)>,
    /// The window to read.
    pub selection: Selection,
    /// True when the caller named the session; a named session's current window is its latest
    /// even when stale, an unnamed sweep reads only live sessions.
    pub explicit_session: bool,
    /// Policy bounds.
    pub policy: &'a StasisPolicy,
    /// The stall classifier.
    pub classifier: &'a Classifier,
    /// Away rulings the caller can resolve.
    pub rulings: &'a [AwayRuling],
}

impl<'a> FoldOptions<'a> {
    /// Options with no monotonic clock and no resolvable rulings.
    pub fn new(
        now: DateTime<Utc>,
        selection: Selection,
        explicit_session: bool,
        policy: &'a StasisPolicy,
        classifier: &'a Classifier,
    ) -> Self {
        Self {
            now,
            now_mono: None,
            selection,
            explicit_session,
            policy,
            classifier,
            rulings: &[],
        }
    }

    /// Add the monotonic reading of `now`.
    pub fn with_clock(mut self, boot_id: &'a str, uptime_seconds: f64) -> Self {
        self.now_mono = Some((boot_id, uptime_seconds));
        self
    }

    /// Add the rulings the caller can resolve.
    pub fn with_rulings(mut self, rulings: &'a [AwayRuling]) -> Self {
        self.rulings = rulings;
        self
    }

    fn now_moment(&self) -> Moment {
        match self.now_mono {
            Some((boot, up)) => Moment {
                wall: self.now,
                boot_id: boot.to_owned(),
                uptime_seconds: up,
            },
            None => Moment::wall_only(self.now),
        }
    }
}

/// Keeper cost attributed to a window by time overlap. Totals are `None` when any
/// contributing figure is unmeasured; `*_known` is then a lower bound.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeeperUse {
    /// Keeper seconds, `None` when any overlapping consultation is unmeasured.
    pub seconds: Option<f64>,
    /// Keeper seconds that are known.
    pub seconds_known: f64,
    /// Overlapping consultations whose duration is unmeasured.
    pub seconds_unmeasured: u32,
    /// Reported tokens, `None` when any finished consultation reported no usage.
    pub tokens: Option<u64>,
    /// Tokens that were reported.
    pub tokens_known: u64,
    /// Finished consultations that reported no usage.
    pub tokens_unreported: u32,
    /// Shift bounds reached by the known figures.
    pub exhausted: Vec<String>,
}

/// Harness exceptions in a window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessReading {
    /// Count of exceptions after the baseline.
    pub count: u64,
    /// Count by hook.
    pub by_hook: BTreeMap<String, u64>,
    /// Historical exceptions read while establishing the transcript cursor, not counted.
    pub baseline_excluded: u64,
}

/// The shift ceiling verdict.
#[derive(Debug, Clone, Serialize)]
pub struct Ceiling {
    /// The limit, seconds.
    pub limit: f64,
    /// Whether the blocked total reached it.
    pub breached: bool,
}

/// One accounting window and what it measured.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReading {
    /// `shift=<id>` for every reading in this window.
    pub id: String,
    /// `marker`, `session`, `all` or `since`.
    pub kind: String,
    /// The session.
    pub session: String,
    /// Window start.
    pub from: DateTime<Utc>,
    /// Window end (now while open).
    pub to: DateTime<Utc>,
    /// Whether the window is still open.
    pub open: bool,
    /// Journal events inside the window, observation gaps included.
    pub events: usize,
    /// Union of blocked seconds; `None` when the window holds no evidence beyond gaps.
    pub value_seconds: Option<f64>,
    /// Union by stall class (each class unioned alone, so classes may overlap).
    pub by_class: BTreeMap<String, f64>,
    /// Union by presence at open.
    pub by_presence: BTreeMap<String, f64>,
    /// Intervals whose start or end is low confidence.
    pub low_confidence_intervals: usize,
    /// Intervals whose end is still open.
    pub open_intervals: usize,
    /// Intervals whose close could not be established.
    pub unknown_closes: usize,
    /// Approvals recorded without a duration, so the end time is unmeasured.
    pub unmeasured_ends: usize,
    /// Observation gaps inside the window plus gaps that cannot be placed in any window.
    pub gaps: usize,
    /// False when gaps, unknown closes or unmeasured ends make `value_seconds` a lower bound.
    pub complete: bool,
    /// The 1800 s ceiling verdict.
    pub ceiling: Option<Ceiling>,
    /// Harness exceptions; `None` when the journal holds no harness-exception evidence at all.
    pub harness_exceptions: Option<HarnessReading>,
    /// Keeper use against the per-shift budget.
    pub keeper: KeeperUse,
}

/// Which rung resolved an incident.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    /// The incident.
    pub incident_id: String,
    /// `C1`..`C4` or `human`.
    pub resolved_by: String,
    /// The binding of the resolving consultation.
    pub binding: Option<String>,
    /// Concern class.
    pub concern_class: ConcernClass,
    /// Presence at open.
    pub presence: Disposition,
}

/// One session, folded.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFold {
    /// The session id.
    pub session: String,
    /// Every incident.
    pub incidents: Vec<Incident>,
    /// Indexes into `incidents` that overlap a selected window.
    #[serde(skip)]
    pub selected_incidents: Vec<usize>,
    /// Evidence gaps.
    pub gaps: Vec<FoldGap>,
    /// Every accounting window.
    pub windows: Vec<WindowReading>,
    /// Indexes into `windows` that the selection chose.
    pub selected: Vec<usize>,
    /// Closed incidents attributed to the rung that resolved them.
    pub ladder_resolution: Vec<Resolution>,
    /// Distinct shadow `would-have` keys seen.
    pub would_have: usize,
    /// The distinct keys.
    pub would_have_keys: Vec<String>,
    /// Harness-classifier denials seen.
    pub classifier_denied: usize,
    /// The session's effective presence at `now`.
    pub presence: Presence,
    /// Time of the last event.
    pub last_event: Option<DateTime<Utc>>,
    /// Whether `session-ended` was seen.
    pub ended: bool,
}

fn parse_seq(id: &str) -> Option<u64> {
    id.rsplit(':').next()?.parse().ok()
}

struct Folder<'a> {
    o: FoldOptions<'a>,
    session: String,
    first_seq: u64,
    incidents: Vec<Incident>,
    by_id: HashMap<String, usize>,
    gaps: Vec<FoldGap>,
    markers: Vec<(String, bool, DateTime<Utc>)>,
    exceptions: Vec<(DateTime<Utc>, String, u64, bool)>,
    would_keys: Vec<String>,
    classifier_denied: usize,
}

impl Folder<'_> {
    fn gap(&mut self, reason: &str, e: &Event, incident: Option<&str>) {
        self.gaps.push(FoldGap {
            reason: reason.into(),
            session: self.session.clone(),
            seq: Some(e.seq),
            at: Some(e.wall),
            incident: incident.map(str::to_owned),
        });
    }

    fn lookup(&mut self, id: &str, e: &Event) -> Option<usize> {
        if let Some(&i) = self.by_id.get(id) {
            return Some(i);
        }
        let dropped = parse_seq(id).is_some_and(|s| s < self.first_seq);
        self.gap(
            if dropped {
                "retention"
            } else {
                "unknown-incident"
            },
            e,
            Some(id),
        );
        None
    }

    fn open(&mut self, e: &Event, id: String, stall: StallClass) {
        if self.by_id.contains_key(&id) {
            return; // A duplicate opening event folds into the one incident.
        }
        let (class, effect) = match stall {
            StallClass::Authorization => {
                let tool = e.tool.as_deref().unwrap_or("unknown");
                let input = e
                    .shape
                    .as_deref()
                    .map_or(CallInput::Opaque, CallInput::Shape);
                let c = self.o.classifier.classify(stall, tool, input);
                (Some(c.clone()), c.effect)
            }
            _ => (None, EffectClass::for_stall(stall)),
        };
        let (concern, reason, rule) = class.map_or(
            (
                ConcernClass::Uncertain,
                "non-authorization-stall".to_owned(),
                None,
            ),
            |c| (c.concern, c.reason, c.matched_rule),
        );
        self.incidents.push(Incident {
            id: id.clone(),
            session: self.session.clone(),
            agent_id: e.agent_id.clone(),
            stall_class: stall,
            concern_class: concern,
            concern_reason: reason,
            matched_rule: rule,
            effect_class: effect,
            opened_at: e.moment(),
            presence_at_open: e.presence.clone(),
            state: IncidentState::Open,
            rung: 0,
            consultations: vec![],
            blocked_intervals: vec![Interval {
                from: e.wall,
                to: None,
            }],
            interval_confidence: if stall == StallClass::Authorization {
                Confidence::Low
            } else {
                Confidence::High
            },
            closed_by: None,
            closed_at: None,
            close_unknown: false,
            end_unmeasured: false,
            budget_used: BudgetUsed::default(),
            budget_exhausted: vec![],
            closed_moment: None,
            dialog_seen: false,
            reshape_offered: false,
            witness_at_open: e.witness_at(),
        });
        self.by_id.insert(id, self.incidents.len() - 1);
    }

    fn close(&mut self, idx: usize, at: Moment, by: Option<ClosedBy>) {
        let inc = &mut self.incidents[idx];
        if inc.closed_at.is_some() {
            return;
        }
        let wall = at.wall.max(inc.opened_at.wall);
        inc.closed_at = Some(wall);
        inc.closed_moment = Some(at);
        inc.closed_by = by;
        inc.close_unknown = by.is_none();
        for iv in &mut inc.blocked_intervals {
            iv.to = Some(wall.max(iv.from));
        }
    }

    fn open_where(&self, stall: StallClass, agent: &str) -> Vec<usize> {
        (0..self.incidents.len())
            .filter(|&i| {
                let inc = &self.incidents[i];
                inc.closed_at.is_none() && inc.stall_class == stall && inc.agent_id == agent
            })
            .collect()
    }

    /// A question is answered when the same agent resumes after a witnessed interaction that
    /// post-dates the question. Another agent's activity, or a witness nobody resumed from, is not
    /// evidence of resolution.
    fn witness(&mut self, e: &Event) {
        let Some(w) = e.witness_at() else { return };
        for i in self.open_where(StallClass::Question, &e.agent_id) {
            if w > self.incidents[i].opened_at.wall {
                let at = e.moment().minus(secs(w, e.wall).max(0.0));
                self.close(i, at, Some(ClosedBy::Answered));
            }
        }
    }

    fn consult(&mut self, e: &Event, c: &ConsultationEvent) {
        let Some(id) = e.incident_ref().map(str::to_owned) else {
            self.gap("consultation-without-incident", e, None);
            return;
        };
        let Some(i) = self.lookup(&id, e) else { return };
        let Some(rung) = c.rung.or(c.level.map(Level::rung)) else {
            self.gap("consultation-without-rung", e, Some(&id));
            return;
        };
        let inc = &mut self.incidents[i];
        // A completion or expiry names its claim; otherwise (incident, rung) is the identity.
        let by_claim = c.claim_seq.and_then(|cs| {
            inc.consultations
                .iter()
                .position(|x| x.claim_seq == Some(cs))
        });
        let found = by_claim.or_else(|| inc.consultations.iter().position(|x| x.rung == rung));
        let pos = found.unwrap_or_else(|| {
            inc.consultations.push(Consultation::new(rung));
            inc.consultations.len() - 1
        });
        let slot = &mut inc.consultations[pos];
        let rung = slot.rung;
        match c.phase {
            Phase::Start => {
                if slot.started_at.is_some() {
                    slot.duplicate_starts += 1; // Folded as one consultation.
                    return;
                }
                let started = c.started_at.clone().unwrap_or_else(|| e.moment());
                slot.deadline_at = c.deadline_at.or_else(|| {
                    c.timeout_seconds
                        .map(|t| started.wall + Duration::milliseconds((t * 1000.0) as i64))
                });
                slot.started_at = Some(started);
                slot.claim_seq = Some(e.seq);
                slot.claimed_by = c.claimed_by.clone();
                slot.level = c.level.or(slot.level);
                slot.binding = c.binding.clone();
                inc.rung = inc.rung.max(rung);
            }
            Phase::Complete => {
                if slot.finished_at.is_some() {
                    return; // Idempotent: the first recorded result stands.
                }
                slot.finished_at = Some(c.finished_at.clone().unwrap_or_else(|| e.moment()));
                slot.decision = c.decision.clone();
                slot.pivot = c.pivot;
                slot.reason = c.reason.clone();
                slot.reported_elapsed_s = c.usage_elapsed_s;
                slot.tokens = c.tokens;
                slot.level = c.level.or(slot.level);
                slot.binding = c.binding.clone().or_else(|| slot.binding.clone());
                inc.rung = inc.rung.max(rung);
                // Shadow: the outcome is recorded and nothing else. A park or proceed never closes
                // the stall (the human is still waiting), and a reshape is only a prospective
                // offer. A Refer moves the folded ladder to the next rung.
                if matches!(c.decision, Some(Decision::Refer(_))) {
                    inc.rung = inc.rung.max((rung + 1).min(5));
                }
            }
            Phase::Expire => {
                if slot.finished_at.is_some() || slot.expired {
                    return;
                }
                slot.expired = true;
                slot.finished_at = Some(c.finished_at.clone().unwrap_or_else(|| e.moment()));
                slot.reported_elapsed_s = c.usage_elapsed_s;
                slot.tokens = c.tokens;
                inc.rung = inc.rung.max((rung + 1).min(5));
            }
        }
    }

    fn event(&mut self, e: &Event) {
        self.witness(e);
        match e.kind {
            EventKind::PermissionRequested => {
                for i in self.open_where(StallClass::Silence, &e.agent_id) {
                    self.close(i, e.moment(), Some(ClosedBy::Answered));
                }
                let id = e
                    .incident_ref()
                    .map_or_else(|| format!("{}:{}", self.session, e.seq), str::to_owned);
                self.open(e, id, StallClass::Authorization);
            }
            EventKind::DialogDisplayed => {
                let high = e
                    .correlation
                    .as_ref()
                    .is_some_and(|c| c.confidence == Confidence::High);
                match (e.incident_ref().map(str::to_owned), high) {
                    (Some(id), true) => {
                        if let Some(i) = self.lookup(&id, e) {
                            let inc = &mut self.incidents[i];
                            if inc.closed_at.is_none() && !inc.dialog_seen {
                                inc.dialog_seen = true;
                                inc.interval_confidence = Confidence::High;
                                let start = e.wall.max(inc.opened_at.wall);
                                if let Some(iv) = inc.blocked_intervals.first_mut() {
                                    iv.from = start;
                                }
                            }
                        }
                    }
                    // Never guess a dialog onto an incident; the start stays low confidence.
                    _ => self.gap("uncorrelated-dialog", e, None),
                }
            }
            EventKind::PromptClosed => {
                let Some(id) = e.incident_ref().map(str::to_owned) else {
                    self.gap("close-without-incident", e, None);
                    return;
                };
                let Some(i) = self.lookup(&id, e) else { return };
                if self.incidents[i].closed_at.is_some() {
                    return;
                }
                match e.closed_by {
                    Some(CloseAttribution::Approved) => match e.duration_ms {
                        // Approval time is the event time minus the tool's own duration.
                        Some(ms) => {
                            self.close(i, e.moment().minus(ms / 1000.0), Some(ClosedBy::Approved))
                        }
                        None => {
                            self.incidents[i].end_unmeasured = true;
                            self.close(i, e.moment(), Some(ClosedBy::Approved));
                        }
                    },
                    Some(CloseAttribution::DeniedInferred) => {
                        let by = if self.incidents[i].reshape_offered {
                            ClosedBy::Reshaped
                        } else {
                            ClosedBy::DeniedInferred
                        };
                        self.close(i, e.moment(), Some(by));
                    }
                    Some(CloseAttribution::Withdrawn) => {
                        self.close(i, e.moment(), Some(ClosedBy::Withdrawn))
                    }
                    Some(CloseAttribution::Expired) => {
                        self.close(i, e.moment(), Some(ClosedBy::Expired))
                    }
                    _ => {
                        self.close(i, e.moment(), None);
                        self.gap("unknown-close", e, Some(&id));
                    }
                }
            }
            EventKind::ClassifierDenied => self.classifier_denied += 1,
            // Silence evidence only: it opens no incident and witnesses no presence.
            EventKind::IdlePrompt => {}
            EventKind::TurnEnded => {
                for i in self.open_where(StallClass::Question, &e.agent_id) {
                    self.close(i, e.moment(), Some(ClosedBy::Withdrawn));
                }
                for i in self.open_where(StallClass::Silence, &e.agent_id) {
                    self.close(i, e.moment(), Some(ClosedBy::Answered));
                }
                if e.question == Some(Question::Yes) {
                    self.open(
                        e,
                        format!("{}:{}", self.session, e.seq),
                        StallClass::Question,
                    );
                }
            }
            EventKind::StewardPing => {
                if self.open_where(StallClass::Silence, &e.agent_id).is_empty() {
                    self.open(
                        e,
                        format!("{}:{}", self.session, e.seq),
                        StallClass::Silence,
                    );
                }
            }
            EventKind::Consultation => {
                if let Some(c) = &e.consultation {
                    self.consult(e, c);
                }
            }
            EventKind::ObservationGap => {
                if let Some(g) = &e.gap {
                    self.gaps.push(FoldGap {
                        reason: g.reason.clone(),
                        session: self.session.clone(),
                        seq: Some(e.seq),
                        at: Some(g.from.unwrap_or(e.wall)),
                        incident: None,
                    });
                }
            }
            EventKind::ShiftMarker => {
                if let Some(s) = &e.shift {
                    self.markers.push((s.id.clone(), s.open, e.wall));
                }
            }
            EventKind::HarnessException => {
                if let Some(x) = &e.exception {
                    self.exceptions
                        .push((e.wall, x.hook_name.clone(), x.count, x.baseline));
                }
            }
            EventKind::SessionEnded => {
                for i in 0..self.incidents.len() {
                    self.close(i, e.moment(), Some(ClosedBy::SessionEnded));
                }
            }
            EventKind::WouldHave => {
                if let Some(w) = &e.would_have {
                    if !self.would_keys.contains(&w.key) {
                        self.would_keys.push(w.key.clone());
                    }
                }
            }
        }
    }
}

struct Window {
    id: String,
    kind: &'static str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    open: bool,
}

fn union_ms(mut spans: Vec<(i64, i64)>) -> f64 {
    spans.sort_unstable();
    let mut total = 0_i64;
    let mut cursor: Option<(i64, i64)> = None;
    for (a, b) in spans {
        match cursor {
            Some((s, e)) if a <= e => cursor = Some((s, e.max(b))),
            Some((s, e)) => {
                total += e - s;
                cursor = Some((a, b));
            }
            None => cursor = Some((a, b)),
        }
    }
    if let Some((s, e)) = cursor {
        total += e - s;
    }
    total as f64 / 1000.0
}

fn stall_name(s: StallClass) -> &'static str {
    match s {
        StallClass::Authorization => "authorization",
        StallClass::Question => "question",
        StallClass::Silence => "silence",
    }
}

fn disposition_name(d: Disposition) -> &'static str {
    match d {
        Disposition::Collaborative => "collaborative",
        Disposition::Automated => "automated",
        Disposition::Unknown => "unknown",
    }
}

/// The session's disposition now. Presence changes only on a witness or an expiring ruling:
/// an away ruling in force overrides; a displayed dialog that reached Breach is a witnessed
/// absence that holds until a witness after it (closing the dialog does not clear it); otherwise
/// the passive T_c bound applies. An expired ruling removes its override and nothing more.
pub fn effective_presence(
    events: &[Event],
    incidents: &[Incident],
    now: DateTime<Utc>,
    p: &StasisPolicy,
    rulings: &[AwayRuling],
) -> Presence {
    let last_witness = events.iter().filter_map(Event::witness_at).max();
    let age = last_witness.map(|w| secs(w, now).max(0.0));
    let ruling = events
        .last()
        .and_then(|e| e.presence.ruling_ref.clone())
        .filter(|id| {
            rulings
                .iter()
                .find(|r| &r.id == id)
                .is_none_or(|r| r.expires_at.is_none_or(|t| t > now))
        });
    let limit = Duration::milliseconds((p.stall.limit * 1000.0) as i64);
    let absent = incidents.iter().any(|i| {
        let breach_at = i.opened_at.wall + limit;
        i.dialog_seen
            && now >= breach_at
            && i.closed_at.is_none_or(|c| c >= breach_at)
            && last_witness.is_none_or(|w| w < breach_at)
    });
    let disposition = if ruling.is_some() || absent {
        Disposition::Automated
    } else {
        match age {
            None => Disposition::Unknown,
            Some(a) if a <= p.collaborative_window_s => Disposition::Collaborative,
            Some(_) => Disposition::Automated,
        }
    };
    Presence {
        disposition,
        last_witnessed_age_seconds: age,
        ruling_ref: ruling,
    }
}

/// Apply deadlines that passed on the fold's clock: the same fact as an expiry record.
fn settle_consultations(inc: &mut Incident, now: &Moment) {
    for c in &mut inc.consultations {
        if c.in_flight() {
            if let Some(d) = c.deadline_at.filter(|d| now.wall >= *d) {
                c.expired = true;
                c.finished_at = Some(Moment::wall_only(d));
                if inc.closed_at.is_none() {
                    inc.rung = inc.rung.max((c.rung + 1).min(5));
                }
            }
        }
    }
}

fn finalize_incident(inc: &mut Incident, now: &Moment, p: &StasisPolicy, shift_exhausted: bool) {
    // A parked incident's unresolved work continues under the same budgets: it keeps running.
    let end = match (inc.closed_by, &inc.closed_moment) {
        (Some(ClosedBy::Parked), _) | (_, None) => now.clone(),
        (_, Some(m)) => m.clone(),
    };
    let mut used = BudgetUsed {
        keeper_calls: inc.consultations.len() as u32,
        elapsed_s: elapsed(&inc.opened_at, &end),
        ..Default::default()
    };
    for c in &inc.consultations {
        match c.seconds(now) {
            Some(s) => used.keeper_seconds_known += s,
            None => used.keeper_seconds_unmeasured += 1,
        }
        match c.tokens {
            Some(t) => used.tokens_known += t,
            None if c.finished_at.is_some() => used.tokens_unreported += 1,
            None => {}
        }
    }
    used.keeper_seconds =
        (used.keeper_seconds_unmeasured == 0).then_some(used.keeper_seconds_known);
    used.tokens = (used.tokens_unreported == 0).then_some(used.tokens_known);
    inc.budget_exhausted.clear();
    if used.keeper_calls >= p.budgets.incident_keeper_calls {
        inc.budget_exhausted.push("keeper-calls".into());
    }
    if used.elapsed_s >= p.budgets.incident_elapsed_s {
        inc.budget_exhausted.push("elapsed".into());
    }
    if used.tokens_known >= p.budgets.incident_tokens {
        inc.budget_exhausted.push("tokens".into());
    }
    inc.budget_used = used;
    // Exhaustion jumps straight to the steward; an in-flight consultation runs to its own
    // deadline but no further rung is taken.
    let open = inc.closed_at.is_none() || inc.closed_by == Some(ClosedBy::Parked);
    if open && (!inc.budget_exhausted.is_empty() || shift_exhausted) {
        inc.rung = 5;
    }
    inc.state = if inc.closed_at.is_some() && inc.closed_by != Some(ClosedBy::Parked) {
        IncidentState::Closed
    } else if inc.rung >= 5 {
        IncidentState::Pending
    } else {
        let age = elapsed(&inc.opened_at, now);
        if p.stall.breached_by(age) {
            IncidentState::Breach
        } else if p.stall.approached_by(age) {
            IncidentState::Approach
        } else {
            IncidentState::Open
        }
    };
}

fn overlaps(inc: &Incident, w: &Window, now: DateTime<Utc>) -> bool {
    inc.opened_at.wall <= w.to && inc.closed_at.unwrap_or(now) >= w.from
}

fn keeper_use(incidents: &[Incident], w: &Window, now: &Moment, p: &StasisPolicy) -> KeeperUse {
    let mut k = KeeperUse::default();
    let in_w = |t: DateTime<Utc>| t >= w.from && t <= w.to;
    for inc in incidents {
        for c in &inc.consultations {
            let anchor = c
                .finished_at
                .as_ref()
                .or(c.started_at.as_ref())
                .map_or(inc.opened_at.wall, |m| m.wall);
            match (c.seconds(now), c.started_at.as_ref()) {
                (Some(total), Some(start)) => {
                    let end = c.finished_at.as_ref().map_or(now.wall, |m| m.wall);
                    let span = secs(start.wall, end);
                    let overlap = secs(start.wall.max(w.from), end.min(w.to)).max(0.0);
                    if span > 0.0 {
                        k.seconds_known += total * overlap / span;
                    } else if in_w(anchor) {
                        k.seconds_known += total;
                    }
                }
                (Some(total), None) if in_w(anchor) => k.seconds_known += total,
                (None, _) if in_w(anchor) => k.seconds_unmeasured += 1,
                _ => {}
            }
            if in_w(anchor) {
                match c.tokens {
                    Some(t) => k.tokens_known += t,
                    None if c.finished_at.is_some() => k.tokens_unreported += 1,
                    None => {}
                }
            }
        }
    }
    k.seconds = (k.seconds_unmeasured == 0).then_some(k.seconds_known);
    k.tokens = (k.tokens_unreported == 0).then_some(k.tokens_known);
    if k.seconds_known >= p.budgets.shift_keeper_s {
        k.exhausted.push("keeper-seconds".into());
    }
    if k.tokens_known >= p.budgets.shift_tokens {
        k.exhausted.push("tokens".into());
    }
    k
}

fn windows_for(
    o: &FoldOptions<'_>,
    session: &str,
    markers: &[(String, bool, DateTime<Utc>)],
    events: &[Event],
    ended: Option<DateTime<Utc>>,
    live: bool,
) -> (Vec<Window>, Vec<usize>) {
    let now = o.now;
    let mut windows = Vec::new();
    // Marker windows: a shift's accounting runs from its open edge to its close edge.
    let mut seen: Vec<&str> = Vec::new();
    for (id, open, at) in markers {
        if !*open || seen.contains(&id.as_str()) {
            continue;
        }
        seen.push(id);
        let close = markers
            .iter()
            .find(|(i, o, t)| i == id && !*o && *t >= *at)
            .map(|m| m.2);
        windows.push(Window {
            id: id.clone(),
            kind: "marker",
            from: *at,
            to: close.unwrap_or(now),
            open: close.is_none(),
        });
    }
    let marker_count = windows.len();
    if let (Some(first), Some(last)) = (events.first(), events.last()) {
        let first = events.iter().map(|e| e.wall).min().unwrap_or(first.wall);
        let end = ended.unwrap_or(now).max(last.wall);
        let mut n = 0;
        loop {
            let from = first + Duration::hours(24 * n);
            if from > end && n > 0 {
                break;
            }
            let nominal = from + Duration::hours(24);
            let last_tile = nominal >= end;
            windows.push(Window {
                id: format!("{session}#{}", n + 1),
                kind: "session",
                from,
                to: if last_tile { end } else { nominal },
                open: last_tile && ended.is_none(),
            });
            if last_tile {
                break;
            }
            n += 1;
        }
    }
    let mut chosen = Vec::new();
    match o.selection {
        Selection::All => {
            if let Some(first) = events.iter().map(|e| e.wall).min() {
                let last = events.iter().map(|e| e.wall).max().unwrap_or(first);
                windows.push(Window {
                    id: "all".into(),
                    kind: "all",
                    from: first,
                    to: ended.unwrap_or(now).max(last),
                    open: ended.is_none(),
                });
                chosen.push(windows.len() - 1);
            }
        }
        Selection::Since(t) => {
            windows.push(Window {
                id: format!("since:{}", t.to_rfc3339_opts(SecondsFormat::Secs, true)),
                kind: "since",
                from: t,
                to: now.max(t),
                open: true,
            });
            chosen.push(windows.len() - 1);
        }
        Selection::ShiftCurrent => {
            // An unnamed sweep reads only live sessions; a named session reads its latest window.
            if o.explicit_session || live {
                let open_marker = (0..marker_count)
                    .filter(|&i| windows[i].open)
                    .max_by_key(|&i| windows[i].from);
                let latest_session = (marker_count..windows.len()).last();
                if let Some(i) = open_marker.or(latest_session) {
                    chosen.push(i);
                }
            }
        }
    }
    (windows, chosen)
}

/// Fold one replayed session.
pub fn fold_session(replay: &Replay, o: &FoldOptions<'_>) -> SessionFold {
    let now_m = o.now_moment();
    let mut f = Folder {
        o: *o,
        session: replay.session.clone(),
        first_seq: replay.events.iter().map(|e| e.seq).min().unwrap_or(1),
        incidents: vec![],
        by_id: HashMap::new(),
        gaps: replay
            .gaps
            .iter()
            .map(|g| FoldGap {
                reason: g.reason.clone(),
                session: replay.session.clone(),
                seq: g.from_seq,
                at: None,
                incident: None,
            })
            .collect(),
        markers: vec![],
        exceptions: vec![],
        would_keys: vec![],
        classifier_denied: 0,
    };
    for e in &replay.events {
        f.event(e);
    }
    let ended = replay
        .events
        .iter()
        .rev()
        .find(|e| e.kind == EventKind::SessionEnded)
        .map(|e| e.wall);
    let last_event = replay.events.iter().map(|e| e.wall).max();
    let live = ended.is_none() && last_event.is_some_and(|t| t + Duration::hours(24) >= o.now);
    let (windows, chosen) =
        windows_for(o, &replay.session, &f.markers, &replay.events, ended, live);
    for inc in &mut f.incidents {
        settle_consultations(inc, &now_m);
    }

    // Presence precedence: an unanswered displayed dialog that reached Breach is a witnessed
    // absence for the session until the next witnessed interaction.
    let mut absent_since: Option<DateTime<Utc>> = None;
    let limit = Duration::milliseconds((o.policy.stall.limit * 1000.0) as i64);
    for inc in &mut f.incidents {
        if let Some(b) = absent_since {
            if inc.witness_at_open.is_some_and(|w| w > b) {
                absent_since = None;
            }
        }
        if absent_since.is_some_and(|b| b <= inc.opened_at.wall) {
            inc.presence_at_open.disposition = Disposition::Automated;
        }
        let breach_at = inc.opened_at.wall + limit;
        if inc.dialog_seen && o.now >= breach_at && inc.closed_at.is_none_or(|c| c >= breach_at) {
            absent_since = Some(breach_at);
        }
    }

    // Per-shift keeper cost, attributed by time overlap, over the partition in effect.
    let marker_windows = windows.iter().any(|w| w.kind == "marker");
    let in_partition = |w: &Window| w.kind == "marker" || (w.kind == "session" && !marker_windows);
    let keeper: Vec<KeeperUse> = windows
        .iter()
        .map(|w| keeper_use(&f.incidents, w, &now_m, o.policy))
        .collect();
    for inc in &mut f.incidents {
        let spent = windows
            .iter()
            .zip(&keeper)
            .any(|(w, k)| in_partition(w) && !k.exhausted.is_empty() && overlaps(inc, w, o.now));
        finalize_incident(inc, &now_m, o.policy, spent);
    }

    let has_harness = !f.exceptions.is_empty();
    let untimed = f.gaps.iter().filter(|g| g.at.is_none()).count();
    let readings: Vec<WindowReading> = windows
        .iter()
        .zip(keeper)
        .map(|(w, keeper)| {
            let in_window = |t: DateTime<Utc>| t >= w.from && t <= w.to;
            let events = replay.events.iter().filter(|e| in_window(e.wall)).count();
            let evidence = replay
                .events
                .iter()
                .filter(|e| in_window(e.wall) && e.kind != EventKind::ObservationGap)
                .count();
            let mut all = vec![];
            let mut by_class: BTreeMap<&str, Vec<(i64, i64)>> = BTreeMap::new();
            let mut by_presence: BTreeMap<&str, Vec<(i64, i64)>> = BTreeMap::new();
            let (mut low, mut open_iv, mut unknown, mut unmeasured_ends) = (0, 0, 0, 0);
            for inc in &f.incidents {
                for iv in &inc.blocked_intervals {
                    let end = iv.to.unwrap_or(o.now);
                    let (a, b) = (iv.from.max(w.from), end.min(w.to));
                    if b <= a {
                        continue;
                    }
                    let span = (a.timestamp_millis(), b.timestamp_millis());
                    all.push(span);
                    by_class
                        .entry(stall_name(inc.stall_class))
                        .or_default()
                        .push(span);
                    by_presence
                        .entry(disposition_name(inc.presence_at_open.disposition))
                        .or_default()
                        .push(span);
                    low += usize::from(
                        inc.interval_confidence == Confidence::Low || inc.end_unmeasured,
                    );
                    open_iv += usize::from(iv.to.is_none());
                    unknown += usize::from(inc.close_unknown);
                    unmeasured_ends += usize::from(inc.end_unmeasured);
                }
            }
            // Gaps alone are not evidence of a measured zero.
            let measured = evidence > 0 || !all.is_empty();
            let total = union_ms(all);
            let gaps = f
                .gaps
                .iter()
                .filter(|g| g.at.is_some_and(in_window))
                .count()
                + untimed;
            let harness = (has_harness && measured).then(|| {
                let mut h = HarnessReading {
                    count: 0,
                    by_hook: BTreeMap::new(),
                    baseline_excluded: 0,
                };
                for (t, hook, n, baseline) in &f.exceptions {
                    if !in_window(*t) {
                        continue;
                    }
                    if *baseline {
                        h.baseline_excluded += n;
                    } else {
                        h.count += n;
                        *h.by_hook.entry(hook.clone()).or_default() += n;
                    }
                }
                h
            });
            let class_value = |name: &str| by_class.get(name).map_or(0.0, |v| union_ms(v.clone()));
            let presence_value =
                |name: &str| by_presence.get(name).map_or(0.0, |v| union_ms(v.clone()));
            WindowReading {
                id: w.id.clone(),
                kind: w.kind.into(),
                session: replay.session.clone(),
                from: w.from,
                to: w.to,
                open: w.open,
                events,
                value_seconds: measured.then_some(total),
                by_class: ["authorization", "question", "silence"]
                    .iter()
                    .map(|n| ((*n).to_owned(), class_value(n)))
                    .collect(),
                by_presence: ["collaborative", "automated", "unknown"]
                    .iter()
                    .map(|n| ((*n).to_owned(), presence_value(n)))
                    .collect(),
                low_confidence_intervals: low,
                open_intervals: open_iv,
                unknown_closes: unknown,
                unmeasured_ends,
                gaps,
                complete: gaps == 0 && unknown == 0 && unmeasured_ends == 0,
                ceiling: measured.then(|| Ceiling {
                    limit: o.policy.shift_ceiling.limit,
                    breached: o.policy.shift_ceiling.breached_by(total),
                }),
                harness_exceptions: harness,
                keeper,
            }
        })
        .collect();

    let selected_windows: Vec<&Window> = chosen.iter().map(|&i| &windows[i]).collect();
    let selected_incidents: Vec<usize> = (0..f.incidents.len())
        .filter(|&i| {
            selected_windows
                .iter()
                .any(|w| overlaps(&f.incidents[i], w, o.now))
        })
        .collect();
    let ladder_resolution = selected_incidents
        .iter()
        .map(|&i| &f.incidents[i])
        .filter_map(|i| {
            let by = i.closed_by?;
            let resolver = i
                .consultations
                .iter()
                .filter(|c| matches!(c.decision, Some(Decision::Permit)))
                .max_by_key(|c| c.rung);
            let (resolved_by, binding) = match by {
                ClosedBy::Reshaped | ClosedBy::Proceeded | ClosedBy::Parked => {
                    let c = resolver?;
                    (c.level?.rung_name(), c.binding.clone())
                }
                ClosedBy::Approved | ClosedBy::DeniedInferred | ClosedBy::Answered => {
                    ("human".to_owned(), None)
                }
                // Unresolved stalls are not successful resolutions.
                ClosedBy::Withdrawn | ClosedBy::Expired | ClosedBy::SessionEnded => return None,
            };
            Some(Resolution {
                incident_id: i.id.clone(),
                resolved_by,
                binding,
                concern_class: i.concern_class,
                presence: i.presence_at_open.disposition,
            })
        })
        .collect();

    let presence = effective_presence(&replay.events, &f.incidents, o.now, o.policy, o.rulings);
    SessionFold {
        session: replay.session.clone(),
        incidents: f.incidents,
        selected_incidents,
        gaps: f.gaps,
        windows: readings,
        selected: chosen,
        ladder_resolution,
        would_have: f.would_keys.len(),
        would_have_keys: f.would_keys,
        classifier_denied: f.classifier_denied,
        presence,
        last_event,
        ended: ended.is_some(),
    }
}

impl Level {
    fn rung_name(self) -> String {
        format!("C{}", self.rung())
    }
}

/// The aggregate `fold` prints.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldReport {
    /// Policy state: [`POLICY_ABSENT`] until slice 6.
    pub policy: &'static str,
    /// Always `shadow` in this slice: nothing the fold reports was executed.
    pub mode: &'static str,
    /// Incidents overlapping the selected windows.
    pub incidents: Vec<Incident>,
    /// The measures.
    pub measures: Value,
    /// Evidence gaps.
    pub gaps: Vec<FoldGap>,
    /// The shifts read, `id` per window: `shift=<id>` in every reading's env.
    pub shifts: Vec<String>,
    /// Folded sessions.
    pub sessions: Vec<String>,
}

/// Fold every named session under `root` (or all of them) and assemble the report.
pub fn fold_root(
    root: &std::path::Path,
    session: Option<&str>,
    o: &FoldOptions<'_>,
) -> Result<(FoldReport, Vec<SessionFold>), StasisError> {
    let names = match session {
        Some(s) => vec![s.to_owned()],
        None => journal::sessions(root)?,
    };
    if names.is_empty() {
        return Err(StasisError::NoJournal(format!(
            "no journal under {}",
            root.display()
        )));
    }
    let mut folds = Vec::new();
    for name in &names {
        let replay = journal::replay(root, name)?;
        folds.push(fold_session(
            &replay,
            &FoldOptions {
                explicit_session: session.is_some(),
                ..*o
            },
        ));
    }
    Ok((report(&folds, o.policy), folds))
}

/// Assemble the report from folded sessions.
pub fn report(folds: &[SessionFold], policy: &StasisPolicy) -> FoldReport {
    let picked: Vec<&WindowReading> = folds
        .iter()
        .flat_map(|f| f.selected.iter().map(|&i| &f.windows[i]))
        .collect();
    let measured: Vec<&&WindowReading> = picked
        .iter()
        .filter(|w| w.value_seconds.is_some())
        .collect();
    let value: Option<f64> =
        (!measured.is_empty()).then(|| measured.iter().filter_map(|w| w.value_seconds).sum());
    let sum_map = |pick: fn(&WindowReading) -> &BTreeMap<String, f64>| -> BTreeMap<String, f64> {
        let mut out: BTreeMap<String, f64> = BTreeMap::new();
        for w in &measured {
            for (k, v) in pick(w) {
                *out.entry(k.clone()).or_default() += v;
            }
        }
        out
    };
    let ids: Vec<String> = picked.iter().map(|w| w.id.clone()).collect();
    let harness: Vec<&HarnessReading> = picked
        .iter()
        .filter_map(|w| w.harness_exceptions.as_ref())
        .collect();
    let harness_value = (!harness.is_empty()).then(|| {
        let mut by_hook: BTreeMap<String, u64> = BTreeMap::new();
        for h in &harness {
            for (k, v) in &h.by_hook {
                *by_hook.entry(k.clone()).or_default() += v;
            }
        }
        json!({"value": harness.iter().map(|h| h.count).sum::<u64>(), "byHook": by_hook,
               "baselineExcluded": harness.iter().map(|h| h.baseline_excluded).sum::<u64>()})
    });
    let resolutions: Vec<&Resolution> = folds
        .iter()
        .flat_map(|f| f.ladder_resolution.iter())
        .collect();
    let measures = json!({
        "dialogue-blocked-seconds": {
            "version": 1,
            "unit": "seconds",
            "value": value,
            "byClass": sum_map(|w| &w.by_class),
            "byPresence": sum_map(|w| &w.by_presence),
            "shift": ids.join(","),
            "ceiling": value.map(|v| json!({"limit": policy.shift_ceiling.limit, "breached": policy.shift_ceiling.breached_by(v)})),
            "complete": !picked.is_empty() && picked.iter().all(|w| w.complete),
            "lowConfidenceIntervals": picked.iter().map(|w| w.low_confidence_intervals).sum::<usize>(),
            "readings": picked,
        },
        "ladder-resolution": resolutions,
        "harness-exceptions": harness_value,
    });
    FoldReport {
        policy: POLICY_ABSENT,
        mode: "shadow",
        incidents: folds
            .iter()
            .flat_map(|f| f.selected_incidents.iter().map(|&i| f.incidents[i].clone()))
            .collect(),
        measures,
        gaps: folds.iter().flat_map(|f| f.gaps.iter().cloned()).collect(),
        shifts: ids,
        sessions: folds.iter().map(|f| f.session.clone()).collect(),
    }
}

/// The count a probe declared `emits: count` prints for a measure, with `shift=<id>` in its env.
///
/// The design text says `--emit-count` prints "a single number", but `epr flow sense` parses a
/// probe's stdout as one JSON **object** (`{"value": N, "env": {...}}`); a bare number is
/// refused as "something other than its declared JSON". This returns that object so the wired
/// probe works and every reading carries its shift id. `None` is unmeasured.
pub fn emit_count(report: &FoldReport, measure: &str) -> Result<Option<Value>, StasisError> {
    let (id, version) = measure.split_once('@').unwrap_or((measure, ""));
    if version != "1" {
        return Err(StasisError::Parse(format!(
            "unsupported measure version in {measure}"
        )));
    }
    let value = match id {
        "dialogue-blocked-seconds" => report.measures[id]["value"].as_f64(),
        "harness-exceptions" => report.measures[id]["value"].as_f64(),
        other => {
            return Err(StasisError::Parse(format!(
                "{other} is not a count measure of this fold"
            )))
        }
    };
    Ok(value.map(|v| {
        json!({"value": v.round(), "env": {"shift": report.shifts.join(","), "session": report.sessions.join(",")}})
    }))
}

// ---------------------------------------------------------------------------------------------
// Shadow decide and status.
// ---------------------------------------------------------------------------------------------

/// The effect a live regulator could have. There is no allow-shaped variant: the regulator can
/// only deny, block, add context or do nothing (`the_regulator_has_no_allow_shaped_effect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// Nothing.
    None,
    /// Deny a PermissionRequest.
    Deny,
    /// Block a Stop.
    Block,
    /// Deliver guidance as context.
    Context,
}

impl Effect {
    fn word(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Deny => "deny",
            Self::Block => "block",
            Self::Context => "context",
        }
    }
}

/// What `decide` prints.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideOutput {
    /// The effect (always `none` in shadow).
    pub effect: Effect,
    /// A message to deliver, when any.
    pub message: Option<String>,
    /// The incident concerned.
    pub incident_id: Option<String>,
    /// Its rung.
    pub rung: u8,
    /// Always true in this slice.
    pub shadow: bool,
    /// Policy state.
    pub policy: &'static str,
    /// True when no journal existed to fold or record into.
    pub unmeasured: bool,
}

/// One adapter-normalized hook event handed to `decide`.
#[derive(Debug, Clone)]
pub struct DecideRequest {
    /// The hook (`PermissionRequest`, `Stop`, ...).
    pub hook: String,
    /// The tool, when any.
    pub tool: Option<String>,
    /// The redacted shape, when any.
    pub shape: Option<String>,
    /// The agent.
    pub agent_id: String,
    /// The incident the adapter correlated, when any.
    pub incident_id: Option<String>,
}

impl DecideRequest {
    /// Read the normalized event: journal-event or hook-payload spellings are both accepted.
    pub fn from_json(v: &Value) -> Result<Self, StasisError> {
        let pick = |keys: &[&str]| {
            keys.iter()
                .find_map(|k| v.get(*k).and_then(Value::as_str))
                .map(str::to_owned)
        };
        let from_kind = v.get("kind").and_then(Value::as_str).map(|k| match k {
            "permission-requested" => "PermissionRequest",
            "turn-ended" => "Stop",
            "session-ended" => "SessionEnd",
            _ => "unknown",
        });
        let hook = pick(&["hookEvent", "hook_event_name"])
            .or_else(|| {
                v.pointer("/source/hookEvent")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .or_else(|| from_kind.map(str::to_owned))
            .ok_or_else(|| StasisError::Parse("event names no hook".into()))?;
        Ok(Self {
            hook,
            tool: pick(&["tool", "tool_name"]),
            shape: pick(&["shape"]),
            agent_id: pick(&["agentId", "agent_id"]).unwrap_or_else(|| "main".into()),
            incident_id: v
                .pointer("/correlation/incidentId")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }
}

/// A clock reading for an appended event.
#[derive(Debug, Clone)]
pub struct ClockReading {
    /// Wall time.
    pub wall: DateTime<Utc>,
    /// Boot id.
    pub boot_id: String,
    /// Uptime seconds.
    pub uptime_seconds: f64,
}

impl ClockReading {
    /// The system clock, with Linux boot id and uptime where readable.
    pub fn system() -> Self {
        let read = |p: &str| std::fs::read_to_string(p).ok();
        Self {
            wall: Utc::now(),
            boot_id: read("/proc/sys/kernel/random/boot_id")
                .map_or_else(|| "unknown-boot".into(), |s| s.trim().to_owned()),
            uptime_seconds: read("/proc/uptime")
                .and_then(|s| s.split_whitespace().next().and_then(|t| t.parse().ok()))
                .unwrap_or(0.0),
        }
    }
}

/// Run `decide` in shadow: fold the session's journal, journal the prospective effect as
/// `would-have` (once per key), execute nothing. A session with no journal reads `unmeasured`
/// with exit 0, so an adapter that calls it before the sensor has written anything still
/// fails open with `effect: none`.
pub fn decide_shadow(
    root: &std::path::Path,
    session: &str,
    req: &DecideRequest,
    classifier: &Classifier,
    policy: &StasisPolicy,
    clock: &ClockReading,
    segment_limit: u64,
) -> Result<DecideOutput, StasisError> {
    let mut out = DecideOutput {
        effect: Effect::None,
        message: None,
        incident_id: req.incident_id.clone(),
        rung: 0,
        shadow: true,
        policy: POLICY_ABSENT,
        unmeasured: true,
    };
    let dir = root.join(session);
    if !dir.is_dir() {
        return Ok(out);
    }
    let _lock = journal::Lock::exclusive(&dir)?;
    let replay = match journal::replay_unlocked(&dir, session) {
        Ok(r) => r,
        Err(StasisError::NoJournal(_)) => return Ok(out),
        Err(e) => return Err(e),
    };
    let Some(last) = replay.events.last() else {
        return Ok(out);
    };
    let fold = fold_session(
        &replay,
        &FoldOptions::new(clock.wall, Selection::All, true, policy, classifier)
            .with_clock(&clock.boot_id, clock.uptime_seconds),
    );
    let incident = req
        .incident_id
        .as_deref()
        .and_then(|id| fold.incidents.iter().find(|i| i.id == id))
        .or_else(|| {
            fold.incidents
                .iter()
                .rev()
                .find(|i| i.state != IncidentState::Closed)
        });
    let classification = (req.hook == "PermissionRequest").then(|| {
        let input = req
            .shape
            .as_deref()
            .map_or(CallInput::Opaque, CallInput::Shape);
        classifier.classify(
            StallClass::Authorization,
            req.tool.as_deref().unwrap_or("unknown"),
            input,
        )
    });
    out.incident_id = incident
        .map(|i| i.id.clone())
        .or_else(|| req.incident_id.clone());
    out.rung = incident.map_or(0, |i| i.rung);
    out.unmeasured = false;
    let key = format!(
        "{}:{}:{}",
        out.effect.word(),
        out.incident_id.as_deref().unwrap_or("-"),
        out.rung
    );
    if fold.would_have_keys.contains(&key) {
        return Ok(out); // The same prospective effect is already recorded.
    }
    let seq = replay
        .events
        .iter()
        .map(|e| e.seq)
        .max()
        .unwrap_or(last.seq)
        + 1;
    let presence = fold.presence.clone();
    let event = json!({
        "schemaVersion": 1,
        "kind": "would-have",
        "session": {
            "runtime": last.session.runtime,
            "session": session,
            "ownerPid": last.session.owner_pid,
            "ownerStartTime": last.session.owner_start_time.map(|t| t.to_string()),
        },
        "agentId": req.agent_id,
        "seq": seq,
        "clock": {
            "wall": clock.wall.to_rfc3339_opts(SecondsFormat::Millis, true),
            "bootId": clock.boot_id,
            "uptimeSeconds": clock.uptime_seconds,
            "provenance": "epr-session-stasis/linux-proc",
        },
        "presence": {
            "disposition": disposition_name(presence.disposition),
            "basis": {"lastWitnessedAgeSeconds": presence.last_witnessed_age_seconds, "rulingRef": presence.ruling_ref},
        },
        "source": {"hookEvent": req.hook},
        "wouldHave": {
            "effect": out.effect.word(),
            "incidentId": out.incident_id,
            "rung": out.rung,
            "key": key,
            "message": out.message,
            "reason": "no learned rung-0 entry in the built-in defaults; keepers are not consulted in shadow",
            "classification": classification,
        },
    });
    journal::append_locked(&dir, &event, seq, segment_limit)?;
    Ok(out)
}

/// Read-only status for one session.
pub fn status(fold: &SessionFold, now: DateTime<Utc>) -> Value {
    let open: Vec<Value> = fold
        .incidents
        .iter()
        .filter(|i| i.state != IncidentState::Closed)
        .map(|i| {
            json!({"id": i.id, "stallClass": i.stall_class, "concernClass": i.concern_class, "state": i.state,
                   "rung": i.rung, "ageSeconds": secs(i.opened_at.wall, now), "budgetUsed": i.budget_used,
                   "budgetExhausted": i.budget_exhausted})
        })
        .collect();
    // A park is a recorded Permit/park consultation; shadow never closes the stall on it.
    let parks: Vec<&str> = fold
        .incidents
        .iter()
        .filter(|i| {
            i.consultations.iter().any(|c| {
                c.pivot == Some(Pivot::Park) && matches!(c.decision, Some(Decision::Permit))
            })
        })
        .map(|i| i.id.as_str())
        .collect();
    let shift = fold.selected.first().map(|&i| &fold.windows[i]);
    json!({
        "session": fold.session,
        "openIncidents": open,
        "pendingParks": parks,
        "shift": shift.map(|w| json!({"id": w.id, "keeper": w.keeper, "blockedSeconds": w.value_seconds})),
        "presence": fold.presence,
        // The coordinator's heartbeat is a journal record the schema does not yet carry; an
        // absent heartbeat reads unknown, never green.
        "coordinator": {"state": "unknown", "heartbeatAgeSeconds": null},
        "policy": POLICY_ABSENT,
    })
}

#[cfg(test)]
mod tests;
