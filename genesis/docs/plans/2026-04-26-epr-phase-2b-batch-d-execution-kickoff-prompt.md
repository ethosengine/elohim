Kick off **EPR Phase 2B — Batch D: Discovery & fanout**. Batch C landed end-to-end on `feature/epr-phase-2b-batch-c` and merged to `dev` at `05a3d0f1`. Batch D is the last Phase 2B batch — it closes Decision #7 (Kad + gossipsub composition) and finishes the substrate hinge so Phase 3 (manifest-graph resolver) can kick off.

Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to drive task-by-task.

## What landed in Batch C (preceding work)

| Layer | Capability |
|---|---|
| Schema | `conductor-signing.schema.json`, `signal-intent.schema.json`; `writeThrough` block on `app-manifest.schema.json` |
| Imagodei zome | `sign_for_agent` coordinator function with signer-match gate; uses `hdk::ed25519::sign_raw` |
| elohim-storage | `ConductorSigningClient` (websocket wrapper), `POST /api/v1/signal/emit` (composes + signs + ingests), `WriteThroughState` (4-layer override stack) |
| Endpoints | `POST /api/v1/signal/emit`, `GET /api/v1/status/write-through`, `POST /admin/write-through` |
| Override layers | (1) manifest default — shefa `enabled: false`; (2) `[write_through.<pillar>]` in policy.toml; (3) `ELOHIM_WRITE_THROUGH_<PILLAR>=on/off`; (4) admin POST replaces declarative override |
| Integrity exception | `is_integrity_kind()` hardcodes KeyRotation, KeyRevocation, RevocationAttestation, AgentPeerBinding to bypass the gate |
| Angular | `SignalEmitService` (3-arm result: emitted/fallback/error); `SignalHarnessService` migrated; transparent 503 → legacy fallback |
| Tests | 53 unit + 2 integration (Rust) + 7 vitest (Angular); E2E intent → projector round-trip green |

Batch C closes Decisions #5 (signal harness migration) and #6 (write-through flag granularity). The producer side of the substrate is live; what remains is the discovery and fanout side — how an EPR atom emitted by one peer reaches the peers that should see it, and how the integrity exception guarantees revocations propagate through every channel.

## Carryover items from Batch C (not blockers; track in Batch D)

These were noted during Batch C and deliberately deferred:

1. **Manifest registry layer-1 source-of-truth** — `main.rs` constructs `WriteThroughState::from_manifest(HashMap::new())` because there is no on-disk pillar manifest loader yet. The infrastructure is in place (`ManifestRegistry::from_pillar_manifests` exists for projections); a sibling loader for `writeThrough` blocks needs to land. Phase 3 owns this when Manifest-EPRs become canonical, but a stopgap loader for Batch D would let env/admin layers compose against real manifest defaults rather than implicit-OFF. Decide in Batch D whether to add the stopgap or leave it for Phase 3.

2. **Real agent CID from Angular harness** — `SignalHarnessService` passes `AgentService.getCurrentAgentId()` straight through as `agentCid`. `prepare_signing_request` calls `Cid::from_str` on it, so a session-id string fails validation with `BadAgentCid` (HTTP 400). C.5's gate fires *before* this (returns 503 first when write-through is OFF). When an operator flips shefa ON, the harness needs a real agent CID. Either: extend `AgentService` with a `getAgentCid()` derived from the session's lair pubkey, or make `signing_client.sign()` derive the CID from `cell_id().agent_pubkey()` server-side and ignore the client field. Surface in C.3 follow-up or D.

3. **Pre-existing typecheck noise from `transaction-import.service.spec.ts` + `vite.config.ts`** — `tsc --noEmit -p tsconfig.json` (the base config) errors on vitest globals not being in scope and a vite.config typing gap. Spec config (`tsconfig.spec.json`) compiles clean. Pre-existing tech debt; not introduced by Batch C. Track separately.

4. **The 174 pillar-boundary ESLint warnings** (memory pin `project_pillar_boundary_violations_backlog`) remain warn-level. Batch C touched `lamad/services/signal-harness.service.ts` to import from `@app/shefa` — this is a *legitimate* cross-pillar import (shefa is the producer, lamad is the harness host) and the rule already allows it. Mention in case D adds boundary-related infrastructure.

5. **`compute_cid` import in tests** — `signal_emit_round_trip.rs` imports `elohim_epr::cid::compute_cid` directly. Acceptable for integration tests, but if Batch D's tests proliferate, consider re-exporting from `elohim_storage::test_util`.

6. **Husky did not run on the Batch C merge push** — the temp worktree used for the `--no-ff` merge never had `pnpm install` so the pre-push hook wasn't wired. All gates ran manually before push. If Batch D's merge follows the same pattern, either (a) `pnpm install` in the temp worktree before push, or (b) push from the feature worktree (which has hooks installed) using `git push origin HEAD:dev` after a local FF.

## Batch D tasks

Read each task in the plan: `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` §Batch D, and the spec at `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` §3.7 (Decision #7 — Kad + gossipsub composition).

| Task | One-liner |
|---|---|
| D.1 | Reach-tier routing policy in `p2p/behaviour.rs` — pure function `route_for(reach: Reach) -> RoutingPlan { kad: bool, gossip: Option<TopicScope>, direct: bool }`; integrity exception short-circuit. |
| D.2 | Kad `start_providing` on Announce for tier ≥ Community (per Decision #7 routing table). Wire into the EPR atom federation publish path. |
| D.3 | Gossipsub topic enumeration: `elohim/<pillar>/<reach>/[<collective-id>]`, `elohim/identity/binding` (already exists; verify name), `elohim/integrity/revocation` (rename of `recovery.revocation` or add alias). Topic name builder + tests. |
| D.4 | Reach-gated subscription enforcement — a peer can only subscribe to topics it's authorized for per its agent's delegations / group memberships. Hook into existing reach-enforcement in `epr_reach_enforcement.rs`. |
| D.5 | Integrity exception routing — KeyRotation, KeyRevocation, RevocationAttestation, AgentPeerBinding bypass tier and go on all three channels (Kad + gossip + direct-notify). Mirror the C.4 `is_integrity_kind()` hardcode here so integrity remains a single source of truth. |
| D.6 | Direct-notify for known-affected peers — index by `signer_cid`; consume Recovery M4's notification list contract. Out-of-band notify via libp2p request/response. |
| D.7 | Dedup LRU on receive path — bounded ~few MB by CID; on duplicate receipt no-op + emit `seen` counter for observability. |
| D.8 | Integration test (`epr_atom_federation_integration.rs` extension): Commons-reach announce discoverable by cold-start peer via Kad; Community-reach announce received by gossip subscribers only; revocation received via all three channels (Kad + gossip + direct). |

## Required reading before starting

1. Plan §Batch D — task-by-task spec.
2. Spec §3.7 (Decision #7) and §5.I3/I4/I7 (invariants the routing must preserve) at `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`.
3. Existing p2p surface: `elohim/elohim-storage/src/p2p/behaviour.rs` (combined behaviour with kademlia + gossipsub + identify + autonat + dcutr already wired); `elohim/elohim-storage/src/p2p/mod.rs` for the swarm event loop.
4. Existing topic state: `IDENTITY_BINDING_TOPIC` in `p2p/identity_binding_gossip.rs:123`; `RECOVERY_REVOCATION_TOPIC` in `p2p/mod.rs:131`. The spec's proposed topic structure renames `recovery.revocation` → `elohim/integrity/revocation` — decide rename vs alias in D.3.
5. Existing federation harness: `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` — D.8 extends this rather than starting fresh.
6. Recovery M4 notification list contract — search for `notification_list` or `affected_peers` in `imagodei` and `recovery` modules; D.6 consumes whatever M4 produces.
7. C.4's `is_integrity_kind()` at `elohim/elohim-storage/src/write_through.rs` — D.5 mirrors this.
8. Memory pin `project_three_layer_truth_model` — Batch D operates entirely in the libp2p layer (Category C, operational); DHT signing is not involved.

## Build commands (per CLAUDE.md)

```bash
# elohim-storage build + tests (note p2p feature flag)
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --features p2p
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --features p2p
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration --features p2p

# Format + clippy
cd elohim/elohim-storage && cargo fmt --check
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p -- -D warnings
```

The `--features p2p` flag matters: most p2p code is gated. Forgetting it makes the test pass trivially because the modules aren't compiled.

## Branch & worktree

Create a fresh worktree off `dev` (Batch C is already merged at `05a3d0f1`):

```bash
git fetch origin dev
git worktree add /projects/elohim/.claude/worktrees/epr-phase-2b-batch-d -b feature/epr-phase-2b-batch-d origin/dev
```

## Done definition (Batch D close)

- [ ] All 8 Batch D tasks have landing commits on this branch
- [ ] `route_for(reach)` pure function exists with tests covering every Reach + integrity bypass
- [ ] Topic name builder produces `elohim/<pillar>/<reach>/[<collective>]` matching spec; existing topics renamed/aliased per D.3 decision
- [ ] Reach-gated subscription enforcement rejects unauthorized topic subscriptions in tests
- [ ] Integrity exception verified: a `KeyRevocation` EPR appears on Kad + gossip + direct-notify channels in the integration test
- [ ] Dedup LRU prevents double-projection on duplicate receipt; observability counter increments
- [ ] `epr_atom_federation_integration.rs` extension passes for: Commons via Kad cold-start, Community via gossip subscribers only, revocation via all three
- [ ] Pre-push hooks pass; clippy + fmt clean; `--features p2p` builds clean
- [ ] Commit + push; branch ready for `--no-ff` merge to `dev`
- [ ] Phase 2B Definition-of-Done checklist (spec §8) reviewed; all items checked
- [ ] **Phase 3 kickoff prompt drafted** at `genesis/docs/plans/<date>-epr-phase-3-manifest-resolver-kickoff-prompt.md` per spec O8 ("at 2B completion, write a Phase 3 kickoff prompt")
- [ ] Addendum `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` §"What follows Batch D" updated with pointer to Phase 2B spec + plan (per spec §8 done item)

## Memory pins worth re-checking on start

- `project_principle_p1_reconciliation_controller` — Batch D is libp2p layer (Category C, operational); DHT remains the manifest, fanout is the controller observing it.
- `project_three_layer_truth_model` — Decision #7 is *entirely* about the libp2p data-ops layer; doorway is not a participant.
- `project_dht_vs_libp2p_scoping` — keep DHT signing out of fanout decisions; reach + integrity-status drive routing.
- `project_epr2b_recovery_m4_convergence` — D.6 consumes M4's notification list. Coordinate if M4's surface is still in flux.
- `feedback_schema_first_ioc` — if D introduces wire types (notification list, dedup metric), schema first.
- `project_doorway_manifest_driven_routes` — no doorway changes expected; if you find yourself editing doorway, you've drifted.
- `project_epr_substrate_vs_vf_graphql` — Batch D is substrate-only; VF-GraphQL is Phase 4 (R&O #4). Do not introduce VF semantics into routing decisions.

## How to handle blockers

Same protocol as Batch C:
1. Plan-bug → fix inline + note in commit
2. Hidden dependency (e.g., M4 notification list shape mismatch) → coordinate with main session operator; do not invent the contract
3. Design-decision gap (e.g., dedup LRU sizing not specified) → note inline default + flag in commit; surface to operator if it changes wire format
4. libp2p version surprises → check `Cargo.lock` for the exact crate version before reading docs; the codebase is on libp2p 0.53 (per CLAUDE.md), which has API differences from 0.54+

Do NOT bypass tests, lint, or husky. The full E2E (`--features p2p` integration tests) must be green before push.

## Why this batch closes Phase 2B

After Batch D, Phase 2B's Definition of Done (spec §8) is met:

- ✅ A → identity & verify (`AgentPeerBinding`, verify cache, signal stream consumer)
- ✅ B → projector & EprHead (column mapping, manifest authority, EprHead refactor)
- ✅ C → producer & write-through (signal-emit, 4-layer flag, integrity exception)
- ⏭️ D → discovery & fanout (this batch)

Phase 2B's outputs become Phase 3's inputs (spec §6.2):
- Manifest-declared projection mappings → Phase 3 Manifest-EPR resolver
- `epr_atoms` populated → Phase 3 `schemaRef` walks manifests as EPRs
- Projector outputs → Phase 4 GraphQL subgraph resolvers
- Tiered routing + Kad providers (this batch) → Phase 4 federated resolver gets cold-discovery + hot-subscription for free

The Phase 3 kickoff prompt is itself a Batch D deliverable (per spec §8 + O8).

---

Go.
