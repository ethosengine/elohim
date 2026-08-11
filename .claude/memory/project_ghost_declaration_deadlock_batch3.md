---
name: project_ghost_declaration_deadlock_batch3
title: "Batch-3 = ghost-declaration deadlock, not missing anchors"
description: "The \"~2000 unanchored rows\" were anchored rows with phantom declared heads (dead incarnations); cure = local-get responder + author-over-ghost decay (ELOHIM_GHOST_DECLARATION_DECAY)"
metadata: 
  node_type: memory
  title: "Batch-3 = ghost-declaration deadlock, not missing anchors"
  type: project
  originSessionId: 88cc5b6b-b8f7-4748-9355-792108825b4c
  modified: 2026-08-10T16:06:45.844Z
---

Diagnosed live 2026-08-10 (branch feat/angular22-node24). The batch-3 residual
("~2000 present-but-unanchored rows, anchor backfill needed") was a
misdiagnosis: matthew's 2028 rows are ANCHORED (1987 reach=familiar → evicted
from the distribution-safe inventory and from unauthenticated serving) with
`declared: true` heads that are **phantoms** — DHT records that exist in no
living conductor incarnation (SQLite PVC outlived DHT resets). Every exit
starved: contest → `no_local_chain` (matthew 130); record fetches →
`budget_elapsed` 202/202 on adam because zome `get_record_for_action` used a
network-strategy get that searches the fleet for nonexistent bytes (adam's
`storageArc: null` defeats the authority short-circuit); elections invisible
(`no_election` 279); adopt-before-author was already ENABLED fleet-wide but
byte-starved; ghost-witness sweeps all-`held` forever. The quiesce-gate
oscillation (actionable 2↔13, page-rotating) is this population's sweep slice.

**Why:** SQL projections claiming heads nobody can back wedge the
anti-self-election guard permanently; absence of an answer was doing the work
of evidence.

**How to apply:** cure landed = (1) `get_record_for_action` →
`GetOptions::local()` (coordinator hot-swap, no DNA-hash move); (2)
`ghost_decay_authorizes_author` in head_adoption — Hold/Contest → Author only
on positive double falsification (own conductor observed empty this sweep +
advertiser's stated no-record within the evidence-absent backoff window + no
local election), flag `ELOHIM_GHOST_DECLARATION_DECAY` (alpha=true via the
adopt-before-author placeholder; prod=false). Watch
`elohim_content_ghost_decay_author_total` + witness `authored` go nonzero,
actionable collapse to 0–2, then bank via `[build:edge] [edge:validate-only]`.
Related: [[project_saga_banking_validate_only_gate]]; adam's wedged puller
(pull total=0, caughtUp=false since boot) filed at
genesis/data/timeline/backlog/2026-08-10-adam-pull-loop-wedged-at-boot.md.
