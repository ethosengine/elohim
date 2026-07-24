---
title: "Third-party gate closure — the consumer's verdict surface, declared"
id: third-party-gate-closure-plan
status: Landed
class: protocol-canonical
created: 2026-07-24
domain: D2
topic: [ontology, closure, app-manifest, gates, agentic-negotiation, byo-ontology]
cites:
  - closure-posture-axis-card-plan | Closure Posture | sha256:9c536692c53f5b91 | path: genesis/docs/superpowers/plans/2026-07-24-closure-posture-axis-card-plan.md
  - genesis/research/owl2-graduation-floor-ceiling-ontology-2026-07-23.md
  - elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
  - elohim/sdk/domains/lamad/manifest/gates.json
  - elohim/sdk/schemas/v1/registries/axis-card.schema.json
---

# Third-party gate closure

> **The founding observation.** The closure slice made every *protocol* axis declare what its
> silence means. It left the **consumer's** verdict surface entirely undeclared — which is the
> surface that matters most, because it is where someone else's ontology reaches a decision. A
> downstream app-manifest can ship `gates` that handle `AttestationWrite` and `CapabilityInvoke`
> with no statement of whether an unmatched gate means *unknown* or *no*. Bringing your own OWL is
> not the risk; bringing it into an undeclared verdict path is.

## What this does and does not close

Three cases, deliberately separated — the boundary is a **verdict-layer** constraint, not a content
or vocabulary one:

1. **OWL as an EPR artifact — stays wide open.** `content-format` is two-tier: 7 DNA-notarized
   `core` values plus an `extensible` tier the app layer owns (already 17 app-declared formats). A
   consumer declaring `owl-turtle` or `rdf-xml` is that mechanism working as intended. Fact-plane
   material; the open posture is correct and this plan does not touch it.
2. **A consumer's own vocabulary complementing a manifest — stays open, and closure is what makes
   accepting it SAFE.** The declaration is not a fence keeping foreign ontologies out; it is what
   lets one in without silently inheriting its assumptions. OWL's built-in answer to absence (OWA)
   is *correct* on the fact plane and *catastrophic* on the verdict plane, so the question that
   decides safety is never "is this OWL?" but "what does silence mean on the terms it introduces?"
3. **A decision procedure on a verdict path — this is what gets declared.** Not banned: *declared*.
   A gate may hold whatever posture it likes, so long as it says which.

## The measurement (why the obvious fix was wrong)

The first instinct was to drop the `TASK_1_TO_3_SURFACE` error filter so real manifests validate
against the whole schema. Measured, that would have surfaced **120 errors** — and **85 of them are
phantom**. The pillar loop validates the manifest *as loaded from disk*, without resolving the
modular `$ref` split, so the validator sees `{"$ref": "./manifest/content-types/article.json"}`
where the schema expects a content-type object, and reports it as a missing `description`, a missing
`coupling`, and an unexpected `$ref` — three errors per content type, none of them real.

The resolver already exists and is precise: `domains/lamad/scripts/codegen.mjs::resolveRefs` inlines
**only** `./manifest/` and `../` prefixed refs, deliberately leaving `./schemas/…` pointers alone
(those are JSON Schema references for metadata payloads and must stay refs). Applying that exact
rule before validating:

| | raw load | codegen ref rule |
|---|---|---|
| lamad | 93 errors | **8** |
| all 8 pillars | 120 errors | **35** |

So the filter is not merely hiding drift — it is **compensating for a loader that was never wired
in**. Fix the loader and the genuine drift is 35 items, counted and nameable, across a known set of
classes (`archetype` 7, `status` 5, `attestations` 4, missing `observations` 3, `minProperties` 3,
`gates` 2, missing `minimumReach` 2, and seven singletons).

This plan fixes the loader and declares the verdict surface. It does **not** clear the remaining
drift — it converts it from an unbounded "pre-existing drift, out of scope for B-MANIFEST" into a
counted backlog with a measurement anyone can re-run.

## P2P Design Gate: Third-party gate closure

### Entity: `gates` / `attestations` as declared manifest properties
- **Classification**: not an entity — **existing shipped manifest structure** being brought inside
  the schema that already claims `additionalProperties: false`. No new data is created; a key that
  ships today becomes describable.
- **Content Address Strategy**: n/a. A gate already carries `processCid` (`epr:gates:<name>`) as its
  own identity; this plan does not mint identity.
- **Source of Truth**: the per-domain `manifest/gates.json`, content-addressed through the manifest
  EPR that carries it.
- **Anti-Pattern Check**: no route, no table, no DHT entry type, no UUID. The schema **describes**
  what ships rather than redesigning it — a gate keeps `processCid`/`handlesEvents`/
  `governanceReach`/`peerReviewedBy`/`supersedes` exactly as authored.

### Entity: `closure` on a gate declaration
- **Classification**: not an entity — a required declaration on an existing structure, reusing
  `epr:schema:enum:closure` unchanged. No new vocabulary.
- **Anti-Pattern Check**: drift check performed — this is the SAME closure enum the axis cards use,
  `$ref`'d not copied. A second closure vocabulary would be the exact drift the axis-card identity
  rules exist to prevent.

### Design Constraints Discovered
- Only `lamad` ships `gates` (3). `attestations` spans 4 pillars (imagodei, infrastructure, mishpat,
  lamad). Declaring both is low-blast-radius.
- `attestations` is declared but **left unconstrained** (`additionalProperties: true`): stopping it
  being an *undeclared* key is this plan's business; governing its interior is not, and
  over-constraining a structure blind would manufacture drift rather than measure it.
- The 35 remaining drift items are NOT cleared here. The filter stays, narrowed so the newly-declared
  surface cannot hide behind it.

## Global Constraints

1. **No DNA, no routes, no tables, no migrations.** Schema + validation + one loader wiring.
2. **Reuse the closure vocabulary, never fork it** — `$ref: epr:schema:enum:closure`.
3. **A gate that handles an event is a verdict path** and must declare `closure`. A gate that handles
   none is inert and need not.
4. **Declaring ≠ constraining.** This plan makes the consumer's verdict surface *visible*; it does
   not dictate the posture a consumer takes. A third party may declare `open` — they may not decline
   to say.
5. **Path-limited commits only**; branch left un-pushed.

## Tasks

- [x] **Task 1 — wire the loader.** Lift the `resolveRefs` rule (`./manifest/` + `../` only) into the
      pillar-validation block of `scripts/test-schema.mjs` so manifests validate as composed rather
      than as split files. Assert the phantom classes are gone (no `additionalProperty: "$ref"`).
- [x] **Task 2 — declare the verdict surface.** Add `gates` to `app-manifest.schema.json`: an object
      of gate declarations (`processCid`, `description`, `handlesEvents`, `governanceReach`
      required; `peerReviewedBy`, `supersedes` optional), plus `attestations` as a declared-but-
      unconstrained object.
- [x] **Task 3 — the closure conditional.** A gate with a non-empty `handlesEvents` MUST declare
      `closure` (`$ref` the shared enum). Assert the negative: a gate handling `CapabilityInvoke`
      with no closure must FAIL.
- [x] **Task 4 — declare lamad's three gates.** Give `discernment-gate-v1-mechanical`,
      `reach-gate-v1` and `content-safety-gate-v1` an honest closure posture with rationale.
- [x] **Task 5 — narrow, don't drop, the filter.** Keep `TASK_1_TO_3_SURFACE` for the 35 counted
      pre-existing items; add the newly-declared tokens so a gate/closure violation fails loudly.
      Replace the open-ended "pre-existing drift" comment with the counted backlog.
- [x] **Task 6 — gates + commits.** `pnpm run schema:test`, `schema:validate`; path-limited commits.

## Landing note (2026-07-24)

Landed. All six tasks done, plus one unplanned fix the work forced.

**Three findings the implementation produced, none of them in the plan:**

1. **`governanceReach` is not a reach.** Shipped gates use `community` (a reach value) and
   `protocol` (not one). `$ref`-ing the field to `epr:schema:enum:reach` would have failed a
   shipped manifest, so it is typed as a constrained string with the divergence recorded in its
   description. Coercing it would have manufactured drift rather than measured it — the plan's own
   constraint. **Whether `protocol` is a reach, or `governanceReach` is a separate authority axis,
   is now an open question with a written home.**
2. **A bare `gates` filter token was too broad.** `gates` also appears as a pre-existing undeclared
   key *inside* `GovernanceLeg` (`vocabulary.contentTypes.*.coupling.governance.gates`) — unrelated
   to the top-level block. The discriminator is the `$defs` name `GateDeclaration`, not the key.
3. **`fixtureAjv` hand-registered five enums** while the regression block globbed all of them. Any
   new enum `$ref` in the manifest schema failed at *compile* time with a `MissingRefError` pointing
   nowhere near the list that caused it. Now globbed, matching the sibling block — the maintenance
   step is gone rather than documented.

**Measured drift: 35 → 30.** The declarations absorbed 5 (`attestations` 4, top-level `gates` 1).
The remaining 30 are counted and classed (`archetype` 7, `status` 5, missing `observations` 3,
`minProperties` 3, `governance-actions` 2, missing `minimumReach` 2, and eight singletons) and are
NOT cleared here — they are now a countable backlog with a re-runnable measurement instead of an
open-ended "pre-existing drift."

**Gate evidence.** `pnpm run schema:test` 76 passed / 2 failed — up from 70, six new assertions, all
green. The two `ContentView` failures are pre-existing on this branch and untouched (measured
baseline 51/2 before the closure slice). `pnpm run schema:validate` 3431 valid / 0 errors.
`codegen-ts --verify` up to date. The closure conditional is proven by four assertions including two
negatives: an event-handling gate without `closure` is REJECTED, and a `closure` outside the shared
vocabulary is REJECTED.

## What this slice deliberately does not do

Clear the 35 counted drift items · constrain the interior of `attestations` · emit any RDF/Turtle ·
require closure on gates that handle no events · dictate which posture a consumer must take ·
validate third-party manifests outside `sdk/domains/` (no such loader exists yet).
