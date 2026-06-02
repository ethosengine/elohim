---
title: Architecture — Index
tier: architecture
status: Living document
created: 2026-05-24
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

## Current architecture specs

### Records Lifecycle (in-flight)
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

These 14 specs were migrated from `genesis/docs/superpowers/specs/` on 2026-05-24. Their frontmatter normalization (to the architecture contract — `tier: architecture` + `realizes:` / `informed-by:` / `informs:`) is a follow-up pass; their content remains canonical as-is.

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

Each epic that has been realized by an architecture spec carries a "Technical Realization" section at its bottom pointing back here. Currently:

- `economic_coordination/epic.md` → records-lifecycle + mint-monarch + applications/INDEX
- `value_scanner/epic.md` → records-lifecycle + mint-monarch (care-stewardship)
- `social_medium/epic.md` → records-lifecycle + meta-facebook + drive + photos + patreon
- `living_memory/epic.md` → records-lifecycle (and memory-lifecycle sibling)

New architecture specs (including new application archetypes and horizon graduations) MUST add their backlinks to the epics they realize before being marked Landed.

## How to use this directory

**If you are designing a new feature or pillar**: read the architecture specs whose primitives you'll touch. Cite them as `informed-by:` in your sprint spec. If you find yourself wanting to introduce a new substrate primitive, propose an architecture spec amendment instead of inventing it locally.

**If you are reading an architecture spec**: walk the graph. `realizes:` takes you to the epic — the *why*. The body of the spec is the *what*. The code anchors in the spec body take you to the *how*. `informed-by:` shows what other architecture this rests on. `informs:` shows what downstream work must conform to it.

**If you are writing a code file that touches an architecture-defined primitive**: add a header comment with `// architecture: genesis/docs/content/elohim-protocol/architecture/<spec>.md` so the spec is reachable from the code. (This convention is recommended, not enforced; we'll mechanize it once we have more architecture specs to anchor against.)
