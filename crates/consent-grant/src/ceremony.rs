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
//! [`ConsentRecord::agree`], has the controller sign its canonical bytes, and
//! passes the result to [`issue`].
//!
//! The store is one atomic primitive, [`Taken`]: take the delivery for a code
//! and mark it redeemed in the same step. [`MemoryStore`] does this for a
//! single runtime. A host with its own store implements the same take.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::act::RequestedAct;
use crate::consent::SignedConsent;
use crate::delivery::{
    admit_redemption, code_digest, DeliveryRefusal, PendingDelivery, Redemption, RedemptionRefusal,
};
use crate::hash_shape;
use crate::request::AdmittedRequest;
use crate::return_path::{return_target, ReturnTarget};

/// What a consent screen shows. Everything a person needs to decide, and
/// nothing they could not check against their own terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentView {
    pub client_id: String,
    pub label: String,
    pub device_fingerprint: String,
    /// Shown only when binding the device root is asked for.
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueRefusal {
    /// The record does not hash to its stated address.
    AddressBroken,
    /// The record was not made for this request.
    RequestMismatch,
    /// Nobody has signed it. An unsigned record is not a consent.
    Unsigned,
    Delivery(DeliveryRefusal),
}

impl IssueRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::AddressBroken => "issue_address_broken",
            Self::RequestMismatch => "issue_request_mismatch",
            Self::Unsigned => "issue_unsigned",
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
    if consent.signatures.is_empty() {
        return Err(IssueRefusal::Unsigned);
    }
    let delivery = PendingDelivery::issue(admitted, &consent.cid, code, now_micros, ttl_micros)
        .map_err(IssueRefusal::Delivery)?;
    let target = return_target(r.return_path, code, &r.state);
    Ok((Held { delivery, consent }, target))
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
) -> Result<SignedConsent, RedemptionRefusal> {
    match taken {
        Taken::Unknown => Err(RedemptionRefusal::CodeUnknown),
        Taken::AlreadyRedeemed => Err(RedemptionRefusal::AlreadyRedeemed),
        Taken::Fresh(held) => {
            admit_redemption(&held.delivery, redemption, now_micros)?;
            Ok(held.consent)
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

    pub fn insert(&self, held: Held) {
        self.lock().insert(held.delivery.code_digest.clone(), held);
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
) -> Result<SignedConsent, RedemptionRefusal> {
    let outcome = settle(store.take(&redemption.code), redemption, now_micros);
    if let Err(refusal) = outcome {
        if refusal.burns_delivery() {
            store.discard(&redemption.code);
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::peer_record;
    use crate::consent::ConsentSignature;
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
                signer: sample_key(9),
                signature: "controller".into(),
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
        let (held, _) = issue(&admitted(), signed(), CODE, NOW, TTL).unwrap();
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
        let consent = redeem(&store, &redemption(), NOW + 1).unwrap();
        assert_eq!(consent, signed());
        assert!(consent.address_holds());
    }

    #[test]
    fn issuing_says_how_the_code_goes_back() {
        let (_, target) = issue(&admitted(), signed(), CODE, NOW, TTL).unwrap();
        let state = "s".repeat(32);
        assert_eq!(target, ReturnTarget::Display(format!("{CODE}#{state}")));

        let mut local = peer_request();
        local.return_path = ReturnPath::Loopback { port: 49152 };
        let local = admit_request(&local, &policy()).unwrap();
        let (_, target) = issue(&local, signed(), CODE, NOW, TTL).unwrap();
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
            issue(&admitted(), unsigned, CODE, NOW, TTL),
            Err(R::Unsigned)
        );

        let mut tampered = signed();
        tampered.record.agreed_acts = vec![RequestedAct::EnrollDevice];
        assert_eq!(
            issue(&admitted(), tampered, CODE, NOW, TTL),
            Err(R::AddressBroken)
        );

        // A record agreed for another terminal's request.
        let mut other = peer_request();
        other.code_challenge = crate::pkce::challenge(&"v".repeat(43));
        let other = admit_request(&other, &policy()).unwrap();
        assert_eq!(
            issue(&other, signed(), CODE, NOW, TTL),
            Err(R::RequestMismatch)
        );

        assert_eq!(
            issue(&admitted(), signed(), "short", NOW, TTL),
            Err(R::Delivery(DeliveryRefusal::CodeWeak))
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
