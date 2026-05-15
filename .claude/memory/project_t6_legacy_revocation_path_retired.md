---
name: project-t6-legacy-revocation-path-retired
description: EPR Foundation Completion T6 — legacy RecoveryV2Signal::KeyRevocationEffective consumer path fully retired; T18 envelope is sole consumer.
metadata:
  type: project
---

# T6 — legacy revocation consumer path retired

**Date:** 2026-05-15
**Sprint:** `2026-05-15-epr-foundation-completion.md` Task 6
**Verdict:** LANDED (consumer-side deletion only; producer-side handover to M4)

## What changed

User directive 2026-05-15: *"we shouldn't be worried about backwards compatibility."* T6 was reframed mid-sprint from "upgrade `derive_compromise_at` from Stage-1 stub to Stage-2 projection lookup" into a pure deletion of the legacy `RecoveryV2Signal::KeyRevocationEffective` consumer path.

Most of the deletion landed on dev independently (via the operator's parallel work) and arrived in this branch via merge `d14b0e3ae`. EPR's closing commit `e3f03e14c` removed the last orphaned import (`KeyRevocationSignal`) and finalized the cleanup.

### Retired pieces (consumer side, elohim-storage)
- `derive_compromise_at` function at `reconcile/holochain_app_signal.rs:289-297` — REMOVED. Zero callers.
- `translate_recovery_v2`'s legacy `KeyRevocationEffective` arm — FOLDED into the existing "not consumed by reconcile controller" catch-all at `reconcile/holochain_app_signal.rs:222-225`. Returns `None`.
- `handle_recovery_v2_signal`'s legacy arm at `signals.rs:1200-1206` — REPLACED with `RecoveryV2Signal::KeyRevocationEffective { .. } => Ok(())` (explicit no-op with a comment noting M4 will delete producer emissions as a follow-up).
- Stage-1 fallback comment that T5 left at `signals.rs:1212-1213` — REMOVED (the call it referenced disappeared with the no-op).
- Module doc-comment at `reconcile/holochain_app_signal.rs:27-35` — rewritten to record the retirement and point readers at `signals::handle_imagodei_dna_signal` as the canonical revocation consumer.
- `KeyRevocationSignal` import at `reconcile/holochain_app_signal.rs:82` — REMOVED (orphaned by the deletions above).

### Canonical replacement
`DnaSignal::KeyRevocation(KeyRevocationEnvelope)` consumed by `signals::handle_imagodei_dna_signal` at `signals.rs:1425`. The envelope carries `metadata.compromise_at` directly (no derivation needed), verifies the issuer ed25519 signature before projecting, then calls `key_revocations::set_effective` + `sweep_dependent_caches_on_revocation` + emits `ImagodeiReconciledEvent::RevocationObserved`. End-to-end semantics preserved; one path now, not two.

## What M4 still owes (producer-side follow-up)

The legacy variant `RecoveryV2Signal::KeyRevocationEffective` still exists on the M4 producer side and is emitted by three imagodei coordinator sites:
- `create_self_revocation`
- `submit_revocation_vote` (threshold-reached branch)
- `submit_specialist_revocation`

These emit to dead air on the EPR consumer side (no-op arm absorbs them). Harmless but wasteful. M4 deletes the variant + emissions + the `#[deprecated]` annotation as part of T19+ tail work or a follow-up cleanup PR. **Coordination contract:** EPR will NOT remove the no-op arm until M4 confirms producer-side deletion lands; this preserves match exhaustiveness across the transition.

## Why this is correct architecture

The T18 envelope (`DnaSignal::KeyRevocation`) is the EPR-shape signal — a cross-stack provenance record with `subjectCid`, signed by the authoring elohim, schema-validatable offline, carrying `compromise_at` directly in metadata. The legacy `RecoveryV2Signal::KeyRevocationEffective` was a substrate-shaped notification that required server-side projection-lookup to derive the load-bearing `compromise_at`. The envelope is the protocol's native primitive for this kind of attestation; the legacy was scaffolding. Replacing scaffolding with the primitive simplifies the consumer surface and aligns the wire with the [[project-epr2b-recovery-m4-convergence]] EPR-envelope-as-signal architecture decision.

## Related

- [[project-epr2b-recovery-m4-convergence]] — EPR-envelope signal architecture decision that made this retirement possible
- [[project-w2-agent-peer-binding-deferred]] — sibling deferral for the W2D AgentPeerBinding arm (still gated on iroh Phase 12 consumer wiring)
