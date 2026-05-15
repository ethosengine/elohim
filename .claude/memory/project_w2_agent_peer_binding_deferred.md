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

- iroh Phase 12 plan (`genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md`) — **59 unticked, 0 ticked.** Plan checkboxes are unstarted.
- `peer_transport_manifest` SQLite table — **EXISTS on dev** (migration `2026-05-10-120000_peer_transport_manifest`, full Phase-12 schema with `agent_cid` PK + dual transport profiles + per-transport supported planes + `capability_level`). Diesel schema entry is at `db/diesel_schema.rs:1536`. So the substrate is partially landed despite the plan-tracking debt — a plan-tracking-debt shape similar to W2A/W2B that this sprint cleaned up.
- **BUT consumer wiring is incomplete.** Code references in `view_fed_service.rs:29`, `epr_atom_service.rs:15`, `views.rs:7796`, `auth_backends.rs:29-135`, `multi_stack_fixture.rs:27` describe the target behavior using hedge language like "when a resolver is wired" (auth_backends.rs:53) — meaning the table exists but the four iroh adapters that should consume it (`epr_atom_backend`, `view_fed_backend`, `auth_backends::trust`, `auth_backends::identity_handshake`) have NOT yet been re-pointed at the new manifest.
- Existing Phase-10 `CallerIdentity` infrastructure IS still in place (`p2p_iroh/epr_atom_backend.rs`, `epr_atom_service.rs`, `p2p/identity_map.rs`, `p2p/mod.rs`) and is still what the runtime currently uses.

Per kickoff prompt's D5 ("Phase 12 caller-identity dependency — runtime check"): the `AgentPeerBinding` IntegrityNotify consumer arm requires Phase 12's new manifest to populate caller identity correctly. Wiring the arm against the legacy Phase-10 surface — or against the bare manifest table without its adapter consumers — would mean re-wiring it after Phase 12 graduates the consumer side. Single-wire is cheaper than double-wire.

**Corrected gate criterion:** the deferral is lifted when the four iroh adapters consume `peer_transport_manifest` (not when the migration lands — that's already done). Look for `auth_backends.rs` to stop saying "when a resolver is wired" and start actually wiring it.

## What to do when Phase 12 lands

1. Verify Phase 12 closure: `grep -c "^- \[x\]"` on the Phase 12 plan should equal `grep -c "^- \[ \]"` count from when it was unstarted (currently 59).
2. Verify `peer_transport_manifest` is consumed by the four iroh adapters — search `auth_backends.rs` for "when a resolver is wired" hedge-comments; those should be gone. Search `epr_atom_backend.rs`, `view_fed_backend.rs` for live reads of the manifest's `capability_level` + `*_supports_json` columns. (Note: the migration itself already exists on dev at `2026-05-10-120000_peer_transport_manifest`, so don't gate on table-existence.)
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
