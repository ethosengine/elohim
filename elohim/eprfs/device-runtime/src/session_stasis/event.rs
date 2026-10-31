//! Journal events: the typed reading of one `session-event.schema.json` line.
//!
//! Parsing is deliberately lenient about fields the schema does not yet carry (the consultation
//! lifecycle, `withdrawn`/`expired` closes, `would-have`): slice 8 extends the schema, and the
//! fold must already replay those fixtures. It is strict about everything it depends on.

use chrono::{DateTime, Utc};
use elohim_epr::verdict::{Decision, ReferQuestion, ReferReason};
use serde_json::Value;

use crate::session::{SessionIdentity, Source};

/// Every journal event kind the fold models (the schema's thirteen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A call reached permission handling.
    PermissionRequested,
    /// A permission-prompt notification arrived.
    DialogDisplayed,
    /// A prompt closed (approved, inferred denial, unknown, or a later-schema close).
    PromptClosed,
    /// The harness classifier denied a call.
    ClassifierDenied,
    /// A turn ended, with a question classification.
    TurnEnded,
    /// A Notification `idle_prompt`: silence evidence only, neither a presence witness nor a stall.
    IdlePrompt,
    /// A consultation lifecycle event.
    Consultation,
    /// A shadow-mode prospective effect that was not executed.
    WouldHave,
    /// The steward asked for attention or progress.
    StewardPing,
    /// Observation was incomplete.
    ObservationGap,
    /// A skill shift opened or closed.
    ShiftMarker,
    /// A harness exception reached the session.
    HarnessException,
    /// The session ended.
    SessionEnded,
}

/// The schema's event kinds, in schema order.
pub const SCHEMA_KINDS: [&str; 13] = [
    "permission-requested",
    "dialog-displayed",
    "prompt-closed",
    "classifier-denied",
    "turn-ended",
    "idle-prompt",
    "consultation",
    "would-have",
    "steward-ping",
    "observation-gap",
    "shift-marker",
    "harness-exception",
    "session-ended",
];

impl EventKind {
    /// Parse a wire kind.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "permission-requested" => Self::PermissionRequested,
            "dialog-displayed" => Self::DialogDisplayed,
            "prompt-closed" => Self::PromptClosed,
            "classifier-denied" => Self::ClassifierDenied,
            "turn-ended" => Self::TurnEnded,
            "idle-prompt" => Self::IdlePrompt,
            "consultation" => Self::Consultation,
            "would-have" => Self::WouldHave,
            "steward-ping" => Self::StewardPing,
            "observation-gap" => Self::ObservationGap,
            "shift-marker" => Self::ShiftMarker,
            "harness-exception" => Self::HarnessException,
            "session-ended" => Self::SessionEnded,
            _ => return None,
        })
    }
}

/// Presence disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    /// The last witness is within the collaborative window.
    Collaborative,
    /// Absent: window passed, away ruling, or a witnessed absence.
    Automated,
    /// No evidence either way.
    Unknown,
}

/// Disposition plus its witnessed basis.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    /// The handling context.
    pub disposition: Disposition,
    /// Age of the last witnessed interaction, when known.
    pub last_witnessed_age_seconds: Option<f64>,
    /// The away ruling in force, when any.
    pub ruling_ref: Option<String>,
}

/// Correlation confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// A reliable match.
    High,
    /// An inferred match.
    Low,
    /// No match could be established.
    Unknown,
}

/// How an event relates to an incident.
#[derive(Debug, Clone, PartialEq)]
pub struct Correlation {
    /// The correlated incident, when one can be named.
    pub incident_id: Option<String>,
    /// Confidence in the match.
    pub confidence: Confidence,
}

/// Question classification of a turn end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// The turn asked the steward a question.
    Yes,
    /// It did not.
    No,
    /// Unknown; never blocks.
    Uncertain,
}

/// How a prompt closed, as written in the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAttribution {
    /// Approved (PostToolUse proves it).
    Approved,
    /// Denial inferred from the next event.
    DeniedInferred,
    /// Close could not be established.
    Unknown,
    /// The request was withdrawn (later schema).
    Withdrawn,
    /// The request expired (later schema).
    Expired,
}

/// A keeper's capability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Level {
    /// Light.
    C1,
    /// Standard.
    C2,
    /// Deep.
    C3,
    /// Frontier.
    C4,
}

impl Level {
    /// The ladder rung this level occupies (rungs 1-4).
    pub fn rung(self) -> u8 {
        match self {
            Self::C1 => 1,
            Self::C2 => 2,
            Self::C3 => 3,
            Self::C4 => 4,
        }
    }

    /// The level occupying a keeper rung.
    pub fn for_rung(rung: u8) -> Option<Self> {
        Some(match rung {
            1 => Self::C1,
            2 => Self::C2,
            3 => Self::C3,
            4 => Self::C4,
            _ => return None,
        })
    }
}

/// The operational pivot beneath a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pivot {
    /// Offer one more-legible command with the same effect.
    Reshape,
    /// Close the stall; park the unresolved step.
    Park,
    /// Resolve a question under a standing ruling.
    Proceed,
    /// Call attention; the incident stays open.
    Notify,
}

/// Which lifecycle moment a consultation event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// A claim started.
    Start,
    /// A result was recorded.
    Complete,
    /// The deadline passed without a result.
    Expire,
}

/// A wall plus monotonic clock reading, as the journal's `clock` objects carry.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moment {
    /// Wall-clock time.
    pub wall: DateTime<Utc>,
    /// Boot id of the monotonic clock; empty when none is known.
    pub boot_id: String,
    /// Seconds of uptime on that boot.
    pub uptime_seconds: f64,
}

impl Moment {
    /// This moment shifted back by `secs` on both clocks.
    pub fn minus(&self, secs: f64) -> Self {
        Self {
            wall: self.wall - chrono::Duration::milliseconds((secs * 1000.0) as i64),
            boot_id: self.boot_id.clone(),
            uptime_seconds: (self.uptime_seconds - secs).max(0.0),
        }
    }

    /// A wall-only moment (no monotonic evidence).
    pub fn wall_only(wall: DateTime<Utc>) -> Self {
        Self {
            wall,
            boot_id: String::new(),
            uptime_seconds: 0.0,
        }
    }
}

/// Seconds from `a` to `b`: monotonic when both share a boot, wall time otherwise.
pub fn elapsed(a: &Moment, b: &Moment) -> f64 {
    let secs = if !a.boot_id.is_empty() && a.boot_id == b.boot_id {
        b.uptime_seconds - a.uptime_seconds
    } else {
        (b.wall - a.wall).num_milliseconds() as f64 / 1000.0
    };
    secs.max(0.0)
}

/// A consultation lifecycle event (schema: `consultation`).
#[derive(Debug, Clone)]
pub struct ConsultationEvent {
    /// Start, completion or expiry; a record with no phase is the single-record completion.
    pub phase: Phase,
    /// The rung (1-5); derived from the level when absent.
    pub rung: Option<u8>,
    /// The capability level.
    pub level: Option<Level>,
    /// The runtime binding.
    pub binding: Option<String>,
    /// Permit or Refer (a keeper never returns Refuse).
    pub decision: Option<Decision>,
    /// The pivot; `None` when a Refer carries none.
    pub pivot: Option<Pivot>,
    /// The recorded reason.
    pub reason: Option<String>,
    /// The start record's seq this completion or expiry closes.
    pub claim_seq: Option<u64>,
    /// Claim start clock.
    pub started_at: Option<Moment>,
    /// Claim deadline.
    pub deadline_at: Option<DateTime<Utc>>,
    /// Declared timeout.
    pub timeout_seconds: Option<f64>,
    /// Completion or expiry clock.
    pub finished_at: Option<Moment>,
    /// Who holds the claim, as `pid@bootId`.
    pub claimed_by: Option<String>,
    /// Measured elapsed seconds; `None` is unmeasured.
    pub usage_elapsed_s: Option<f64>,
    /// Reported combined tokens; `None` is unmeasured.
    pub tokens: Option<u64>,
}

/// A recorded shadow effect (schema: `wouldHave`).
#[derive(Debug, Clone, PartialEq)]
pub struct WouldHave {
    /// The prospective effect.
    pub effect: String,
    /// The incident concerned.
    pub incident_id: Option<String>,
    /// Its rung.
    pub rung: Option<u8>,
    /// Deduplication key.
    pub key: String,
}

/// A shift marker edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftMarker {
    /// The shift id, equal at both edges.
    pub id: String,
    /// True for the opening edge.
    pub open: bool,
}

/// An observation gap note.
#[derive(Debug, Clone, PartialEq)]
pub struct GapNote {
    /// Why observation was incomplete.
    pub reason: String,
    /// Interval start, when known.
    pub from: Option<DateTime<Utc>>,
    /// Interval end, when known.
    pub to: Option<DateTime<Utc>>,
}

/// A coalesced harness exception.
#[derive(Debug, Clone, PartialEq)]
pub struct ExceptionNote {
    /// The hook that raised it.
    pub hook_name: String,
    /// How many occurrences were coalesced.
    pub count: u64,
    /// True for historical exceptions read while establishing a cursor.
    pub baseline: bool,
}

/// One journal event.
#[derive(Debug, Clone)]
pub struct Event {
    /// The kind.
    pub kind: EventKind,
    /// The session incarnation (declared by the hook payload).
    pub session: SessionIdentity,
    /// The agent that produced it.
    pub agent_id: String,
    /// The atomic sequence number.
    pub seq: u64,
    /// Wall-clock time.
    pub wall: DateTime<Utc>,
    /// Boot id of the monotonic clock.
    pub boot_id: String,
    /// Uptime seconds of the monotonic clock.
    pub uptime_seconds: f64,
    /// Presence at this event.
    pub presence: Presence,
    /// Shift marker payload.
    pub shift: Option<ShiftMarker>,
    /// Tool name.
    pub tool: Option<String>,
    /// Redacted shape.
    pub shape: Option<String>,
    /// Source duration in milliseconds.
    pub duration_ms: Option<f64>,
    /// Correlation.
    pub correlation: Option<Correlation>,
    /// Gap payload.
    pub gap: Option<GapNote>,
    /// Question classification.
    pub question: Option<Question>,
    /// Consultation payload.
    pub consultation: Option<ConsultationEvent>,
    /// Close attribution.
    pub closed_by: Option<CloseAttribution>,
    /// Exception payload.
    pub exception: Option<ExceptionNote>,
    /// Shadow-effect payload.
    pub would_have: Option<WouldHave>,
}

impl Event {
    /// This event's wall and monotonic clock.
    pub fn moment(&self) -> Moment {
        Moment {
            wall: self.wall,
            boot_id: self.boot_id.clone(),
            uptime_seconds: self.uptime_seconds,
        }
    }

    /// The incident this event names, when any.
    pub fn incident_ref(&self) -> Option<&str> {
        self.correlation
            .as_ref()
            .and_then(|c| c.incident_id.as_deref())
    }

    /// When the human last interacted, as far as this event can tell.
    pub fn witness_at(&self) -> Option<DateTime<Utc>> {
        let age = self.presence.last_witnessed_age_seconds?;
        Some(self.wall - chrono::Duration::milliseconds((age * 1000.0) as i64))
    }
}

type Parsed<T> = Result<T, String>;

fn time(v: &Value, what: &str) -> Parsed<DateTime<Utc>> {
    let s = v
        .as_str()
        .ok_or_else(|| format!("{what} must be an RFC 3339 string"))?;
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| format!("{what}: {e}"))
}

fn opt_time(v: Option<&Value>, what: &str) -> Parsed<Option<DateTime<Utc>>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => time(v, what).map(Some),
    }
}

fn opt_str(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_owned)
}

fn required_str<'a>(v: &'a Value, key: &str) -> Parsed<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn clock_at(v: Option<&Value>, what: &str) -> Parsed<Option<Moment>> {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    if v.is_string() {
        return Ok(Some(Moment::wall_only(time(v, what)?)));
    }
    Ok(Some(Moment {
        wall: time(
            v.get("wall")
                .ok_or_else(|| format!("{what}.wall is required"))?,
            what,
        )?,
        boot_id: required_str(v, "bootId")?.to_owned(),
        uptime_seconds: v
            .get("uptimeSeconds")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{what}.uptimeSeconds is required"))?,
    }))
}

fn parse_consultation(c: &Value) -> Parsed<ConsultationEvent> {
    let level = match c.get("level").and_then(Value::as_str) {
        None => None,
        Some("C1") => Some(Level::C1),
        Some("C2") => Some(Level::C2),
        Some("C3") => Some(Level::C3),
        Some("C4") => Some(Level::C4),
        Some(other) => return Err(format!("unknown capability level {other}")),
    };
    let decision = match c.get("decision").and_then(Value::as_str) {
        None => None,
        Some("Permit") => Some(Decision::Permit),
        Some("Refer") => Some(Decision::Refer(ReferQuestion {
            layer: "stasis-ladder".into(),
            reason: ReferReason::InsufficientAuthority,
            note: opt_str(c.get("reason")),
        })),
        Some(other) => return Err(format!("a keeper decides Permit or Refer, not {other}")),
    };
    let pivot = match c.get("pivot").and_then(Value::as_str) {
        None => None,
        Some("reshape") => Some(Pivot::Reshape),
        Some("park") => Some(Pivot::Park),
        Some("proceed") => Some(Pivot::Proceed),
        Some("notify") => Some(Pivot::Notify),
        Some(other) => return Err(format!("unknown pivot {other}")),
    };
    let phase = match c.get("phase").and_then(Value::as_str) {
        Some("start") => Phase::Start,
        Some("complete") | None => Phase::Complete,
        Some("expired") => Phase::Expire,
        Some(other) => return Err(format!("unknown consultation phase {other}")),
    };
    if phase == Phase::Complete && decision.is_none() {
        return Err("a completed consultation needs a decision".into());
    }
    let usage = c.get("usage");
    let claimed_by = c.get("claimedBy").and_then(|b| {
        Some(format!(
            "{}@{}",
            b.get("pid")?.as_u64()?,
            b.get("bootId")?.as_str()?
        ))
    });
    Ok(ConsultationEvent {
        phase,
        rung: c
            .get("rung")
            .and_then(Value::as_u64)
            .map(|r| r.min(5) as u8),
        level,
        binding: opt_str(c.get("binding")),
        decision,
        pivot,
        reason: opt_str(c.get("reason")),
        claim_seq: c.get("claimSeq").and_then(Value::as_u64),
        started_at: clock_at(c.get("startedAt"), "consultation.startedAt")?,
        deadline_at: opt_time(c.get("deadlineAt"), "consultation.deadlineAt")?,
        timeout_seconds: c.get("timeoutSeconds").and_then(Value::as_f64),
        finished_at: clock_at(c.get("finishedAt"), "consultation.finishedAt")?,
        claimed_by,
        usage_elapsed_s: usage
            .and_then(|u| u.get("elapsedSeconds"))
            .and_then(Value::as_f64),
        tokens: usage.and_then(|u| u.get("tokens")).and_then(Value::as_u64),
    })
}

impl Event {
    /// Read one journal line's JSON.
    pub fn from_value(v: &Value) -> Parsed<Self> {
        if v.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
            return Err("schemaVersion must be 1".into());
        }
        let kind_name = required_str(v, "kind")?;
        let kind =
            EventKind::parse(kind_name).ok_or_else(|| format!("unknown event kind {kind_name}"))?;
        let s = v.get("session").ok_or("session is required")?;
        let session = SessionIdentity {
            runtime: required_str(s, "runtime")?.to_owned(),
            session: Some(required_str(s, "session")?.to_owned()),
            owner_pid: s.get("ownerPid").and_then(Value::as_u64).map(|p| p as u32),
            owner_start_time: s
                .get("ownerStartTime")
                .and_then(Value::as_str)
                .and_then(|t| t.parse().ok()),
            source: Source::Declared,
        };
        let clock = v.get("clock").ok_or("clock is required")?;
        let p = v.get("presence").ok_or("presence is required")?;
        let disposition = match p.get("disposition").and_then(Value::as_str) {
            Some("collaborative") => Disposition::Collaborative,
            Some("automated") => Disposition::Automated,
            Some("unknown") => Disposition::Unknown,
            _ => return Err("presence.disposition is required".into()),
        };
        let basis = p.get("basis");
        let presence = Presence {
            disposition,
            last_witnessed_age_seconds: basis
                .and_then(|b| b.get("lastWitnessedAgeSeconds"))
                .and_then(Value::as_f64),
            ruling_ref: opt_str(basis.and_then(|b| b.get("rulingRef"))),
        };
        let correlation = match v.get("correlation") {
            None => None,
            Some(c) => Some(Correlation {
                incident_id: opt_str(c.get("incidentId")),
                confidence: match c.get("confidence").and_then(Value::as_str) {
                    Some("high") => Confidence::High,
                    Some("low") => Confidence::Low,
                    Some("unknown") => Confidence::Unknown,
                    _ => return Err("correlation.confidence is required".into()),
                },
            }),
        };
        let shift = match v.get("shift") {
            None => None,
            Some(s) => Some(ShiftMarker {
                id: required_str(s, "id")?.to_owned(),
                open: match s.get("edge").and_then(Value::as_str) {
                    Some("open") => true,
                    Some("close") => false,
                    _ => return Err("shift.edge must be open or close".into()),
                },
            }),
        };
        let gap = match v.get("gap") {
            None => None,
            Some(g) => Some(GapNote {
                reason: required_str(g, "reason")?.to_owned(),
                from: opt_time(g.get("from"), "gap.from")?,
                to: opt_time(g.get("to"), "gap.to")?,
            }),
        };
        let question = match v.get("question").and_then(Value::as_str) {
            None => None,
            Some("yes") => Some(Question::Yes),
            Some("no") => Some(Question::No),
            Some("uncertain") => Some(Question::Uncertain),
            Some(other) => return Err(format!("unknown question class {other}")),
        };
        let closed_by = match v.get("closedBy").and_then(Value::as_str) {
            None => None,
            Some("approved") => Some(CloseAttribution::Approved),
            Some("denied-inferred") => Some(CloseAttribution::DeniedInferred),
            Some("unknown") => Some(CloseAttribution::Unknown),
            Some("withdrawn") => Some(CloseAttribution::Withdrawn),
            Some("expired") => Some(CloseAttribution::Expired),
            Some(other) => return Err(format!("unknown close attribution {other}")),
        };
        let exception = match v.get("exception") {
            None => None,
            Some(e) => Some(ExceptionNote {
                hook_name: required_str(e, "hookName")?.to_owned(),
                count: e
                    .get("count")
                    .and_then(Value::as_u64)
                    .ok_or("exception.count is required")?,
                baseline: e.get("baseline").and_then(Value::as_bool).unwrap_or(false),
            }),
        };
        let would_have = match v.get("wouldHave") {
            None => None,
            Some(w) => Some(WouldHave {
                effect: required_str(w, "effect")?.to_owned(),
                incident_id: opt_str(w.get("incidentId")),
                rung: w.get("rung").and_then(Value::as_u64).map(|r| r as u8),
                key: required_str(w, "key")?.to_owned(),
            }),
        };
        let consultation = match v.get("consultation") {
            None => None,
            Some(c) => Some(parse_consultation(c)?),
        };
        let event = Self {
            kind,
            session,
            agent_id: required_str(v, "agentId")?.to_owned(),
            seq: v
                .get("seq")
                .and_then(Value::as_u64)
                .ok_or("seq is required")?,
            wall: time(
                clock.get("wall").ok_or("clock.wall is required")?,
                "clock.wall",
            )?,
            boot_id: required_str(clock, "bootId")?.to_owned(),
            uptime_seconds: clock
                .get("uptimeSeconds")
                .and_then(Value::as_f64)
                .ok_or("clock.uptimeSeconds is required")?,
            presence,
            shift,
            tool: opt_str(v.get("tool")),
            shape: opt_str(v.get("shape")),
            duration_ms: v.get("durationMs").and_then(Value::as_f64),
            correlation,
            gap,
            question,
            consultation,
            closed_by,
            exception,
            would_have,
        };
        let need = |ok: bool, what: &str| {
            if ok {
                Ok(())
            } else {
                Err(format!("{kind_name} requires {what}"))
            }
        };
        match kind {
            EventKind::ShiftMarker => need(event.shift.is_some(), "shift")?,
            EventKind::PromptClosed => need(
                event.closed_by.is_some() && event.correlation.is_some(),
                "closedBy and correlation",
            )?,
            EventKind::ObservationGap => need(event.gap.is_some(), "gap")?,
            EventKind::TurnEnded => need(event.question.is_some(), "question")?,
            EventKind::Consultation => need(event.consultation.is_some(), "consultation")?,
            EventKind::WouldHave => need(event.would_have.is_some(), "wouldHave")?,
            EventKind::HarnessException => need(event.exception.is_some(), "exception")?,
            EventKind::PermissionRequested | EventKind::DialogDisplayed => need(
                event.tool.is_some() && event.correlation.is_some(),
                "tool and correlation",
            )?,
            _ => {}
        }
        Ok(event)
    }
}
