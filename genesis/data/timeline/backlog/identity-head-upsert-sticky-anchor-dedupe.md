---
id: "backlog-identity-head-upsert-sticky-anchor-dedupe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A-class projection upserts triplicate the sticky-on-set anchor rule (identity_heads / lenses / mishpat_commitments) — optional DRY extraction (NOT a robustness defect)"
slug: "identity-head-upsert-sticky-anchor-dedupe"
written: "2026-07-18"
author: "identity-head arc (Wave C1 minor, whole-arc review follow-up)"
status: "open"
priority: "low"
area: "elohim-storage/db-projection"
domain: "D2"
jobs: [elohim-edge]
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
tags: [storage, projection, upsert, diesel, dry, sticky-on-set, dht-anchor, refactor, non-blocking]
---

# A-class projection upserts triplicate the sticky-on-set anchor rule

## What this actually is (correcting the ledger phrasing)
The whole-arc review ledger recorded a C1 minor as *"systemic unwrapped upsert (mirrors
lenses)."* Investigated precisely (2026-07-18): **this is NOT an error-handling / robustness
defect.** Every production upsert on the identity-head + sibling projection paths propagates
its `Result` correctly — `signals.rs:937` (`identity_heads::upsert_with_anchor(...).map_err(...)?`),
`signals.rs:914` (`lenses`), `signals.rs:903` (`mishpat_commitments`), all `?`-propagated;
`did_identity_store.rs` contains no upserts at all (reads only). The only `.expect()`s are in
`#[test]` code (correct). "Unwrapped" in the ledger meant **un-factored** (not DRY), not
error-unhandled.

## The real (low-value) observation
Three `db::*::upsert_with_anchor` functions hand-implement the SAME "sticky-on-set
anchor/revoked preservation" upsert field-for-field:
- `elohim/elohim-storage/src/db/identity_heads.rs::upsert_with_anchor`
- `elohim/elohim-storage/src/db/lenses.rs::upsert_with_anchor`
- `elohim/elohim-storage/src/db/mishpat_commitments.rs::upsert_with_anchor`

The rule (idempotent `on_conflict(cid) do_update`, where `dht_anchor_hash` + `revoked_at` are
overwritten ONLY when the incoming value is `Some(_)`, so a re-projection replay never strips
the notarized anchor nor resurrects a revoked row) is identical across all three; only the
per-table column set differs. The duplication is already **documented** — `identity_heads.rs`
module doc says *"Mirrors `db::lenses` field-for-field, including the sticky-on-set anchor/revoked
preservation rule,"* and each carries the cross-reference — so a reader is not misled.

## Disposition
**Recommended: ACCEPT the documented duplication** — it is safe, well-cross-referenced, and a
generic extraction is awkward in Diesel (each `.set((...))` is a distinct statically-typed column
tuple, so a shared helper needs a macro or a trait over the table DSLs, adding indirection over
three correct call sites). If picked up as a bite-sized session: extract a `sticky_upsert!` macro
(or a trait) capturing the on_conflict + Some-guarded-overwrite rule, migrate all three, and keep
the per-table column lists as the only macro input. Property-test the "replay never strips anchor /
never un-revokes" invariant once, shared. No behavior change — pure refactor.
