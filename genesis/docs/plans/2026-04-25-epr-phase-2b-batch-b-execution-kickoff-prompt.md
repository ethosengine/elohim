Kick off **EPR Phase 2B — Batch B: Projector & read-model reconciliation**. Batch A landed end-to-end on `feature/epr-phase-2b-batch-a` and merges to `dev`. Batch B builds on that foundation.

Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to drive task-by-task.

## What landed in Batch A (preceding work)

| Layer | Capability |
|---|---|
| Schema | `agent-peer-binding`, `device-archetype`, `identity-handshake`, `dna-signal-stream` (Category A/C classifications) |
| imagodei DNA | `AgentPeerBinding` integrity entry + validators; `create_agent_peer_binding` coordinator with signer-match gate; `ImagodeiSignal::AgentPeerBindingCreated` |
| elohim-storage projections | `peer_identity_bindings` table + `HolochainBackedPeerIdentityMap`; `pubkey_timeline_cache`; `verified_at` + Ed25519 verify; eager revocation sweep |
| elohim-storage controller | `ReconcileController` with `on_key_rotation`, `on_key_revocation`, `on_agent_peer_binding`, `on_revocation_attestation` dispatch |
| elohim-storage signal stream | `HolochainAppSignalStream` (real `holochain_client::AppWebsocket` connection); `DnaSignal` enum; M4 + A.13 signal translation |
| libp2p | `/elohim/identity/handshake/1.0.0` request-response (Category C, signed binding exchange on connection-established); `elohim/identity/binding` gossipsub topic with structural verify + DB upsert |
| Integration test | `epr_2b_batch_a_full_loop_rotation_then_revocation_clears_verified_at` — full controller loop end-to-end with real DB |

**Decision #1** (libp2p identity resolution) and **Decision #2** (verify caching under rotation) are both fully closed end-to-end. Batch B addresses Decisions #3 and #4.

## Carryover items from Batch A (not blockers; track in Batch B)

These were flagged in the Batch A final review and deliberately deferred:

1. **`handle_recovery_v2_signal` orphaned parallel implementation** at `elohim/elohim-storage/src/signals.rs:968` — direct-write projection logic that duplicates what the controller's handlers will do. Currently only called from tests. Either gate behind `#[cfg(test)]` or remove as Batch B's controller handlers cover its variants. Watch for drift.

2. **`on_key_rotation` is still a stub** at `elohim/elohim-storage/src/reconcile/controller.rs:225` — the A.12 integration test pre-warms the pubkey timeline cache to compensate. Wire the real handler in Batch B (insert/update pubkey timeline on rotation signal).

3. **`on_revocation_attestation` is still a stub** at `controller.rs:399`. Wire it to upsert `revocation_votes` projection. Note: this also unlocks the `derive_compromise_at` projection-lookup branch in `holochain_app_signal.rs` — currently always falls back to `effective_at`, conservatively over-estimating the tainted window.

4. **`device_archetype` not persisted in `peer_identity_bindings`** — gossip payload carries it; receive arm logs the deferral. Add column in a Batch B migration if the Stage 2 lookup needs it.

5. **Gossip-received binding bypasses `ReconcileController`** — `p2p/mod.rs:2711-2789` writes directly to `peer_identity_bindings` from the swarm event loop. Stage 1 limitation. Batch B should route this through a controller-facing channel to honour P1 fully.

6. **`observed_kinds` accumulator unbounded** — `controller.rs:94` is `pub` and grows without bound. Gate behind `#[cfg(test)]` or replace with bounded counter before long-running production sessions.

7. **AppWebsocket reconnect-on-disconnect** — `run_loop` exits cleanly when the conductor disconnects; the spawned task in `main.rs` does not retry. `resume_from` returns `NotImplemented`. Batch B should add reconnect with cursor-based replay.

8. **`/health` endpoint observability** — does not expose imagodei conductor connection status or controller running state. Add `reconcileController: { connected: bool, signalsDelivered: u64 }` field at `info` detail.

9. **`agent_cid` schema description vs Stage 1 substitution** — schemas for key-rotation/key-revocation now describe the substitution explicitly (pubkey/human_id at Stage 1, real CID at Stage 2). Batch B should populate the actual Human-entry-derived CID once that derivation exists.

10. **A.13 self-sovereign carve-out** — `create_agent_peer_binding` allows unregistered `agent_cid` (binding without a corresponding Agent EPR). Stage 1 acceptable for elohim-node peers. Stage 2 should require the Agent EPR exists.

11. **`AgentPeerBindingCreated` emit asymmetry** — emitted from coordinator function rather than `post_commit` (other ImagodeiSignal variants). Stage 2 should evaluate consistency.

## Batch B tasks

Read each task in the plan: `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` §Batch B (lines 735-1090).

| Task | One-liner |
|---|---|
| B.1 | Projector mapping schema extension (`pillar-projection.schema.json`, optional `projections` field on app-manifest) |
| B.2 | Shefa manifest declares first projection (`EconomicEvent → economic_events`) |
| B.3 | Projector skeleton + cursor table (manifest-driven projector controller alongside `ReconcileController`) |
| B.4 | Projector — `EconomicEvent` round-trip (DHT entry → manifest mapping → SQLite row) |
| B.5 | Invariants I1–I9 enforcement (idempotency, manifest-authority, unmapped transparency) |
| B.6 | Invariants I3, I4, I5 (causal ordering, revocation propagation, verified-state consistency) |
| B.7 | `EprHead` production-path refactor |
| B.8 | Projector signal emission + reconciliation-lag metric |

## Required reading before starting

1. Plan §Batch B (lines 735-1090) — task-by-task spec.
2. Spec sections referenced by the plan: Decisions #3 (projector) and #4 (EprHead) from the EPR Phase 2B design doc at `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`.
3. Batch A's `ReconcileController` at `elohim/elohim-storage/src/reconcile/controller.rs` — Batch B's projector lives alongside, not inside.
4. Existing manifest-driven patterns at `elohim/sdk/domains/lamad/manifest.json` and `elohim/sdk/domains/shefa/manifest.json`.
5. EPR atom federation tests at `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` — extend with projector tests.

## Build commands (per CLAUDE.md)

```bash
# elohim-storage build + tests
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration

# Schema validation (after B.1 schema add)
pnpm run schema:test && pnpm run schema:validate && pnpm run schema:codegen:ts

# Manifest codegen (after B.1 + B.2)
pnpm run lamad:codegen
```

## Branch & worktree

Create a fresh worktree off `dev` (after Batch A merges):

```bash
git worktree add /projects/elohim/.claude/worktrees/epr-phase-2b-batch-b -b feature/epr-phase-2b-batch-b origin/dev
```

If Batch A has not merged yet, branch from `feature/epr-phase-2b-batch-a` instead and rebase later.

## Done definition (Batch B close)

- [ ] All 8 tasks from §Batch B have landing commits on this branch
- [ ] B.7's `EprHead` refactor passes existing federation integration tests
- [ ] Projector round-trip integration test (B.4) passes end-to-end
- [ ] Invariants I1–I9 each have a test pinning the contract (B.5, B.6)
- [ ] Reconciliation-lag metric exposed (B.8) — observable via `/health` or `/metrics`
- [ ] Pre-push hooks pass; clippy + fmt clean
- [ ] Commit message + push; branch ready for merge to `dev` via orchestrator DAG
- [ ] Next-batch kickoff (Batch C — Resilience & federation) drafted

## Memory pins worth re-checking on start

- `project_principle_p1_reconciliation_controller` — projector is the second integration controller alongside ReconcileController; both consume DHT-truth, emit operational state.
- `project_three_layer_truth_model` — projector lives at libp2p/operational layer (writes SQLite from DHT-source-of-truth manifest decisions).
- `project_epr2b_recovery_m4_convergence` — Batch A merged M4; Batch B builds on the merged base.
- `feedback_schema_first_ioc` — `pillar-projection.schema.json` is the source of truth for projection mappings.
- `feedback_swarm_composition_fresh_tree_build` — verify cargo build in fresh tree pre-commit.

## How to handle blockers

Same protocol as Batch A:
1. Plan-bug → fix inline + note in commit
2. Hidden dependency → coordinate (likely with main session operator if cross-batch)
3. Design-decision gap → BLOCK and surface to operator; do not invent

Do NOT bypass tests, lint, or husky.

---

Go.
