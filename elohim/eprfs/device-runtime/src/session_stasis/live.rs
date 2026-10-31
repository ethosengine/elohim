//! Live `decide` under a pinned policy: the regulator's own, deliberately narrow, acts.
//!
//! Spec: Ladder → Pivots, Automated path and questions, Denial authority and human delivery.
//! The only effects are a `deny` (never an `allow`: there is no allow-shaped value anywhere in
//! this module), and `none`. A deny parks a PermissionRequest in automated presence, or offers a
//! learned legible form at rung 0. Every act is journaled as a rung-0 consultation, each record
//! its own consultation, before it is answered.
//!
//! * **Park** (automated presence only): `Permit` + `park`. Parking grants no authority to run
//!   the step; its lifecycle (`pending → answered | withdrawn | expired`) is the batch's.
//! * **Rung-0 reshape**: shape-class concerns only, with no ask or deny rule matching, whose
//!   effect equals an ACTIVE learned form of the pinned policy. One per incident (an incident has
//!   one effect class); a second **Refers** instead. Risk and uncertain never reshape.
//! * **`proceed`** from a keeper is never executed here: it is recorded and downgraded to a park
//!   until scoped-ruling validation exists.

use chrono::SecondsFormat;
use elohim_epr::verdict::Decision;
use serde::Serialize;
use serde_json::{json, Value};

use super::classify::{CallInput, Classifier, ConcernClass, StallClass};
use super::event::{Disposition, Pivot};
use super::journal::{self, StasisError};
use super::policy::PolicyAssessment;
use super::{
    fold_session, AwayRuling, ClockReading, DecideRequest, Effect, FoldOptions, Incident,
    IncidentState, Selection,
};

/// What live `decide` prints once the policy is pinned-match.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveOutput {
    /// `deny` or `none`; never anything that approves.
    pub effect: Effect,
    /// The message to deliver with a deny.
    pub message: Option<String>,
    /// The incident concerned.
    pub incident_id: Option<String>,
    /// Its rung.
    pub rung: u8,
    /// Always false: this is live.
    pub shadow: bool,
    /// Always true.
    pub live: bool,
    /// The policy label.
    pub policy: String,
    /// The pinned policy's CID.
    pub policy_cid: String,
    /// True when the answer is a Refer (a second reshape was asked for).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub refer: bool,
}

/// A LIVE Permit consultation with `pivot`: shadow records, and live-claiming records that do not
/// verify against the pinned policy, never count toward a park, a reshape allowance or a Refer.
fn consultation_of(inc: &Incident, pivot: Pivot) -> Option<&super::Consultation> {
    inc.consultations
        .iter()
        .find(|c| c.live && c.pivot == Some(pivot) && matches!(c.decision, Some(Decision::Permit)))
}

/// The fixed agent identity every live regulator record is written under.
const REGULATOR_AGENT: &str = "stasis-regulator";

struct Writer<'a> {
    dir: &'a std::path::Path,
    session: &'a str,
    /// The presence of the journal's last event, carried onto every record this writer appends:
    /// the away ruling and disposition are read from the LAST event, so a regulator record must
    /// not clear them. The witness age is not carried (this record witnesses nothing).
    presence: Value,
    /// The pinned policy CID these records are decided under.
    policy_cid: String,
    runtime: String,
    owner_pid: Option<u32>,
    owner_start: Option<String>,
    next_seq: u64,
    clock: &'a ClockReading,
    segment_limit: u64,
}

impl Writer<'_> {
    /// Append one rung-0 consultation on `incident`.
    fn consult(
        &mut self,
        incident: &str,
        decision: &str,
        pivot: Option<&str>,
        reason: &str,
    ) -> Result<(), StasisError> {
        let seq = self.next_seq;
        self.next_seq += 1;
        let now = self.clock.wall.to_rfc3339_opts(SecondsFormat::Millis, true);
        // A single-record completion (no `phase`): the regulator's act is decided in one write.
        let c = json!({
            "rung": 0,
            "live": true,
            "level": "C1",
            "binding": "epr-session-stasis/regulator",
            "decision": decision,
            "pivot": pivot,
            "reason": reason,
        });
        let event = json!({
            "schemaVersion": 1,
            "kind": "consultation",
            "session": {
                "runtime": self.runtime,
                "session": self.session,
                "ownerPid": self.owner_pid,
                "ownerStartTime": self.owner_start,
            },
            "agentId": REGULATOR_AGENT,
            "regulator": {"kind": "native-decide"},
            "policyCid": self.policy_cid,
            "seq": seq,
            "clock": {
                "wall": now,
                "bootId": self.clock.boot_id,
                "uptimeSeconds": self.clock.uptime_seconds,
                "provenance": "epr-session-stasis/linux-proc",
            },
            "presence": self.presence,
            "correlation": {"incidentId": incident, "confidence": "high", "method": "none"},
            "consultation": c,
        });
        journal::append_locked(self.dir, &event, seq, self.segment_limit)
    }
}

/// The outcome of a live decision: an answer, or the reason the regulator stayed inert.
#[derive(Debug, Clone)]
pub enum LiveDecision {
    /// The policy was pinned under the lock; this is the answer.
    Acted(LiveOutput),
    /// The policy no longer verified under the lock; nothing was written.
    Inert(String),
}

/// Decide under a pinned policy. `recheck` re-reads the policy bytes and the pin AFTER this
/// function holds the journal lock; the decision uses that assessment (its policy, label and CID),
/// never one taken before the lock, and anything but a pinned match is inert.
/// `rulings` are the away rulings the caller resolved from the flow sidecar; an unresolved
/// reference counts as not away.
#[allow(
    clippy::too_many_arguments,
    reason = "The live decision needs the journal root, the request, the classifier, the re-verification, the resolved rulings and clock; bundling them would only rename the same inputs"
)]
pub fn decide_live(
    root: &std::path::Path,
    session: &str,
    req: &DecideRequest,
    classifier: &Classifier,
    recheck: &dyn Fn() -> Result<PolicyAssessment, StasisError>,
    rulings: &[AwayRuling],
    reclock: &dyn Fn() -> ClockReading,
    segment_limit: u64,
) -> Result<LiveDecision, StasisError> {
    let dir = root.join(session);
    let _lock = if dir.is_dir() {
        Some(journal::Lock::exclusive(&dir)?)
    } else {
        None
    };
    // The clock is read AFTER the lock is held, so away-ruling expiry and presence are judged at
    // the moment of the decision, not at process start.
    let clock_now = reclock();
    let clock = &clock_now;
    let assessment = recheck()?;
    if let Some(reason) = assessment.inert_reason() {
        return Ok(LiveDecision::Inert(reason));
    }
    let Some(policy_cid) = assessment.cid().map(str::to_owned) else {
        return Ok(LiveDecision::Inert("policy CID unavailable".into()));
    };
    let policy = &assessment.policy;
    let policy_label = assessment.label(true);
    let mut out = LiveOutput {
        effect: Effect::None,
        message: None,
        incident_id: req.incident_id.clone(),
        rung: 0,
        shadow: false,
        live: true,
        policy: policy_label,
        policy_cid: policy_cid.clone(),
        refer: false,
    };
    if _lock.is_none() {
        return Ok(LiveDecision::Acted(out));
    }
    let replay = match journal::replay_unlocked(&dir, session) {
        Ok(r) => r,
        Err(StasisError::NoJournal(_)) => return Ok(LiveDecision::Acted(out)),
        Err(e) => return Err(e),
    };
    let Some(last) = replay.events.last() else {
        return Ok(LiveDecision::Acted(out));
    };
    let fold = fold_session(
        &replay,
        &FoldOptions::new(clock.wall, Selection::All, true, policy, classifier)
            .with_clock(&clock.boot_id, clock.uptime_seconds)
            .with_rulings(rulings)
            .with_pinned_cid(&policy_cid),
    );
    let by_id = req
        .incident_id
        .as_deref()
        .and_then(|id| fold.incidents.iter().find(|i| i.id == id));
    let incident = by_id.or_else(|| {
        fold.incidents
            .iter()
            .rev()
            .find(|i| i.state != IncidentState::Closed)
    });
    let mut w = Writer {
        dir: &dir,
        session,
        presence: json!({
            "disposition": super::disposition_name(last.presence.disposition),
            "basis": {
                "lastWitnessedAgeSeconds": null,
                "rulingRef": last.presence.ruling_ref,
            },
        }),
        policy_cid: policy_cid.clone(),
        runtime: last.session.runtime.clone(),
        owner_pid: last.session.owner_pid,
        owner_start: last.session.owner_start_time.map(|t| t.to_string()),
        next_seq: replay
            .events
            .iter()
            .map(|e| e.seq)
            .max()
            .unwrap_or(last.seq)
            + 1,
        clock,
        segment_limit,
    };
    let Some(inc) = incident else {
        return Ok(LiveDecision::Acted(out));
    };
    out.incident_id = Some(inc.id.clone());
    out.rung = inc.rung;
    let mut parked = consultation_of(inc, Pivot::Park).is_some();

    // A keeper's `proceed` is never executed here. A LIVE proceed (the coordinator's) is
    // recorded as downgraded to park, once, and only in automated presence: in collaborative or
    // unknown presence nothing is written. A shadow proceed never reaches this.
    if inc.state != IncidentState::Closed
        && fold.presence.disposition == Disposition::Automated
        && !parked
        && consultation_of(inc, Pivot::Proceed).is_some()
    {
        w.consult(
            &inc.id,
            "Permit",
            Some("park"),
            "proceed downgraded to park: scoped-ruling validation is not implemented; \
             ask: resolve this parked step; recommendation: keep it parked",
        )?;
        parked = true;
    }
    if inc.state == IncidentState::Closed {
        return Ok(LiveDecision::Acted(out));
    }

    match req.hook.as_str() {
        "PermissionRequest" => {
            if inc.stall_class != StallClass::Authorization
                || fold.presence.disposition != Disposition::Automated
            {
                // Collaborative (or unknown) presence: the dialog displays; the coordinator
                // harvests at Approach and Breach.
                return Ok(LiveDecision::Acted(out));
            }
            if !parked {
                let shape = req.shape.as_deref().unwrap_or("-");
                w.consult(
                    &inc.id,
                    "Permit",
                    Some("park"),
                    &format!(
                        "parked in automated presence: step `{shape}` awaits steward guidance; \
                         ask: resolve this parked step; recommendation: keep it parked pending \
                         fresh validation"
                    ),
                )?;
            }
            out.effect = Effect::Deny;
            out.message = Some(format!(
                "parked as stasis `{}`. Do not retry this effect by any tool or spelling. \
                 Continue independent authorized work. Guidance will arrive.",
                inc.id
            ));
        }
        "PreToolUse" => {
            let (Some(tool), Some(shape)) = (req.tool.as_deref(), req.shape.as_deref()) else {
                return Ok(LiveDecision::Acted(out));
            };
            let c = classifier.classify(StallClass::Authorization, tool, CallInput::Shape(shape));
            // Risk, uncertain and any ask/deny match never get a reshape.
            // Both the candidate AND the incident's recorded concern must be shape, with no
            // ask/deny rule matching; risk and uncertain never get a reshape.
            if inc.concern_class != ConcernClass::Shape
                || c.concern != ConcernClass::Shape
                || c.rule_match != "none"
            {
                return Ok(LiveDecision::Acted(out));
            }
            let effect = serde_json::to_value(&c.effect).unwrap_or(Value::Null);
            // Only a fully resolved, equal effect: an unresolved target or path never reshapes.
            if !c.effect.same_effect(&inc.effect_class) {
                return Ok(LiveDecision::Acted(out));
            }
            let Some(form) = policy.learned_forms.iter().find(|f| {
                f.is_active() && f.tool == tool && f.shape == shape && f.effect_class == effect
            }) else {
                return Ok(LiveDecision::Acted(out));
            };
            if consultation_of(inc, Pivot::Reshape).is_some() {
                // The allowance is spent: Refer, once, instead of a second reshape.
                let referred = inc.consultations.iter().any(|x| {
                    matches!(x.decision, Some(Decision::Refer(_)))
                        && x.reason
                            .as_deref()
                            .is_some_and(|r| r.starts_with("reshape allowance"))
                });
                if !referred {
                    w.consult(
                        &inc.id,
                        "Refer",
                        None,
                        "reshape allowance already used for this incident; referring up the ladder",
                    )?;
                }
                out.refer = true;
                out.rung = out.rung.max(1);
                return Ok(LiveDecision::Acted(out));
            }
            w.consult(
                &inc.id,
                "Permit",
                Some("reshape"),
                &format!("reshape `{shape}` to learned form `{}`", form.name),
            )?;
            out.effect = Effect::Deny;
            out.message = Some(format!(
                "use the legible form: {} (same effect, passes the same permission checks)",
                form.legible_form
            ));
        }
        _ => {}
    }
    Ok(LiveDecision::Acted(out))
}
