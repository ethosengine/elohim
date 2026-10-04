//! Getting an agreed consent back to the terminal that asked, once.
//!
//! This is delivery state, kept apart from the consent record. It lives in the
//! portal for a few minutes and is gone once redeemed or expired. Nothing here
//! is addressed, signed or shown to anyone.
//!
//! The portal never stores the code, only its digest, so a leaked store yields
//! nothing redeemable. A delivery is redeemed once. The caller's store must
//! make that so: mark it redeemed with a compare-and-set in the same step that
//! reads it, and treat a lost race as [`RedemptionRefusal::AlreadyRedeemed`].

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pkce;
use crate::request::AdmittedRequest;
use crate::return_path::{is_token, ReturnPath};

/// Shortest code a portal may issue: 32 URL-safe characters is at least 190
/// bits when drawn at random.
pub const MIN_CODE_LEN: usize = 32;
pub const MAX_CODE_LEN: usize = 128;

/// How the portal keys a delivery without keeping the code.
pub fn code_digest(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryRefusal {
    CodeWeak,
    WindowInvalid,
}

impl DeliveryRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::CodeWeak => "delivery_code_weak",
            Self::WindowInvalid => "delivery_window_invalid",
        }
    }
}

/// An agreed consent waiting to be collected. Ephemeral: rebuilt by asking again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingDelivery {
    pub code_digest: String,
    /// The address of the consent record this delivers.
    pub consent_cid: String,
    pub client_id: String,
    pub device_key: String,
    pub code_challenge: String,
    pub state: String,
    pub return_path: ReturnPath,
    pub expires_at_micros: i64,
    pub redeemed: bool,
}

impl PendingDelivery {
    /// Hold `consent_cid` for the terminal that made `admitted`.
    ///
    /// `code` is the portal's freshly drawn random token and `ttl_micros` how
    /// long it may wait to be redeemed.
    pub fn issue(
        admitted: &AdmittedRequest,
        consent_cid: &str,
        code: &str,
        now_micros: i64,
        ttl_micros: i64,
    ) -> Result<Self, DeliveryRefusal> {
        if !is_token(code, MIN_CODE_LEN, MAX_CODE_LEN) {
            return Err(DeliveryRefusal::CodeWeak);
        }
        if ttl_micros <= 0 {
            return Err(DeliveryRefusal::WindowInvalid);
        }
        let r = admitted.request();
        Ok(Self {
            code_digest: code_digest(code),
            consent_cid: consent_cid.to_string(),
            client_id: r.client_id.clone(),
            device_key: r.device_key.clone(),
            code_challenge: r.code_challenge.clone(),
            state: r.state.clone(),
            return_path: r.return_path,
            expires_at_micros: now_micros
                .checked_add(ttl_micros)
                .ok_or(DeliveryRefusal::WindowInvalid)?,
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

    /// Whether the portal should discard the delivery on this refusal. A second
    /// presentation of a redeemed code, or a wrong verifier for a real code,
    /// means the code has been seen by someone other than the asker.
    pub fn burns_delivery(self) -> bool {
        matches!(
            self,
            Self::AlreadyRedeemed | Self::VerifierMismatch | Self::DeviceMismatch
        )
    }
}

/// Decide whether `redemption` may collect what `delivery` holds.
pub fn admit_redemption(
    delivery: &PendingDelivery,
    redemption: &Redemption,
    now_micros: i64,
) -> Result<(), RedemptionRefusal> {
    use RedemptionRefusal as R;

    if !pkce::constant_time_eq(
        code_digest(&redemption.code).as_bytes(),
        delivery.code_digest.as_bytes(),
    ) {
        return Err(R::CodeUnknown);
    }
    if delivery.redeemed {
        return Err(R::AlreadyRedeemed);
    }
    if now_micros >= delivery.expires_at_micros {
        return Err(R::Expired);
    }
    if redemption.client_id != delivery.client_id {
        return Err(R::ClientMismatch);
    }
    if redemption.device_key != delivery.device_key {
        return Err(R::DeviceMismatch);
    }
    if !pkce::verifier_matches(&redemption.code_verifier, &delivery.code_challenge) {
        return Err(R::VerifierMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::peer_record;
    use crate::hash_shape::sample_key;
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy, VERIFIER};

    const CODE: &str = "c0dec0dec0dec0dec0dec0dec0dec0de";
    const NOW: i64 = 1_000_000;
    const TTL: i64 = 300_000_000;

    fn admitted() -> AdmittedRequest {
        admit_request(&peer_request(), &policy()).unwrap()
    }

    fn delivery() -> PendingDelivery {
        let cid = peer_record().cid().unwrap();
        PendingDelivery::issue(&admitted(), &cid, CODE, NOW, TTL).unwrap()
    }

    fn redemption() -> Redemption {
        Redemption {
            code: CODE.into(),
            code_verifier: VERIFIER.into(),
            client_id: "epr-cli".into(),
            device_key: sample_key(7),
        }
    }

    fn refused(change: impl FnOnce(&mut Redemption)) -> RedemptionRefusal {
        let mut r = redemption();
        change(&mut r);
        admit_redemption(&delivery(), &r, NOW + 1).unwrap_err()
    }

    #[test]
    fn the_asking_terminal_redeems() {
        assert_eq!(
            admit_redemption(&delivery(), &redemption(), NOW + 1),
            Ok(())
        );
        assert_eq!(delivery().consent_cid, peer_record().cid().unwrap());
    }

    #[test]
    fn the_store_never_holds_the_code() {
        let json = serde_json::to_string(&delivery()).unwrap();
        assert!(!json.contains(CODE));
        assert_eq!(delivery().code_digest, code_digest(CODE));
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
    fn a_delivery_redeems_once_and_within_its_window() {
        use RedemptionRefusal as R;
        let mut used = delivery();
        used.redeemed = true;
        assert_eq!(
            admit_redemption(&used, &redemption(), NOW + 1),
            Err(R::AlreadyRedeemed)
        );
        assert_eq!(
            admit_redemption(&delivery(), &redemption(), NOW + TTL),
            Err(R::Expired)
        );
        assert_eq!(
            admit_redemption(&delivery(), &redemption(), NOW + TTL - 1),
            Ok(())
        );
    }

    #[test]
    fn signs_of_a_seen_code_burn_the_delivery() {
        use RedemptionRefusal as R;
        for r in [R::AlreadyRedeemed, R::VerifierMismatch, R::DeviceMismatch] {
            assert!(r.burns_delivery(), "{r:?}");
        }
        for r in [R::CodeUnknown, R::Expired, R::ClientMismatch] {
            assert!(!r.burns_delivery(), "{r:?}");
        }
    }

    #[test]
    fn a_weak_code_or_an_empty_window_cannot_be_issued() {
        use DeliveryRefusal as R;
        let issue =
            |code: &str, now, ttl| PendingDelivery::issue(&admitted(), "cid", code, now, ttl);
        assert_eq!(issue("short", NOW, TTL), Err(R::CodeWeak));
        assert_eq!(issue(&"a#".repeat(16), NOW, TTL), Err(R::CodeWeak));
        assert_eq!(issue(CODE, NOW, 0), Err(R::WindowInvalid));
        assert_eq!(issue(CODE, i64::MAX, TTL), Err(R::WindowInvalid));
    }
}
