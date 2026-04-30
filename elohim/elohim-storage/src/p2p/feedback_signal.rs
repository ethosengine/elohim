//! FeedbackSignal wire type — Phase 3.5 Trust-Compute Gradient substrate.
//!
//! Wire contract for the `FeedbackSignal` EPR kind. Travels via the existing
//! `/elohim/epr-atom/1.0.0` libp2p protocol as a signed EPR atom payload.
//!
//! ## Category classification
//!
//! Category **B2** — agent-scoped with attestation.
//!
//! A FeedbackSignal is authored by a specific agent (the signer) about a
//! specific piece of content (targetCid). The signed-by + signature fields
//! provide an attestation proof. The signal is NOT a DHT source-of-truth at
//! this layer (that is T4's job); this module is the libp2p data-ops wire
//! shape for carrying signals between peers.
//!
//! ## Wire format
//!
//! MessagePack-encoded via `rmp_serde`. `camelCase` field names via
//! `#[serde(rename_all = "camelCase")]`. Enum variants use `kebab-case`
//! on the wire so `"debit-soft"` round-trips correctly.
//!
//! Schema: `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json`
//!
//! ## Four signal kinds (graduated)
//!
//! - **squelch**: steward discretion — do not propagate further. Private
//!   to that peer's forwarding decisions. evidenceCid is optional.
//! - **correction**: epistemic — the claim was wrong; `evidence_cid` is
//!   **required** and must point to a Correction EPR with claims/citations.
//! - **retraction**: author-withdrawal — terminates the propagation chain.
//! - **quarantine**: governance-collective determination — structural cost
//!   imposed; requires mishpat/qahal authorization.
//!
//! Source: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §5.1

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Graduated signal kind for a [`FeedbackSignal`].
///
/// Wire encoding: kebab-case via `#[serde(rename_all = "kebab-case")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum SignalKind {
    /// Steward discretion — do not propagate further. Private to that
    /// peer's forwarding decisions. `evidence_cid` is optional.
    Squelch,
    /// Epistemic — the previous claim was wrong. `evidence_cid` is
    /// **required** and must point to a Correction EPR with claims/citations.
    Correction,
    /// Author-withdrawal — the original author retracts this content.
    /// Terminates the propagation chain.
    Retraction,
    /// Governance-collective determination — structural cost imposed.
    /// Requires mishpat/qahal authorization.
    Quarantine,
}

/// Graduated standing impact for a [`FeedbackSignal`].
///
/// Wire encoding: kebab-case via `#[serde(rename_all = "kebab-case")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum StandingImpact {
    /// Informational — no score change. Used for squelch and speculative
    /// corrections.
    Advisory,
    /// Standing debit recorded; reversal is possible on successful appeal.
    DebitSoft,
    /// Standing debit applied immediately; requires mishpat authorization
    /// for reversal.
    DebitFirm,
}

// ---------------------------------------------------------------------------
// Wire struct
// ---------------------------------------------------------------------------

/// FeedbackSignal EPR kind — Phase 3.5 sense-respond nervous system primitive.
///
/// Invariant: when `signal_kind == SignalKind::Correction`, `evidence_cid`
/// MUST be `Some`. This invariant is enforced by the constructor
/// ([`FeedbackSignal::new_correction`]) and validated by the JSON schema
/// (`if/then` clause on `signalKind == "correction"`).
///
/// All other signal kinds MAY carry an `evidence_cid`, but it is not required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeedbackSignal {
    /// CIDv1 (dag-cbor sha256) of the content EPR being acted on.
    pub target_cid: String,

    /// Graduated signal kind.
    pub signal_kind: SignalKind,

    /// CIDv1 of a Correction EPR with claims and citations.
    /// Required when `signal_kind == SignalKind::Correction`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_cid: Option<String>,

    /// Graduated standing impact.
    pub standing_impact: StandingImpact,

    /// Base64-encoded ed25519 public key of the issuing agent.
    pub signed_by: String,

    /// Base64-encoded ed25519 signature over canonical bytes.
    pub signature: String,
}

impl FeedbackSignal {
    /// Construct a `correction` signal. Enforces that `evidence_cid` is
    /// provided — the schema invariant that `correction` requires evidence.
    pub fn new_correction(
        target_cid: String,
        evidence_cid: String,
        standing_impact: StandingImpact,
        signed_by: String,
        signature: String,
    ) -> Self {
        Self {
            target_cid,
            signal_kind: SignalKind::Correction,
            evidence_cid: Some(evidence_cid),
            standing_impact,
            signed_by,
            signature,
        }
    }

    /// Validate the correction invariant: `correction` signals MUST have
    /// `evidence_cid`. Returns `Ok(())` if valid, `Err` with a description
    /// if not.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.signal_kind == SignalKind::Correction && self.evidence_cid.is_none() {
            return Err("correction signal_kind requires evidence_cid");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(kind: SignalKind, impact: StandingImpact, evidence: Option<&str>) -> FeedbackSignal {
        FeedbackSignal {
            target_cid: "bafyreiabcdef1234567890".to_string(),
            signal_kind: kind,
            evidence_cid: evidence.map(|s| s.to_string()),
            standing_impact: impact,
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // JSON round-trips for all 4 signal_kind values
    // -----------------------------------------------------------------------

    #[test]
    fn json_roundtrip_squelch() {
        let sig = make_signal(SignalKind::Squelch, StandingImpact::Advisory, None);
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"squelch\""), "wire name should be squelch, got: {json}");
        assert!(json.contains("\"advisory\""), "wire name should be advisory, got: {json}");
        let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sig);
    }

    #[test]
    fn json_roundtrip_correction() {
        let sig = make_signal(
            SignalKind::Correction,
            StandingImpact::DebitSoft,
            Some("bafyreiabcdef_correction"),
        );
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"correction\""), "wire name should be correction, got: {json}");
        assert!(json.contains("\"debit-soft\""), "wire name should be debit-soft, got: {json}");
        let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sig);
    }

    #[test]
    fn json_roundtrip_retraction() {
        let sig = make_signal(SignalKind::Retraction, StandingImpact::DebitFirm, None);
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"retraction\""), "wire name should be retraction, got: {json}");
        assert!(json.contains("\"debit-firm\""), "wire name should be debit-firm, got: {json}");
        let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sig);
    }

    #[test]
    fn json_roundtrip_quarantine() {
        let sig = make_signal(SignalKind::Quarantine, StandingImpact::DebitFirm, None);
        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"quarantine\""), "wire name should be quarantine, got: {json}");
        let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sig);
    }

    // -----------------------------------------------------------------------
    // All 3 standing_impact values
    // -----------------------------------------------------------------------

    #[test]
    fn json_roundtrip_standing_impact_all() {
        for (impact, expected) in &[
            (StandingImpact::Advisory, "advisory"),
            (StandingImpact::DebitSoft, "debit-soft"),
            (StandingImpact::DebitFirm, "debit-firm"),
        ] {
            let sig = make_signal(SignalKind::Squelch, *impact, None);
            let json = serde_json::to_string(&sig).unwrap();
            assert!(
                json.contains(&format!("\"{expected}\"")),
                "expected wire name {expected}, got: {json}"
            );
            let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
            assert_eq!(back.standing_impact, *impact);
        }
    }

    // -----------------------------------------------------------------------
    // Required field rejection
    // -----------------------------------------------------------------------

    #[test]
    fn deserialize_rejects_missing_target_cid() {
        let json = r#"{
            "signalKind": "squelch",
            "standingImpact": "advisory",
            "signedBy": "AAAAAAA=",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<FeedbackSignal, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing targetCid");
    }

    #[test]
    fn deserialize_rejects_missing_signal_kind() {
        let json = r#"{
            "targetCid": "bafyreiabcdef1234567890",
            "standingImpact": "advisory",
            "signedBy": "AAAAAAA=",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<FeedbackSignal, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing signalKind");
    }

    #[test]
    fn deserialize_rejects_missing_standing_impact() {
        let json = r#"{
            "targetCid": "bafyreiabcdef1234567890",
            "signalKind": "squelch",
            "signedBy": "AAAAAAA=",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<FeedbackSignal, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing standingImpact");
    }

    #[test]
    fn deserialize_rejects_missing_signed_by() {
        let json = r#"{
            "targetCid": "bafyreiabcdef1234567890",
            "signalKind": "squelch",
            "standingImpact": "advisory",
            "signature": "BBBBBBB="
        }"#;
        let result: Result<FeedbackSignal, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing signedBy");
    }

    #[test]
    fn deserialize_rejects_missing_signature() {
        let json = r#"{
            "targetCid": "bafyreiabcdef1234567890",
            "signalKind": "squelch",
            "standingImpact": "advisory",
            "signedBy": "AAAAAAA="
        }"#;
        let result: Result<FeedbackSignal, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject missing signature");
    }

    // -----------------------------------------------------------------------
    // correction without evidenceCid is rejected by validate()
    // -----------------------------------------------------------------------

    #[test]
    fn validate_rejects_correction_without_evidence_cid() {
        let sig = make_signal(SignalKind::Correction, StandingImpact::DebitSoft, None);
        assert!(
            sig.validate().is_err(),
            "correction without evidence_cid should fail validate()"
        );
    }

    #[test]
    fn validate_accepts_correction_with_evidence_cid() {
        let sig = make_signal(
            SignalKind::Correction,
            StandingImpact::DebitSoft,
            Some("bafyreiabcdef_correction"),
        );
        assert!(
            sig.validate().is_ok(),
            "correction with evidence_cid should pass validate()"
        );
    }

    // -----------------------------------------------------------------------
    // squelch with evidenceCid is allowed (optional)
    // -----------------------------------------------------------------------

    #[test]
    fn validate_allows_squelch_with_evidence_cid() {
        let sig = make_signal(
            SignalKind::Squelch,
            StandingImpact::Advisory,
            Some("bafyreiabcdef_optional_evidence"),
        );
        assert!(
            sig.validate().is_ok(),
            "squelch with evidence_cid should be allowed"
        );
    }

    #[test]
    fn validate_allows_squelch_without_evidence_cid() {
        let sig = make_signal(SignalKind::Squelch, StandingImpact::Advisory, None);
        assert!(sig.validate().is_ok(), "squelch without evidence_cid should be allowed");
    }

    // -----------------------------------------------------------------------
    // MessagePack round-trip (rmp_serde)
    // -----------------------------------------------------------------------

    #[test]
    fn msgpack_roundtrip_all_kinds() {
        let cases = vec![
            make_signal(SignalKind::Squelch, StandingImpact::Advisory, None),
            make_signal(
                SignalKind::Correction,
                StandingImpact::DebitSoft,
                Some("bafyreiabcdef_evidence"),
            ),
            make_signal(SignalKind::Retraction, StandingImpact::DebitFirm, None),
            make_signal(SignalKind::Quarantine, StandingImpact::DebitFirm, None),
        ];

        for sig in cases {
            let bytes = rmp_serde::to_vec_named(&sig)
                .unwrap_or_else(|e| panic!("msgpack encode failed for {sig:?}: {e}"));
            let back: FeedbackSignal = rmp_serde::from_slice(&bytes)
                .unwrap_or_else(|e| panic!("msgpack decode failed for {sig:?}: {e}"));
            assert_eq!(
                back, sig,
                "MessagePack round-trip failed for signal_kind={:?}",
                sig.signal_kind
            );
        }
    }

    #[test]
    fn msgpack_roundtrip_all_standing_impacts() {
        for impact in [
            StandingImpact::Advisory,
            StandingImpact::DebitSoft,
            StandingImpact::DebitFirm,
        ] {
            let sig = make_signal(SignalKind::Squelch, impact, None);
            let bytes = rmp_serde::to_vec_named(&sig).unwrap();
            let back: FeedbackSignal = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(back.standing_impact, impact);
        }
    }

    #[test]
    fn msgpack_bytes_encode_and_decode_identically() {
        let sig = make_signal(
            SignalKind::Correction,
            StandingImpact::DebitFirm,
            Some("bafyreiabcdef_evidence"),
        );
        let bytes1 = rmp_serde::to_vec_named(&sig).unwrap();
        let bytes2 = rmp_serde::to_vec_named(&sig).unwrap();
        assert_eq!(bytes1, bytes2, "identical structs must produce identical MessagePack bytes");
    }

    // -----------------------------------------------------------------------
    // new_correction constructor
    // -----------------------------------------------------------------------

    #[test]
    fn new_correction_sets_signal_kind_and_evidence() {
        let sig = FeedbackSignal::new_correction(
            "bafyreiabcdef_target".to_string(),
            "bafyreiabcdef_evidence".to_string(),
            StandingImpact::DebitSoft,
            "AAAAAAA=".to_string(),
            "BBBBBBB=".to_string(),
        );
        assert_eq!(sig.signal_kind, SignalKind::Correction);
        assert_eq!(sig.evidence_cid, Some("bafyreiabcdef_evidence".to_string()));
        assert!(sig.validate().is_ok());
    }
}
