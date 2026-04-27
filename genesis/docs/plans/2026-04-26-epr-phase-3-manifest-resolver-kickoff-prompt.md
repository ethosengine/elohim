# EPR Phase 3 — Manifest-EPR Resolver: Kickoff Prompt

**Date:** 2026-04-26
**Status:** Scaffold — operator fills in TBD sections before executing
**Precondition:** Phase 2B fully landed on `dev` (all four batches A/B/C/D green)

---

## Framing

Phase 3 builds the Manifest-EPR resolver on top of the Phase 2B substrate. Where Phase 2B notarized + projected + fanned out raw atoms, Phase 3 makes manifests first-class EPRs that `schemaRef`-walk into other EPRs. The `epr_atoms` table is now populated with real signed envelopes; the projector already maps kinds to pillar tables via a provisional hardcoded registry. Phase 3 replaces that provisional registry with a `ManifestRegistry` — pillar manifests become Manifest-EPRs, their projection declarations become Manifest-EPR payload fields, and `schemaRef` CID resolution enables recursive graph walks from manifest to atom to manifest. The cold-fetch gap (local miss → swarm resolve) closes here too, completing the full content-addressed delivery path.

Phase 3 is the prerequisite for Phase 4's GraphQL surface (hREA / VF-GraphQL lands there, per memory pin `project_epr_substrate_vs_vf_graphql`). Do not introduce VF semantics in Phase 3 code.

---

## Required Reading

1. **Phase 2B design spec** — `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`
   - §6.2 "Graph surface (Phase 3–7) consumption" — the explicit handoff table
   - §5 projector invariants I1–I9 (Phase 3 must not break them)
   - §3 Decisions #1–#8 (context for why the substrate looks as it does)

2. **Phase 2B execution plan** — `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`
   - §Batch B tasks (projector + manifest authority) — Phase 3 extends this work

3. **Phase 2B addendum** — `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md`
   - §"What follows Batch D" → §"TODO(phase-3) markers planted by Z.1"

4. **Current TODO(phase-3) inventory** (grep before starting):
   ```bash
   grep -rn "TODO(phase-3)\|FIXME(phase-3)" elohim/elohim-storage/src/
   ```
   Expected hits at Z.1 close-out:
   - `src/api/epr.rs` — 5x `local_libp2p_peer_id` dedup wiring (fetch ×3, verify ×1, list ×1)
   - `src/services/epr_store.rs` — cold-fetch via `swarm_handle.resolve_epr(cid)`
   - `src/services/epr_kind.rs` — `pillar_for_kind_provisional` FIXME

5. **Memory pins**:
   - `project_principle_p1_reconciliation_controller` — DHT is manifest, libp2p is controller; still applies; ManifestRegistry is a reconciliation surface, not a config file
   - `project_three_layer_truth_model` — Manifest-EPRs are DHT-notarized (Category A); ManifestRegistry is a local projection (Category C)
   - `project_dht_vs_libp2p_scoping` — schemaRef walks are libp2p/local ops; signing stays off the hot path
   - `project_epr_substrate_vs_vf_graphql` — Phase 3 is substrate; VF-GraphQL is Phase 4
   - `feedback_schema_first_ioc` — write JSON schemas before any new wire types

---

## Task Table (Scaffold — TBD detail)

| Task | One-liner | Priority | Status |
|------|-----------|----------|--------|
| P3.1 | **ManifestRegistry** — replace `pillar_for_kind_provisional` with a registry that loads pillar manifests from DHT-notarized Manifest-EPRs; local projection (Category C) | P0 | TBD |
| P3.2 | **Manifest-as-EPR** — define `kind: Manifest` EPR variant; integrity zome entry type in `elohim` DNA; projector mapping `Manifest → manifests` table | P0 | TBD |
| P3.3 | **schemaRef resolver** — walks `schemaRef` CID field from an EPR atom to its Manifest-EPR; recursive up to configurable depth; terminates on cycle or missing atom | P0 | TBD |
| P3.4 | **Cold-fetch via swarm** — implement `swarm_handle.resolve_epr(cid)` on local miss in `FederatedEprStore::fetch`; see `epr_store.rs` TODO(phase-3) for spec | P0 | TBD |
| P3.5 | **Manifest write-through source-of-truth** — replace `WriteThroughState::from_manifest(HashMap::new())` stub with real manifest loader; pillar manifests drive layer-1 defaults | P1 | TBD |
| P3.6 | **Dedup wiring on read routes** — wire `ctx.local_libp2p_peer_id` into fetch/verify/list routes now that dedup LRU is live; 5x TODO(phase-3) in `api/epr.rs` | P1 | TBD |
| P3.7 | **Integration test extension** — cold-fetch via swarm (Peer A has atom, Peer B resolves on miss); schemaRef walk resolves Manifest-EPR and projects to `manifests` table | P1 | TBD |

> Operator: review task boundaries before executing. P3.1–P3.4 are the core spine. P3.5–P3.6 are carryover cleanup. P3.7 is done-signal verification.

---

## Build Commands

```bash
# elohim-storage build + tests
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --features p2p
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --features p2p
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration --features p2p

# Format + clippy
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests --features p2p -- -D warnings

# Schema gates (from repo root)
pnpm run schema:test
pnpm run schema:validate
```

---

## Branch & Worktree

```bash
git fetch origin dev
git worktree add /projects/elohim/.claude/worktrees/epr-phase-3 -b feature/epr-phase-3-manifest-resolver origin/dev
```

---

## Done Definition (TBD — operator fills in)

- [ ] `pillar_for_kind_provisional` replaced by `ManifestRegistry`; all existing projector tests still pass
- [ ] `kind: Manifest` EPR variant defined with DNA entry type; projector maps it to `manifests` table
- [ ] `schemaRef` resolver walks CID chains; unit tests cover depth limit + cycle detection
- [ ] `FederatedEprStore::fetch` cold-miss triggers `swarm_handle.resolve_epr(cid)`; integration test verifies cross-peer resolution
- [ ] `WriteThroughState` loaded from real manifest defaults; layer-1 no longer `HashMap::new()`
- [ ] All 5 `TODO(phase-3)` dedup wiring sites in `api/epr.rs` resolved
- [ ] `epr_atom_federation_integration` extended with cold-fetch + schemaRef walk scenarios
- [ ] Pre-push hooks pass; clippy + fmt clean; `--features p2p` builds clean
- [ ] TBD: operator adds any additional acceptance criteria here

---

## How to Handle Blockers

Same protocol as Phase 2B batches:

1. **Plan-bug** → fix inline + note in commit message
2. **Hidden dependency** (e.g., ManifestRegistry needs a DHT query API not yet exposed) → surface to operator; do not invent the contract; stub at the seam and mark `TODO(phase-3-blocked)`
3. **Design-decision gap** (e.g., cycle depth limit not specified) → apply a reasonable default (e.g., depth ≤ 8), note it inline, flag in commit message
4. **Wire format change** → schema first (`feedback_schema_first_ioc`); update JSON schema before touching Rust structs
5. **libp2p surprises** → check `Cargo.lock` for exact version before reading docs; codebase is on libp2p 0.53

Do NOT bypass tests, lint, or husky. The full `--features p2p` integration tests must be green before push.

---

## Why Phase 3 Starts Here

Phase 2B's substrate hinge is complete:

- DHT truth layer: `AgentPeerBinding` entry type, real identity verification, DNA signal stream
- Projector: single elohim-storage projector with manifest-declared pillar mapping, real signed envelopes in `epr_atoms`
- Producer side: `/api/v1/signal/emit` + 4-layer write-through flag + integrity-always-on exception
- Discovery + fanout: tiered routing by reach, Kad providers, gossipsub topics, dedup LRU, direct-notify for integrity events

What is missing is the *semantic* layer: pillar manifests as first-class EPRs, `schemaRef` walks connecting atoms to their schemas, and the ability to cold-fetch any atom a peer doesn't have locally. Phase 3 closes those gaps. Phase 4's GraphQL subgraph resolvers depend on all three being present — they need manifest-typed EPRs to know which VF vocabulary applies, `schemaRef` walks to traverse the graph, and cold-fetch to handle the open-world case where a resolver asks for an atom it hasn't seen yet.

---

*Operator: fill in TBD sections, then hand to a fresh subagent with `superpowers:subagent-driven-development`.*
