//! Graduation evaluator — converts observation aggregates into attestations
//! (Path 1, spec §8.1) or summary EconomicEvents (Path 2, spec §8.2).

pub mod attestation;
pub mod diversity;
pub mod summary_event;
