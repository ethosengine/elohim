---
id: "backlog-projection-namespace-h-app-id-class"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "h_app_id projection-namespace vs installed-hApp-id confusion is a structural class — two tables still hold dark rows; a ProjectionNamespace newtype would end the class"
slug: "projection-namespace-h-app-id-class"
written: "2026-07-26"
author: "claude (resiliency-saga sprint-2 mirror leg, operator-directed)"
status: "open"
priority: "high"
ci_status: none
jobs: [elohim-edge]
tags: [projection, namespace, h-app-id, rea, signals, gate-decisions, newtype, data-integrity]
cites:
  - elohim/elohim-storage/src/signals.rs
  - elohim/elohim-storage/src/services/mishpat_mirror_backfill.rs
  - elohim/elohim-storage/src/api/mod.rs
---

# The h_app_id class: installed-hApp id ≠ projection namespace

## The class (proven live, 2026-07-26)

The conductor's installed-hApp id (`HOLOCHAIN_APP_ID="elohim"`) and the
projection namespace every reader scopes to (`h_app_id="lamad"`, via
`extract_app_context`'s `X-App-Id` default — no caller in the tree sends the
header) are different vocabularies flowing through the same bare `&str`
parameters. The signal subscriber stamped the installed id onto projections;
every reader filtered on the namespace; rows landed invisible. Because `id` is
the sole PRIMARY KEY on `rea_commitments` (h_app_id is a scoping column, not
part of the key), a mis-filed row is simultaneously invisible AND an
insert-blocking duplicate — insert-only backfills no-op silently and report
success.

The rea_commitments instance is CURED (subscriber wiring fix +
`mishpat_mirror_backfill` level-triggered re-file sweep). Two residues remain:

## 1. Dark rows in two more tables (bounded follow-up)

`signals.rs` stamped the same mis-supplied app_id onto
`gate_decision_challenges.app_id` and `challenge_outcomes.app_id`; their readers
(`http.rs` challenge surfaces) scope `ctx.h_app_id` = lamad. The wiring fix
corrects future writes; **existing rows in those two tables stay dark** — they
need the same three-way re-file sweep (absent/mis-filed/correct), keyed by each
table's real PK.

## 2. ProjectionNamespace newtype (design decision)

`h_app_id` as a bare `&str` has no type-level distinction from an installed-hApp
id, so the confusion recurs at every new call site (three namespace-class
defects landed in one sprint arc: agent-key vs libp2p id in joins, epr:-prefixed
head_ref vs bare content id in the provide desired-set, and this one). A
`ProjectionNamespace` newtype threaded through the db layer would make the
mistake unrepresentable. Design-level change (touches many signatures) — plan
it, don't drive-by it.
