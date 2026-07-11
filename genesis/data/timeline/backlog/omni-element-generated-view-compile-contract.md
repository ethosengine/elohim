---
id: "backlog-omni-element-generated-view-compile-contract"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Compile the native omni chrome element against the generated ResilienceSnapshotView + ChromeContext — phantom wire fields become compile errors, not silent neutrals"
slug: "omni-element-generated-view-compile-contract"
written: "2026-07-11"
author: "chrome-asset resilience-contract fix follow-up (2026-07-11)"
status: "open"
priority: "medium"
area: "chrome/wire-contract"
domain: "protocol"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:feedback-backend-authoritative-frontend-senses"
cites:
  - elohim/elohim-chrome-asset/src/lib.rs
  - elohim/elohim-chrome-asset/src/omni-element.js
  - elohim/elohim-chrome-asset/src/context.rs
  - elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json
  - elohim/sdk/schemas/generated-ts/views/resilience-snapshot-view.ts
tags: [chrome, omni-element, wire-contract, codegen, ts-rs, typescript, compile-time-safety, resilience, doorway]
---

# Compile the omni element against the generated view — durable fix for the phantom-wire-contract regression class

## The regression this closes the door on

The native, hand-written vanilla-JS omni element
(`elohim/elohim-chrome-asset/src/omni-element.js`, SSR-spliced onto every
page a doorway serves) shipped an `applyResilience()` mapper reading
`data.glyph` / `data.standing` / `data.reach` — fields that never existed on
`ResilienceSnapshotView`. The real contract is `protectionStatus` +
`feltStatus` (`/api/v1/resilience/{slug}/household`). Because the element is
plain JS with no type system, the fetch always succeeded against the live
endpoint and the mismatch was silent: the mapper always computed `null`, so
the resilience segment stayed the neutral `◉` glyph
("Resilience snapshot unavailable") forever. Nothing in CI caught it — the
only a2o omni coverage targeted the Angular `<app-protocol-omni>` component,
never this element (see the a2o regression scenario added alongside this
backlog item, `genesis/a2o/features/resilience/observable-distribution
.feature` — "Native omni chrome resilience segment speaks the real snapshot
contract").

Fixed at the DATA level in two commits:

- **cf7679688** — `fix(chrome): omni resilience segment speaks the REAL
  snapshot contract + gains the drilldown card`. Real `protectionStatus` /
  `feltStatus` mapping, glyph ladder (`●` protected / `◐` partial / `○`
  at-risk), click-through drilldown card.
- **b13d4d04e** — `chore(chrome): tri-state resilience-loaded marker +
  structural wire-contract test`. `data-omni-resilience-loaded` tri-state
  (`loading` → `applied` | `unmatched`) so the DOM itself testifies whether
  the mapper found mappable fields; `chrome_context_speaks_the_producer_
  contract` / `resilience_mapper_speaks_the_snapshot_contract` in `lib.rs`
  pin the wire-field list and ban the phantom accessors from ever
  returning.

## Why the fix so far is interim, not durable

Both landed fixes are DATA-level (correct the mapper's field reads) plus a
STRUCTURAL string-contract test (`ELEMENT_JS.contains("protectionStatus")`,
bans `"data.glyph"` etc. — `elohim-chrome-asset/src/lib.rs`). A string
`.contains()` scan on the raw JS text catches today's known phantom fields
staying dead, but it is not type-checked: nothing stops a FUTURE edit from
reading a field that was never on `ResilienceSnapshotView` in the first
place (the string scan can only ban fields it was told to ban — it has no
notion of "the wire schema" independent of a human keeping the two lists in
sync by hand). The same class of gap exists on the `ChromeContext` producer
side (`context.rs`): the JS's `ctx.<field>` reads and the Rust struct's
fields are kept in sync by a second hand-maintained string-contract test
(`chrome_context_speaks_the_producer_contract`), not by the compiler.

This is the same drift class the Rust→TypeScript boundary rule exists to
prevent everywhere else in the codebase (`CLAUDE.md` "Data Flow:
Rust-to-TypeScript Boundary" — types flow from Rust through `ts-rs`
codegen to `elohim/sdk/storage-client-ts/src/generated/`, and "snake_case
never leaves the Rust boundary"). The omni element is the one wire consumer
that sits OUTSIDE that pipeline: it is hand-written vanilla JS with no
build step (`include_str!`'d verbatim into the crate), so it never gets the
compile-time guarantee every TypeScript consumer already has.

## The durable fix (this item)

Author the omni element as TypeScript, compiled against:

1. The generated `ResilienceSnapshotView`
   (`elohim/sdk/schemas/generated-ts/views/resilience-snapshot-view.ts`) —
   the resilience-mapper's read surface becomes a typed field access;
   reading a phantom field is a compile error, not a silent runtime no-op.
2. The new `ChromeContext` producer struct (`context.rs`, landed alongside
   this backlog item) — once it grows a `#[derive(TS)]` / `ts-rs`
   projection of its own, `mount(ctx)` / `buildMarkup(ctx)` /
   `contextFromContentNode(slug, node)`'s `ctx.<field>` reads become typed
   against the same struct the doorway serializes, closing the inline
   context-island edge the same way.

`elohim-chrome-asset` then `include_str!`s the BUILT artifact (the
TypeScript compiler's output — still a single self-contained,
framework-free, no-runtime-deps JS file; the `V8`-free crate boundary
constraint in `lib.rs`'s module doc is unaffected, only the AUTHORING
surface changes) instead of the hand-written source directly. A
codegen-freshness gate (mirroring the existing pre-push check that
validates `schema:codegen:ts` output is current — see CLAUDE.md "Adding a
new view" step 6, "pre-push hook validates codegen freshness
automatically") must also validate the compiled element artifact is fresh
against its TypeScript source and the generated view types it imports, so
a stale build can't silently ship a JS file that no longer matches the
struct/schema it was compiled against.

**Interim protection** (in place today, sufficient until this item is
picked up): the structural string-contract tests in
`elohim-chrome-asset/src/lib.rs`
(`resilience_mapper_speaks_the_snapshot_contract`,
`chrome_context_speaks_the_producer_contract`) plus the new a2o browser-tier
regression scenario. Both catch a REPEAT of exactly this regression shape;
neither catches a NOVEL phantom field on a wire contract nobody thought to
add a string assertion for.

## Design decision date

2026-07-11, decided alongside the `ChromeContext` producer struct landing —
see `chrome_context_speaks_the_producer_contract`'s doc comment in
`elohim-chrome-asset/src/lib.rs`, which names this backlog item as the
tracked durable fix.

## Adjacent live defect (2026-07-11, spotted at deploy-verification)

`STABLE_ELEMENT_PATH` (`/chrome/omni-element.js` — the non-content-addressed
alias for static references like the Tauri `index.html`) answers
**404 "chrome asset not found"** on both doorways while the content-addressed
path serves correctly. The doorway's alias lookup misses; only the hashed
path is wired. Harmless to the doorway-served landing (it splices the hashed
path) but the Tauri static consumer this alias exists FOR would get no chrome
at all. Fix alongside this item (same route family), or sooner if Tauri
chrome testing starts.
