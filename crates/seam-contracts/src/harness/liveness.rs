//! The `Liveness` table harness — C3 (liveness) as an exhaustively-enumerated
//! state-space check.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C3; Design surface 3; task P2.1).
//!
//! ## The guarantee
//!
//! > From every reachable non-terminal state, at least one **automated**
//! > transition is enabled. A human hand (or a deploy) as the only exit is a
//! > liveness hole, not an escape hatch.
//!
//! That second sentence is the whole law, and it is why this harness classifies
//! transitions by *agency* rather than merely counting them. A state whose only
//! exit is `operator DELETE /acquisition/pins/{id}` has a transition; it does
//! not have liveness. The fleet cannot take it.
//!
//! ## Why a table, and why the caller supplies the table
//!
//! Liveness is a property of the **conjunction** of guards, and a conjunction is
//! not visible from any one guard's source. The 2026-08-02 deadlock was four
//! *individually correct* refusals — each with its own passing unit tests —
//! composing into a state with no legal move. Nothing in a per-predicate review
//! loop asks the composite question, so the composite question gets a harness.
//!
//! The state space arrives as **plain data the caller enumerates**: a slice of
//! states, a classifier, and a transition-enumeration function. Deliberately not
//! `proptest`/`arbitrary` — a randomized search that misses the one dead corner
//! reports green, and the corners here are few and nameable
//! (`decide_head_action`'s own space is 2⁵ = 32). Exhaustive enumeration over a
//! small hand-named space is a *proof*; sampling over it is only evidence. Plan
//! Design surface 7 authorizes splitting a `-testkit` crate if and only if a
//! harness needs those dependencies — a table harness does not, so this crate
//! stays a leaf with zero new dependencies.
//!
//! ## What this harness checks
//!
//! | Check | Catches |
//! |---|---|
//! | every reachable non-terminal state enumerates ≥ 1 transition | [`LivenessViolationClass::DeadEnd`] |
//! | …and ≥ 1 of them is **automated** | [`LivenessViolationClass::ManualOnlyExit`] |
//! | a terminal state is genuinely an end (no exits but self-loops) | [`LivenessViolationClass::TerminalStateHasExits`] |
//! | an `Unreachable` claim is not contradicted by an inbound transition | [`LivenessViolationClass::UnreachableStateContradicted`] |
//! | every modeled target is a state the table enumerates | [`LivenessViolationClass::UnknownTargetState`] |
//! | some sequence of automated transitions reaches a terminal state | [`LivenessViolationClass::NoAutomatedPathToTerminal`] |
//! | the table is not empty / not over its declared ceiling | [`LivenessViolationClass::EmptyStateSpace`], [`LivenessViolationClass::StateSpaceOverBudget`] |
//!
//! The last check is the composition catch and it runs only when the table
//! models every target (see [`ProgressSkip`]). Per-state legality is what four
//! locally-correct guards each pass; *progress* is what their conjunction lost.
//!
//! ## Forcing incidents
//!
//! - **2026-08-02, the four-wall two-way-declared deadlock.** Four stacked
//!   walls — supply deadlock (`AdoptPeer` unreachable once the local row is
//!   declared), consumption wall (the stamp guard comparing a wall clock the
//!   deploy PATCH path had NULLed), declare wall (a target-independent no-chain
//!   gate), admission wall (declared+divergent rows dying at the
//!   `SkippedDeclared` refusal before entering the candidate list) — each a
//!   correct safety rule, together leaving the two-way-declared state with no
//!   automated exit at all. Its only live exit was the deploy declare-cycle:
//!   a [`Agency::Deploy`] transition, which is precisely what the law refuses to
//!   count. Cured in `496a4aba8`, `134331c83`, `da8975176`; recorded in
//!   `genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md`
//!   (§"RCA v3").
//! - **2026-07-11, the over-broad GapFill guard** that froze forward
//!   convergence — the same shape a month earlier, at a seam the later shift
//!   never touched. Two independently-dated instances is what makes C3 a class
//!   rather than an anecdote, and plan task P2.2 turns exactly this pair into
//!   the harness's own regression demonstration (fails on both reconstructed
//!   predicate sets, passes current). That demonstration is P2.2's; this module
//!   is the instrument it runs.
//!
//! Two more instances shape the violation vocabulary directly: `192f77cd0`
//! (2026-07-31) — the only transition out of an exhausted acquisition pin was a
//! human `DELETE`, which is [`ManualOnlyExit`](LivenessViolationClass::ManualOnlyExit)
//! exactly; and `c16daaa79` (2026-08-01) — a dropped doorway half-open trial
//! latching shed-forever, which is [`DeadEnd`](LivenessViolationClass::DeadEnd).
//!
//! ## What this harness does NOT check
//!
//! It does not check that the transitions are *correct*, or that the state space
//! is *complete*. A table that omits the deadlocked corner passes, which is why
//! the table is authored beside the predicate and reviewed as part of it. It
//! also says nothing about *time*: "eventually" is not modeled, so a legal-but-
//! glacial move counts as liveness here. Throughput is C6a/C11 territory.
//!
//! It cannot prove progress for a table with **no terminal state** (a pure
//! controller loop). Such a table gets the per-state checks and an honest
//! [`ProgressSkip::NoTerminalState`] in the report rather than a false green:
//! declare the converged/settled state to unlock the composition check.

use crate::ReasonLabel;

/// Who takes a transition — the distinction the C3 law turns on.
///
/// **Concerns:** C3 (liveness — *"a human hand (or a deploy) as the only exit is
/// a liveness hole, not an escape hatch"*), C8 (the harness's own vocabulary is
/// reason-labeled).
///
/// **Forcing incident:** `192f77cd0` (2026-07-31) — the only transition out of
/// an exhausted acquisition pin was a human `DELETE /acquisition/pins/{id}`. The
/// state machine was *complete*: every state had an outgoing edge, and a
/// transition-counting check would have passed it. What it lacked was an edge
/// the fleet could take by itself. And 2026-08-02: the two-way-declared class's
/// only live exit was the deploy declare-cycle — a [`Agency::Deploy`] edge that
/// R1 had deliberately made load-bearing as an *interim* channel, which is a
/// labeled scaffold (C13), never a liveness answer.
///
/// **Contract test:** `manual_only_exit_is_caught`, `deploy_only_exit_is_caught`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Agency {
    /// The fleet takes this move by itself — a sweep, a retry, a gossip
    /// delivery, a reconcile tick. The only agency that satisfies C3.
    Automated,
    /// A human hand: an operator `DELETE`, a `kubectl`, a support request, a
    /// manual re-declaration. Real, sometimes necessary — never an answer to
    /// "does the fleet have a move?"
    Human,
    /// A deploy, restart, or rollout. Counted separately from [`Agency::Human`]
    /// because it *feels* automated (CI runs it) while being exactly as
    /// unavailable to a running fleet at 03:00 as a human hand is.
    Deploy,
}

impl Agency {
    /// `true` only for [`Agency::Automated`].
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `agency_automated_is_the_only_automated_agency`.
    #[must_use]
    pub const fn is_automated(self) -> bool {
        matches!(self, Agency::Automated)
    }
}

impl ReasonLabel for Agency {
    const ALL: &'static [Self] = &[Agency::Automated, Agency::Human, Agency::Deploy];

    fn label(&self) -> &'static str {
        match self {
            Agency::Automated => "automated",
            Agency::Human => "human",
            Agency::Deploy => "deploy",
        }
    }
}

/// One enabled move out of a state.
///
/// **Concerns:** C3 (the unit the harness counts), C8 (`label` is what the
/// counterexample names, so it should read like the code path: `contest_peer`,
/// `operator DELETE /acquisition/pins/{id}`).
///
/// **Forcing incident:** see [`Agency`]. A transition is only a liveness answer
/// if the fleet can take it, so agency is a required field, not a flag.
///
/// **Contract test:** `transition_constructors_carry_agency`,
/// `unknown_target_state_is_caught`.
///
/// `to` is optional. `None` means *"this harness does not model where the move
/// lands"* — the per-state checks still run, but the composition check
/// ([`LivenessViolationClass::NoAutomatedPathToTerminal`]) is skipped for the
/// whole table and the report says so ([`ProgressSkip::UnmodeledTargets`]).
/// Modeling targets is what turns "every state has a move" into "the moves go
/// somewhere," and the difference between those two is the 2026-08-01 limit
/// cycle: ~35k legal operations per hour, none of them ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition<S> {
    /// What the move is, in the vocabulary of the code — this is what a
    /// counterexample prints.
    pub label: &'static str,
    /// Who can take it. See [`Agency`].
    pub agency: Agency,
    /// Where it lands, if the table models it. See the type-level note.
    pub to: Option<S>,
}

impl<S> Transition<S> {
    /// A move the fleet takes by itself.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `transition_constructors_carry_agency`.
    #[must_use]
    pub const fn automated(label: &'static str) -> Self {
        Self {
            label,
            agency: Agency::Automated,
            to: None,
        }
    }

    /// A move only a human hand takes.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `manual_only_exit_is_caught`.
    #[must_use]
    pub const fn human(label: &'static str) -> Self {
        Self {
            label,
            agency: Agency::Human,
            to: None,
        }
    }

    /// A move only a deploy/restart/rollout takes.
    ///
    /// **Concerns:** C3, C13 (a deploy-only exit is a labeled interim scaffold
    /// at best; it names a successor criterion or it is capture).
    ///
    /// **Contract test:** `deploy_only_exit_is_caught`.
    #[must_use]
    pub const fn deploy(label: &'static str) -> Self {
        Self {
            label,
            agency: Agency::Deploy,
            to: None,
        }
    }

    /// Declare where this move lands, unlocking the composition check.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `limit_cycle_without_an_exit_is_caught`.
    #[must_use]
    pub fn to(mut self, state: S) -> Self {
        self.to = Some(state);
        self
    }
}

impl<S: PartialEq> Transition<S> {
    /// `true` iff this move lands on `state`.
    ///
    /// **Concerns:** C3 — used to exempt self-loops from the terminal-exit
    /// check: a controller that keeps re-checking a settled state has not left
    /// it.
    ///
    /// **Contract test:** `terminal_self_loop_is_not_an_exit`.
    #[must_use]
    pub fn targets(&self, state: &S) -> bool {
        self.to.as_ref() == Some(state)
    }
}

/// How a state is classified for the liveness question.
///
/// **Concerns:** C3, C0 (a state's class is a claim about *which plane* it lives
/// on — an "unreachable" that is only unreachable in the projection is not
/// unreachable), C4 (the three arms are distinct answers; collapsing
/// `Unreachable` into `Terminal` is exactly the collapse C4 forbids elsewhere).
///
/// **Forcing incident:** 2026-07-31's exhausted acquisition pin was treated by
/// the code as an end state. It was not an end; it was a stall that a human
/// rescued. That is why both non-`Live` arms carry a **required justification
/// string**: the classifier is the one place a caller can defeat this harness
/// (call every dead end "terminal" and the table goes green), so each such claim
/// has to be *written down* where a reviewer reads it beside the state.
///
/// **Contract test:** `unreachable_dead_state_does_not_fail_the_table`,
/// `terminal_state_with_a_manual_exit_is_caught`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateClass {
    /// Reachable, and work remains. Must have ≥ 1 enabled automated transition.
    Live,
    /// A legitimate end: the decision is over and nothing further is owed.
    ///
    /// The `&'static str` is the argument for why this is an **end** and not a
    /// stall someone rescues. A terminal state should enumerate no transitions
    /// at all (self-loops excepted) — if a hand can move it, it was never
    /// terminal; classify it [`StateClass::Live`] and let the harness tell you
    /// the exit is manual-only, which is the truth you wanted to see.
    Terminal(&'static str),
    /// Cannot occur in production.
    ///
    /// The `&'static str` is the argument for why. An unjustified `Unreachable`
    /// is how a deadlock hides behind a classifier, and the harness
    /// cross-checks the claim: any modeled transition from a reachable state
    /// into this one raises
    /// [`LivenessViolationClass::UnreachableStateContradicted`].
    Unreachable(&'static str),
}

impl StateClass {
    /// `true` for [`StateClass::Live`].
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `state_class_predicates_partition_the_arms`.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, StateClass::Live)
    }

    /// `true` for [`StateClass::Terminal`].
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `state_class_predicates_partition_the_arms`.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, StateClass::Terminal(_))
    }

    /// `true` for [`StateClass::Unreachable`].
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `state_class_predicates_partition_the_arms`.
    #[must_use]
    pub const fn is_unreachable(self) -> bool {
        matches!(self, StateClass::Unreachable(_))
    }
}

/// Declared budget for [`check_liveness_with`] — C6a applied to the instrument.
///
/// **Concerns:** C6a (bounded work), C7 (the report says which regime ran).
///
/// **Forcing incident:** `6369721fd` (2026-07-04) — a responder rejected its own
/// oversized reply. An unbounded table walk is the same failure with a friendlier
/// face: a caller who enumerates a 20-boolean space asks for 1,048,576 states and
/// a hung test run.
///
/// **Contract test:** `state_space_over_budget_is_caught`,
/// `violation_report_is_capped_and_says_so`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessBudget {
    /// Ceiling on enumerated states. Exceeding it is a **violation**, never a
    /// silent sample: a liveness check has no honest sampled mode, because the
    /// one state you skip is the one that deadlocks.
    pub max_states: usize,
    /// Ceiling on violations rendered in the failure. The total is always
    /// reported, so a truncated list never reads as a short list.
    pub max_violations_reported: usize,
}

impl Default for LivenessBudget {
    /// 4096 states, 32 violations rendered.
    ///
    /// 4096 = a 12-boolean input space. `decide_head_action`'s own space is
    /// 2⁵ = 32; a predicate that needs more than twelve independent booleans is
    /// asking to be decomposed, and this ceiling says so at the moment it is
    /// crossed rather than after the test run hangs.
    ///
    /// 32 rendered violations is enough to see a whole broken composition — the
    /// 2026-08-02 deadlock was four walls — without a systematically-mis-authored
    /// table producing a page of noise.
    fn default() -> Self {
        Self {
            max_states: 4096,
            max_violations_reported: 32,
        }
    }
}

/// Why the composition (progress) check did not run.
///
/// **Concerns:** C4 (skipped ≠ passed — the distinction this crate exists to
/// preserve), C7 (the instrument advertises exactly what it served), C8.
///
/// **Forcing incident:** the 2026-08-01 fleet limit cycle read as *activity*
/// (~35k heal operations/hour) while converging nothing. An instrument that
/// silently skipped the progress check and reported "liveness OK" would have
/// been one more meter agreeing with the deadlock.
///
/// **Contract test:** `progress_check_is_skipped_without_a_terminal_state`,
/// `progress_check_is_skipped_when_targets_are_unmodeled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgressSkip {
    /// The table declares no terminal state, so "progress" has no destination.
    /// A pure controller loop is a legitimate shape; declare the
    /// converged/settled state to unlock the check.
    NoTerminalState,
    /// At least one transition does not model its target (`to: None`), or a
    /// target names a state the table does not enumerate, so the transition
    /// graph is incomplete and a reachability closure over it would be a guess.
    UnmodeledTargets,
}

impl ReasonLabel for ProgressSkip {
    const ALL: &'static [Self] = &[
        ProgressSkip::NoTerminalState,
        ProgressSkip::UnmodeledTargets,
    ];

    fn label(&self) -> &'static str {
        match self {
            ProgressSkip::NoTerminalState => "no_terminal_state",
            ProgressSkip::UnmodeledTargets => "unmodeled_targets",
        }
    }
}

/// What the harness actually did — the honest report (C7).
///
/// **Concerns:** C7, C8 (`progress_checked` is the label that keeps a per-state
/// pass from being read as a proof of convergence).
///
/// **Contract test:** `report_counts_every_class_of_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessReport {
    /// States enumerated by the caller.
    pub states_checked: usize,
    /// Of those, [`StateClass::Live`].
    pub live_states: usize,
    /// Of those, [`StateClass::Terminal`].
    pub terminal_states: usize,
    /// Of those, [`StateClass::Unreachable`] — excluded from the analysis, and
    /// their transition enumerator is never called (a state that cannot occur
    /// may not have well-defined moves).
    pub unreachable_states: usize,
    /// Transitions enumerated across all reachable states.
    pub transitions_enumerated: usize,
    /// `true` iff the composition check ran. When `false`, a pass means "every
    /// state has an automated move," NOT "the moves converge."
    pub progress_checked: bool,
    /// Why the composition check did not run. `None` iff `progress_checked`.
    pub progress_skipped: Option<ProgressSkip>,
}

impl LivenessReport {
    fn empty() -> Self {
        Self {
            states_checked: 0,
            live_states: 0,
            terminal_states: 0,
            unreachable_states: 0,
            transitions_enumerated: 0,
            progress_checked: false,
            progress_skipped: Some(ProgressSkip::NoTerminalState),
        }
    }
}

impl core::fmt::Display for LivenessReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "checked {} state(s) ({} live, {} terminal, {} unreachable) and {} transition(s); \
             composition check: {}",
            self.states_checked,
            self.live_states,
            self.terminal_states,
            self.unreachable_states,
            self.transitions_enumerated,
            match self.progress_skipped {
                None => "ran".to_string(),
                Some(skip) => format!("SKIPPED ({})", skip.label()),
            }
        )
    }
}

/// Why a liveness property failed.
///
/// **Concerns:** C3, C8 (the harness's own outcomes are a closed, labeled
/// vocabulary).
///
/// **Contract test:** `violation_class_is_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LivenessViolationClass {
    /// The table enumerates no states, so the check is vacuous. A green from an
    /// empty table is the purest form of an instrument agreeing with a bug.
    EmptyStateSpace,
    /// The table exceeds [`LivenessBudget::max_states`]. Raise the ceiling
    /// deliberately or decompose the predicate — never sample.
    StateSpaceOverBudget,
    /// A reachable non-terminal state enumerates **no** transitions at all.
    /// Nothing moves it, by anyone.
    DeadEnd,
    /// A reachable non-terminal state's only exits are [`Agency::Human`] or
    /// [`Agency::Deploy`]. The state machine is complete and the fleet is still
    /// stuck — the law's exact shape.
    ManualOnlyExit,
    /// A state declared [`StateClass::Terminal`] enumerates an exit (self-loops
    /// excepted). If something can move it, it was not an end.
    TerminalStateHasExits,
    /// A state declared [`StateClass::Unreachable`] is the target of a
    /// transition from a reachable state — the claim that excused it from the
    /// analysis is false.
    UnreachableStateContradicted,
    /// A transition names a target the table does not enumerate. The table is
    /// not the state space it advertises (C7), and every graph-derived answer
    /// over it would be a guess.
    UnknownTargetState,
    /// No sequence of automated transitions from this reachable state reaches
    /// any terminal state. Every individual move is legal; the composition never
    /// ends. This is the deadlock/limit-cycle class — the one no per-guard
    /// review can see.
    NoAutomatedPathToTerminal,
}

impl ReasonLabel for LivenessViolationClass {
    const ALL: &'static [Self] = &[
        LivenessViolationClass::EmptyStateSpace,
        LivenessViolationClass::StateSpaceOverBudget,
        LivenessViolationClass::DeadEnd,
        LivenessViolationClass::ManualOnlyExit,
        LivenessViolationClass::TerminalStateHasExits,
        LivenessViolationClass::UnreachableStateContradicted,
        LivenessViolationClass::UnknownTargetState,
        LivenessViolationClass::NoAutomatedPathToTerminal,
    ];

    fn label(&self) -> &'static str {
        match self {
            LivenessViolationClass::EmptyStateSpace => "empty_state_space",
            LivenessViolationClass::StateSpaceOverBudget => "state_space_over_budget",
            LivenessViolationClass::DeadEnd => "dead_end",
            LivenessViolationClass::ManualOnlyExit => "manual_only_exit",
            LivenessViolationClass::TerminalStateHasExits => "terminal_state_has_exits",
            LivenessViolationClass::UnreachableStateContradicted => {
                "unreachable_state_contradicted"
            }
            LivenessViolationClass::UnknownTargetState => "unknown_target_state",
            LivenessViolationClass::NoAutomatedPathToTerminal => "no_automated_path_to_terminal",
        }
    }
}

/// One dead-end (or table-integrity) finding, naming the state.
///
/// **Concerns:** C3, C8 (typed class), C14 (the `state` + `detail` pair is a
/// context capsule: enough to act on, never a metric dimension).
///
/// **Contract test:** `dead_end_failure_names_the_state`.
///
/// `state` is the `Debug` rendering of the offending state rather than the state
/// itself, deliberately: the violation is a **report**, not a resumption point,
/// and a non-generic violation can be collected across differently-typed tables
/// and parked in a findings ledger without the ledger becoming generic too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivenessViolation {
    /// The typed failure class.
    pub class: LivenessViolationClass,
    /// `Debug` rendering of the state the finding is about (`<table>` for
    /// whole-table findings).
    pub state: String,
    /// Human-readable context: what was enumerated, and what the cure looks
    /// like.
    pub detail: String,
}

impl core::fmt::Display for LivenessViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{}] state `{}`: {}",
            self.class.label(),
            self.state,
            self.detail
        )
    }
}

impl std::error::Error for LivenessViolation {}

/// Every liveness finding for one table, plus what the harness measured.
///
/// **Concerns:** C3, C14.
///
/// **Forcing incident:** the 2026-08-02 deadlock was **four** walls, each only
/// observable after the previous was cured — five serial waves of production
/// debugging. A harness that reported the first dead end and stopped would have
/// reproduced that serial rediscovery inside the test suite. This type reports
/// them all at once, which is the entire reason a harness beats a bare `assert`.
///
/// **Contract test:** `four_wall_deadlock_reports_every_wall_at_once`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivenessFailure {
    /// The findings, capped at [`LivenessBudget::max_violations_reported`],
    /// ordered most-structural-first (table integrity, then per-state exits,
    /// then composition).
    pub violations: Vec<LivenessViolation>,
    /// How many findings there were in total, before the cap.
    pub total_violations: usize,
    /// What the harness measured before failing.
    pub report: LivenessReport,
}

impl LivenessFailure {
    /// `true` iff any finding carries `class`.
    ///
    /// **Concerns:** C3.
    ///
    /// **Contract test:** `dead_end_failure_names_the_state`.
    #[must_use]
    pub fn has_class(&self, class: LivenessViolationClass) -> bool {
        self.violations.iter().any(|v| v.class == class)
    }

    fn single(
        class: LivenessViolationClass,
        state: &str,
        detail: String,
        report: LivenessReport,
    ) -> Self {
        Self {
            violations: vec![LivenessViolation {
                class,
                state: state.to_string(),
                detail,
            }],
            total_violations: 1,
            report,
        }
    }
}

impl core::fmt::Display for LivenessFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "{} liveness violation(s){}",
            self.total_violations,
            if self.violations.len() < self.total_violations {
                format!(" (showing the first {})", self.violations.len())
            } else {
                String::new()
            }
        )?;
        for (n, v) in self.violations.iter().enumerate() {
            writeln!(f, "  {}. {v}", n + 1)?;
        }
        write!(f, "  {}", self.report)
    }
}

impl std::error::Error for LivenessFailure {}

/// Check a state table for liveness with the default budget.
///
/// **Concerns:** C3 (the guarantee), C6a (bounded by
/// [`LivenessBudget::default`]).
///
/// **Forcing incident:** the 2026-08-02 four-wall deadlock and the 2026-07-11
/// over-broad GapFill guard — see the module docs. The law this function
/// enforces, verbatim from the canon: *"From every reachable non-terminal state,
/// at least one AUTOMATED transition is enabled. A human hand (or a deploy) as
/// the only exit is a liveness hole, not an escape hatch."*
///
/// **Contract test:** `cured_two_way_declared_table_passes`,
/// `four_wall_deadlock_reports_every_wall_at_once`,
/// `unreachable_dead_state_does_not_fail_the_table`.
///
/// # Arguments
///
/// - `states` — the enumerated state space, as plain data. Exhaustive by
///   construction: what you do not list is not checked.
/// - `classify` — [`StateClass`] per state. Both non-`Live` arms require a
///   written justification; see [`StateClass`].
/// - `transitions` — every move enabled *from* a state, with its [`Agency`].
///   Never called for a state classified [`StateClass::Unreachable`].
///
/// All three must be pure: the harness may call them in any order.
///
/// # Errors
///
/// Returns [`LivenessFailure`] listing **every** finding — dead ends,
/// manual-only exits, table-integrity problems, and (when targets are modeled)
/// states from which no automated path ever reaches a terminal state.
///
/// # Example
///
/// ```
/// use seam_contracts::harness::liveness::{check_liveness, StateClass, Transition};
///
/// #[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// enum Pin { Queued, InFlight, Exhausted, Filled }
///
/// let states = [Pin::Queued, Pin::InFlight, Pin::Exhausted, Pin::Filled];
///
/// let report = check_liveness(
///     &states,
///     |s| match s {
///         Pin::Filled => StateClass::Terminal("the bytes are here; nothing further is owed"),
///         _ => StateClass::Live,
///     },
///     |s| match s {
///         Pin::Queued => vec![Transition::automated("dial_advertiser").to(Pin::InFlight)],
///         Pin::InFlight => vec![
///             Transition::automated("bytes_verified").to(Pin::Filled),
///             Transition::automated("attempts_exhausted").to(Pin::Exhausted),
///         ],
///         // Without this arm the only exit was `operator DELETE /pins/{id}`.
///         Pin::Exhausted => vec![Transition::automated("backoff_retry").to(Pin::Queued)],
///         Pin::Filled => vec![],
///     },
/// )
/// .expect("every state has an automated move that eventually ends");
///
/// assert!(report.progress_checked);
/// ```
pub fn check_liveness<S, C, T>(
    states: &[S],
    classify: C,
    transitions: T,
) -> Result<LivenessReport, LivenessFailure>
where
    S: PartialEq + core::fmt::Debug,
    C: Fn(&S) -> StateClass,
    T: Fn(&S) -> Vec<Transition<S>>,
{
    check_liveness_with(states, classify, transitions, LivenessBudget::default())
}

/// Check a state table for liveness against an explicit budget.
///
/// **Concerns:** C3, C6a, C7.
///
/// **Contract test:** `state_space_over_budget_is_caught`,
/// `violation_report_is_capped_and_says_so`.
///
/// See [`check_liveness`] for the arguments and the law. The findings are
/// ordered most-structural-first: a broken table (unknown targets, contradicted
/// unreachability) is reported before the liveness holes that reading it would
/// produce, because a graph-derived answer over a broken table is a guess.
///
/// # Errors
///
/// Returns [`LivenessFailure`] listing every finding, capped for rendering at
/// [`LivenessBudget::max_violations_reported`] with the true total preserved.
pub fn check_liveness_with<S, C, T>(
    states: &[S],
    classify: C,
    transitions: T,
    budget: LivenessBudget,
) -> Result<LivenessReport, LivenessFailure>
where
    S: PartialEq + core::fmt::Debug,
    C: Fn(&S) -> StateClass,
    T: Fn(&S) -> Vec<Transition<S>>,
{
    if states.is_empty() {
        return Err(LivenessFailure::single(
            LivenessViolationClass::EmptyStateSpace,
            "<table>",
            "the state table enumerates nothing, so every liveness claim over it is vacuously \
             true. An empty table is not a proof of liveness; it is an instrument with no input. \
             Enumerate the predicate's actual corners (a boolean input space is 2^n rows and is \
             meant to be written out)."
                .to_string(),
            LivenessReport::empty(),
        ));
    }

    if states.len() > budget.max_states {
        return Err(LivenessFailure::single(
            LivenessViolationClass::StateSpaceOverBudget,
            "<table>",
            format!(
                "the table enumerates {} state(s), over the declared ceiling of {}. A liveness \
                 check has no honest sampled mode — the one state left out is the one that \
                 deadlocks — so this is a hard stop, not a degradation. Either raise \
                 LivenessBudget::max_states deliberately, or decompose the predicate: a decision \
                 needing this many independent inputs is asking to be split.",
                states.len(),
                budget.max_states
            ),
            LivenessReport::empty(),
        ));
    }

    let classes: Vec<StateClass> = states.iter().map(&classify).collect();
    let outs: Vec<Vec<Transition<S>>> = states
        .iter()
        .zip(classes.iter())
        .map(|(state, class)| {
            if class.is_unreachable() {
                Vec::new()
            } else {
                transitions(state)
            }
        })
        .collect();

    let live_states = classes.iter().filter(|c| c.is_live()).count();
    let terminal_states = classes.iter().filter(|c| c.is_terminal()).count();
    let unreachable_states = classes.iter().filter(|c| c.is_unreachable()).count();
    let transitions_enumerated: usize = outs.iter().map(Vec::len).sum();

    let mut violations: Vec<LivenessViolation> = Vec::new();

    // Pass 1 — table integrity. A graph-derived answer over a broken table is a
    // guess, so these are reported first and they disable the composition check.
    let mut has_unmodeled_targets = false;
    let mut has_unknown_targets = false;
    for (i, outgoing) in outs.iter().enumerate() {
        for t in outgoing {
            let Some(target) = t.to.as_ref() else {
                has_unmodeled_targets = true;
                continue;
            };
            match index_of(states, target) {
                None => {
                    has_unknown_targets = true;
                    violations.push(LivenessViolation {
                        class: LivenessViolationClass::UnknownTargetState,
                        state: format!("{:?}", states[i]),
                        detail: format!(
                            "transition `{}` lands on {target:?}, which the table does not \
                             enumerate. The table advertises a state space it does not serve \
                             (C7): either add the state, or the predicate has a corner nobody \
                             has classified — which is where deadlocks live.",
                            t.label
                        ),
                    });
                }
                Some(j) if classes[j].is_unreachable() => {
                    violations.push(LivenessViolation {
                        class: LivenessViolationClass::UnreachableStateContradicted,
                        state: format!("{:?}", states[j]),
                        detail: format!(
                            "declared Unreachable, but transition `{}` from {:?} lands on it. The \
                             justification that excused this state from the liveness analysis is \
                             false; reclassify it Live (and answer the liveness question for it) \
                             or remove the transition.",
                            t.label, states[i]
                        ),
                    });
                }
                Some(_) => {}
            }
        }
    }

    // Pass 2 — per-state exits. This is the law itself.
    let mut reported = vec![false; states.len()];
    for (i, outgoing) in outs.iter().enumerate() {
        match classes[i] {
            StateClass::Unreachable(_) => {}
            StateClass::Terminal(_) => {
                let exits: Vec<&Transition<S>> =
                    outgoing.iter().filter(|t| !t.targets(&states[i])).collect();
                if !exits.is_empty() {
                    violations.push(LivenessViolation {
                        class: LivenessViolationClass::TerminalStateHasExits,
                        state: format!("{:?}", states[i]),
                        detail: format!(
                            "declared Terminal, yet {} exit(s) leave it: {}. If something can move \
                             it, it was never an end — it is a stall that whoever owns that \
                             transition rescues (the 2026-07-31 exhausted-pin shape). Classify it \
                             Live and let this harness tell you whether its exits are automated.",
                            exits.len(),
                            render_transitions(&exits)
                        ),
                    });
                }
            }
            StateClass::Live => {
                if outgoing.is_empty() {
                    reported[i] = true;
                    violations.push(LivenessViolation {
                        class: LivenessViolationClass::DeadEnd,
                        state: format!("{:?}", states[i]),
                        detail: "reachable, non-terminal, and enumerates NO transition at all — \
                                 nothing moves it, by anyone. Whatever guard set produced this \
                                 corner, each of its rules is probably correct in isolation; it is \
                                 the conjunction that left no move (the 2026-08-02 four-wall \
                                 shape). The cure adds a transition, it does not relax a guard."
                            .to_string(),
                    });
                } else if !outgoing.iter().any(|t| t.agency.is_automated()) {
                    reported[i] = true;
                    violations.push(LivenessViolation {
                        class: LivenessViolationClass::ManualOnlyExit,
                        state: format!("{:?}", states[i]),
                        detail: format!(
                            "reachable, non-terminal, and every exit needs a hand: {}. The state \
                             machine is COMPLETE and the fleet is still stuck — a transition only \
                             counts as liveness if the fleet can take it. A human hand (or a \
                             deploy) as the only exit is a liveness hole, not an escape hatch.",
                            render_transitions(&outgoing.iter().collect::<Vec<_>>())
                        ),
                    });
                }
            }
        }
    }

    // Pass 3 — composition. Per-state legality is what four locally-correct
    // guards each pass; progress is what their conjunction lost.
    let progress_skipped = if terminal_states == 0 {
        Some(ProgressSkip::NoTerminalState)
    } else if has_unmodeled_targets || has_unknown_targets {
        Some(ProgressSkip::UnmodeledTargets)
    } else {
        None
    };

    if progress_skipped.is_none() {
        let can_finish = automated_finish_closure(states, &classes, &outs);
        for (i, class) in classes.iter().enumerate() {
            if !class.is_live() || can_finish[i] || reported[i] {
                continue;
            }
            let trapped = automated_trap_set(states, &outs, &can_finish, i);
            violations.push(LivenessViolation {
                class: LivenessViolationClass::NoAutomatedPathToTerminal,
                state: format!("{:?}", states[i]),
                detail: format!(
                    "every move out of it is legal and automated, and NO sequence of them ever \
                     reaches a terminal state. Trapped together with it: {}. This is the shape \
                     that reads as health on a dashboard — work happening, nothing finishing (the \
                     2026-08-01 fleet limit cycle: ~35k heal operations/hour, zero convergence). \
                     The cure is an automated transition OUT of the trap, not more throughput \
                     inside it.",
                    render_states(states, &trapped)
                ),
            });
        }
    }

    let report = LivenessReport {
        states_checked: states.len(),
        live_states,
        terminal_states,
        unreachable_states,
        transitions_enumerated,
        progress_checked: progress_skipped.is_none(),
        progress_skipped,
    };

    if violations.is_empty() {
        return Ok(report);
    }

    let total_violations = violations.len();
    violations.truncate(budget.max_violations_reported);
    Err(LivenessFailure {
        violations,
        total_violations,
        report,
    })
}

/// Panicking form of [`check_liveness`], for use inside `#[test]`.
///
/// **Concerns:** C3.
///
/// **Contract test:** `assert_form_panics_on_violation`.
///
/// # Panics
///
/// If any state fails the liveness law or the table fails its integrity checks.
/// The panic message renders **every** finding, so one run names every wall.
pub fn assert_liveness<S, C, T>(states: &[S], classify: C, transitions: T)
where
    S: PartialEq + core::fmt::Debug,
    C: Fn(&S) -> StateClass,
    T: Fn(&S) -> Vec<Transition<S>>,
{
    if let Err(failure) = check_liveness(states, classify, transitions) {
        panic!("liveness contract (C3) failed: {failure}");
    }
}

fn index_of<S: PartialEq>(states: &[S], needle: &S) -> Option<usize> {
    states.iter().position(|s| s == needle)
}

fn render_transitions<S>(transitions: &[&Transition<S>]) -> String {
    transitions
        .iter()
        .map(|t| format!("`{}` [{}]", t.label, t.agency.label()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Up to eight state names — enough to see the shape of a trap, short enough to
/// read in a panic message.
fn render_states<S: core::fmt::Debug>(states: &[S], indices: &[usize]) -> String {
    const PREVIEW: usize = 8;
    if indices.is_empty() {
        return "(only itself)".to_string();
    }
    let shown = indices
        .iter()
        .take(PREVIEW)
        .map(|&i| format!("{:?}", states[i]))
        .collect::<Vec<_>>()
        .join(", ");
    if indices.len() > PREVIEW {
        format!("{shown}, … ({} more)", indices.len() - PREVIEW)
    } else {
        shown
    }
}

/// Least fixed point of "can reach a terminal state via automated transitions."
///
/// Seeded with the terminal states; a live state joins when one of its automated
/// transitions lands on a member. Unreachable states never join — a contradicted
/// `Unreachable` is already a reported violation, and letting it grant progress
/// would launder the contradiction into a green.
fn automated_finish_closure<S: PartialEq>(
    states: &[S],
    classes: &[StateClass],
    outs: &[Vec<Transition<S>>],
) -> Vec<bool> {
    let mut can_finish: Vec<bool> = classes.iter().map(|c| c.is_terminal()).collect();
    loop {
        let mut changed = false;
        for (i, class) in classes.iter().enumerate() {
            if can_finish[i] || !class.is_live() {
                continue;
            }
            let reaches = outs[i].iter().any(|t| {
                t.agency.is_automated()
                    && t.to
                        .as_ref()
                        .and_then(|target| index_of(states, target))
                        .is_some_and(|j| can_finish[j])
            });
            if reaches {
                can_finish[i] = true;
                changed = true;
            }
        }
        if !changed {
            return can_finish;
        }
    }
}

/// The other trapped states reachable from `from` via automated transitions —
/// the shape of the cycle, not just its entry point.
fn automated_trap_set<S: PartialEq>(
    states: &[S],
    outs: &[Vec<Transition<S>>],
    can_finish: &[bool],
    from: usize,
) -> Vec<usize> {
    let mut seen = vec![false; states.len()];
    seen[from] = true;
    let mut frontier = vec![from];
    let mut trapped = Vec::new();
    while let Some(i) = frontier.pop() {
        for t in &outs[i] {
            if !t.agency.is_automated() {
                continue;
            }
            let Some(j) = t.to.as_ref().and_then(|target| index_of(states, target)) else {
                continue;
            };
            if seen[j] || can_finish[j] {
                continue;
            }
            seen[j] = true;
            trapped.push(j);
            frontier.push(j);
        }
    }
    trapped.sort_unstable();
    trapped
}

#[cfg(test)]
mod liveness_tests {
    use super::*;

    // ---------------------------------------------------------------------
    // Toy 1 — the acquisition pin (the `192f77cd0` shape, 2026-07-31).
    //
    // A TOY, not a reconstruction: plan task P2.2 owns the regression
    // demonstration against the two real historical predicate sets. These
    // fixtures exercise the instrument; P2.2 aims it at history.
    // ---------------------------------------------------------------------

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Pin {
        Queued,
        InFlight,
        Exhausted,
        Filled,
    }

    const PINS: [Pin; 4] = [Pin::Queued, Pin::InFlight, Pin::Exhausted, Pin::Filled];

    fn classify_pin(s: &Pin) -> StateClass {
        match s {
            Pin::Filled => StateClass::Terminal("the bytes are local and verified; nothing owed"),
            _ => StateClass::Live,
        }
    }

    fn pin_moves_common(s: &Pin) -> Vec<Transition<Pin>> {
        match s {
            Pin::Queued => vec![Transition::automated("dial_advertiser").to(Pin::InFlight)],
            Pin::InFlight => vec![
                Transition::automated("bytes_verified").to(Pin::Filled),
                Transition::automated("attempts_exhausted").to(Pin::Exhausted),
            ],
            Pin::Filled => vec![],
            Pin::Exhausted => unreachable!("each fixture supplies its own Exhausted arm"),
        }
    }

    /// The defect: an exhausted pin has no move at all.
    fn pin_moves_dead_end(s: &Pin) -> Vec<Transition<Pin>> {
        match s {
            Pin::Exhausted => vec![],
            other => pin_moves_common(other),
        }
    }

    /// The defect as it actually shipped: the only exit was a human `DELETE`.
    fn pin_moves_manual_only(s: &Pin) -> Vec<Transition<Pin>> {
        match s {
            Pin::Exhausted => {
                vec![Transition::human("operator DELETE /acquisition/pins/{id}").to(Pin::Queued)]
            }
            other => pin_moves_common(other),
        }
    }

    /// The cure: a backoff retry the fleet takes by itself.
    fn pin_moves_cured(s: &Pin) -> Vec<Transition<Pin>> {
        match s {
            Pin::Exhausted => vec![
                Transition::automated("backoff_retry").to(Pin::Queued),
                Transition::human("operator DELETE /acquisition/pins/{id}").to(Pin::Queued),
            ],
            other => pin_moves_common(other),
        }
    }

    /// (a) A dead-end state FAILS, and the counterexample NAMES it.
    #[test]
    fn dead_end_failure_names_the_state() {
        let failure = check_liveness(&PINS, classify_pin, pin_moves_dead_end)
            .expect_err("an exhausted pin with no move at all is a dead end");

        assert!(failure.has_class(LivenessViolationClass::DeadEnd));
        let rendered = failure.to_string();
        assert!(
            rendered.contains("Exhausted"),
            "a harness that says 'liveness failed' without naming the state is a bare assert \
             with extra steps; got:\n{rendered}"
        );
        assert!(rendered.contains("dead_end"));
        // The counterexample carries the cure direction, not just the defect.
        assert!(rendered.contains("adds a transition"));
    }

    /// (b) The fixed toy PASSES.
    #[test]
    fn cured_table_passes() {
        let report = check_liveness(&PINS, classify_pin, pin_moves_cured)
            .expect("an automated backoff retry restores liveness");
        assert_eq!(report.states_checked, 4);
        assert_eq!(report.live_states, 3);
        assert_eq!(report.terminal_states, 1);
        assert!(
            report.progress_checked,
            "every transition models its target, so the composition check must run"
        );
    }

    #[test]
    fn manual_only_exit_is_caught() {
        let failure = check_liveness(&PINS, classify_pin, pin_moves_manual_only)
            .expect_err("a human DELETE is not an automated transition");
        assert!(failure.has_class(LivenessViolationClass::ManualOnlyExit));
        let rendered = failure.to_string();
        assert!(rendered.contains("operator DELETE /acquisition/pins/{id}"));
        assert!(
            rendered.contains("[human]"),
            "the counterexample must name the AGENCY — that is the whole distinction; got:\n\
             {rendered}"
        );
        assert!(
            !failure.has_class(LivenessViolationClass::DeadEnd),
            "a manual-only exit is a distinct finding from having no exit at all"
        );
    }

    #[test]
    fn deploy_only_exit_is_caught() {
        // The 2026-08-02 interim channel: the deploy declare-cycle. CI runs it,
        // which is exactly why it feels automated and is not.
        let failure = check_liveness(&PINS, classify_pin, |s: &Pin| match s {
            Pin::Exhausted => {
                vec![Transition::deploy("edge redeploy declare-cycle").to(Pin::Queued)]
            }
            other => pin_moves_common(other),
        })
        .expect_err("a deploy is not a move the running fleet can take");
        assert!(failure.has_class(LivenessViolationClass::ManualOnlyExit));
        assert!(failure.to_string().contains("[deploy]"));
    }

    // ---------------------------------------------------------------------
    // (c) The reachability qualifier.
    // ---------------------------------------------------------------------

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Row {
        Undeclared,
        Declared,
        /// Both a peer and this node declared different heads.
        TwoWayDeclared,
        Contested,
        Converged,
        /// Neither side holds the chain — a corner the code cannot construct.
        BothSidesMissing,
    }

    const ROWS: [Row; 6] = [
        Row::Undeclared,
        Row::Declared,
        Row::TwoWayDeclared,
        Row::Contested,
        Row::Converged,
        Row::BothSidesMissing,
    ];

    fn row_moves_cured(s: &Row) -> Vec<Transition<Row>> {
        match s {
            Row::Undeclared => vec![Transition::automated("adopt_peer").to(Row::Declared)],
            Row::Declared => vec![Transition::automated("election_obey").to(Row::Converged)],
            Row::TwoWayDeclared => vec![Transition::automated("contest_peer").to(Row::Contested)],
            Row::Contested => vec![Transition::automated("election_obey").to(Row::Converged)],
            Row::Converged => vec![],
            Row::BothSidesMissing => vec![],
        }
    }

    /// (c-i) An unreachable dead state does NOT fail the table.
    #[test]
    fn unreachable_dead_state_does_not_fail_the_table() {
        let report = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("the declared head matches the election"),
                Row::BothSidesMissing => StateClass::Unreachable(
                    "no storage arm can construct it: admission requires at least one side to \
                     hold the chain",
                ),
                _ => StateClass::Live,
            },
            row_moves_cured,
        )
        .expect("a state that cannot occur owes no automated exit");
        assert_eq!(report.unreachable_states, 1);
        assert_eq!(report.live_states, 4);
    }

    /// (c-ii) The SAME dead state, marked reachable, DOES fail.
    #[test]
    fn the_same_dead_state_marked_reachable_fails() {
        let failure = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("the declared head matches the election"),
                _ => StateClass::Live,
            },
            row_moves_cured,
        )
        .expect_err("once it is reachable, it owes an automated exit like every other state");
        assert!(failure.has_class(LivenessViolationClass::DeadEnd));
        assert!(failure.to_string().contains("BothSidesMissing"));
    }

    /// (c-iii) The `Unreachable` claim is cross-checked, not trusted.
    #[test]
    fn unreachability_contradicted_by_an_inbound_transition_is_caught() {
        let failure = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("the declared head matches the election"),
                Row::BothSidesMissing => {
                    StateClass::Unreachable("claimed unconstructable — but see the table")
                }
                _ => StateClass::Live,
            },
            |s: &Row| match s {
                // The admission arm demonstrably routes into it.
                Row::TwoWayDeclared => vec![
                    Transition::automated("contest_peer").to(Row::Contested),
                    Transition::automated("no_local_chain").to(Row::BothSidesMissing),
                ],
                other => row_moves_cured(other),
            },
        )
        .expect_err("a state something routes into is not unreachable");
        assert!(failure.has_class(LivenessViolationClass::UnreachableStateContradicted));
        let rendered = failure.to_string();
        assert!(rendered.contains("BothSidesMissing"));
        assert!(rendered.contains("no_local_chain"));
    }

    // ---------------------------------------------------------------------
    // The composition catch — the class no per-guard review can see.
    // ---------------------------------------------------------------------

    /// The pre-cure shape: four locally-correct refusals, no automated exit
    /// from the two-way-declared corner; only the deploy declare-cycle moved it.
    #[test]
    fn four_wall_deadlock_reports_every_wall_at_once() {
        let failure = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("the declared head matches the election"),
                Row::BothSidesMissing => StateClass::Unreachable("no storage arm can construct it"),
                _ => StateClass::Live,
            },
            |s: &Row| match s {
                Row::Undeclared => vec![Transition::automated("adopt_peer").to(Row::Declared)],
                // Wall 2: the stamp guard compared a NULLed wall clock, so
                // AdoptLocal was structurally unreachable.
                Row::Declared => vec![],
                // Walls 1/3/4: AdoptPeer unreachable once declared; the declare
                // fails the no-chain gate; the row dies at SkippedDeclared
                // before entering the candidate list. Only R1's interim channel
                // moved it.
                Row::TwoWayDeclared => {
                    vec![Transition::deploy("deploy declare-cycle").to(Row::Declared)]
                }
                Row::Contested => vec![Transition::automated("election_obey").to(Row::Converged)],
                Row::Converged => vec![],
                Row::BothSidesMissing => vec![],
            },
        )
        .expect_err("the composition of four correct refusals leaves no automated move");

        assert_eq!(
            failure.total_violations, 3,
            "one run must name EVERY wall — serial rediscovery is what cost five production \
             waves; got:\n{failure}"
        );
        assert!(failure.has_class(LivenessViolationClass::DeadEnd));
        assert!(failure.has_class(LivenessViolationClass::ManualOnlyExit));
        // The third finding is the one no per-state check could produce, and it
        // is the reason the composition pass exists: `Undeclared` has a legal
        // automated move, and it is still doomed, because the only place that
        // move lands is the dead-ended `Declared`. A liveness hole propagates
        // BACKWARD along automated edges — every upstream state inherits it.
        // Reporting only the walls themselves would understate the blast radius
        // exactly the way the live fleet understated it.
        assert!(failure.has_class(LivenessViolationClass::NoAutomatedPathToTerminal));
        let rendered = failure.to_string();
        assert!(rendered.contains("Declared"));
        assert!(rendered.contains("TwoWayDeclared"));
        assert!(rendered.contains("[deploy]"));
        assert!(
            rendered.contains("state `Undeclared`"),
            "the upstream state that inherits the deadlock must be named too; got:\n{rendered}"
        );
    }

    #[test]
    fn cured_two_way_declared_table_passes() {
        let report = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("the declared head matches the election"),
                Row::BothSidesMissing => StateClass::Unreachable("no storage arm can construct it"),
                _ => StateClass::Live,
            },
            row_moves_cured,
        )
        .expect("contest_peer + election_obey restore the automated path");
        assert!(report.progress_checked);
        assert_eq!(report.transitions_enumerated, 4);
    }

    /// Every move legal, every move automated, nothing ever ends — the
    /// 2026-08-01 limit cycle. Per-state checks pass; only the composition
    /// check sees it.
    #[test]
    fn limit_cycle_without_an_exit_is_caught() {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Gap {
            Discovered,
            Refused,
            Requeued,
            Healed,
        }
        let states = [Gap::Discovered, Gap::Refused, Gap::Requeued, Gap::Healed];

        let failure = check_liveness(
            &states,
            |s: &Gap| match s {
                Gap::Healed => StateClass::Terminal("bytes present and verified locally"),
                _ => StateClass::Live,
            },
            |s: &Gap| match s {
                Gap::Discovered => vec![Transition::automated("adjudicate").to(Gap::Refused)],
                Gap::Refused => vec![Transition::automated("re-sweep").to(Gap::Requeued)],
                Gap::Requeued => vec![Transition::automated("re-discover").to(Gap::Discovered)],
                Gap::Healed => vec![],
            },
        )
        .expect_err("35k legal operations per hour that never reach Healed is not liveness");

        assert!(failure.has_class(LivenessViolationClass::NoAutomatedPathToTerminal));
        assert_eq!(
            failure.total_violations, 3,
            "all three cycle members are trapped, and each is named"
        );
        let rendered = failure.to_string();
        assert!(rendered.contains("Refused"));
        assert!(
            rendered.contains("Trapped together with it"),
            "naming the trap SET is what turns 'a state is stuck' into 'here is the cycle'; \
             got:\n{rendered}"
        );
    }

    #[test]
    fn a_state_already_reported_dead_is_not_reported_twice() {
        // A dead end trivially cannot reach a terminal state; reporting both
        // classes for it would pad the count and bury the distinct defects.
        let failure =
            check_liveness(&PINS, classify_pin, pin_moves_dead_end).expect_err("dead end");
        assert_eq!(failure.total_violations, 1);
        assert!(!failure.has_class(LivenessViolationClass::NoAutomatedPathToTerminal));
    }

    // ---------------------------------------------------------------------
    // Table integrity + the instrument's own honesty.
    // ---------------------------------------------------------------------

    #[test]
    fn empty_state_space_is_caught() {
        let empty: [Pin; 0] = [];
        let failure = check_liveness(&empty, classify_pin, pin_moves_cured)
            .expect_err("an empty table proves nothing and must not read as green");
        assert!(failure.has_class(LivenessViolationClass::EmptyStateSpace));
        assert!(failure.to_string().contains("vacuously"));
    }

    #[test]
    fn state_space_over_budget_is_caught() {
        let failure = check_liveness_with(
            &PINS,
            classify_pin,
            pin_moves_cured,
            LivenessBudget {
                max_states: 3,
                ..LivenessBudget::default()
            },
        )
        .expect_err("4 states over a ceiling of 3");
        assert!(failure.has_class(LivenessViolationClass::StateSpaceOverBudget));
        assert!(
            failure.to_string().contains("no honest sampled mode"),
            "the message must say WHY this is a hard stop rather than a degradation"
        );
    }

    #[test]
    fn unknown_target_state_is_caught() {
        let failure = check_liveness(
            &[Pin::Queued, Pin::Filled],
            classify_pin,
            |s: &Pin| match s {
                // InFlight is not in the enumerated table.
                Pin::Queued => vec![Transition::automated("dial_advertiser").to(Pin::InFlight)],
                _ => vec![],
            },
        )
        .expect_err("a target the table does not enumerate breaks the table's own claim");
        assert!(failure.has_class(LivenessViolationClass::UnknownTargetState));
        assert!(failure.to_string().contains("InFlight"));
    }

    #[test]
    fn terminal_state_with_a_manual_exit_is_caught() {
        let failure = check_liveness(
            &PINS,
            |s: &Pin| match s {
                Pin::Filled => StateClass::Terminal("bytes present"),
                // Classifying the stall as an END is the escape hatch this
                // check closes.
                Pin::Exhausted => StateClass::Terminal("we gave up"),
                _ => StateClass::Live,
            },
            pin_moves_manual_only,
        )
        .expect_err("a terminal state a human still rescues was never terminal");
        assert!(failure.has_class(LivenessViolationClass::TerminalStateHasExits));
        assert!(failure.to_string().contains("never an end"));
    }

    #[test]
    fn terminal_self_loop_is_not_an_exit() {
        // A controller re-checking a settled state has not left it.
        let report = check_liveness(&PINS, classify_pin, |s: &Pin| match s {
            Pin::Filled => vec![Transition::automated("re-verify on sweep").to(Pin::Filled)],
            other => pin_moves_cured(other),
        })
        .expect("a self-loop on a terminal state is eager reconciliation, not an exit");
        assert!(report.progress_checked);
    }

    #[test]
    fn progress_check_is_skipped_without_a_terminal_state() {
        let report = check_liveness(
            &[Pin::Queued, Pin::InFlight],
            |_| StateClass::Live,
            |s: &Pin| match s {
                Pin::Queued => vec![Transition::automated("dial").to(Pin::InFlight)],
                _ => vec![Transition::automated("requeue").to(Pin::Queued)],
            },
        )
        .expect("per-state liveness holds");
        assert!(!report.progress_checked);
        assert_eq!(report.progress_skipped, Some(ProgressSkip::NoTerminalState));
        assert!(
            report.to_string().contains("SKIPPED (no_terminal_state)"),
            "skipped must never read as passed (C4 applied to the instrument)"
        );
    }

    #[test]
    fn progress_check_is_skipped_when_targets_are_unmodeled() {
        let report = check_liveness(&PINS, classify_pin, |s: &Pin| match s {
            // No `.to(..)` — the harness cannot close the graph.
            Pin::Exhausted => vec![Transition::automated("backoff_retry")],
            other => pin_moves_cured(other),
        })
        .expect("per-state liveness holds even when targets are unmodeled");
        assert!(!report.progress_checked);
        assert_eq!(
            report.progress_skipped,
            Some(ProgressSkip::UnmodeledTargets)
        );
    }

    #[test]
    fn violation_report_is_capped_and_says_so() {
        // Six dead ends, cap of two.
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        struct Corner(u8);
        let states: Vec<Corner> = (0..6).map(Corner).collect();
        let failure = check_liveness_with(
            &states,
            |_| StateClass::Live,
            |_| Vec::new(),
            LivenessBudget {
                max_violations_reported: 2,
                ..LivenessBudget::default()
            },
        )
        .expect_err("six dead ends");
        assert_eq!(failure.total_violations, 6);
        assert_eq!(failure.violations.len(), 2);
        assert!(
            failure.to_string().contains("showing the first 2"),
            "a truncated list must never read as a short list"
        );
    }

    #[test]
    fn report_counts_every_class_of_state() {
        let report = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("head matches election"),
                Row::BothSidesMissing => StateClass::Unreachable("unconstructable"),
                _ => StateClass::Live,
            },
            row_moves_cured,
        )
        .expect("cured");
        assert_eq!(report.states_checked, 6);
        assert_eq!(report.live_states, 4);
        assert_eq!(report.terminal_states, 1);
        assert_eq!(report.unreachable_states, 1);
        assert_eq!(report.transitions_enumerated, 4);
    }

    #[test]
    fn unreachable_states_never_have_their_transitions_enumerated() {
        use std::cell::Cell;
        let asked = Cell::new(0u32);
        let _ = check_liveness(
            &ROWS,
            |s: &Row| match s {
                Row::Converged => StateClass::Terminal("head matches election"),
                Row::BothSidesMissing => StateClass::Unreachable("unconstructable"),
                _ => StateClass::Live,
            },
            |s: &Row| {
                if matches!(s, Row::BothSidesMissing) {
                    asked.set(asked.get() + 1);
                }
                row_moves_cured(s)
            },
        );
        assert_eq!(
            asked.get(),
            0,
            "a state that cannot occur may not have well-defined moves; asking for them invites \
             a fixture that lies in the other direction"
        );
    }

    #[test]
    #[should_panic(expected = "liveness contract (C3) failed")]
    fn assert_form_panics_on_violation() {
        assert_liveness(&PINS, classify_pin, pin_moves_dead_end);
    }

    #[test]
    fn assert_form_is_silent_on_a_live_table() {
        assert_liveness(&PINS, classify_pin, pin_moves_cured);
    }

    // ---------------------------------------------------------------------
    // The instrument's own conformance.
    // ---------------------------------------------------------------------

    #[test]
    fn violation_class_is_conformant() {
        crate::assert_reason_labels_conformant::<LivenessViolationClass>();
        crate::assert_reason_labels_discriminating::<LivenessViolationClass>();
        crate::assert_reason_labels_stable::<LivenessViolationClass>(&[
            "empty_state_space",
            "state_space_over_budget",
            "dead_end",
            "manual_only_exit",
            "terminal_state_has_exits",
            "unreachable_state_contradicted",
            "unknown_target_state",
            "no_automated_path_to_terminal",
        ]);
    }

    #[test]
    fn agency_and_progress_skip_are_conformant() {
        crate::assert_reason_labels_conformant::<Agency>();
        crate::assert_reason_labels_discriminating::<Agency>();
        crate::assert_reason_labels_stable::<Agency>(&["automated", "human", "deploy"]);

        crate::assert_reason_labels_conformant::<ProgressSkip>();
        crate::assert_reason_labels_discriminating::<ProgressSkip>();
        crate::assert_reason_labels_stable::<ProgressSkip>(&[
            "no_terminal_state",
            "unmodeled_targets",
        ]);
    }

    #[test]
    fn agency_automated_is_the_only_automated_agency() {
        assert!(Agency::Automated.is_automated());
        assert!(!Agency::Human.is_automated());
        assert!(!Agency::Deploy.is_automated());
    }

    #[test]
    fn transition_constructors_carry_agency() {
        let a: Transition<Pin> = Transition::automated("sweep");
        let h: Transition<Pin> = Transition::human("operator delete");
        let d: Transition<Pin> = Transition::deploy("rollout");
        assert_eq!(a.agency, Agency::Automated);
        assert_eq!(h.agency, Agency::Human);
        assert_eq!(d.agency, Agency::Deploy);
        assert_eq!(a.to, None);
        assert_eq!(
            Transition::<Pin>::automated("sweep").to(Pin::Filled).to,
            Some(Pin::Filled)
        );
        assert!(Transition::automated("sweep")
            .to(Pin::Filled)
            .targets(&Pin::Filled));
    }

    #[test]
    fn state_class_predicates_partition_the_arms() {
        for (class, live, terminal, unreachable) in [
            (StateClass::Live, true, false, false),
            (StateClass::Terminal("done"), false, true, false),
            (StateClass::Unreachable("nope"), false, false, true),
        ] {
            assert_eq!(class.is_live(), live);
            assert_eq!(class.is_terminal(), terminal);
            assert_eq!(class.is_unreachable(), unreachable);
        }
    }

    #[test]
    fn violation_display_carries_class_state_and_detail() {
        let v = LivenessViolation {
            class: LivenessViolationClass::DeadEnd,
            state: "Exhausted".to_string(),
            detail: "no move".to_string(),
        };
        assert_eq!(v.to_string(), "[dead_end] state `Exhausted`: no move");
    }
}
