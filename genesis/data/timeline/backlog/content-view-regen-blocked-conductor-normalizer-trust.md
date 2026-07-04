---
id: "backlog-content-view-regen-blocked-conductor-normalizer-trust"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content-view.ts regeneration is blocked: conductor-normalizer.ts must populate the trust field before the generated TS surfaces can be refreshed"
slug: "content-view-regen-blocked-conductor-normalizer-trust"
written: "2026-07-04"
author: "notary-authority-land shift"
status: "open"
priority: "medium"
ci_status: open
jobs: [elohim]
tags: [codegen, generated-drift, content-view, trust-tier, elohim-library, adapters, REQ-F10]
cites:
  - app/elohim-library/projects/elohim-service/src/adapters/conductor-normalizer.ts
  - elohim/sdk/schemas/v1/views/content-view.schema.json
  - elohim/elohim-storage/src/views_convert/lamad.rs
---

# content-view.ts regen blocked — conductor-normalizer must populate `trust`

## The failure (observed 2026-07-04, elohim/dev #1586)

Regenerating the committed `content-view.ts` files (7 surfaces) to pick up the
REQ-F10 `trust: string` field — live on the Rust wire since Phase A — fails the
app build with TS2741: `conductor-normalizer.ts:87` (elohim-service adapter)
hand-constructs a `ContentView`-shaped literal without `trust`. The committed
generated files' lag is therefore load-bearing, not cosmetic drift: they cannot
be refreshed until the adapter is fixed. Reverted in dev@d07da9442.

## The fix (sketch)

In `conductor-normalizer.ts`, populate `trust` when building the view from a
conductor-normalized record, mirroring the Rust `trust_label`
(`elohim-storage/src/views_convert/lamad.rs:73-81`): `"notarized"` when the
normalized record carries a DHT anchor / action provenance, else
`"unconfirmed"` — never hardcode `"notarized"` without provenance (the trust
label is REQ-F10 legibility; a false green padlock is worse than amber). Then
re-run `pnpm run schema:codegen:ts` and commit the 7 regenerated
`content-view.ts` files together with the adapter change (they are one atomic
unit — this failure is what happens when they land separately).

## Acceptance

`pnpm run schema:codegen:ts` followed by `pnpm exec ng build
--configuration=alpha` (in app/elohim-app) compiles clean with the regenerated
files, and the adapter's `trust` derivation has a unit test covering the
anchored and unanchored cases.
