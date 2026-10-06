//! Witnessed moments, and the one this crate holds: authorizing a device.
//!
//! A witnessed moment is a point in a protocol flow where someone other than
//! the parties may attend: an elohim attending the person, say. Such moments
//! exist all over the protocol (a reach gate is another); this crate sets the
//! default one at the sign-in and authorization floor. Every moment has the
//! same shape:
//!
//! - it says what kind of moment it is ([`MomentKind`]);
//! - it hands the attending witness the claims in front of it;
//! - it takes back one outcome ([`Witnessing`]): proceed, proceed with the
//!   witness's signatures, or pause for re-authentication.
//!
//! [`WitnessedMoment`] and [`Witnessing`] carry no consent types, so another
//! moment can implement the same shape with its own claims and signature
//! type. A shared home for witnessed moments is not designed; until it is,
//! the shape lives here.
//!
//! The device-authorization moment falls after the person has agreed and
//! before the controller signs, so it costs nothing when it stops the
//! ceremony: no signature is made, no mandate grant is written and no code is
//! issued. Its claims are the admitted request, the agreed consent, and what
//! is known about who is approving ([`AuthorizationClaims`],
//! [`AuthorizationContext`]): data for a witness to weigh, never a threshold
//! this crate applies.
//!
//! A witness never has to be present and adds no step for the person. A
//! signed-in device acts with the person's authority, as in OAuth. What an
//! attending witness may do is notice that something does not feel right and
//! pause until the person signs in again; the host then refuses with a plain
//! reason, and the person signs in and approves again. Otherwise a witness
//! only strengthens a consent: it cannot change what was agreed and cannot
//! remove or replace anyone else's signature, because [`attend`] keeps only
//! the witness signatures it adds. Today nobody attends ([`Unattended`]), and
//! the ceremony passes straight through.

use crate::consent::{ConsentSignature, SignedConsent, SignerRole};
use crate::request::AdmittedRequest;

/// The longest pause reason kept; a longer one is cut.
pub const MAX_PAUSE_REASON: usize = 280;

/// What kind of witnessed moment is attended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MomentKind {
    /// A person's node authorizing a device to act for them.
    DeviceAuthorization,
    /// A person signing in to their own node ([`crate::signin`]).
    SignIn,
}

/// What an attending witness returns from a moment. `S` is the moment's
/// signature type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Witnessing<S> {
    /// Go on, adding nothing.
    Proceed,
    /// Go on, with these signatures added as the witness's.
    ProceedWithSignatures(Vec<S>),
    /// Something does not feel right: pause until the person signs in again.
    /// The reason is plain words for the person and the log.
    PauseForReauthentication { reason: String },
}

/// Someone who may attend a witnessed moment whose claims are `C` and whose
/// signatures are `S`. An implementation must not block the person: a
/// witness that cannot attend returns [`Witnessing::Proceed`].
pub trait WitnessedMoment<C: ?Sized, S>: Send + Sync {
    fn attend(&self, kind: MomentKind, claims: &C) -> Witnessing<S>;
}

/// Nobody attends, at any moment: every moment proceeds at once and is never
/// paused.
#[derive(Debug, Default, Clone, Copy)]
pub struct Unattended;

impl<C: ?Sized, S> WitnessedMoment<C, S> for Unattended {
    fn attend(&self, _kind: MomentKind, _claims: &C) -> Witnessing<S> {
        Witnessing::Proceed
    }
}

/// The claims in front of the witness when a device is authorized: what the
/// device asked, what the person agreed, and what is known about who is
/// approving, before the controller signs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationClaims {
    pub admitted: AdmittedRequest,
    pub consent: SignedConsent,
    pub context: AuthorizationContext,
}

/// What is known, at a witnessed moment, about the device that speaks for the
/// person there. Data for a witness to weigh; nothing here is compared with a
/// threshold. `None` is "not known" (the network was not read in time, or the
/// fact has no source yet), never zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpeakerContext {
    /// Whether it speaks as one of the authority's root controllers rather
    /// than as a joined device.
    pub root: bool,
    /// How long it has spoken for the person, in milliseconds: since its
    /// joining record. Not known for a root controller.
    pub speaking_for_ms: Option<i64>,
    /// How many of the person's other devices have affirmed its own joining.
    pub affirmed_by: Option<usize>,
    /// How many devices speak for the person.
    pub devices_speaking: Option<usize>,
}

impl SpeakerContext {
    /// From this node's standing and, when it was read, the network's list.
    pub fn of(
        standing: &crate::ControllerStanding,
        read: Option<&crate::controller::DevicesRead>,
        me: &str,
        now_ms: i64,
    ) -> Self {
        let mine = read.and_then(|r| r.devices.iter().find(|d| d.device_key == me));
        Self {
            root: standing.speaks_via.is_none(),
            speaking_for_ms: mine.map(|d| now_ms.saturating_sub(d.joined_at).max(0)),
            affirmed_by: mine.map(|d| d.affirmed_count),
            devices_speaking: read.map(|r| {
                let mut all = r.speakers();
                for key in &standing.controllers {
                    if !all.contains(key) {
                        all.push(key.clone());
                    }
                }
                all.len()
            }),
        }
    }
}

/// What is known at the device-authorization moment beyond the request and
/// the consent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorizationContext {
    /// The device approving.
    pub approver: SpeakerContext,
    /// Whether the asking device began an identity of its own, as it said
    /// over the carrier. Not known when the request arrived another way.
    pub asking_device_has_own_identity: Option<bool>,
}

/// A witness of the device-authorization moment. Anything that implements
/// [`WitnessedMoment`] for [`AuthorizationClaims`] and [`ConsentSignature`] is
/// one.
pub trait WitnessBeat: WitnessedMoment<AuthorizationClaims, ConsentSignature> {}

impl<T: WitnessedMoment<AuthorizationClaims, ConsentSignature>> WitnessBeat for T {}

/// The moment paused: the person is asked to sign in again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paused {
    /// Plain words: control characters dropped, at most [`MAX_PAUSE_REASON`]
    /// characters, never empty.
    pub reason: String,
}

/// A pause reason made plain: control characters dropped, at most
/// [`MAX_PAUSE_REASON`] characters, never empty.
pub(crate) fn plain_reason(reason: &str) -> String {
    let plain: String = reason
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_PAUSE_REASON)
        .collect();
    let plain = plain.trim();
    if plain.is_empty() {
        "a witness asked for the person to sign in again".to_string()
    } else {
        plain.to_string()
    }
}

/// Run the device-authorization moment. Keep only the witness signatures the
/// beat adds, on the same record; a signature in any other role is discarded,
/// so a beat that misbehaves costs the consent nothing. A pause comes back as
/// [`Paused`], before anything is signed or issued.
pub fn attend(
    beat: &dyn WitnessBeat,
    admitted: &AdmittedRequest,
    consent: SignedConsent,
    context: AuthorizationContext,
) -> Result<SignedConsent, Paused> {
    let claims = AuthorizationClaims {
        admitted: admitted.clone(),
        consent,
        context,
    };
    let added = match beat.attend(MomentKind::DeviceAuthorization, &claims) {
        Witnessing::Proceed => Vec::new(),
        Witnessing::ProceedWithSignatures(signatures) => signatures,
        Witnessing::PauseForReauthentication { reason } => {
            return Err(Paused {
                reason: plain_reason(&reason),
            });
        }
    };
    let consent = claims.consent;
    let added: Vec<_> = added
        .into_iter()
        .filter(|s| s.role == SignerRole::Witness && !consent.signatures.contains(s))
        .collect();
    Ok(added
        .into_iter()
        .fold(consent, SignedConsent::with_signature))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::peer_record;
    use crate::hash_shape::sample_key;
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy};

    fn admitted() -> AdmittedRequest {
        admit_request(&peer_request(), &policy()).unwrap()
    }

    fn signature(role: SignerRole, who: u8) -> ConsentSignature {
        ConsentSignature {
            role,
            signer: sample_key(who),
            signature: format!("{role:?}-{who}"),
        }
    }

    /// The agreed consent, as the beat sees it: before the controller signs.
    fn agreed() -> SignedConsent {
        SignedConsent::new(peer_record()).unwrap()
    }

    struct Answers(Witnessing<ConsentSignature>);
    impl WitnessedMoment<AuthorizationClaims, ConsentSignature> for Answers {
        fn attend(
            &self,
            kind: MomentKind,
            claims: &AuthorizationClaims,
        ) -> Witnessing<ConsentSignature> {
            assert_eq!(kind, MomentKind::DeviceAuthorization);
            assert_eq!(claims.consent.cid, agreed().cid);
            self.0.clone()
        }
    }

    fn pauses(reason: &str) -> Answers {
        Answers(Witnessing::PauseForReauthentication {
            reason: reason.to_string(),
        })
    }

    #[test]
    fn by_default_nobody_attends_nothing_is_added_and_nothing_pauses() {
        let out = attend(
            &Unattended,
            &admitted(),
            agreed(),
            AuthorizationContext::default(),
        )
        .unwrap();
        assert_eq!(out, agreed());
        assert_eq!(out.signed_in(SignerRole::Witness), 0);
        // The default attends any moment, with any claims, the same way.
        let other: Witnessing<u8> = WitnessedMoment::<str, u8>::attend(
            &Unattended,
            MomentKind::DeviceAuthorization,
            "any claims",
        );
        assert_eq!(other, Witnessing::Proceed);
    }

    /// A witness that remembers what context it was shown.
    struct Sees(std::sync::Mutex<Option<AuthorizationContext>>);

    impl WitnessedMoment<AuthorizationClaims, ConsentSignature> for Sees {
        fn attend(
            &self,
            _: MomentKind,
            claims: &AuthorizationClaims,
        ) -> Witnessing<ConsentSignature> {
            *self.0.lock().unwrap() = Some(claims.context.clone());
            Witnessing::Proceed
        }
    }

    #[test]
    fn the_witness_is_shown_who_is_approving_as_data() {
        use crate::controller::{DevicesRead, StandingDevice};
        let me = sample_key(12);
        let standing = crate::ControllerStanding {
            identity_root: "r".into(),
            authority: "a".into(),
            network_dna: "n".into(),
            controllers: vec![sample_key(9), me.clone()],
            required: 1,
            speaks_via: Some("via".into()),
            also_speaks_for: vec![],
        };
        let read = DevicesRead {
            roots: vec![sample_key(9)],
            devices: vec![StandingDevice {
                device_key: me.clone(),
                device_fingerprint: "f".into(),
                binding: "b".into(),
                content_dna: "c".into(),
                joined_at: 1_000,
                approved_by: vec![sample_key(9)],
                affirmed_by: vec![sample_key(11)],
                affirmed_count: 1,
                this_device: true,
            }],
            not_standing: 0,
            truncated: false,
        };
        let context = AuthorizationContext {
            approver: SpeakerContext::of(&standing, Some(&read), &me, 61_000),
            asking_device_has_own_identity: Some(true),
        };
        assert_eq!(
            context.approver,
            SpeakerContext {
                root: false,
                speaking_for_ms: Some(60_000),
                affirmed_by: Some(1),
                devices_speaking: Some(2),
            }
        );
        // Unread is unknown, never zero.
        let unread = SpeakerContext::of(&standing, None, &me, 61_000);
        assert_eq!(
            (unread.speaking_for_ms, unread.devices_speaking),
            (None, None)
        );
        let beat = Sees(std::sync::Mutex::new(None));
        attend(&beat, &admitted(), agreed(), context.clone()).unwrap();
        assert_eq!(beat.0.lock().unwrap().clone(), Some(context));
    }

    #[test]
    fn a_witness_adds_only_its_own_signatures() {
        let beat = Answers(Witnessing::ProceedWithSignatures(vec![
            signature(SignerRole::Witness, 5),
            signature(SignerRole::Controller, 6),
            signature(SignerRole::Device, 7),
        ]));
        let out = attend(
            &beat,
            &admitted(),
            agreed(),
            AuthorizationContext::default(),
        )
        .unwrap();
        assert_eq!(out.signatures, vec![signature(SignerRole::Witness, 5)]);
        assert_eq!(out.record, agreed().record);
        assert!(out.address_holds());
    }

    #[test]
    fn an_attending_witness_may_pause_with_a_plain_reason() {
        assert_eq!(
            attend(
                &pauses("this is not how they usually sign in"),
                &admitted(),
                agreed(),
                AuthorizationContext::default()
            ),
            Err(Paused {
                reason: "this is not how they usually sign in".into()
            })
        );
        assert_eq!(
            attend(
                &pauses("\u{7}  "),
                &admitted(),
                agreed(),
                AuthorizationContext::default()
            )
            .unwrap_err()
            .reason,
            "a witness asked for the person to sign in again"
        );
        let long = "x".repeat(MAX_PAUSE_REASON + 50);
        assert_eq!(
            attend(
                &pauses(&long),
                &admitted(),
                agreed(),
                AuthorizationContext::default()
            )
            .unwrap_err()
            .reason
            .chars()
            .count(),
            MAX_PAUSE_REASON
        );
    }
}
