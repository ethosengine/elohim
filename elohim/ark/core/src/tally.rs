//! The persisted child-death tally.

use serde::{Deserialize, Serialize};

use crate::ExitClass;

const MAX_RETAINED_DEATHS: usize = 256;

/// One normalized child death retained across ark restarts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DeathRecord {
    /// Wall-clock time of the death in Unix seconds.
    pub at_epoch_s: u64,
    /// Normalized process termination cause.
    pub class: ExitClass,
    /// How long the child lived before this death.
    pub uptime_ms: u64,
    /// First structured standard-error line, when one was observed.
    pub first_stderr_line: Option<String>,
}

/// Persisted history used by the restart governor.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default, rename_all = "snake_case")]
pub struct DeathTally {
    /// Child deaths in observation order.
    pub deaths: Vec<DeathRecord>,
}

impl DeathTally {
    /// Appends a newly observed death, retaining only the most recent 256.
    ///
    /// This bound exceeds every S0 expressible intensity-window and
    /// same-cause limit while keeping the persisted projection finite.
    pub fn record(&mut self, d: DeathRecord) {
        self.deaths.push(d);
        let excess = self.deaths.len().saturating_sub(MAX_RETAINED_DEATHS);
        if excess > 0 {
            self.deaths.drain(..excess);
        }
    }

    /// Counts deaths in the inclusive sliding window ending at `now_epoch_s`.
    pub fn deaths_within(&self, now_epoch_s: u64, window_s: u64) -> u32 {
        let window_start = now_epoch_s.saturating_sub(window_s);
        let count = self
            .deaths
            .iter()
            .filter(|death| death.at_epoch_s >= window_start && death.at_epoch_s <= now_epoch_s)
            .count();
        count.min(u32::MAX as usize) as u32
    }

    /// Returns the trailing number of deaths with the same normalized cause.
    pub fn same_cause_run(&self) -> u32 {
        let Some(last) = self.deaths.last() else {
            return 0;
        };
        let last_key = same_cause_key(last);
        let count = self
            .deaths
            .iter()
            .rev()
            .take_while(|death| same_cause_key(death) == last_key)
            .count();
        count.min(u32::MAX as usize) as u32
    }

    /// Clears the restart-intensity history after the child becomes ready.
    pub fn reset_on_ready(&mut self) {
        self.deaths.clear();
    }
}

/// Builds the stable key used to coalesce repeated termination causes.
pub fn same_cause_key(d: &DeathRecord) -> String {
    format!(
        "{}|{}|fast:{}",
        d.class.same_cause_token(),
        d.first_stderr_line.as_deref().unwrap_or_default(),
        d.uptime_ms < 5_000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn death(at_epoch_s: u64, class: ExitClass) -> DeathRecord {
        DeathRecord {
            at_epoch_s,
            class,
            uptime_ms: 1_000,
            first_stderr_line: None,
        }
    }

    #[test]
    fn intensity_window_counts_only_recent_deaths_and_readiness_resets_it() {
        let mut tally = DeathTally::default();
        for at_epoch_s in [0, 100, 200, 300, 400, 600] {
            tally.record(death(at_epoch_s, ExitClass::Exited { code: 1 }));
        }

        assert_eq!(tally.deaths_within(600, 300), 3);
        tally.reset_on_ready();
        assert_eq!(tally.deaths_within(600, 300), 0);
    }

    #[test]
    fn oom_killed_deaths_share_the_same_cause_key() {
        let first = death(10, ExitClass::OomKilled);
        let second = death(20, ExitClass::OomKilled);

        assert_eq!(same_cause_key(&first), same_cause_key(&second));
        assert!(same_cause_key(&first).starts_with("oom|"));
    }

    #[test]
    fn tally_retains_only_the_most_recent_256_deaths() {
        let mut tally = DeathTally::default();
        for at_epoch_s in 0..=256 {
            tally.record(death(at_epoch_s, ExitClass::Exited { code: 1 }));
        }

        assert_eq!(tally.deaths.len(), 256);
        assert_eq!(tally.deaths.first().unwrap().at_epoch_s, 1);
        assert_eq!(tally.deaths.last().unwrap().at_epoch_s, 256);
    }
}
