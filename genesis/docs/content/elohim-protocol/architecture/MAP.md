---
title: Architecture — MAP (the developer's walk)
id: map
tier: architecture
status: Living document
created: 2026-06-02
last-verified: 2026-08-11
maintainers: Matthew Dowell + Opus 4.8
realizes:
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (the household living core — the default reading path)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/INDEX.md (the graph; MAP is the walk over it)
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md (the composition-stack router that runs upstream of this lattice)
  - .claude/memory-kit/Q1-canonical-organization.md (the two-axis analysis this map renders)
informs:
  - new-developer onboarding; any sprint that needs "where does X live and what do I read first"
memory_anchors:
  - project_household_living_core_lived_contrast_diffusion
  - project_three_temporal_perspectives
defers:
  - the per-spec relationship graph (that is INDEX.md's job — MAP points back to it, never duplicates it)
---

# Architecture — MAP

**INDEX is the graph. MAP is the walk.**

[`INDEX.md`](./INDEX.md) tells you *what each architecture spec is* and *how the specs link to each
other* (the `realizes / informed-by / informs / defers` edges). This MAP tells you the thing the
graph can't: **how a human developer walks manifesto → seed epic → architecture seed → pillar guide →
code → scenarios**, by concern and by pillar, without reconstructing the graph by hand.

> **If you read one path first, read the household path.** The household is the protocol's *living
> core* — the foundation, the seed, and the driver — **not one of four equals.** The care-economy
> made computable (a boy buying strawberries for his sister) is the substrate primitive everything
> else composes outward from. Section 2 defaults you there. (`value_scanner/epic.md`;
> `project_household_living_core_lived_contrast_diffusion`.)

This map has three parts:

1. **[The Subject-Domain Map](#1--the-subject-domain-map)** — the D1–D10 concern lattice: what each
   architecture domain owns and where its boundary ends.
2. **[The Spine, as a Walk](#2--the-spine-as-a-walk)** — six per-pillar reading stanzas, household
   first.
3. **[The Gap Ledger](#3--the-gap-ledger)** — the honest list of what this canonical surface does
   *not* yet cover, each pointing at its tracked gap-item.

---

## 1 · The Subject-Domain Map

> **Axis 0 · Subject class (read FIRST).** The D1–D10 lattice and the pillar axis below are **both
> substrate-scoped** — they answer "where on the *product* surface does this live?" They are HONEST ONLY
> for **`protocol-canonical`** work (changes a learner/peer experience; lands in `app/` + `architecture/` +
> `a2o/features/<pillar>`). **`process-meta`** work (changes a developer/agent experience; lands in
> `.claude/` + a CLAUDE.md gospel — the memory-kit, skills, agents, CI, the loops) has **no honest D# and
> no pillar**; routing it through this lattice is the *D4 name-collision* that mis-filed four specs (see
> `history/2026-06-02-d4-name-collision`). The subject-class axis lives in **`.claude/subject-routing.yaml`**
> (the parent constitution, read by the brainstorm/decompose gates). Classify by deliverable-TARGET there
> FIRST; only `protocol-canonical` proceeds into D#+pillar below.

> **Axis 0b · Composition seam (the router upstream of this lattice).** D1–D10 answer "which *product*
> domain owns this?" They do **not** answer "which *layer of the stack* do I add to?" — hardware ·
> OS/packaging · runtime · mods/plugins · SDK · bridges · clients, and the four participation tracks a
> running node uses (T1 DHT-notary / T2 substrate / T3 spoke / T4 doorway projection). That axis is the
> [**seam-map concern-routing atlas**](./2026-06-21-elohim-seam-map-concern-routing.md) — the surface the
> root `CLAUDE.md` sends you to first for "where does this live?". Its disambiguator: adding a *manifest*
> → SDK seam; a *crate* → bridge seam; *native code* → mod/plugin seam. Read it when the concern is a
> **layer** question; read the lattice below when it is a **domain** question. The atlas is deliberately
> not a D# row — it routes across the composition stack, an axis no single domain owns.

The architecture seeds are **concern-organized, not pillar-organized** — by design. A single seed
(e.g. records-lifecycle) fans across all six pillars, so filing them by pillar would shred them. The
operative division is the **concern boundary**: *what is an atom?* vs *what state does it carry?* vs
*how does it travel?* These ten domains partition the **substrate spine**. They do not claim to cover
every `tier: architecture` doc in this directory — later seeds and the standing cross-cutting
*principle* docs sit outside the lattice by design, and the ones still awaiting a domain assignment are
listed honestly in the [Gap Ledger](#3--the-gap-ledger).

For each domain: **the seeds inside it**, **the pillars × layers it shapes**, and — the load-bearing
column — **where it ends** (the boundary to the next domain).

| # | Subject domain | Seeds (in `architecture/`) | Pillars × layer | Where it ENDS → next |
|---|----------------|----------------------------|-----------------|----------------------|
| **D1** | **EPR Envelope & Graph Substrate** — the foundation no pillar owns | `…core-graph-substrate`, `…epr-integrator-compatibility-contract`, `…epr-phase-2c-libp2p-federation` | **elohim** core · DHT-notary + libp2p | Ends at *what an atom is + how it signs and travels*. Acquires lifecycle state → **D3**; is typed evidence → **D2**. |
| **D2** | **Evidence Primitives** — attestation ↔ observation split | `…experience-story-epr`, `…attestation-consolidation`, `…observation-event-layer`, `…dna-signal-as-epr-envelope` | **elohim**/**lamad**/**imagodei** · attestations DHT-durable, observations libp2p-ephemeral | Ends at *the typed evidence carriers + the DHT/libp2p seam*. Acquires Active→Closed status → **D3**. |
| **D3** | **Records Lifecycle & State Transitions** — the master spec | `…records-lifecycle-design` (+ `applications/`, `horizons/`) | **all six** · DHT state machine | Ends at *the substantive-record state machine* (Active / Subordinate / Shelved / Closed + `surface`). When the subject is *data retention/forgetting* → **D4**. |
| **D4** | **Memory Lifecycle & Consolidation** — the comet shape | `…memory-lifecycle-design` | **elohim**/**imagodei** (agent memory) | Ends at *submerge/surface, consolidation, network-scale merge*. Sibling to D3; seam = "substantive records vs data retention." (Reconciliation a known todo — see Gap Ledger.) |
| **D5** | **Data Plane & Transport** | `…iroh-libp2p-complementarity`, `…tiered-quilt-stewardship` | **elohim**/**shefa** · libp2p+iroh, quilt custody | iroh-libp2p = *transport choice/diversity*; tiered-quilt = *governed custody* atop either. When bytes are governed-as-economy → emits into **D9**. |
| **D6** | **Runtime Topology & Composition** | `…elohim-hub-boundaries` | **elohim** · hub-trait / node / storage split | Ends at *the Hub trait + three-crate split*. When collectives coordinate across hubs → **D7**. |
| **D7** | **Collective Coordination & Governance** | `…multi-collective-collaboration-epr`, `…dna-upgrade-governance`, `…upgrade-revert-and-constitutional-consensus` | **qahal**/**mishpat** · DHT membership + consensus | Ends at *recursive Qahal + dual-integrity consensus + friction-gradient*, plus *how the governing rules themselves upgrade, revert, and amend*. The upgrade **policy + its enforcement** is as-implemented; the **constitutional** layer above it (amendment-at-reach, simulation gate, eternity clause) is spec'd `truth:VISION` — read its §11 for the enforced/vision line before citing it as a mechanism (Gap Ledger). |
| **D8** | **Web2 Projection & Doorway** | `…doorway-access-tier-patterns`, `…doorway-ssr-runtime`, `…doorway-two-axis-scaling` | **doorway** · projection + SSR compute | access-tiers = *reach-gating + cache-scoping*; ssr-runtime = *compute capability*; two-axis-scaling = *the projection read path vs the conductor/identity-hosting pool, and the graduation flywheel between them* (graduation is **accounting only** as-implemented — no source-chain export moves a hosted user's history to their device). The Track-4 projection boundary; peer-native stays clean behind it. **Doorway is OPTIONAL, not architectural** — the D5 mesh *is* the hosting layer (peers who care about content shard+replicate it); doorway only projects mesh-hosted content to HTTP browsers and absorbs the read-mass a peer can't. A peer functions with zero doorway (layers 1+2 only). |
| **D9** | **Economic Coordination & REA Interop** | `…wave3-valueflows-hrea-interop` | **shefa**/**elohim** · in elohim-storage (protocol-shaped), NOT doorway | Ends at *hREA / VF-GraphQL ↔ EPR-REA translation* (`bridges/valueflows`). |
| **D10** | **Testing Infrastructure** — the non-architectural floor | `…sweettest-integration-layer` | **elohim** · DNA integration tier | Ends at *the in-process-conductor test tier*. Exercises D1 substrate; not a substrate concern itself. |

**Cross-cutting:** D1's envelope + the IoC integrator-compatibility contract are foundational to all
of D2–D9; no single domain owns them.

**The second axis — pillar (where the *code* lives):**

| Pillar | Path alias | Owns code in | Shaped by domains |
|--------|-----------|--------------|-------------------|
| **elohim** (core) | `@app/elohim` | substrate primitives, runtime, data-ops | D1, D2, D3, D5, D6, D9 |
| **lamad** (learning) | `@app/lamad` | content/observation vocabulary, archival | D2, D3, D9 |
| **shefa** (economy) | `@app/shefa` | REA events, tier economics | D5, D9 |
| **imagodei** (identity) | `@app/imagodei` | custody, key-revocation, agent memory | D2, D4 |
| **qahal** (community) | `@app/qahal` | collective primitives, membership | D7 |
| **mishpat** (governance) | (DNA + sdk domain) | scenario hygiene, demotion/dissolution | D3, D7 |
| **doorway** (web2) | `@app/doorway` | Track-4 projection, SSR | D8 |
| **avodah** (process) | `@app/avodah` | reference impl of work-as-participation | D1 (demonstrator only) |

---

## 2 · The Spine, as a Walk

The spine is **manifesto → seed epic → architecture seed (D#) → pillar guide → code → scenarios**,
knit by the four frontmatter edges. The graph is real; this is how you *follow* it. Each stanza is a
five-step reading order for working on that pillar.

### Start here — the Household Living Core (the seed, not one of equals)

> The household is the living core — the foundation, the seed, and the driver. Care made computable
> is the substrate primitive the whole protocol composes outward from; every other surface is the
> household's pattern carried into another institution ("why isn't this like home?"). Read this
> cluster *first*, before any single-pillar walk.

**To work on the household care-economy:**
1. **Seed epic** → [`value_scanner/epic.md`](../value_scanner/epic.md) (the strawberry revolution; 21
   life-stage archetypes under [`value_scanner/`](../value_scanner/)) + its sibling
   [`resilience/README.md`](../resilience/README.md) (the grandma standard / mutual-aid recovery test).
2. **Architecture seeds** → [`records-lifecycle-design`](./2026-05-24-records-lifecycle-design.md)
   **Part A + B** (D3) — the household primitive — anchored by
   [`applications/mint-monarch-application-design.md`](./applications/mint-monarch-application-design.md)
   (the household exemplar), plus [`memory-lifecycle-design`](./2026-05-10-memory-lifecycle-design.md)
   (D4, so a small node's ledger doesn't melt) and a thin edge-elohim (D6).
3. **Pillar guides** → [`shefa/CLAUDE.md`](../../../../../app/elohim-app/src/app/shefa/CLAUDE.md)
   (REA events) + [`lamad CLAUDE.md`](../../../../../app/lamad/CLAUDE.md) (observation vocabulary).
4. **Code** → `elohim/elohim-storage/src/` (the REA substrate) + `app/elohim-app/src/app/shefa/` +
   the value-scanner content in [`value_scanner/`](../value_scanner/).
5. **Scenarios** → `genesis/a2o/features/shefa/`, `genesis/a2o/features/lamad/`,
   `genesis/a2o/features/resilience/`.

### elohim — protocol core (the substrate everything rests on)
**To work on `elohim`:** read (1) the substrate epics
[`economic_coordination/epic.md`](../economic_coordination/epic.md) +
[`living_memory/epic.md`](../living_memory/epic.md); (2) architecture seeds **D1, D3, D5, D6**
(core-graph-substrate, records-lifecycle, iroh-libp2p, hub-boundaries); (3) pillar guide
[`elohim/CLAUDE.md`](../../../../../app/elohim-app/src/app/elohim/CLAUDE.md) (the cross-pillar owner);
(4) code `elohim/elohim-storage/`, `elohim/epr/`, `elohim/elohim-hub/`; (5) scenarios
`genesis/a2o/features/elohim-core/`, `content/`, `federation/`.

### shefa — economy
**To work on `shefa`:** read (1) [`economic_coordination/epic.md`](../economic_coordination/epic.md);
(2) seeds **D9, D5** (wave3-valueflows-hrea-interop, tiered-quilt-stewardship); (3) pillar guide
[`shefa/CLAUDE.md`](../../../../../app/elohim-app/src/app/shefa/CLAUDE.md); (4) code
`app/elohim-app/src/app/shefa/` + `bridges/valueflows/` (the REA bridge in elohim-storage, not
doorway); (5) scenarios `genesis/a2o/features/shefa/`.

### imagodei — identity, recovery, agent memory
**To work on `imagodei`:** read (1) [`resilience/README.md`](../resilience/README.md) (recovery is
the vision's designated MVP acceptance test); (2) seeds **D2, D4** (attestation-consolidation,
memory-lifecycle); (3) pillar guide
[`imagodei/CLAUDE.md`](../../../../../app/elohim-app/src/app/imagodei/CLAUDE.md) (the 218-line
template the other guides mirror); (4) code `app/elohim-app/src/app/imagodei/`; (5) scenarios
`genesis/a2o/features/auth/`, `resilience/`.

### lamad — learning
**To work on `lamad`:** read (1) [`value_scanner/epic.md`](../value_scanner/epic.md) (content as
observation) + the seed-content `lamad.md`; (2) seeds **D2, D3** (observation-event-layer,
experience-story-epr); (3) pillar guide [`app/lamad/CLAUDE.md`](../../../../../app/lamad/CLAUDE.md)
(lamad is a **separate SPA** at `app/lamad/`, not a view inside elohim-app — that decomposition is
itself a known gap); (4) code `app/lamad/src/` + the `@elohim/sophia-*` renderer wrap; (5) scenarios
`genesis/a2o/features/lamad/`, `content/`.

### qahal — community & governance
**To work on `qahal`:** read (1) [`governance/epic.md`](../governance/epic.md) +
[`social_medium/epic.md`](../social_medium/epic.md); (2) seeds **D7** (multi-collective-collaboration-epr,
dna-upgrade-governance, and
[`upgrade-revert-and-constitutional-consensus`](./2026-07-14-upgrade-revert-and-constitutional-consensus.md)
for how the governing rules amend — check its §11 enforced-vs-vision split before building against it)
— mishpat (D3/D7) co-owns the demotion/dissolution + scenario-hygiene side; (3) pillar guide
[`qahal/CLAUDE.md`](../../../../../app/elohim-app/src/app/qahal/CLAUDE.md); (4) code
`app/elohim-app/src/app/qahal/` + the mishpat DNA at `elohim/holochain/dna/mishpat/`; (5) scenarios
`genesis/a2o/features/qahal/`.

### doorway — web2 projection
**To work on `doorway`:** read (1) [`public_observer/epic.md`](../public_observer/epic.md) (the web2
read-mass it absorbs); (2) seeds **D8** (doorway-access-tier-patterns, doorway-ssr-runtime,
[`doorway-two-axis-scaling`](./2026-06-11-doorway-two-axis-scaling.md) — read this one before touching
capacity or the graduation endpoints); (3)
pillar guide [`doorway/CLAUDE.md`](../../../../../doorway/CLAUDE.md); (4) code
`doorway/doorway-service/` (Rust gateway) + `app/elohim-app/src/app/doorway/` (Angular integration);
(5) scenarios `genesis/a2o/features/doorway/`, `ssr/`, `peer-oauth-portal/`. **Frame it correctly:**
"peers host for each other; doorway projects that back to the web" — never "doorway hosts content for
users." Don't route through doorway what peers can serve directly.

> **avodah** is a reference impl of work-as-participation (D1 demonstrator), not a true pillar — code
> lives at `app/elohim-app/src/app/avodah/`, now with its own guide at
> [`avodah/CLAUDE.md`](../../../../../app/elohim-app/src/app/avodah/CLAUDE.md).

---

## 3 · The Gap Ledger

This map is **self-aware about what it does not yet cover** — matching the repo culture where INDEX
is transparent about its own scope. Each row tags the gap **OPEN** (no spec) / **STRADDLE** (clarity,
not duplicate) / **CODE-NO-DOC** (built, un-doc'd at architecture tier) / **GUIDE-GAP** (missing
pillar guide), and points at its tracked entry in `.claude/memory-kit/gap-items/` (the decomposed
implement-surface) where one exists. **Do not author these as architecture seeds in a navigation
pass** — they are implementation-spec work, listed here so the walk is honest.

| Gap | Kind | Status | Tracked at |
|-----|------|--------|------------|
| **Elohim-agent subsystem** — crate/service/sdk/specialists/mcp exist (`elohim/elohim-agent/`) but no `tier: architecture` seed; an architect can't answer "what is an elohim-agent's role in the three-layer truth model" from canonical. The seed must establish the **observed-not-flagged invariant**: `Phase::ElohimActive` vs `DevContext` is derived from whether real inference actually ran (the `/wisdom/invoke` response is the source of truth, threaded into the attestation) — never assigned from a config flag, or reputation accumulation off those attestations would be corruptible | CODE-NO-DOC | OPEN (largest hole; Q2 Sprint 4 writes the thin seed first) | `gap-items/specs__2026-05-28-conductor-agent-info-substrate-gossip-design.json`; `project_elohim_active_observed_not_flagged` |
| **Pillar-service interface spec** — six pillar modules exist in `app/elohim-app/src/app/` but no canonical seed defines "what makes a pillar service" (interface, composition rules, truth-layer mapping); described only through application archetypes | CODE-NO-DOC | OPEN | `gap-items/plans__2026-05-25-pillar-epr-decomposition-plan.json`, `gap-items/specs__2026-05-25-pillar-epr-decomposition-design.json` |
| **Chain-layer consensus mechanics** — named in multi-collective ("consensus weight = care-aggregate"); [`upgrade-revert-and-constitutional-consensus`](./2026-07-14-upgrade-revert-and-constitutional-consensus.md) now specifies amendment-by-consensus-at-reach, but declares itself `truth:VISION` in §11 — **the mechanism is designed, not built**, so this stays open as *implementation*, no longer as *design* | OPEN | NARROWED (spec'd at vision tier; executable mechanics absent) | `gap-items/plans__2026-05-19-doorway-stewardship-chain-design.json` |
| **Bridge governance-acceptance gate** — records-lifecycle "Gap 9" (normative for all bridges) but no seed defines a bridge's governance gate; blocks any new `bridges/<vendor>` | OPEN | OPEN | `gap-items/specs__2026-04-26-storage-phase-11-zome-forwarding-bridge-design.json`; `bridges/CLAUDE.md` |
| **Governance multi-factor merge check** — memory-lifecycle §3.3 names it load-bearing (content-reach / authoritative-governance / valueflows-to-stewards / resiliency) but defers to sibling specs that don't exist | OPEN | OPEN (known dependency) | `gap-items/plans__2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.json` |
| **records-lifecycle ↔ memory-lifecycle reconciliation** — both say "lifecycle," distinct subjects (substantive records vs data retention); frontmatter admits "sibling vocabulary; reconciliation incomplete" | STRADDLE | OPEN (known todo; Q2 Sprint 3) | `gap-items/plans__2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.json` |
| **doorway-access-tier ↔ doorway-ssr-runtime** — both doorway; seam clear (reach-gating vs compute) but conceptually tight; candidate for a shared doorway sub-index | STRADDLE | OPEN (low-risk) | — (INDEX D8 rows) |
| **Cold-archive terminus** — tiered-quilt names the shelved terminus; records-lifecycle defers "organization-dissolution"; neither closes "what happens at the terminus" | STRADDLE | OPEN | `gap-items/2026-05-11-tiered-quilt-stewardship-design.json` (needs agent decomposition) |
| **lamad pillar guide** — bounces reader to a separate SPA CLAUDE.md → sdk domain vocabulary; the SPA-vs-view decomposition is itself unexplained | GUIDE-GAP | OPEN | [`app/lamad/CLAUDE.md`](../../../../../app/lamad/CLAUDE.md) exists but un-bridged from T2 |
| **Infrastructure crates** — `elohim-bitswap`, `elohim-compute`, `elohim-render` lack README/CLAUDE.md; unclear what is load-bearing vs operational. (`elohim-cache-core` and `elohim-token` have since been documented — this row narrowed to the remaining three) | CODE-NO-DOC | OPEN (narrowed) | — |
| **Lattice coverage** — §1's D1–D10 lattice was drawn over the substrate-spine seeds and has not absorbed the seeds authored since, nor the standing cross-cutting *principle* docs (`trust-as-efficiency-signal`, `social-reach-nervous-system`, `protocol-formal-substrate-rationale`, `ubiquitous-wisdom-dissolves-chokepoint`, `compute-commitment-substrate-floor`, `governance-layers-architecture`, `cluster-topology`, `imagodei-surfaces-design`, `blob-custody-reconciliation`, `qahal-epr-household-lattice`, `bloom-mastery-progression`, `alpha-test-bench-compute-envelope`). Some are genuinely lattice-external (a principle that governs *every* surface has no single D#); the rest need a domain assignment. Assigning them is a **domain-boundary decision**, not a navigation edit — it is deliberately not done in a currency pass | STRADDLE | OPEN (needs an operator-gated §1 pass) | — (this row is the tracker) |
| **brit crate** (git-as-covenant) — rich internal design docs, NOT surfaced in canonical architecture | CODE-NO-DOC | OPEN | `elohim/brit/` internal docs |
| **Code→spec backlinks one-directional** — specs cite code paths, but code files don't cite back via `// architecture:` headers (INDEX recommends, not enforced); graph is traversable spec→code, not code→spec | CODE-NO-DOC | OPEN (mechanize later) | INDEX.md §"How to use this directory" |

---

## How to use this MAP

- **Onboarding a new dev?** Start at Section 2's Household path, then the pillar stanza they'll work in.
- **Designing a feature?** Find your concern in Section 1 (D1–D10), read that domain's seeds, cite
  them `informed-by:` per [INDEX](./INDEX.md)'s frontmatter contract.
- **Hit a wall the canonical surface doesn't cover?** Check Section 3 — it may be a *known* gap with a
  tracked gap-item, not a hole you need to fill from scratch.
- **Not designing — *operating*?** This MAP is a design-time and onboarding-time walk. When the live
  substrate misbehaves — a dataplane probe reds, a deploy lands but the head doesn't propagate — the
  door is the [**substrate trust-contract runbook**](./2026-07-12-substrate-trust-contract-runbook.md):
  the invariants the dataplane holds, the probe watching each one, and the per-red decision tree. It is
  the operate-time counterpart to this walk, and where it and this map disagree, **the probes are the
  authority**.

MAP is a **Living document**: when a pillar guide lands, a gap closes, or a seed graduates, update the
relevant stanza/row here and the matching row in [INDEX](./INDEX.md). The two files split cleanly and
never duplicate — **INDEX is the graph; MAP is the walk.**
