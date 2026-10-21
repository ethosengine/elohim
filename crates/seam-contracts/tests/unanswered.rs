//! Contract tests for ruling 7's shared type and status channel (interfaces spec §2.0).
//!
//! Three clauses: every seam's reason type used in an `Unanswered` passes `check_reason_labels`; a
//! cursor whose inputs did not change returns `None`; a cursor reaches a terminal state only through
//! `Answered` or `Withdrawn`, and says nothing after it. The cursor under test is the crate's
//! `Changes` over a seam-shaped `StatusSource`, and the same script is run through
//! `harness::status::check_status_cursor`, the check a seam with a hand-written cursor runs.

use seam_contracts::{
    check_reason_labels, AnswerReason, AnswerState, Changes, ComeBack, Meanwhile, PendingStatus,
    ReasonLabel, StatusCursor, StatusSource, Unanswered,
};

/// A seam-local reason vocabulary of the shape a seam declares (fieldless, labeled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotCarried {
    NoEligibleCarrier,
    ParametersUnverified,
}

impl ReasonLabel for NotCarried {
    const ALL: &'static [Self] = &[
        NotCarried::NoEligibleCarrier,
        NotCarried::ParametersUnverified,
    ];
    fn label(&self) -> &'static str {
        match self {
            NotCarried::NoEligibleCarrier => "no_eligible_carrier",
            NotCarried::ParametersUnverified => "parameters_unverified",
        }
    }
}

fn labelled<R: ReasonLabel>(u: &Unanswered<R>) -> &'static str {
    check_reason_labels::<R>().expect("the seam's reason type is a conformant ReasonLabel");
    u.label()
}

#[test]
fn every_reason_type_used_in_unanswered_is_a_conformant_reason_label() {
    let meanwhile = || Meanwhile::Nothing {
        because: "no copy is held here".into(),
    };
    let at = ComeBack::At { epoch_secs: 100 };
    assert_eq!(
        labelled(&Unanswered::new(
            AnswerReason::Timeout,
            at.clone(),
            meanwhile()
        )),
        "timeout"
    );
    assert_eq!(
        labelled(&Unanswered::new(
            AnswerState::Unreachable,
            at.clone(),
            meanwhile()
        )),
        "unreachable"
    );
    assert_eq!(
        labelled(&Unanswered::new(
            NotCarried::ParametersUnverified,
            ComeBack::OnEvent {
                event: "parameters-declared".into()
            },
            Meanwhile::BuiltInInForce,
        )),
        "parameters_unverified"
    );
    // map_why lifts an inner seam's reason into the outer seam's vocabulary, and the result still
    // labels.
    let lifted = Unanswered::new(AnswerReason::Refused, at, Meanwhile::RunHere)
        .map_why(|_| NotCarried::NoEligibleCarrier);
    assert_eq!(labelled(&lifted), "no_eligible_carrier");
}

/// The inputs a pending placement's status is derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Inputs {
    tier_reachable: bool,
    value: Option<u32>,
    withdrawn: bool,
}

/// The seam's pure status function over its inputs. `Changes` turns it into the cursor, so the
/// change-only and stop-after-terminal behaviour under test is the crate's, not this file's.
impl StatusSource for Inputs {
    type Value = u32;
    type Reason = AnswerReason;

    fn status(&self, now: u64) -> PendingStatus<u32, AnswerReason> {
        if self.withdrawn {
            return PendingStatus::Withdrawn;
        }
        match (self.tier_reachable, self.value) {
            (true, Some(v)) => PendingStatus::Answered(v),
            (true, None) => PendingStatus::Queued,
            (false, _) if now < 50 => PendingStatus::RetryingAt { epoch_secs: 50 },
            (false, _) => PendingStatus::Unanswered(Unanswered::new(
                AnswerReason::Timeout,
                ComeBack::OnEvent {
                    event: "tier-reachable".into(),
                },
                Meanwhile::UseHeld {
                    what: "the last verified release".into(),
                },
            )),
        }
    }
}

type WatchedNeed = Changes<Inputs>;

fn watched(tier_reachable: bool, value: Option<u32>) -> WatchedNeed {
    Changes::new(Inputs {
        tier_reachable,
        value,
        withdrawn: false,
    })
}

#[test]
fn a_cursor_whose_inputs_did_not_change_returns_none() {
    let mut c = watched(true, None);
    assert_eq!(c.poll_status(10), Some(PendingStatus::Queued));
    for now in 11..40 {
        assert_eq!(c.poll_status(now), None, "a quiet channel says nothing");
    }
}

#[test]
fn a_cursor_reaches_a_terminal_state_only_through_answered_or_withdrawn() {
    let mut c = watched(false, None);
    let mut trace = Vec::new();
    for now in [10u64, 20, 60, 70] {
        trace.extend(c.poll_status(now));
    }
    c.source_mut().tier_reachable = true;
    trace.extend(c.poll_status(80));
    c.source_mut().value = Some(7);
    trace.extend(c.poll_status(90));
    trace.extend(c.poll_status(91));
    c.source_mut().withdrawn = true;
    trace.extend(c.poll_status(92));

    assert_eq!(trace.len(), 4, "{trace:?}");
    assert_eq!(trace[0], PendingStatus::RetryingAt { epoch_secs: 50 });
    assert!(matches!(trace[1], PendingStatus::Unanswered(_)));
    assert_eq!(trace[2], PendingStatus::Queued);
    assert_eq!(trace[3], PendingStatus::Answered(7));

    // Only the last status is terminal, and it is Answered or Withdrawn.
    for s in &trace[..trace.len() - 1] {
        assert!(!s.is_terminal(), "{s:?} is followed by another status");
    }
    assert!(matches!(
        trace.last(),
        Some(PendingStatus::Answered(_) | PendingStatus::Withdrawn)
    ));
}

#[test]
fn withdrawal_is_terminal_and_never_read_as_answered() {
    let mut c = watched(true, None);
    assert_eq!(c.poll_status(1), Some(PendingStatus::Queued));
    c.source_mut().withdrawn = true;
    let s = c.poll_status(2).expect("withdrawal is a change");
    assert_eq!(s, PendingStatus::Withdrawn);
    assert!(s.is_terminal());
    c.source_mut().value = Some(3);
    assert_eq!(c.poll_status(3), None);
}

#[test]
fn the_cursor_is_object_safe() {
    let mut boxed: Box<dyn StatusCursor<Value = u32, Reason = AnswerReason>> =
        Box::new(watched(true, Some(1)));
    assert_eq!(boxed.poll_status(0), Some(PendingStatus::Answered(1)));
    assert_eq!(boxed.poll_status(1), None);
}

#[cfg(feature = "harness")]
#[test]
fn changes_over_a_seam_source_passes_the_status_harness() {
    use seam_contracts::harness::check_status_cursor;

    let mut c = watched(false, None);
    let report = check_status_cursor(
        &mut c,
        &[10, 20, 60, 70, 80, 90, 91, 92],
        |c, step| {
            let inputs = c.source_mut();
            match step {
                4 => inputs.tier_reachable = true,
                5 => inputs.value = Some(7),
                7 => inputs.withdrawn = true,
                _ => {}
            }
        },
        |c, now| c.source().status(now),
    )
    .expect("Changes keeps the status-channel contract");
    assert_eq!(report.statuses, 4, "{report:?}");
    assert!(report.terminal);
}
