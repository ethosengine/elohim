//! The pure lifecycle state machine — filled by Task 6.
//!
//! KEPT rather than projected onto [`elohim_epr_rea::model::CommitmentState`]: that is the
//! lifecycle of a PROMISE (proposed → active → fulfilled → revoked), while this is the
//! lifecycle of a running child (idle → spawning → booting → live → dying → dead). The
//! transitions that are economically meaningful already leave through [`crate::rea`] as
//! intents and events.

use crate::{exit::ExitClass, intent::IntentAction, verdict::RestartVerdict};

/// Pure lifecycle state of one child process.
#[derive(Clone, Debug, PartialEq)]
pub enum ChildState {
    /// No child is running or being started.
    Idle,
    /// A spawn is in progress for this restart attempt.
    Spawning { attempt: u32 },
    /// A child is climbing its readiness ladder.
    Booting { pid: u32, rung: usize },
    /// A child has completed readiness.
    Live { pid: u32 },
    /// A child is expected to terminate.
    Dying { pid: u32, since_epoch_ms: u64 },
    /// A child died unexpectedly and awaits a restart verdict.
    Dead,
    /// Restart policy permanently stopped retrying the child.
    GaveUp,
}

/// External observation or request applied to a [`ChildState`].
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Begin the first spawn.
    SpawnRequested,
    /// The child was spawned with this operating-system process identifier.
    Spawned { pid: u32 },
    /// One readiness rung passed out of the declared total.
    RungPassed { rung: usize, of: usize },
    /// One readiness rung exhausted its patience budget.
    RungTimedOut { rung: usize },
    /// The child exited with this normalized cause.
    Died { class: ExitClass },
    /// Request graceful shutdown with the policy-selected signal.
    StopRequested { signal: i32 },
    /// The graceful shutdown period elapsed.
    GraceExpired,
    /// Restart policy reached a verdict for the latest death.
    VerdictReached { verdict: RestartVerdict },
}

/// Side effect requested by a pure lifecycle transition.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Persist the intent before performing its process action.
    RecordIntent(IntentAction),
    /// Spawn the child.
    Spawn,
    /// Open an incident if the supervisor has none open.
    OpenIncident,
    /// Persist the death witness before deciding what follows.
    WriteWitness,
    /// Ask the restart governor for a verdict.
    Decide,
    /// Delay before continuing, in seconds.
    SleepThen(u64),
    /// Send a graceful-shutdown signal.
    SendSignal(i32),
    /// Forcefully terminate the child.
    Kill,
    /// Close the current incident with this outcome.
    CloseIncident(IncidentCloseKind),
    /// Mark the child ready.
    MarkReady,
    /// Exit the per-child supervisor loop.
    Exit,
}

/// Terminal outcome requested for the current incident.
#[derive(Clone, Debug, PartialEq)]
pub enum IncidentCloseKind {
    /// The child completed readiness after recovery.
    ReadyAgain,
    /// Restart policy permanently gave up.
    GaveUp,
    /// The child stopped intentionally.
    Stopped,
}

/// Applies one event to one child state without performing I/O or reading a clock.
pub fn step(state: ChildState, event: Event) -> (ChildState, Vec<Action>) {
    match (state, event) {
        (ChildState::Idle, Event::SpawnRequested) => (
            ChildState::Spawning { attempt: 0 },
            vec![Action::RecordIntent(IntentAction::Spawn), Action::Spawn],
        ),
        (ChildState::Spawning { .. }, Event::Spawned { pid }) => {
            (ChildState::Booting { pid, rung: 0 }, vec![])
        }
        (ChildState::Booting { pid, .. }, Event::RungPassed { rung, of }) => {
            let next_rung = rung.saturating_add(1);
            if next_rung < of {
                (
                    ChildState::Booting {
                        pid,
                        rung: next_rung,
                    },
                    vec![],
                )
            } else {
                (
                    ChildState::Live { pid },
                    vec![
                        Action::MarkReady,
                        Action::CloseIncident(IncidentCloseKind::ReadyAgain),
                    ],
                )
            }
        }
        (ChildState::Booting { pid, .. }, Event::RungTimedOut { rung: _ }) => (
            ChildState::Dying {
                pid,
                since_epoch_ms: 0,
            },
            vec![Action::RecordIntent(IntentAction::Kill), Action::Kill],
        ),
        (ChildState::Booting { .. } | ChildState::Live { .. }, Event::Died { class: _ }) => (
            ChildState::Dead,
            vec![Action::OpenIncident, Action::WriteWitness, Action::Decide],
        ),
        (
            ChildState::Dead,
            Event::VerdictReached {
                verdict: RestartVerdict::Restart { after_s, attempt },
            },
        ) => (
            ChildState::Spawning { attempt },
            vec![
                Action::RecordIntent(IntentAction::Restart { attempt, after_s }),
                Action::SleepThen(after_s),
                Action::Spawn,
            ],
        ),
        (
            ChildState::Dead,
            Event::VerdictReached {
                verdict: RestartVerdict::GiveUp { .. },
            },
        ) => (
            ChildState::GaveUp,
            vec![
                Action::RecordIntent(IntentAction::GiveUp),
                Action::CloseIncident(IncidentCloseKind::GaveUp),
            ],
        ),
        (
            ChildState::Dead,
            Event::VerdictReached {
                verdict: RestartVerdict::Stop,
            },
        ) => (
            ChildState::Idle,
            vec![Action::CloseIncident(IncidentCloseKind::Stopped)],
        ),
        (
            ChildState::Booting { pid, .. } | ChildState::Live { pid },
            Event::StopRequested { signal },
        ) => (
            ChildState::Dying {
                pid,
                since_epoch_ms: 0,
            },
            vec![
                Action::RecordIntent(IntentAction::Stop {
                    signal,
                    grace_ms: 0,
                }),
                Action::SendSignal(signal),
            ],
        ),
        (ChildState::Dying { .. }, Event::Died { class: _ }) => (
            ChildState::Idle,
            vec![
                Action::CloseIncident(IncidentCloseKind::Stopped),
                Action::Exit,
            ],
        ),
        (state @ ChildState::Dying { .. }, Event::GraceExpired) => (state, vec![Action::Kill]),
        (state, _) => (state, vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GiveUpReason;

    #[test]
    fn idle_spawn_requested_records_intent_then_spawns() {
        assert_eq!(
            step(ChildState::Idle, Event::SpawnRequested),
            (
                ChildState::Spawning { attempt: 0 },
                vec![Action::RecordIntent(IntentAction::Spawn), Action::Spawn,],
            )
        );
    }

    #[test]
    fn spawned_child_starts_the_boot_ladder() {
        assert_eq!(
            step(
                ChildState::Spawning { attempt: 3 },
                Event::Spawned { pid: 42 },
            ),
            (ChildState::Booting { pid: 42, rung: 0 }, vec![])
        );
    }

    #[test]
    fn rung_pass_advances_or_marks_live_on_the_last_rung() {
        assert_eq!(
            step(
                ChildState::Booting { pid: 42, rung: 0 },
                Event::RungPassed { rung: 0, of: 2 },
            ),
            (ChildState::Booting { pid: 42, rung: 1 }, vec![])
        );
        assert_eq!(
            step(
                ChildState::Booting { pid: 42, rung: 1 },
                Event::RungPassed { rung: 1, of: 2 },
            ),
            (
                ChildState::Live { pid: 42 },
                vec![
                    Action::MarkReady,
                    Action::CloseIncident(IncidentCloseKind::ReadyAgain),
                ],
            )
        );
    }

    #[test]
    fn rung_timeout_records_kill_and_enters_dying() {
        assert_eq!(
            step(
                ChildState::Booting { pid: 42, rung: 1 },
                Event::RungTimedOut { rung: 1 },
            ),
            (
                ChildState::Dying {
                    pid: 42,
                    since_epoch_ms: 0,
                },
                vec![Action::RecordIntent(IntentAction::Kill), Action::Kill,],
            )
        );
    }

    #[test]
    fn booting_or_live_death_opens_incident_writes_witness_then_decides() {
        let event = Event::Died {
            class: ExitClass::Exited { code: 1 },
        };
        let expected_actions = vec![Action::OpenIncident, Action::WriteWitness, Action::Decide];

        assert_eq!(
            step(ChildState::Booting { pid: 42, rung: 1 }, event.clone()),
            (ChildState::Dead, expected_actions.clone())
        );
        assert_eq!(
            step(ChildState::Live { pid: 42 }, event),
            (ChildState::Dead, expected_actions)
        );
    }

    #[test]
    fn restart_verdict_records_intent_sleeps_then_spawns() {
        assert_eq!(
            step(
                ChildState::Dead,
                Event::VerdictReached {
                    verdict: RestartVerdict::Restart {
                        after_s: 8,
                        attempt: 3,
                    },
                },
            ),
            (
                ChildState::Spawning { attempt: 3 },
                vec![
                    Action::RecordIntent(IntentAction::Restart {
                        attempt: 3,
                        after_s: 8,
                    }),
                    Action::SleepThen(8),
                    Action::Spawn,
                ],
            )
        );
    }

    #[test]
    fn give_up_verdict_records_intent_and_closes_incident() {
        assert_eq!(
            step(
                ChildState::Dead,
                Event::VerdictReached {
                    verdict: RestartVerdict::GiveUp {
                        reason: GiveUpReason::PolicyTemporary,
                    },
                },
            ),
            (
                ChildState::GaveUp,
                vec![
                    Action::RecordIntent(IntentAction::GiveUp),
                    Action::CloseIncident(IncidentCloseKind::GaveUp),
                ],
            )
        );
    }

    #[test]
    fn stop_verdict_closes_incident_and_returns_idle() {
        assert_eq!(
            step(
                ChildState::Dead,
                Event::VerdictReached {
                    verdict: RestartVerdict::Stop,
                },
            ),
            (
                ChildState::Idle,
                vec![Action::CloseIncident(IncidentCloseKind::Stopped)],
            )
        );
    }

    #[test]
    fn stop_request_uses_carried_signal_for_booting_or_live_child() {
        for state in [
            ChildState::Booting { pid: 42, rung: 1 },
            ChildState::Live { pid: 42 },
        ] {
            let (next, actions) = step(state, Event::StopRequested { signal: 15 });

            assert_eq!(
                next,
                ChildState::Dying {
                    pid: 42,
                    since_epoch_ms: 0,
                }
            );
            assert_eq!(actions.len(), 2);
            assert!(matches!(
                &actions[0],
                Action::RecordIntent(IntentAction::Stop { signal: 15, .. })
            ));
            assert_eq!(actions[1], Action::SendSignal(15));
        }
    }

    #[test]
    fn dying_child_death_closes_stopped_incident_and_exits() {
        assert_eq!(
            step(
                ChildState::Dying {
                    pid: 42,
                    since_epoch_ms: 1_000,
                },
                Event::Died {
                    class: ExitClass::Signaled {
                        signal: 2,
                        core_dumped: false,
                    },
                },
            ),
            (
                ChildState::Idle,
                vec![
                    Action::CloseIncident(IncidentCloseKind::Stopped),
                    Action::Exit,
                ],
            )
        );
    }

    #[test]
    fn grace_expiry_kills_and_remains_dying() {
        let state = ChildState::Dying {
            pid: 42,
            since_epoch_ms: 1_000,
        };

        assert_eq!(
            step(state.clone(), Event::GraceExpired),
            (state, vec![Action::Kill])
        );
    }

    #[test]
    fn illegal_transitions_are_no_ops() {
        assert_eq!(
            step(
                ChildState::Idle,
                Event::Died {
                    class: ExitClass::Exited { code: 1 },
                },
            ),
            (ChildState::Idle, vec![])
        );
    }
}
