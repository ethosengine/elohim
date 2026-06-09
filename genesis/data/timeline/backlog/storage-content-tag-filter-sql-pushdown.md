---
id: "backlog-storage-content-tag-filter-sql-pushdown"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "searchContent: push tag filter into SQL before LIMIT/OFFSET — remove the limit=10000 crutch and the latent truncation cliff"
slug: "storage-content-tag-filter-sql-pushdown"
written: "2026-06-08"
author: "cartographer"
status: "refined"
priority: "low"
relatedNodeIds:
  - "backlog-seed-provenance-anchor-gap"
tags: [storage, content, search, tag-filter, sql, diesel, latent-hardening, code-domain, unit-testable]
shift_objective: |
  searchContent (elohim-storage content_diesel.rs:288-306) filters by tag IN-MEMORY
  AFTER applying .order(created_at.desc()).limit().offset() — it loads up to `limit`
  rows from SQL, then `.retain()`s the ones whose tags match. This is a latent
  correctness cliff: a tag-matched row that sorts OLDER than the limit'th row is silently
  dropped before the in-memory filter ever sees it. Currently masked because content is
  idempotent-by-id (~3455 rows < limit=10000), but it is the same limit=N truncation class
  that bit the seeder idempotency read (ec5937287). Push the tag filter into SQL BEFORE
  order/limit/offset — join/subquery against the content_tags table so the LIMIT applies
  to the already-tag-filtered set — and remove the limit=10000 crutch. In-tree, unit-
  testable, no live stack needed. Done when a --lib test seeds >limit rows where the
  tag-matched rows sort oldest and asserts they are still returned.
---

# `searchContent` SQL-LIMIT-then-in-memory-tag-filter — latent truncation cliff

## The defect (latent, currently masked)

`elohim-storage` `content_diesel.rs:288-306`, the `searchContent` query path:

```rust
let contents: Vec<Content> = base_query
    .order(content::created_at.desc())
    .limit(query.limit)       // <-- LIMIT applied to the UN-tag-filtered set
    .offset(query.offset)
    .load(conn)?;
// ... load tags per content ...
if !query.tags.is_empty() {
    results.retain(|c| query.tags.iter().any(|t| c.tags.contains(t)));  // <-- tag filter IN MEMORY, AFTER limit
}
```

The tag filter runs **in memory, after** `.limit()`. So a content row that **matches the
requested tag but sorts older than the `limit`th row** never makes it into the loaded page and is
silently dropped — the caller sees fewer (or zero) tag matches than truly exist. This is the
**same `limit=N` truncation class** that defeated the seeder idempotency diff (fixed in `ec5937287`
by paging) — here it manifests as a search-result cliff rather than an idempotency miss.

**Why it's masked today:** content is idempotent-by-id, so the table holds ~3455 rows, well under the
`limit=10000` default. Every tag-matched row currently fits in the first page, so the in-memory
filter is correct *by accident of scale*. The crutch is the large default limit, and the cliff is one
order-of-magnitude growth away.

## The fix — push the tag filter into SQL before LIMIT

Filter by tag in SQL **before** `order/limit/offset`, so the LIMIT bounds the already-tag-filtered
set, not the whole table:

- Join (or `EXISTS` subquery) `content` against the `content_tags` table on `content_id`, constrained
  to the requested tag(s), **then** `.order(created_at.desc()).limit().offset()`.
- For multi-tag `any`-semantics (current `.iter().any()` behavior), a `WHERE content_id IN (SELECT
  content_id FROM content_tags WHERE tag = ANY($tags))` subquery preserves semantics; confirm `all`
  vs `any` matches the existing in-memory `.any()` before changing it.
- Once the tag filter is in SQL, **remove the `limit=10000` crutch** — the LIMIT can return to a sane
  page size because it no longer needs to over-fetch to compensate for post-filter shrinkage.

Keep the per-content tag-load loop for the *returned* page only (it's a display concern, not a filter
concern) — or fold it into the same join to avoid the N+1 noted in the existing comment
("This could be optimized with a single query and grouping").

## Test plan (unit, `cargo --lib`, no live stack)

1. Seed a test DB with **> `limit`** content rows.
2. Arrange so the **tag-matched rows sort OLDEST** (earliest `created_at`) — i.e. they would fall
   outside the first LIMIT page under the current code.
3. Call `searchContent` with that tag and a small `limit`.
4. **Assert** the tag-matched rows are still returned (under the current code they are dropped; under
   the SQL-pushdown they survive).
5. Regression: assert `any`-semantics unchanged for the multi-tag case, and that `offset` paginates
   the tag-filtered set correctly.

This is a clean, pickup-able, **in-tree-now** item — no alpha cluster, no peers, no shem. It hardens
the read path the provenance gap exposed (`seed-provenance-anchor-gap.md`) without depending on it.
