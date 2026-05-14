---
id: "backlog-stewarded-device-sync-feature-authoring"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Author stewarded-device-sync.feature to resolve canonical story's dangling reference + close /deliver iter-1 loop"
slug: "stewarded-device-sync-feature-authoring"
written: "2026-05-14"
author: "cartographer"
status: "refined"
priority: "high"
relatedNodeIds:
  - "story:james-son--as-stewardee--stewarded-device-sync"
  - "memory:feedback_story_delivery_status_axis"
  - "memory:project_three_layer_truth_model"
  - "memory:project_seed_whoever_is_ready"
  - "chronicle:2026-05-14-memory-ceremony-run-2"
  - "backlog:persona-rename-canonical-flip"
tags: [a2o, canonical-feature, deliver-loop, undelivered-to-active, james-son]
shift_objective: |
  The james-son--as-stewardee--stewarded-device-sync story is the protocol's first canonical
  experience-story (flipped to canonical 2026-05-14, Run #2). Its frontmatter declares
  `feature: stewarded-device-sync` (in genesis/a2o/features/auth/), but the file does not
  exist on disk — it is the single dangling reference in today's story-coverage-audit. The
  story is therefore at delivery_status: undelivered with delivery_status_source:
  deliver-bridge-floor: there is no canonical feature for /deliver to verdict against.

  Author the feature file at `genesis/a2o/features/auth/stewarded-device-sync.feature`,
  matching the story's narrative beats: James opens elohim-app on a second household device;
  the stewardship handshake completes via the bootstrap pair; identity claims project
  cleanly across the three-layer truth model (DHT → libp2p → doorway); recovery seed never
  surfaces to the human; the second device participates in the household quilt per
  seed-whoever-is-ready. Use the story's scenarios block as the seed source; pair the
  authoring with at least one Gherkin scenario per story beat (target 5-8 scenarios).
  Avoid serialization-bug scenarios (per a2o_is_human_experience_not_dev_bugs) — those
  belong in unit tests.

  Done when (a) feature file exists at the declared path with 5-8 scenarios; (b)
  story-coverage-audit.py shows zero dangling references; (c) /deliver runs and emits its
  first verdict for the canonical story (likely active.alpha or partial — that's the
  point); (d) backlog entry retires to active status as /deliver's verdict lands; (e)
  the resulting verdict propagates to the story's delivery_status frontmatter via
  delivery-status-poll.py.
---

# Stewarded-device-sync.feature authoring

## Why this matters

The james-son story is the substrate's first canonical experience-story. It carries the
weight of the persona-rename canonical-flip (six memory entries graduated/memorialized
against it) and proves the experience-story-EPR pattern at N=1. But its canonical feature
reference is dangling: `genesis/a2o/features/auth/stewarded-device-sync.feature` does not
exist on disk. Story-coverage-audit (2026-05-14) measures exactly this — `Dangling feature
references (story → missing file): 1`.

This entry closes the loop:
- The story exists, refined, canonical.
- The feature does not exist on disk.
- `delivery_status: undelivered` cannot move forward until /deliver has something to verdict.
- /deliver has nothing to verdict until the feature file exists.

Until this lands, the protocol's first canonical story is permanently stuck at
delivery_status floor. It is the cleanest, smallest /deliver loop-closing move available.

## What's blocking

Nothing substantive — the story is canonical, the persona-rename has stabilized, the
narrative beats are written into the story body. Authoring blocked only on operator pickup.

## What's ready

- Story is canonical (`status: canonical` per Run #2)
- Story body contains the scenario beats in prose form ready to translate to Gherkin
- Six related memory entries (now archived) provide ground-truth detail for stepdef phrasing
- Three-layer truth model + seed-whoever-is-ready memory entries define the technical shape
  of the handshake/projection/quilt-join the scenarios must exercise
- The dangling-reference audit will go to zero on a single file landing — measurement is free

## Convergence

- Storyteller Wave 2 (Run #3): NEEDS-CANONICAL-FEATURE flagged for james-son
- Cartographer Wave 1 (Run #3): horizon-scan + coverage audit elevated this as substrate-debt
- Story-coverage-audit (deterministic): 1 dangling reference, points here
- Run #2 chronicle: recorded the canonical flip but explicitly left feature authoring to
  a follow-up backlog entry — this is that entry

## Definition of done

1. Feature file authored at `genesis/a2o/features/auth/stewarded-device-sync.feature`
2. 5-8 scenarios covering the story's beats (bootstrap-pair-handshake, identity-claim
   projection, recovery-seed-invisibility, quilt-participation)
3. Scenarios honor the a2o-is-human-experience rule — narrative-level steps, not
   dev-internal serialization bugs
4. `story-coverage-audit.py` re-run shows zero dangling references
5. /deliver is invoked against the canonical story and emits its first verdict
6. delivery-status-poll.py flips the story's `delivery_status` from `undelivered` to
   whatever /deliver verdicts (likely `active.alpha` or `partial`)
7. This backlog entry retires (status → active.alpha matching the /deliver verdict, or
   `stable` if the verdict tier-3s clean)

## Pillar

`imagodei` (auth-lifecycle adjacent), with cross-cuts into `elohim` (three-layer truth
model) and `qahal` (household-as-collective context). Place under
`genesis/a2o/features/auth/` per the persona-rename canonical-flip story's frontmatter.

## Sequencing note

This is a single-author single-file authoring move. Recommend the storyteller (or an
imagodei-fluent operator) author directly rather than dispatching — the scenarios need
narrative judgment about what counts as the human's experience vs. dev plumbing. After
authoring, /deliver fires automatically (or by manual invocation) and the verdict
propagates without further cartographer involvement.

## Vision-alignment notes

- **Stewardship over ownership** (P5) — story is "James, the stewardee" not "James the
  owner"; feature scenarios should phrase device addition as stewardship handshake
- **Grandma standard** (recovery_grandma_standard memory) — recovery seed never surfaces;
  scenarios must assert seed-invisibility from the human's vantage
- **It just works** (subsume_g_f_a memory) — handshake completes without explicit
  configuration; scenarios should read like Apple-mantra polish
- **P1 reconciliation controller** — second device joining quilt is eager reconciliation;
  scenarios may exercise the deterministic floor

## Readiness score

**Vision-alignment 9/10** — the canonical story sits at the intersection of stewardship,
identity, and household-as-resilience-unit. Authoring the feature is the move that lets
those principles propagate to executable substrate.

**Readiness 9/10** — story canonical, beats written, dependencies stable, measurement
deterministic (audit), no blocking decisions outstanding. The only friction is finding an
authoring slot.
