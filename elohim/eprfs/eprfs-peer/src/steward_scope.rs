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

/// The steward-scoped resources a policy declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StewardScope {
    names: BTreeSet<String>,
    prefixes: BTreeSet<String>,
    confirm_wait_s: u64,
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
            confirm_wait_s: DEFAULT_CONFIRM_WAIT_S,
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
        if let Some(wait) = berth
            .and_then(|b| b.get("channel"))
            .and_then(|c| c.get("confirm_wait_s"))
        {
            scope.confirm_wait_s = wait.as_u64().ok_or_else(|| {
                invalid("berth.channel.confirm_wait_s: not a whole number of seconds".into())
            })?;
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
        self.confirm_wait_s
    }
}
