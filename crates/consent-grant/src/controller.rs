//! The controller who agrees: who they are, and whether they may agree to this
//! request.
//!
//! A host reads this from the controller's own cell before it asks that cell to
//! sign. Checking it here first lets a person hear plainly why they cannot
//! agree, rather than read a refusal from the zome.
//!
//! Every device a person has joined speaks for them, alongside the root
//! controllers their identity's authority names; together these are the
//! "controllers" this module and its wire speak of, kept under that name so
//! the wire stays stable. By default any one of them may agree: the others
//! affirm the device later. A person who wants more than one to agree before a
//! device counts says so in their policy, and `required` reports it. Nothing
//! in the ceremony waits for the others.

use crate::act::RequestedAct;
use crate::consent::Agreement;
use crate::request::AdmittedRequest;

/// What the controller's cell reports about the identity it acts for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerStanding {
    /// The Human record the identity is rooted in.
    pub identity_root: String,
    /// The identity's current authority record.
    pub authority: String,
    /// The network the authority is kept on.
    pub network_dna: String,
    /// Every key known to speak for the person: the authority's root
    /// controllers, and this node's own key when it speaks as a joined device.
    pub controllers: Vec<String>,
    /// How many distinct devices that speak for the person the policy asks to
    /// approve a device.
    pub required: usize,
    /// When this node speaks as a joined device rather than a root
    /// controller, the joining record it speaks through.
    pub speaks_via: Option<String>,
    /// Other identities this node also speaks for (it began one of its own and
    /// joined another as it is). Approvals made here are for `identity_root`.
    pub also_speaks_for: Vec<String>,
}

/// What a portal is told about the identity a person's node speaks for, so it
/// can say what the identity rests on before any device asks.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingView {
    pub identity_root: String,
    pub authority: String,
    pub network_dna: String,
    /// Every device known to speak for the person (the name is the wire's,
    /// kept stable: it now means devices that speak, not only root
    /// controllers).
    pub controllers: Vec<String>,
    /// How many devices speak for the person.
    pub controller_count: usize,
    /// How many of them must approve a new device.
    pub required: usize,
    /// Whether the node answering is one of the devices that speak for the
    /// person.
    pub this_node_is_controller: bool,
    /// Whether the node answering is the only device that speaks for them.
    pub rests_on_this_node_alone: bool,
    /// Each device that speaks for the person: who approved it, when, and how
    /// many of their other devices have affirmed it since. Additive; absent
    /// when the node could not read the list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<StandingDevice>,
    /// Other identities this node also speaks for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also_speaks_for: Vec<String>,
    /// The word the person signs in with: a claim, shown only
    /// ([`crate::declaration::identifier_claim`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// The person's name as given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// One device that speaks for the person, as standing shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingDevice {
    pub device_key: String,
    pub device_fingerprint: String,
    /// Its joining record.
    pub binding: String,
    /// The content network it joined on, which affirming it names.
    pub content_dna: String,
    /// When it joined, in milliseconds since the epoch.
    pub joined_at: i64,
    /// Who approved it.
    pub approved_by: Vec<String>,
    /// The person's other devices that have affirmed it since.
    pub affirmed_by: Vec<String>,
    pub affirmed_count: usize,
    /// Whether this is the node answering.
    pub this_device: bool,
}

/// The devices the network shows speaking for a person, each verified by the
/// rule the mishpat coordinator states: the authority's root controllers, and
/// every joined device whose joining record stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicesRead {
    pub roots: Vec<String>,
    pub devices: Vec<StandingDevice>,
    /// Joining records found that did not stand (or could not be read yet).
    pub not_standing: u32,
    /// Whether the read stopped before every record was followed.
    pub truncated: bool,
}

impl DevicesRead {
    /// Every key known to speak for the person, each once: the root
    /// controllers, then the joined devices.
    pub fn speakers(&self) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        for key in self
            .roots
            .iter()
            .chain(self.devices.iter().map(|d| &d.device_key))
        {
            if !all.contains(key) {
                all.push(key.clone());
            }
        }
        all
    }

    /// The devices this node should affirm now, at most `cap`: devices that
    /// speak for the person and that this node has not affirmed, did not
    /// approve, and is not. Oldest first, so a node that was away catches up
    /// in the order the devices joined.
    ///
    /// Affirming is not approving: it adds no voice and changes no verdict.
    /// It is a second device of the person saying, on its own chain, that it
    /// saw this one join and found its record standing.
    pub fn to_affirm(&self, me: &str, cap: usize) -> Vec<&StandingDevice> {
        let mut due: Vec<&StandingDevice> = self
            .devices
            .iter()
            .filter(|d| {
                d.device_key != me
                    && !d.approved_by.iter().any(|a| a == me)
                    && !d.affirmed_by.iter().any(|a| a == me)
            })
            .collect();
        due.sort_by_key(|d| d.joined_at);
        due.truncate(cap);
        due
    }
}

impl StandingDevice {
    /// The plain words for it: recorded by one device; affirmed by N others.
    pub fn words(&self) -> String {
        let by = match self.approved_by.as_slice() {
            [] => "no device".to_string(),
            [one] => format!("device {}", crate::hash_shape::fingerprint(one)),
            many => format!("{} devices", many.len()),
        };
        let affirmed = match self.affirmed_count {
            0 => "affirmed by no other device yet".to_string(),
            1 => "affirmed by one other device".to_string(),
            n => format!("affirmed by {n} other devices"),
        };
        format!("approved by {by}; {affirmed}")
    }
}

impl StandingView {
    pub fn of(standing: &ControllerStanding, this_node: &str) -> Self {
        let this_node_is_controller = standing.controllers.iter().any(|c| c == this_node);
        Self {
            identity_root: standing.identity_root.clone(),
            authority: standing.authority.clone(),
            network_dna: standing.network_dna.clone(),
            controllers: standing.controllers.clone(),
            controller_count: standing.controllers.len(),
            devices: Vec::new(),
            also_speaks_for: standing.also_speaks_for.clone(),
            required: standing.required,
            this_node_is_controller,
            rests_on_this_node_alone: this_node_is_controller && standing.controllers.len() == 1,
            identifier: None,
            display_name: None,
        }
    }

    /// The same view, naming the person by their sign-in word.
    pub fn with_identifier(mut self, identifier: Option<String>) -> Self {
        self.identifier = identifier;
        self
    }

    /// The same view, counting every device the network shows speaking for
    /// the person (the authority's root controllers and the joined devices
    /// whose records stand) and listing them. This node is kept among them
    /// when its own cell says it speaks, even before the network shows it.
    pub fn with_devices(mut self, read: DevicesRead, this_node: &str) -> Self {
        let mut all = read.speakers();
        for key in &self.controllers {
            if !all.contains(key) {
                all.push(key.clone());
            }
        }
        self.this_node_is_controller = all.iter().any(|k| k == this_node);
        self.rests_on_this_node_alone = self.this_node_is_controller && all.len() == 1;
        self.controller_count = all.len();
        self.controllers = all;
        self.devices = read
            .devices
            .into_iter()
            .map(|mut d| {
                d.this_device = d.device_key == this_node;
                d
            })
            .collect();
        self
    }

    /// The same view, with the person's name as given.
    pub fn with_display_name(mut self, display_name: Option<String>) -> Self {
        self.display_name = display_name;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandingRefusal {
    /// The request names a network this identity is not kept on.
    NetworkForeign,
    /// The key that would sign is not one of the identity's controllers.
    NotAController,
}

impl StandingRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::NetworkForeign => "consent_network_foreign",
            Self::NotAController => "consent_not_a_controller",
        }
    }
}

impl ControllerStanding {
    /// Whether `signer` may agree to `admitted` for this identity.
    pub fn admits(&self, admitted: &AdmittedRequest, signer: &str) -> Result<(), StandingRefusal> {
        if admitted.request().network_dna != self.network_dna {
            return Err(StandingRefusal::NetworkForeign);
        }
        if !self.controllers.iter().any(|c| c == signer) {
            return Err(StandingRefusal::NotAController);
        }
        Ok(())
    }

    /// The agreement this controller makes, for [`crate::ConsentRecord::agree`].
    pub fn agreement(&self, agreed_acts: Vec<RequestedAct>, agreed_at_micros: i64) -> Agreement {
        Agreement {
            identity_root: self.identity_root.clone(),
            authority: self.authority.clone(),
            agreed_acts,
            agreed_at_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_shape::{sample_action, sample_key};
    use crate::request::admit_request;
    use crate::request::tests::{policy, request, CONTENT, NETWORK};

    fn standing() -> ControllerStanding {
        ControllerStanding {
            identity_root: sample_action(1),
            authority: sample_action(2),
            network_dna: NETWORK.into(),
            controllers: vec![sample_key(9), sample_key(10)],
            required: 1,
            speaks_via: None,
            also_speaks_for: vec![],
        }
    }

    fn device(key: u8, joined_at: i64, approved_by: &[u8], affirmed_by: &[u8]) -> StandingDevice {
        StandingDevice {
            device_key: sample_key(key),
            device_fingerprint: crate::hash_shape::fingerprint(&sample_key(key)),
            binding: sample_action(key),
            content_dna: CONTENT.into(),
            joined_at,
            approved_by: approved_by.iter().map(|k| sample_key(*k)).collect(),
            affirmed_by: affirmed_by.iter().map(|k| sample_key(*k)).collect(),
            affirmed_count: affirmed_by.len(),
            this_device: false,
        }
    }

    #[test]
    fn standing_counts_every_device_that_speaks_once_and_names_this_one() {
        // Root 9; device 11 approved by 9; device 12 approved by 11 and
        // affirmed by 9.
        let read = DevicesRead {
            roots: vec![sample_key(9)],
            devices: vec![device(11, 10, &[9], &[]), device(12, 20, &[11], &[9])],
            not_standing: 0,
            truncated: false,
        };
        let mut alone = standing();
        alone.controllers = vec![sample_key(9)];
        let view =
            StandingView::of(&alone, &sample_key(12)).with_devices(read.clone(), &sample_key(12));
        assert_eq!(view.controller_count, 3);
        assert!(view.this_node_is_controller && !view.rests_on_this_node_alone);
        assert!(view.devices[1].this_device && !view.devices[0].this_device);
        assert_eq!(
            view.devices[1].words(),
            format!(
                "approved by device {}; affirmed by one other device",
                crate::hash_shape::fingerprint(&sample_key(11))
            )
        );
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["devices"][1]["affirmedCount"], 1);
        assert_eq!(json["devices"][0]["approvedBy"][0], sample_key(9));
        // The root affirms neither what it approved nor what it affirmed.
        assert!(read.to_affirm(&sample_key(9), 4).is_empty());
        // Device 12 affirms 11 (approved by someone else), never itself.
        let due: Vec<_> = read
            .to_affirm(&sample_key(12), 4)
            .iter()
            .map(|d| d.device_key.clone())
            .collect();
        assert_eq!(due, vec![sample_key(11)]);
        // A node that was away catches up oldest first, a few at a time.
        let many = DevicesRead {
            roots: vec![sample_key(9)],
            devices: vec![
                device(14, 40, &[9], &[]),
                device(13, 30, &[9], &[]),
                device(15, 50, &[9], &[]),
            ],
            not_standing: 0,
            truncated: false,
        };
        let due: Vec<_> = many
            .to_affirm(&sample_key(20), 2)
            .iter()
            .map(|d| d.joined_at)
            .collect();
        assert_eq!(due, vec![30, 40]);
    }

    #[test]
    fn any_named_controller_may_agree_on_the_identitys_network() {
        let admitted = admit_request(&request(), &policy()).unwrap();
        assert_eq!(standing().admits(&admitted, &sample_key(9)), Ok(()));
        assert_eq!(standing().admits(&admitted, &sample_key(10)), Ok(()));
        assert_eq!(
            standing().admits(&admitted, &sample_key(7)),
            Err(StandingRefusal::NotAController)
        );
        let mut elsewhere = request();
        elsewhere.network_dna = CONTENT.into();
        let elsewhere = admit_request(&elsewhere, &policy()).unwrap();
        assert_eq!(
            standing().admits(&elsewhere, &sample_key(9)),
            Err(StandingRefusal::NetworkForeign)
        );
    }

    #[test]
    fn the_view_says_whether_the_identity_rests_on_this_node_alone() {
        let mut alone = standing();
        alone.controllers.truncate(1);
        let view = StandingView::of(&alone, &sample_key(9));
        assert!(view.this_node_is_controller && view.rests_on_this_node_alone);
        assert_eq!(view.controller_count, 1);
        let shared = StandingView::of(&standing(), &sample_key(9));
        assert!(shared.this_node_is_controller && !shared.rests_on_this_node_alone);
        let elsewhere = StandingView::of(&alone, &sample_key(3));
        assert!(!elsewhere.this_node_is_controller && !elsewhere.rests_on_this_node_alone);
        assert_eq!(serde_json::to_value(&view).unwrap().get("identifier"), None);
        let named = view.clone().with_identifier(Some("matthew".into()));
        assert_eq!(
            serde_json::to_value(&named).unwrap()["identifier"],
            "matthew"
        );
        let json = serde_json::to_value(&view).unwrap();
        for field in [
            "identityRoot",
            "controllerCount",
            "required",
            "restsOnThisNodeAlone",
        ] {
            assert!(json.get(field).is_some(), "{field}");
        }
    }

    #[test]
    fn the_agreement_names_the_standing_identity() {
        let a = standing().agreement(vec![RequestedAct::EnrollDevice], 5);
        assert_eq!(a.identity_root, sample_action(1));
        assert_eq!(a.authority, sample_action(2));
        assert_eq!(a.agreed_at_micros, 5);
    }
}
