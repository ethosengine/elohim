//! Which berth resources are steward-scoped, shared by every device of one person, and how long a
//! claim on one waits for the other devices: `berth.resources` and `berth.channel` in
//! `pool-policy.json` (spec §3.4).
//!
//! A resource entry is a capacity (`"mesh": 1`, container-scoped) or an object
//! `{"capacity": n, "scope": "container" | "steward"}`. A key ending in `:` names a prefix:
//! `"lane:"` covers every `lane:<id>`. Container-scoped resources never consult peers.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::PeerError;

/// How long a steward-scoped claim waits for every other device's acknowledgement when the policy
/// does not say: `berth.channel.confirm_wait_s`.
pub const DEFAULT_CONFIRM_WAIT_S: u64 = 10;
/// How far before ttl expiry a holder stops trusting a silent peer.
pub const DEFAULT_FENCE_MARGIN_S: u64 = 30;
/// The clock skew the offline override wait covers.
pub const DEFAULT_MAX_SKEW_S: u64 = 30;

/// The rulebook's numeric settlement thresholds, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settlement {
    /// How long an unacknowledged claim remains pending.
    pub confirm_wait_s: u64,
    /// How far before ttl expiry a holder self-demotes without a fresh covering head.
    pub fence_margin_s: u64,
    /// The bounded clock skew included in an offline override's delay.
    pub max_skew_s: u64,
}

impl Default for Settlement {
    fn default() -> Self {
        Self {
            confirm_wait_s: DEFAULT_CONFIRM_WAIT_S,
            fence_margin_s: DEFAULT_FENCE_MARGIN_S,
            max_skew_s: DEFAULT_MAX_SKEW_S,
        }
    }
}

/// The steward-scoped resources a policy declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StewardScope {
    names: BTreeSet<String>,
    prefixes: BTreeSet<String>,
    settlement: Settlement,
}

impl StewardScope {
    /// The scope a parsed `pool-policy.json` declares. A policy with no `berth.resources`
    /// declares none.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when an entry is neither a capacity nor an object with a known
    /// `scope`, or `confirm_wait_s` is not a whole number of seconds.
    pub fn from_policy(policy: &Value) -> Result<StewardScope, PeerError> {
        Self::read(policy, Path::new("pool-policy.json"))
    }

    /// The scope `path` (a `pool-policy.json`) declares.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when the file cannot be read, is not JSON, or its entries are
    /// malformed.
    pub fn from_policy_file(path: &Path) -> Result<StewardScope, PeerError> {
        let invalid = |reason: String| PeerError::Policy {
            path: path.to_path_buf(),
            reason,
        };
        let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
        let policy: Value = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
        Self::read(&policy, path)
    }

    fn read(policy: &Value, path: &Path) -> Result<StewardScope, PeerError> {
        let invalid = |reason: String| PeerError::Policy {
            path: PathBuf::from(path),
            reason,
        };
        let berth = policy.get("berth");
        let mut scope = StewardScope {
            names: BTreeSet::new(),
            prefixes: BTreeSet::new(),
            settlement: Settlement::default(),
        };
        let resources = berth.and_then(|b| b.get("resources"));
        if let Some(resources) = resources {
            let resources = resources.as_object().ok_or_else(|| {
                invalid("berth.resources: not an object of resource -> entry".into())
            })?;
            for (name, entry) in resources {
                if name.starts_with('_') {
                    continue;
                }
                let steward = match entry {
                    Value::Number(_) => false,
                    Value::Object(fields) => match fields.get("scope").and_then(Value::as_str) {
                        None | Some("container") => false,
                        Some("steward") => true,
                        Some(other) => {
                            return Err(invalid(format!(
                                "berth.resources.{name}: unknown scope `{other}` \
                                 (container | steward)"
                            )))
                        }
                    },
                    _ => {
                        return Err(invalid(format!(
                            "berth.resources.{name}: neither a capacity nor an object"
                        )))
                    }
                };
                if !steward {
                    continue;
                }
                if name.ends_with(':') {
                    scope.prefixes.insert(name.clone());
                } else {
                    scope.names.insert(name.clone());
                }
            }
        }
        let channel = berth.and_then(|b| b.get("channel"));
        for (name, target) in [
            ("confirm_wait_s", &mut scope.settlement.confirm_wait_s),
            ("fence_margin_s", &mut scope.settlement.fence_margin_s),
            ("max_skew_s", &mut scope.settlement.max_skew_s),
        ] {
            if let Some(value) = channel.and_then(|c| c.get(name)) {
                *target = value.as_u64().ok_or_else(|| {
                    invalid(format!(
                        "berth.channel.{name}: not a whole number of seconds"
                    ))
                })?;
            }
        }
        Ok(scope)
    }

    /// Whether `resource` is steward-scoped: named, or under a declared prefix (a bare prefix is
    /// not a resource).
    pub fn covers(&self, resource: &str) -> bool {
        self.names.contains(resource)
            || self.prefixes.iter().any(|prefix| {
                resource.len() > prefix.len() && resource.starts_with(prefix.as_str())
            })
    }

    /// How long a steward-scoped claim waits for every other device's acknowledgement, seconds.
    pub fn confirm_wait_s(&self) -> u64 {
        self.settlement.confirm_wait_s
    }

    /// The settlement thresholds this scope's channel declares.
    pub fn settlement(&self) -> Settlement {
        self.settlement
    }

    /// The ordered exact resource names used to encode the rules digest.
    pub(crate) fn names(&self) -> &BTreeSet<String> {
        &self.names
    }

    /// The ordered resource prefixes used to encode the rules digest.
    pub(crate) fn prefixes(&self) -> &BTreeSet<String> {
        &self.prefixes
    }
}
