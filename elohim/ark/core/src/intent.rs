//! Write-ahead records of actions the ark intends to take.

use serde::{Deserialize, Serialize};

/// A decision durably recorded before its process action occurs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Intent {
    /// Wall-clock time at which the decision was recorded.
    pub at_epoch_ms: u64,
    /// Berth incarnation in which the decision was made.
    pub incarnation: u64,
    /// Process named by the decision.
    pub process: String,
    /// Process action that will follow this record.
    pub action: IntentAction,
    /// Human-readable decision reason.
    pub reason: String,
}

/// An action that must have a write-ahead intent.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    /// Start a child for the first time in this incarnation.
    Spawn,
    /// Restart a child after a prior death.
    Restart {
        /// Restart attempt in the active policy window.
        attempt: u32,
        /// Delay before the restart is acted on.
        after_s: u64,
    },
    /// Ask the child to stop gracefully.
    Stop {
        /// Signal sent to request shutdown.
        signal: i32,
        /// Grace period before a forced kill.
        grace_ms: u64,
    },
    /// Forcefully terminate a child.
    Kill,
    /// Permanently stop attempting to run a child.
    GiveUp,
}
