//! Synthetic-journal fixtures for the dialogue-stasis fold (Verification, slice 5). No keeper,
//! no coordinator, no live harness: every event here is built by hand, on a fake clock.

use std::{fs, path::Path};

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::{json, Value};

use super::classify::{Classifier, ConcernClass, EffectContext, RuleSet, StallClass};
use super::event::{Disposition, Event, EventKind, Moment, Pivot, SCHEMA_KINDS};
use super::journal::{self, Replay, StasisError};
use super::*;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 31, 10, 0, 0).unwrap()
}

fn at(secs: i64) -> DateTime<Utc> {
    t0() + Duration::seconds(secs)
}

fn stamp(secs: i64) -> String {
    at(secs).to_rfc3339()
}

/// An event at `t` seconds after the fixture epoch. Presence is unknown with no witness.
fn ev(seq: u64, t: i64, kind: &str, extra: Value) -> Value {
    let mut v = json!({
        "schemaVersion": 1, "kind": kind,
        "session": {"runtime": "claude-code", "session": "s1"},
        "agentId": "main", "seq": seq,
        "clock": {"wall": stamp(t), "bootId": "boot-1", "uptimeSeconds": 1000.0 + t as f64, "provenance": "test"},
        "presence": {"disposition": "unknown", "basis": {"lastWitnessedAgeSeconds": null, "rulingRef": null}},
    });
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

fn witnessed(age: f64) -> Value {
    json!({"presence": {"disposition": "collaborative", "basis": {"lastWitnessedAgeSeconds": age, "rulingRef": null}}})
}

fn corr(id: &str, conf: &str) -> Value {
    json!({"incidentId": id, "confidence": conf, "method": "prompt-id-preceding-pretooluse"})
}

fn perm(seq: u64, t: i64, shape: &str) -> Value {
    ev(
        seq,
        t,
        "permission-requested",
        json!({"tool": "Bash", "shape": shape, "source": {"hookEvent": "PermissionRequest"},
               "correlation": corr(&format!("s1:{seq}"), "high")}),
    )
}

fn dialog(seq: u64, t: i64, incident: &str) -> Value {
    ev(
        seq,
        t,
        "dialog-displayed",
        json!({"tool": "Bash", "source": {"hookEvent": "Notification"}, "correlation": corr(incident, "high")}),
    )
}

fn closed(seq: u64, t: i64, incident: &str, by: &str, duration_ms: Value) -> Value {
    ev(
        seq,
        t,
        "prompt-closed",
        json!({"closedBy": by, "durationMs": duration_ms, "correlation": corr(incident, "low")}),
    )
}

fn consult(seq: u64, t: i64, incident: &str, body: Value) -> Value {
    ev(
        seq,
        t,
        "consultation",
        json!({"correlation": corr(incident, "high"), "consultation": body}),
    )
}

fn clk(t: i64) -> Value {
    json!({"wall": stamp(t), "bootId": "boot-1", "uptimeSeconds": 1000.0 + t as f64, "provenance": "test"})
}

fn start(rung: u64, t: i64, deadline: i64) -> Value {
    json!({"phase": "start", "rung": rung, "level": format!("C{rung}"), "binding": "claude/sonnet",
           "startedAt": clk(t), "deadlineAt": stamp(deadline), "claimedBy": {"pid": 4242, "bootId": "boot-1"},
           "timeoutSeconds": (deadline - t) as f64})
}

fn usage(tokens: Option<u64>) -> Value {
    json!({"elapsedSeconds": null, "tokens": tokens})
}

fn finish(rung: u64, decision: &str, pivot: &str, tokens: Option<u64>) -> Value {
    json!({"phase": "complete", "rung": rung, "level": format!("C{rung}"), "binding": "claude/sonnet",
           "decision": decision, "pivot": pivot, "reason": "because", "usage": usage(tokens)})
}

fn replay_of(events: &[Value]) -> Replay {
    let parsed: Vec<Event> = events
        .iter()
        .map(|v| Event::from_value(v).unwrap_or_else(|e| panic!("{e}: {v}")))
        .collect();
    Replay {
        session: "s1".into(),
        events: parsed,
        gaps: vec![],
        segments: 1,
    }
}

fn classifier() -> Classifier {
    let settings = json!({"permissions": {"ask": ["Bash(git *rebase*)", "Bash(git *push* +*)"], "deny": ["Bash(rm -rf:*)"]}});
    Classifier::new(
        Some(RuleSet::from_settings(&[settings])),
        EffectContext::default(),
    )
}

fn run(events: &[Value], now: i64, selection: Selection) -> SessionFold {
    let policy = StasisPolicy::builtin();
    let c = classifier();
    fold_session(
        &replay_of(events),
        &FoldOptions::new(at(now), selection, true, &policy, &c),
    )
}

fn blocked(f: &SessionFold) -> Option<f64> {
    f.windows[f.selected[0]].value_seconds
}

// --- Every schema event kind -------------------------------------------------------------

fn one_of_each() -> Vec<Value> {
    vec![
        perm(1, 0, "git status"),
        dialog(2, 5, "s1:1"),
        closed(3, 60, "s1:1", "approved", json!(1000)),
        ev(
            4,
            61,
            "classifier-denied",
            json!({"tool": "Bash", "shape": "rm -rf <path>"}),
        ),
        ev(5, 62, "turn-ended", json!({"question": "no"})),
        ev(
            12,
            69,
            "idle-prompt",
            json!({"source": {"hookEvent": "Notification"}}),
        ),
        consult(
            6,
            63,
            "s1:1",
            json!({"level": "C1", "binding": "claude/haiku", "decision": "Refer", "pivot": "notify", "reason": "x"}),
        ),
        ev(7, 64, "steward-ping", json!({})),
        ev(
            8,
            65,
            "observation-gap",
            json!({"gap": {"reason": "hook-error", "from": null, "to": null}}),
        ),
        ev(
            9,
            66,
            "shift-marker",
            json!({"shift": {"id": "sh1", "edge": "open"}}),
        ),
        ev(
            10,
            67,
            "harness-exception",
            json!({"exception": {"source": "hook_cancelled", "hookName": "native-seam-guidance", "durationMs": 2000, "timeoutMs": 2000, "count": 3}}),
        ),
        ev(11, 68, "session-ended", json!({})),
    ]
}

#[test]
fn every_schema_event_kind_validates_parses_and_folds() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sdk/domains/elohim-agent/schemas/session-event.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
    let declared: Vec<&str> = schema["properties"]["kind"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    assert_eq!(
        declared, SCHEMA_KINDS,
        "the fold models exactly the schema's kinds"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut events = one_of_each();
    events.push(ev(
        14,
        71,
        "would-have",
        json!({"wouldHave": {"effect": "none", "incidentId": "s1:1", "rung": 0, "key": "none:s1:1:0"}}),
    ));
    for v in &events {
        assert!(validator.is_valid(v), "fixture is schema-valid: {v}");
        Event::from_value(v).unwrap();
    }
    let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    for k in SCHEMA_KINDS {
        assert!(kinds.contains(&k), "fixture covers {k}");
    }
    let f = run(&events, 100, Selection::All);
    assert!(f.ended);
    assert_eq!(f.classifier_denied, 1);
    assert_eq!(
        f.incidents
            .iter()
            .filter(|i| i.stall_class == StallClass::Silence)
            .count(),
        1
    );
}

#[test]
fn would_have_and_unsupported_shapes_are_handled() {
    let w = ev(
        2,
        5,
        "would-have",
        json!({"wouldHave": {"effect": "none", "incidentId": "s1:1", "rung": 0, "key": "none:s1:1:0"}}),
    );
    let f = run(&[perm(1, 0, "git status"), w], 10, Selection::All);
    assert_eq!(f.would_have, 1);
    assert_eq!(
        f.incidents.len(),
        1,
        "a shadow record never opens or closes an incident"
    );
    assert!(Event::from_value(&ev(1, 0, "no-such-kind", json!({}))).is_err());
    let refuse = consult(
        1,
        0,
        "s1:1",
        json!({"level": "C1", "binding": "b", "decision": "Refuse", "pivot": "park", "reason": "r"}),
    );
    assert!(
        Event::from_value(&refuse).is_err(),
        "a keeper never refuses on the steward's behalf"
    );
    assert!(Event::from_value(&json!({"schemaVersion": 2})).is_err());
}

// --- Incident closure at every attribution -------------------------------------------------

fn close_of(events: &[Value], now: i64) -> (Option<ClosedBy>, bool) {
    let f = run(events, now, Selection::All);
    (f.incidents[0].closed_by, f.incidents[0].close_unknown)
}

#[test]
fn closure_is_attributed_for_every_listed_close() {
    assert_eq!(
        close_of(
            &[
                perm(1, 0, "ls"),
                closed(2, 50, "s1:1", "approved", json!(0))
            ],
            60
        )
        .0,
        Some(ClosedBy::Approved)
    );
    assert_eq!(
        close_of(
            &[
                perm(1, 0, "ls"),
                closed(2, 50, "s1:1", "denied-inferred", Value::Null)
            ],
            60
        )
        .0,
        Some(ClosedBy::DeniedInferred)
    );
    assert_eq!(
        close_of(
            &[
                perm(1, 0, "ls"),
                closed(2, 50, "s1:1", "withdrawn", Value::Null)
            ],
            60
        )
        .0,
        Some(ClosedBy::Withdrawn)
    );
    assert_eq!(
        close_of(
            &[
                perm(1, 0, "ls"),
                closed(2, 50, "s1:1", "expired", Value::Null)
            ],
            60
        )
        .0,
        Some(ClosedBy::Expired)
    );
    assert_eq!(
        close_of(
            &[perm(1, 0, "ls"), ev(2, 50, "session-ended", json!({}))],
            60
        )
        .0,
        Some(ClosedBy::SessionEnded)
    );
    // Shadow consultation outcomes never close a stall: a Permit/reshape, park or proceed is
    // recorded and the human is still waiting, so the incident stays open.
    let reshaped = [
        perm(1, 0, "cd <path> && ls"),
        consult(2, 10, "s1:1", finish(1, "Permit", "reshape", Some(10))),
        closed(3, 20, "s1:1", "denied-inferred", Value::Null),
    ];
    assert_eq!(close_of(&reshaped, 60).0, Some(ClosedBy::DeniedInferred));
    let parked = [
        perm(1, 0, "git rebase main"),
        consult(2, 30, "s1:1", finish(1, "Permit", "park", None)),
    ];
    let f = run(&parked, 60, Selection::All);
    assert_eq!(
        (
            f.incidents[0].closed_by,
            f.incidents[0].state != IncidentState::Closed
        ),
        (None, true)
    );
    assert_eq!(f.incidents[0].consultations[0].pivot, Some(Pivot::Park));
    let proceeded = [
        ev(1, 0, "turn-ended", json!({"question": "yes"})),
        consult(2, 30, "s1:1", finish(2, "Permit", "proceed", None)),
    ];
    assert_eq!(
        close_of(&proceeded, 60).0,
        None,
        "proceed needs a validated ruling (slice 6)"
    );
    // A close that cannot be established is flagged, never confident.
    let (by, unknown) = close_of(
        &[
            perm(1, 0, "ls"),
            closed(2, 50, "s1:1", "unknown", Value::Null),
        ],
        60,
    );
    assert_eq!((by, unknown), (None, true));
}

#[test]
fn questions_and_silence_close_on_evidence() {
    // Answered: a later event proves a witnessed interaction after the question.
    let answered = [
        ev(1, 0, "turn-ended", json!({"question": "yes"})),
        ev(2, 100, "turn-ended", {
            let mut p = witnessed(30.0);
            p["question"] = json!("no");
            p
        }),
    ];
    let f = run(&answered, 200, Selection::All);
    let q = &f.incidents[0];
    assert_eq!(
        (q.stall_class, q.closed_by, q.closed_at),
        (StallClass::Question, Some(ClosedBy::Answered), Some(at(70)))
    );
    // Withdrawn: the agent carried on with no witness since.
    let withdrawn = [
        ev(1, 0, "turn-ended", json!({"question": "yes"})),
        ev(2, 100, "turn-ended", json!({"question": "no"})),
    ];
    assert_eq!(
        run(&withdrawn, 200, Selection::All).incidents[0].closed_by,
        Some(ClosedBy::Withdrawn)
    );
    // Uncertain never blocks.
    assert!(run(
        &[ev(1, 0, "turn-ended", json!({"question": "uncertain"}))],
        200,
        Selection::All
    )
    .incidents
    .is_empty());
    // Silence: a ping opens it, the agent's next turn answers it.
    let silence = [
        ev(1, 0, "steward-ping", json!({})),
        ev(2, 90, "turn-ended", json!({"question": "no"})),
    ];
    let f = run(&silence, 200, Selection::All);
    assert_eq!(
        (f.incidents[0].stall_class, f.incidents[0].closed_by),
        (StallClass::Silence, Some(ClosedBy::Answered))
    );
    assert_eq!(blocked(&f), Some(90.0));
    // Question and silence stalls are always uncertain: no same-effect reshape is ever offered.
    assert!(f
        .incidents
        .iter()
        .all(|i| i.concern_class == ConcernClass::Uncertain));
}

#[test]
fn incidents_classify_from_the_recorded_shape() {
    let f = run(
        &[
            perm(1, 0, "git -C <path> rebase --onto <ref> <ref>"),
            perm(2, 1, "cd <path> && pnpm exec vitest run <path>"),
            perm(3, 2, "cargo test"),
        ],
        10,
        Selection::All,
    );
    let classes: Vec<ConcernClass> = f.incidents.iter().map(|i| i.concern_class).collect();
    assert_eq!(
        classes,
        [
            ConcernClass::Risk,
            ConcernClass::Shape,
            ConcernClass::Uncertain
        ]
    );
    assert_eq!(
        f.incidents[0].matched_rule.as_deref(),
        Some("Bash(git *rebase*)")
    );
}

// --- Authorization interval evidence -------------------------------------------------------

#[test]
fn authorization_interval_starts_at_the_dialog_when_it_was_displayed() {
    let with = [
        perm(1, 0, "ls"),
        dialog(2, 6, "s1:1"),
        closed(3, 100, "s1:1", "approved", json!(4000)),
    ];
    let f = run(&with, 120, Selection::All);
    assert_eq!(
        blocked(&f),
        Some(90.0),
        "6 s to 96 s: approval time is PostToolUse minus duration"
    );
    assert_eq!(f.incidents[0].interval_confidence, Confidence::High);
    assert_eq!(f.incidents[0].blocked_intervals[0].to, Some(at(96)));
}

#[test]
fn absent_dialog_falls_back_to_the_request_with_low_confidence() {
    let without = [
        perm(1, 0, "ls"),
        closed(2, 100, "s1:1", "approved", json!(4000)),
    ];
    let f = run(&without, 120, Selection::All);
    assert_eq!(blocked(&f), Some(96.0));
    assert_eq!(f.incidents[0].interval_confidence, Confidence::Low);
    assert_eq!(f.windows[f.selected[0]].low_confidence_intervals, 1);
    // An uncorrelated dialog is never guessed onto an incident.
    let uncorrelated = ev(
        2,
        6,
        "dialog-displayed",
        json!({"tool": "Bash", "source": {"hookEvent": "Notification"},
        "correlation": {"incidentId": null, "confidence": "unknown", "method": "none"}}),
    );
    let f = run(&[perm(1, 0, "ls"), uncorrelated], 20, Selection::All);
    assert_eq!(f.incidents[0].interval_confidence, Confidence::Low);
    assert!(f.gaps.iter().any(|g| g.reason == "uncorrelated-dialog"));
}

// --- Stage thresholds ----------------------------------------------------------------------

#[test]
fn stage_follows_the_stall_bound_from_the_incident_opening() {
    let open = [perm(1, 0, "ls")];
    let state = |now| run(&open, now, Selection::All).incidents[0].state;
    assert_eq!(state(179), IncidentState::Open);
    assert_eq!(state(180), IncidentState::Approach);
    assert_eq!(state(299), IncidentState::Approach);
    assert_eq!(state(300), IncidentState::Breach);
    let p = StasisPolicy::builtin();
    assert_eq!(p.approach_s(), 180.0);
    let closed = [
        perm(1, 0, "ls"),
        closed(2, 400, "s1:1", "approved", json!(0)),
    ];
    assert_eq!(
        run(&closed, 500, Selection::All).incidents[0].state,
        IncidentState::Closed
    );
}

#[test]
fn latch_declaration_reuses_the_shared_hysteresis() {
    let p = StasisPolicy::builtin();
    let config = p.latch_config("stasis", "s1:authorization", "bafy-bound");
    config.hysteresis.validate(config.sense).unwrap();
    assert_eq!(config.hysteresis.edge_pct(config.sense), 60.0);
    assert_eq!((config.limit, config.hysteresis.dwell_s), (300.0, 300));
}

// --- Interval union, shifts, reset boundary -------------------------------------------------

#[test]
fn overlapping_and_duplicate_intervals_never_inflate_the_union() {
    let events = [
        perm(1, 0, "ls"),
        perm(2, 50, "cat x"),
        // A duplicate opening event for incident s1:1 folds into the one incident.
        ev(
            3,
            60,
            "permission-requested",
            json!({"tool": "Bash", "shape": "ls", "source": {"hookEvent": "PermissionRequest"}, "correlation": corr("s1:1", "high")}),
        ),
        closed(4, 100, "s1:1", "approved", json!(0)),
        closed(5, 200, "s1:2", "approved", json!(0)),
        ev(6, 250, "turn-ended", json!({"question": "yes"})),
        ev(7, 300, "steward-ping", json!({})),
    ];
    let f = run(&events, 400, Selection::All);
    let w = &f.windows[f.selected[0]];
    assert_eq!(
        f.incidents
            .iter()
            .filter(|i| i.stall_class == StallClass::Authorization)
            .count(),
        2
    );
    // Authorization: [0,100] U [50,200] = 200 (not 250). The question [250, 400) and silence
    // [300, 400) overlap each other: the total is the union across classes, not their sum.
    assert_eq!(w.by_class["authorization"], 200.0);
    assert_eq!(w.value_seconds, Some(200.0 + 150.0));
    assert_eq!(
        w.by_class["question"] + w.by_class["silence"],
        150.0 + 100.0
    );
}

#[test]
fn presence_buckets_by_disposition_at_open() {
    let mut a = perm(1, 0, "ls");
    a["presence"] = json!({"disposition": "automated", "basis": {"lastWitnessedAgeSeconds": 900.0, "rulingRef": null}});
    let b = perm(2, 1000, "ls");
    let events = [
        a,
        closed(3, 100, "s1:1", "approved", json!(0)),
        b,
        closed(4, 1050, "s1:2", "approved", json!(0)),
    ];
    let w = &run(&events, 1100, Selection::All).windows;
    let w = &w[w.len() - 1];
    assert_eq!(
        (
            w.by_presence["automated"],
            w.by_presence["unknown"],
            w.by_presence["collaborative"]
        ),
        (100.0, 50.0, 0.0)
    );
}

fn shift(seq: u64, t: i64, id: &str, edge: &str) -> Value {
    ev(
        seq,
        t,
        "shift-marker",
        json!({"shift": {"id": id, "edge": edge}}),
    )
}

#[test]
fn shift_markers_select_the_window_and_reset_the_cumulative_measure() {
    let events = [
        shift(1, 0, "shiftA", "open"),
        perm(2, 10, "ls"),
        closed(3, 110, "s1:2", "approved", json!(0)),
        shift(4, 200, "shiftA", "close"),
        shift(5, 300, "shiftB", "open"),
        perm(6, 310, "ls"),
        closed(7, 360, "s1:6", "approved", json!(0)),
    ];
    // shiftB is the latest open marker window: its total resets, it does not carry shiftA's 100 s.
    let f = run(&events, 400, Selection::ShiftCurrent);
    let w = &f.windows[f.selected[0]];
    assert_eq!(
        (w.id.as_str(), w.kind.as_str(), w.value_seconds),
        ("shiftB", "marker", Some(50.0))
    );
    // Before shiftB opened, shiftA was the window.
    let f = run(&events[..4], 250, Selection::ShiftCurrent);
    assert_eq!(f.windows[f.selected[0]].value_seconds, Some(100.0));
    // With every marker closed the current session window applies, and it spans the session.
    let closed_all = [
        events[0].clone(),
        events[1].clone(),
        events[2].clone(),
        events[3].clone(),
    ];
    let f = run(&closed_all, 250, Selection::ShiftCurrent);
    let w = &f.windows[f.selected[0]];
    assert_eq!((w.id.as_str(), w.kind.as_str()), ("s1#1", "session"));
}

#[test]
fn a_session_window_splits_every_24_hours() {
    let h = 3600;
    let events = [
        perm(1, 0, "ls"),
        closed(2, 100, "s1:1", "approved", json!(0)),
        perm(3, 25 * h, "ls"),
        closed(4, 25 * h + 40, "s1:3", "approved", json!(0)),
    ];
    let f = run(&events, 26 * h, Selection::ShiftCurrent);
    let sessions: Vec<&WindowReading> = f.windows.iter().filter(|w| w.kind == "session").collect();
    assert_eq!(sessions.len(), 2);
    assert_eq!(
        (sessions[0].id.as_str(), sessions[0].value_seconds),
        ("s1#1", Some(100.0))
    );
    assert_eq!(
        (sessions[1].id.as_str(), sessions[1].value_seconds),
        ("s1#2", Some(40.0))
    );
    assert_eq!(
        f.windows[f.selected[0]].id, "s1#2",
        "the current window is the one containing now"
    );
}

#[test]
fn a_since_window_clips_intervals_and_open_intervals_run_to_now() {
    let events = [
        perm(1, 0, "ls"),
        closed(2, 100, "s1:1", "approved", json!(0)),
        perm(3, 200, "ls"),
    ];
    let f = run(&events, 260, Selection::Since(at(50)));
    let w = &f.windows[f.selected[0]];
    assert_eq!(w.value_seconds, Some(50.0 + 60.0));
    assert_eq!(w.open_intervals, 1);
    assert!(w.id.starts_with("since:"));
}

#[test]
fn readings_carry_the_shift_id_in_their_env_and_unmeasured_is_not_zero() {
    let events = [
        shift(1, 0, "shiftA", "open"),
        perm(2, 10, "ls"),
        closed(3, 110, "s1:2", "approved", json!(0)),
    ];
    let policy = StasisPolicy::builtin();
    let c = classifier();
    let fold = run(&events, 120, Selection::ShiftCurrent);
    let rep = report(&[fold], &policy);
    let out = emit_count(&rep, "dialogue-blocked-seconds@1")
        .unwrap()
        .unwrap();
    assert_eq!(out["value"], 100.0);
    assert_eq!(out["env"]["shift"], "shiftA");
    assert!(emit_count(&rep, "dialogue-blocked-seconds@2").is_err());
    assert!(emit_count(&rep, "ladder-resolution@1").is_err());
    // A selection that reads no window (an unnamed sweep over a stale session) is unmeasured.
    let stale = fold_session(
        &replay_of(&events),
        &FoldOptions::new(at(10 * 86_400), Selection::ShiftCurrent, false, &policy, &c),
    );
    assert!(stale.selected.is_empty());
    let rep = report(&[stale], &policy);
    assert_eq!(
        rep.measures["dialogue-blocked-seconds"]["value"],
        Value::Null
    );
    assert!(emit_count(&rep, "dialogue-blocked-seconds@1")
        .unwrap()
        .is_none());
    // The ceiling is the cumulative union against 1800 s.
    let long = [
        perm(1, 0, "ls"),
        closed(2, 1800, "s1:1", "approved", json!(0)),
    ];
    let f = run(&long, 1900, Selection::All);
    assert!(f.windows[f.selected[0]].ceiling.as_ref().unwrap().breached);
}

#[test]
fn gaps_make_a_reading_a_lower_bound_not_complete() {
    let events = [
        perm(1, 0, "ls"),
        ev(
            2,
            10,
            "observation-gap",
            json!({"gap": {"reason": "hook-error", "from": null, "to": null}}),
        ),
        closed(3, 50, "s1:1", "unknown", Value::Null),
    ];
    let f = run(&events, 60, Selection::All);
    let w = &f.windows[f.selected[0]];
    assert!(!w.complete);
    assert_eq!((w.gaps >= 2, w.unknown_closes), (true, 1));
}

#[test]
fn harness_exceptions_count_per_shift_by_hook_and_exclude_the_baseline() {
    let exc = |seq, t, hook: &str, n, baseline: bool| {
        ev(
            seq,
            t,
            "harness-exception",
            json!({"exception": {"source": "hook_cancelled", "hookName": hook, "count": n, "baseline": baseline}}),
        )
    };
    let events = [
        exc(1, 0, "native-seam-guidance", 40, true),
        shift(2, 10, "sh", "open"),
        exc(3, 20, "native-seam-guidance", 3, false),
        exc(4, 30, "run-projection", 2, false),
    ];
    let f = run(&events, 100, Selection::ShiftCurrent);
    let h = f.windows[f.selected[0]]
        .harness_exceptions
        .as_ref()
        .unwrap();
    assert_eq!(
        (
            h.count,
            h.by_hook["native-seam-guidance"],
            h.baseline_excluded
        ),
        (5, 3, 0)
    );
    // No harness evidence at all is unmeasured, not zero.
    let none = run(&[perm(1, 0, "ls")], 10, Selection::All);
    assert!(none.windows[none.selected[0]].harness_exceptions.is_none());
}

// --- Presence precedence -------------------------------------------------------------------

#[test]
fn an_unanswered_displayed_dialog_at_breach_is_witnessed_absence() {
    let base = |b: Value| [perm(1, 0, "ls"), dialog(2, 6, "s1:1"), b];
    let later = perm(3, 400, "ls");
    let f = run(&base(later), 500, Selection::All);
    assert_eq!(
        f.incidents[1].presence_at_open.disposition,
        Disposition::Automated
    );
    assert_eq!(f.presence.disposition, Disposition::Automated);
    // A witnessed interaction after the Breach returns the session to its envelope disposition.
    let mut witnessed_later = perm(3, 400, "ls");
    witnessed_later["presence"] = witnessed(10.0)["presence"].clone();
    let f = run(&base(witnessed_later), 500, Selection::All);
    assert_eq!(
        f.incidents[1].presence_at_open.disposition,
        Disposition::Collaborative
    );
    // A request that was never displayed is not an observation of absence.
    let f = run(&[perm(1, 0, "ls"), perm(2, 400, "ls")], 500, Selection::All);
    assert_ne!(
        f.incidents[1].presence_at_open.disposition,
        Disposition::Automated
    );
}

#[test]
fn an_away_ruling_and_the_collaborative_window_set_the_passive_disposition() {
    let p = StasisPolicy::builtin();
    let at_age = |age: f64| {
        let mut e = ev(1, 0, "steward-ping", json!({}));
        e["presence"] = witnessed(age)["presence"].clone();
        let parsed = Event::from_value(&e).unwrap();
        effective_presence(&[parsed], &[], at(0), &p, &[]).disposition
    };
    assert_eq!(at_age(599.0), Disposition::Collaborative);
    assert_eq!(at_age(601.0), Disposition::Automated);
    let mut away = ev(1, 0, "steward-ping", json!({}));
    away["presence"] = json!({"disposition": "automated", "basis": {"lastWitnessedAgeSeconds": 1.0, "rulingRef": "ruling-away-1"}});
    let parsed = Event::from_value(&away).unwrap();
    assert_eq!(
        effective_presence(&[parsed], &[], at(0), &p, &[]).disposition,
        Disposition::Automated
    );
    assert_eq!(
        effective_presence(&[], &[], at(0), &p, &[]).disposition,
        Disposition::Unknown
    );
}

// --- Rungs, consultations and budgets -------------------------------------------------------

#[test]
fn consultation_lifecycle_start_completion_and_expiry() {
    let events = [
        perm(1, 0, "git rebase main"),
        consult(2, 10, "s1:1", start(1, 10, 100)),
        consult(3, 11, "s1:1", start(1, 10, 100)), // duplicate claim start
        consult(4, 12, "s1:1", start(1, 10, 100)),
    ];
    let f = run(&events, 50, Selection::All);
    let i = &f.incidents[0];
    assert_eq!(
        (
            i.consultations.len(),
            i.consultations[0].duplicate_starts,
            i.rung
        ),
        (1, 2, 1),
        "duplicate starts fold idempotently"
    );
    assert_eq!(i.budget_used.keeper_calls, 1);
    // Completion with Refer climbs one rung; a repeated completion is idempotent.
    let mut more = events.to_vec();
    more.push(consult(
        5,
        40,
        "s1:1",
        finish(1, "Refer", "notify", Some(100)),
    ));
    more.push(consult(
        6,
        41,
        "s1:1",
        finish(1, "Permit", "park", Some(999)),
    ));
    let i = &run(&more, 50, Selection::All).incidents[0].clone();
    assert_eq!(
        (
            i.rung,
            i.state != IncidentState::Closed,
            i.consultations[0].tokens
        ),
        (2, true, Some(100))
    );
    // An explicit expiry advances the ladder, and a result recorded after it is ignored.
    let mut expired = events.to_vec();
    expired.push(consult(
        5,
        100,
        "s1:1",
        json!({"phase": "expired", "rung": 1, "claimSeq": 2, "finishedAt": clk(100), "usage": usage(None)}),
    ));
    expired.push(consult(6, 101, "s1:1", finish(1, "Permit", "park", None)));
    let i = &run(&expired, 110, Selection::All).incidents[0].clone();
    assert!(i.consultations[0].expired);
    assert_eq!((i.rung, i.closed_by), (2, None));
    // With no expiry event, the deadline passing on the fold's clock is the same fact.
    let i = &run(&events[..2], 200, Selection::All).incidents[0].clone();
    assert_eq!((i.consultations[0].expired, i.rung), (true, 2));
    // In flight before the deadline: no advance.
    let i = &run(&events[..2], 50, Selection::All).incidents[0].clone();
    assert_eq!((i.consultations[0].expired, i.rung), (false, 1));
}

#[test]
fn the_ladder_runs_in_rung_order_to_the_steward() {
    let mut events = vec![perm(1, 0, "git rebase main")];
    for rung in 1..=4_i64 {
        events.push(consult(
            10 * rung as u64,
            10 * rung,
            "s1:1",
            start(rung as u64, 10 * rung, 10 * rung + 5),
        ));
        events.push(consult(
            10 * rung as u64 + 1,
            10 * rung + 2,
            "s1:1",
            finish(rung as u64, "Refer", "notify", Some(1000)),
        ));
    }
    let i = &run(&events, 60, Selection::All).incidents[0].clone();
    assert_eq!((i.rung, i.state), (5, IncidentState::Pending));
    assert_eq!(i.budget_used.keeper_calls, 4);
    assert!(i.budget_exhausted.contains(&"keeper-calls".to_owned()));
    // Pending closes like any state the moment the stall is resolved.
    events.push(closed(100, 90, "s1:1", "approved", json!(0)));
    assert_eq!(
        run(&events, 100, Selection::All).incidents[0].state,
        IncidentState::Closed
    );
}

#[test]
fn builtin_defaults_equal_every_budget_in_the_spec() {
    let p = StasisPolicy::builtin();
    assert_eq!(
        (
            p.budgets.incident_keeper_calls,
            p.budgets.incident_elapsed_s,
            p.budgets.incident_tokens
        ),
        (4, 1800.0, 200_000)
    );
    assert_eq!(
        (p.budgets.shift_keeper_s, p.budgets.shift_tokens),
        (7200.0, 2_000_000)
    );
    assert_eq!(
        (
            p.collaborative_window_s,
            p.stall.limit,
            p.approach_s(),
            p.dwell_s
        ),
        (600.0, 300.0, 180.0, 300)
    );
    assert_eq!(p.shift_ceiling.limit, 1800.0);
    assert_eq!(POLICY_ABSENT, "absent (defaults, shadow)");
}

#[test]
fn each_incident_bound_exhausts_to_the_steward() {
    // Elapsed: 1800 s since opening, nothing in flight.
    let f = run(&[perm(1, 0, "ls")], 1800, Selection::All);
    let i = &f.incidents[0];
    assert_eq!(
        (i.rung, i.state, i.budget_exhausted.clone()),
        (5, IncidentState::Pending, vec!["elapsed".to_owned()])
    );
    // One second short is not exhausted.
    assert_eq!(
        run(&[perm(1, 0, "ls")], 1799, Selection::All).incidents[0].rung,
        0
    );
    // Tokens: 200000 combined reaches the bound; 199999 does not.
    let tokens = |n| {
        let ev = [
            perm(1, 0, "ls"),
            consult(2, 5, "s1:1", finish(1, "Permit", "notify", Some(n))),
        ];
        run(&ev, 20, Selection::All).incidents[0].clone()
    };
    assert_eq!(
        (
            tokens(200_000).rung,
            tokens(200_000).budget_exhausted.clone()
        ),
        (5, vec!["tokens".to_owned()])
    );
    assert_eq!(
        (
            tokens(199_999).rung,
            tokens(199_999).budget_exhausted.is_empty()
        ),
        (1, true)
    );
    // Missing usage is reported, never counted as zero spend.
    let unreported = [
        perm(1, 0, "ls"),
        consult(2, 5, "s1:1", finish(1, "Permit", "notify", None)),
    ];
    assert_eq!(
        run(&unreported, 20, Selection::All).incidents[0]
            .budget_used
            .tokens_unreported,
        1
    );
}

#[test]
fn each_shift_bound_exhausts_open_incidents_to_the_steward() {
    // 7200 keeper seconds in one shift: a second, untouched incident also jumps to rung 5.
    let events = [
        shift(1, 0, "sh", "open"),
        perm(2, 10, "ls"),
        consult(3, 20, "s1:2", start(1, 20, 20_000)),
        consult(4, 7220, "s1:2", finish(1, "Refer", "notify", Some(10))),
        perm(5, 7230, "ls"),
    ];
    let f = run(&events, 7240, Selection::ShiftCurrent);
    let w = &f.windows[f.selected[0]];
    assert_eq!(w.keeper.seconds, Some(7200.0));
    assert_eq!(w.keeper.exhausted, vec!["keeper-seconds".to_owned()]);
    assert_eq!(f.incidents[1].state, IncidentState::Pending);
    // 2M tokens in the shift.
    let events = [
        shift(1, 0, "sh", "open"),
        perm(2, 10, "ls"),
        consult(3, 20, "s1:2", finish(1, "Refer", "notify", Some(199_999))),
        perm(4, 30, "ls"),
        consult(5, 40, "s1:4", finish(1, "Refer", "notify", Some(1_800_001))),
    ];
    let f = run(&events, 60, Selection::ShiftCurrent);
    assert_eq!(
        f.windows[f.selected[0]].keeper.exhausted,
        vec!["tokens".to_owned()]
    );
    // Under both bounds nothing is exhausted.
    let f = run(&events[..3], 60, Selection::ShiftCurrent);
    assert!(f.windows[f.selected[0]].keeper.exhausted.is_empty());
}

#[test]
fn ladder_resolution_names_the_rung_that_resolved_each_incident() {
    let events = [
        perm(1, 0, "cd <path> && ls"),
        consult(2, 5, "s1:1", finish(1, "Permit", "park", None)),
        perm(3, 10, "ls"),
        closed(4, 20, "s1:3", "approved", json!(0)),
        perm(5, 30, "ls"),
        ev(6, 40, "session-ended", json!({})),
    ];
    let f = run(&events, 50, Selection::All);
    let got: Vec<(&str, &str)> = f
        .ladder_resolution
        .iter()
        .map(|r| (r.incident_id.as_str(), r.resolved_by.as_str()))
        .collect();
    assert_eq!(
        got,
        [("s1:3", "human")],
        "unresolved stalls (and shadow parks) are not resolutions"
    );
}

// --- Journal files: segments, retention, locking --------------------------------------------

fn write_lines(path: &Path, events: &[Value]) {
    let body: String = events.iter().map(|e| format!("{e}\n")).collect();
    fs::write(path, body).unwrap();
}

#[test]
fn replay_orders_numbered_segments_before_the_active_file_and_reports_retention() {
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    // events.2 sorts before events.10 numerically; seq 1-2 were retained away.
    write_lines(
        &s.join("events.10.jsonl"),
        &[closed(5, 50, "s1:3", "approved", json!(0))],
    );
    write_lines(
        &s.join("events.2.jsonl"),
        &[
            ev(3, 20, "turn-ended", json!({"question": "no"})),
            perm(4, 30, "ls"),
        ],
    );
    write_lines(
        &s.join("events.jsonl"),
        &[ev(6, 60, "steward-ping", json!({}))],
    );
    let r = journal::replay(dir.path(), "s1").unwrap();
    assert_eq!(
        r.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
    assert_eq!(r.segments, 3);
    assert!(r
        .gaps
        .iter()
        .any(|g| g.reason == "retention" && g.to_seq == Some(2)));
    // An incident that references a dropped seq is a retention gap, not a silent zero.
    let f = fold_session(
        &r,
        &FoldOptions::new(
            at(100),
            Selection::All,
            true,
            &StasisPolicy::builtin(),
            &classifier(),
        ),
    );
    assert!(f.gaps.iter().any(|g| g.reason == "retention"));
    assert!(f
        .gaps
        .iter()
        .any(|g| g.reason == "unknown-incident" && g.incident.as_deref() == Some("s1:3")));
}

#[test]
fn replay_detects_discontinuity_torn_tails_duplicates_and_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    let a = perm(1, 0, "ls");
    write_lines(
        &s.join("events.jsonl"),
        &[
            a.clone(),
            a.clone(),
            closed(4, 50, "s1:1", "approved", json!(0)),
        ],
    );
    let r = journal::replay(dir.path(), "s1").unwrap();
    let reasons: Vec<&str> = r.gaps.iter().map(|g| g.reason.as_str()).collect();
    assert!(
        reasons.contains(&"duplicate-seq") && reasons.contains(&"sequence-discontinuity"),
        "{reasons:?}"
    );
    // A torn final line is missing evidence.
    let mut body = fs::read_to_string(s.join("events.jsonl")).unwrap();
    body.push_str("{\"schemaVersion\":1,\"kin");
    fs::write(s.join("events.jsonl"), &body).unwrap();
    assert!(journal::replay(dir.path(), "s1")
        .unwrap()
        .gaps
        .iter()
        .any(|g| g.reason == "torn-record"));
    // A malformed line in the middle is a parse error (exit 1), as is an unknown shape.
    fs::write(
        s.join("events.jsonl"),
        format!("not json\n{}\n", perm(1, 0, "ls")),
    )
    .unwrap();
    assert!(matches!(
        journal::replay(dir.path(), "s1"),
        Err(StasisError::Parse(_))
    ));
    // No session directory and no root are both "no journal".
    assert!(matches!(
        journal::replay(dir.path(), "nope"),
        Err(StasisError::NoJournal(_))
    ));
    assert!(matches!(
        journal::sessions(&dir.path().join("absent")),
        Err(StasisError::NoJournal(_))
    ));
}

#[test]
fn fold_root_reads_all_sessions_and_reports_no_journal() {
    let dir = tempfile::tempdir().unwrap();
    let policy = StasisPolicy::builtin();
    let c = classifier();
    let o = FoldOptions::new(at(100), Selection::All, false, &policy, &c);
    assert!(matches!(
        fold_root(dir.path(), None, &o),
        Err(StasisError::NoJournal(_))
    ));
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    write_lines(
        &s.join("events.jsonl"),
        &[
            perm(1, 0, "ls"),
            closed(2, 40, "s1:1", "approved", json!(0)),
        ],
    );
    let (report, folds) = fold_root(dir.path(), None, &o).unwrap();
    assert_eq!((folds.len(), report.policy), (1, POLICY_ABSENT));
    assert_eq!(report.measures["dialogue-blocked-seconds"]["value"], 40.0);
    assert!(
        report.measures["dialogue-blocked-seconds"]["byClass"]["authorization"]
            .as_f64()
            .unwrap()
            > 0.0
    );
}

// --- Shadow decide ----------------------------------------------------------------------------

fn decide_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    write_lines(
        &s.join("events.jsonl"),
        &[
            perm(1, 0, "git -C <path> rebase main"),
            dialog(2, 6, "s1:1"),
        ],
    );
    fs::write(
        s.join("state.json"),
        r#"{"schemaVersion":1,"nextSeq":3,"openPrompts":{}}"#,
    )
    .unwrap();
    let root = dir.path().to_path_buf();
    (dir, root)
}

fn request() -> DecideRequest {
    DecideRequest::from_json(&json!({"hook_event_name": "PermissionRequest", "tool_name": "Bash", "shape": "git -C <path> rebase main", "agent_id": "main"})).unwrap()
}

fn clock(secs: i64) -> ClockReading {
    ClockReading {
        wall: at(secs),
        boot_id: "boot-1".into(),
        uptime_seconds: 1000.0 + secs as f64,
    }
}

#[test]
fn shadow_decide_journals_would_have_and_executes_nothing() {
    let (_guard, root) = decide_fixture();
    let out = decide_shadow(
        &root,
        "s1",
        &request(),
        &classifier(),
        &StasisPolicy::builtin(),
        &clock(20),
        5 << 20,
    )
    .unwrap();
    let printed: Value = serde_json::to_value(&out).unwrap();
    assert_eq!(printed["effect"], "none");
    assert_eq!(printed["shadow"], true);
    assert_eq!(printed["policy"], "absent (defaults, shadow)");
    assert_eq!(printed["incidentId"], "s1:1");
    assert_eq!(printed["rung"], 0);
    let keys: Vec<&str> = printed
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for k in [
        "effect",
        "message",
        "incidentId",
        "rung",
        "shadow",
        "policy",
    ] {
        assert!(keys.contains(&k), "decide prints {k}");
    }
    // The would-have is journalled with the next seq and the cache moves in step.
    let r = journal::replay(&root, "s1").unwrap();
    let last = r.events.last().unwrap();
    assert_eq!((last.kind, last.seq), (EventKind::WouldHave, 3));
    let raw: Value = serde_json::from_str(
        fs::read_to_string(root.join("s1/events.jsonl"))
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(raw["wouldHave"]["classification"]["concern"], "risk");
    assert_eq!(raw["wouldHave"]["effect"], "none");
    let state: Value =
        serde_json::from_slice(&fs::read(root.join("s1/state.json")).unwrap()).unwrap();
    assert_eq!(state["nextSeq"], 4);
    assert_eq!(
        state["openPrompts"],
        json!({}),
        "unrelated cache keys survive"
    );
    // Folding afterwards leaves the incident exactly as it was: nothing advanced or closed.
    let f = run_root(&root, 30);
    assert_eq!(
        (f.would_have, f.incidents.len(), f.incidents[0].state),
        (1, 1, IncidentState::Open)
    );
}

fn run_root(root: &Path, now: i64) -> SessionFold {
    let policy = StasisPolicy::builtin();
    fold_session(
        &journal::replay(root, "s1").unwrap(),
        &FoldOptions::new(at(now), Selection::All, true, &policy, &classifier()),
    )
}

#[test]
fn shadow_decide_rotates_segments_and_reports_a_missing_journal() {
    let (_guard, root) = decide_fixture();
    // A tiny segment limit rotates the active file exactly as the writer does.
    decide_shadow(
        &root,
        "s1",
        &request(),
        &classifier(),
        &StasisPolicy::builtin(),
        &clock(20),
        10,
    )
    .unwrap();
    assert_eq!(journal::segments(&root.join("s1")).len(), 1);
    let state: Value =
        serde_json::from_slice(&fs::read(root.join("s1/state.json")).unwrap()).unwrap();
    assert_eq!(
        (state["nextSeq"].clone(), state["nextSegment"].clone()),
        (json!(4), json!(2))
    );
    assert_eq!(
        journal::replay(&root, "s1").unwrap().events.len(),
        3,
        "replay reads across the rotation"
    );
    // No journal: unmeasured with exit 0 (fail open); decide never creates one.
    let r = decide_shadow(
        &root,
        "ghost",
        &request(),
        &classifier(),
        &StasisPolicy::builtin(),
        &clock(20),
        5 << 20,
    );
    assert!(r.unwrap().unmeasured);
    assert!(!root.join("ghost").exists());
}

#[test]
fn the_regulator_has_no_allow_shaped_effect() {
    for (effect, name) in [
        (Effect::None, "none"),
        (Effect::Deny, "deny"),
        (Effect::Block, "block"),
        (Effect::Context, "context"),
    ] {
        assert_eq!(serde_json::to_value(effect).unwrap(), name);
    }
    assert!(
        DecideRequest::from_json(&json!({"kind": "permission-requested", "tool": "Bash"})).is_ok()
    );
    assert!(DecideRequest::from_json(&json!({"tool": "Bash"})).is_err());
}

#[test]
fn status_is_read_only_and_never_reports_the_coordinator_green() {
    let events = [
        perm(1, 0, "ls"),
        perm(2, 10, "cd <path> && ls"),
        consult(3, 20, "s1:2", finish(1, "Permit", "park", None)),
    ];
    let f = run(&events, 100, Selection::All);
    let s = status(&f, at(100));
    assert_eq!(
        s["openIncidents"].as_array().unwrap().len(),
        2,
        "shadow parks do not close"
    );
    assert_eq!(s["pendingParks"], json!(["s1:2"]));
    assert_eq!(s["coordinator"]["state"], "unknown");
    assert_eq!(s["policy"], POLICY_ABSENT);
}

// --- Review findings: lifecycle contract, unmeasured evidence, scoping --------------------------

fn at_moment(t: i64) -> Moment {
    Moment {
        wall: at(t),
        boot_id: "boot-1".into(),
        uptime_seconds: 1000.0 + t as f64,
    }
}

#[test]
fn the_coordinator_lifecycle_replays_with_clock_objects_claim_seq_and_expired() {
    let events = [
        perm(1, 0, "git rebase main"),
        // Out of rung order on purpose: rung 3's claim is written before rung 2's.
        consult(2, 10, "s1:1", start(3, 10, 70)),
        consult(3, 12, "s1:1", start(2, 12, 40)),
        consult(
            4,
            30,
            "s1:1",
            json!({"phase": "complete", "rung": 2, "claimSeq": 3, "decision": "Refer", "pivot": null,
                   "reason": "x", "finishedAt": clk(30), "usage": {"elapsedSeconds": 18.0, "tokens": 500}}),
        ),
        consult(
            5,
            70,
            "s1:1",
            json!({"phase": "expired", "rung": 3, "claimSeq": 2, "finishedAt": clk(70), "usage": usage(None)}),
        ),
    ];
    let f = run(&events, 80, Selection::All);
    let i = &f.incidents[0];
    let by_rung = |r: u8| i.consultations.iter().find(|c| c.rung == r).unwrap();
    assert_eq!(
        (by_rung(2).claim_seq, by_rung(3).claim_seq),
        (Some(3), Some(2))
    );
    assert_eq!(
        (by_rung(2).tokens, by_rung(2).reported_elapsed_s),
        (Some(500), Some(18.0))
    );
    assert!(by_rung(3).expired && !by_rung(2).expired);
    assert_eq!(by_rung(3).claimed_by.as_deref(), Some("4242@boot-1"));
    assert_eq!(
        i.rung, 4,
        "the highest rung reached plus its Refer/expiry, however the records were ordered"
    );
    // The folded decision serializes as the schema's word, not a tagged object.
    let json = serde_json::to_value(i).unwrap();
    let c2 = json["consultations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["rung"] == 2)
        .unwrap();
    assert_eq!(c2["decision"], "Refer");
}

#[test]
fn missing_start_or_usage_is_unmeasured_not_zero() {
    // A completion with no start record: duration unmeasured, tokens reported as unreported.
    let orphan = [
        perm(1, 0, "ls"),
        consult(2, 20, "s1:1", finish(1, "Refer", "notify", None)),
    ];
    let f = run(&orphan, 30, Selection::All);
    let b = &f.incidents[0].budget_used;
    assert_eq!(
        (
            b.keeper_seconds,
            b.keeper_seconds_unmeasured,
            b.tokens,
            b.tokens_unreported
        ),
        (None, 1, None, 1)
    );
    let k = &f.windows[f.selected[0]].keeper;
    assert_eq!((k.seconds, k.tokens), (None, None));
    assert_eq!((k.seconds_unmeasured, k.tokens_unreported), (1, 1));
    // The same record with measurements is measured.
    let known = [
        perm(1, 0, "ls"),
        consult(2, 10, "s1:1", start(1, 10, 100)),
        consult(3, 20, "s1:1", finish(1, "Refer", "notify", Some(70))),
    ];
    let f = run(&known, 30, Selection::All);
    let b = &f.incidents[0].budget_used;
    assert_eq!((b.keeper_seconds, b.tokens), (Some(10.0), Some(70)));
    // No consultations at all spent nothing: that is a measured zero.
    let none = run(&[perm(1, 0, "ls")], 30, Selection::All);
    assert_eq!(
        (
            none.incidents[0].budget_used.keeper_seconds,
            none.incidents[0].budget_used.tokens
        ),
        (Some(0.0), Some(0))
    );
}

#[test]
fn shift_cost_follows_time_overlap_and_the_shift_filters_incidents() {
    // The consultation runs t=50..250 across a shift edge at t=100.
    let events = [
        shift(1, 0, "A", "open"),
        perm(2, 10, "ls"),
        consult(3, 50, "s1:2", start(1, 50, 5000)),
        shift(4, 100, "A", "close"),
        shift(5, 100, "B", "open"),
        consult(6, 250, "s1:2", finish(1, "Refer", "notify", Some(10))),
        perm(7, 260, "ls"),
        closed(8, 270, "s1:7", "approved", json!(0)),
    ];
    let f = run(&events, 300, Selection::ShiftCurrent);
    let window = |id: &str| {
        f.windows
            .iter()
            .find(|w| w.id == id && w.kind == "marker")
            .unwrap()
    };
    let (a, b) = (window("A"), window("B"));
    assert_eq!(
        (a.keeper.seconds_known, b.keeper.seconds_known),
        (50.0, 150.0)
    );
    assert_eq!(a.keeper.seconds_known + b.keeper.seconds_known, 200.0);
    // Tokens land in the window where the consultation finished.
    assert_eq!((a.keeper.tokens_known, b.keeper.tokens_known), (0, 10));
    // `--shift-current` (B) reports only incidents overlapping B: s1:2 is still open in B and
    // s1:7 opened there, and an incident that closed before B opened would be filtered out.
    let policy = StasisPolicy::builtin();
    let rep = report(&[f], &policy);
    assert_eq!(rep.incidents.len(), 2);
    let earlier = [
        perm(1, 0, "ls"),
        closed(2, 20, "s1:1", "approved", json!(0)),
        shift(3, 100, "B", "open"),
        perm(4, 110, "ls"),
    ];
    let rep = report(&[run(&earlier, 120, Selection::ShiftCurrent)], &policy);
    assert_eq!(
        rep.incidents
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["s1:4"]
    );
}

#[test]
fn exhaustion_reaches_the_steward_even_with_a_consultation_in_flight() {
    let flight = [
        perm(1, 0, "ls"),
        consult(2, 1700, "s1:1", start(1, 1700, 5000)),
    ];
    let i = &run(&flight, 1800, Selection::All).incidents[0].clone();
    assert_eq!((i.rung, i.state), (5, IncidentState::Pending));
    assert!(
        i.consultations[0].in_flight(),
        "the running consultation is not cut off"
    );
}

#[test]
fn a_parked_incident_keeps_spending_its_elapsed_budget() {
    let policy = StasisPolicy::builtin();
    let f = run(&[perm(1, 0, "ls")], 100, Selection::All);
    let mut inc = f.incidents[0].clone();
    inc.closed_by = Some(ClosedBy::Parked);
    inc.closed_at = Some(at(50));
    inc.closed_moment = Some(at_moment(50));
    finalize_incident(
        &mut inc,
        &Moment {
            wall: at(2000),
            boot_id: "boot-1".into(),
            uptime_seconds: 3000.0,
        },
        &policy,
        false,
    );
    assert_eq!(
        inc.budget_used.elapsed_s, 2000.0,
        "elapsed runs to now, not to the park"
    );
    assert_eq!((inc.rung, inc.state), (5, IncidentState::Pending));
    // An ordinary close freezes it.
    inc.closed_by = Some(ClosedBy::Approved);
    inc.rung = 0;
    finalize_incident(
        &mut inc,
        &Moment {
            wall: at(2000),
            boot_id: "boot-1".into(),
            uptime_seconds: 3000.0,
        },
        &policy,
        false,
    );
    assert_eq!(
        (inc.budget_used.elapsed_s, inc.state),
        (50.0, IncidentState::Closed)
    );
}

#[test]
fn absence_holds_after_the_dialog_closes_until_a_witness() {
    let p = StasisPolicy::builtin();
    let c = classifier();
    let events = [
        perm(1, 0, "ls"),
        dialog(2, 6, "s1:1"),
        closed(3, 400, "s1:1", "denied-inferred", Value::Null),
    ];
    let fold = run(&events, 500, Selection::All);
    assert_eq!(
        fold.presence.disposition,
        Disposition::Automated,
        "closing the dialog is not a witness"
    );
    // A later event with a witness after the Breach restores the passive reading.
    let mut witnessed_later = ev(4, 450, "steward-ping", json!({}));
    witnessed_later["presence"] = witnessed(5.0)["presence"].clone();
    let events = [
        events[0].clone(),
        events[1].clone(),
        events[2].clone(),
        witnessed_later,
    ];
    let fold = fold_session(
        &replay_of(&events),
        &FoldOptions::new(at(460), Selection::All, true, &p, &c),
    );
    assert_eq!(fold.presence.disposition, Disposition::Collaborative);
}

#[test]
fn an_expired_away_ruling_removes_its_override_and_nothing_else() {
    let p = StasisPolicy::builtin();
    let mut e = ev(1, 0, "steward-ping", json!({}));
    e["presence"] = json!({"disposition": "automated", "basis": {"lastWitnessedAgeSeconds": 5.0, "rulingRef": "away-1"}});
    let events = [Event::from_value(&e).unwrap()];
    let live = [AwayRuling {
        id: "away-1".into(),
        expires_at: Some(at(1000)),
    }];
    assert_eq!(
        effective_presence(&events, &[], at(100), &p, &live).disposition,
        Disposition::Automated
    );
    // Expired: the passive bound decides (a witness 5 s old at t=0 is 1005 s old at t=1000).
    let after = effective_presence(&events, &[], at(1000), &p, &live);
    assert_eq!(
        (after.disposition, after.ruling_ref),
        (Disposition::Automated, None),
        "witness is now stale, so passive T_c applies"
    );
    let fresh = effective_presence(
        &events,
        &[],
        at(300),
        &p,
        &[AwayRuling {
            id: "away-1".into(),
            expires_at: Some(at(200)),
        }],
    );
    assert_eq!(
        (fresh.disposition, fresh.ruling_ref),
        (Disposition::Collaborative, None),
        "expiry invents no absence"
    );
}

#[test]
fn questions_resolve_only_for_the_asking_agent_with_evidence() {
    let mut other_stop = ev(2, 100, "turn-ended", json!({"question": "no"}));
    other_stop["agentId"] = json!("sub-1");
    let events = [
        ev(1, 0, "turn-ended", json!({"question": "yes"})),
        other_stop,
    ];
    let f = run(&events, 200, Selection::All);
    assert_eq!(
        f.incidents[0].closed_by, None,
        "another agent's Stop does not withdraw main's question"
    );
    // A witness seen on another agent's event does not answer it either.
    let mut other_witnessed = ev(2, 100, "turn-ended", json!({"question": "no"}));
    other_witnessed["agentId"] = json!("sub-1");
    other_witnessed["presence"] = witnessed(10.0)["presence"].clone();
    let f = run(&[events[0].clone(), other_witnessed], 200, Selection::All);
    assert_eq!(f.incidents[0].closed_by, None);
}

#[test]
fn a_missing_duration_is_an_unmeasured_end_and_monotonic_time_is_preferred() {
    let events = [
        perm(1, 0, "ls"),
        closed(2, 100, "s1:1", "approved", Value::Null),
    ];
    let f = run(&events, 120, Selection::All);
    let w = &f.windows[f.selected[0]];
    assert_eq!(
        (f.incidents[0].end_unmeasured, w.unmeasured_ends, w.complete),
        (true, 1, false)
    );
    // Same boot: the monotonic reading wins over a wall clock that jumped (an NTP step).
    let mut a = perm(1, 0, "ls");
    let mut b = closed(2, 100, "s1:1", "denied-inferred", Value::Null);
    a["clock"]["uptimeSeconds"] = json!(1000.0);
    b["clock"]["uptimeSeconds"] = json!(1040.0);
    let f = run(&[a, b], 120, Selection::All);
    assert_eq!(f.incidents[0].budget_used.elapsed_s, 40.0);
    // Different boots: wall time.
    let mut a = perm(1, 0, "ls");
    let mut b = closed(2, 100, "s1:1", "denied-inferred", Value::Null);
    a["clock"]["bootId"] = json!("boot-0");
    b["clock"]["uptimeSeconds"] = json!(1040.0);
    let f = run(&[a, b], 120, Selection::All);
    assert_eq!(f.incidents[0].budget_used.elapsed_s, 100.0);
}

#[test]
fn gaps_alone_are_unmeasured_and_untimed_gaps_make_a_reading_incomplete() {
    let gap = |seq, t| {
        ev(
            seq,
            t,
            "observation-gap",
            json!({"gap": {"reason": "hook-error", "from": null, "to": null}}),
        )
    };
    let f = run(&[gap(1, 0), gap(2, 10)], 20, Selection::All);
    let w = &f.windows[f.selected[0]];
    assert_eq!(
        (w.value_seconds, w.ceiling.is_none()),
        (None, true),
        "gaps are not evidence of a zero"
    );
    assert!(emit_count(
        &report(&[f], &StasisPolicy::builtin()),
        "dialogue-blocked-seconds@1"
    )
    .unwrap()
    .is_none());
    // Retention: the value is measured from what survives, but the reading is incomplete.
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    write_lines(
        &s.join("events.jsonl"),
        &[
            perm(5, 0, "ls"),
            closed(6, 40, "s1:5", "approved", json!(0)),
        ],
    );
    let r = journal::replay(dir.path(), "s1").unwrap();
    let p = StasisPolicy::builtin();
    let c = classifier();
    let f = fold_session(&r, &FoldOptions::new(at(60), Selection::All, true, &p, &c));
    let w = &f.windows[f.selected[0]];
    assert_eq!((w.value_seconds, w.complete), (Some(40.0), false));
    assert!(w.gaps >= 1);
}

#[test]
fn replay_follows_segment_order_and_names_backwards_and_duplicate_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let s = dir.path().join("s1");
    fs::create_dir_all(&s).unwrap();
    // The newer segment holds LOWER sequence numbers: segment order is the journal's order.
    write_lines(
        &s.join("events.1.jsonl"),
        &[perm(1, 0, "ls"), perm(2, 10, "ls")],
    );
    write_lines(
        &s.join("events.2.jsonl"),
        &[perm(2, 10, "ls"), ev(1, 20, "steward-ping", json!({}))],
    );
    write_lines(
        &s.join("events.jsonl"),
        &[ev(9, 30, "turn-ended", json!({"question": "no"}))],
    );
    let r = journal::replay(dir.path(), "s1").unwrap();
    assert_eq!(
        r.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [1, 2, 9],
        "file order, repeated seq dropped"
    );
    let reasons: Vec<&str> = r.gaps.iter().map(|g| g.reason.as_str()).collect();
    assert!(
        reasons.contains(&"duplicate-seq") && reasons.contains(&"sequence-discontinuity"),
        "{reasons:?}"
    );
    let back = tempfile::tempdir().unwrap();
    let b = back.path().join("s1");
    fs::create_dir_all(&b).unwrap();
    write_lines(
        &b.join("events.jsonl"),
        &[perm(1, 0, "ls"), perm(5, 10, "ls"), perm(3, 20, "ls")],
    );
    let r = journal::replay(back.path(), "s1").unwrap();
    assert_eq!(
        r.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [1, 5, 3]
    );
    assert!(r.gaps.iter().any(|g| g.reason == "backwards-seq"));
}

#[test]
fn appending_after_a_torn_tail_starts_a_new_line_and_replay_names_the_torn_record() {
    let (_guard, root) = decide_fixture();
    let active = root.join("s1/events.jsonl");
    let mut body = fs::read_to_string(&active).unwrap();
    body.push_str("{\"schemaVersion\":1,\"kind\":\"steward-pi");
    fs::write(&active, &body).unwrap();
    decide_shadow(
        &root,
        "s1",
        &request(),
        &classifier(),
        &StasisPolicy::builtin(),
        &clock(20),
        5 << 20,
    )
    .unwrap();
    let text = fs::read_to_string(&active).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.last().unwrap().contains("\"kind\":\"would-have\""),
        "the new record is on its own line"
    );
    assert!(
        lines[lines.len() - 2].ends_with("steward-pi"),
        "the torn record was terminated, not concatenated"
    );
    let r = journal::replay(&root, "s1").unwrap();
    assert!(r.gaps.iter().any(|g| g.reason == "torn-record"));
    assert_eq!(r.events.last().unwrap().kind, EventKind::WouldHave);
}

#[test]
fn would_have_carries_a_deterministic_key_and_repeats_are_not_rewritten() {
    let (_guard, root) = decide_fixture();
    let go = |t| {
        decide_shadow(
            &root,
            "s1",
            &request(),
            &classifier(),
            &StasisPolicy::builtin(),
            &clock(t),
            5 << 20,
        )
        .unwrap()
    };
    go(20);
    go(21);
    let raw: Value = serde_json::from_str(
        fs::read_to_string(root.join("s1/events.jsonl"))
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(raw["wouldHave"]["key"], "none:s1:1:0");
    let r = journal::replay(&root, "s1").unwrap();
    assert_eq!(
        r.events
            .iter()
            .filter(|e| e.kind == EventKind::WouldHave)
            .count(),
        1,
        "the same key is recorded once"
    );
}

#[test]
fn shadow_decide_without_a_journal_exits_clean_with_an_unmeasured_marker() {
    let dir = tempfile::tempdir().unwrap();
    let out = decide_shadow(
        dir.path(),
        "ghost",
        &request(),
        &classifier(),
        &StasisPolicy::builtin(),
        &clock(1),
        5 << 20,
    )
    .unwrap();
    let v = serde_json::to_value(&out).unwrap();
    assert_eq!(
        (
            v["effect"].clone(),
            v["shadow"].clone(),
            v["unmeasured"].clone()
        ),
        (json!("none"), json!(true), json!(true))
    );
    assert!(!dir.path().join("ghost").exists());
}
