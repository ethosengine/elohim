//! `Unanswered<R>` and the status channel — ruling 7 as a type.
//!
//! Design: `genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`
//! §2.0 (finding 17); the ruling itself is ruling 7 of
//! `genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md`.
//!
//! ## The guarantee
//!
//! > Every refusal, absence or "not carried" a seam returns says three things: **why**, **when to
//! > come back**, and **what can be done meanwhile** — and a repeat ask before the come-back time
//! > escalates rather than repeats.
//!
//! [`Answer<T>`](crate::Answer) says whether a fact the decider needs arrived. This module says why
//! a *request* was not met. The two compose: a seam reads its facts through `Answer<T>`, and when the
//! request cannot be met it returns an `Unanswered<R>` whose `why` is the seam's own typed reason.
//!
//! ## Why the three parts are fields, not `Option`s
//!
//! A reply that names a reason but no come-back leaves the caller to poll, which is how a quiet
//! refusal turns into a retry storm; a reply that names neither leaves the person staring at a
//! spinner. Making `come_back` and `meanwhile` required fields means a constructor that omits one
//! does not compile. "Nothing can be done meanwhile" is still allowed — it is
//! [`Meanwhile::Nothing`] with a stated `because`, said rather than implied.
//!
//! ## The status channel
//!
//! A pending answer travels as a sequence of [`PendingStatus`] values pulled through a
//! [`StatusCursor`]. In a pure core the cursor is a pull interface with the time supplied by the
//! caller (this crate never reads a clock); an adapter crate wraps it as a stream. A cursor whose
//! inputs did not change returns `None`, so a quiet channel costs nothing at rest.
//!
//! ## Scope: this is not the wire shape
//!
//! The wire form is `elohim/sdk/schemas/v1/objects/unanswered.schema.json`, and the TypeScript twin
//! is generated from that schema. The `serde` derives here are the internal convenience form, as
//! everywhere else in this crate.

use crate::reason::ReasonLabel;

/// A request that was not met, with why, when to come back and what can be done meanwhile — R7.
///
/// **Concerns:** C4 (honest absence — "not carried" is never silent), C3 (liveness — every
/// unanswered request names the event or time after which it can move), C8 (when `R` is a
/// [`ReasonLabel`], [`Unanswered::label`] gives the metric-safe label).
///
/// **Forcing incident:** the 2026-10-20 placement design found every seam (tier fetch, walk,
/// derivation, eligibility, carrying, parameter selection) answering a refusal with a bare reason or
/// `None`, and none saying when to return or what to use meanwhile (interfaces spec, finding 17).
///
/// **Contract test:** `unanswered_tests` in this module and `tests/unanswered.rs`.
///
/// Generic over the seam's own reason so each seam keeps its vocabulary. The struct puts no bound on
/// `R`: a seam whose reason carries data (an absence naming its path) keeps that data in `R` and
/// exposes a fieldless [`ReasonLabel`] face for counting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct Unanswered<R> {
    /// The seam's own typed reason.
    pub why: R,
    /// When the answer can change.
    pub come_back: ComeBack,
    /// What the asker can do in the meantime.
    pub meanwhile: Meanwhile,
    /// Set when this is a repeat ask before `come_back`: the answer escalates rather than repeats.
    pub escalation: Option<Escalation>,
}

/// When an unanswered request can change — R7's "when to come back".
///
/// **Concerns:** C3 (liveness — a named exit from the waiting state), C2 (the time is one a tier or
/// gate gave, supplied by the caller, never read from a local clock here).
///
/// **Forcing incident:** as [`Unanswered`] (interfaces spec, finding 17).
///
/// **Contract test:** `come_back_and_meanwhile_carry_their_payloads`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(
        rename_all = "kebab-case",
        rename_all_fields = "camelCase",
        tag = "when"
    )
)]
pub enum ComeBack {
    /// A time a tier or gate gave, in seconds since the Unix epoch.
    At {
        /// The time to come back.
        epoch_secs: u64,
    },
    /// An event after which the answer can change, e.g. `tier-reachable`, `posture-changed`,
    /// `parameters-declared`.
    OnEvent {
        /// The event's name.
        event: String,
    },
}

/// What can be done locally while a request is unanswered — R7's "meanwhile".
///
/// **Concerns:** C4 (`Nothing` must say why, so "nothing" is a statement, not a gap), the
/// local-first floor (what one device can do alone is named first).
///
/// **Forcing incident:** as [`Unanswered`] (interfaces spec, finding 17).
///
/// **Contract test:** `come_back_and_meanwhile_carry_their_payloads`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(
        rename_all = "kebab-case",
        rename_all_fields = "camelCase",
        tag = "local"
    )
)]
pub enum Meanwhile {
    /// Something already local can be used; `what` names it.
    UseHeld {
        /// What is already held, named.
        what: String,
    },
    /// The device can do it itself.
    RunHere,
    /// A built-in default is answering in its place.
    BuiltInInForce,
    /// Nothing can be done meanwhile, and `because` says why.
    Nothing {
        /// Why nothing can be done.
        because: String,
    },
}

/// A repeat ask before the come-back time: the answer escalates rather than repeats.
///
/// **Concerns:** C3 (liveness — a repeat that only repeats is a loop with no exit), C4 (`changed`
/// says what moved since the last answer, so an unchanged state is not re-read as new).
///
/// **Forcing incident:** as [`Unanswered`] (interfaces spec, finding 17).
///
/// **Contract test:** `escalate_counts_asks_and_keeps_the_three_parts`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct Escalation {
    /// How many times the request has been asked, this ask included (so the first escalation is 2).
    pub asked: u32,
    /// What changed since the last answer, said even when it is "nothing".
    pub changed: String,
    /// Who else can be asked.
    pub who_can_be_asked: Vec<String>,
}

impl<R> Unanswered<R> {
    /// An unanswered request with its three parts and no escalation.
    ///
    /// **Concerns:** C4, C3.
    ///
    /// **Contract test:** `new_has_no_escalation`.
    pub fn new(why: R, come_back: ComeBack, meanwhile: Meanwhile) -> Self {
        Self {
            why,
            come_back,
            meanwhile,
            escalation: None,
        }
    }

    /// A repeat before `come_back`: the same three parts plus what changed and who can be asked.
    ///
    /// The ask count starts at 2 on the first escalation and rises by one on each further one,
    /// saturating at `u32::MAX`.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `escalate_counts_asks_and_keeps_the_three_parts`.
    pub fn escalate(self, changed: String, who_can_be_asked: Vec<String>) -> Self {
        let asked = self
            .escalation
            .as_ref()
            .map_or(2, |e| e.asked.saturating_add(1));
        Self {
            escalation: Some(Escalation {
                asked,
                changed,
                who_can_be_asked,
            }),
            ..self
        }
    }

    /// Re-type the reason, keeping come-back, meanwhile and escalation.
    ///
    /// **Concerns:** C8 (a seam lifts an inner seam's reason into its own vocabulary without losing
    /// the other two parts).
    ///
    /// **Contract test:** `map_why_keeps_the_other_parts`.
    pub fn map_why<Q>(self, f: impl FnOnce(R) -> Q) -> Unanswered<Q> {
        Unanswered {
            why: f(self.why),
            come_back: self.come_back,
            meanwhile: self.meanwhile,
            escalation: self.escalation,
        }
    }
}

impl<R: ReasonLabel> Unanswered<R> {
    /// The metric-safe label of the reason, for a seam whose reason is itself a [`ReasonLabel`].
    ///
    /// **Concerns:** C8.
    ///
    /// **Contract test:** `label_is_the_reason_label`.
    pub fn label(&self) -> &'static str {
        self.why.label()
    }
}

/// One status on the channel a pending answer travels on — R7.
///
/// **Concerns:** C3 (liveness — every non-terminal status is followed by another; `Unanswered`
/// names when through `come_back`), C4 (`Withdrawn` is not `Answered`: a withdrawn need is never
/// read as met).
///
/// **Forcing incident:** as [`Unanswered`] (interfaces spec, finding 17): no status channel existed,
/// so a pending placement could only be polled by re-asking.
///
/// **Contract test:** `only_answered_and_withdrawn_are_terminal` and `tests/unanswered.rs`.
///
/// Terminal states are `Answered` and `Withdrawn`; see [`PendingStatus::is_terminal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingStatus<T, R> {
    /// Accepted and waiting its turn.
    Queued,
    /// Will be retried at the given time (seconds since the Unix epoch).
    RetryingAt {
        /// The retry time.
        epoch_secs: u64,
    },
    /// Not met yet. Not terminal: `come_back` says when it will change.
    Unanswered(Unanswered<R>),
    /// Terminal: the answer.
    Answered(T),
    /// Terminal: the need or request was withdrawn.
    Withdrawn,
}

impl<T, R> PendingStatus<T, R> {
    /// `true` for `Answered` and `Withdrawn`, the only statuses after which no other follows.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `only_answered_and_withdrawn_are_terminal`.
    pub fn is_terminal(&self) -> bool {
        matches!(self, PendingStatus::Answered(_) | PendingStatus::Withdrawn)
    }
}

/// A pull cursor over a pending answer's statuses — R7's status channel in a pure core.
///
/// **Concerns:** C3 (liveness), C6a (bounded work — a quiet channel returns `None` and costs
/// nothing at rest), C2 (the time is supplied by the caller; the cursor never reads a clock).
///
/// **Forcing incident:** as [`PendingStatus`].
///
/// **Contract test:** `tests/unanswered.rs` — a cursor whose inputs did not change returns `None`,
/// and a cursor reaches a terminal state only through `Answered` or `Withdrawn`.
///
/// An implementation returns `Some(status)` only when the status changed since the last poll, and
/// returns `None` forever after it has returned a terminal status. [`Changes`] is that
/// implementation over a seam's pure [`StatusSource`]; a seam that writes its own cursor runs
/// `harness::status::check_status_cursor` over it.
pub trait StatusCursor {
    /// The answer's value.
    type Value;
    /// The seam's reason type.
    type Reason;
    /// The status if it changed since the last poll. `None` means no change since the last poll.
    fn poll_status(
        &mut self,
        now_epoch_secs: u64,
    ) -> Option<PendingStatus<Self::Value, Self::Reason>>;
}

/// A seam's pure status function: the status a pending answer has now, from the seam's own inputs.
/// Wrapped in [`Changes`], it becomes a [`StatusCursor`] that reports only changes and stops after a
/// terminal status, so a seam writes the function and not the cursor bookkeeping.
///
/// **Concerns:** C3 (liveness), C2 (`now_epoch_secs` is the caller's; the function reads no clock).
///
/// **Forcing incident:** fix round 1 of C1 (2026-10-20): the cursor's change-only and
/// stop-after-terminal behaviour was written only inside a test, so every seam would have written
/// it again by hand.
///
/// **Contract test:** `tests/unanswered.rs`.
pub trait StatusSource {
    /// The answer's value.
    type Value;
    /// The seam's reason type.
    type Reason;
    /// The status now. Called on every poll; it should depend only on the seam's inputs and `now`.
    fn status(&self, now_epoch_secs: u64) -> PendingStatus<Self::Value, Self::Reason>;
}

/// The [`StatusCursor`] over a [`StatusSource`]: it returns a status only when it differs from the
/// last one returned, and returns `None` forever after a terminal one.
///
/// **Concerns:** C3 (the terminal set is the only exit, and nothing follows it), C6a (a quiet
/// channel returns `None`).
///
/// **Forcing incident:** as [`StatusSource`].
///
/// **Contract test:** `tests/unanswered.rs::a_cursor_whose_inputs_did_not_change_returns_none`,
/// `a_cursor_reaches_a_terminal_state_only_through_answered_or_withdrawn`, and
/// `harness::status::check_status_cursor` run over it.
///
/// It keeps a clone of the last status it returned, so `Value` and `Reason` must be `Clone` and
/// `PartialEq`. The seam changes its inputs through [`Changes::source_mut`].
#[derive(Debug, Clone)]
pub struct Changes<S: StatusSource> {
    source: S,
    last: Option<PendingStatus<S::Value, S::Reason>>,
    done: bool,
}

impl<S: StatusSource> Changes<S> {
    /// A cursor that has returned nothing yet, so its first poll returns the current status.
    pub fn new(source: S) -> Self {
        Self {
            source,
            last: None,
            done: false,
        }
    }

    /// The source, to read its inputs.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The source, to change its inputs. The next poll reports the status if it changed.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// `true` once a terminal status has been returned.
    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl<S> StatusCursor for Changes<S>
where
    S: StatusSource,
    S::Value: Clone + PartialEq,
    S::Reason: Clone + PartialEq,
{
    type Value = S::Value;
    type Reason = S::Reason;

    fn poll_status(&mut self, now_epoch_secs: u64) -> Option<PendingStatus<S::Value, S::Reason>> {
        if self.done {
            return None;
        }
        let status = self.source.status(now_epoch_secs);
        if self.last.as_ref() == Some(&status) {
            return None;
        }
        self.done = status.is_terminal();
        self.last = Some(status.clone());
        Some(status)
    }
}

#[cfg(test)]
mod unanswered_tests {
    use super::*;
    use crate::AnswerReason;

    fn later() -> ComeBack {
        ComeBack::OnEvent {
            event: "tier-reachable".into(),
        }
    }

    fn held() -> Meanwhile {
        Meanwhile::UseHeld {
            what: "the cached release".into(),
        }
    }

    #[test]
    fn new_has_no_escalation() {
        let u = Unanswered::new(AnswerReason::Timeout, later(), held());
        assert_eq!(u.why, AnswerReason::Timeout);
        assert_eq!(u.come_back, later());
        assert_eq!(u.meanwhile, held());
        assert_eq!(u.escalation, None);
    }

    #[test]
    fn escalate_counts_asks_and_keeps_the_three_parts() {
        let u = Unanswered::new(AnswerReason::Timeout, later(), held())
            .escalate("nothing yet".into(), vec!["agent:jessica".into()]);
        let e = u.escalation.clone().expect("escalated");
        assert_eq!(e.asked, 2, "the first escalation is the second ask");
        assert_eq!(e.changed, "nothing yet");
        assert_eq!(e.who_can_be_asked, vec!["agent:jessica".to_string()]);
        assert_eq!(u.why, AnswerReason::Timeout);
        assert_eq!(u.come_back, later());
        assert_eq!(u.meanwhile, held());

        let again = u.escalate("a second doorway answered".into(), vec![]);
        let e = again.escalation.expect("escalated");
        assert_eq!(e.asked, 3);
        assert_eq!(e.changed, "a second doorway answered");
        assert!(
            e.who_can_be_asked.is_empty(),
            "the latest list replaces the earlier one"
        );
    }

    #[test]
    fn escalate_saturates() {
        let mut u = Unanswered::new(AnswerReason::Timeout, later(), held());
        u.escalation = Some(Escalation {
            asked: u32::MAX,
            changed: String::new(),
            who_can_be_asked: vec![],
        });
        let u = u.escalate("x".into(), vec![]);
        assert_eq!(u.escalation.map(|e| e.asked), Some(u32::MAX));
    }

    #[test]
    fn map_why_keeps_the_other_parts() {
        let u = Unanswered::new(
            AnswerReason::Refused,
            ComeBack::At { epoch_secs: 42 },
            Meanwhile::RunHere,
        )
        .escalate("posture changed".into(), vec!["agent:james".into()]);
        let expected_escalation = u.escalation.clone();
        let mapped = u.map_why(|r| r.state());
        assert_eq!(mapped.why, crate::AnswerState::Unreachable);
        assert_eq!(mapped.come_back, ComeBack::At { epoch_secs: 42 });
        assert_eq!(mapped.meanwhile, Meanwhile::RunHere);
        assert_eq!(mapped.escalation, expected_escalation);
    }

    #[test]
    fn label_is_the_reason_label() {
        let u = Unanswered::new(AnswerReason::NotYetDelivered, later(), held());
        assert_eq!(u.label(), "not_yet_delivered");
    }

    #[test]
    fn come_back_and_meanwhile_carry_their_payloads() {
        assert_ne!(
            ComeBack::At { epoch_secs: 1 },
            ComeBack::At { epoch_secs: 2 }
        );
        assert_ne!(
            Meanwhile::Nothing {
                because: "no local copy".into()
            },
            Meanwhile::Nothing {
                because: "no compute here".into()
            }
        );
        assert_ne!(Meanwhile::RunHere, Meanwhile::BuiltInInForce);
    }

    #[test]
    fn only_answered_and_withdrawn_are_terminal() {
        type S = PendingStatus<u8, AnswerReason>;
        let statuses: [S; 5] = [
            PendingStatus::Queued,
            PendingStatus::RetryingAt { epoch_secs: 9 },
            PendingStatus::Unanswered(Unanswered::new(AnswerReason::Timeout, later(), held())),
            PendingStatus::Answered(7),
            PendingStatus::Withdrawn,
        ];
        let terminal: Vec<bool> = statuses.iter().map(PendingStatus::is_terminal).collect();
        assert_eq!(terminal, vec![false, false, false, true, true]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn wire_convenience_derives_exist() {
        fn both<T: serde::Serialize + for<'de> serde::Deserialize<'de>>() {}
        both::<Unanswered<AnswerReason>>();
        both::<ComeBack>();
        both::<Meanwhile>();
        both::<Escalation>();
    }
}
