//! Shared READ helpers over the flow sidecar — the readers every leg must agree with.
//!
//! Two questions live here, and both are here rather than in a caller because a second answer
//! to either is a defect rather than a duplication.
//!
//! **"What is the latest event on this commitment?"** — lifted VERBATIM from `fulfill`, whose
//! module doc explains at length why the rule is `occurred_at` first and sidecar append order
//! only as the tie-break: `saga-status.py` derives its `regressed` state from the same
//! latest-BY-TIME question, and append order alone diverges under replay/backfill. `fulfill`
//! now calls this one. A reader that disagreed with `fulfill` about "latest" would report a
//! discharged commitment as open, or the reverse, with nothing to distinguish the two.
//!
//! **"What notes stand on this atom?"** — `walk` filters lineage to `Produce`, which is a
//! stability promise its JSON consumers depend on, so notes (which are `Cite` events) are
//! invisible to it BY CONSTRUCTION. That filter is NOT widened. This reader goes to the store
//! directly instead, selecting on the note unit, and leaves `walk`'s contract exactly where it
//! is.
//!
//! Slot parsing carries an unrecognised slot through in [`NoteView::extra`] rather than
//! dropping it: the slot vocabulary is additive by design, and a render that silently discarded
//! a slot it did not know would make the next addition invisible precisely to the readers that
//! need to see it.

use std::cmp::Ordering;

use chrono::DateTime;
use cid::Cid;
use elohim_epr_rea::{FlowRecord, Magnitude, ReaVerb};
use serde::Serialize;

use super::note::{
    NOTE_UNIT, REASON_SLOT_PREFIX, STEWARD_SLOT_PREFIX, SWITCHED_TO_SLOT_PREFIX,
    VERDICT_SLOT_PREFIX,
};

/// A comparable key for an `occurred_at` (RFC3339) string: parses when possible, falls back
/// to raw-string comparison on failure — the exact fallback `saga-status.py`'s `_sort_key`
/// uses (still monotonic for same-precision UTC `Z` timestamps, per its own comment). Cross-
/// variant comparisons (one side parsed, the other raw because it failed to parse) fall back
/// to comparing the two ORIGINAL strings — never panics, never picks an arbitrary ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OccurredAtKey {
    Parsed(DateTime<chrono::FixedOffset>),
    Raw(String),
}

impl OccurredAtKey {
    pub fn parse(occurred_at: &str) -> Self {
        match DateTime::parse_from_rfc3339(occurred_at) {
            Ok(dt) => OccurredAtKey::Parsed(dt),
            Err(_) => OccurredAtKey::Raw(occurred_at.to_string()),
        }
    }

    fn raw(&self) -> String {
        match self {
            OccurredAtKey::Parsed(dt) => dt.to_rfc3339(),
            OccurredAtKey::Raw(s) => s.clone(),
        }
    }
}

impl PartialOrd for OccurredAtKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OccurredAtKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (OccurredAtKey::Parsed(a), OccurredAtKey::Parsed(b)) => a.cmp(b),
            (OccurredAtKey::Raw(a), OccurredAtKey::Raw(b)) => a.cmp(b),
            // Mixed parse success — one side failed to parse. Fall back to comparing the
            // original strings rather than guessing an ordering across variants.
            (a, b) => a.raw().cmp(&b.raw()),
        }
    }
}

/// Is `a` (an `occurred_at`/`generated_at` RFC3339 string) STRICTLY newer than `b`, by the
/// same comparison rule as [`OccurredAtKey`]?
pub fn is_strictly_newer(a: &str, b: &str) -> bool {
    OccurredAtKey::parse(a) > OccurredAtKey::parse(b)
}

/// The `(action, occurred_at)` of the LATEST event associated with a commitment, ordered by
/// `occurred_at` timestamp and tie-broken by sidecar append order — mirroring
/// `saga-status.py`'s `index_flow_state`/`_sort_key` EXACTLY (same two-pass association scan,
/// same tagged-list-then-stable-sort shape), so no two readers can disagree on "latest".
/// Append order ALONE diverges under replay/backfill: a delayed report can append after a
/// chronologically newer event and would then read as "latest" by position even though it is
/// not by time.
///
/// An event is "associated" the same way `saga-status.py` associates one: a `Produce` naming
/// `commit_cid` directly in its `fulfills`, or a `Dismiss` whose `resource` matches the
/// resource FIRST learned (in append order) from such a `Produce` (Dismiss events carry an
/// empty `fulfills`). Returns `None` if the commitment has no associated events at all.
pub fn commitment_latest_event(
    records: &[(Cid, FlowRecord)],
    commit_cid: &Cid,
) -> Option<(ReaVerb, String)> {
    let mut resource: Option<Cid> = None;
    let mut produce_ts: Vec<String> = Vec::new();
    for (_, record) in records {
        let FlowRecord::Event(e) = record else {
            continue;
        };
        if e.action == ReaVerb::Produce && e.fulfills.contains(commit_cid) {
            if resource.is_none() {
                resource = Some(e.resource);
            }
            produce_ts.push(e.occurred_at.clone());
        }
    }

    let mut dismiss_ts: Vec<String> = Vec::new();
    if let Some(resource) = resource {
        for (_, record) in records {
            let FlowRecord::Event(e) = record else {
                continue;
            };
            if e.action == ReaVerb::Dismiss && e.resource == resource {
                dismiss_ts.push(e.occurred_at.clone());
            }
        }
    }

    if produce_ts.is_empty() {
        return None;
    }

    // Tagged in the same order saga-status builds it: all Produce timestamps (in the order
    // encountered), THEN all Dismiss timestamps (in the order encountered) — a stable sort by
    // time then preserves that relative order among exact ties.
    let mut tagged: Vec<(OccurredAtKey, ReaVerb, String)> = produce_ts
        .into_iter()
        .map(|t| (OccurredAtKey::parse(&t), ReaVerb::Produce, t))
        .chain(
            dismiss_ts
                .into_iter()
                .map(|t| (OccurredAtKey::parse(&t), ReaVerb::Dismiss, t)),
        )
        .collect();
    tagged.sort_by(|a, b| a.0.cmp(&b.0));

    tagged
        .pop()
        .map(|(_, verb, occurred_at)| (verb, occurred_at))
}

/// One note, with its positional slots parsed back out.
#[derive(Debug, Clone, Serialize)]
pub struct NoteView {
    /// The note event's own atom address.
    pub cid: String,
    /// The slot-0 tag (`run:ruling`, `run:correction`, …).
    pub kind: String,
    /// The slot-1 subject — the target's repo-relative path or CID, as the note recorded it.
    pub subject: String,
    /// The event's provider: the claimed identity when one was named, else the commit author.
    pub actor: String,
    pub reason: Option<String>,
    pub switched_to: Option<String>,
    pub verdict: Option<String>,
    pub steward: Option<String>,
    pub occurred_at: String,
    /// Slots this parser did not recognise, carried through verbatim. The vocabulary is
    /// additive, so a future slot must surface as unknown rather than vanish from the render.
    pub extra: Vec<String>,
}

/// The notes standing on one atom: `Cite` events whose `resource` is `resource_cid` and whose
/// quantity unit is the note unit, NEWEST FIRST, truncated to `limit`.
///
/// The unit test is the selector, not the verb alone: `Cite` is a general reference verb, and
/// a future citing leg with a different unit must not appear in a note render.
pub fn notes_on(records: &[(Cid, FlowRecord)], resource_cid: &Cid, limit: usize) -> Vec<NoteView> {
    let mut matched: Vec<(OccurredAtKey, usize, NoteView)> = Vec::new();
    for (index, (cid, record)) in records.iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if event.action != ReaVerb::Cite || &event.resource != resource_cid {
            continue;
        }
        let Magnitude::Count { unit, .. } = &event.quantity else {
            continue;
        };
        if unit != NOTE_UNIT {
            continue;
        }
        matched.push((
            OccurredAtKey::parse(&event.occurred_at),
            index,
            note_view(cid, event),
        ));
    }
    // Ascending by (time, append order), then reversed: newest first with the LAST-appended
    // note of an exact timestamp tie leading, which is the order a session authored them in.
    matched.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    matched
        .into_iter()
        .rev()
        .take(limit)
        .map(|(_, _, view)| view)
        .collect()
}

fn note_view(cid: &Cid, event: &elohim_epr_rea::FlowEvent) -> NoteView {
    let mut view = NoteView {
        cid: cid.to_string(),
        kind: event.classified_as.first().cloned().unwrap_or_default(),
        subject: event.classified_as.get(1).cloned().unwrap_or_default(),
        actor: event.provider.0.clone(),
        reason: None,
        switched_to: None,
        verdict: None,
        steward: None,
        occurred_at: event.occurred_at.clone(),
        extra: Vec::new(),
    };
    for slot in event.classified_as.iter().skip(2) {
        if let Some(rest) = slot.strip_prefix(REASON_SLOT_PREFIX) {
            view.reason = Some(rest.to_string());
        } else if let Some(rest) = slot.strip_prefix(SWITCHED_TO_SLOT_PREFIX) {
            view.switched_to = Some(rest.to_string());
        } else if let Some(rest) = slot.strip_prefix(VERDICT_SLOT_PREFIX) {
            view.verdict = Some(rest.to_string());
        } else if let Some(rest) = slot.strip_prefix(STEWARD_SLOT_PREFIX) {
            view.steward = Some(rest.to_string());
        } else {
            view.extra.push(slot.clone());
        }
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::{atom_cid, AgentRef, FlowEvent};

    fn cid_of(seed: &str) -> Cid {
        atom_cid(&seed.to_string()).expect("cid")
    }

    fn event(
        action: ReaVerb,
        resource: Cid,
        unit: &str,
        fulfills: Vec<Cid>,
        classified_as: &[&str],
        occurred_at: &str,
    ) -> FlowRecord {
        FlowRecord::Event(FlowEvent {
            action,
            provider: AgentRef("agent:reviewer@opus-5".to_string()),
            receiver: AgentRef("repo:ethosengine/elohim".to_string()),
            resource,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: unit.to_string(),
            },
            process: None,
            in_scope_of: cid_of("scope"),
            fulfills,
            satisfies: Vec::new(),
            classified_as: classified_as.iter().map(|s| s.to_string()).collect(),
            occurred_at: occurred_at.to_string(),
        })
    }

    /// The rule `fulfill` has always held, now held in one place: occurred-at FIRST, append
    /// order only as the tie-break. The Dismiss is appended BEFORE the Produce but is
    /// chronologically newer, so append order alone would give the wrong answer.
    #[test]
    fn latest_is_by_occurred_at_with_append_order_only_as_the_tie_break() {
        let commitment = cid_of("commitment");
        let resource = cid_of("resource");
        let records = vec![
            (
                cid_of("r0"),
                event(
                    ReaVerb::Dismiss,
                    resource,
                    "red-run",
                    Vec::new(),
                    &[],
                    "2026-09-04T00:00:00Z",
                ),
            ),
            (
                cid_of("r1"),
                event(
                    ReaVerb::Produce,
                    resource,
                    "green-run",
                    vec![commitment],
                    &[],
                    "2026-09-01T00:00:00Z",
                ),
            ),
        ];
        assert_eq!(
            commitment_latest_event(&records, &commitment),
            Some((ReaVerb::Dismiss, "2026-09-04T00:00:00Z".to_string())),
            "the chronologically newer Dismiss is latest even though it was appended first"
        );
        assert!(is_strictly_newer(
            "2026-09-04T00:00:00Z",
            "2026-09-01T00:00:00Z"
        ));
        assert!(!is_strictly_newer(
            "2026-09-01T00:00:00Z",
            "2026-09-01T00:00:00Z"
        ));
        assert_eq!(commitment_latest_event(&records, &cid_of("other")), None);
    }

    #[test]
    fn notes_on_selects_by_unit_orders_newest_first_and_truncates() {
        let atom = cid_of("atom");
        let records = vec![
            (
                cid_of("n0"),
                event(
                    ReaVerb::Cite,
                    atom,
                    NOTE_UNIT,
                    Vec::new(),
                    &["run:observation", "plan.md", "reason:first"],
                    "2026-09-01T00:00:00Z",
                ),
            ),
            (
                cid_of("n1"),
                event(
                    ReaVerb::Cite,
                    atom,
                    NOTE_UNIT,
                    Vec::new(),
                    &["run:ruling", "plan.md", "reason:second"],
                    "2026-09-02T00:00:00Z",
                ),
            ),
            (
                cid_of("n2"),
                event(
                    ReaVerb::Cite,
                    atom,
                    NOTE_UNIT,
                    Vec::new(),
                    &["run:verdict", "plan.md", "reason:third"],
                    "2026-09-03T00:00:00Z",
                ),
            ),
            // A Cite in a DIFFERENT unit is not a note, and a note on another atom is not ours.
            (
                cid_of("n3"),
                event(
                    ReaVerb::Cite,
                    atom,
                    "some-other-unit",
                    Vec::new(),
                    &["run:observation", "plan.md", "reason:not a note"],
                    "2026-09-09T00:00:00Z",
                ),
            ),
            (
                cid_of("n4"),
                event(
                    ReaVerb::Cite,
                    cid_of("elsewhere"),
                    NOTE_UNIT,
                    Vec::new(),
                    &["run:observation", "other.md", "reason:elsewhere"],
                    "2026-09-09T00:00:00Z",
                ),
            ),
        ];

        let all = notes_on(&records, &atom, 10);
        assert_eq!(all.len(), 3, "unit and resource both select");
        assert_eq!(all[0].reason.as_deref(), Some("third"));
        assert_eq!(all[1].reason.as_deref(), Some("second"));
        assert_eq!(all[2].reason.as_deref(), Some("first"));

        let capped = notes_on(&records, &atom, 2);
        assert_eq!(capped.len(), 2, "the limit truncates the newest window");
        assert_eq!(capped[0].kind, "run:verdict");
    }

    #[test]
    fn every_slot_parses_back_out_and_an_unknown_one_is_carried_never_dropped() {
        let atom = cid_of("atom");
        let records = vec![(
            cid_of("n0"),
            event(
                ReaVerb::Cite,
                atom,
                NOTE_UNIT,
                Vec::new(),
                &[
                    "run:verdict",
                    "genesis/plan.md",
                    "reason:the gate line is present",
                    "switched-to:a different approach",
                    "verdict:approved",
                    "steward:author@example.test",
                    "future-slot:not yet invented",
                ],
                "2026-09-05T00:00:00Z",
            ),
        )];
        let view = &notes_on(&records, &atom, 5)[0];
        assert_eq!(view.kind, "run:verdict");
        assert_eq!(view.subject, "genesis/plan.md");
        assert_eq!(view.actor, "agent:reviewer@opus-5");
        assert_eq!(view.reason.as_deref(), Some("the gate line is present"));
        assert_eq!(view.switched_to.as_deref(), Some("a different approach"));
        assert_eq!(view.verdict.as_deref(), Some("approved"));
        assert_eq!(view.steward.as_deref(), Some("author@example.test"));
        assert_eq!(
            view.extra,
            vec!["future-slot:not yet invented".to_string()],
            "an unrecognised slot is carried through, never dropped"
        );
    }
}
