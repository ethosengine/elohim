//! A node's identity, declared: the same acts as the terminal verbs, written
//! down once so a node that nobody types at is provisioned the same way.
//!
//! The declaration is a file on the node's own disk. Writing it is an act on
//! that machine, which is exactly who may make the node sign as its person, so
//! a declaration carries the same authority as `epr identity begin` typed
//! there, and no more. It holds no secret.
//!
//! Its keys are the words the verbs already use: `[identity]` is the body of
//! `POST /auth/identity/begin`; each `[[devices]]` entry is what a person would
//! answer `epr device approve` with for one device; `[asking]` is what
//! `epr device ask` is told on an asking node.
//!
//! What a declaration never does:
//! - overwrite or re-key an identity that exists. A declaration that disagrees
//!   with the identity on the node is refused, by name;
//! - approve a device by itself. A device still asks, and the code is still
//!   bound to its own request and PKCE verifier. A declared device only spares
//!   the person the question, and only for no more than the acts declared.

use serde::{Deserialize, Serialize};

use crate::act::RequestedAct;
use crate::request::GrantRequest;

/// The whole declaration. Every part is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declaration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<DeclaredIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<DeclaredDevice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asking: Option<DeclaredAsking>,
}

/// The person this node begins an identity for when it has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredIdentity {
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_reach: Option<String>,
    /// How many of the person's stewards an approval needs. Absent means one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approvals_needed: Option<usize>,
}

/// A device the person expects to join, and what they agree it may ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredDevice {
    pub label: String,
    pub device_key: String,
    pub acts: Vec<RequestedAct>,
}

/// On an asking node: where its steward is, and what this node is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredAsking {
    pub label: String,
    pub portal: String,
    /// The steward node's own address; the portal's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
}

/// The identity that exists on a node, as far as a declaration can disagree
/// with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingIdentity {
    pub human_id: String,
    pub display_name: String,
    pub profile_reach: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationConflict {
    HumanId { declared: String, existing: String },
    DisplayName { declared: String, existing: String },
    ProfileReach { declared: String, existing: String },
}

impl DeclarationConflict {
    pub fn code(&self) -> &'static str {
        "identity_declaration_disagrees"
    }
}

impl std::fmt::Display for DeclarationConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (what, declared, existing) = match self {
            Self::HumanId { declared, existing } => ("person id", declared, existing),
            Self::DisplayName { declared, existing } => ("name", declared, existing),
            Self::ProfileReach { declared, existing } => ("profile reach", declared, existing),
        };
        write!(
            f,
            "the declared {what} `{declared}` is not the existing identity's `{existing}`; \
             the existing identity is kept as it is"
        )
    }
}

impl DeclaredIdentity {
    /// The profile reach the declaration asks for: `private` when unsaid.
    pub fn reach(&self) -> &str {
        self.profile_reach.as_deref().unwrap_or("private")
    }

    /// How many stewards an approval needs: one when unsaid.
    pub fn approvals(&self) -> usize {
        self.approvals_needed.unwrap_or(1)
    }

    /// Whether this declaration agrees with the identity that exists. Only
    /// what the declaration says is compared; what it leaves out it accepts.
    pub fn agrees_with(&self, existing: &ExistingIdentity) -> Result<(), DeclarationConflict> {
        if let Some(id) = &self.human_id {
            if id != &existing.human_id {
                return Err(DeclarationConflict::HumanId {
                    declared: id.clone(),
                    existing: existing.human_id.clone(),
                });
            }
        }
        if self.display_name.trim() != existing.display_name {
            return Err(DeclarationConflict::DisplayName {
                declared: self.display_name.clone(),
                existing: existing.display_name.clone(),
            });
        }
        if let Some(reach) = &self.profile_reach {
            if reach != &existing.profile_reach {
                return Err(DeclarationConflict::ProfileReach {
                    declared: reach.clone(),
                    existing: existing.profile_reach.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Declaration {
    /// The acts already agreed for `request` by declaration: the request's own
    /// acts, when it comes from a declared device's key and asks for no more
    /// than that device was declared for. `None` means the person must answer.
    pub fn agreed_for(
        &self,
        request: &GrantRequest,
    ) -> Option<(&DeclaredDevice, Vec<RequestedAct>)> {
        let device = self
            .devices
            .iter()
            .find(|d| d.device_key == request.device_key)?;
        request
            .acts
            .iter()
            .all(|a| device.acts.contains(a))
            .then(|| (device, request.acts.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::tests::{peer_request, request};

    fn declared(acts: Vec<RequestedAct>) -> Declaration {
        Declaration {
            identity: None,
            devices: vec![DeclaredDevice {
                label: "workspace".into(),
                device_key: request().device_key,
                acts,
            }],
            asking: None,
        }
    }

    #[test]
    fn a_declared_device_asking_no_more_than_declared_needs_no_question() {
        use RequestedAct::{BindDeviceRoot, EnrollDevice};
        let both = declared(vec![EnrollDevice, BindDeviceRoot]);
        let (device, acts) = both.agreed_for(&request()).unwrap();
        assert_eq!(device.label, "workspace");
        assert_eq!(acts, [EnrollDevice]);
        assert_eq!(
            both.agreed_for(&peer_request()).unwrap().1,
            [EnrollDevice, BindDeviceRoot]
        );
    }

    #[test]
    fn more_than_declared_or_an_unknown_key_still_needs_the_person() {
        let enroll_only = declared(vec![RequestedAct::EnrollDevice]);
        assert!(enroll_only.agreed_for(&peer_request()).is_none());
        let mut stranger = request();
        stranger.device_key = crate::hash_shape::sample_key(8);
        assert!(enroll_only.agreed_for(&stranger).is_none());
        assert!(Declaration::default().agreed_for(&request()).is_none());
    }

    fn identity() -> DeclaredIdentity {
        DeclaredIdentity {
            display_name: "Matthew".into(),
            human_id: None,
            identifier: None,
            profile_reach: None,
            approvals_needed: None,
        }
    }

    fn existing() -> ExistingIdentity {
        ExistingIdentity {
            human_id: "h-1".into(),
            display_name: "Matthew".into(),
            profile_reach: "private".into(),
        }
    }

    #[test]
    fn a_declaration_agrees_with_what_it_says_and_accepts_what_it_leaves_out() {
        assert_eq!(identity().agrees_with(&existing()), Ok(()));
        assert_eq!(identity().reach(), "private");
        assert_eq!(identity().approvals(), 1);
        let mut other = identity();
        other.display_name = "Matt".into();
        assert!(matches!(
            other.agrees_with(&existing()),
            Err(DeclarationConflict::DisplayName { .. })
        ));
        let mut other = identity();
        other.human_id = Some("h-2".into());
        let conflict = other.agrees_with(&existing()).unwrap_err();
        assert_eq!(conflict.code(), "identity_declaration_disagrees");
        assert!(conflict.to_string().contains("kept as it is"));
        let mut other = identity();
        other.profile_reach = Some("public".into());
        assert!(other.agrees_with(&existing()).is_err());
    }

    #[test]
    fn the_declaration_reads_in_the_verbs_own_words() {
        let json = serde_json::json!({
            "identity": {"displayName": "Matthew", "identifier": "matthew", "approvalsNeeded": 2},
            "devices": [{"label": "home", "deviceKey": request().device_key, "acts": ["device.enroll"]}],
            "asking": {"label": "workspace", "portal": "http://127.0.0.1:8191"}
        });
        let d: Declaration = serde_json::from_value(json).unwrap();
        assert_eq!(d.identity.as_ref().unwrap().approvals(), 2);
        assert_eq!(d.devices[0].acts, [RequestedAct::EnrollDevice]);
        let unknown = serde_json::json!({"identity": {"displayName": "M", "password": "x"}});
        assert!(serde_json::from_value::<Declaration>(unknown).is_err());
    }
}
