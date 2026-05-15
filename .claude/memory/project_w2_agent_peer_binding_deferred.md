---
name: project-w2-agent-peer-binding-deferred
description: EPR W2 AgentPeerBinding IntegrityNotify arm deferred — iroh Phase 12 caller-identity not yet live. Re-evaluate after Phase 12 lands.
metadata:
  type: project
---

# W2 AgentPeerBinding arm — deferred to follow-up Wave-2 mini-plan

**Date:** 2026-05-15
**Sprint:** `2026-05-15-epr-foundation-completion.md` Task 8 (D5 runtime check)
**Verdict:** DEFER

## Gate state at decision time

- iroh Phase 12 plan (`genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md`) — **59 unticked, 0 ticked.** Plan is unstarted.
- `peer_transport_manifest` SQLite table — does not exist on dev.
- Existing Phase-10 `CallerIdentity` infrastructure IS present (`p2p_iroh/epr_atom_backend.rs`, `epr_atom_service.rs`, `p2p/identity_map.rs`, `p2p/mod.rs`) but it predates the Phase-12 manifest graduation.

Per kickoff prompt's D5 ("Phase 12 caller-identity dependency — runtime check"): the `AgentPeerBinding` IntegrityNotify consumer arm requires Phase 12's new manifest to populate caller identity correctly. Wiring the arm against the legacy Phase-10 surface would mean re-wiring it after Phase 12 lands. Single-wire is cheaper than double-wire.

## What to do when Phase 12 lands

1. Verify Phase 12 closure: `grep -c "^- \[x\]"` on the Phase 12 plan should equal `grep -c "^- \[ \]"` count from when it was unstarted (currently 59).
2. Verify `peer_transport_manifest` table exists in `elohim/elohim-storage/migrations/` and is referenced in `elohim/elohim-storage/src/p2p/peer_map.rs` (the rewrite target per Phase 12 Architecture line).
3. Mirror EPR Task 7 (`RevocationAttestation` arm) for `AgentPeerBinding`:
   - Create `elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs` matching the published `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` field-for-field (read the schema first; do not invent fields).
   - Insert `"AgentPeerBinding"` arm in `epr_atom_service.rs::handle_integrity_notify`, after the `"RevocationAttestation"` arm (which landed at commit `68df467cd` + polish `5cfbddafc` on `worktree-epr-foundation-completion`), before the `other_kind` catch-all.
   - Dedup key shape: `format!("AgentPeerBinding:{}", msg.binding_action_hash)` (single-key suffices — binding_action_hash is per-DHT-entry unique).
   - Two tests: `integrity_notify_agent_peer_binding_acks_received_true` + `_dedup_returns_duplicate_reason`.
   - Same delivery-optimistic discipline — no projection-table writes from the arm; canonical writes happen via the reconcile controller's `DnaSignal::AgentPeerBinding` path (`reconcile/holochain_app_signal.rs:translate_imagodei`).
4. Tick the W2 deferred-arm box in `2026-05-15-epr-foundation-completion.md` Task 8 and close the loop.

## Why this is not a sprint blocker

The EPR foundation-completion sprint's acceptance is met when (per kickoff §"Acceptance for the sprint as a whole" + Wave 6 closing condition):
- Cross-stack integration test round-trips `KeyRotation` + `RevocationAttestation` signals end-to-end. ← does NOT require AgentPeerBinding.
- 17 `@wip` scenarios lift. ← does NOT require AgentPeerBinding.
- orchestrator dev SUCCESS or UNSTABLE-not-regressed for the 1-week soak. ← does NOT require AgentPeerBinding.

The `AgentPeerBinding` arm is forward-looking infrastructure for Phase 12 graduation. Per kickoff D5: "treat AgentPeerBinding as a Wave-2 follow-up, not a sprint blocker."

## Backlog handoff

Pointer for the future implementer: this memory entry + Task 8 of `2026-05-15-epr-foundation-completion.md` + the published schema at `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` together specify the work. Estimated effort post-Phase-12: ~1 hour (mirrors T7 pattern exactly).

Related: [[project-epr-m4-coordination-2026-05-15]] (if written), [[project-iroh-phase11-all-backends-wired]] (Phase 12 prereq context).
