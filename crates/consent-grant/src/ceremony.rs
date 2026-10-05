//! The ceremony: the three steps a portal performs, written once so every
//! portal performs them the same way.
//!
//! A single peer runtime can run the whole ceremony alone: the person's own
//! node shows the consent screen, signs as itself, and holds the delivery in
//! memory. A doorway mounts the same steps for people it hosts and adds only
//! how the hosted node is asked to sign. No step lives in one host and not the
//! other.
//!
//! 1. [`ConsentView::of`] — what the consent screen shows for a request.
//! 2. [`issue`] — after the controller has agreed and signed, hold the signed
//!    record for the asking terminal and say how the code goes back.
//! 3. [`settle`] — when a terminal presents a code, decide what it receives.
//!
//! Signing happens between steps 1 and 2 and belongs to the host, because only
//! the host can reach the controller's key. The host builds the record with
//! [`ConsentRecord::agree`] and runs the witness beat
//! ([`crate::witness::attend`]) on it first: a beat that pauses stops the
//! ceremony there, before anything is signed or issued, and the person is
//! asked to sign in again. Otherwise the host has the controller sign the
//! record's [`consent_message`](crate::consent::consent_message) and, when
//! enrolling was agreed, the [`EnrollmentIntent`] derived from it, and passes
//! the result to [`issue`].
//!
//! One controller's signature is enough to issue. An identity with several
//! controllers may have the others affirm the device later; the ceremony never
//! waits for them.
//!
//! The store is one atomic primitive, [`Taken`]: take the delivery for a code
//! and mark it redeemed in the same step. [`MemoryStore`] does this for a
//! single runtime. A host with its own store implements the same take.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::act::RequestedAct;
use crate::consent::{SignedConsent, SignerRole};
use crate::delivery::{
    admit_redemption, code_digest, DeliveryRefusal, PendingDelivery, Redemption, RedemptionRefusal,
};
use crate::enrollment::{Enrollment, EnrollmentIntent};
use crate::hash_shape;
use crate::request::AdmittedRequest;
use crate::return_path::{return_target, ReturnTarget};

/// What a consent screen shows. Everything a person needs to decide, and
/// nothing they could not check against their own terminal. This is the wire
/// form every host returns, so one consent screen serves them all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentView {
    pub client_id: String,
    pub label: String,
    pub device_fingerprint: String,
    /// Shown only when binding the device root is asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_root_fingerprint: Option<String>,
    /// Each thing asked for, to be agreed to or declined separately.
    pub asked_acts: Vec<RequestedAct>,
}

impl ConsentView {
    pub fn of(admitted: &AdmittedRequest) -> Self {
        let r = admitted.request();
        Self {
            client_id: r.client_id.clone(),
            label: r.label.clone(),
            device_fingerprint: admitted.device_fingerprint(),
            device_root_fingerprint: r.device_root_key.as_deref().map(hash_shape::fingerprint),
            asked_acts: r.acts.clone(),
        }
    }
}

/// A signed consent held for collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub delivery: PendingDelivery,
    pub consent: SignedConsent,
    /// The enrollment the consent agreed to, with its controller proofs.
    pub enrollment: Option<Enrollment>,
}

/// What a terminal collects with its code: the signed consent and, when
/// enrolling was agreed, the enrollment it completes on its own cell by adding
/// its possession proof. The consent's fields sit at the top level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivered {
    #[serde(flatten)]
    pub consent: SignedConsent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment: Option<Enrollment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueRefusal {
    /// The record does not hash to its stated address.
    AddressBroken,
    /// The record was not made for this request.
    RequestMismatch,
    /// No controller has signed it. Without a controller's signature a record
    /// is not a consent, whoever else has signed.
    Unsigned,
    /// The enrollment offered is not the one the record agreed to, is missing
    /// although enrolling was agreed, or carries no controller proof.
    EnrollmentMismatch,
    Delivery(DeliveryRefusal),
}

impl IssueRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::AddressBroken => "issue_address_broken",
            Self::RequestMismatch => "issue_request_mismatch",
            Self::Unsigned => "issue_unsigned",
            Self::EnrollmentMismatch => "issue_enrollment_mismatch",
            Self::Delivery(d) => d.code(),
        }
    }
}

/// Hold a signed consent for the terminal that asked, and say how the code
/// returns to it.
///
/// `code` is the host's freshly drawn random token. The caller stores the
/// returned [`Held`] under its code digest and sends the person to the
/// returned target.
pub fn issue(
    admitted: &AdmittedRequest,
    consent: SignedConsent,
    enrollment: Option<Enrollment>,
    code: &str,
    now_micros: i64,
    ttl_micros: i64,
) -> Result<(Held, ReturnTarget), IssueRefusal> {
    let r = admitted.request();
    if !consent.address_holds() {
        return Err(IssueRefusal::AddressBroken);
    }
    if consent.record.request_binding != r.code_challenge
        || consent.record.device_key != r.device_key
        || consent.record.client_id != r.client_id
    {
        return Err(IssueRefusal::RequestMismatch);
    }
    if !consent.controller_signed() {
        return Err(IssueRefusal::Unsigned);
    }
    let enrollment_agreed = EnrollmentIntent::agreed_in(&consent.record).is_some();
    let enrollment_holds = match &enrollment {
        Some(e) => e.is_agreed_in(&consent.record),
        None => !enrollment_agreed,
    };
    if !enrollment_holds {
        return Err(IssueRefusal::EnrollmentMismatch);
    }
    let delivery = PendingDelivery::issue(admitted, &consent.cid, code, now_micros, ttl_micros)
        .map_err(IssueRefusal::Delivery)?;
    let target = return_target(r.return_path, code, &r.state);
    Ok((
        Held {
            delivery,
            consent,
            enrollment,
        },
        target,
    ))
}

/// What a store hands back when asked to take the delivery for a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Taken {
    /// It was waiting, and is now marked redeemed. Only one caller ever
    /// receives this for a given code.
    Fresh(Box<Held>),
    /// It had already been taken.
    AlreadyRedeemed,
    Unknown,
}

/// Decide what a terminal receives for the code it presented.
///
/// By the time this runs the delivery is already consumed, so every refusal
/// leaves the code unusable. A refusal that [`RedemptionRefusal::burns_delivery`]
/// also tells the host to drop what it holds.
pub fn settle(
    taken: Taken,
    redemption: &Redemption,
    now_micros: i64,
) -> Result<Delivered, RedemptionRefusal> {
    match taken {
        Taken::Unknown => Err(RedemptionRefusal::CodeUnknown),
        Taken::AlreadyRedeemed => Err(RedemptionRefusal::AlreadyRedeemed),
        Taken::Fresh(held) => {
            admit_redemption(&held.delivery, redemption, now_micros)?;
            let held = *held;
            Ok(Delivered {
                consent: held.consent,
                enrollment: held.enrollment,
            })
        }
    }
}

/// The delivery store for a single runtime. Nothing is written to disk: a
/// delivery lasts minutes, and one lost to a restart is recovered by asking
/// again.
#[derive(Debug, Default)]
pub struct MemoryStore {
    held: Mutex<HashMap<String, Held>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold a delivery. One request has one live delivery: anything already
    /// held for the same request (the same PKCE challenge) is dropped, so a
    /// person who agrees again leaves only the newest code working.
    pub fn insert(&self, held: Held) {
        let mut map = self.lock();
        // bounded-work: one pass over deliveries that each live minutes.
        map.retain(|_, h| h.delivery.code_challenge != held.delivery.code_challenge);
        map.insert(held.delivery.code_digest.clone(), held);
    }

    /// Take the delivery for `code` and mark it redeemed, in one step.
    pub fn take(&self, code: &str) -> Taken {
        let mut held = self.lock();
        match held.get_mut(&code_digest(code)) {
            None => Taken::Unknown,
            Some(entry) if entry.delivery.redeemed => Taken::AlreadyRedeemed,
            Some(entry) => {
                let fresh = entry.clone();
                entry.delivery.redeemed = true;
                Taken::Fresh(Box::new(fresh))
            }
        }
    }

    /// Drop what is held for `code`, after a refusal that burns it.
    pub fn discard(&self, code: &str) {
        self.lock().remove(&code_digest(code));
    }

    /// Drop everything past its window. Returns how many were dropped.
    pub fn sweep(&self, now_micros: i64) -> usize {
        let mut held = self.lock();
        let before = held.len();
        held.retain(|_, h| now_micros < h.delivery.expires_at_micros);
        before - held.len()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Held>> {
        // A panic while holding this lock cannot leave the map half-written:
        // every critical section is a single map operation.
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Run a redemption against a [`MemoryStore`] end to end, burning the delivery
/// when the refusal calls for it.
pub fn redeem(
    store: &MemoryStore,
    redemption: &Redemption,
    now_micros: i64,
) -> Result<Delivered, RedemptionRefusal> {
    let outcome = settle(store.take(&redemption.code), redemption, now_micros);
    if let Err(refusal) = outcome {
        if refusal.burns_delivery() {
            store.discard(&redemption.code);
        }
    }
    outcome
}

/// Where the code goes once the person has agreed, as the consent screen is
/// told it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ReturnTargetView {
    /// Show `value` for the person to paste into their terminal.
    Display { value: String },
    /// Send the browser to `url`, the terminal's own listener.
    Redirect { url: String },
}

impl From<ReturnTarget> for ReturnTargetView {
    fn from(target: ReturnTarget) -> Self {
        match target {
            ReturnTarget::Display(value) => Self::Display { value },
            ReturnTarget::Redirect(url) => Self::Redirect { url },
        }
    }
}

/// How many controllers the identity's policy asks to agree, and how many
/// signatures this consent carries. Fewer signed than required is not a
/// refusal: the others may affirm later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerTally {
    pub required: usize,
    pub signed: usize,
}

/// Who a signer is to the person reading the consent screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relation {
    /// The node the person is using to agree.
    ThisDevice,
    /// Another of the person's own nodes.
    YourDevice,
    /// Someone else who attests to the agreement.
    VouchesForYou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendanceAct {
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendanceState {
    Done,
}

/// One party that signed the consent, as the consent screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendance {
    /// The signer's key.
    pub id: String,
    pub act: AttendanceAct,
    pub relation: Relation,
    pub state: AttendanceState,
}

/// Every party that actually signed `consent`, in the order they signed.
/// `this_node` is the key of the node the person agreed on. Nobody who did not
/// sign is listed.
pub fn attendance(consent: &SignedConsent, this_node: &str) -> Vec<Attendance> {
    consent
        .signatures
        .iter()
        .map(|s| Attendance {
            id: s.signer.clone(),
            act: AttendanceAct::Signed,
            relation: match s.role {
                _ if s.signer == this_node => Relation::ThisDevice,
                SignerRole::Controller | SignerRole::Device => Relation::YourDevice,
                SignerRole::Witness => Relation::VouchesForYou,
            },
            state: AttendanceState::Done,
        })
        .collect()
}

/// What the consent screen is told once the person has agreed. The same wire
/// form from every host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgreedView {
    pub return_target: ReturnTargetView,
    /// When the code stops working, in epoch milliseconds.
    pub expires_at: i64,
    pub consent_cid: String,
    pub controllers: ControllerTally,
    pub witnesses: Vec<Attendance>,
}

impl AgreedView {
    /// Describe what [`issue`] held. `required` is the identity's declared
    /// controller policy; `this_node` the key of the node that agreed.
    pub fn of(held: &Held, target: ReturnTarget, required: usize, this_node: &str) -> Self {
        Self {
            return_target: target.into(),
            expires_at: held.delivery.expires_at_micros.div_euclid(1000),
            consent_cid: held.consent.cid.clone(),
            controllers: ControllerTally {
                required,
                signed: held.consent.signed_in(SignerRole::Controller),
            },
            witnesses: attendance(&held.consent, this_node),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::peer_record;
    use crate::consent::ConsentSignature;
    use crate::enrollment::ControllerProof;
    use crate::hash_shape::sample_key;
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy, request, VERIFIER};
    use crate::return_path::ReturnPath;

    const CODE: &str = "c0dec0dec0dec0dec0dec0dec0dec0de";
    const NOW: i64 = 1_000_000;
    const TTL: i64 = 300_000_000;

    fn admitted() -> AdmittedRequest {
        admit_request(&peer_request(), &policy()).unwrap()
    }

    fn signed() -> SignedConsent {
        SignedConsent::new(peer_record())
            .unwrap()
            .with_signature(ConsentSignature {
                role: SignerRole::Controller,
                signer: sample_key(9),
                signature: "controller".into(),
            })
    }

    fn enrollment() -> Option<Enrollment> {
        Some(Enrollment {
            intent: EnrollmentIntent::agreed_in(&peer_record()).unwrap(),
            controllers: vec![ControllerProof {
                agent: sample_key(9),
                signature: "enrollment".into(),
            }],
        })
    }

    fn redemption() -> Redemption {
        Redemption {
            code: CODE.into(),
            code_verifier: VERIFIER.into(),
            client_id: "epr-cli".into(),
            device_key: sample_key(7),
        }
    }

    /// A store holding one issued consent, as a runtime would after approval.
    fn store() -> MemoryStore {
        let (held, _) = issue(&admitted(), signed(), enrollment(), CODE, NOW, TTL).unwrap();
        let store = MemoryStore::new();
        store.insert(held);
        store
    }

    #[test]
    fn the_view_shows_what_the_person_must_decide() {
        let view = ConsentView::of(&admitted());
        assert_eq!(view.label, "Matthew's workspace");
        assert_eq!(view.asked_acts.len(), 2);
        assert!(view.device_root_fingerprint.is_some());
        let plain = admit_request(&request(), &policy()).unwrap();
        assert_eq!(ConsentView::of(&plain).device_root_fingerprint, None);
    }

    #[test]
    fn one_runtime_runs_the_whole_ceremony() {
        let store = store();
        let delivered = redeem(&store, &redemption(), NOW + 1).unwrap();
        assert_eq!(delivered.consent, signed());
        assert!(delivered.consent.address_holds());
        assert_eq!(delivered.enrollment, enrollment());
    }

    #[test]
    fn issuing_says_how_the_code_goes_back() {
        let (_, target) = issue(&admitted(), signed(), enrollment(), CODE, NOW, TTL).unwrap();
        let state = "s".repeat(32);
        assert_eq!(target, ReturnTarget::Display(format!("{CODE}#{state}")));

        let mut local = peer_request();
        local.return_path = ReturnPath::Loopback { port: 49152 };
        let local = admit_request(&local, &policy()).unwrap();
        let (_, target) = issue(&local, signed(), enrollment(), CODE, NOW, TTL).unwrap();
        assert_eq!(
            target,
            ReturnTarget::Redirect(format!(
                "http://127.0.0.1:49152/callback?code={CODE}&state={state}"
            ))
        );
    }

    #[test]
    fn only_a_signed_record_made_for_this_request_is_issued() {
        use IssueRefusal as R;
        let unsigned = SignedConsent::new(peer_record()).unwrap();
        assert_eq!(
            issue(&admitted(), unsigned.clone(), enrollment(), CODE, NOW, TTL),
            Err(R::Unsigned)
        );
        // The device's own signature, or a witness's, is not agreement.
        let device_only = unsigned.with_signature(ConsentSignature {
            role: SignerRole::Device,
            signer: sample_key(7),
            signature: "device".into(),
        });
        assert_eq!(
            issue(&admitted(), device_only, enrollment(), CODE, NOW, TTL),
            Err(R::Unsigned)
        );

        let mut tampered = signed();
        tampered.record.agreed_acts = vec![RequestedAct::EnrollDevice];
        assert_eq!(
            issue(&admitted(), tampered, enrollment(), CODE, NOW, TTL),
            Err(R::AddressBroken)
        );

        // A record agreed for another terminal's request.
        let mut other = peer_request();
        other.code_challenge = crate::pkce::challenge(&"v".repeat(43));
        let other = admit_request(&other, &policy()).unwrap();
        assert_eq!(
            issue(&other, signed(), enrollment(), CODE, NOW, TTL),
            Err(R::RequestMismatch)
        );

        assert_eq!(
            issue(&admitted(), signed(), enrollment(), "short", NOW, TTL),
            Err(R::Delivery(DeliveryRefusal::CodeWeak))
        );
    }

    #[test]
    fn an_agreed_enrollment_travels_with_the_consent_and_must_match_it() {
        use IssueRefusal as R;
        assert_eq!(
            issue(&admitted(), signed(), None, CODE, NOW, TTL),
            Err(R::EnrollmentMismatch)
        );
        let mut foreign = enrollment().unwrap();
        foreign.intent.device_key = sample_key(8);
        assert_eq!(
            issue(&admitted(), signed(), Some(foreign), CODE, NOW, TTL),
            Err(R::EnrollmentMismatch)
        );
        let mut unproven = enrollment().unwrap();
        unproven.controllers.clear();
        assert_eq!(
            issue(&admitted(), signed(), Some(unproven), CODE, NOW, TTL),
            Err(R::EnrollmentMismatch)
        );
    }

    #[test]
    fn the_terminal_collects_the_consent_fields_and_the_enrollment() {
        let delivered = redeem(&store(), &redemption(), NOW + 1).unwrap();
        let json = serde_json::to_value(&delivered).unwrap();
        for field in ["cid", "record", "signatures", "enrollment"] {
            assert!(json.get(field).is_some(), "{field}");
        }
        assert_eq!(json["enrollment"]["intent"]["deviceKey"], sample_key(7));
        assert_eq!(json["enrollment"]["controllers"][0]["agent"], sample_key(9));
    }

    #[test]
    fn agreeing_again_replaces_the_earlier_code() {
        let store = store();
        const SECOND: &str = "5ec0nd5ec0nd5ec0nd5ec0nd5ec0nd5e";
        let (held, _) = issue(&admitted(), signed(), enrollment(), SECOND, NOW, TTL).unwrap();
        store.insert(held);
        assert_eq!(store.len(), 1);
        assert_eq!(
            redeem(&store, &redemption(), NOW + 1),
            Err(RedemptionRefusal::CodeUnknown)
        );
        let mut second = redemption();
        second.code = SECOND.into();
        assert!(redeem(&store, &second, NOW + 1).is_ok());

        // Another request's delivery is left alone.
        let mut other = peer_request();
        other.code_challenge = crate::pkce::challenge(&"v".repeat(43));
        let other = admit_request(&other, &policy()).unwrap();
        let mut record = peer_record();
        record.request_binding = other.request().code_challenge.clone();
        let consent = SignedConsent::new(record)
            .unwrap()
            .with_signature(signed().signatures[0].clone());
        let (held, _) = issue(&other, consent, enrollment(), CODE, NOW, TTL).unwrap();
        store.insert(held);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn the_agreed_view_lists_only_who_signed() {
        let (held, target) = issue(&admitted(), signed(), enrollment(), CODE, NOW, TTL).unwrap();
        let view = AgreedView::of(&held, target, 1, &sample_key(9));
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["returnTarget"]["kind"], "display");
        assert_eq!(
            json["returnTarget"]["value"],
            format!("{CODE}#{}", "s".repeat(32))
        );
        assert_eq!(json["expiresAt"], (NOW + TTL) / 1000);
        assert_eq!(json["consentCid"], held.consent.cid);
        assert_eq!(json["controllers"]["required"], 1);
        assert_eq!(json["controllers"]["signed"], 1);
        assert_eq!(
            json["witnesses"],
            serde_json::json!([{
                "id": sample_key(9),
                "act": "signed",
                "relation": "this-device",
                "state": "done"
            }])
        );
    }

    #[test]
    fn a_policy_asking_for_more_controllers_still_issues_on_one() {
        let (held, target) = issue(&admitted(), signed(), enrollment(), CODE, NOW, TTL).unwrap();
        let view = AgreedView::of(&held, target, 2, &sample_key(9));
        assert_eq!(
            view.controllers,
            ControllerTally {
                required: 2,
                signed: 1
            }
        );
    }

    #[test]
    fn signers_are_named_by_their_relation_to_the_person() {
        let consent = signed()
            .with_signature(ConsentSignature {
                role: SignerRole::Controller,
                signer: sample_key(10),
                signature: "other-node".into(),
            })
            .with_signature(ConsentSignature {
                role: SignerRole::Witness,
                signer: sample_key(5),
                signature: "elohim".into(),
            });
        let relations: Vec<_> = attendance(&consent, &sample_key(9))
            .into_iter()
            .map(|a| a.relation)
            .collect();
        assert_eq!(
            relations,
            [
                Relation::ThisDevice,
                Relation::YourDevice,
                Relation::VouchesForYou
            ]
        );
        let redirect: ReturnTargetView =
            ReturnTarget::Redirect("http://127.0.0.1:5/cb".into()).into();
        assert_eq!(
            serde_json::to_value(redirect).unwrap(),
            serde_json::json!({"kind": "redirect", "url": "http://127.0.0.1:5/cb"})
        );
    }

    #[test]
    fn a_code_is_taken_once() {
        let store = store();
        assert!(matches!(store.take(CODE), Taken::Fresh(_)));
        assert_eq!(store.take(CODE), Taken::AlreadyRedeemed);
        assert_eq!(store.take(&"d".repeat(32)), Taken::Unknown);
    }

    #[test]
    fn a_second_presentation_is_refused_and_burns_what_is_held() {
        let store = store();
        assert!(redeem(&store, &redemption(), NOW + 1).is_ok());
        assert_eq!(
            redeem(&store, &redemption(), NOW + 2),
            Err(RedemptionRefusal::AlreadyRedeemed)
        );
        assert!(store.is_empty());
    }

    #[test]
    fn a_stranger_with_the_code_gets_nothing_and_spoils_it_for_everyone() {
        let store = store();
        let mut stranger = redemption();
        stranger.code_verifier = "x".repeat(43);
        assert_eq!(
            redeem(&store, &stranger, NOW + 1),
            Err(RedemptionRefusal::VerifierMismatch)
        );
        // The asking terminal can no longer use it either.
        assert_eq!(
            redeem(&store, &redemption(), NOW + 2),
            Err(RedemptionRefusal::CodeUnknown)
        );
    }

    #[test]
    fn an_expired_code_is_refused_and_swept() {
        let store = store();
        assert_eq!(store.sweep(NOW + 1), 0);
        assert_eq!(
            redeem(&store, &redemption(), NOW + TTL),
            Err(RedemptionRefusal::Expired)
        );
        assert_eq!(store.sweep(NOW + TTL), 1);
        assert!(store.is_empty());
    }

    #[test]
    fn concurrent_redemptions_of_one_code_yield_one_consent() {
        let store = std::sync::Arc::new(store());
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let store = std::sync::Arc::clone(&store);
                std::thread::spawn(move || redeem(&store, &redemption(), NOW + 1).is_ok())
            })
            .collect();
        let delivered = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count();
        assert_eq!(delivered, 1);
    }
}
