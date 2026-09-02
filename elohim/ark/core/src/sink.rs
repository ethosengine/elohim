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
