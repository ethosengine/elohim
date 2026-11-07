//! Which derivation artifacts this device serves, and to whom: `berth.artifacts.serves` in
//! `pool-policy.json` (artifact-store design §1.4).
//!
//! The grant is per method name — the `name` of the method descriptor a manifest's derivation
//! record keys — never per artifact and never per CID: `{"mesh-build": ["steward"]}` serves
//! every artifact this device holds custody of whose method is named `mesh-build`. An artifact of
//! any other method is held and never served. Default deny: a policy with no
//! `berth.artifacts.serves` serves nothing.
//!
//! Audience vocabulary:
//! - `"steward"` — every device of the person [`Admission::admits_device`] admits now;
//! - `"carrier:<did:key>"` — reserved for a carrier grant (D9). It parses, enters the rules
//!   digest, and grants nothing until that grant has a verifier.
//!
//! Anything else is refused as a malformed policy, never widened.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::Value;

use crate::admission::Admission;
use crate::error::PeerError;

/// The path of the grant in `pool-policy.json`.
pub const POLICY_KEYS: [&str; 3] = ["berth", "artifacts", "serves"];

/// The audience every admitted device of the person is.
pub const STEWARD_AUDIENCE: &str = "steward";

/// The prefix of a reserved carrier audience (D9).
pub const CARRIER_AUDIENCE_PREFIX: &str = "carrier:";

/// The artifact serve grant: method name → audiences.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactServeList {
    by_method: BTreeMap<String, BTreeSet<String>>,
}

impl ArtifactServeList {
    /// The grant a parsed `pool-policy.json` declares; empty when it declares none.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when `berth.artifacts.serves` is not an object of method name →
    /// array of audiences, or an audience is outside the vocabulary.
    pub fn from_policy(policy: &Value) -> Result<ArtifactServeList, PeerError> {
        let invalid = |reason: String| PeerError::Policy {
            path: PathBuf::from("pool-policy.json"),
            reason,
        };
        let mut list = ArtifactServeList::default();
        let Some(serves) = POLICY_KEYS
            .iter()
            .try_fold(policy, |value, key| value.get(key))
        else {
            return Ok(list);
        };
        let serves = serves.as_object().ok_or_else(|| {
            invalid("berth.artifacts.serves: not an object of method name -> audiences".into())
        })?;
        for (method, audiences) in serves {
            if method.starts_with('_') {
                continue;
            }
            if method.is_empty() {
                return Err(invalid(
                    "berth.artifacts.serves: an empty method name".into(),
                ));
            }
            let audiences = audiences.as_array().ok_or_else(|| {
                invalid(format!(
                    "berth.artifacts.serves.{method}: not an array of audiences"
                ))
            })?;
            let mut admitted = BTreeSet::new();
            for audience in audiences {
                let audience = audience.as_str().ok_or_else(|| {
                    invalid(format!(
                        "berth.artifacts.serves.{method}: an audience that is not a string"
                    ))
                })?;
                let known = audience == STEWARD_AUDIENCE
                    || audience
                        .strip_prefix(CARRIER_AUDIENCE_PREFIX)
                        .is_some_and(|did| did.starts_with("did:key:"));
                if !known {
                    return Err(invalid(format!(
                        "berth.artifacts.serves.{method}: unknown audience `{audience}` \
                         (steward | carrier:<did:key>)"
                    )));
                }
                admitted.insert(audience.to_string());
            }
            list.by_method.insert(method.clone(), admitted);
        }
        Ok(list)
    }

    /// Whether `device` may be served artifacts of the method named `method_name`, judged against
    /// `admission` now. Only the steward audience grants today; a carrier entry is reserved.
    pub fn grants(&self, device: &str, method_name: &str, admission: &Admission) -> bool {
        self.by_method
            .get(method_name)
            .is_some_and(|audiences| audiences.contains(STEWARD_AUDIENCE))
            && admission.admits_device(device)
    }

    /// Whether the grant serves nothing.
    pub fn is_empty(&self) -> bool {
        self.by_method.is_empty()
    }

    /// The ordered grant used to encode the rules digest.
    pub(crate) fn by_method(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.by_method
    }
}
