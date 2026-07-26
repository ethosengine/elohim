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
`mishpat_mirror_backfill` level-triggered re-file sweep). One residue remains:

## 1. Dark rows in two more tables — CLOSED (2026-07-26, sprint-3, commit 2e321017c)

`signals.rs` stamped the same mis-supplied app_id onto
`gate_decision_challenges.app_id` and `challenge_outcomes.app_id`; their readers
(`http.rs` challenge surfaces) scope `ctx.h_app_id` = lamad. Cured by
`gate_challenge_namespace_backfill.rs` — a level-triggered **re-file-only** sweep
(no synthetic inserts: unlike rea_commitments, these tables have a **composite**
PK `(app_id, id)`, so a mis-filed row is not insert-blocking, and there is no
local desired-state ledger to synthesize an absent row from — an absent row means
the signal never landed, healable only by DHT re-read). Keyed on both PK halves;
skips the mis-filed-but-already-shadowed case non-destructively. Wired into the
same boot tick as `mishpat_mirror_backfill`.

**Latent defect surfaced and cured in the same commit:** `challenge_outcomes`
had **never been created on any node** since 2026-04-19 — its migration shared a
`YYYY-MM-DD-HHMMSS` prefix with `stewarded_nodes_add_archetype`, so
`embed_migrations!` silently kept only the sibling ([[feedback_diesel_migration_timestamp_collision]]).
`GET /db/challenge-outcomes` returned 500 "no such table" (not empty); every
`ChallengeOutcomeCreated` projection had failed on delivery for ~3 months. Fixed
by renaming the migration past the newest (`2026-07-26-000000`) with
`CREATE TABLE IF NOT EXISTS`. A pre-push guard (`ls migrations | sed 's/_.*//' |
sort | uniq -d` must be empty) is worth adding — this class hid a broken table
for three months.

## 2. ProjectionNamespace newtype (design decision)

`h_app_id` as a bare `&str` has no type-level distinction from an installed-hApp
id, so the confusion recurs at every new call site (three namespace-class
defects landed in one sprint arc: agent-key vs libp2p id in joins, epr:-prefixed
head_ref vs bare content id in the provide desired-set, and this one). A
`ProjectionNamespace` newtype threaded through the db layer would make the
mistake unrepresentable. Design-level change (touches many signatures) — plan
it, don't drive-by it.
