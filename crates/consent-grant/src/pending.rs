//! Asks that arrived over a carrier and wait on an approving node for someone to
//! decide.
//!
//! A carrier is anything that brings a device's request to an approving node without a
//! person carrying the link: today, local discovery on the private network. An
//! ask that arrives is evidence that a node asked, never authority. Nothing
//! here signs, approves or stores anything beyond the approving node's memory. The
//! list is rebuilt by the device announcing again, and an ask leaves it when
//! it is decided or when its five minutes are up.
//!
//! The list is bounded three ways:
//! - a count cap. Over it, a new ask is dropped, not queued;
//! - a per-source limit, so one carrier peer cannot crowd out the rest;
//! - a five-minute life, the code's own window.
//!
//! The same device asking again replaces its earlier ask. Every outcome
//! carries a reason for the log.
//!
//! The source is the carrier's transport id, kept only to send the code back.
//! The request names the device's agent key. They are different namespaces
//! and are never compared.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::act::RequestedAct;
use crate::controller::ControllerStanding;
use crate::declaration::ExistingIdentity;
use crate::hash_shape::fingerprint;
use crate::request::{admit_request, AdmittedRequest, GrantPolicy, GrantRequest, RequestRefusal};

/// How long an ask waits: the same five minutes a code is good for.
pub const ASK_LIFE_MICROS: i64 = 5 * 60 * 1_000_000;
/// The most asks an approving node holds at once.
pub const MAX_PENDING: usize = 32;
/// The most asks one carrier source may hold at once.
pub const MAX_PER_SOURCE: usize = 4;

/// What the asking node says about itself. Read-only and reported as the node
/// sees it; nothing on the approving node depends on it being true except what it
/// shows the person and which deciders may act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum NodeState {
    /// No identity of its own.
    Unassigned,
    /// It began an identity of its own. `made` says whether it has made
    /// anything under it, when the node can tell.
    OwnIdentity { made: Made },
    /// It is already one of some person's nodes.
    Joined,
}

/// Whether a node has made anything under the identity it began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Made {
    /// The node holds something it made.
    Things,
    /// The node cannot tell cheaply. Never read as "nothing".
    Unknown,
}

impl NodeState {
    /// Classify a node from what its own cell says.
    ///
    /// - no Human: unassigned;
    /// - a Human whose authority names this node among its root controllers,
    ///   or a Human with no authority yet: an identity of its own;
    /// - a node that speaks through a joining record (`joined`), or whose
    ///   identity's authority does not name it: one of someone's devices.
    pub fn of(
        has_human: bool,
        controllers: Option<&[String]>,
        joined: bool,
        me: &str,
        made: Made,
    ) -> Self {
        match (has_human, controllers) {
            (false, _) => Self::Unassigned,
            (true, _) if joined => Self::Joined,
            (true, Some(c)) if !c.iter().any(|k| k == me) => Self::Joined,
            (true, _) => Self::OwnIdentity { made },
        }
    }

    /// The sentence a person is shown about this node.
    pub fn words(self) -> &'static str {
        match self {
            Self::Unassigned => "has no identity of its own",
            Self::OwnIdentity { made: Made::Things } => {
                "began an identity of its own and has made things under it"
            }
            Self::OwnIdentity {
                made: Made::Unknown,
            } => "began an identity of its own; whether it made anything under it is not known",
            Self::Joined => "already speaks for someone",
        }
    }
}

/// Whose identity an approval on this node would be for: the identity whose
/// authority names this node as one of its controllers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeaksFor {
    /// The word the person signs in with: a claim, shown only, never used to
    /// decide or join ([`crate::declaration::identifier_claim`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// The id of the person's Human record, when it can be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The Human record the identity is rooted in.
    pub identity_root: String,
    pub identity_fingerprint: String,
    /// Fingerprints of other identities this device also speaks for. An
    /// approval here is never for those.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also_speaks_for: Vec<String>,
}

/// Whether this node may approve devices for anyone, and for whom.
///
/// A node lists asks only when it speaks for a person: as one of the root
/// controllers its identity's authority names, or as a device whose joining
/// record stands (every device a person has joined speaks for them, and any
/// of them may approve the next). A node with no identity, or whose joining
/// record no longer stands, speaks for nobody. A node that joined someone's
/// identity as it is speaks for two identities: approvals here are for the
/// one its key resolves to (the one it began), and the other is named with
/// every ask so the person is never left to guess which.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Speaks {
    Person(SpeaksFor),
    /// This node is a controller of no identity.
    Nobody,
    /// This node's standing has not been read recently enough to say.
    Unknown,
}

impl Speaks {
    /// From what this node's own cell says. `standing` is the authority of the
    /// identity the cell's agent resolves to, when it has one; `person` is the
    /// cell's own Human record. Only a standing that counts `me` among those
    /// that speak (a root controller, or this device through its standing
    /// joining record) makes this node speak for anyone. `identifier` is the
    /// person's sign-in word, shown and never compared.
    pub fn of(
        standing: Option<&ControllerStanding>,
        me: &str,
        person: Option<&ExistingIdentity>,
        identifier: Option<String>,
    ) -> Self {
        match standing {
            Some(s) if s.controllers.iter().any(|k| k == me) => Self::Person(SpeaksFor {
                identifier,
                human_id: person.map(|p| p.human_id.clone()),
                display_name: person.map(|p| p.display_name.clone()),
                identity_root: s.identity_root.clone(),
                identity_fingerprint: fingerprint(&s.identity_root),
                also_speaks_for: s.also_speaks_for.iter().map(|r| fingerprint(r)).collect(),
            }),
            _ => Self::Nobody,
        }
    }

    pub fn person(&self) -> Option<&SpeaksFor> {
        match self {
            Self::Person(p) => Some(p),
            _ => None,
        }
    }

    /// The sentence a person at this node is shown.
    pub fn words(&self) -> String {
        match self {
            Self::Person(p) => {
                // The sign-in word leads; the record id only when nothing
                // else names the person.
                let who = match (&p.identifier, &p.display_name, &p.human_id) {
                    (Some(word), Some(name), _) if word != name => format!("{word} ({name})"),
                    (Some(word), _, _) => word.clone(),
                    (None, Some(name), _) => name.clone(),
                    // Never the record id: a person reads this.
                    (None, None, _) => "a person".to_string(),
                };
                let mut words = format!(
                    "An approval here is for {who}, identity {}.",
                    p.identity_fingerprint
                );
                if !p.also_speaks_for.is_empty() {
                    words.push_str(&format!(
                        " This device also speaks for identity {}; to approve for that one, \
                         use another of its devices.",
                        p.also_speaks_for.join(", ")
                    ));
                }
                words
            }
            Self::Nobody => {
                "This device does not speak for anyone, so it lists no requests.".to_string()
            }
            Self::Unknown => "This device cannot say yet whom it speaks for, so it lists no \
                              requests for now."
                .to_string(),
        }
    }
}

/// What a device sends over a carrier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub request: GrantRequest,
    pub state: NodeState,
    /// When the device declared its approver, that approver's agent key. Any
    /// other node ignores the ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_approver: Option<String>,
}

/// One ask waiting on this node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAsk {
    /// A small number the person types to pick it.
    pub number: u32,
    pub ask: Ask,
    pub admitted: AdmittedRequest,
    /// The carrier's transport id, kept only to send the code back.
    pub source: String,
    pub received_at_micros: i64,
}

impl PendingAsk {
    pub fn expires_at_micros(&self) -> i64 {
        self.received_at_micros.saturating_add(ASK_LIFE_MICROS)
    }

    /// Whether `text` picks this ask: its number, its device key, or that
    /// key's fingerprint.
    pub fn picked_by(&self, text: &str) -> bool {
        let text = text.trim();
        let key = &self.ask.request.device_key;
        text == self.number.to_string() || text == key || text == fingerprint(key)
    }
}

/// Why an ask was not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dropped {
    /// This node speaks for nobody, so it lists nothing.
    SpeaksForNobody,
    /// This node's standing is not known recently enough to list anything.
    StandingUnknown,
    /// This very ask (its state token) was decided here already; the device
    /// asked again before its code reached it.
    AlreadyDecided,
    /// Not a request a portal would show a person.
    Malformed(RequestRefusal),
    /// Addressed to another approving node.
    NotForThisApprover,
    /// The list is full; dropped, not queued.
    Full,
    /// This source already holds its share.
    SourceOverLimit,
}

impl Dropped {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SpeaksForNobody => "ask_node_speaks_for_nobody",
            Self::StandingUnknown => "ask_node_standing_unknown",
            Self::AlreadyDecided => "ask_already_decided",
            Self::Malformed(r) => r.code(),
            Self::NotForThisApprover => "ask_for_another_approver",
            Self::Full => "ask_list_full",
            Self::SourceOverLimit => "ask_source_over_limit",
        }
    }
}

/// How an ask was listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listed {
    New(u32),
    /// The same device asked again; its earlier ask is replaced.
    Replaced(u32),
}

/// The approving node's pending asks, in memory only.
#[derive(Debug, Default)]
pub struct PendingAsks {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    next: u32,
    asks: BTreeMap<u32, PendingAsk>,
    /// The state tokens of asks already decided, with when, so the same ask
    /// arriving again while its code travels back is not listed anew.
    decided: std::collections::VecDeque<(String, i64)>,
}

/// How many decided asks are remembered.
pub const MAX_DECIDED: usize = 4 * MAX_PENDING;

impl PendingAsks {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Every critical section is a few map operations, so a panic inside
        // one cannot leave the list half-written.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// List `ask` from `source`, or say why not. `me` is this node's agent
    /// key, when it is known; `speaks` is whom this node speaks for. A node
    /// that speaks for nobody, or cannot yet say, lists nothing.
    pub fn admit(
        &self,
        ask: Ask,
        source: &str,
        me: Option<&str>,
        speaks: &Speaks,
        policy: &GrantPolicy,
        now_micros: i64,
    ) -> Result<Listed, Dropped> {
        match speaks {
            Speaks::Person(_) => {}
            Speaks::Nobody => return Err(Dropped::SpeaksForNobody),
            Speaks::Unknown => return Err(Dropped::StandingUnknown),
        }
        let admitted = admit_request(&ask.request, policy).map_err(Dropped::Malformed)?;
        {
            let mut inner = self.lock();
            // bounded-work: at most MAX_DECIDED entries, each an ask's life.
            inner
                .decided
                .retain(|(_, at)| now_micros < at.saturating_add(ASK_LIFE_MICROS));
            if inner
                .decided
                .iter()
                .any(|(state, _)| *state == ask.request.state)
            {
                return Err(Dropped::AlreadyDecided);
            }
        }
        if let Some(wanted) = &ask.for_approver {
            if me != Some(wanted.as_str()) {
                return Err(Dropped::NotForThisApprover);
            }
        }
        let mut inner = self.lock();
        // bounded-work: one pass over at most MAX_PENDING entries.
        inner.asks.retain(|_, p| now_micros < p.expires_at_micros());
        let device = ask.request.device_key.clone();
        let earlier = inner
            .asks
            .iter()
            .find(|(_, p)| p.ask.request.device_key == device)
            .map(|(n, p)| (*n, p.source.clone()));
        let from_source = inner.asks.values().filter(|p| p.source == source).count();
        let listed = match earlier {
            Some((n, _)) => {
                inner.asks.remove(&n);
                Listed::Replaced(n)
            }
            None if inner.asks.len() >= MAX_PENDING => return Err(Dropped::Full),
            None if from_source >= MAX_PER_SOURCE => return Err(Dropped::SourceOverLimit),
            None => {
                inner.next = inner.next.wrapping_add(1).max(1);
                Listed::New(inner.next)
            }
        };
        let number = match listed {
            Listed::New(n) | Listed::Replaced(n) => n,
        };
        inner.asks.insert(
            number,
            PendingAsk {
                number,
                ask,
                admitted,
                source: source.to_string(),
                received_at_micros: now_micros,
            },
        );
        Ok(listed)
    }

    /// The asks still waiting, oldest number first.
    pub fn list(&self, now_micros: i64) -> Vec<PendingAsk> {
        let mut inner = self.lock();
        inner.asks.retain(|_, p| now_micros < p.expires_at_micros());
        inner.asks.values().cloned().collect()
    }

    /// The waiting ask `text` picks, if exactly one does.
    pub fn pick(&self, text: &str, now_micros: i64) -> Option<PendingAsk> {
        let found: Vec<_> = self
            .list(now_micros)
            .into_iter()
            .filter(|p| p.picked_by(text))
            .collect();
        match found.as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        }
    }

    /// Take a decided ask off the list.
    pub fn remove(&self, number: u32) -> Option<PendingAsk> {
        self.lock().asks.remove(&number)
    }

    /// Take a decided ask off the list, and remember it was decided until its
    /// life ends, so the same ask is not listed again while its code travels.
    pub fn decided(&self, number: u32, now_micros: i64) -> Option<PendingAsk> {
        let mut inner = self.lock();
        let taken = inner.asks.remove(&number)?;
        if inner.decided.len() >= MAX_DECIDED {
            inner.decided.pop_front();
        }
        inner
            .decided
            .push_back((taken.ask.request.state.clone(), now_micros));
        Some(taken)
    }

    pub fn len(&self) -> usize {
        self.lock().asks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One waiting ask, as the approving node's terminal and portal are shown it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingView {
    pub number: u32,
    pub label: String,
    pub device_key: String,
    pub device_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_root_fingerprint: Option<String>,
    pub asked_acts: Vec<RequestedAct>,
    pub seconds_left: i64,
    pub state: NodeState,
    /// The sentence for `state`.
    pub state_words: String,
    /// Whether the device addressed this node by name.
    pub addressed_here: bool,
    /// Whose identity an approval of this ask here would be for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_identity: Option<SpeaksFor>,
}

impl PendingView {
    /// `speaks` is whom this node speaks for now, shown with the ask so
    /// nobody approves a device into the wrong identity.
    pub fn of(p: &PendingAsk, now_micros: i64, speaks: &Speaks) -> Self {
        let r = &p.ask.request;
        Self {
            number: p.number,
            label: r.label.clone(),
            device_key: r.device_key.clone(),
            device_fingerprint: fingerprint(&r.device_key),
            device_root_fingerprint: r.device_root_key.as_deref().map(fingerprint),
            asked_acts: r.acts.clone(),
            seconds_left: ((p.expires_at_micros() - now_micros) / 1_000_000).max(0),
            state: p.ask.state,
            state_words: p.ask.state.words().to_string(),
            addressed_here: p.ask.for_approver.is_some(),
            for_identity: speaks.person().cloned(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hash_shape::{sample_action, sample_key};
    use crate::request::tests::{policy, request, NETWORK};

    /// A node that speaks for the person rooted at `sample_action(1)`.
    pub(crate) fn speaks() -> Speaks {
        Speaks::of(
            Some(&standing(&[sample_key(9)])),
            &sample_key(9),
            Some(&person()),
            Some("matthew".into()),
        )
    }

    fn standing(controllers: &[String]) -> ControllerStanding {
        ControllerStanding {
            identity_root: sample_action(1),
            authority: sample_action(2),
            network_dna: NETWORK.into(),
            controllers: controllers.to_vec(),
            required: 1,
            speaks_via: None,
            also_speaks_for: vec![],
        }
    }

    fn person() -> ExistingIdentity {
        ExistingIdentity {
            human_id: "h-4c1d".into(),
            display_name: "Matthew".into(),
            profile_reach: "private".into(),
        }
    }

    pub(crate) fn ask(device: u8) -> Ask {
        let mut r = request();
        r.device_key = sample_key(device);
        Ask {
            request: r,
            state: NodeState::Unassigned,
            for_approver: None,
        }
    }

    const NOW: i64 = 1_000_000;

    #[test]
    fn an_ask_is_listed_and_the_same_device_replaces_its_own() {
        let list = PendingAsks::new();
        assert_eq!(
            list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW),
            Ok(Listed::New(1))
        );
        assert_eq!(
            list.admit(ask(2), "peer-a", None, &speaks(), &policy(), NOW),
            Ok(Listed::New(2))
        );
        assert_eq!(
            list.admit(ask(1), "peer-b", None, &speaks(), &policy(), NOW + 5),
            Ok(Listed::Replaced(1))
        );
        let listed = list.list(NOW + 6);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].source, "peer-b");
    }

    #[test]
    fn the_list_is_bounded_by_count_source_and_life() {
        let list = PendingAsks::new();
        for d in 0..MAX_PER_SOURCE as u8 {
            list.admit(ask(d + 1), "peer-a", None, &speaks(), &policy(), NOW)
                .unwrap();
        }
        assert_eq!(
            list.admit(ask(90), "peer-a", None, &speaks(), &policy(), NOW),
            Err(Dropped::SourceOverLimit)
        );
        for d in 0..(MAX_PENDING - MAX_PER_SOURCE) as u8 {
            list.admit(
                ask(d + 20),
                &format!("peer-{d}"),
                None,
                &speaks(),
                &policy(),
                NOW,
            )
            .unwrap();
        }
        assert_eq!(list.len(), MAX_PENDING);
        assert_eq!(
            list.admit(ask(99), "peer-z", None, &speaks(), &policy(), NOW),
            Err(Dropped::Full)
        );
        // Five minutes later they are gone, and room is made.
        assert!(list.list(NOW + ASK_LIFE_MICROS).is_empty());
        assert!(list
            .admit(
                ask(99),
                "peer-z",
                None,
                &speaks(),
                &policy(),
                NOW + ASK_LIFE_MICROS
            )
            .is_ok());
    }

    #[test]
    fn an_ask_for_another_approver_or_a_malformed_one_is_dropped() {
        let list = PendingAsks::new();
        let mut addressed = ask(1);
        addressed.for_approver = Some(sample_key(9));
        assert_eq!(
            list.admit(
                addressed.clone(),
                "p",
                Some(&sample_key(8)),
                &speaks(),
                &policy(),
                NOW
            ),
            Err(Dropped::NotForThisApprover)
        );
        assert_eq!(
            list.admit(addressed.clone(), "p", None, &speaks(), &policy(), NOW),
            Err(Dropped::NotForThisApprover)
        );
        assert!(list
            .admit(
                addressed,
                "p",
                Some(&sample_key(9)),
                &speaks(),
                &policy(),
                NOW
            )
            .is_ok());
        let mut bad = ask(2);
        bad.request.client_id = "stranger".into();
        assert_eq!(
            list.admit(bad, "p", None, &speaks(), &policy(), NOW)
                .unwrap_err()
                .code(),
            "request_client_unknown"
        );
    }

    #[test]
    fn an_ask_is_picked_by_number_key_or_fingerprint_and_never_by_its_source() {
        let list = PendingAsks::new();
        list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW)
            .unwrap();
        let key = sample_key(1);
        for text in ["1", key.as_str(), fingerprint(&key).as_str()] {
            assert!(list.pick(text, NOW).is_some(), "{text}");
        }
        assert!(list.pick("peer-a", NOW).is_none());
        assert!(list.pick("2", NOW).is_none());
    }

    #[test]
    fn a_node_says_what_it_is_and_unknown_is_never_nothing() {
        let me = sample_key(1);
        assert_eq!(
            NodeState::of(false, None, false, &me, Made::Unknown),
            NodeState::Unassigned
        );
        assert_eq!(
            NodeState::of(true, None, false, &me, Made::Unknown),
            NodeState::OwnIdentity {
                made: Made::Unknown
            }
        );
        assert_eq!(
            NodeState::of(
                true,
                Some(std::slice::from_ref(&me)),
                false,
                &me,
                Made::Things
            ),
            NodeState::OwnIdentity { made: Made::Things }
        );
        assert_eq!(
            NodeState::of(true, Some(&[sample_key(2)]), false, &me, Made::Unknown),
            NodeState::Joined
        );
        // A joined device is counted among those that speak, and is still a
        // device of someone's, not the beginning of an identity.
        assert_eq!(
            NodeState::of(
                true,
                Some(&[sample_key(2), me.clone()]),
                true,
                &me,
                Made::Unknown
            ),
            NodeState::Joined
        );
        let json = serde_json::to_value(NodeState::OwnIdentity {
            made: Made::Unknown,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "own-identity", "made": "unknown"})
        );
    }

    #[test]
    fn the_view_shows_the_device_its_acts_and_its_state() {
        let list = PendingAsks::new();
        list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW)
            .unwrap();
        let view = PendingView::of(&list.list(NOW)[0], NOW + 60_000_000, &speaks());
        assert_eq!(view.seconds_left, 240);
        assert_eq!(view.device_fingerprint, fingerprint(&sample_key(1)));
        let json = serde_json::to_value(&view).unwrap();
        for field in [
            "number",
            "deviceFingerprint",
            "askedActs",
            "secondsLeft",
            "state",
            "stateWords",
        ] {
            assert!(json.get(field).is_some(), "{field}");
        }
    }

    #[test]
    fn a_node_lists_only_when_it_speaks_for_a_person() {
        let list = PendingAsks::new();
        assert_eq!(
            list.admit(ask(1), "peer-a", None, &Speaks::Nobody, &policy(), NOW),
            Err(Dropped::SpeaksForNobody)
        );
        assert_eq!(
            list.admit(ask(1), "peer-a", None, &Speaks::Unknown, &policy(), NOW),
            Err(Dropped::StandingUnknown)
        );
        assert!(list.is_empty());
        assert_eq!(
            Dropped::SpeaksForNobody.code(),
            "ask_node_speaks_for_nobody"
        );
        assert_eq!(Dropped::StandingUnknown.code(), "ask_node_standing_unknown");
        assert_eq!(
            list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW),
            Ok(Listed::New(1))
        );
    }

    #[test]
    fn a_node_speaks_only_for_the_identity_whose_authority_names_it() {
        let me = sample_key(9);
        // A controller of its own identity, joined to someone else's as it is
        // or not: it speaks for the identity it is a controller of.
        let mine = Speaks::of(
            Some(&standing(std::slice::from_ref(&me))),
            &me,
            Some(&person()),
            Some("matthew".into()),
        );
        let p = mine.person().expect("speaks for a person");
        assert_eq!(p.identity_root, sample_action(1));
        assert_eq!(p.identity_fingerprint, fingerprint(&sample_action(1)));
        assert_eq!(p.human_id.as_deref(), Some("h-4c1d"));
        // The sign-in word leads, the name follows, the record id stays out.
        assert_eq!(
            mine.words(),
            format!(
                "An approval here is for matthew (Matthew), identity {}.",
                fingerprint(&sample_action(1))
            )
        );
        assert!(!mine.words().contains("h-4c1d"));
        // No word recorded: the name alone. No name either: the record id.
        let unnamed = Speaks::of(
            Some(&standing(std::slice::from_ref(&me))),
            &me,
            Some(&person()),
            None,
        );
        assert!(unnamed
            .words()
            .starts_with("An approval here is for Matthew,"));
        let bare = Speaks::of(Some(&standing(std::slice::from_ref(&me))), &me, None, None);
        assert!(bare
            .words()
            .starts_with("An approval here is for a person,"));
        // A key whose standing does not count it among those that speak (a
        // device whose joining record no longer stands).
        assert_eq!(
            Speaks::of(Some(&standing(&[sample_key(3)])), &me, None, None),
            Speaks::Nobody
        );
        // A joined device whose record stands speaks for the person.
        let mut joined = standing(&[sample_key(3), me.clone()]);
        joined.speaks_via = Some(sample_action(4));
        assert!(matches!(
            Speaks::of(Some(&joined), &me, None, None),
            Speaks::Person(_)
        ));
        // No identity at all.
        assert_eq!(Speaks::of(None, &me, None, None), Speaks::Nobody);
        assert!(Speaks::Nobody.words().contains("does not speak for anyone"));
    }

    #[test]
    fn the_view_names_whose_identity_an_approval_is_for() {
        let list = PendingAsks::new();
        list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW)
            .unwrap();
        let view = PendingView::of(&list.list(NOW)[0], NOW, &speaks());
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(
            json["forIdentity"],
            serde_json::json!({
                "identifier": "matthew",
                "humanId": "h-4c1d",
                "displayName": "Matthew",
                "identityRoot": sample_action(1),
                "identityFingerprint": fingerprint(&sample_action(1)),
            })
        );
        let wire = serde_json::to_value(speaks()).unwrap();
        assert_eq!(wire["kind"], "person");
        assert_eq!(wire["identityRoot"], sample_action(1));
        assert_eq!(
            serde_json::to_value(Speaks::Nobody).unwrap(),
            serde_json::json!({"kind": "nobody"})
        );
        let back: Speaks = serde_json::from_value(wire).unwrap();
        assert_eq!(back, speaks());
    }

    #[test]
    fn a_decided_ask_is_not_listed_again_while_its_code_travels() {
        let list = PendingAsks::new();
        list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW)
            .unwrap();
        assert!(list.decided(1, NOW).is_some());
        assert_eq!(
            list.admit(ask(1), "peer-a", None, &speaks(), &policy(), NOW + 1),
            Err(Dropped::AlreadyDecided)
        );
        assert_eq!(Dropped::AlreadyDecided.code(), "ask_already_decided");
        // A new ask from the same device (a new state token) is listed.
        let mut fresh = ask(1);
        fresh.request.state = "t".repeat(32);
        assert!(list
            .admit(fresh, "peer-a", None, &speaks(), &policy(), NOW + 1)
            .is_ok());
        // Once the decided ask's life is over it is forgotten.
        assert!(list
            .admit(
                ask(1),
                "peer-a",
                None,
                &speaks(),
                &policy(),
                NOW + ASK_LIFE_MICROS
            )
            .is_ok());
    }
}
