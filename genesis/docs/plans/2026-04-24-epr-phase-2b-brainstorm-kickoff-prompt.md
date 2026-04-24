I want to open a brainstorm for **EPR Phase 2B** — the block of work that sits between "wire-complete federation" (Phase 2C just landed on dev) and "native graph visible to the pillars." This is not yet an execution sprint. Several of the work items have coupling decisions that need to settle before a plan doc exists. The deliverable of this session is a **design spec** plus the first-draft **plan**, not production code.

## Context (self-contained)

EPR (Elohim Protocol Record) is the content-addressed graph substrate: CID-derived identity, detached Ed25519 signatures, Coupling (knowledge/value/governance), Reach enum (private/self/intimate/trusted/familiar/community/public/commons). Phase 2C (Apr 23–24) delivered the **wire-complete libp2p federation** — `/elohim/epr-atom/1.0.0` request-response, Fetch / FetchBatch / Announce handlers, integration-test harness, and Batch D Tasks 15–18 green on `dev` (merged `aa60247c` → addendum-driven work through commits through `e9e2806a`).

What Phase 2C delivered and Phase 2B inherits:

- ✅ CBOR codec with length prefix + size cap, version-pinned by golden vectors (`tests/vectors/epr_atom_messages.json`)
- ✅ Four request/response variants, structural-only `verify_incoming_epr` at `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs:220-270` (docstring explicitly calls out the deferred gap)
- ✅ `StubIdentityMap` for PeerId↔AgentPubKey resolution inside reach-gating — a stub by design
- ✅ `FederatedEprStore` with swarm-handle seam stubbed (5 `TODO(phase-2b)` markers in `elohim/elohim-storage/src/services/epr_store.rs:7,192,221,230,261`)
- ✅ Gossipsub foundation committed (`elohim/elohim-storage/src/p2p/behaviour.rs` — "recovery.invitation" topic subscribed)
- ✅ Harness `two_peer_swarm()` that future tests can compose

What Phase 2B must deliver (canonical list from `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` §"What follows Batch D", lines 180–194):

1. **PeerId → AgentPubKey real identity mapping** (replaces `StubIdentityMap`) — unblocks cross-peer private-reach serving that Task 16's second test documents as scoped
2. **Resolver-backed Ed25519 signature verify** — closes the gap Task 18's third test pins (byte-flipped 64-byte sigs currently accepted by structural-only verify)
3. **EprHead ↔ Envelope reconciliation** — the two read-models coexist; design the canonical projection path and the invariants that keep them coherent
4. **Projector: `epr_atoms` → pillar tables** — where "native graph" actually becomes visible to app queries; the projector is the crossing from substrate to pillars
5. **Signal Harness migration to emit EPRs** — the event system learns to publish through the substrate instead of (or alongside) its current types
6. **Write-through feature flag wiring** — operator-chosen ramp; not a global on/off
7. **Kademlia provider records + announcement fanout/dedup policy** — the discovery half of federation; Phase 2C only did fetch-if-you-know-the-CID

Items 1–4 have **coupling decisions** that must settle before a plan can be written. Items 5–7 can be scoped once 1–4 are framed.

### Why this is a brainstorm, not a plan

Phase 2C shipped a batch-structured execution (A/B/C/D) because the wire shape was already designed. Phase 2B is different: the projector's shape depends on reconciliation, which depends on how identity resolves, which depends on whether verification caches. These are *design* coupling, not *work* coupling. A plan drafted before the coupling settles is a plan that rewrites itself.

The brainstorming skill (`superpowers:brainstorming`) exists for exactly this shape.

### Breadcrumbs — read these in this order

**Phase 2C context (so Phase 2B inherits rather than re-derives):**

1. `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` — **primary reference.** §"Retrospective" (lines 28–94) captures what is now wire-locked; §"Out of scope" (lines 128–139) is Phase 2B's agenda; §"What follows Batch D" (lines 180–194) is the canonical 7-item list.
2. `genesis/docs/superpowers/plans/2026-04-23-epr-phase-2c-libp2p-federation-plan.md` — the parent plan. Skim Tasks 15–19 (lines 1449–1775) for the batch shape that Phase 2B *may* mirror if coupling settles cleanly.

**Substrate design (parent spec):**

3. `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md` — original graph-substrate design. The projector section is directly load-bearing for item 4; the envelope/payload two-layer model frames item 3.

**Code seams where Phase 2B lands:**

4. `elohim/elohim-storage/src/services/epr_store.rs` — 5 `TODO(phase-2b)` markers at lines 7, 192, 221, 230, 261. These are the swarm-handle seams for the `FederatedEprStore`.
5. `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs:220-270` — `verify_incoming_epr` docstring: *"does NOT verify the ed25519 signature under a public key — that requires a resolver we don't have at this layer"*. Item 2's scope is exactly this gap.
6. `elohim/elohim-storage/src/p2p/behaviour.rs` — gossipsub foundation committed in `e9e2806a`. Item 7's fanout/dedup work composes here.
7. `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` — Batch D harness. Phase 2B adds tests here; do not rebuild the harness.

**Downstream consumers — why this matters:**

8. `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` — R&O #4 (hREA / VF-GraphQL, 🔴) and #9 (graduation-path narrative, ✅ drafted) both depend on the projector being real. Narrative assumes exchange events become visible EPRs; that is item 4.
9. Memory `project_epr_substrate_vs_vf_graphql.md` — EPR is the substrate; shefa-speaks-VF-GraphQL is app-layer. Phase 2B must not conflate. The projector produces graph atoms, not VF events.

## Session scope

**Deliverables (two artifacts, one session):**

1. `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` — design spec capturing the resolved coupling decisions for items 1–4, and shape-only decisions for items 5–7.
2. `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` — first-draft plan mirroring the Phase 2C batch shape (A/B/C/D or whatever decomposition the brainstorm surfaces). Accept that the plan is a **draft** — a second session will tighten task-by-task scope.

**Out of scope (this session):**

- Writing production Rust for any of the 7 items. This session is design + plan, not implementation.
- Redesigning the Phase 2C wire format. That is version-locked by `epr_atom_messages.json`; any wire-breaking item becomes `/elohim/epr-atom/2.0.0` and requires its own sprint.
- Detailed operational gates (Jenkins stages, orchestrator DAG edits) — those belong to the plan's execution pass, not the brainstorm.
- R&O #4 design (hREA / VF-GraphQL mapping) — the projector must expose the substrate; VF mapping is one reader, not the projector's shape.
- Relabeling the existing `TODO(phase-2c)` markers (already done in commit `7f76082b`).

## Known unknowns the brainstorm must resolve

These are **coupling questions**, not task questions. Resolving them is the whole point of the brainstorm.

1. **Identity resolution — where does PeerId ↔ AgentPubKey actually live?** Three candidate shapes:
   - (a) DHT lookup via Kademlia records published at node-join
   - (b) Gossipsub announce on a dedicated topic (`identity.binding`), cached locally
   - (c) Bootstrap-time pairing signed by the imagodei zome
   Each has different freshness/availability/trust properties. The `p2p-design-gate` skill's A/A2/B/B2/C classification applies: an identity-binding record is a Category A-candidate (notarized) or A2 (derived via link), *not* C (operational). Pick and justify.

2. **Verify caching — does signature verification need a layer?** A resolver-backed verify that hits the DHT on every envelope is a per-ingest amplifier. Options: (a) verify once, mark envelope trusted in local store; (b) public-key LRU keyed by AgentPubKey; (c) no caching, design for bulk. The choice touches item 4 (projector inherits trust tags).

3. **Projector ownership — what owns the write into pillar tables?** Two framings: (a) projector is an `elohim-storage` internal that tails `epr_atoms` and writes pillar rows on commit; (b) each pillar service subscribes to a substrate event stream and updates itself. (a) is simpler and centralizes the invariant. (b) respects pillar autonomy. Memory: `project_three_layer_truth_model.md` — projection is elohim-storage's concern, which votes (a).

4. **EprHead vs Envelope — one read-model or two?** Current code has both. Reconciliation options: (a) EprHead becomes a derived projection of Envelope (single source of truth in Envelope); (b) EprHead is the read-optimized index and Envelope is the full atom (separate but reconciled). The addendum's §"Lesson for Phase 2B planning" (lines 43–49) warns that the wire shape is locked; Envelope is canonical. Validate that (a) holds.

5. **Signal harness migration — replace or coexist?** The current signal system has its own event types. Does Phase 2B migrate those types to EPRs immediately, publish through both for a window, or introduce a compat layer? The answer sets item 5's size.

6. **Write-through flag granularity — per-host, per-pillar, per-entity-type, per-operator?** Operator-chosen ramp implies per-host minimum. Per-pillar gives finer rollout. Per-entity-type is probably too fine-grained. Pick a default; memory `project_cadence_archetype_tunable_with_dev_overrides.md` frames the 4-layer override pattern this should slot into.

7. **Kad provider policy vs gossipsub fanout — how do these compose?** Announce can both publish a Kad provider record AND gossip the announce. Default policy: (a) Kad-only (pull-based discovery), (b) gossip-only (push-based), (c) both with dedup, (d) tiered (gossip for community-reach, Kad for commons). Memory `project_dht_vs_libp2p_scoping.md` applies — DHT is expensive/authoritative, keep narrow; operational gossip should carry discovery.

8. **Do items 5–7 need their own brainstorm pass?** If item-1 through item-4 couple tightly, the brainstorm may legitimately defer 5–7 to a second session. Honor that if it surfaces.

## How to run this session

1. **Check branch.** Fresh branch `feature/epr-phase-2b-design` off `dev` (current `dev` HEAD is `8f8d648e` — recovery M4 work stacking on top of Phase 2C). Do not mix design work into any in-flight branch.

2. **Invoke the brainstorming skill** (`superpowers:brainstorming`) — this is a design session. Follow it rigorously; do not skip the decomposition pass.

3. **For every entity the brainstorm touches, invoke `p2p-design-gate`.** Project CLAUDE.md's "P2P Design Gate (MANDATORY)" section applies. The projector, identity binding, write-through flag state, and reconciliation index are all entity-shaped — classify each as A / A2 / B / B2 / C before proposing storage/transport.

4. **Resolve the 8 known-unknowns** in §"Known unknowns" above. Write resolutions into the spec as they settle; do not hold them in conversation state.

5. **Draft the design spec** at `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`. Target shape:
   - Problem framing (one page — what Phase 2C left, what 2B closes)
   - The 8 coupling decisions, one section each, with chosen answer and justification
   - Sketches of the 7 work items in the now-decided coupling frame
   - Invariants the projector preserves (most important section — this is the contract between substrate and pillars)
   - Open questions explicitly deferred to a later session

6. **Draft the plan** at `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` using `superpowers:writing-plans`. Mirror the Phase 2C A/B/C/D shape if the brainstorm supports it. Accept that this is first-draft; a second session will narrow scope per batch.

7. **Commit.** One commit for the spec, one commit for the plan. Pattern: `docs(epr-2b): phase 2b design spec` / `docs(epr-2b): phase 2b first-draft plan`. Husky runs docs-only lint on these; do not bypass.

8. **Do not kick off implementation.** Design session ends when the plan lands. A second session dispatches execution.

## Constraints & conventions

- **Three-layer truth model.** DHT = notary; libp2p = data-ops; doorway = web2 projection. The projector lives in elohim-storage (data-ops layer). Memory: `project_three_layer_truth_model.md`.
- **DHT is expensive; keep it narrow.** Identity binding and provider records are authoritative — DHT candidates. Fanout and dedup are operational — libp2p/gossipsub. Memory: `project_dht_vs_libp2p_scoping.md`.
- **Schema-first IoC.** Any new wire contract the brainstorm produces gets a JSON schema in `elohim/sdk/schemas/v1/` *before* the plan writes Rust tasks. Memory: `feedback_schema_first_ioc.md`.
- **No sovereignty, no ownership.** The projector stewards the pillar view; it does not own pillar data. The pillars themselves steward their domains. Memory: `project_no_sovereignty_stewardship_over_ownership.md`.
- **Ungrudging service.** The projector produces pillar-visible data whether any specific pillar consumes it or not. No gating, no "did the pillar ack?" handshake. Memory: `project_ungrudging_service.md`.
- **Observed, not flagged.** Any "is this feature on?" question resolves by observing real state (a feature flag's *configured* state ≠ its *effective* state if no pillar listens). Memory: `project_elohim_active_observed_not_flagged.md`.
- **Substrate ≠ VF-GraphQL.** The projector produces graph atoms. VF mapping is app-layer, downstream. Do not let R&O #4 shape the projector's wire format. Memory: `project_epr_substrate_vs_vf_graphql.md`.
- **Plan & spec locations.** Spec → `genesis/docs/superpowers/specs/`. Plan → `genesis/docs/superpowers/plans/`. Never repo-root `docs/`. This kickoff is in `genesis/docs/plans/` because it is audience-facing (you, starting fresh). Memory: `reference_superpowers_docs_location.md`.

## Build & verification commands (reference only — this session does not build)

- Rebuild storage crate when design touches Rust shapes: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
- Regenerate TS types after any view change: `cd elohim/elohim-storage && cargo test export_bindings`
- Schema validation: `pnpm run schema:validate` + `pnpm run schema:check-dna`
- Federation integration tests (harness): `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`

## Definition of done

- [ ] `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` exists, resolves the 8 coupling decisions, sketches all 7 work items, and lists invariants the projector preserves.
- [ ] `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` exists as a first-draft plan mirroring the Phase 2C batch shape (or the alternative decomposition the brainstorm surfaces).
- [ ] Each entity-shaped artifact in the design has been run through `p2p-design-gate` with an A / A2 / B / B2 / C classification.
- [ ] Committed on `feature/epr-phase-2b-design` with husky passing.
- [ ] `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` §"What follows Batch D" updated with a pointer to the new spec + plan.
- [ ] Explicit deferrals written into the spec — any item 5–7 decision left unresolved gets named, so the next session's scope is clear.

## Memories worth checking on start

- `project_three_layer_truth_model.md` — DHT / libp2p / doorway split; the projector lives in libp2p-layer storage, not doorway.
- `project_dht_vs_libp2p_scoping.md` — DHT signing is expensive; keep it narrow.
- `feedback_schema_first_ioc.md` — wire contract → JSON schema first, Rust/TS second.
- `project_epr_substrate_vs_vf_graphql.md` — do not conflate substrate with app-layer mapping.
- `project_elohim_active_observed_not_flagged.md` — "is it on?" is observed, not a flag's state.
- `project_no_sovereignty_stewardship_over_ownership.md` — projector stewards, does not own.
- `project_ungrudging_service.md` — projector emits whether anyone listens or not.
- `project_cadence_archetype_tunable_with_dev_overrides.md` — 4-layer override pattern for the write-through flag default.
- `reference_superpowers_docs_location.md` — spec+plan go under `genesis/docs/superpowers/`.

Go.
