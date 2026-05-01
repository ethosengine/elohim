//! FeedbackSignal integrity entry type — Phase 3.5 P3.5.1 (DHT layer).
//!
//! Mirrors the wire shape from `elohim-storage/src/p2p/feedback_signal.rs`
//! (T1), adapted for the Holochain HDI boundary:
//!
//! - Enum types (`SignalKind`, `StandingImpact`) are replaced with `String`
//!   plus module-level whitelist constants. Cross-crate enum imports would
//!   couple the integrity zome to `elohim-storage`; string + const is the
//!   same pattern `Manifest::manifest_kind` uses.
//!
//! - All fields are concrete primitives — no `serde_json::Value` on the zome
//!   boundary (see `feedback_serde_json_value_breaks_zome_boundary`).
//!
//! ## Validation floors (deterministic, create-time)
//!
//! 1. `signal_kind` must be one of the four bootstrap kinds.
//! 2. `standing_impact` must be one of the three bootstrap impacts.
//! 3. `squelch` ⇒ `standing_impact` MUST be `advisory`.
//! 4. `correction` ⇒ `evidence_cid` MUST be `Some`.
//!
//! ## Cross-entity rules (NOT here)
//!
//! The following rules require DHT lookups or link traversal and therefore
//! belong in the T8 coordinator pre-commit gate, NOT in this HDI validator
//! (see `project_hdi_no_get_links_in_validators`):
//!
//! - Retraction signer must equal the original author of `target_cid`
//!   (requires `must_get_action` over target's action hash).
//! - `correction`'s `evidence_cid` must point to an existing Correction EPR
//!   (requires DHT lookup on the evidence hash).
//!
//! These rules are listed here so a future reader knows they are intentionally
//! deferred, not forgotten.

use hdi::prelude::*;

/// Bootstrap `signal_kind` taxonomy.
///
/// Values mirror `SignalKind` enum wire names (kebab-case) from T1.
/// `"vouch"` added in T5 (Light Up the Graph sprint).
const SIGNAL_KINDS: &[&str] = &["squelch", "correction", "retraction", "quarantine", "vouch"];

/// Bootstrap `vouch_kind` taxonomy.
///
/// Required when `signal_kind == "vouch"`. Mirrors `VouchKind` enum wire names.
const VOUCH_KINDS: &[&str] = &["accept-correction", "restitution"];

/// Bootstrap `standing_impact` taxonomy.
///
/// Values mirror `StandingImpact` enum wire names (kebab-case) from T1.
const STANDING_IMPACTS: &[&str] = &["advisory", "debit-soft", "debit-firm"];

/// FeedbackSignal integrity entry — Phase 3.5 sense-respond nervous system.
///
/// One FeedbackSignal per authored signal event. DHT anchors signer identity
/// (unforgeable agent key), content identity (`target_cid`), and signal
/// classification; the full operational record (with projections, counts,
/// standing deltas) lives in `elohim-storage` SQLite, joined on the
/// `dht_anchor_hash`.
///
/// The `signer_pubkey` field carries the raw ed25519 public key bytes. On the
/// wire (T1) this is base64-encoded; in the integrity zome we store the raw
/// bytes so the DHT anchor is compact and canonical. The coordinator (T8) is
/// responsible for verifying the signature before calling `create_entry`.
///
/// `vouch_kind` added in T5 (Light Up the Graph sprint) — required when
/// `signal_kind == "vouch"`, forbidden otherwise.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct FeedbackSignal {
    /// CIDv1 (dag-cbor sha256) of the content EPR being acted on.
    pub target_cid: String,

    /// Graduated signal kind. Whitelisted: squelch / correction /
    /// retraction / quarantine / vouch.
    pub signal_kind: String,

    /// Vouch sub-semantics. Required when `signal_kind == "vouch"`.
    /// Whitelisted: accept-correction / restitution.
    /// Must be `None` for all other signal kinds.
    pub vouch_kind: Option<String>,

    /// CIDv1 of a Correction EPR with claims and citations.
    /// Required when `signal_kind == "correction"`.
    pub evidence_cid: Option<String>,

    /// Graduated standing impact. Whitelisted: advisory / debit-soft /
    /// debit-firm.
    pub standing_impact: String,

    /// Raw ed25519 public key bytes of the issuing agent.
    pub signer_pubkey: Vec<u8>,
}

impl FeedbackSignal {
    /// Phase 3.5 create-time floor checks. Deterministic — uses only
    /// `must_get_*` primitives where DHT access is needed (vouch no-self-vouch).
    /// Returns `Ok(())` when valid; `Err(reason)` when a floor is violated.
    pub fn validate(&self) -> Result<(), String> {
        // Floor 1: signal_kind whitelist.
        if !SIGNAL_KINDS.contains(&self.signal_kind.as_str()) {
            return Err(format!(
                "unknown signal_kind: {} (allowed: {:?})",
                self.signal_kind, SIGNAL_KINDS
            ));
        }

        // Floor 2: standing_impact whitelist.
        if !STANDING_IMPACTS.contains(&self.standing_impact.as_str()) {
            return Err(format!(
                "unknown standing_impact: {} (allowed: {:?})",
                self.standing_impact, STANDING_IMPACTS
            ));
        }

        // Floor 3: squelch must be advisory.
        //
        // Squelch is a steward's private discretion signal — it must never
        // carry a scoring debit, since squelch visibility is per-peer and
        // not collectively attested.
        if self.signal_kind == "squelch" && self.standing_impact != "advisory" {
            return Err(format!(
                "squelch signal must have standing_impact=advisory, got {}",
                self.standing_impact
            ));
        }

        // Floor 4: correction requires evidence_cid.
        //
        // Epistemic corrections without cited evidence are bare assertions.
        // The evidence CID must point to a Correction EPR (enforced at the
        // coordinator level in T8 — see module doc for why that is deferred).
        if self.signal_kind == "correction" && self.evidence_cid.is_none() {
            return Err("correction signal requires evidence_cid".to_string());
        }

        // Floor 5 (T5): vouch validation.
        //
        // vouch requires vouch_kind ∈ VOUCH_KINDS.
        // Non-vouch must NOT carry vouch_kind (strict field invariant).
        // No-self-vouch enforced in the coordinator (coordinator has access
        // to must_get_valid_record for the target signal resolution).
        if self.signal_kind == "vouch" {
            let vk = match &self.vouch_kind {
                Some(v) => v,
                None => return Err("vouch signal requires vouch_kind".to_string()),
            };
            if !VOUCH_KINDS.contains(&vk.as_str()) {
                return Err(format!(
                    "unknown vouch_kind: {} (allowed: {:?})",
                    vk, VOUCH_KINDS
                ));
            }
        } else if self.vouch_kind.is_some() {
            return Err(format!(
                "vouch_kind must be None for signal_kind={}, got: {:?}",
                self.signal_kind, self.vouch_kind
            ));
        }

        Ok(())
    }
}

// =============================================================================
// Inline tests (host target; no HDK context required)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_signal(
        signal_kind: &str,
        standing_impact: &str,
        evidence_cid: Option<&str>,
    ) -> FeedbackSignal {
        FeedbackSignal {
            target_cid: "bafyreiabcdef1234567890".to_string(),
            signal_kind: signal_kind.to_string(),
            vouch_kind: None,
            evidence_cid: evidence_cid.map(|s| s.to_string()),
            standing_impact: standing_impact.to_string(),
            signer_pubkey: vec![0u8; 32],
        }
    }

    fn make_vouch(vouch_kind: &str, standing_impact: &str) -> FeedbackSignal {
        FeedbackSignal {
            target_cid: "bafyreiabcdef1234567890".to_string(),
            signal_kind: "vouch".to_string(),
            vouch_kind: Some(vouch_kind.to_string()),
            evidence_cid: None,
            standing_impact: standing_impact.to_string(),
            signer_pubkey: vec![0u8; 32],
        }
    }

    // -----------------------------------------------------------------------
    // Floor 1: signal_kind whitelist
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_squelch() {
        assert!(make_signal("squelch", "advisory", None).validate().is_ok());
    }

    #[test]
    fn accepts_correction_with_evidence() {
        assert!(
            make_signal("correction", "debit-soft", Some("bafyreiabcdef_evidence"))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn accepts_retraction() {
        assert!(make_signal("retraction", "debit-soft", None)
            .validate()
            .is_ok());
    }

    #[test]
    fn accepts_quarantine() {
        assert!(make_signal("quarantine", "debit-firm", None)
            .validate()
            .is_ok());
    }

    #[test]
    fn rejects_unknown_signal_kind() {
        let result = make_signal("ban", "advisory", None).validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("unknown signal_kind"),
            "expected signal_kind mention, got: {msg}"
        );
        assert!(
            msg.contains("ban"),
            "expected offending value in error, got: {msg}"
        );
    }

    #[test]
    fn rejects_empty_signal_kind() {
        let result = make_signal("", "advisory", None).validate();
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Floor 2: standing_impact whitelist
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_advisory_impact() {
        assert!(make_signal("squelch", "advisory", None).validate().is_ok());
    }

    #[test]
    fn accepts_debit_soft_impact() {
        assert!(make_signal("retraction", "debit-soft", None)
            .validate()
            .is_ok());
    }

    #[test]
    fn accepts_debit_firm_impact() {
        assert!(make_signal("quarantine", "debit-firm", None)
            .validate()
            .is_ok());
    }

    #[test]
    fn rejects_unknown_standing_impact() {
        let result = make_signal("squelch", "advisory", None).validate();
        assert!(result.is_ok()); // baseline

        let result = make_signal("retraction", "hard-ban", None).validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("unknown standing_impact"), "got: {msg}");
        assert!(msg.contains("hard-ban"), "got: {msg}");
    }

    #[test]
    fn rejects_empty_standing_impact() {
        let result = make_signal("retraction", "", None).validate();
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Floor 3: squelch ⇒ advisory
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_squelch_with_debit_soft() {
        let result = make_signal("squelch", "debit-soft", None).validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("squelch signal must have standing_impact=advisory"),
            "got: {msg}"
        );
        assert!(msg.contains("debit-soft"), "got: {msg}");
    }

    #[test]
    fn rejects_squelch_with_debit_firm() {
        let result = make_signal("squelch", "debit-firm", None).validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("debit-firm"), "got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Floor 4: correction ⇒ evidence_cid required
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_correction_without_evidence_cid() {
        let result = make_signal("correction", "debit-soft", None).validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("correction signal requires evidence_cid"),
            "got: {msg}"
        );
    }

    #[test]
    fn accepts_correction_with_evidence_cid() {
        assert!(
            make_signal("correction", "debit-firm", Some("bafyreiabcdef_evidence"))
                .validate()
                .is_ok()
        );
    }

    // -----------------------------------------------------------------------
    // evidence_cid is optional for non-correction kinds
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_retraction_with_evidence_cid() {
        // retraction may optionally carry evidence (not required)
        assert!(
            make_signal("retraction", "debit-firm", Some("bafyreiabcdef_evidence"))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn accepts_quarantine_with_evidence_cid() {
        assert!(
            make_signal("quarantine", "debit-firm", Some("bafyreiabcdef_evidence"))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn accepts_squelch_with_evidence_cid() {
        assert!(
            make_signal("squelch", "advisory", Some("bafyreiabcdef_evidence"))
                .validate()
                .is_ok()
        );
    }

    // -----------------------------------------------------------------------
    // Update-rejection coverage note
    // -----------------------------------------------------------------------
    //
    // `validate_update_entry` in `lib.rs` explicitly rejects any update to a
    // FeedbackSignal entry with:
    //
    //   ValidateCallbackResult::Invalid("FeedbackSignal entries are immutable…")
    //
    // A direct unit test for that path cannot be written here because the
    // dispatch function takes `&EntryTypes` — a type generated by the
    // `#[hdk_entry_types]` proc-macro that depends on the HDI host context
    // (wasm32-unknown-unknown target).  Constructing `EntryTypes::FeedbackSignal`
    // in a cfg(test) native build would require linking against a host mock that
    // does not exist at the integrity-zome level.
    //
    // This is the same constraint that defers cross-entity rules to the
    // coordinator (see module doc).  The update-rejection path is covered by
    // the T8 sweettest (`validate_feedback_signal_update_is_rejected`) which
    // runs inside a real Holochain conductor.

    // -----------------------------------------------------------------------
    // Floor 5: vouch validation (T5 additions)
    // -----------------------------------------------------------------------

    #[test]
    fn accepts_vouch_accept_correction() {
        assert!(make_vouch("accept-correction", "debit-soft")
            .validate()
            .is_ok());
    }

    #[test]
    fn accepts_vouch_restitution() {
        assert!(make_vouch("restitution", "advisory").validate().is_ok());
    }

    #[test]
    fn rejects_vouch_without_vouch_kind() {
        let mut sig = make_vouch("accept-correction", "debit-soft");
        sig.vouch_kind = None;
        let result = sig.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("vouch signal requires vouch_kind"),
            "got: {msg}"
        );
    }

    #[test]
    fn rejects_vouch_with_unknown_vouch_kind() {
        let mut sig = make_vouch("accept-correction", "debit-soft");
        sig.vouch_kind = Some("endorse".to_string());
        let result = sig.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("unknown vouch_kind"), "got: {msg}");
        assert!(msg.contains("endorse"), "got: {msg}");
    }

    #[test]
    fn rejects_non_vouch_with_vouch_kind_set() {
        let mut sig = make_signal("squelch", "advisory", None);
        sig.vouch_kind = Some("accept-correction".to_string());
        let result = sig.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("vouch_kind must be None for signal_kind=squelch"),
            "got: {msg}"
        );
    }

    #[test]
    fn rejects_correction_with_vouch_kind_set() {
        let mut sig = make_signal("correction", "debit-soft", Some("bafyreievidence"));
        sig.vouch_kind = Some("accept-correction".to_string());
        let result = sig.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("vouch_kind must be None"), "got: {msg}");
    }

    #[test]
    fn vouch_does_not_require_evidence_cid() {
        // vouch targets a FeedbackSignal, not a content EPR — evidence is not required.
        let sig = make_vouch("accept-correction", "debit-soft");
        assert!(
            sig.evidence_cid.is_none(),
            "vouch must not require evidence_cid"
        );
        assert!(sig.validate().is_ok());
    }
}
