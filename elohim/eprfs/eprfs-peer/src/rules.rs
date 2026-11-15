//! The advisory digest of the crossing, steward-scope and artifact-serve rules one device
//! enforces.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::allow::CrossingAllowList;
use crate::error::PeerError;
use crate::serves::{ArtifactServeList, NO_SERVE_LIST};
use crate::steward_scope::StewardScope;

/// The version of the artifact gate's semantics, named in the rules digest so two devices whose
/// gates decide differently never show the same digest (K12 threat T17). Version 1 (the method-row
/// gate, before K12) wrote no such field; 2 decides each record on its own reach claim.
pub const GATE_SEMANTICS: u32 = 2;

/// The crossing allow-list, steward scope and artifact serve grant loaded together from one
/// policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    allow: CrossingAllowList,
    scope: StewardScope,
    /// `None` when no serve policy was loaded: the gate then serves no artifact at all, by row or
    /// by reach (K12 threat T5), rather than reading an empty list.
    serves: Option<ArtifactServeList>,
}

impl Rules {
    /// Load every enforced rule set from one parsed policy.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when any rule set is malformed.
    ///
    /// A policy with no `berth.artifacts` section loads no serve policy at all (`None`), so the
    /// gate refuses every artifact; it is never read as an empty list, which narrows nothing and
    /// would serve every `self` record (K12 threat T5 through a lost or misspelled section).
    pub fn from_policy(policy: &Value) -> Result<Rules, PeerError> {
        let rules = Rules::new(
            CrossingAllowList::from_policy(policy)?,
            StewardScope::from_policy(policy)?,
        );
        if policy.pointer("/berth/artifacts").is_none() {
            return Ok(rules);
        }
        Ok(rules.with_serves(ArtifactServeList::from_policy(policy)?))
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

    /// Combine an already-loaded crossing allow-list and steward scope. No serve policy is
    /// loaded, so no artifact is served, by its method's row or by its reach.
    pub fn new(allow: CrossingAllowList, scope: StewardScope) -> Rules {
        Rules {
            allow,
            scope,
            serves: None,
        }
    }

    /// These rules with `serves` as the loaded artifact serve policy.
    #[must_use]
    pub fn with_serves(mut self, serves: ArtifactServeList) -> Rules {
        self.serves = Some(serves);
        self
    }

    /// The crossing allow-list this device enforces.
    pub fn allow(&self) -> &CrossingAllowList {
        &self.allow
    }

    /// The steward scope this device enforces.
    pub fn scope(&self) -> &StewardScope {
        &self.scope
    }

    /// The artifact serve grant this device enforces; an empty one when no serve policy is
    /// loaded. The gate reads [`Self::serve_policy`], never this.
    pub fn serves(&self) -> &ArtifactServeList {
        self.serves.as_ref().unwrap_or(&NO_SERVE_LIST)
    }

    /// The loaded artifact serve policy, or `None` when none is loaded. The gate holds this as
    /// its shared policy and builds [`crate::ServePolicy`] from it.
    pub fn serve_policy(&self) -> Option<&ArtifactServeList> {
        self.serves.as_ref()
    }

    /// The digest of only the crossing kinds, the steward-scoped resource names and prefixes, the
    /// artifact serve grant and the gate's semantics version ([`GATE_SEMANTICS`]). An empty grant
    /// is left out of the canonical form, so a policy that serves nothing keeps the digest of
    /// the same policy before the grant existed, under the same gate.
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
            #[serde(skip_serializing_if = "serves_nothing")]
            serves: &'a BTreeMap<String, BTreeSet<String>>,
            #[serde(skip_serializing_if = "BTreeSet::is_empty")]
            disclose_method: &'a BTreeSet<String>,
            // No serve policy loaded refuses everything, while a loaded empty one serves `self`
            // records: the two decide differently, so they never share a digest.
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            no_serve_policy: bool,
            gate_semantics: u32,
        }

        let canonical = Canonical {
            crosses: self.allow.crosses(),
            steward: Steward {
                names: self.scope.names(),
                prefixes: self.scope.prefixes(),
            },
            serves: self.serves().by_method(),
            disclose_method: self.serves().disclosed_methods(),
            no_serve_policy: self.serves.is_none(),
            gate_semantics: GATE_SEMANTICS,
        };
        let bytes = serde_json::to_vec(&canonical)
            .expect("canonical rules contain only JSON strings and ordered collections");
        RulesDigest(format!("{:x}", Sha256::digest(bytes)))
    }
}

/// Whether a canonical serve grant is empty, and so left out of the digest.
fn serves_nothing(serves: &&BTreeMap<String, BTreeSet<String>>) -> bool {
    serves.is_empty()
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
