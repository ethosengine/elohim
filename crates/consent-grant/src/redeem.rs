//! The approved grant a portal holds, and whether a terminal may redeem it.
//!
//! The portal never stores the code, only its digest, so a leaked store yields
//! nothing redeemable. A grant is redeemed once. The caller's store must make
//! that so: mark the grant redeemed with a compare-and-set in the same step
//! that reads it, and treat a lost race as [`RedemptionRefusal::AlreadyRedeemed`].

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::act::RequestedAct;
use crate::pkce;
use crate::request::{AdmittedRequest, Standing};
use crate::return_path::{is_token, ReturnPath};

/// Shortest code a portal may issue: 32 URL-safe characters is at least 190
/// bits when drawn at random.
pub const MIN_CODE_LEN: usize = 32;
pub const MAX_CODE_LEN: usize = 128;

/// How the portal keys a grant without keeping the code.
pub fn code_digest(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

/// What the controller granted. Every approval produces a grant, and what the
/// device may do afterwards is read from these claims and nowhere else: a
/// person may grant less than was asked, never more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantedClaims {
    /// The acts the controller agreed to sign, a subset of those asked for.
    pub acts: Vec<RequestedAct>,
    /// Present exactly when binding the participant key was granted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant_key: Option<String>,
    /// When the time-bound acts lapse. `None` when none was granted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_micros: Option<i64>,
}

impl GrantedClaims {
    pub fn standing(&self) -> Standing {
        Standing::of(&self.acts)
    }
}

/// A controller's decision on an admitted request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    /// Agent keys of the controllers who approved.
    pub approved_by: Vec<String>,
    /// The acts they agreed to, chosen from those asked for.
    pub granted_acts: Vec<RequestedAct>,
    /// The lifetime they agreed to for time-bound acts.
    pub valid_for_secs: Option<u64>,
}

impl Approval {
    /// Approve the request exactly as asked.
    pub fn in_full(admitted: &AdmittedRequest, approved_by: Vec<String>) -> Self {
        let r = admitted.request();
        Self {
            approved_by,
            granted_acts: r.acts.clone(),
            valid_for_secs: r.valid_for_secs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueRefusal {
    CodeWeak,
    Unapproved,
    GrantedNothing,
    GrantedBeyondRequest,
    ParticipantBindNeedsEnrollment,
    ValidityMissing,
    ValidityUngranted,
    ValidityBeyondRequest,
    WindowInvalid,
}

impl IssueRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::CodeWeak => "issue_code_weak",
            Self::Unapproved => "issue_unapproved",
            Self::GrantedNothing => "issue_granted_nothing",
            Self::GrantedBeyondRequest => "issue_granted_beyond_request",
            Self::ParticipantBindNeedsEnrollment => "issue_participant_bind_needs_enrollment",
            Self::ValidityMissing => "issue_validity_missing",
            Self::ValidityUngranted => "issue_validity_ungranted",
            Self::ValidityBeyondRequest => "issue_validity_beyond_request",
            Self::WindowInvalid => "issue_window_invalid",
        }
    }
}

/// An approved request awaiting redemption. Ephemeral: rebuilt by asking again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredGrant {
    pub code_digest: String,
    pub client_id: String,
    pub device_key: String,
    pub network_dna: String,
    pub claims: GrantedClaims,
    pub code_challenge: String,
    pub state: String,
    pub return_path: ReturnPath,
    /// Agent keys of the controllers who approved.
    pub approved_by: Vec<String>,
    pub expires_at_micros: i64,
    pub redeemed: bool,
}

impl StoredGrant {
    /// Record a controller's approval of an admitted request.
    ///
    /// `code` is the portal's freshly drawn random token and `code_ttl_micros`
    /// how long it may wait to be redeemed. The claims are checked against the
    /// request here, so a portal cannot issue a grant wider than what the
    /// person was shown.
    pub fn issue(
        admitted: &AdmittedRequest,
        code: &str,
        approval: Approval,
        now_micros: i64,
        code_ttl_micros: i64,
    ) -> Result<Self, IssueRefusal> {
        use IssueRefusal as R;

        if !is_token(code, MIN_CODE_LEN, MAX_CODE_LEN) {
            return Err(R::CodeWeak);
        }
        if approval.approved_by.is_empty() {
            return Err(R::Unapproved);
        }
        let r = admitted.request();
        let granted = &approval.granted_acts;
        if granted.is_empty() {
            return Err(R::GrantedNothing);
        }
        let distinct = granted
            .iter()
            .enumerate()
            .all(|(i, act)| !granted[..i].contains(act));
        if !distinct || !granted.iter().all(|act| r.acts.contains(act)) {
            return Err(R::GrantedBeyondRequest);
        }
        let binds_participant = granted.contains(&RequestedAct::BindParticipantKey);
        if binds_participant && Standing::of(granted) != Standing::StewardedPeer {
            return Err(R::ParticipantBindNeedsEnrollment);
        }
        let time_bound = granted.iter().any(RequestedAct::is_time_bound);
        let valid_until_micros = match (approval.valid_for_secs, time_bound) {
            (None, false) => None,
            (Some(_), false) => return Err(R::ValidityUngranted),
            (None, true) => return Err(R::ValidityMissing),
            (Some(secs), true) => {
                if secs == 0 || r.valid_for_secs.is_none_or(|asked| secs > asked) {
                    return Err(R::ValidityBeyondRequest);
                }
                let micros = i64::try_from(secs)
                    .ok()
                    .and_then(|s| s.checked_mul(1_000_000))
                    .and_then(|m| now_micros.checked_add(m))
                    .ok_or(R::WindowInvalid)?;
                Some(micros)
            }
        };
        if code_ttl_micros <= 0 {
            return Err(R::WindowInvalid);
        }
        Ok(Self {
            code_digest: code_digest(code),
            client_id: r.client_id.clone(),
            device_key: r.device_key.clone(),
            network_dna: r.network_dna.clone(),
            claims: GrantedClaims {
                acts: approval.granted_acts,
                participant_key: binds_participant
                    .then(|| r.participant_key.clone())
                    .flatten(),
                valid_until_micros,
            },
            code_challenge: r.code_challenge.clone(),
            state: r.state.clone(),
            return_path: r.return_path,
            approved_by: approval.approved_by,
            expires_at_micros: now_micros
                .checked_add(code_ttl_micros)
                .ok_or(R::WindowInvalid)?,
            redeemed: false,
        })
    }
}

/// What a terminal presents to redeem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redemption {
    pub code: String,
    pub code_verifier: String,
    pub client_id: String,
    pub device_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedemptionRefusal {
    CodeUnknown,
    AlreadyRedeemed,
    Expired,
    ClientMismatch,
    DeviceMismatch,
    VerifierMismatch,
}

impl RedemptionRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::CodeUnknown => "redemption_code_unknown",
            Self::AlreadyRedeemed => "redemption_already_redeemed",
            Self::Expired => "redemption_expired",
            Self::ClientMismatch => "redemption_client_mismatch",
            Self::DeviceMismatch => "redemption_device_mismatch",
            Self::VerifierMismatch => "redemption_verifier_mismatch",
        }
    }

    /// Whether the portal should discard the grant on this refusal. A second
    /// presentation of a redeemed code, or a wrong verifier for a real code,
    /// means the code has been seen by someone other than the asker.
    pub fn burns_grant(self) -> bool {
        matches!(
            self,
            Self::AlreadyRedeemed | Self::VerifierMismatch | Self::DeviceMismatch
        )
    }
}

/// Decide whether `redemption` may collect what `grant` holds.
pub fn admit_redemption(
    grant: &StoredGrant,
    redemption: &Redemption,
    now_micros: i64,
) -> Result<(), RedemptionRefusal> {
    use RedemptionRefusal as R;

    if !pkce::constant_time_eq(
        code_digest(&redemption.code).as_bytes(),
        grant.code_digest.as_bytes(),
    ) {
        return Err(R::CodeUnknown);
    }
    if grant.redeemed {
        return Err(R::AlreadyRedeemed);
    }
    if now_micros >= grant.expires_at_micros {
        return Err(R::Expired);
    }
    if redemption.client_id != grant.client_id {
        return Err(R::ClientMismatch);
    }
    if redemption.device_key != grant.device_key {
        return Err(R::DeviceMismatch);
    }
    if !pkce::verifier_matches(&redemption.code_verifier, &grant.code_challenge) {
        return Err(R::VerifierMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_key::sample_key;
    use crate::request::admit_request;
    use crate::request::tests::{policy, request, VERIFIER};
    use crate::request::AdmittedRequest;

    const CODE: &str = "c0dec0dec0dec0dec0dec0dec0dec0de";
    const NOW: i64 = 1_000_000;
    const TTL: i64 = 300_000_000;

    fn approver() -> Vec<String> {
        vec![sample_key(9)]
    }

    fn grant() -> StoredGrant {
        let admitted = admit_request(&request(), &policy()).unwrap();
        let approval = Approval::in_full(&admitted, approver());
        StoredGrant::issue(&admitted, CODE, approval, NOW, TTL).unwrap()
    }

    /// A request for everything: enroll, bind the participant key, and one
    /// time-bound delegation for an hour.
    fn wide() -> AdmittedRequest {
        let mut r = request();
        r.acts = vec![
            RequestedAct::EnrollDevice,
            RequestedAct::BindParticipantKey,
            head(),
        ];
        r.participant_key = Some(format!("did:key:z6Mk{}", "h".repeat(44)));
        r.valid_for_secs = Some(3600);
        admit_request(&r, &policy()).unwrap()
    }

    fn head() -> RequestedAct {
        RequestedAct::DelegateHead {
            item_id: "item".into(),
        }
    }

    fn issue(
        acts: Vec<RequestedAct>,
        valid_for_secs: Option<u64>,
    ) -> Result<StoredGrant, IssueRefusal> {
        let approval = Approval {
            approved_by: approver(),
            granted_acts: acts,
            valid_for_secs,
        };
        StoredGrant::issue(&wide(), CODE, approval, NOW, TTL)
    }

    fn redemption() -> Redemption {
        Redemption {
            code: CODE.into(),
            code_verifier: VERIFIER.into(),
            client_id: "elohim-cli".into(),
            device_key: sample_key(7),
        }
    }

    fn refused(change: impl FnOnce(&mut Redemption)) -> RedemptionRefusal {
        let mut r = redemption();
        change(&mut r);
        admit_redemption(&grant(), &r, NOW + 1).unwrap_err()
    }

    #[test]
    fn the_asking_terminal_redeems() {
        assert_eq!(admit_redemption(&grant(), &redemption(), NOW + 1), Ok(()));
    }

    #[test]
    fn the_store_never_holds_the_code() {
        let json = serde_json::to_string(&grant()).unwrap();
        assert!(!json.contains(CODE));
        assert_eq!(grant().code_digest, code_digest(CODE));
    }

    #[test]
    fn a_code_alone_redeems_nothing() {
        use RedemptionRefusal as R;
        // Someone who saw the code but holds neither the verifier nor the device.
        assert_eq!(
            refused(|r| r.code_verifier = "x".repeat(43)),
            R::VerifierMismatch
        );
        assert_eq!(refused(|r| r.device_key = sample_key(8)), R::DeviceMismatch);
        assert_eq!(refused(|r| r.client_id = "other".into()), R::ClientMismatch);
        assert_eq!(refused(|r| r.code = "d".repeat(32)), R::CodeUnknown);
    }

    #[test]
    fn a_grant_redeems_once_and_within_its_window() {
        use RedemptionRefusal as R;
        let mut used = grant();
        used.redeemed = true;
        assert_eq!(
            admit_redemption(&used, &redemption(), NOW + 1),
            Err(R::AlreadyRedeemed)
        );
        assert_eq!(
            admit_redemption(&grant(), &redemption(), NOW + TTL),
            Err(R::Expired)
        );
        assert_eq!(
            admit_redemption(&grant(), &redemption(), NOW + TTL - 1),
            Ok(())
        );
    }

    #[test]
    fn signs_of_a_seen_code_burn_the_grant() {
        use RedemptionRefusal as R;
        for r in [R::AlreadyRedeemed, R::VerifierMismatch, R::DeviceMismatch] {
            assert!(r.burns_grant(), "{r:?}");
        }
        for r in [R::CodeUnknown, R::Expired, R::ClientMismatch] {
            assert!(!r.burns_grant(), "{r:?}");
        }
    }

    #[test]
    fn every_approval_yields_a_grant_whose_claims_say_what_was_granted() {
        let full = StoredGrant::issue(
            &wide(),
            CODE,
            Approval::in_full(&wide(), approver()),
            NOW,
            TTL,
        )
        .unwrap();
        assert_eq!(full.claims.standing(), Standing::StewardedPeer);
        assert!(full.claims.participant_key.is_some());
        assert_eq!(full.claims.valid_until_micros, Some(NOW + 3600 * 1_000_000));

        // The person keeps only the delegation: still a grant, now ephemeral.
        let narrowed = issue(vec![head()], Some(600)).unwrap();
        assert_eq!(narrowed.claims.standing(), Standing::Ephemeral);
        assert_eq!(narrowed.claims.acts, vec![head()]);
        assert_eq!(narrowed.claims.participant_key, None);
        assert_eq!(
            narrowed.claims.valid_until_micros,
            Some(NOW + 600 * 1_000_000)
        );

        // Enrollment alone carries no lifetime.
        let enrolled = issue(vec![RequestedAct::EnrollDevice], None).unwrap();
        assert_eq!(enrolled.claims.valid_until_micros, None);
        assert_eq!(enrolled.claims.participant_key, None);
    }

    #[test]
    fn a_grant_is_never_wider_than_the_request() {
        use IssueRefusal as R;
        let other = RequestedAct::DelegateHead {
            item_id: "other".into(),
        };
        assert_eq!(issue(vec![other], Some(600)), Err(R::GrantedBeyondRequest));
        assert_eq!(
            issue(vec![head(), head()], Some(600)),
            Err(R::GrantedBeyondRequest)
        );
        assert_eq!(
            issue(vec![head()], Some(3601)),
            Err(R::ValidityBeyondRequest)
        );
        assert_eq!(issue(vec![head()], Some(0)), Err(R::ValidityBeyondRequest));
        assert_eq!(issue(vec![], None), Err(R::GrantedNothing));
    }

    #[test]
    fn granted_claims_stay_coherent() {
        use IssueRefusal as R;
        assert_eq!(
            issue(vec![RequestedAct::BindParticipantKey], None),
            Err(R::ParticipantBindNeedsEnrollment)
        );
        assert_eq!(issue(vec![head()], None), Err(R::ValidityMissing));
        assert_eq!(
            issue(vec![RequestedAct::EnrollDevice], Some(600)),
            Err(R::ValidityUngranted)
        );
    }

    #[test]
    fn a_weak_code_or_an_unapproved_grant_cannot_be_issued() {
        use IssueRefusal as R;
        let admitted = admit_request(&request(), &policy()).unwrap();
        let ok = || Approval::in_full(&admitted, approver());
        let issue = |code: &str, approval, now, ttl| {
            StoredGrant::issue(&admitted, code, approval, now, ttl)
        };
        assert_eq!(issue("short", ok(), NOW, TTL), Err(R::CodeWeak));
        assert_eq!(issue(&"a#".repeat(16), ok(), NOW, TTL), Err(R::CodeWeak));
        assert_eq!(
            issue(CODE, Approval::in_full(&admitted, vec![]), NOW, TTL),
            Err(R::Unapproved)
        );
        assert_eq!(issue(CODE, ok(), NOW, 0), Err(R::WindowInvalid));
        assert_eq!(issue(CODE, ok(), i64::MAX, TTL), Err(R::WindowInvalid));
    }
}
