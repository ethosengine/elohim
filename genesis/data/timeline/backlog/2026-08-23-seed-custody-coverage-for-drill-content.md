---
id: "backlog-seed-custody-coverage-for-drill-content"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Mesh prologue: every artifact a chaos drill names (manifesto, chaos-ladder, the landing blob) gets custody-blob commitments on all three household peers — close the commitmentBacked 0 / heldBy [] precondition gap"
slug: "seed-custody-coverage-for-drill-content"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (operator-requested Codex queue, doorway-federated continuity roadmap)"
status: "refined"
priority: "high"
area: "a2o/mesh-prologue"
domain: "protocol"
jobs: [elohim-genesis]
relatedNodeIds:
  - "habit:blob-durability"
  - "feature:genesis/a2o/features/resilience/chaos-peer-churn.feature"
cites:
  - genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md
  - genesis/a2o/reports/sprint-report-household-20260823T000551Z-32aff87a.md
tags: [a2o, mesh, custody, seeder, bounded-code-fix, codex-claimable, agent-agnostic]
---

# Custody coverage for drill content

**Why this exists.** Three `@concern:blob-durability` chaos scenarios
(`chaos-peer-churn.feature`: flapping peer, cascading loss, simultaneous loss)
red HONESTLY at their Given — `"manifesto" has no custody footprint on this
mesh` (`commitmentBacked 1 / heldBy []`, lane run 20260823T000551Z). The
custody fold (`elohim-storage/src/services/household_resilience.rs`) is
correct; the INPUT is missing: initial custodians are operator-seed-driven
(`genesis/seeder/src/seed-commitments.ts`, pairs from
`CUSTODY_PAIRS_JSON` = `$MESH_DIR/drill-custody-pairs.json` written by the
prologue's `seed-drill-custody` leg, `hc-mesh-prologue.sh:419-425`), and the
pair set only names the drill fixtures, not the content the scenarios read.

## Scope (seeder + prologue only — no Rust)

1. Extend the drill-pairs producer (the script that writes
   `drill-custody-pairs.json`) so every content id the chaos drills name —
   `manifesto`, `chaos-ladder`, and the landing EPR's attached blob — resolves
   to its blob hash(es) and yields one `custody-blob` pair per household peer
   (matthew, jessica, james as provider; receiver = the content's steward
   persona per the collective-topology rule — never a synthetic genesis
   identity).
2. The hash MUST be resolved from the live mesh at prologue time
   (`GET /db/content/<id>` → `blobHash`/`serverBlobHash`), never hard-coded.
3. Keep `assertPairsNotSuspended` semantics; a content id that does not
   resolve fails the leg loudly (the leg is `soft` today — leave it soft but
   print which ids were not covered).
4. Make the leg idempotent: re-running the prologue must not mint duplicate
   commitments (check the existing dedupe path in `seed-commitments.ts`).

## DoD / verification

- `just seed validate` green.
- On a running mesh: `just mesh prologue` then
  `curl localhost:8090/api/v1/resilience/household?content=manifesto` (or the
  status route the steps use — see
  `steps/mesh/household-chaos.steps.ts` "household protection status")
  reports `protected` at 3 copies.
- `just test mesh features/resilience/chaos-peer-churn.feature` — the three
  scenarios pass their Given (they may still red later for product reasons;
  report the new failure line, do not weaken).
- `cd genesis/a2o && pnpm exec tsc --noEmit -p .` if any step glue changes.

## Disjointness

`genesis/seeder/` + `app/elohim-app/scripts/hc-mesh-prologue.sh` only. Do not
touch elohim-storage or doorway-service.
