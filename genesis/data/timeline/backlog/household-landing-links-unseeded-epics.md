---
id: "backlog-household-landing-links-unseeded-epics"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The household landing links ten atoms the prologue never seeds — every household doorway answers /epr-head/<slug> 404 for them alike"
slug: "household-landing-links-unseeded-epics"
written: "2026-09-26"
author: "one-head-delivered sprint, Lane F task F1"
status: "open"
priority: "low"
jobs: [elohim-genesis]
cluster: "mesh-prologue-cast-and-env-gaps"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [household-mesh, prologue, seed-set, landing, epr-head, campaign-1.x, d8]
cites:
  - genesis/a2o/reports/recovery/serving-edge-20260926/apex-transition-run18.log
  - app/elohim-app/scripts/hc-mesh-prologue.sh
  - app/elohim-app/src/app/components/vision/vision.component.ts
  - app/elohim-app/src/app/components/path-forward/path-forward.component.ts
  - app/elohim-app/src/app/components/learning-success/learning-success.component.ts
  - genesis/a2o/steps/dataplane/apex-transition.steps.ts
---

**The fact.** The landing page the household serves renders self-resolving reference cards for ten
atoms the Act I prologue never seeds, so the browser reads `/epr-head/<slug>` and every household
doorway (A :8888, B :8889, C :8890) answers 404 for the same ten slugs. Probed live on 2026-09-26:
all three doorways return 404 for these, and 200 for the neighbouring cards the prologue does seed
(`elohim-protocol`, `evolution-of-trust`, `know-thyself-path`). This is a corpus gap, not a failover
defect. The apex-transition sibling step now compares the sibling's browser with the primary's own
browser reading, so these shared 404s no longer fail it.

| Slug | Linked from | Seed source present |
|---|---|---|
| `value-scanner-epic` | `vision.component.ts` | `genesis/data/lamad/content/value-scanner-epic.json` |
| `autonomous-entity-epic` | `vision.component.ts` | `genesis/data/lamad/graph/epic-autonomous_entity.json` |
| `governance-epic` | `vision.component.ts` | `genesis/data/lamad/content/governance-epic.json` |
| `social-medium-epic` | `vision.component.ts` | `genesis/data/lamad/content/social-medium-epic.json` |
| `economic-coordination-epic` | `vision.component.ts` | `genesis/data/lamad/content/economic-coordination-epic.json` |
| `public-observer-epic` | `vision.component.ts` | `genesis/data/lamad/graph/epic-public_observer.json` |
| `quiz-who-are-you` | `learning-success.component.ts` | `genesis/data/lamad/content/quiz-who-are-you.json` |
| `concept-path-forward-policymakers` | `path-forward.component.ts` | `genesis/data/lamad/content/concept-path-forward-policymakers.json` |
| `concept-path-forward-developers` | `path-forward.component.ts` | `genesis/data/lamad/content/concept-path-forward-developers.json` |
| `concept-path-forward-communities` | `path-forward.component.ts` | `genesis/data/lamad/content/concept-path-forward-communities.json` |

**Evidence.** `genesis/a2o/reports/recovery/serving-edge-20260926/apex-transition-run18.log`, lines
36–120: scenario "The apex name survives its doorway's shed" failed on the sibling (:8889) with these
ten 404s. Before task F1 the step required the sibling's browser to see no HTTP errors at all. The
primary's first visit was a raw fetch, so it never loaded the cards.

**Fix shape (pick one).** (a) Add the ten ids to the prologue's `BASE_CORPUS_IDS`
(`app/elohim-app/scripts/hc-mesh-prologue.sh:288`). The seed JSON already exists for each one. This
costs a longer prologue and more head-plane entries on a household cast. (b) Drop or gate the links
in the household landing content. Option (a) keeps the household landing the same page the fleet
serves, so it is the default. Done when a fresh prologue answers 200 on all three doorways for every
slug above.
