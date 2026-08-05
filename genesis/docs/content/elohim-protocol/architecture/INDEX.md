---
title: Architecture — Index
tier: architecture
status: Living document
created: 2026-05-24
last-verified: 2026-07-30
maintainers: Matthew Dowell + Opus 4.7
---

# Architecture — the junction between vision and code

This directory holds the protocol's **architecture specs** — design documents that codify the foundational tech-stack and composition choices. Every other spec, sprint, agent, and code change must be coherent with these. They are the load-bearing documents that connect the **manifestos / epics** (which live as siblings to this directory under `genesis/docs/content/elohim-protocol/`) with the **implementation code** (which lives across `elohim/`, `doorway/`, `steward/`, `bridges/`, `app/`).

The architecture specs are canonical: they define vocabulary, primitives, lifecycle states, and interop patterns that downstream work must conform to. Sprint-shape specs (under `genesis/docs/superpowers/specs/`) describe specific deliveries and cite these as `informed-by:`.

## Architecture vs sprint-shape — which is which?

| | **Architecture spec** (this directory) | **Sprint-shape spec** (`genesis/docs/superpowers/specs/`) |
|---|---|---|
| Lifespan | Lives as long as the protocol does | Lives until the sprint lands |
| Audience | Anyone designing a new feature, agent, or pillar | The sprint operators implementing the feature |
| Authority | Normative — defines invariants, vocabulary, patterns | Descriptive — describes a specific delivery |
| Frontmatter `tier:` | `architecture` | (unset; defaults to `sprint`) |
| Citations | Cited by sprint specs as `informed-by:` | Cite architecture specs as `informed-by:` |
| Naming | Topic-led (e.g., `records-lifecycle`, `observation-event-layer`) | Date-prefixed delivery (e.g., `2026-04-26-storage-phase-11-…`) |
| Edit pattern | Amendment with versioned status changes | Replaced when superseded |

A spec graduates from sprint-shape to architecture when its scope is **substrate-defining**, not delivery-defining: it specifies primitives, vocabulary, lifecycle states, or interop patterns that other specs must conform to.

## The frontmatter-glue contract

Every architecture spec MUST declare in its YAML frontmatter:

```yaml
---
title: <descriptive title>
tier: architecture
status: <Draft | In-flight | Landed | Superseded>
created: <YYYY-MM-DD>
authors: <human + AI co-authors>
pillar coupling: <which pillars touch this primitive>
realizes:                          # ← epics this spec gives technical form to
  - genesis/docs/content/elohim-protocol/<epic-dir>/epic.md (one-line context)
informed-by:                       # ← architecture or sprint specs this builds on
  - <path> (one-line context)
informs:                           # ← downstream specs / code this constrains
  - <category or specific spec>
memory_anchors:                    # ← MemPalace entries the spec leans on
  - project_<slug>
defers:                            # ← things explicitly out of scope
  - <one-line description>
---
```

The four relationship fields — `realizes`, `informed-by`, `informs`, `defers` — are the **graph edges** that knit architecture specs to epics, to each other, to downstream sprint specs, and to scope boundaries. Together they make the architecture navigable in both directions: epic ↔ architecture spec ↔ sprint spec ↔ code.

**`status:` is load-bearing, not decorative.** It declares how much of the spec is actually *built* — a seed may be an as-implemented distillation, or may declare itself vision (`truth:VISION`) for mechanisms that are designed but not yet built. Keep it honest as the spec's reality changes: a status that overstates implementation is worse than no spec at all, because downstream work will cite it as a mechanism.

## Current architecture specs

> **Completeness contract — the directory is the authority, not this list.** Every `tier: architecture`
> document in this directory MUST appear somewhere below. A doc present on disk but absent here is a
> **defect in this file**, not an unofficial spec. The check is mechanical and needs no maintained count
> — diff the directory against the links in this file:
>
> ```bash
> cd genesis/docs/content/elohim-protocol/architecture
> comm -23 <(ls *.md | grep -vxE 'INDEX.md|MAP.md' | sort) \
>          <(grep -oE '\]\(\./[^)/]+\.md\)' INDEX.md | sed 's|^](\./||; s|)$||' | sort -u)
> ```
>
> Empty output means the graph is complete. Anything printed is an uncatalogued seed. Deliberately no
> spec *count* is stated anywhere in this file: a hand-maintained tally is the thing that rots first,
> and a stale tally is how a reader learns to distrust the whole index. The complementary question —
> which *concern domain* (D1–D10) a seed belongs to — is **not** answered here; that is the walk's job,
> and the seeds still awaiting a domain assignment are tracked in [MAP](./MAP.md)'s Lattice-coverage row.

### Records Lifecycle — the anchor spec
[`2026-05-24-records-lifecycle-design.md`](./2026-05-24-records-lifecycle-design.md)

The eight foundational primitives (EPR, Event, Resource, Observation, Commitment, Attestation, FeedbackSignal, Links) — and the ten substrate gaps that close the records lifecycle gradient (Active → Subordinate → Shelved → Closed, with `surface` re-elevation, plus the bridge pattern for legacy systems).

**Realizes**: economic_coordination, value_scanner, social_medium, living_memory epics.

**Status**: Part A.1 (EPR primitive walkthrough) reviewed; Part A.2–A.8 stubbed. Part B refactored into per-archetype files under [`applications/`](./applications/) (active subsumption targets) and [`horizons/`](./horizons/) (deferred-but-coherent patterns).

### Substrate primitives — informed-by canonical specs

The records-lifecycle spec rests on these substrate-defining architecture specs (each is a `tier: architecture` canonical document):

| Spec | What it defines |
|---|---|
| [`2026-05-11-observation-event-layer-design.md`](./2026-05-11-observation-event-layer-design.md) | Observation substrate tier (libp2p+iroh; graduation pipeline) |
| [`2026-05-11-tiered-quilt-stewardship-design.md`](./2026-05-11-tiered-quilt-stewardship-design.md) | Tiered cold-archive substrate |
| [`2026-05-11-attestation-consolidation-design.md`](./2026-05-11-attestation-consolidation-design.md) | Attestation substrate (Content + content_type discriminator) |
| [`2026-05-20-wave3-valueflows-hrea-interop-design.md`](./2026-05-20-wave3-valueflows-hrea-interop-design.md) | REA / ValueFlows / hREA bridge substrate |
| [`2026-05-23-multi-collective-collaboration-epr-design.md`](./2026-05-23-multi-collective-collaboration-epr-design.md) | Cross-collective EPR custody handoff |
| [`2026-05-10-memory-lifecycle-design.md`](./2026-05-10-memory-lifecycle-design.md) | Memory lifecycle vocabulary (submerge ↔ surface; sibling to records-lifecycle) |
| [`2026-04-18-experience-story-epr-design.md`](./2026-04-18-experience-story-epr-design.md) | Foundational EPR design (content_type discriminator pattern) |
| [`2026-04-21-elohim-core-graph-substrate-design.md`](./2026-04-21-elohim-core-graph-substrate-design.md) | First-class graph pattern (EPRs as nodes, couplings as edges) |
| [`2026-04-21-elohim-epr-integrator-compatibility-contract.md`](./2026-04-21-elohim-epr-integrator-compatibility-contract.md) | EPR integrator-compatibility contract; the IoC layers |
| [`2026-04-23-epr-phase-2c-libp2p-federation-design.md`](./2026-04-23-epr-phase-2c-libp2p-federation-design.md) | EPR transport over libp2p |
| [`2026-05-08-iroh-libp2p-complementarity.md`](./2026-05-08-iroh-libp2p-complementarity.md) | Substrate transport architecture; three-track dual-stack |
| [`2026-05-02-elohim-hub-boundaries-design.md`](./2026-05-02-elohim-hub-boundaries-design.md) | Hub pattern (Tier-1 hub / Tier-3 node) |
| [`2026-05-15-dna-signal-as-epr-envelope.md`](./2026-05-15-dna-signal-as-epr-envelope.md) | DNA signal as EPR envelope pattern |
| [`2026-05-23-doorway-access-tier-patterns.md`](./2026-05-23-doorway-access-tier-patterns.md) | Doorway web2 projection access tiers |
| [`2026-06-02-sweettest-integration-layer.md`](./2026-06-02-sweettest-integration-layer.md) | DNA-level integration test tier (in-process conductors; native-build CI gotchas) |
| [`2026-06-02-doorway-ssr-runtime.md`](./2026-06-02-doorway-ssr-runtime.md) | Doorway server-render as an honest compute capability (Angular-19 SSR build-glue) |
| [`2026-05-02-blob-custody-reconciliation-design.md`](./2026-05-02-blob-custody-reconciliation-design.md) | Blob custody reconciliation — placement/salvage as a reconciled substrate primitive (placement signals are economic inputs to shefa) |
| [`2026-06-11-doorway-two-axis-scaling.md`](./2026-06-11-doorway-two-axis-scaling.md) | Doorway's two independent scaling axes — the projection read path vs the conductor/identity-hosting pool — and the graduation flywheel between them (graduation is accounting-only as-implemented; no source-chain export exists) |
| [`2026-08-05-wave2-relay-sovereignty-design.md`](./2026-08-05-wave2-relay-sovereignty-design.md) | Relay custody on the conductor transport plane — self-hosted iroh-relay, the tx5 retirement, and the never-n0 boundary (kitsune2 relay plane, distinct from the elohim-storage iroh dataplane) |

Most of this table arrived in a **migration from `genesis/docs/superpowers/specs/` dated 2026-05-24**; seeds authored in this directory afterwards were never part of that migration. Frontmatter normalization of the migrated set (to the architecture contract — `tier: architecture` + `realizes:` / `informed-by:` / `informs:`) is a follow-up pass; their content remains canonical as-is.

### Standing principles — patterns that govern every surface

These are not seeds for one subject area; each states an invariant that any surface must satisfy. They are cited `informed-by:` far more often than they are read end-to-end, and they are deliberately **outside** MAP's D1–D10 concern lattice — a principle that governs everything has no single domain.

| Spec | What it defines |
|---|---|
| [`trust-as-efficiency-signal.md`](./trust-as-efficiency-signal.md) | Trust as a compute-burden gradient — why trust is an efficiency signal, not a moral score; governs anything that propagates, discovers, validates, or replicates |
| [`protocol-formal-substrate-rationale.md`](./protocol-formal-substrate-rationale.md) | Why the substrate is formal — collapsing bureaucracy into the protocol rather than re-encoding it in policy |
| [`2026-05-04-compute-commitment-substrate-floor-design.md`](./2026-05-04-compute-commitment-substrate-floor-design.md) | The two-layer decision architecture — enforced **substrate floor** vs discerning **elohim ceiling**; the floor bounded/revocable/attested agent authority rests on |
| [`social-reach-nervous-system.md`](./social-reach-nervous-system.md) | Reach as a sense-respond nervous system, and the legitimate user-side filter — gating, propagation, anti-bubble policy, restitution as an economic event |
| [`ubiquitous-wisdom-dissolves-chokepoint.md`](./ubiquitous-wisdom-dissolves-chokepoint.md) | Capture-resistance as an AI-**deployment** property, not a substrate trick — wisdom at every node, not at a gate |

### Orientation and operations — the cross-cutting surfaces

Routers and runbooks rather than subject seeds. Two of these are what the root `CLAUDE.md` sends an agent to *first*, before any domain reasoning begins.

| Spec | What it defines |
|---|---|
| [`2026-06-21-elohim-seam-map-concern-routing.md`](./2026-06-21-elohim-seam-map-concern-routing.md) | The concern-routing atlas — the device spectrum × composition stack, the three extension seams (SDK / bridge / mod), and the four participation tracks. Answers "**where does this live?**" on the *layer* axis, upstream of MAP's domain lattice |
| [`2026-07-12-substrate-trust-contract-runbook.md`](./2026-07-12-substrate-trust-contract-runbook.md) | The dataplane's trust contract — the invariants you may assume, the probe watching each one, and the per-red decision tree. The **operate-time** door; where it and a design doc disagree, the probes are the authority |
| [`cluster-topology.md`](./cluster-topology.md) | The live P2P modeling canvas — the multi-node topology the test environment actually runs, as distinct from the protocol architecture it hosts |

### Governance, upgrade, and agent authority

How the protocol's own rules change, and how an agent earns the authority to act. Read the `status:` line of each before citing it as a mechanism — this cluster deliberately separates what is **enforced** from what is **vision**.

| Spec | What it defines |
|---|---|
| [`2026-06-11-dna-upgrade-governance.md`](./2026-06-11-dna-upgrade-governance.md) | The upgrade **policy** home and its enforcement (as-implemented distillation; the stewardship philosophy itself lives in protocol canon, not here) |
| [`2026-07-14-upgrade-revert-and-constitutional-consensus.md`](./2026-07-14-upgrade-revert-and-constitutional-consensus.md) | Propagation-is-consent, the paired upgrade/revert pattern, earned ceiling authority, amendment-by-consensus-at-reach, the simulation gate, the eternity clause. **Declares itself `truth:VISION` in its §11** — the constitutional mechanisms are designed, not built |
| [`governance-layers-architecture.md`](./governance-layers-architecture.md) | What an *elohim* is operationally — a pattern of context-bound, ephemeral, constitutionally-disclosed specialist subagents, not a monolithic agent that knows a human |
| [`2026-07-16-alpha-test-bench-compute-envelope.md`](./2026-07-16-alpha-test-bench-compute-envelope.md) | Observed capacity constraint promoted to a governed, bounded commitment — the test bench as a compute envelope rather than an architecture |

### Pillar-scoped seeds

Seeds whose subject sits inside one pillar rather than across the substrate.

| Spec | What it defines |
|---|---|
| [`imagodei-surfaces-design.md`](./imagodei-surfaces-design.md) | Imagodei decomposed into three architecturally distinct identity surfaces (identity core, web2 projection + recovery path, defender attestations) |
| [`2026-06-04-qahal-epr-household-lattice-design.md`](./2026-06-04-qahal-epr-household-lattice-design.md) | The qahal household living-core lattice — a deliberately thin frame gathering the qahal work |
| [`2026-06-11-bloom-mastery-progression-design.md`](./2026-06-11-bloom-mastery-progression-design.md) | Mastery progression over a Bloom-style gradient (lamad) |

> **Frontmatter-contract exceptions.** Five documents in this directory do not declare `tier:` —
> `2026-06-21-elohim-seam-map-concern-routing`, `2026-07-12-substrate-trust-contract-runbook`
> (both `status: reference`), `2026-06-04-qahal-epr-household-lattice-design`,
> `2026-06-11-bloom-mastery-progression-design`, and `2026-08-05-wave2-relay-sovereignty-design`
> (all three `status: Draft`; the last also carries `class: substrate`). They are catalogued above
> because they live here and are cited as canonical; whether `reference` should be a declared tier
> alongside `architecture` is an open question for the contract, not a defect in the docs.

### Application archetypes (the proof gallery)

Active subsumption targets — each is its own canonical-architecture file with frontmatter bridging epic narrative → technical composition → code anchors. See [`applications/INDEX.md`](./applications/INDEX.md) for the architect-audience framing.

- [`applications/mint-monarch-application-design.md`](./applications/mint-monarch-application-design.md) — personal finance + household stuff (full draft / exemplar)
- [`applications/khan-academy-application-design.md`](./applications/khan-academy-application-design.md) — learning platform
- [`applications/google-drive-application-design.md`](./applications/google-drive-application-design.md) — file store + collaboration
- [`applications/google-photos-application-design.md`](./applications/google-photos-application-design.md) — media library
- [`applications/meta-facebook-application-design.md`](./applications/meta-facebook-application-design.md) — social graph + feed
- [`applications/patreon-application-design.md`](./applications/patreon-application-design.md) — creator monetization
- [`applications/requests-offers-application-design.md`](./applications/requests-offers-application-design.md) — cooperative commerce
- [`applications/aws-compute-application-design.md`](./applications/aws-compute-application-design.md) — peer-native compute marketplace

### Horizons (deferred-but-coherent)

Patterns we've thought through that are NOT on the active subsumption path right now; preserved so the architectural thinking isn't lost. See [`horizons/INDEX.md`](./horizons/INDEX.md).

- [`horizons/youtube-application-design.md`](./horizons/youtube-application-design.md) — digital media platform
- [`horizons/wordpress-application-design.md`](./horizons/wordpress-application-design.md) — composed SPA / personal site
- [`horizons/factory-application-design.md`](./horizons/factory-application-design.md) — industrial supply chain as collective
- [`horizons/bank-application-design.md`](./horizons/bank-application-design.md) — financial institution as collective

## Bidirectional links from epics

Each epic that has been realized by an architecture spec carries a "Technical Realization" section at its bottom pointing back here:

- `economic_coordination/epic.md` → records-lifecycle + mint-monarch + applications/INDEX
- `value_scanner/epic.md` → records-lifecycle + mint-monarch (care-stewardship)
- `social_medium/epic.md` → records-lifecycle + meta-facebook + drive + photos + patreon
- `living_memory/epic.md` → records-lifecycle (and memory-lifecycle sibling)

New architecture specs (including new application archetypes and horizon graduations) MUST add their backlinks to the epics they realize before being marked Landed.

## How to use this directory

**If you are designing a new feature or pillar**: read the architecture specs whose primitives you'll touch. Cite them as `informed-by:` in your sprint spec. If you find yourself wanting to introduce a new substrate primitive, propose an architecture spec amendment instead of inventing it locally.

**If you are reading an architecture spec**: walk the graph. `realizes:` takes you to the epic — the *why*. The body of the spec is the *what*. The code anchors in the spec body take you to the *how*. `informed-by:` shows what other architecture this rests on — **read the target's `status:` before you build on it**, because resting an `informed-by:` edge on a vision-tier spec means resting on a *design*, not a mechanism; carry that qualifier forward into your own spec rather than letting a vision read as a delivery. `informs:` shows what downstream work must conform to it.

**If you are writing a code file that touches an architecture-defined primitive**: add a header comment with `// architecture: genesis/docs/content/elohim-protocol/architecture/<spec>.md` so the spec is reachable from the code. (This convention is recommended, not enforced; we'll mechanize it once we have more architecture specs to anchor against.)
