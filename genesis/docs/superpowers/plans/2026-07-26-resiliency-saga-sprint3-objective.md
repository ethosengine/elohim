---
title: "Resiliency Saga Sprint 3 — two doorways one truth (fresh-session Objective)"
id: resiliency-saga-sprint3-objective
tier: plan
status: Open
created: 2026-07-26
maintainers: Matthew Dowell + Claude Fable 5
sprint: resiliency-saga
requires_env: []
topic: [resiliency-saga, convergence, declare-canonical-head, coordinator-hot-swap, ghost-witness, mirror-backfill, stations]
cites:
  - resiliency-saga-valueflow | Resiliency Saga | sha256:1ffcaefb3212d80b | path: genesis/docs/superpowers/plans/2026-07-25-resiliency-saga-valueflow-plan.md
  - sprint-result-2026-07-26-resiliency-saga-overnight-cure | 2026-07-26-resiliency-saga-overnight-cure-sprint-result | sha256:87a6fd6ce75b523f | path: genesis/docs/superpowers/sprints/2026-07-26-resiliency-saga-overnight-cure-sprint-result.md
  - genesis/data/timeline/backlog/projection-namespace-h-app-id-class.md
  - genesis/data/timeline/backlog/security-doorway-auth-required-unenforced.md
---

# Resiliency-Saga Sprint 3 — pre-authored Objective (fresh-session handoff)

Written at the close of the 2026-07-26 overnight + sprint-2 arc as the clean
handoff seam. A fresh orchestration session boots from the SessionStart SAGA
line + this Objective + the cited records — no prior transcript needed.

## Objective

**Two doorways, one truth — close declare gate 2 and drain the station reds.**
`GET /api/v1/resilience/elohim-host-landing/household` reports the same
non-zero `stewardingCollectives` on doorway-alpha.elohim.host and elohim.host
(saga ch10 green), with the ch06 anchor-equality station
(`A.dhtAnchorHash == B.dhtAnchorHash`) flipped or demonstrably one slice away.

## Ranked work queue (evidence-grounded at sprint-2 close)

1. **declare-carries-Record coordinator slice** (centerpiece):
   `declare_canonical_head` (content_store coordinator zome) requires the
   target action to be locally retrievable — impossible on a full-arc fleet
   until gossip heals (spine notary-authority red). Change: accept the target's
   serialized `Record`, validate-and-integrate, then link. Coordinator-only ⇒
   DNA-hash-neutral ⇒ ships via the `update_coordinators` hot-swap, no re-key,
   no DHT churn. The App pipeline's DECLARE_ONLY ladder (stage-spa-blob.sh)
   then sends the Record it can already resolve from SOURCE_DOORWAY_URL.
2. **Verify the sprint-2 cures measured** (session's first 30 min): edge
   #1241+ deployed `3bf50e2a9` — confirm `mishpat_mirror_backfill` re-filed the
   active commitment (ch05 finish line + ch09 `commonsCommitments` flip) and
   `witness_ghost_anchors` is draining B's ~3k ghost rows (B
   `stewardingCollectives` rising). If not, those sweeps' own log lines are the
   first probe.
3. **ch02/07/08 measurement timing**: Dataplane Validation probes gauges ~2min
   post-restart, before the 5-min sweeps populate — chronically pending. Delay
   gauge probes, emit at boot, or add a second measurement pass.
4. **gate_decision_challenges / challenge_outcomes dark rows**: same h_app_id
   namespace class — needs the same three-way re-file sweep (cited backlog).
5. **ch06 divergentAnchor assertion reframe**: it watches an oscillating
   2000-row windowed sample; re-aim to the sprint-1 heal-outcome labels
   (`elohim_projection_heal_outcomes_total{outcome}`).
6. **grandma-album-1974**: fix the two a2o seed steps' invalid
   `content_type: 'album'` (→ `narrative`), correct the live row, add the
   symmetric content_type guard to reanchor_backfill's per-row skip.

## Operator-owned decisions (do not automate)

- `auth_required` doorway enforcement (cited backlog — CRITICAL; verify the App
  pipeline X-API-Key path before enabling).
- Pin-retirement policy (retry-exhausted pins live forever).
- ProjectionNamespace newtype (three namespace-class defects in one arc argue
  for it; design-level, plan it deliberately).

## Where the truth lives

- Story + stations: `genesis/a2o/features/dataplane/resiliency-saga/` (README
  chapter table current; ch05 station-decomposed; ch06 carries the
  anchor-equality node).
- The loop: push `[build:edge]` → edge Dataplane Validation →
  `genesis/scripts/jenkins-sync.sh` → SAGA line. One push per batch.
- Discipline: seams discovered mid-flight are minted as stations (story-harvest
  "Maintainer Role — Atom Perspective"; trigger lives in root
  CLAUDE.md/AGENTS.md).
