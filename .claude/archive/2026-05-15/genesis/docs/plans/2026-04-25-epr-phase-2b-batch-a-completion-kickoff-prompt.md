Continue **EPR Phase 2B — Batch A** with the remaining four tasks (A.9, A.10, A.11, A.12). The first eight tasks (A.1–A.8) landed in the prior session as 8 atomic commits on `feature/epr-phase-2b-batch-a`. This session wires the libp2p surface (handshake + gossip), connects the real DNA signal stream from the imagodei conductor, and proves the full loop works end-to-end.

Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to drive task-by-task.

## What landed (prior session)

| SHA | Task | One-liner |
|---|---|---|
| `f62cb1fa` | A.1 | `agent-peer-binding` + `device-archetype` schemas (Category A — DHT) |
| `835b7ec1` | A.2 | `AgentPeerBinding` entry type + validators in imagodei integrity zome (HDI-compatible — no `get_links`) |
| `bf3b50f2` | A.3 | DNA signal stream contract + stub subscriber (`DnaSignal` enum, `InMemoryDnaSignalStream`, `ChannelSignalStream`) |
| `17d36f00` | A.4 | `ReconcileController` skeleton with stub handlers |
| `a4547b17` | A.5 | `peer_identity_bindings` table + `HolochainBackedPeerIdentityMap` |
| `a3883bbd` | A.6 | Pubkey timeline LRU (`PubkeyTimeline`, `PubkeyTimelineCache`) |
| `81536acd` | A.7 | `verified_at` + `verified_signer_fingerprint` columns + resolver-backed Ed25519 verify in ingest |
| `a5a9fd45` | A.8 | Eager revocation sweep + cache invalidation in `ReconcileController::on_key_revocation` |

**Decision #2 (verify caching under rotation) is now fully closed end-to-end** — signal stream, controller routing, pubkey timeline cache, ingest-time verify, revocation sweep, eager cache invalidation. Decision #1 (identity resolution) has the storage half landed; the libp2p wire surface (A.9, A.10) remains.

## Known deviations to address at batch close

1. **A.5 StubIdentityMap retention.** The plan's DoD said "zero callsites remain at p2p/mod.rs:913,988". The implementation kept `StubIdentityMap` as a transient fallback in `P2PNode::new()` (line ~919); production code paths replace it via `with_db_pool()` to `HolochainBackedPeerIdentityMap`. Test harness still uses `StubIdentityMap`. Resolve at batch close: either accept the lazy-init pattern as documented design, or refactor `P2PNode` to require a pool at construction. Decide based on how A.11 wires startup.

2. **Coordinator `create_agent_peer_binding` function.** The spec §3.1 line 141 lists this under "New DNA work (Batch A)" but no specific task added it. A.2's sweettest is `#[ignore]`-pending its existence. Decide whether A.11 (real signal stream) needs the coordinator function to exist, or whether a separate task A.13 handles it. The existing M4 coordinator emits the recovery signals; the binding-creation coordinator is the equivalent for `AgentPeerBinding`.

## Remaining tasks

Read each task in the plan: `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`.

### Task A.9 — libp2p identity handshake

**Plan §Task A.9 lines 608-641.** New request-response protocol `/elohim/identity/handshake/1.0.0`. Schema: `elohim/sdk/schemas/v1/p2p/identity-handshake.schema.json`. On connection-established, peers exchange signed `AgentPeerBinding`. Receiver verifies + inserts into `peer_identity_bindings` with `source='handshake'`.

Dependencies: A.2's `AgentPeerBinding` shape, A.5's table, the existing libp2p behaviour at `elohim/elohim-storage/src/p2p/behaviour.rs`.

Integration test extension: `epr_atom_federation_integration::two_peer_swarm()` — assert peer B's identity map resolves peer A after connect.

### Task A.10 — Gossipsub `elohim/identity/binding` topic

**Plan §Task A.10 lines 643-667.** Subscribe + publish on the topic. Add subscription next to existing `recovery.invitation` (commit `e9e2806a` foundation). When local agent creates a binding, publish it; when receiving, verify + insert with `source='gossip'`. The `ReconcileController::on_agent_peer_binding` handler (currently a stub from A.4) becomes the publish trigger.

Integration test: peer A rotates → emits new binding → peer B (connected) receives via gossip → cache updates → stale verifications invalidated.

### Task A.11 — Real `HolochainAppSignalStream`

**Plan §Task A.11 lines 669-711.** Replaces A.3's stubs with a real `holochain_client_rust::AppWebsocket` subscription to the imagodei cell. Translates `Signal::App(_)` events to A.3's `DnaSignal::*` shape:

- imagodei `RecoveryV2Signal::KeyRotation` → `DnaSignal::KeyRotation`
- imagodei `RecoveryV2Signal::KeyRevocationEffective` → `DnaSignal::KeyRevocation`
- imagodei `RecoveryV2Signal::KeyRevocationRequested` + `RevocationVoteSubmitted` → `DnaSignal::RevocationAttestation`
- (future) imagodei `agent_peer_binding_created` signal → `DnaSignal::AgentPeerBinding` — needs a coordinator function to emit it (see deviation #2 above)

Wires into storage startup: at binary init, construct `HolochainAppSignalStream` alongside the db pool, pass to `ReconcileController::new_with_storage`, spawn `controller.run_loop()` as a tokio task.

**Convergence with Recovery M4 (already on dev as commit `dbe684ad`):** M4 emits the three revocation variants but does not yet emit `KeyRotation` (verify what the existing imagodei coordinator emits — search for `emit_signal` in the imagodei coordinator zome). If `KeyRotation` is not yet emitted, A.11 either (a) emits from a coordinator extension, or (b) defers `DnaSignal::KeyRotation` translation until M5 / a future recovery milestone adds it.

For `KeyRevocationSignal::compromise_at`: the M4 `KeyRevocationEffective` signal does NOT carry compromise_at directly. A.11's translator must derive it from the `KeyRevocation` DHT entry (one extra coordinator round-trip) — flagged in A.3's commit message as a deferred A.11 concern.

### Task A.12 — End-to-end integration test

**Plan §Task A.12 lines 713-731.** Two peers each running imagodei conductor + storage controller. EPR signed with K1 → K2 rotation → K1 revocation with `compromise_at` before EPR signing → assert peer B's `verified_at` is cleared on the affected EPR. Tests the full controller loop with real DNA signals, real storage tables, real libp2p. This is the proof the entire Batch A delivery works end-to-end.

## Build commands (per CLAUDE.md)

```bash
# elohim-storage build + tests
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration

# Schema validation (after A.9 schema add)
pnpm run schema:test && pnpm run schema:validate && pnpm run schema:codegen:ts

# Sweettest (after coordinator function lands per deviation #2)
cd elohim/holochain && nix develop -c hc dna pack dna/imagodei
cd elohim/holochain/tests/sweettest && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test imagodei_peer_binding
```

## Branch & worktree

Already on `feature/epr-phase-2b-batch-a` at `/projects/elohim/.claude/worktrees/epr-phase-2b-batch-a`. Continue from current HEAD (`a5a9fd45`).

## Done definition (Batch A close)

- [ ] All 12 tasks from the plan have landing commits on this branch
- [ ] Both deviations above are resolved (StubIdentityMap, coordinator function)
- [ ] Sweettest `imagodei_peer_binding` runs (un-ignored after coordinator lands)
- [ ] Federation integration tests pass (no regressions)
- [ ] Batch A integration test (A.12) passes end-to-end
- [ ] Commit message + push: branch ready for merge to `dev` via orchestrator DAG
- [ ] Next-batch kickoff: Batch B execution prompt drafted at `genesis/docs/plans/YYYY-MM-DD-epr-phase-2b-batch-b-execution-kickoff-prompt.md`

## Memory pins worth re-checking on start

- `project_principle_p1_reconciliation_controller`
- `project_epr2b_recovery_m4_convergence`
- `project_three_layer_truth_model`
- `feedback_schema_first_ioc`
- `project_hdi_no_get_links_in_validators`

## How to handle blockers

Same protocol as the original Batch A kickoff:
1. Plan-bug → fix inline + note in commit
2. Hidden dependency → coordinate (likely with M4 if DNA-side; with main session operator if storage-side)
3. Design-decision gap → BLOCK and surface to operator; do not invent

Do NOT bypass tests, lint, or husky. The hook cooldown for p2p-design-gate is set through 2026-05-01 — no further silencing needed.

---

Go.
