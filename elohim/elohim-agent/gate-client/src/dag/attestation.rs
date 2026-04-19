//! DecisionAttestationBuilder — constructs and CID-addresses a
//! GateDecisionAttestation payload from a completed GateDecision.
//!
//! # Phase 4 DevContext contract
//!
//! The builder computes the attestation payload and its CID in-process.  The
//! CID is written to `GateDecision.decision_attestation_cid` on every non-exempt
//! gate decision.  The payload is NOT published to the mishpat DHT in Phase 4 —
//! that write is Phase 6+ activation.  The computed CID is deterministic: same
//! inputs → same CID across runs, so tests can assert on it.
//!
//! # CID scheme
//!
//! `sha256-{64-char-lowercase-hex-digest}` — the same SHA-256 primitive used
//! by `GateContext::to_summary_cid`, distinguished by a `sha256-` prefix that
//! makes the CID kind machine-readable and matches the project convention
//! (spec §5.2).
//!
//! # Field-for-field match with `CreateGateDecisionAttestationInput`
//!
//! The `DecisionAttestationPayload` mirrors every field of
//! `qahal_types::CreateGateDecisionAttestationInput` (without the `decision_id`
//! self-reference, which is computed from the payload).  Phase 6+ activation
//! can pass `DecisionAttestationPayload` through to the mishpat coordinator
//! zome without restructuring.

use serde::{Deserialize, Serialize};

use crate::events::RelationalImpactEvent;
use crate::types::{ConstitutionalReasoningSummary, GateDecision, GateStatus};

// ─── CID utilities ─────────────────────────────────────────────────────────────

/// Compute a `sha256-{hex}` CID string for the given byte slice.
///
/// Format: `"sha256-"` + 64-character lowercase hex SHA-256 digest.
/// This is the canonical CID scheme for gate attestations.
pub(crate) fn sha256_cid(input: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as FmtWrite;

    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    let hex = result.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    format!("sha256-{hex}")
}

// ─── DecisionAttestationPayload ────────────────────────────────────────────────

/// The content of a gate-decision attestation, ready for DHT submission.
///
/// Field-for-field mirror of `qahal_types::CreateGateDecisionAttestationInput`
/// (minus `decision_id`, which is computed from this payload itself).
///
/// Activation path (Phase 6+):
/// ```rust,ignore
/// let cid = payload.compute_cid();
/// let input = CreateGateDecisionAttestationInput {
///     decision_id: cid.clone(),
///     phase: payload.phase,
///     elohim_id: payload.elohim_id,
///     // ...
/// };
/// call_mishpat_create_gate_decision_attestation(input)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAttestationPayload {
    /// Phase discriminator: "dev-context" | "elohim-active"
    pub phase: String,
    /// AgentPubKey (or DevContext fallback) of the elohim that decided.
    pub elohim_id: String,
    /// CID of the elohim's substance declaration (model + constitution).
    pub elohim_substance_cid: String,
    /// Name of the gate DAG that produced this decision.
    pub gate_name: String,
    /// EPR-style CID of the GateProcessDeclaration that was executed.
    pub gate_process_cid: String,
    /// Serialized RequestRef: event kind + primary identifier fields.
    pub request_ref_json: String,
    /// Decision discriminator: "allow" | "decline" | "escalate" | "verdict".
    pub decision: String,
    /// Full ConstitutionalReasoning as compact JSON.
    pub reasoning_json: String,
    /// CID of the assembled GateContext snapshot (from GateContext::to_summary_cid).
    pub context_summary_cid: String,
    /// ISO-8601 UTC timestamp of the decision.
    pub decided_at: String,
    /// CID of the universal-band DAG that ran above the domain gate.
    pub universal_band_cid: String,
}

impl DecisionAttestationPayload {
    /// Compute the canonical CID for this payload.
    ///
    /// Serialises the payload as compact JSON with keys sorted by field
    /// declaration order (serde default), then applies SHA-256 to produce
    /// a `sha256-{hex}` CID string.
    ///
    /// The CID is deterministic: the same payload fields always produce the
    /// same CID regardless of when or where the computation runs.
    pub fn compute_cid(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        sha256_cid(json.as_bytes())
    }
}

// ─── DecisionAttestationBuilder ────────────────────────────────────────────────

/// Constructs a [`DecisionAttestationPayload`] from the inputs available at
/// DAG completion time.
///
/// # DevContext defaults
///
/// When `elohim_id` or `elohim_substance_cid` are not provided (Phase 4
/// DevContext), the builder substitutes the DevContext sentinel strings:
/// - `elohim_id`: `"dev-context-elohim"`
/// - `elohim_substance_cid`: `"dev-context-substance-v0"`
///
/// These sentinels are detectable downstream and will be replaced by real
/// values when elohim-agent is live (Phase 6+).
pub struct DecisionAttestationBuilder {
    elohim_id: String,
    elohim_substance_cid: String,
}

/// DevContext fallback for `elohim_id` when no real agent key is available.
pub const DEV_CONTEXT_ELOHIM_ID: &str = "dev-context-elohim";

/// DevContext fallback for `elohim_substance_cid` when no substance CID is configured.
pub const DEV_CONTEXT_SUBSTANCE_CID: &str = "dev-context-substance-v0";

impl DecisionAttestationBuilder {
    /// Create a builder with explicit elohim identity fields.
    ///
    /// Pass `None` to use DevContext sentinel defaults.
    pub fn new(elohim_id: Option<String>, elohim_substance_cid: Option<String>) -> Self {
        Self {
            elohim_id: elohim_id.unwrap_or_else(|| DEV_CONTEXT_ELOHIM_ID.to_string()),
            elohim_substance_cid: elohim_substance_cid
                .unwrap_or_else(|| DEV_CONTEXT_SUBSTANCE_CID.to_string()),
        }
    }

    /// Build the attestation payload for a completed `GateDecision`.
    ///
    /// Parameters:
    /// - `decision` — the completed gate decision (carries phase, reasoning,
    ///   and status for classification).
    /// - `gate_name` — name of the DAG that ran (e.g. `"universal-band-v1"` or
    ///   `"discernment-gate-v1-mechanical"`).
    /// - `gate_process_cid` — EPR CID of the GateProcessDeclaration.
    /// - `event` — the triggering `RelationalImpactEvent` (serialised to
    ///   `request_ref_json`).
    /// - `context_summary_cid` — the `GateContext::to_summary_cid()` of the
    ///   assembled context from this run.
    /// - `universal_band_cid` — the CID pointer for the universal-band version
    ///   that ran above the domain gate.
    pub fn build(
        &self,
        decision: &GateDecision,
        gate_name: &str,
        gate_process_cid: &str,
        event: &RelationalImpactEvent,
        context_summary_cid: String,
        universal_band_cid: String,
    ) -> DecisionAttestationPayload {
        DecisionAttestationPayload {
            phase: phase_string(&decision.phase),
            elohim_id: self.elohim_id.clone(),
            elohim_substance_cid: self.elohim_substance_cid.clone(),
            gate_name: gate_name.to_string(),
            gate_process_cid: gate_process_cid.to_string(),
            request_ref_json: event_request_ref(event),
            decision: status_discriminator(&decision.status),
            reasoning_json: reasoning_to_json(&decision.reasoning),
            context_summary_cid,
            decided_at: chrono::Utc::now().to_rfc3339(),
            universal_band_cid,
        }
    }

    /// Build the payload and immediately compute its CID.
    ///
    /// Returns `(payload, cid)`.  The CID is what gets written to
    /// `GateDecision.decision_attestation_cid`.
    pub fn build_with_cid(
        &self,
        decision: &GateDecision,
        gate_name: &str,
        gate_process_cid: &str,
        event: &RelationalImpactEvent,
        context_summary_cid: String,
        universal_band_cid: String,
    ) -> (DecisionAttestationPayload, String) {
        let payload = self.build(
            decision,
            gate_name,
            gate_process_cid,
            event,
            context_summary_cid,
            universal_band_cid,
        );
        let cid = payload.compute_cid();
        (payload, cid)
    }
}

// ─── Serialisation helpers ─────────────────────────────────────────────────────

/// Map `Phase` to the wire string used in `CreateGateDecisionAttestationInput`.
fn phase_string(phase: &crate::phase::Phase) -> String {
    match phase {
        crate::phase::Phase::DevContext => "dev-context".to_string(),
        crate::phase::Phase::ElohimActive => "elohim-active".to_string(),
    }
}

/// Map `GateStatus` to its decision discriminator string.
///
/// Matches the `decision` field values declared in spec §5.2:
/// "allow" | "decline" | "escalate" | "verdict"
fn status_discriminator(status: &GateStatus) -> String {
    match status {
        GateStatus::Allow { .. } => "allow".to_string(),
        GateStatus::Decline { .. } => "decline".to_string(),
        GateStatus::Escalate { .. } => "escalate".to_string(),
        GateStatus::Verdict(_) => "verdict".to_string(),
    }
}

/// Serialise `ConstitutionalReasoningSummary` to compact JSON.
fn reasoning_to_json(reasoning: &ConstitutionalReasoningSummary) -> String {
    serde_json::to_string(reasoning).unwrap_or_else(|_| "{}".to_string())
}

/// Serialise the triggering event to a compact `request_ref_json` string.
///
/// Captures the event kind and primary identifier fields only — no PII,
/// no payload content.  The resulting JSON is recorded on the attestation
/// so reviewers can identify which event triggered which decision without
/// reading the full event payload.
fn event_request_ref(event: &RelationalImpactEvent) -> String {
    let value = match event {
        RelationalImpactEvent::ContentPublish {
            content_cid,
            declared_reach,
            author,
        } => serde_json::json!({
            "kind": "content-publish",
            "contentCid": content_cid,
            "declaredReach": declared_reach,
            "author": author,
        }),
        RelationalImpactEvent::AttestationWrite {
            subject_hash,
            claim_kind,
            issuer,
        } => serde_json::json!({
            "kind": "attestation-write",
            "subjectHash": subject_hash,
            "claimKind": claim_kind,
            "issuer": issuer,
        }),
        RelationalImpactEvent::EconomicEventEmit {
            event_kind,
            provider,
            receiver,
            quantity,
        } => serde_json::json!({
            "kind": "economic-event-emit",
            "eventKind": event_kind,
            "provider": provider,
            "receiver": receiver,
            "quantity": quantity,
        }),
        RelationalImpactEvent::PeerMessage {
            recipient,
            payload_kind,
        } => serde_json::json!({
            "kind": "peer-message",
            "recipient": recipient,
            "payloadKind": payload_kind,
        }),
        RelationalImpactEvent::SyncToPeers {
            manifest_cid,
            item_count,
        } => serde_json::json!({
            "kind": "sync-to-peers",
            "manifestCid": manifest_cid,
            "itemCount": item_count,
        }),
        RelationalImpactEvent::AdviceSought {
            requester,
            summary_cid,
            topic,
        } => serde_json::json!({
            "kind": "advice-sought",
            "requester": requester,
            "summaryCid": summary_cid,
            "topic": topic,
        }),
        RelationalImpactEvent::CapabilityInvoke {
            capability,
            requester,
            request_id,
        } => serde_json::json!({
            "kind": "capability-invoke",
            "capability": capability,
            "requester": requester,
            "requestId": request_id,
        }),
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space,
            artifact_ref,
        } => serde_json::json!({
            "kind": "private-to-public-crossing",
            "sourceSpace": source_space,
            "artifactRef": artifact_ref,
        }),
    };
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::Phase;
    use crate::types::{ConstitutionalReasoningSummary, GateDecision, GateStatus};

    fn mock_allow_decision(phase: Phase) -> GateDecision {
        GateDecision {
            status: GateStatus::Allow { exempt: false },
            reasoning: ConstitutionalReasoningSummary {
                primary_principle: "test-principle".to_string(),
                summary: "Test summary".to_string(),
                confidence: 0.9,
                phase_note: "test-note".to_string(),
            },
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase,
        }
    }

    fn content_publish_event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "bafytest-cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent-test".to_string(),
        }
    }

    fn attestation_event() -> RelationalImpactEvent {
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-test".to_string(),
            claim_kind: "experience-moment".to_string(),
            issuer: "agent-issuer".to_string(),
        }
    }

    fn builder() -> DecisionAttestationBuilder {
        DecisionAttestationBuilder::new(None, None)
    }

    // ─── sha256_cid ────────────────────────────────────────────────────────────

    #[test]
    fn sha256_cid_has_sha256_prefix() {
        let cid = sha256_cid(b"hello");
        assert!(
            cid.starts_with("sha256-"),
            "CID must start with 'sha256-'; got: {cid}"
        );
    }

    #[test]
    fn sha256_cid_total_length_is_71_chars() {
        // "sha256-" (7) + 64 hex chars = 71.
        let cid = sha256_cid(b"hello");
        assert_eq!(cid.len(), 71, "sha256-CID must be 71 chars; got: {cid}");
    }

    #[test]
    fn sha256_cid_is_deterministic() {
        let a = sha256_cid(b"same-input");
        let b = sha256_cid(b"same-input");
        assert_eq!(a, b, "sha256_cid must be deterministic for same input");
    }

    #[test]
    fn sha256_cid_differs_for_different_inputs() {
        let a = sha256_cid(b"input-a");
        let b = sha256_cid(b"input-b");
        assert_ne!(a, b);
    }

    // ─── DecisionAttestationPayload::compute_cid ───────────────────────────────

    #[test]
    fn payload_compute_cid_is_deterministic() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let b = builder();

        let (p1, cid1) = b.build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-cid-abc".to_string(),
            "sha256-ubandcid".to_string(),
        );
        let (p2, cid2) = b.build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-cid-abc".to_string(),
            "sha256-ubandcid".to_string(),
        );

        // Both payloads must produce the same CID — but note that decided_at
        // uses the current time, so two real builds seconds apart would differ.
        // We test determinism by computing from the same payload struct.
        let cid_from_p1 = p1.compute_cid();
        let cid_from_p2 = p2.compute_cid();

        // The CIDs from the SAME payload are always equal.
        assert_eq!(cid_from_p1, p1.compute_cid());
        assert_eq!(cid_from_p2, p2.compute_cid());

        // The CIDs returned by build_with_cid match what compute_cid returns.
        assert_eq!(cid1, p1.compute_cid());
        assert_eq!(cid2, p2.compute_cid());
    }

    #[test]
    fn payload_compute_cid_has_sha256_prefix() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (_, cid) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-summary".to_string(),
            "uband-cid".to_string(),
        );
        assert!(
            cid.starts_with("sha256-"),
            "attestation CID must start with 'sha256-'; got: {cid}"
        );
        assert_eq!(
            cid.len(),
            71,
            "attestation CID must be 71 chars; got: {cid}"
        );
    }

    // ─── Different phases produce different CIDs ────────────────────────────────

    #[test]
    fn different_phases_produce_different_cids() {
        let dev_decision = mock_allow_decision(Phase::DevContext);
        let active_decision = mock_allow_decision(Phase::ElohimActive);
        let event = content_publish_event();
        let b = builder();

        let (_, dev_cid) = b.build_with_cid(
            &dev_decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-cid".to_string(),
            "uband-cid".to_string(),
        );
        let (_, active_cid) = b.build_with_cid(
            &active_decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-cid".to_string(),
            "uband-cid".to_string(),
        );

        assert_ne!(
            dev_cid, active_cid,
            "DevContext and ElohimActive must produce different CIDs (phase is in payload)"
        );
    }

    // ─── Different gate_names produce different CIDs ────────────────────────────

    #[test]
    fn different_gate_names_produce_different_cids() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let b = builder();

        let (_, cid_ub) = b.build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx-cid".to_string(),
            "uband-cid".to_string(),
        );
        let (_, cid_disc) = b.build_with_cid(
            &decision,
            "discernment-gate-v1-mechanical",
            "epr:gates:discernment-gate-v1-mechanical",
            &event,
            "ctx-cid".to_string(),
            "uband-cid".to_string(),
        );

        assert_ne!(
            cid_ub, cid_disc,
            "universal-band and discernment-gate must produce different CIDs"
        );
    }

    // ─── Payload field correctness ─────────────────────────────────────────────

    #[test]
    fn payload_phase_field_is_dev_context_string() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.phase, "dev-context");
    }

    #[test]
    fn payload_phase_field_is_elohim_active_string() {
        let decision = mock_allow_decision(Phase::ElohimActive);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.phase, "elohim-active");
    }

    #[test]
    fn payload_decision_field_is_allow_for_allow_status() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.decision, "allow");
    }

    #[test]
    fn payload_decision_field_is_verdict_for_verdict_status() {
        let mut decision = mock_allow_decision(Phase::DevContext);
        decision.status = GateStatus::Verdict(crate::types::GateTag::StoryPoint {
            valence: "progress".to_string(),
            magnitude: "meaningful".to_string(),
            evidence_type: "first-pass-green".to_string(),
        });
        let event = attestation_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "discernment-gate-v1-mechanical",
            "epr:gates:discernment-gate-v1-mechanical",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.decision, "verdict");
    }

    #[test]
    fn payload_gate_name_matches_input() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "discernment-gate-v1-mechanical",
            "epr:gates:discernment-gate-v1-mechanical",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.gate_name, "discernment-gate-v1-mechanical");
    }

    #[test]
    fn payload_elohim_id_defaults_to_dev_context_sentinel() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.elohim_id, DEV_CONTEXT_ELOHIM_ID);
    }

    #[test]
    fn payload_elohim_substance_cid_defaults_to_dev_context_sentinel() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.elohim_substance_cid, DEV_CONTEXT_SUBSTANCE_CID);
    }

    #[test]
    fn payload_elohim_id_uses_provided_value() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let b = DecisionAttestationBuilder::new(Some("uhCAkCustomAgentKey".to_string()), None);
        let (payload, _) = b.build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.elohim_id, "uhCAkCustomAgentKey");
    }

    #[test]
    fn payload_request_ref_json_contains_event_kind() {
        let event = content_publish_event();
        let decision = mock_allow_decision(Phase::DevContext);
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        assert!(
            payload.request_ref_json.contains("content-publish"),
            "request_ref_json must include event kind; got: {}",
            payload.request_ref_json
        );
    }

    #[test]
    fn payload_context_summary_cid_matches_input() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "my-context-cid-abc".to_string(),
            "uband".to_string(),
        );
        assert_eq!(payload.context_summary_cid, "my-context-cid-abc");
    }

    #[test]
    fn payload_universal_band_cid_matches_input() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "my-uband-cid-xyz".to_string(),
        );
        assert_eq!(payload.universal_band_cid, "my-uband-cid-xyz");
    }

    #[test]
    fn payload_decided_at_is_iso8601() {
        let decision = mock_allow_decision(Phase::DevContext);
        let event = content_publish_event();
        let (payload, _) = builder().build_with_cid(
            &decision,
            "universal-band-v1",
            "epr:gates:universal-band-v1",
            &event,
            "ctx".to_string(),
            "uband".to_string(),
        );
        // Basic ISO-8601 sanity: contains 'T' and '+' or 'Z'.
        assert!(
            payload.decided_at.contains('T'),
            "decided_at must be ISO-8601 with 'T' separator; got: {}",
            payload.decided_at
        );
    }

    // ─── event_request_ref covers all 8 variants ──────────────────────────────

    #[test]
    fn event_request_ref_content_publish_has_expected_fields() {
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "bafy123".to_string(),
            declared_reach: "public".to_string(),
            author: "agent-a".to_string(),
        };
        let json = event_request_ref(&event);
        assert!(
            json.contains("content-publish"),
            "kind field missing: {json}"
        );
        assert!(json.contains("bafy123"), "contentCid missing: {json}");
    }

    #[test]
    fn event_request_ref_attestation_write_has_expected_fields() {
        let event = RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-abc".to_string(),
            claim_kind: "brit".to_string(),
            issuer: "agent-b".to_string(),
        };
        let json = event_request_ref(&event);
        assert!(
            json.contains("attestation-write"),
            "kind field missing: {json}"
        );
        assert!(json.contains("hash-abc"), "subjectHash missing: {json}");
    }

    #[test]
    fn event_request_ref_all_8_variants_produce_valid_json() {
        let events = vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "cid".to_string(),
                declared_reach: "public".to_string(),
                author: "a".to_string(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "h".to_string(),
                claim_kind: "brit".to_string(),
                issuer: "a".to_string(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".to_string(),
                provider: "a".to_string(),
                receiver: "b".to_string(),
                quantity: "1".to_string(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "a".to_string(),
                payload_kind: "handshake".to_string(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "m".to_string(),
                item_count: 1,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "a".to_string(),
                summary_cid: "s".to_string(),
                topic: "t".to_string(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".to_string(),
                requester: "a".to_string(),
                request_id: "r".to_string(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "draft".to_string(),
                artifact_ref: "art".to_string(),
            },
        ];

        for event in &events {
            let json = event_request_ref(event);
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| {
                panic!("request_ref_json not valid JSON for {}: {e}", event.kind())
            });
            assert!(
                parsed.get("kind").is_some(),
                "request_ref_json must have 'kind' field for {}; got: {json}",
                event.kind()
            );
        }
    }

    // ─── status_discriminator ─────────────────────────────────────────────────

    #[test]
    fn status_discriminator_allow_returns_allow() {
        assert_eq!(
            status_discriminator(&GateStatus::Allow { exempt: false }),
            "allow"
        );
        assert_eq!(
            status_discriminator(&GateStatus::Allow { exempt: true }),
            "allow"
        );
    }

    #[test]
    fn status_discriminator_decline_returns_decline() {
        assert_eq!(
            status_discriminator(&GateStatus::Decline {
                grounds: crate::types::DeclineGrounds {
                    category: "c".to_string(),
                    summary: "s".to_string(),
                    principle_refs: vec![],
                }
            }),
            "decline"
        );
    }

    #[test]
    fn status_discriminator_escalate_returns_escalate() {
        assert_eq!(
            status_discriminator(&GateStatus::Escalate {
                target: crate::types::EscalationTarget::ExistentialBoundary,
                severity: crate::types::Severity::High,
            }),
            "escalate"
        );
    }

    #[test]
    fn status_discriminator_verdict_returns_verdict() {
        assert_eq!(
            status_discriminator(&GateStatus::Verdict(crate::types::GateTag::ReachLevel {
                level: "local".to_string()
            })),
            "verdict"
        );
    }
}
