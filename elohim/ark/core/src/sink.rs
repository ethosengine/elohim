//! Pure boundaries for time and durable ark records.

use crate::{DeathTally, DeathWitness, Incident, Intent, Passport, WitnessError};

/// Time source supplied by the I/O supervisor.
pub trait Clock {
    /// Returns Unix time in milliseconds.
    fn now_epoch_ms(&self) -> u64;
}

/// Failure while storing or loading ark records.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum SinkError {
    /// A record could not be encoded.
    #[error("sink encoding: {0}")]
    Encode(String),
    /// An underlying store operation failed.
    #[error("sink I/O: {0}")]
    Io(String),
    /// Stored data could not be interpreted.
    #[error("sink data: {0}")]
    Data(String),
}

impl From<WitnessError> for SinkError {
    fn from(error: WitnessError) -> Self {
        Self::Encode(error.to_string())
    }
}

/// Persistence boundary implemented by the amber-local spool in S0.
pub trait WitnessSink {
    /// Appends a write-ahead intent before its action occurs.
    fn intent(&mut self, i: &Intent) -> Result<(), SinkError>;
    /// Stores a witness and returns its content-derived CID string.
    fn witness(&mut self, w: &DeathWitness) -> Result<String, SinkError>;
    /// Stores the current incident projection.
    fn incident(&mut self, i: &Incident) -> Result<(), SinkError>;
    /// Stores the current passport projection.
    fn passport(&mut self, p: &Passport) -> Result<(), SinkError>;
    /// Stores the current death tally for a process.
    fn tally(&mut self, process: &str, t: &DeathTally) -> Result<(), SinkError>;
    /// Loads the latest death tally for a process.
    fn load_tally(&self, process: &str) -> Result<Option<DeathTally>, SinkError>;
    /// Loads the latest berth passport.
    fn load_passport(&self) -> Result<Option<Passport>, SinkError>;
}

/// In-memory test implementation shared by core and downstream crate tests.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;

    /// Vec-backed witness sink for deterministic tests.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct MemorySink {
        /// Intents in append order.
        pub intents: Vec<Intent>,
        /// Witnesses in write order, including repeated incident writes.
        pub witnesses: Vec<DeathWitness>,
        /// Incident projections in write order.
        pub incidents: Vec<Incident>,
        /// Passport projections in write order.
        pub passports: Vec<Passport>,
        /// Process tally projections in write order.
        pub tallies: Vec<(String, DeathTally)>,
    }

    impl WitnessSink for MemorySink {
        fn intent(&mut self, i: &Intent) -> Result<(), SinkError> {
            self.intents.push(i.clone());
            Ok(())
        }

        fn witness(&mut self, w: &DeathWitness) -> Result<String, SinkError> {
            let cid = w.cid()?;
            self.witnesses.push(w.clone());
            Ok(cid)
        }

        fn incident(&mut self, i: &Incident) -> Result<(), SinkError> {
            self.incidents.push(i.clone());
            Ok(())
        }

        fn passport(&mut self, p: &Passport) -> Result<(), SinkError> {
            self.passports.push(p.clone());
            Ok(())
        }

        fn tally(&mut self, process: &str, t: &DeathTally) -> Result<(), SinkError> {
            self.tallies.push((process.to_string(), t.clone()));
            Ok(())
        }

        fn load_tally(&self, process: &str) -> Result<Option<DeathTally>, SinkError> {
            Ok(self
                .tallies
                .iter()
                .rev()
                .find(|(name, _)| name == process)
                .map(|(_, tally)| tally.clone()))
        }

        fn load_passport(&self) -> Result<Option<Passport>, SinkError> {
            Ok(self.passports.last().cloned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::MemorySink;
    use super::*;
    use crate::{
        EffectiveTier, ExitClass, ProcessPassport, RestartVerdict, PASSPORT_KIND, WITNESS_KIND,
    };

    fn passport(updated_at_epoch_ms: u64) -> Passport {
        Passport {
            schema: 1,
            kind: PASSPORT_KIND.to_string(),
            manifest: "bafy-manifest".to_string(),
            node: None,
            incarnation: 1,
            ark_version: "0.1.0".to_string(),
            processes: vec![ProcessPassport {
                name: "conductor".to_string(),
                artifact_sha256: "ab".repeat(32),
                artifact_path: "/opt/elohim/conductor".to_string(),
                pid: Some(42),
                started_at_epoch_ms: Some(1_000),
                ready: false,
                effective_tier: EffectiveTier::None,
                deaths_in_window: 1,
            }],
            last_verdict: None,
            updated_at_epoch_ms,
        }
    }

    fn witness() -> DeathWitness {
        DeathWitness {
            schema: 1,
            kind: WITNESS_KIND.to_string(),
            incident: "bafy-incident".to_string(),
            process: "conductor".to_string(),
            incarnation: 1,
            pid: 42,
            artifact_sha256: "ab".repeat(32),
            artifact_path: "/opt/elohim/conductor".to_string(),
            started_at_epoch_ms: 1_000,
            died_at_epoch_ms: 2_000,
            uptime_ms: 1_000,
            exit: ExitClass::Exited { code: 1 },
            last_stderr: Vec::new(),
            last_stdout: Vec::new(),
            sample: None,
            last_intent: None,
            passport: passport(2_000),
            verdict: None,
            refusal: None,
        }
    }

    #[test]
    fn witness_two_write_contract_retains_both_content_addresses() {
        let mut sink = MemorySink::default();
        let write_ahead = witness();
        let first_cid = sink.witness(&write_ahead).unwrap();

        let mut decided = write_ahead;
        decided.verdict = Some(RestartVerdict::Stop);
        let second_cid = sink.witness(&decided).unwrap();

        assert_ne!(first_cid, second_cid);
        assert_eq!(sink.witnesses.len(), 2);
        assert_eq!(sink.witnesses[0].verdict, None);
        assert_eq!(sink.witnesses[1].verdict, Some(RestartVerdict::Stop));
    }

    #[test]
    fn loads_last_stored_tally_and_passport() {
        let mut sink = MemorySink::default();
        let first_tally = DeathTally::default();
        let mut last_tally = DeathTally::default();
        last_tally.record(crate::DeathRecord {
            at_epoch_s: 2,
            class: ExitClass::Exited { code: 1 },
            uptime_ms: 1_000,
            first_stderr_line: None,
        });
        sink.tally("conductor", &first_tally).unwrap();
        sink.tally("storage", &DeathTally::default()).unwrap();
        sink.tally("conductor", &last_tally).unwrap();

        let first_passport = passport(1_000);
        let last_passport = passport(2_000);
        sink.passport(&first_passport).unwrap();
        sink.passport(&last_passport).unwrap();

        assert_eq!(sink.load_tally("conductor").unwrap(), Some(last_tally));
        assert_eq!(sink.load_passport().unwrap(), Some(last_passport));
    }
}
