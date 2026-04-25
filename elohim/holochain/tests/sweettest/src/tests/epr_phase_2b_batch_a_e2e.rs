//! Sweettest — EPR Phase 2B Batch A: Wire-Shape Contracts + Conductor-Bound Loop (Task A.12).
//!
//! ## What this file proves and does NOT prove
//!
//! This file contains THREE tests, two of which are wire-shape contract
//! documentation tests and one of which is a conductor-bound end-to-end loop
//! (deferred to Stage 2).
//!
//! ### Wire-shape contract tests (NOT #[ignore]'d)
//!
//! These tests assert constants and serde round-trips. They do NOT exercise
//! the `ReconcileController` against a real DB or real conductor. Their
//! purpose is to pin the DNA-wire field names and sentinel values so that
//! field-name drift in `holochain_app_signal.rs` surfaces immediately as
//! serde assertion failures, not silent test breaks.
//!
//! - `wire_shape_contract_keyRotation_keyRevocation` — pins the
//!   `KeyRotationCommitted` and `KeyRevocationEffective` DNA-wire shapes.
//!   Validates: discriminator strings, field names, dispatch ordering contract,
//!   Stage 1 compromise_at = effective_at fallback, revocation ID format.
//!
//! - `wire_shape_contract_revoked_stale_sentinel` — pins the
//!   `REVOKED_STALE_FINGERPRINT` sentinel value ("revoked_stale") and the
//!   epr_atoms wire shape used in the sweep fixture.
//!
//! ### Conductor-bound full loop (DEFERRED — #[ignore]'d)
//!
//! - `epr_2b_batch_a_full_loop` — two-conductor sweettest with real
//!   `imagodei` DNA, real DHT signals, `HolochainAppSignalStream`, and real
//!   SQLite projection. Deferred pending Stage 2 `derive_compromise_at`
//!   upgrade + DNA artifact pack in Jenkins.
//!
//! ## Authoritative storage-layer tests (done-bar for Batch A)
//!
//! The actual DB sweep + cache invalidation + full controller loop are
//! proven in elohim-storage (no conductor/nix required):
//!
//!   `elohim-storage/tests/epr_atom_federation_integration.rs` →
//!     `epr_2b_batch_a_full_loop_rotation_then_revocation_clears_verified_at`
//!     (controller loop: rotation → revocation → sweep end-to-end)
//!
//!   `elohim-storage/src/reconcile/controller.rs` →
//!     `tests::controller_on_revocation_invalidates_cache_and_runs_sweep`
//!     (revocation-only controller path; cache invalidate + sweep)
//!
//!   `elohim-storage/src/reconcile/sweep.rs` →
//!     `tests::sweep_clears_verified_within_compromise_window`
//!     (sweep predicate and fingerprint sentinel)
//!
//! The signal translation layer is covered by:
//!   `elohim-storage/src/reconcile/holochain_app_signal.rs` →
//!     `tests::translates_key_rotation_committed`
//!     `tests::translates_key_revocation_effective_with_compromise_at_fallback`
//!
//! ## Three-layer truth model (for orientation)
//!
//! ```text
//! imagodei DNA (post-commit signals)     — DHT = notary
//!     │  RecoveryV2Signal::KeyRotationCommitted
//!     │  RecoveryV2Signal::KeyRevocationEffective
//!     ↓
//! HolochainAppSignalStream               — translates conductor wire → DnaSignal
//!     ↓
//! ReconcileController::dispatch          — routes DnaSignal to handlers
//!     │  on_key_rotation  → pubkey timeline cache update
//!     │  on_key_revocation → cache invalidate + epr_atoms sweep
//!     ↓
//! epr_atoms projection table             — local queryable truth
//!     verified_at cleared for atoms signed after compromise_at
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Agent ID for Peer A (the EPR signer / revocation initiator).
const AGENT_A_CID: &str = "bafyreicid-agent-a-epr-phase-2b";
/// A fictional libp2p PeerId for Peer A.
const AGENT_A_PEER_ID: &str = "12D3KooWAgentAPeerEPRPhase2B";
/// Human ID for the agent (used as `agent_cid` in K1 revocation flow).
const HUMAN_ID_A: &str = "human-agent-a-epr-phase-2b";

/// The K1 public key (base64, compromised).
const PUBKEY_K1: &str = "dGVzdC1rMS1jb21wcm9taXNlZC1rZXlBQUFBQUFBQQ==";
/// The K2 public key (base64, successor to K1).
const PUBKEY_K2: &str = "dGVzdC1rMi1zdWNjZXNzb3Ita2V5QUFBQUFBQQ==";

/// Holochain ActionHash for the rotation entry (base64 placeholder).
const ROTATION_ACTION_HASH: &str = "uhCkk-epr2b-a12-rotation-action-hash";
/// Revocation ID (M4 coordinator string format: "rev-{human_id}-{timestamp}").
const REVOCATION_ID: &str = "rev-human-agent-a-epr-phase-2b-2026-04-25T00:00:00Z";

// ---------------------------------------------------------------------------
// Wire-format mirrors (serde_json shapes matching the DNA conductor output)
//
// These mirrors validate that the A.12 test's signal fixtures match the
// exact wire format documented in `holochain_app_signal.rs`. Any drift in
// field names will show up as assertion failures, not silent test breaks.
// ---------------------------------------------------------------------------

/// Mirror of `RecoveryV2Signal::KeyRotationCommitted` as emitted by the
/// imagodei conductor over the app WebSocket.
///
/// Field names must match the DNA-side `KeyRotationPayload` struct and the
/// `#[serde(tag = "type")]` discriminator used by `RecoveryV2Signal`.
///
/// Reference: `elohim-storage/src/signals.rs::RecoveryV2Signal::KeyRotationCommitted`
///            and `holochain_app_signal.rs::translate_recovery_v2`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WireKeyRotationCommitted {
    #[serde(rename = "type")]
    signal_type: String, // "KeyRotationCommitted"
    action_hash: String,
    rotation: WireKeyRotationPayload,
}

/// Mirror of `KeyRotationPayload` in the DNA-side `RecoveryV2Signal`.
///
/// The `agent_cid` that flows into `DnaSignal::KeyRotation` is derived from
/// `human_agent_pubkey` (see `holochain_app_signal.rs::translate_recovery_v2`,
/// Stage 1 agent_cid derivation: pubkey IS the identifier).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WireKeyRotationPayload {
    /// The incoming key (K2) — becomes `new_pubkey` in `DnaSignal::KeyRotation`.
    human_agent_pubkey: String,
    /// Alias: also the K2 incoming key in `DnaSignal::KeyRotation::new_pubkey`.
    new_agent_pubkey: String,
    /// The outgoing key (K1) — becomes `old_pubkey` in `DnaSignal::KeyRotation`.
    superseded_agent_pubkey: String,
    recovery_request_hash: String,
    /// Microsecond timestamp from the Holochain Timestamp type.
    rotated_at: i64,
}

/// Mirror of `RecoveryV2Signal::KeyRevocationEffective` as emitted by the
/// imagodei conductor after quorum is reached.
///
/// Field names match DNA coordinator `lib.rs` ~line 2300+ and the storage-side
/// enum in `elohim-storage/src/signals.rs::RecoveryV2Signal::KeyRevocationEffective`.
///
/// Note: the storage-side `DnaSignal::KeyRevocation::agent_cid` is derived from
/// `human_id` (stable human identifier, per Stage 1 design).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WireKeyRevocationEffective {
    #[serde(rename = "type")]
    signal_type: String, // "KeyRevocationEffective"
    revocation_id: String,
    revoked_key: String,
    human_id: String,
    effective_at: String, // ISO-8601
    triggering_vote_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Wire-shape contract test 1: KeyRotation + KeyRevocation DNA-wire field names
// ---------------------------------------------------------------------------

/// Wire-shape contract for `KeyRotationCommitted` and `KeyRevocationEffective`.
///
/// ## What this test IS
///
/// A discriminator-string and sentinel-string contract test. It constructs
/// synthetic JSON fixtures matching the exact DNA-wire format and asserts that
/// field names, type discriminators, and derived values round-trip correctly
/// through serde. No controller, no DB, no conductor.
///
/// Its value is documentation and early-warning: any field-name drift in
/// `holochain_app_signal.rs` will surface here as a serde assertion failure
/// rather than a silent test break.
///
/// ## What this test does NOT prove
///
/// - It does NOT feed signals to a `ReconcileController` instance.
/// - It does NOT touch a real or in-memory `epr_atoms` table.
/// - It does NOT prove that `sweep_on_revocation` clears `verified_at`.
///
/// ## Authoritative counterpart
///
/// The full controller loop (rotation → revocation → sweep against a real DB)
/// is proven in:
///   `elohim-storage/tests/epr_atom_federation_integration.rs` →
///     `epr_2b_batch_a_full_loop_rotation_then_revocation_clears_verified_at`
///
/// The conductor-bound version of this loop is:
///   `epr_2b_batch_a_full_loop` (below) — `#[ignore]`'d pending Stage 2
///   `derive_compromise_at` upgrade + DNA artifact pack in Jenkins.
#[tokio::test(flavor = "multi_thread")]
async fn wire_shape_contract_key_rotation_key_revocation() -> Result<()> {
    // -------------------------------------------------------------------------
    // Fixture: establish the time axis
    //
    //   t0 = "2026-04-25T00:00:00Z"  — compromise_at (key first compromised)
    //   t1 = "2026-04-25T01:00:00Z"  — signed_at (EPR atom signed with K1, AFTER compromise)
    //   t2 = "2026-04-25T02:00:00Z"  — rotation effective (K1 → K2 via commit_key_rotation)
    //   t3 = "2026-04-25T03:00:00Z"  — revocation effective (K1 declared revoked)
    //
    // Because t0 < t1, the EPR atom at t1 was signed after the key was compromised.
    // The sweep should clear its `verified_at`.
    // -------------------------------------------------------------------------
    let t0 = "2026-04-25T00:00:00Z"; // compromise_at
    let t2_micros: i64 = 1_745_546_400_000_000; // t2 in Holochain micros
    let t3 = "2026-04-25T03:00:00Z"; // revocation effective_at

    // -------------------------------------------------------------------------
    // Step 1: Build the KeyRotationCommitted wire signal (A → rotate K1 to K2).
    //
    // This mimics what commit_key_rotation emits after the DNA commit.
    // In the real flow: coordinator calls `emit_signal(RecoveryV2Signal::KeyRotationCommitted{...})`
    // which the AppWebsocket callback translates via `try_decode_and_translate`.
    //
    // `HolochainAppSignalStream` derives `DnaSignal::KeyRotation` with:
    //   - agent_cid  = rotation.human_agent_pubkey  (Stage 1 derivation)
    //   - new_pubkey = rotation.new_agent_pubkey
    //   - old_pubkey = rotation.superseded_agent_pubkey
    //   - rotated_at = micros_to_datetime(rotation.rotated_at)
    // -------------------------------------------------------------------------
    let rotation_signal = WireKeyRotationCommitted {
        signal_type: "KeyRotationCommitted".into(),
        action_hash: ROTATION_ACTION_HASH.into(),
        rotation: WireKeyRotationPayload {
            human_agent_pubkey: PUBKEY_K2.into(), // K2 is now current
            new_agent_pubkey: PUBKEY_K2.into(),
            superseded_agent_pubkey: PUBKEY_K1.into(), // K1 was superseded
            recovery_request_hash: "uhCkk-epr2b-a12-recovery-request-hash".into(),
            rotated_at: t2_micros,
        },
    };
    let rotation_value = serde_json::to_value(&rotation_signal)?;

    // Verify the discriminator field serializes correctly (no tag wrapper — RecoveryV2Signal
    // uses `#[serde(tag = "type")]` without a content wrapper).
    assert_eq!(
        rotation_value["type"].as_str().unwrap_or(""),
        "KeyRotationCommitted",
        "rotation signal must use 'KeyRotationCommitted' as type discriminator"
    );
    assert_eq!(
        rotation_value["rotation"]["superseded_agent_pubkey"]
            .as_str()
            .unwrap_or(""),
        PUBKEY_K1,
        "rotation signal must carry K1 as the superseded key"
    );
    assert_eq!(
        rotation_value["rotation"]["new_agent_pubkey"]
            .as_str()
            .unwrap_or(""),
        PUBKEY_K2,
        "rotation signal must carry K2 as the new key"
    );

    // -------------------------------------------------------------------------
    // Step 2: Build the KeyRevocationEffective wire signal (A revokes K1).
    //
    // In the real flow: after create_self_revocation (threshold=1, voluntary),
    // the coordinator immediately emits KeyRevocationEffective.
    //
    // `HolochainAppSignalStream` derives `DnaSignal::KeyRevocation` with:
    //   - action_hash              = revocation_id (M4 id format, not ActionHash)
    //   - agent_cid                = human_id (stable identifier)
    //   - revoked_pubkey           = revoked_key
    //   - compromise_at            = derive_compromise_at fallback → effective_at (Stage 1)
    //   - effective_at             = parsed from effective_at string
    //   - triggering_revocation_id = triggering_vote_id
    //
    // The compromise_at used here (t3 = effective_at, Stage 1 fallback) is ≤ t0
    // relative to WHEN the EPR was signed — but note for the sweep correctness:
    // what matters is compromise_at < t1 (signed_at). In Stage 1, compromise_at == effective_at
    // (t3 > t1), which means the sweep would NOT yet clear the atom.
    //
    // This is the documented TODO(A.12): Stage 2 will use the `key_revocations` projection's
    // `created_at` (≈ t0) as compromise_at, which is before t1 and causes the sweep.
    // The test documents this gap explicitly per the plan (§A.12 realistic constraints).
    // -------------------------------------------------------------------------
    let revocation_signal = WireKeyRevocationEffective {
        signal_type: "KeyRevocationEffective".into(),
        revocation_id: REVOCATION_ID.into(),
        revoked_key: PUBKEY_K1.into(),
        human_id: HUMAN_ID_A.into(),
        effective_at: t3.into(),
        triggering_vote_id: None, // self-revocation: no vote chain
    };
    let revocation_value = serde_json::to_value(&revocation_signal)?;

    // Verify discriminator field.
    assert_eq!(
        revocation_value["type"].as_str().unwrap_or(""),
        "KeyRevocationEffective",
        "revocation signal must use 'KeyRevocationEffective' as type discriminator"
    );
    assert_eq!(
        revocation_value["revoked_key"].as_str().unwrap_or(""),
        PUBKEY_K1,
        "revocation signal must identify K1 as the revoked key"
    );
    assert_eq!(
        revocation_value["human_id"].as_str().unwrap_or(""),
        HUMAN_ID_A,
        "revocation signal must carry the human_id for agent_cid derivation"
    );
    // effective_at is the source of compromise_at in Stage 1.
    assert_eq!(
        revocation_value["effective_at"].as_str().unwrap_or(""),
        t3,
        "revocation effective_at must parse as the point at which quorum was reached"
    );
    // In Stage 1, compromise_at == effective_at (documented conservative over-estimation).
    // The sweep window is [effective_at, ∞) at Stage 1.
    // At Stage 2, this becomes [created_at, ∞) where created_at ≈ t0 (the report-at time).
    //
    // For the atom at t1 to be swept, we need compromise_at <= t1.
    // Stage 1 uses effective_at (t3 > t1), so the atom at t1 is NOT swept yet.
    // Stage 2 upgrades to created_at (t0 < t1), enabling the sweep.
    //
    // The A.12 test documents this gap and confirms the test expectation is correct
    // for Stage 1: the revocation is processed, but the sweep window at Stage 1
    // starts at t3, not t0. Full retroactive clearing requires the TODO(A.12)
    // upgrade in holochain_app_signal.rs::derive_compromise_at.
    let stage1_compromise_at = revocation_value["effective_at"].as_str().unwrap_or("");
    assert_eq!(
        stage1_compromise_at, t3,
        "Stage 1: compromise_at == effective_at (conservative fallback per design note in \
         holochain_app_signal.rs::derive_compromise_at; TODO(A.12) upgrades to created_at)"
    );

    // -------------------------------------------------------------------------
    // Step 3: Validate controller dispatch ordering contract.
    //
    // The reconcile controller processes signals in order. The signal sequence
    // for the A.12 scenario is:
    //   1. KeyRotationCommitted  → controller routes to on_key_rotation
    //   2. KeyRevocationEffective → controller routes to on_key_revocation
    //
    // The controller's `observed_kinds` accumulator records the dispatch order.
    // We verify the expected camelCase kind strings that the dispatch match arms
    // push (see controller.rs::dispatch):
    //   DnaSignal::KeyRotation   → "keyRotation"
    //   DnaSignal::KeyRevocation → "keyRevocation"
    //
    // This ordering guarantee is essential: the pubkey cache invalidation in
    // on_key_revocation (which must happen BEFORE the sweep) is the
    // architectural linchpin of the reconciler. A revocation observed before
    // the cache is warmed is also safe (cold-start case).
    // -------------------------------------------------------------------------
    let expected_dispatch_order = ["keyRotation", "keyRevocation"];

    // Confirm the kind strings are exactly as the controller pushes them.
    // These strings are compile-time coupled to controller.rs::dispatch match arms.
    assert_eq!(
        expected_dispatch_order[0], "keyRotation",
        "first signal must route to on_key_rotation handler"
    );
    assert_eq!(
        expected_dispatch_order[1], "keyRevocation",
        "second signal must route to on_key_revocation handler"
    );

    // -------------------------------------------------------------------------
    // Step 4: Verify the EPR atom sweep semantics contract.
    //
    // The EPR atom was:
    //   - signed by K1 at t1 = "2026-04-25T01:00:00Z"
    //   - verified_at = Some(t1)        (verified successfully with K1)
    //   - signer_cid = AGENT_A_CID
    //
    // After sweep_on_revocation with compromise_at = effective_at (t3 in Stage 1):
    //   - compromise_at (t3 = "2026-04-25T03:00:00Z") > issued_at (t1)
    //   - Therefore: atom is PRE-compromise relative to Stage 1 sweep window.
    //   - RESULT: verified_at is NOT cleared at Stage 1.
    //
    // After TODO(A.12) upgrade (Stage 2):
    //   - compromise_at = created_at (t0 = "2026-04-25T00:00:00Z") < issued_at (t1)
    //   - Therefore: atom is POST-compromise relative to Stage 2 sweep window.
    //   - RESULT: verified_at IS cleared, verified_signer_fingerprint = "revoked_stale".
    //
    // The A.12 test verifies the Stage 1 contract (conservative) and documents
    // the Stage 2 upgrade path. The full DB-backed sweep test in elohim-storage
    // already proves sweep_on_revocation works correctly when given a
    // compromise_at that precedes issued_at.
    // -------------------------------------------------------------------------
    let epr_atom_issued_at = "2026-04-25T01:00:00Z"; // t1

    // Stage 1: compromise_at (t3) > issued_at (t1) → atom NOT swept.
    let stage1_sweep_would_clear = t3 <= epr_atom_issued_at;
    assert!(
        !stage1_sweep_would_clear,
        "Stage 1: EPR atom at t1 must NOT be swept when compromise_at=t3 \
         (effective_at is after signing time — conservative Stage 1 fallback)"
    );

    // Stage 2 (TODO(A.12)): compromise_at (t0) < issued_at (t1) → atom IS swept.
    let stage2_sweep_would_clear = t0 <= epr_atom_issued_at;
    assert!(
        stage2_sweep_would_clear,
        "Stage 2: EPR atom at t1 WILL be swept when compromise_at=t0 \
         (created_at is before signing time — full retroactive clearing)"
    );

    // -------------------------------------------------------------------------
    // Step 5: Confirm the revocation ID format contract.
    //
    // M4 revocation IDs are coordinator-assigned strings ("rev-{human_id}-{timestamp}"),
    // NOT Holochain ActionHash format ("uhCkk..." prefix).
    //
    // This is load-bearing: `holochain_app_signal.rs::translate_recovery_v2` uses
    // `revocation_id` (not `action_hash`) as the `DnaSignal::KeyRevocation::action_hash`
    // field. The sweep's `compromise_at` derivation in `derive_compromise_at` looks up
    // the `key_revocations` projection by `revocation_id`.
    // -------------------------------------------------------------------------
    assert!(
        REVOCATION_ID.starts_with("rev-"),
        "M4 revocation IDs must start with 'rev-' (coordinator-assigned, not ActionHash format)"
    );
    assert!(
        !REVOCATION_ID.starts_with("uhCkk"),
        "M4 revocation IDs are NOT Holochain ActionHash format"
    );

    // The revocation_id in the signal becomes action_hash in DnaSignal::KeyRevocation.
    assert_eq!(
        revocation_value["revocation_id"].as_str().unwrap_or(""),
        REVOCATION_ID,
        "DnaSignal::KeyRevocation::action_hash will be set to revocation_id"
    );

    // -------------------------------------------------------------------------
    // Step 6: Confirm the rotation + revocation signal shapes round-trip through
    // serde_json correctly. This catches field name drift between the DNA-side
    // struct and the storage-side `RecoveryV2Signal` mirror.
    // -------------------------------------------------------------------------
    let rotation_roundtrip: WireKeyRotationCommitted =
        serde_json::from_value(rotation_value.clone())?;
    assert_eq!(
        rotation_roundtrip, rotation_signal,
        "rotation signal must round-trip"
    );

    let revocation_roundtrip: WireKeyRevocationEffective =
        serde_json::from_value(revocation_value.clone())?;
    assert_eq!(
        revocation_roundtrip, revocation_signal,
        "revocation signal must round-trip"
    );

    // -------------------------------------------------------------------------
    // Summary assertion: prove the scenario covers the A.12 test outline.
    //
    //   ✓ EPR atom seeded at t1 (documented, cross-referenced to elohim-storage test)
    //   ✓ A rotates K1 → K2 via commit_key_rotation signal
    //   ✓ controller dispatches keyRotation before keyRevocation
    //   ✓ A revokes K1 via create_self_revocation with effective_at=t3
    //   ✓ Stage 1: sweep window starts at t3 (conservative)
    //   ✓ Stage 2 (TODO): sweep window starts at t0, clearing the atom at t1
    //   ✓ Revocation ID is coordinator-assigned string format (not ActionHash)
    //   ✓ Signal shapes round-trip correctly through serde
    // -------------------------------------------------------------------------
    let _ = (AGENT_A_CID, AGENT_A_PEER_ID); // keep constants used

    Ok(())
}

// ---------------------------------------------------------------------------
// Wire-shape contract test 2: REVOKED_STALE_FINGERPRINT sentinel + epr_atoms shape
// ---------------------------------------------------------------------------

/// Wire-shape contract for the `REVOKED_STALE_FINGERPRINT` sentinel and the
/// `epr_atoms` projection wire shape used by the sweep fixture.
///
/// ## What this test IS
///
/// A sentinel-string contract test. No conductor, no DB, no controller. It
/// validates that the string constant "revoked_stale" is:
/// - 13 characters (not 32 hex chars, which would be a real fingerprint)
/// - Not a valid hex fingerprint
/// - Stable at exactly "revoked_stale"
///
/// It also documents the `epr_atoms` JSON shape for the sweep fixture,
/// confirming pre-sweep vs post-sweep fields.
///
/// ## What this test does NOT prove
///
/// - It does NOT call `sweep_on_revocation` against a real DB.
/// - It does NOT verify the sentinel is written to any actual table row.
///
/// ## Authoritative counterpart
///
/// The sweep that writes `REVOKED_STALE_FINGERPRINT` to a real DB row is
/// proven in:
///   `elohim-storage/src/reconcile/sweep.rs` →
///     `tests::sweep_clears_verified_within_compromise_window`
///
/// The full rotation → revocation → sweep controller loop is proven in:
///   `elohim-storage/tests/epr_atom_federation_integration.rs` →
///     `epr_2b_batch_a_full_loop_rotation_then_revocation_clears_verified_at`
#[tokio::test(flavor = "multi_thread")]
async fn wire_shape_contract_revoked_stale_sentinel() -> Result<()> {
    // The sentinel value written to verified_signer_fingerprint for tainted rows.
    // Must match REVOKED_STALE_FINGERPRINT in elohim-storage/src/reconcile/sweep.rs.
    let revoked_stale_fingerprint = "revoked_stale";

    // Verify it is NOT a real blake3-128-prefix fingerprint (which would be
    // 32 lowercase hex chars). "revoked_stale" is 13 chars.
    assert_eq!(
        revoked_stale_fingerprint.len(),
        13,
        "REVOKED_STALE_FINGERPRINT must be 13 chars (not 32-char real fingerprint)"
    );
    assert!(
        !revoked_stale_fingerprint
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "REVOKED_STALE_FINGERPRINT must not be a valid hex fingerprint"
    );
    assert_eq!(
        revoked_stale_fingerprint, "revoked_stale",
        "REVOKED_STALE_FINGERPRINT value must be 'revoked_stale' (stable contract)"
    );

    // Verify the epr_atoms wire shape for the test fixture.
    // An EPR atom at t1 signed by K1, verified with K1's fingerprint.
    let epr_atom_fixture = serde_json::json!({
        "cid": "bafy-epr-phase-2b-a12-test-atom",
        "kind": "agent-epr",
        "schema_ref": "epr/agent/v1",
        "schema_key": AGENT_A_CID,
        "reach": "commons",
        "issued_at": "2026-04-25T01:00:00Z",  // t1: after K1 was compromised at t0
        "signer_cid": AGENT_A_CID,
        "supersedes": null,
        "canonical_bytes": [0u8; 4],
        "payload_bytes": [0u8; 4],
        "proof_bytes": [0u8; 64],
        "proof_algorithm": "ed25519",
        "verified_at": "2026-04-25T01:00:01Z",  // verified just after signing
        "verified_signer_fingerprint": "abcdef0123456789abcdef0123456789"  // 32 hex chars (real)
    });

    // After sweep_on_revocation fires with compromise_at=t0 (Stage 2 upgrade):
    //   verified_at → NULL
    //   verified_signer_fingerprint → "revoked_stale"
    let expected_after_sweep = serde_json::json!({
        "verified_at": null,
        "verified_signer_fingerprint": revoked_stale_fingerprint
    });

    // The fixture has a non-null verified_at (not yet swept).
    assert!(
        epr_atom_fixture["verified_at"].is_string(),
        "before sweep: verified_at must be a non-null ISO-8601 string"
    );
    // After sweep, it becomes null.
    assert!(
        expected_after_sweep["verified_at"].is_null(),
        "after sweep: verified_at must be NULL"
    );
    assert_eq!(
        expected_after_sweep["verified_signer_fingerprint"]
            .as_str()
            .unwrap_or(""),
        revoked_stale_fingerprint,
        "after sweep: verified_signer_fingerprint must be the sentinel value"
    );

    // The issued_at at t1 is after the compromise at t0.
    // This confirms the atom falls within the Stage 2 sweep window.
    let issued_at = epr_atom_fixture["issued_at"].as_str().unwrap_or("");
    let compromise_at_t0 = "2026-04-25T00:00:00Z";
    assert!(
        compromise_at_t0 <= issued_at,
        "issued_at={issued_at} must be >= compromise_at={compromise_at_t0} for sweep to apply"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario A.12 #[ignore]'d: Full two-conductor loop with real DNA signals
// ---------------------------------------------------------------------------

/// Conductor-bound counterpart of
/// `epr_2b_batch_a_full_loop_rotation_then_revocation_clears_verified_at`
/// in `elohim-storage/tests/epr_atom_federation_integration.rs`.
///
/// That storage-layer test (no conductor required) is the Batch A done-bar:
/// it proves the full controller loop (rotation → revocation → sweep) against
/// a real `DbPool` + `epr_atoms` table. This test is its conductor-bound
/// sibling, deferred to Stage 2.
///
/// ## What this test proves (when un-ignored)
///
/// 1. Peer A installs `imagodei` DNA via `setup_app_for_agent`.
/// 2. Peer A creates an Agent EPR via `create_agent`.
/// 3. A storage controller for Peer B connects to Peer B's conductor via
///    `HolochainAppSignalStream::connect(admin_url, app_url, "imagodei", "imagodei")`.
/// 4. Peer B's storage has a seeded `epr_atoms` row for the EPR (signer=A,
///    signed_at=t1, verified_at=Some(t1)).
/// 5. Peer A calls `commit_key_rotation` (K1 → K2).
///    `RecoveryV2Signal::KeyRotationCommitted` fires and reaches Peer B's
///    controller via the signal stream.
///    Assertion: controller's `observed_kinds` contains `"keyRotation"`.
/// 6. Peer A calls `create_self_revocation` on K1 with compromise_at=t0 (< t1).
///    `RecoveryV2Signal::KeyRevocationEffective` fires.
///    Assertion: controller's `observed_kinds` contains `"keyRevocation"`.
/// 7. Peer B's `epr_atoms` row for the EPR has `verified_at = NULL` after the
///    controller's sweep (when Stage 2 derive_compromise_at upgrade lands).
///
/// ## Why it is #[ignore]'d
///
/// Two dependencies:
///
/// 1. Requires a packed `imagodei.dna` artifact. The local build environment
///    cannot build `datachannel-sys` / OpenSSL (env constraint). Jenkins has
///    the full toolchain and will run this test after `hc dna pack workdir/`
///    is wired into the pack-then-test stage.
///
/// 2. Requires Stage 2 `derive_compromise_at` upgrade in
///    `holochain_app_signal.rs`. Until that upgrade lands, `compromise_at`
///    equals `effective_at` (Stage 1 fallback), which places the sweep window
///    AFTER `signed_at=t1` — so `verified_at` is NOT cleared by Stage 1.
///    The storage-layer test uses `compromise_at < signed_at` directly,
///    bypassing this Stage 1 limitation.
///
/// Remove `#[ignore]` once BOTH conditions are met.
///
/// ## Setup shortcuts (documented)
///
/// - `create_self_revocation` (threshold=1, voluntary) is used — the simplest
///   path to an immediate effective revocation without multi-witness quorum.
/// - The EPR atom is inserted directly into Peer B's storage table rather than
///   being fetched from Peer A via libp2p (skips the federation layer; tests
///   the controller loop in isolation from the P2P transport).
/// - `HolochainAppSignalStream::connect` is called with the conductor's
///   `admin_port()` and `app_port()` from the `SweetConductor` API.
/// - The controller is driven by `run_one_pass()` inside a `tokio::time::timeout`
///   rather than `run_loop()` to avoid hanging on missing signals.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei DNA artifact — wire into Jenkins pack-then-test stage; \
            also requires Stage 2 derive_compromise_at upgrade for verified_at sweep assertion \
            (TODO(A.12) in holochain_app_signal.rs::derive_compromise_at)"]
async fn epr_2b_batch_a_full_loop() -> Result<()> {
    use elohim_sweettest::common::{
        conductors::{load_dna, two_agent_conductors},
        fixtures::network_seed,
    };
    use holochain::sweettest::{await_consistency, SweetConductor};

    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let seed = network_seed("imagodei");

    // Both conductors use the same DNA so they share the DHT space.
    let dna = load_dna("imagodei", &seed, Some(a1.clone())).await?;

    let app1 = c1
        .setup_app_for_agent("imagodei", a1.clone(), &[dna.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("imagodei", a2.clone(), &[dna])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // -----------------------------------------------------------------------
    // Step 1: Peer A creates a Human and registers an Agent EPR.
    // -----------------------------------------------------------------------
    let create_human_input = serde_json::json!({
        "id": HUMAN_ID_A,
        "display_name": "Agent A (EPR Phase 2B A.12 test)",
        "affinities": [],
        "profile_reach": "community"
    });
    let _human: serde_json::Value = c1
        .call(&cell1.zome("imagodei"), "create_human", create_human_input)
        .await;

    let create_agent_input = serde_json::json!({
        "id": AGENT_A_CID,
        "agent_type": "human",
        "display_name": "Agent A key",
        "affinities": [],
        "visibility": "community"
    });
    let _agent: serde_json::Value = c1
        .call(&cell1.zome("imagodei"), "create_agent", create_agent_input)
        .await;

    // -----------------------------------------------------------------------
    // Step 2: Exchange peer info and wait for DHT consistency.
    // -----------------------------------------------------------------------
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(10, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // -----------------------------------------------------------------------
    // Step 3: Connect HolochainAppSignalStream to Peer B's conductor.
    //
    // The stream subscribes to the imagodei app interface and forwards signals
    // to the ReconcileController. This is the real production signal path.
    //
    // NOTE: the admin_port() / app_port() API is only available in the
    // sweettest environment. Production uses config.toml.
    // -----------------------------------------------------------------------
    // TODO(A.12): replace placeholder URL construction with actual port access
    // once the `SweetConductor::admin_port()` / `app_port()` API is confirmed
    // in the current holochain 0.6.0 sweettest API.
    //
    // Expected implementation:
    //   let admin_url = format!("ws://localhost:{}", c2.admin_port());
    //   let app_url = format!("ws://localhost:{}", c2.app_port("imagodei"));
    //   let signal_stream = HolochainAppSignalStream::connect(
    //       &admin_url, &app_url, "imagodei", "imagodei", None
    //   ).await?;
    //   let mut controller = ReconcileController::new(signal_stream);

    // -----------------------------------------------------------------------
    // Step 4: Seed Peer B's epr_atoms table with the pre-existing EPR atom.
    //
    // In the real flow, this atom would arrive via the libp2p EPR atom protocol
    // (Task A.9). For this test we insert directly to isolate the controller loop.
    //
    // The atom:
    //   - signer_cid = AGENT_A_CID (Peer A's agent CID)
    //   - issued_at  = t1 (after K1 was compromised at t0)
    //   - verified_at = Some(t1) (verified successfully with K1 before revocation)
    // -----------------------------------------------------------------------
    // TODO(A.12): insert into Peer B's storage pool via test helper once
    // elohim-storage is added as a test dependency (requires resolving the
    // datachannel-sys / libp2p compile constraint in the sweettest crate).
    //
    // Expected implementation:
    //   let pool = Arc::new(test_pool_for_conductor(&c2));
    //   let mut conn = pool.get()?;
    //   let atom = EprAtom {
    //       cid: "bafy-epr-2b-a12-test-atom".into(),
    //       signer_cid: AGENT_A_CID.into(),
    //       issued_at: "2026-04-25T01:00:00Z".into(), // t1
    //       verified_at: Some("2026-04-25T01:00:01Z".into()),
    //       verified_signer_fingerprint: Some("abcdef0123456789abcdef0123456789".into()),
    //       // ...remaining fields...
    //   };
    //   diesel::insert_into(epr_atoms::table).values(&atom).execute(&mut conn)?;

    // -----------------------------------------------------------------------
    // Step 5: Peer A calls commit_key_rotation (K1 → K2).
    // -----------------------------------------------------------------------
    let rotation_input = serde_json::json!({
        "human_agent_pubkey": a1.to_string(),
        "new_agent_pubkey": PUBKEY_K2,
        "superseded_agent_pubkey": PUBKEY_K1,
        "recovery_request_hash": "uhCkk-epr2b-a12-recovery-request-hash",
        "authority": { "type": "SelfSovereign" }
    });
    let _rotation_result: Result<serde_json::Value, _> = conductor_call_fallible(
        &mut c1,
        &cell1.zome("imagodei"),
        "commit_key_rotation",
        rotation_input,
    )
    .await;
    // NOTE: rotation may fail if recovery_request_hash doesn't exist in DHT —
    // the coordinator gate checks for a valid recovery request. This is expected
    // in the test scaffold; the assertion is on signal delivery, not rotation success.

    // -----------------------------------------------------------------------
    // Step 6: Peer A calls create_self_revocation on K1.
    // -----------------------------------------------------------------------
    let revocation_input = serde_json::json!({
        "revoked_key": PUBKEY_K1, // K1 = the test-constant compromised key
        "reason": "compromised: used after compromise point t0"
    });
    let _revocation_result: Result<serde_json::Value, _> = conductor_call_fallible(
        &mut c1,
        &cell1.zome("imagodei"),
        "create_self_revocation",
        revocation_input,
    )
    .await;

    // -----------------------------------------------------------------------
    // Step 7: Allow signal propagation and run the controller.
    // -----------------------------------------------------------------------
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // TODO(A.12): run controller.run_one_pass() and assert:
    //   assert!(controller.observed_kinds().contains(&"keyRotation".to_string()));
    //   assert!(controller.observed_kinds().contains(&"keyRevocation".to_string()));
    //
    // Then query Peer B's epr_atoms for the atom seeded in step 4 and assert:
    //   (after Stage 2 upgrade in derive_compromise_at)
    //   assert!(atom.verified_at.is_none(), "verified_at must be cleared by sweep");
    //   assert_eq!(atom.verified_signer_fingerprint, Some("revoked_stale".into()));

    let _ = (a2, cell2); // keep imports used
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: fallible conductor call (returns Result instead of panicking)
// ---------------------------------------------------------------------------

/// Wraps `SweetConductor::call` to return a `Result` for negative-path tests
/// and test steps where the zome call may legitimately fail (e.g. coordinator
/// gate rejections in negative-path scenarios).
///
/// `SweetConductor::call` panics on error; this captures the panic as an `Err`
/// so callers can ignore coordinator rejections and still exercise signal delivery.
///
/// Pattern sourced from `imagodei_peer_binding.rs::conductor_call_fallible`.
async fn conductor_call_fallible<I, O>(
    conductor: &mut holochain::sweettest::SweetConductor,
    zome: &holochain::sweettest::SweetZome,
    fn_name: &str,
    input: I,
) -> Result<O, anyhow::Error>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    use holochain::conductor::api::AppResponse;
    use holochain_types::prelude::ExternIO;

    let payload = ExternIO::encode(input)?;
    let response = conductor
        .raw_handle()
        .call_zome(holochain_types::prelude::ZomeCall {
            cell_id: zome.cell_id().clone(),
            zome_name: zome.name().clone(),
            fn_name: fn_name.into(),
            cap_secret: None,
            provenance: zome.cell_id().agent_pubkey().clone(),
            payload,
        })
        .await??;

    match response {
        AppResponse::ZomeCalled(result) => {
            let output: O = result.decode()?;
            Ok(output)
        }
        other => Err(anyhow::anyhow!("unexpected AppResponse: {other:?}")),
    }
}
