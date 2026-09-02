//! Pure process-exit and readiness classification.

use serde::{Deserialize, Serialize};

/// Outcome of one readiness polling attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessOutcome {
    /// The child has already exited, so polling must stop immediately.
    ChildExited,
    /// The child is alive and attempts remain.
    Retry,
    /// The child is alive but attempts are exhausted.
    GiveUp,
}

/// Chooses the next readiness action, preferring observed child death over
/// the remaining attempt budget.
///
/// Lifted from elohim-storage `process_manager.rs` (264ce8ce4); storage
/// delegates here in S1.
pub fn classify_readiness_outcome(
    child_exited: bool,
    attempt: u32,
    max_retries: u32,
) -> ReadinessOutcome {
    if child_exited {
        return ReadinessOutcome::ChildExited;
    }
    if attempt < max_retries {
        ReadinessOutcome::Retry
    } else {
        ReadinessOutcome::GiveUp
    }
}

/// A process's normalized termination cause.
///
/// KEPT rather than projected onto [`elohim_epr_rea::Magnitude::Classification`]: that is a
/// frame-ref into a *governed* classification atom, subject to judgement, while a termination
/// cause is a kernel fact with a closed set of shapes that no one may reclassify.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case", tag = "class")]
pub enum ExitClass {
    /// The process returned an exit code.
    Exited { code: i32 },
    /// The process was terminated by a signal.
    Signaled { signal: i32, core_dumped: bool },
    /// The supervisor identified an out-of-memory kill using runtime evidence.
    OomKilled,
    /// The wait status did not describe an exit or signal termination.
    Unknown,
}

impl ExitClass {
    /// Decodes a POSIX wait status using the wait-macro bit layout.
    ///
    /// This pure decoder never produces [`Self::OomKilled`]. The supervisor
    /// promotes SIGKILL when `/proc` or cgroup evidence identifies an OOM kill.
    pub fn from_raw_wait_status(status: i32) -> Self {
        let signal = status & 0x7f;
        if signal == 0 {
            return Self::Exited {
                code: (status >> 8) & 0xff,
            };
        }

        if signal != 0x7f {
            return Self::Signaled {
                signal,
                core_dumped: status & 0x80 != 0,
            };
        }

        Self::Unknown
    }

    /// Returns true only for a zero exit code.
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Exited { code: 0 })
    }

    /// Returns the stable token used to compare repeated termination causes.
    pub fn same_cause_token(&self) -> String {
        match self {
            Self::Exited { code } => format!("exited:{code}"),
            Self::Signaled { signal, .. } => format!("signaled:{signal}"),
            Self::OomKilled => "oom".to_string(),
            Self::Unknown => "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_outcome_prefers_child_death_over_attempt_budget() {
        // Table test: (child_exited, attempt, max_retries) -> expected outcome.
        // A dead child always wins immediately, even on attempt 1 of a large
        // budget — that is the entire point of the fix (never wait out a
        // dead child's retry window).
        let cases: &[(bool, u32, u32, ReadinessOutcome)] = &[
            // Child alive, attempts remain: retry.
            (false, 1, 60, ReadinessOutcome::Retry),
            (false, 59, 60, ReadinessOutcome::Retry),
            // Child alive, attempts exhausted: give up (but child_alive is
            // reported true by the caller).
            (false, 60, 60, ReadinessOutcome::GiveUp),
            // Child exited: stop immediately regardless of how many attempts
            // remain — including the very first attempt.
            (true, 1, 60, ReadinessOutcome::ChildExited),
            (true, 30, 60, ReadinessOutcome::ChildExited),
            (true, 60, 60, ReadinessOutcome::ChildExited),
        ];

        for (child_exited, attempt, max_retries, expected) in cases.iter().copied() {
            assert_eq!(
                classify_readiness_outcome(child_exited, attempt, max_retries),
                expected,
                "child_exited={child_exited:?} attempt={attempt} max_retries={max_retries}"
            );
        }
    }

    #[test]
    fn exit_class_decodes_posix_wait_status() {
        // POSIX wait status encoding: exit code in bits 8..16; signal in bits 0..7; core in bit 7.
        assert_eq!(
            ExitClass::from_raw_wait_status(0),
            ExitClass::Exited { code: 0 }
        );
        assert_eq!(
            ExitClass::from_raw_wait_status(1 << 8),
            ExitClass::Exited { code: 1 }
        );
        assert_eq!(
            ExitClass::from_raw_wait_status(9),
            ExitClass::Signaled {
                signal: 9,
                core_dumped: false
            }
        );
        assert_eq!(
            ExitClass::from_raw_wait_status(11 | 0x80),
            ExitClass::Signaled {
                signal: 11,
                core_dumped: true
            }
        );
        assert!(ExitClass::Exited { code: 0 }.is_clean());
        assert!(!ExitClass::Signaled {
            signal: 9,
            core_dumped: false
        }
        .is_clean());
        assert_eq!(
            ExitClass::Signaled {
                signal: 9,
                core_dumped: false
            }
            .same_cause_token(),
            "signaled:9"
        );
        assert_eq!(ExitClass::OomKilled.same_cause_token(), "oom");
    }

    #[test]
    fn exit_class_serde_is_tagged_kebab() {
        let j = serde_json::to_string(&ExitClass::Signaled {
            signal: 9,
            core_dumped: false,
        })
        .unwrap();
        assert_eq!(j, r#"{"class":"signaled","signal":9,"core_dumped":false}"#);
    }
}
