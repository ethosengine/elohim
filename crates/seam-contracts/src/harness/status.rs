//! The status-cursor harness — ruling 7's status channel (interfaces spec §2.0) as a check a seam
//! runs over its own [`StatusCursor`].
//!
//! Design: `genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`
//! §2.0.
//!
//! ## The guarantee
//!
//! > A cursor whose inputs did not change returns `None`; a cursor whose status changed reports
//! > the new status at the next poll; a status is reported only when it differs from the last one
//! > reported; nothing is reported after `Answered` or `Withdrawn`.
//!
//! A seam that builds its cursor from [`Changes`](crate::unanswered::Changes) gets this by
//! construction. A seam that writes its own cursor runs [`check_status_cursor`] over it with a
//! script of poll times, the input changes it makes between them, and the status its inputs hold
//! at each step.
//!
//! ## How the script is read
//!
//! For each step `i`, the harness calls `change(cursor, i)` (the seam changes its inputs, or does
//! nothing), asks `expected(cursor, times[i])` for the status the inputs now hold, polls at
//! `times[i]`, then polls again at the same time with no change in between.
//!
//! - The first poll must return `Some(expected)` when `expected` differs from the last status
//!   reported (or nothing was reported yet), and `None` when it is the same. A cursor that stays
//!   silent when its status changed fails with `silent_after_change`, so a cursor that never
//!   speaks cannot pass: its first step already has a status to report.
//! - The second poll must return `None`: the inputs and the time are the same, so nothing changed.
//! - After a terminal status every poll must return `None`, whatever `expected` says.
//!
//! The harness cannot know on its own what a cursor ought to report; `expected` is the seam's
//! statement of it, and for a seam with a pure [`StatusSource`](crate::unanswered::StatusSource)
//! it is that function. The check is exhaustive over the script it is given and nothing else (C6a:
//! the script is the budget).

use crate::unanswered::{PendingStatus, StatusCursor};
use crate::ReasonLabel;

/// What the harness observed.
///
/// **Concerns:** C7 (the report says what ran), C8.
///
/// **Contract test:** `changes_passes_the_harness`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCursorReport {
    /// How many polls ran, re-polls included.
    pub polls: usize,
    /// How many statuses the cursor reported.
    pub statuses: usize,
    /// Whether a terminal status was reported.
    pub terminal: bool,
}

/// Why a status cursor failed the check.
///
/// **Concerns:** C3, C6a, C8.
///
/// **Contract test:** `status_violation_class_is_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatusCursorViolationClass {
    /// A re-poll at the same time with no input change returned a status: the channel is not quiet
    /// at rest.
    RepolledWithoutChange,
    /// A status equal to the last one reported was reported again.
    ReportedUnchanged,
    /// A status was reported after `Answered` or `Withdrawn`.
    SpokeAfterTerminal,
    /// The inputs' status differed from the last one reported (or nothing was reported yet), and
    /// the first poll after the change returned `None`: a waiter on this cursor never hears it.
    SilentAfterChange,
    /// A status was reported that is not the status the inputs hold.
    ReportedOtherThanExpected,
}

impl ReasonLabel for StatusCursorViolationClass {
    const ALL: &'static [Self] = &[
        StatusCursorViolationClass::RepolledWithoutChange,
        StatusCursorViolationClass::ReportedUnchanged,
        StatusCursorViolationClass::SpokeAfterTerminal,
        StatusCursorViolationClass::SilentAfterChange,
        StatusCursorViolationClass::ReportedOtherThanExpected,
    ];

    fn label(&self) -> &'static str {
        match self {
            StatusCursorViolationClass::RepolledWithoutChange => "repolled_without_change",
            StatusCursorViolationClass::ReportedUnchanged => "reported_unchanged",
            StatusCursorViolationClass::SpokeAfterTerminal => "spoke_after_terminal",
            StatusCursorViolationClass::SilentAfterChange => "silent_after_change",
            StatusCursorViolationClass::ReportedOtherThanExpected => "reported_other_than_expected",
        }
    }
}

/// A failed status-cursor check: typed class + detail.
///
/// **Concerns:** C3, C8.
///
/// **Contract test:** `repolled_without_change_is_caught`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCursorViolation {
    /// The typed failure class.
    pub class: StatusCursorViolationClass,
    /// Which step, at what time, and what was reported.
    pub detail: String,
}

impl core::fmt::Display for StatusCursorViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.class.label(), self.detail)
    }
}

impl std::error::Error for StatusCursorViolation {}

/// Check a status cursor against the status-channel contract over a script of poll times.
///
/// **Concerns:** C3 (a changed status is reported at the next poll, and nothing follows a terminal
/// one), C6a (a quiet channel returns `None`).
///
/// **Contract test:** `changes_passes_the_harness`, `repolled_without_change_is_caught`,
/// `reported_unchanged_is_caught`, `spoke_after_terminal_is_caught`,
/// `a_cursor_that_never_speaks_is_caught`, `a_missed_answer_is_caught`,
/// `reported_other_than_expected_is_caught`.
///
/// `change(cursor, i)` runs before step `i`'s poll and may change the cursor's inputs or leave them.
/// `expected(cursor, now)` is the status the cursor's inputs hold at `now`, after that change.
pub fn check_status_cursor<C, F, E>(
    cursor: &mut C,
    times: &[u64],
    mut change: F,
    expected: E,
) -> Result<StatusCursorReport, StatusCursorViolation>
where
    C: StatusCursor,
    C::Value: PartialEq + core::fmt::Debug,
    C::Reason: PartialEq + core::fmt::Debug,
    F: FnMut(&mut C, usize),
    E: Fn(&C, u64) -> PendingStatus<C::Value, C::Reason>,
{
    let mut report = StatusCursorReport {
        polls: 0,
        statuses: 0,
        terminal: false,
    };
    let mut last: Option<PendingStatus<C::Value, C::Reason>> = None;

    for (step, &now) in times.iter().enumerate() {
        change(cursor, step);
        let want = expected(cursor, now);

        report.polls += 1;
        match cursor.poll_status(now) {
            Some(status) => {
                if report.terminal {
                    return Err(StatusCursorViolation {
                        class: StatusCursorViolationClass::SpokeAfterTerminal,
                        detail: format!(
                            "step {step} at {now} reported {status:?} after a terminal status"
                        ),
                    });
                }
                if last.as_ref() == Some(&status) {
                    return Err(StatusCursorViolation {
                        class: StatusCursorViolationClass::ReportedUnchanged,
                        detail: format!(
                            "step {step} at {now} reported {status:?}, the same as the last status"
                        ),
                    });
                }
                if status != want {
                    return Err(StatusCursorViolation {
                        class: StatusCursorViolationClass::ReportedOtherThanExpected,
                        detail: format!(
                            "step {step} at {now} reported {status:?}; the inputs hold {want:?}"
                        ),
                    });
                }
                report.statuses += 1;
                report.terminal = status.is_terminal();
                last = Some(status);
            }
            None => {
                if !report.terminal && last.as_ref() != Some(&want) {
                    return Err(StatusCursorViolation {
                        class: StatusCursorViolationClass::SilentAfterChange,
                        detail: format!(
                            "step {step} at {now} returned None; the inputs hold {want:?} and the \
                             last status reported was {last:?}"
                        ),
                    });
                }
            }
        }

        report.polls += 1;
        if let Some(status) = cursor.poll_status(now) {
            let class = if report.terminal {
                StatusCursorViolationClass::SpokeAfterTerminal
            } else {
                StatusCursorViolationClass::RepolledWithoutChange
            };
            return Err(StatusCursorViolation {
                class,
                detail: format!(
                    "step {step}: a re-poll at {now} with no input change reported {status:?}"
                ),
            });
        }
    }

    Ok(report)
}

/// Panicking form of [`check_status_cursor`], for use inside `#[test]`.
///
/// **Concerns:** C3.
///
/// **Contract test:** `assert_form_panics_on_violation`.
///
/// # Panics
///
/// If the cursor breaks the status-channel contract on the script.
pub fn assert_status_cursor<C, F, E>(
    cursor: &mut C,
    times: &[u64],
    change: F,
    expected: E,
) -> StatusCursorReport
where
    C: StatusCursor,
    C::Value: PartialEq + core::fmt::Debug,
    C::Reason: PartialEq + core::fmt::Debug,
    F: FnMut(&mut C, usize),
    E: Fn(&C, u64) -> PendingStatus<C::Value, C::Reason>,
{
    match check_status_cursor(cursor, times, change, expected) {
        Ok(report) => report,
        Err(v) => panic!("status cursor contract violated: {v}"),
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;
    use crate::reason::assert_reason_labels_conformant;
    use crate::unanswered::{Changes, StatusSource};
    use crate::AnswerReason;

    type S = PendingStatus<u8, AnswerReason>;

    struct Value(Option<u8>);

    impl StatusSource for Value {
        type Value = u8;
        type Reason = AnswerReason;
        fn status(&self, now: u64) -> S {
            match self.0 {
                Some(v) => PendingStatus::Answered(v),
                None if now < 10 => PendingStatus::Queued,
                None => PendingStatus::RetryingAt { epoch_secs: 20 },
            }
        }
    }

    /// A hand-written cursor that returns a fixed list of poll results, in order.
    struct Scripted(Vec<Option<S>>);

    impl StatusCursor for Scripted {
        type Value = u8;
        type Reason = AnswerReason;
        fn poll_status(&mut self, _: u64) -> Option<S> {
            if self.0.is_empty() {
                None
            } else {
                self.0.remove(0)
            }
        }
    }

    fn always(s: S) -> impl Fn(&Scripted, u64) -> S {
        move |_, _| s.clone()
    }

    #[test]
    fn changes_passes_the_harness() {
        let mut c = Changes::new(Value(None));
        let report = check_status_cursor(
            &mut c,
            &[1, 5, 12, 15, 30, 31],
            |c, step| {
                if step == 4 {
                    c.source_mut().0 = Some(3);
                }
            },
            |c, now| c.source().status(now),
        )
        .expect("Changes keeps the contract");
        assert_eq!(report.polls, 12);
        assert_eq!(report.statuses, 3, "Queued, RetryingAt, Answered");
        assert!(report.terminal);
        assert!(c.is_done());
    }

    #[test]
    fn repolled_without_change_is_caught() {
        let mut c = Scripted(vec![
            Some(PendingStatus::Queued),
            Some(PendingStatus::Queued),
        ]);
        let v = check_status_cursor(&mut c, &[1], |_, _| {}, always(PendingStatus::Queued))
            .expect_err("caught");
        assert_eq!(v.class, StatusCursorViolationClass::RepolledWithoutChange);
    }

    #[test]
    fn reported_unchanged_is_caught() {
        let mut c = Scripted(vec![
            Some(PendingStatus::Queued),
            None,
            Some(PendingStatus::Queued),
        ]);
        let v = check_status_cursor(&mut c, &[1, 2], |_, _| {}, always(PendingStatus::Queued))
            .expect_err("caught");
        assert_eq!(v.class, StatusCursorViolationClass::ReportedUnchanged);
    }

    #[test]
    fn spoke_after_terminal_is_caught() {
        let mut c = Scripted(vec![
            Some(PendingStatus::Answered(1)),
            None,
            Some(PendingStatus::Queued),
        ]);
        let v = check_status_cursor(
            &mut c,
            &[1, 2],
            |_, _| {},
            always(PendingStatus::Answered(1)),
        )
        .expect_err("caught");
        assert_eq!(v.class, StatusCursorViolationClass::SpokeAfterTerminal);

        let mut c = Scripted(vec![
            Some(PendingStatus::Withdrawn),
            Some(PendingStatus::Answered(1)),
        ]);
        let v = check_status_cursor(&mut c, &[1], |_, _| {}, always(PendingStatus::Withdrawn))
            .expect_err("caught");
        assert_eq!(v.class, StatusCursorViolationClass::SpokeAfterTerminal);
    }

    /// The cursor fix round 2 named: it never speaks, so it breaks none of the safety clauses.
    struct Silent;

    impl StatusCursor for Silent {
        type Value = u8;
        type Reason = AnswerReason;
        fn poll_status(&mut self, _: u64) -> Option<S> {
            None
        }
    }

    #[test]
    fn a_cursor_that_never_speaks_is_caught() {
        let v = check_status_cursor(
            &mut Silent,
            &[1, 2, 3],
            |_, _| {},
            |_, _| PendingStatus::Queued,
        )
        .expect_err("a silent cursor fails at its first step");
        assert_eq!(v.class, StatusCursorViolationClass::SilentAfterChange);
        assert!(v.detail.starts_with("step 0"), "{v}");
    }

    #[test]
    fn a_missed_answer_is_caught() {
        // Reports Queued, then stays silent when the inputs move to Answered.
        let mut c = Scripted(vec![Some(PendingStatus::Queued)]);
        let v = check_status_cursor(
            &mut c,
            &[1, 2, 3],
            |_, _| {},
            |_, now| {
                if now < 3 {
                    PendingStatus::Queued
                } else {
                    PendingStatus::Answered(4)
                }
            },
        )
        .expect_err("the missed Answered is caught");
        assert_eq!(v.class, StatusCursorViolationClass::SilentAfterChange);
        assert!(v.detail.starts_with("step 2"), "{v}");
    }

    #[test]
    fn reported_other_than_expected_is_caught() {
        let mut c = Scripted(vec![Some(PendingStatus::Answered(9))]);
        let v = check_status_cursor(&mut c, &[1], |_, _| {}, always(PendingStatus::Answered(4)))
            .expect_err("caught");
        assert_eq!(
            v.class,
            StatusCursorViolationClass::ReportedOtherThanExpected
        );
    }

    #[test]
    #[should_panic(expected = "status cursor contract violated")]
    fn assert_form_panics_on_violation() {
        assert_status_cursor(&mut Silent, &[1], |_, _| {}, |_, _| PendingStatus::Queued);
    }

    #[test]
    fn status_violation_class_is_conformant() {
        assert_reason_labels_conformant::<StatusCursorViolationClass>();
    }
}
