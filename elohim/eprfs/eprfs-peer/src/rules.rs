//! The advisory digest of the crossing and steward-scope rules one device enforces.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::allow::CrossingAllowList;
use crate::error::PeerError;
use crate::steward_scope::StewardScope;

/// The crossing allow-list and steward scope loaded together from one policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    allow: CrossingAllowList,
    scope: StewardScope,
}

impl Rules {
    /// Load both enforced rule sets from one parsed policy.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when either rule set is malformed.
    pub fn from_policy(policy: &Value) -> Result<Rules, PeerError> {
        Ok(Rules::new(
            CrossingAllowList::from_policy(policy)?,
            StewardScope::from_policy(policy)?,
        ))
    }

    /// Load both enforced rule sets from one `pool-policy.json` file.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when the file cannot be read, is not JSON, or either rule set is
    /// malformed.
    pub fn from_policy_file(path: &Path) -> Result<Rules, PeerError> {
        let invalid = |reason: String| PeerError::Policy {
            path: path.to_path_buf(),
            reason,
        };
        let text = std::fs::read_to_string(path).map_err(|error| invalid(error.to_string()))?;
        let policy: Value =
            serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?;
        Self::from_policy(&policy).map_err(|error| match error {
            PeerError::Policy { reason, .. } => PeerError::Policy {
                path: path.to_path_buf(),
                reason,
            },
            other => other,
        })
    }

    /// Combine an already-loaded crossing allow-list and steward scope.
    pub fn new(allow: CrossingAllowList, scope: StewardScope) -> Rules {
        Rules { allow, scope }
    }

    /// The crossing allow-list this device enforces.
    pub fn allow(&self) -> &CrossingAllowList {
        &self.allow
    }

    /// The steward scope this device enforces.
    pub fn scope(&self) -> &StewardScope {
        &self.scope
    }

    /// The digest of only the crossing kinds and steward-scoped resource names and prefixes.
    pub fn digest(&self) -> RulesDigest {
        #[derive(Serialize)]
        struct Steward<'a> {
            names: &'a BTreeSet<String>,
            prefixes: &'a BTreeSet<String>,
        }

        #[derive(Serialize)]
        struct Canonical<'a> {
            crosses: &'a BTreeMap<String, BTreeSet<String>>,
            steward: Steward<'a>,
        }

        let canonical = Canonical {
            crosses: self.allow.crosses(),
            steward: Steward {
                names: self.scope.names(),
                prefixes: self.scope.prefixes(),
            },
        };
        let bytes = serde_json::to_vec(&canonical)
            .expect("canonical rules contain only JSON strings and ordered collections");
        RulesDigest(format!("{:x}", Sha256::digest(bytes)))
    }
}

/// A lowercase hexadecimal SHA-256 digest of the rules a device enforces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RulesDigest(String);

impl RulesDigest {
    /// The complete lowercase hexadecimal digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The first eight hexadecimal characters for human status lines.
    pub fn short(&self) -> &str {
        self.0.get(..8).unwrap_or(&self.0)
    }
}

impl fmt::Display for RulesDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The reloadable policy state persisted by the daemon for status and berth readers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesState {
    /// The rules the daemon has loaded and continues to enforce.
    pub digest: RulesDigest,
    /// The `pool-policy.json` file the daemon reloads.
    pub policy: PathBuf,
    /// This device's clock when these rules were loaded, unix milliseconds.
    pub loaded_at: u64,
    /// Why the file on disk could not replace the loaded rules.
    pub pending: Option<String>,
}
