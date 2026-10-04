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
use crate::request::AdmittedRequest;
use crate::return_path::{is_token, ReturnPath};

/// Shortest code a portal may issue: 32 URL-safe characters is at least 190
/// bits when drawn at random.
pub const MIN_CODE_LEN: usize = 32;
pub const MAX_CODE_LEN: usize = 128;

/// How the portal keys a grant without keeping the code.
pub fn code_digest(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

/// An approved request awaiting redemption. Ephemeral: rebuilt by asking again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredGrant {
    pub code_digest: String,
    pub client_id: String,
    pub device_key: String,
    pub network_dna: String,
    pub acts: Vec<RequestedAct>,
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
    /// `code` is the portal's freshly drawn random token; `None` is returned
    /// if it is too short or outside the URL-safe alphabet, so a weak code
    /// cannot be issued by mistake.
    pub fn issue(
        admitted: &AdmittedRequest,
        code: &str,
        approved_by: Vec<String>,
        now_micros: i64,
        ttl_micros: i64,
    ) -> Option<Self> {
        if !is_token(code, MIN_CODE_LEN, MAX_CODE_LEN) || approved_by.is_empty() || ttl_micros <= 0
        {
            return None;
        }
        let r = admitted.request();
        Some(Self {
            code_digest: code_digest(code),
            client_id: r.client_id.clone(),
            device_key: r.device_key.clone(),
            network_dna: r.network_dna.clone(),
            acts: r.acts.clone(),
            code_challenge: r.code_challenge.clone(),
            state: r.state.clone(),
            return_path: r.return_path,
            approved_by,
            expires_at_micros: now_micros.checked_add(ttl_micros)?,
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

    const CODE: &str = "c0dec0dec0dec0dec0dec0dec0dec0de";
    const NOW: i64 = 1_000_000;
    const TTL: i64 = 300_000_000;

    fn grant() -> StoredGrant {
        let admitted = admit_request(&request(), &policy()).unwrap();
        StoredGrant::issue(&admitted, CODE, vec![sample_key(9)], NOW, TTL).unwrap()
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
    fn a_weak_code_or_an_unapproved_grant_cannot_be_issued() {
        let admitted = admit_request(&request(), &policy()).unwrap();
        let approver = vec![sample_key(9)];
        assert!(StoredGrant::issue(&admitted, "short", approver.clone(), NOW, TTL).is_none());
        assert!(
            StoredGrant::issue(&admitted, &"a#".repeat(16), approver.clone(), NOW, TTL).is_none()
        );
        assert!(StoredGrant::issue(&admitted, CODE, vec![], NOW, TTL).is_none());
        assert!(StoredGrant::issue(&admitted, CODE, approver.clone(), NOW, 0).is_none());
        assert!(StoredGrant::issue(&admitted, CODE, approver, i64::MAX, TTL).is_none());
    }
}
