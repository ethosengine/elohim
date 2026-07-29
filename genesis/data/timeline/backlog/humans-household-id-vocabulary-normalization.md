---
id: "backlog-humans-household-id-vocabulary-normalization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "humans.household_id carries two vocabularies (slug vs raw collective cid) — normalize the column"
slug: "humans-household-id-vocabulary-normalization"
written: "2026-07-29"
author: "deliver-the-saga morning sprint"
status: "open"
priority: "medium"
tags: [storage, identity-coherence, household, tech-debt, delegable]
---

# humans.household_id two-vocabulary normalization

Discovered during the 2026-07-29 overnight collectives-bootstrap work: the
`humans.household_id` column carries TWO value vocabularies — seeded rows hold a
slug (`household-dowell`), live-created rows hold a raw collective cid
(`elohim/elohim-storage/src/controller.rs:1085`). The identity-fill
membership-join (91719540c) deliberately EXCLUDES cid-form values to stay
deterministic, which means live-created humans are invisible to the
collective_cid gap-fill until normalized.

## Scope (disjoint, delegable to any agent)

- Decide the canonical vocabulary (slug is what the membership-join and seeded
  corpus use; cid is content-derived — check the p2p-design-gate identity rules
  before choosing cid removal vs dual-column).
- Normalize the write path in `controller.rs:1085` and add a fill-style
  migration/backfill for existing cid-form rows.
- Do NOT widen the membership-join to accept both vocabularies as a shortcut —
  that re-introduces the ambiguity the join's exclusion was protecting against.

## DoD / verification

- `cargo nextest run --lib` green in elohim/elohim-storage (CARGO_TARGET_DIR per
  pool preflight; RUSTFLAGS='--cfg getrandom_backend="custom"').
- A unit test proving a live-created human lands with the canonical vocabulary.
- Grep proof: no remaining writer stores the non-canonical form.
