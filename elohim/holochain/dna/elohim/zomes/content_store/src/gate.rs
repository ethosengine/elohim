//! Gate check helpers for the content_store coordinator zome.
//!
//! All gate calls fire BEFORE any DHT commit — this is the wisdom-as-auth
//! invariant: no entry reaches the source chain unless the gate allows it.
//!
//! # Routing logic
//!
//! Content types that are attestation events (`experience-moment`) emit
//! [`RelationalImpactEvent::AttestationWrite`] so that the Phase 6 dispatcher
//! routes them through BOTH `content-safety-gate-v1` AND
//! `discernment-gate-v1-mechanical`.  All other content types emit
//! [`RelationalImpactEvent::ContentPublish`], which routes through
//! `content-safety-gate-v1` only.
//!
//! # DevContext behaviour
//!
//! In Phase 7 DevContext both gates return `Allow` unconditionally.  The seam
//! wiring — not real gate execution — is the deliverable here.  Phase 8+ will
//! replace the pure-sync mock with a real elohim-agent-service bridge.
//!
//! # subject_hash derivation for AttestationWrite
//!
//! The `subject_hash` field identifies **what the moment attests about** — the
//! parent `experience-story` ContentNode.  It is read from
//! `metadata_json.relatedExperienceStory` per the lamad schema:
//! `elohim/sdk/domains/lamad/schemas/experience-moment-metadata.schema.json`.
//!
//! If the field is absent (e.g. the caller omitted it or the JSON is
//! malformed), the moment's own `id` is used as a safe fallback.  Phase 7
//! DevContext does not inspect the value; Phase 8+ will require it to be a
//! valid content-addressed reference.

use gate_client_zome::{check_blocking, GateStatus, RelationalImpactEvent};
use hdk::prelude::*;
use lamad_types::CreateContentInput;

/// Discriminator constant for experience-moment attestation events.
///
/// Used as `claim_kind` in [`RelationalImpactEvent::AttestationWrite`].
/// Keeping this a compile-time constant prevents typo drift between zome
/// code and any future gate-rule that matches on `claim_kind`.
pub const EXPERIENCE_MOMENT_CLAIM_KIND: &str = "experience-moment-recording";

/// Content type string for the experience-moment domain type.
///
/// Matches the key declared in the lamad manifest under
/// `vocabulary.contentTypes."experience-moment"`.
const EXPERIENCE_MOMENT_CONTENT_TYPE: &str = "experience-moment";

// ─── Unified gate check ───────────────────────────────────────────────────────

/// Invoke the appropriate gate(s) for a content creation, before any DHT
/// commit.
///
/// Routes to [`RelationalImpactEvent::AttestationWrite`] for
/// `experience-moment` content (dispatches through content-safety-gate AND
/// discernment-gate per Phase 6 ordering), and to
/// [`RelationalImpactEvent::ContentPublish`] for all other content types
/// (content-safety-gate only).
///
/// Replaces the former `gate_check_content_publish` helper introduced in
/// Task 7.2b.  All callers that previously invoked
/// `gate_check_content_publish` should call this function instead.
///
/// # Errors
///
/// Returns a [`WasmError`] if:
/// - `check_blocking` fails (wrapped as `Guest` error)
/// - The gate returns `GateStatus::Decline`
/// - The gate returns `GateStatus::Escalate`
pub fn gate_check_for_content(input: &CreateContentInput) -> ExternResult<()> {
    let event = build_event(input)?;
    let decision = check_blocking(event)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("gate error: {e}"))))?;
    match decision.status {
        GateStatus::Allow { .. } => Ok(()),
        // Verdict carries a tag (e.g. StoryPointTag from discernment-gate).
        // In Phase 7 DevContext the tag is not yet recorded; the DHT write
        // proceeds transparently.  Phase 8+ will persist the verdict tag.
        GateStatus::Verdict(_) => Ok(()),
        GateStatus::Decline { grounds } => {
            let label = if input.content_type == EXPERIENCE_MOMENT_CONTENT_TYPE {
                "experience-moment"
            } else {
                "content"
            };
            Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Gate declined {label}: {}",
                grounds.summary
            ))))
        }
        GateStatus::Escalate { .. } => Err(wasm_error!(WasmErrorInner::Guest(
            "Gate escalated to steward review".to_string()
        ))),
    }
}

// ─── Event construction ───────────────────────────────────────────────────────

/// Build the correct [`RelationalImpactEvent`] variant for the given input.
///
/// Separated from [`gate_check_for_content`] so that tests can assert on the
/// event variant without exercising `check_blocking`.
fn build_event(input: &CreateContentInput) -> ExternResult<RelationalImpactEvent> {
    if input.content_type == EXPERIENCE_MOMENT_CONTENT_TYPE {
        build_attestation_write_event(input)
    } else {
        build_content_publish_event(input)
    }
}

/// Construct [`RelationalImpactEvent::ContentPublish`] for non-attestation
/// content types.
///
/// Shape note: `content_cid` uses `blob_cid` when present, falling back to
/// the content `id`.  The DevContext mock does not inspect the CID value.
/// Phase 8+ will require a fully-resolved CID.
fn build_content_publish_event(input: &CreateContentInput) -> ExternResult<RelationalImpactEvent> {
    let author = agent_info()?.agent_initial_pubkey.to_string();
    let content_cid = input.blob_cid.clone().unwrap_or_else(|| input.id.clone());
    Ok(RelationalImpactEvent::ContentPublish {
        content_cid,
        declared_reach: input.reach.clone(),
        author,
    })
}

/// Construct [`RelationalImpactEvent::AttestationWrite`] for
/// `experience-moment` content.
///
/// # subject_hash derivation
///
/// Parses `metadata_json` and reads `relatedExperienceStory`.  This field
/// holds a CID or slug reference to the parent `experience-story` ContentNode
/// (declared in the lamad manifest as the `ATTESTS_TO` relationship target).
///
/// Falls back to the moment's own `id` when:
/// - `metadata_json` is not valid JSON
/// - `relatedExperienceStory` is absent or not a string
///
/// Phase 7 DevContext: the mock does not inspect `subject_hash`; the fallback
/// is safe.  Phase 8+: callers should ensure `relatedExperienceStory` is
/// populated before raising the activation gate.
fn build_attestation_write_event(
    input: &CreateContentInput,
) -> ExternResult<RelationalImpactEvent> {
    let issuer = agent_info()?.agent_initial_pubkey.to_string();
    let subject_hash =
        extract_related_experience_story(&input.metadata_json).unwrap_or_else(|| input.id.clone());
    Ok(RelationalImpactEvent::AttestationWrite {
        subject_hash,
        claim_kind: EXPERIENCE_MOMENT_CLAIM_KIND.to_string(),
        issuer,
    })
}

/// Extract `relatedExperienceStory` from the experience-moment `metadata_json`
/// blob.
///
/// Returns `None` if the JSON is malformed or the field is absent — callers
/// should substitute an appropriate fallback.
fn extract_related_experience_story(metadata_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(metadata_json).ok()?;
    parsed
        .get("relatedExperienceStory")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ─── Test-only event builders (no agent_info() call) ─────────────────────────
//
// Production callers use `build_content_publish_event` / `build_attestation_write_event`
// which derive the author from `agent_info()`.  In tests there is no HDK runtime, so
// these variants accept an explicit author string.  They exist solely to let the routing
// assertions (`experience-moment → AttestationWrite`, `lesson → ContentPublish`) run
// without a live conductor.  They are not re-exported and have no production callers.

#[cfg(test)]
fn build_content_publish_event_with_author(
    input: &CreateContentInput,
    author: &str,
) -> RelationalImpactEvent {
    let content_cid = input.blob_cid.clone().unwrap_or_else(|| input.id.clone());
    RelationalImpactEvent::ContentPublish {
        content_cid,
        declared_reach: input.reach.clone(),
        author: author.to_string(),
    }
}

#[cfg(test)]
fn build_attestation_write_event_with_author(
    input: &CreateContentInput,
    issuer: &str,
) -> RelationalImpactEvent {
    let subject_hash =
        extract_related_experience_story(&input.metadata_json).unwrap_or_else(|| input.id.clone());
    RelationalImpactEvent::AttestationWrite {
        subject_hash,
        claim_kind: EXPERIENCE_MOMENT_CLAIM_KIND.to_string(),
        issuer: issuer.to_string(),
    }
}

/// Test-only: invoke `check_blocking` on a pre-built event and map the result
/// to `ExternResult<()>` using the same error-shape as `gate_check_for_content`.
///
/// This lets decline/escalate path assertions run without an HDK runtime (the
/// real `gate_check_for_content` calls `build_event` → `agent_info()` first).
/// Production callers MUST use `gate_check_for_content`.
#[cfg(test)]
fn gate_check_with_event(event: RelationalImpactEvent) -> ExternResult<()> {
    let decision = check_blocking(event)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("gate error: {e}"))))?;
    match decision.status {
        GateStatus::Allow { .. } | GateStatus::Verdict(_) => Ok(()),
        GateStatus::Decline { grounds } => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Gate declined content: {}",
            grounds.summary
        )))),
        GateStatus::Escalate { .. } => Err(wasm_error!(WasmErrorInner::Guest(
            "Gate escalated to steward review".to_string()
        ))),
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn base_input(content_type: &str, metadata_json: &str) -> CreateContentInput {
        CreateContentInput {
            id: "test-id-001".to_string(),
            content_type: content_type.to_string(),
            title: "Test".to_string(),
            description: "Test description".to_string(),
            summary: None,
            content: String::new(),
            content_format: "markdown".to_string(),
            tags: vec![],
            source_path: None,
            related_node_ids: vec![],
            reach: "commons".to_string(),
            estimated_minutes: None,
            thumbnail_url: None,
            metadata_json: metadata_json.to_string(),
            blob_cid: None,
            content_size_bytes: None,
            content_hash: None,
        }
    }

    fn experience_moment_metadata(related_story: Option<&str>) -> String {
        match related_story {
            Some(story) => format!(
                r#"{{"subjectRef":"persona:alice","roleRef":"role:learner","featureRef":"feat:001","scenarioName":"Test scenario","scenarioUri":"lamad/test.feature","status":"passed","durationMs":1000,"commit":"abc1234","runId":"run-1","computeFingerprint":"pod1:desktop:abcdef01","relatedExperienceStory":"{story}"}}"#
            ),
            None => r#"{"subjectRef":"persona:alice","roleRef":"role:learner","featureRef":"feat:001","scenarioName":"Test scenario","scenarioUri":"lamad/test.feature","status":"passed","durationMs":1000,"commit":"abc1234","runId":"run-1","computeFingerprint":"pod1:desktop:abcdef01"}"#.to_string(),
        }
    }

    // ── extract_related_experience_story ─────────────────────────────────────

    #[test]
    fn extracts_related_experience_story_when_present() {
        let json = experience_moment_metadata(Some("epr:story:abc123"));
        let result = extract_related_experience_story(&json);
        assert_eq!(result, Some("epr:story:abc123".to_string()));
    }

    #[test]
    fn returns_none_when_field_absent() {
        let json = experience_moment_metadata(None);
        let result = extract_related_experience_story(&json);
        assert!(
            result.is_none(),
            "should return None when relatedExperienceStory is absent"
        );
    }

    #[test]
    fn returns_none_on_malformed_json() {
        let result = extract_related_experience_story("not-json{{{");
        assert!(result.is_none(), "should return None on malformed JSON");
    }

    #[test]
    fn returns_none_on_empty_string() {
        let result = extract_related_experience_story("");
        assert!(result.is_none(), "should return None on empty string");
    }

    // ── EXPERIENCE_MOMENT_CLAIM_KIND constant ────────────────────────────────

    #[test]
    fn claim_kind_constant_is_stable() {
        // If this test fails, update gate-types/gate-client tests to match.
        assert_eq!(EXPERIENCE_MOMENT_CLAIM_KIND, "experience-moment-recording");
    }

    // ── Phase 7 E2E — event routing (no agent_info() / HDK required) ─────────
    //
    // These tests use the test-only event builders that accept an explicit author
    // string, bypassing `agent_info()`.  They prove the routing invariant:
    //
    //   experience-moment → AttestationWrite (content-safety + discernment)
    //   all other types   → ContentPublish   (content-safety only)
    //
    // What IS verified:
    //   - Event variant selection based on content_type
    //   - subject_hash derivation (relatedExperienceStory or id fallback)
    //   - claim_kind constant stability
    //   - blob_cid fallback to id for ContentPublish
    //   - gate_check_with_event: Allow → Ok(()), Decline → WasmError, Escalate → WasmError
    //
    // What IS NOT verified:
    //   - agent_info() resolution (requires a live conductor / sweettest harness;
    //     no sweettest harness exists in this crate as of Phase 7)
    //   - Actual DHT commit (no conductor running)
    //   - Real wisdom evaluation (DevContext mock; overrides used for Decline tests)

    #[test]
    fn experience_moment_content_type_produces_attestation_write_event() {
        let meta = experience_moment_metadata(Some("epr:story:parent-001"));
        let input = base_input("experience-moment", &meta);
        let event = build_attestation_write_event_with_author(&input, "agent-alice");

        assert!(
            matches!(event, RelationalImpactEvent::AttestationWrite { .. }),
            "experience-moment must produce AttestationWrite; got: {:?}",
            event
        );
        if let RelationalImpactEvent::AttestationWrite {
            claim_kind,
            subject_hash,
            issuer,
        } = event
        {
            assert_eq!(claim_kind, EXPERIENCE_MOMENT_CLAIM_KIND);
            assert_eq!(
                subject_hash, "epr:story:parent-001",
                "subject_hash must be the relatedExperienceStory value"
            );
            assert_eq!(issuer, "agent-alice");
        }
    }

    #[test]
    fn non_experience_moment_content_type_produces_content_publish_event() {
        let input = base_input("lesson", "{}");
        let event = build_content_publish_event_with_author(&input, "agent-bob");

        assert!(
            matches!(event, RelationalImpactEvent::ContentPublish { .. }),
            "non-experience-moment must produce ContentPublish; got: {:?}",
            event
        );
        if let RelationalImpactEvent::ContentPublish {
            content_cid,
            declared_reach,
            author,
        } = event
        {
            // No blob_cid set → falls back to input.id
            assert_eq!(content_cid, "test-id-001");
            assert_eq!(declared_reach, "commons");
            assert_eq!(author, "agent-bob");
        }
    }

    #[test]
    fn content_publish_event_uses_blob_cid_when_present() {
        let mut input = base_input("lesson", "{}");
        input.blob_cid = Some("bafkreiabc123".to_string());
        let event = build_content_publish_event_with_author(&input, "agent-carol");

        if let RelationalImpactEvent::ContentPublish { content_cid, .. } = event {
            assert_eq!(
                content_cid, "bafkreiabc123",
                "blob_cid must be preferred over id when present"
            );
        } else {
            panic!("expected ContentPublish event");
        }
    }

    #[test]
    fn experience_moment_without_related_story_falls_back_to_id() {
        let meta = experience_moment_metadata(None); // no relatedExperienceStory
        let input = base_input("experience-moment", &meta);
        let event = build_attestation_write_event_with_author(&input, "agent-dave");

        if let RelationalImpactEvent::AttestationWrite { subject_hash, .. } = event {
            assert_eq!(
                subject_hash, "test-id-001",
                "subject_hash must fall back to input.id when relatedExperienceStory is absent"
            );
        } else {
            panic!("expected AttestationWrite event");
        }
    }

    // ── gate_check_with_event: seam tests (no agent_info required) ────────────
    //
    // These tests prove that the gate → ExternResult mapping works correctly
    // for Allow / Decline / Escalate without invoking the full gate_check_for_content
    // (which would call agent_info() and require a conductor).

    #[test]
    fn gate_check_with_event_allow_returns_ok() {
        // DevContext: check_blocking always returns Allow in the absence of overrides.
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "bafkrei-test".to_string(),
            declared_reach: "commons".to_string(),
            author: "agent-test".to_string(),
        };
        let result = gate_check_with_event(event);
        assert!(
            result.is_ok(),
            "DevContext Allow must map to ExternResult::Ok(())"
        );
    }

    #[test]
    fn gate_check_with_event_decline_returns_wasm_error() {
        // Inject a Decline via gate_client_zome override — check_blocking will
        // consume it and return Decline before any HDK call is made.
        //
        // Types are re-exported flat from gate_types via `pub use gate_types::*`
        // in gate_client_zome — no ::types:: sub-module.
        use gate_client_zome::{
            ConstitutionalReasoningSummary, DeclineGrounds, GateDecision, GateStatus, Phase,
        };
        let decline = GateDecision {
            status: GateStatus::Decline {
                grounds: DeclineGrounds {
                    category: "phase7-e2e-probe".to_string(),
                    summary: "injected decline for Phase 7 E2E test".to_string(),
                    principle_refs: vec![],
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: vec![],
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        gate_client_zome::__test_set_decision_override(Some(decline));

        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "bafkrei-decline-test".to_string(),
            declared_reach: "commons".to_string(),
            author: "agent-test".to_string(),
        };
        let result = gate_check_with_event(event);
        assert!(
            result.is_err(),
            "Decline must map to ExternResult::Err(WasmError)"
        );
        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.contains("Gate declined"),
            "error message must carry 'Gate declined'; got: {err_str}"
        );
        assert!(
            err_str.contains("injected decline"),
            "error message must carry the grounds summary; got: {err_str}"
        );
    }

    #[test]
    fn gate_check_with_event_escalate_returns_wasm_error() {
        use gate_client_zome::{
            ConstitutionalReasoningSummary, EscalationTarget, GateDecision, GateStatus, Phase,
            Severity,
        };

        let escalate = GateDecision {
            status: GateStatus::Escalate {
                target: EscalationTarget::AppSteward {
                    steward_id: "steward-phase7".to_string(),
                },
                severity: Severity::High,
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: vec![],
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        gate_client_zome::__test_set_decision_override(Some(escalate));

        let event = RelationalImpactEvent::AttestationWrite {
            subject_hash: "epr:story:test".to_string(),
            claim_kind: EXPERIENCE_MOMENT_CLAIM_KIND.to_string(),
            issuer: "agent-test".to_string(),
        };
        let result = gate_check_with_event(event);
        assert!(
            result.is_err(),
            "Escalate must map to ExternResult::Err(WasmError)"
        );
        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.contains("escalated"),
            "error message must carry 'escalated'; got: {err_str}"
        );
    }
}
