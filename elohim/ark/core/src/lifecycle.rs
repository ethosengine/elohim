//! The pure lifecycle state machine — filled by Task 6.
//!
//! KEPT rather than projected onto [`elohim_epr_rea::model::CommitmentState`]: that is the
//! lifecycle of a PROMISE (proposed → active → fulfilled → revoked), while this is the
//! lifecycle of a running child (idle → spawning → booting → live → dying → dead). The
//! transitions that are economically meaningful already leave through [`crate::rea`] as
//! intents and events.
