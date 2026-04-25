//! Peer policy engine.
//!
//! Operator-declared policy (`peer-policy.toml`) + live runtime state →
//! `PeerCapabilityFlags` that the heartbeat task projects into Mishpat.
//!
//! - [`config`] — TOML-loaded configuration types.
//! - [`evaluator`] — derives capability flags from config + live state.

pub mod config;
pub mod evaluator;

pub use config::{
    AutoOrBool, NetworkConfig, PolicyConfig, PoolConfig, StewardshipConfig, WriteThroughEntry,
};
pub use evaluator::{evaluate, EvaluatedFlags, LiveState};

use crate::write_through::{WriteThroughConfig, WriteThroughOverride};

/// Convert the policy.toml `[write_through.<pillar>]` map into a
/// `WriteThroughOverride` (Task C.6 layer 2).
///
/// Returns `None` when the policy section is empty so callers can pass
/// `None` to `WriteThroughState::with_policy_override` and let lower layers
/// speak unimpeded.
pub fn write_through_override_from_policy(
    entries: &std::collections::HashMap<String, WriteThroughEntry>,
) -> Option<WriteThroughOverride> {
    if entries.is_empty() {
        return None;
    }
    let mut pillars = std::collections::HashMap::new();
    for (pillar, entry) in entries {
        pillars.insert(
            pillar.clone(),
            WriteThroughConfig {
                enabled: entry.enabled,
                kinds: entry.kinds.clone(),
            },
        );
    }
    Some(WriteThroughOverride { pillars })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_through_override_from_policy_returns_none_for_empty() {
        let entries = std::collections::HashMap::new();
        assert!(write_through_override_from_policy(&entries).is_none());
    }

    #[test]
    fn write_through_override_from_policy_carries_pillars_and_kinds() {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "shefa".to_string(),
            WriteThroughEntry {
                enabled: true,
                kinds: Some(vec!["EconomicEvent".to_string()]),
            },
        );
        entries.insert(
            "lamad".to_string(),
            WriteThroughEntry {
                enabled: false,
                kinds: None,
            },
        );
        let ov = write_through_override_from_policy(&entries).expect("non-empty");
        assert_eq!(ov.pillars.len(), 2);
        assert!(ov.for_pillar("shefa").unwrap().enabled);
        assert_eq!(
            ov.for_pillar("shefa").unwrap().kinds.as_deref(),
            Some(&["EconomicEvent".to_string()][..])
        );
        assert!(!ov.for_pillar("lamad").unwrap().enabled);
        assert!(ov.for_pillar("lamad").unwrap().kinds.is_none());
    }
}
