# EPR Phase 3 — Manifest-EPR Resolver: Kickoff Prompt

**Date:** 2026-04-26 (refreshed 2026-04-30 post-brainstorm)
**Status:** Refined; ready for execution
**Precondition:** Phase 2B fully landed on `dev` (all four batches A/B/C/D green)

---

## Framing

Phase 3 builds the Manifest-EPR resolver on top of the Phase 2B substrate. Where Phase 2B notarized + projected + fanned out raw atoms, Phase 3 makes manifests first-class EPRs that `schemaRef`-walk into other EPRs. The `epr_atoms` table is now populated with real signed envelopes; the projector already maps kinds to pillar tables via a provisional hardcoded registry. Phase 3 replaces that provisional registry with a `ManifestRegistry` — pillar manifests become Manifest-EPRs, their projection declarations become Manifest-EPR payload fields, and `schemaRef` CID resolution enables recursive graph walks from manifest to atom to manifest. The cold-fetch gap (local miss → swarm resolve) closes here too, completing the full content-addressed delivery path.

**The 2026-04-30 brainstorm refinement:** every Phase 3 task now ships
**standing-aware code paths** — function signatures take a `Standing`
argument, control flow respects standing-gradient policy, but the actual
signal returns `Standing::Unknown` until Phase 3.5 lights up the gradient
substrate. The wiring is in place; the gradient *architecture* is testable;
live signal flow follows in Phase 3.5. This means Phase 3 can land scope-
bounded while still encoding the architectural commitments from the
brainstorm.

Phase 3 is the prerequisite for Phase 3.5 (new substrate: `FeedbackSignal`
EPR, `AttentionTending` EPR, constitutional floor, edge-local back-prop)
and Phase 4 (hREA / VF-GraphQL surface, per memory pin
`project_epr_substrate_vs_vf_graphql`). Do not introduce VF semantics or
gradient-signal computation in Phase 3 code.

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

5. **Brainstorm artifact (architectural foundation)** — `genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md`
   - §1 Sacred-attention thesis (the moral spine — every decision must answer to it)
   - §2 Nine foundational principles (load-bearing claims)
   - §3 Trust→compute gradient (per-layer table)
   - §8 Phase 3 compute-burden refinements (the table this kickoff embeds in §Task Table below)
   - §9 Phase 3.5 proposal (new substrate; what lights up Phase 3's standing-aware code paths)
   - Appendix A Phase 2B §3.7/§7 O2 reconciliation (already applied to Phase 2B spec)
   - Appendix B aunt-and-rage-bait worked example (target integration scenario for P3.5.10)

6. **Memory pins**:
   - `project_principle_p1_reconciliation_controller` — DHT is manifest, libp2p is controller; still applies; ManifestRegistry is a reconciliation surface, not a config file
   - `project_three_layer_truth_model` — Manifest-EPRs are DHT-notarized (Category A); ManifestRegistry is a local projection (Category C)
   - `project_dht_vs_libp2p_scoping` — schemaRef walks are libp2p/local ops; signing stays off the hot path
   - `project_epr_substrate_vs_vf_graphql` — Phase 3 is substrate; VF-GraphQL is Phase 4
   - `feedback_schema_first_ioc` — write JSON schemas before any new wire types
   - `project_trust_as_efficiency_signal` — compute-economic frame; cost asymmetry is architectural
   - `project_reach_earned_at_authoring` — author-side floor (already in Phase 2B Batch D.4); Phase 3 builds gradient on top
   - `project_first_class_graph_pattern` — substrate is a graph; standing/reach/provenance are graph properties
   - `project_hdi_no_get_links_in_validators` — HDI integrity validators cannot use get_links; constraint for P3.2 DNA work

---

## Architectural foundation (from brainstorm)

Phase 3 implements the manifest-EPR resolver under nine load-bearing
commitments captured in the brainstorm artifact (§2). Brief summary so the
executing subagent has the right architectural lens:

1. **Trust as efficiency signal** — compute-economic; cost asymmetry distributed at every edge
2. **Standing on agents, reach on content** — distinct properties; graph-derived views, not stored scores
3. **Power coupled to responsibility, Dunbar-by-design** — trust-bubble boundaries are humane features
4. **Constitutional revealability of provenance** — private by default; governance-recoverable
5. **Paced reconciliation accountable to stewarded compute** — P1 controller pattern at peer scale
6. **Carrot before stick** — author-time elohim conversation is primary; aggregate sensemaking is safety net (substrate must support cheap author-side queries)
7. **Substrate-thin / manifest-medium / agent-thick-at-scale** — bullish architecture; agent layer absorbs nuance load
8. **Constitutional floors** — standing-immune (5 classes) + tending-immune (5 classes); never erodable by any gradient
9. **Honesty at onboarding** — virtue commitments declared up-front; standing accountability is discernment-gated, not compute-gated

Phase 3 does not implement these primitives directly. It implements the
manifest-EPR resolver with **standing-aware code paths** that respect the
gradient's architecture but return `Standing::Unknown` placeholder until
Phase 3.5 substrate lights up the signal. Floor protections, however, are
honored from day one — they are non-negotiable, mishpat-DNA-notarized,
present in code (not behind a placeholder).

---

## Task Table (refined post-brainstorm — ready for execution)

| Task | One-liner | Compute-burden constraint | Priority |
|------|-----------|---------------------------|----------|
| P3.1 | **ManifestRegistry** — replace `pillar_for_kind_provisional` with a registry that loads pillar manifests from DHT-notarized Manifest-EPRs; local projection (Category C) | High-trust manifests (mishpat-constitutional, qahal-collective) cached eagerly; experimental/unverified manifests lazy-loaded; refresh schedule modulated by manifest's own standing (placeholder hook); **author-side lookups are fast-path** for compose-time elohim tender conversation | P0 |
| P3.2 | **Manifest-as-EPR** — define `kind: Manifest` EPR variant; integrity zome entry type in `elohim` DNA; projector mapping `Manifest → manifests` table | Constitutional category — **full per-message validation, never amortized**; eager projection into local `manifests` table (Category C); HDI-validation must be deterministic per `project_hdi_no_get_links_in_validators` (no get_links in integrity zome) | P0 |
| P3.3 | **schemaRef resolver** — walks `schemaRef` CID field from an EPR atom to its Manifest-EPR; recursive up to configurable depth; terminates on cycle or missing atom | Depth limit shorter on low-standing chains (3-5 hops vs 8 high-standing) — placeholder until Phase 3.5; cache-first (locally-cached → no walk); walk highest-standing peer provider first; **floor: protocol-load-bearing schemaRef (DNA-notarized manifest types) cannot be standing-gated — always resolvable at full depth** | P0 |
| P3.4 | **Cold-fetch via swarm** — implement `swarm_handle.resolve_epr(cid)` on local miss in `FederatedEprStore::fetch` | High-standing providers queried first parallel; low-standing serial-fallback shorter timeout — placeholder until Phase 3.5; **floor: low-standing fallback mandatory if no high-standing provider** (CID-targeted lookup unconditional per §2.8 standing-immune floor); fast-path: cached + high-trust signer → no swarm; slow-path: cold + low-trust → swarm with rigor | P0 |
| P3.5 | **Manifest write-through source-of-truth** — replace `WriteThroughState::from_manifest(HashMap::new())` stub with real manifest loader; pillar manifests drive layer-1 defaults | Per-manifest absorption rate (paced reconciliation per principle P1); manifest mutations themselves are **constitutional — full validation, never amortized**; reconciliation-lag observable via metrics | P1 |
| P3.6 | **Dedup wiring on read routes** — wire `ctx.local_libp2p_peer_id` into fetch/verify/list routes now that dedup LRU is live; 5x TODO(phase-3) in `api/epr.rs` | Dedup window length **standing-aware** (placeholder until Phase 3.5): shorter for high-standing peers (less re-injection risk); longer for low-standing; PeerId threading already prepped at Z.1 | P1 |
| P3.7 | **Integration test extension** — cold-fetch via swarm; schemaRef walk; **floor-protection assertions** | Add: floor-protection scenarios (CID lookup unconditional even at lowest standing; constitutional schemaRef always at full depth; child reach unconditional within family) and persona-stress-test scenarios (child / refugee / activist) — uses `Standing::Unknown` placeholder + manual standing fixtures for testability | P1 |

> P3.1–P3.4 are the core spine. P3.5–P3.6 are carryover cleanup with
> compute-burden refinements. P3.7 is done-signal verification including
> floor-protection regression scenarios.

---

## Phase 3.5 sequencing — what comes next

Phase 3 ships scope-bounded with standing-aware code paths and
`Standing::Unknown` placeholder. **Phase 3.5** (separate plan; brainstorm
to follow) introduces the new substrate that lights up the gradient:

| Task | One-liner | Priority |
|---|---|---|
| P3.5.1 | `FeedbackSignal` EPR kind + libp2p protocol extension (squelch/correction/retraction/quarantine) | P0 |
| P3.5.2 | Edge-local predecessor map + sealed-against-self record format (interim 2-of-2 mishpat + imagodei) | P0 |
| P3.5.3 | Hop-by-hop back-prop walk impl (Primitive 2 from brainstorm §5) | P0 |
| P3.5.4 | Gossip-flood notification (Primitive 3) layered on top | P1 |
| P3.5.5 | `AttentionTending` EPR kind + tending TTL/lifecycle | P0 |
| P3.5.6 | Collective-wisdom aggregator (anonymous, k-anonymous) | P1 |
| P3.5.7 | Constitutional floor manifest schema (mishpat-DNA-notarized; both standing-immune + tending-immune classes) | P0 |
| P3.5.8 | Bootstrap default standing-policy manifest | P0 |
| P3.5.9 | Author-side compose-time query API (cheap; for elohim tender conversation) | P0 |
| P3.5.10 | Integration test: aunt-and-rage-bait scenario end-to-end (Appendix B of brainstorm) | P0 |

Phase 3.5 lights up `Standing::Unknown` placeholders into live signals;
Phase 4 (VF-GraphQL surface) follows after Phase 3.5 lands.

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

## Done Definition

- [ ] `pillar_for_kind_provisional` replaced by `ManifestRegistry`; all existing projector tests still pass
- [ ] `kind: Manifest` EPR variant defined with DNA entry type; HDI-deterministic validation (no `get_links`); projector maps to `manifests` table; full per-message verification (never amortized)
- [ ] `schemaRef` resolver walks CID chains; unit tests cover depth limit + cycle detection; protocol-load-bearing schemaRef walks at full depth regardless of `Standing` arg (floor protection)
- [ ] `FederatedEprStore::fetch` cold-miss triggers `swarm_handle.resolve_epr(cid)`; integration test verifies cross-peer resolution; CID-targeted fetch returns content even at `Standing::Unknown` (floor protection)
- [ ] `WriteThroughState` loaded from real manifest defaults; layer-1 no longer `HashMap::new()`; absorption rate per-manifest declared (paced reconciliation)
- [ ] All 5 `TODO(phase-3)` dedup wiring sites in `api/epr.rs` resolved with standing-aware window length (placeholder)
- [ ] `epr_atom_federation_integration` extended with cold-fetch + schemaRef walk scenarios + **floor-protection scenarios** (CID lookup unconditional; constitutional schemaRef at full depth; child reach unconditional within family) + **persona-stress-test scenarios**
- [ ] Standing-aware function signatures: every gradient-relevant function takes a `Standing` argument; control flow respects gradient policy; signal returns `Standing::Unknown` placeholder (Phase 3.5 lights up)
- [ ] No `FeedbackSignal` or `AttentionTending` EPR work — those are Phase 3.5
- [ ] No VF-GraphQL semantics — that is Phase 4
- [ ] Pre-push hooks pass; clippy + fmt clean; `--features p2p` builds clean

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

What is missing is the *semantic* layer: pillar manifests as first-class EPRs, `schemaRef` walks connecting atoms to their schemas, and the ability to cold-fetch any atom a peer doesn't have locally. Phase 3 closes those gaps. Phase 3.5's gradient substrate lights up the standing-aware code paths Phase 3 wires. Phase 4's GraphQL subgraph resolvers depend on all three being present — they need manifest-typed EPRs to know which VF vocabulary applies, `schemaRef` walks to traverse the graph, and cold-fetch to handle the open-world case where a resolver asks for an atom it hasn't seen yet.

---

*Hand to a fresh subagent with `superpowers:subagent-driven-development`. The brainstorm artifact at `genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md` is the architectural source of truth; this kickoff is the execution-shaped projection.*
