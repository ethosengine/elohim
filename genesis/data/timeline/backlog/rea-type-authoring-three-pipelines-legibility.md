---
id: "backlog-rea-type-authoring-three-pipelines-legibility"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "REA type authoring: three type-generation pipelines are invisible to the priming surfaces"
slug: "rea-type-authoring-three-pipelines-legibility"
written: "2026-07-30"
author: "memory-ceremony"
status: "refined"
priority: "medium"
tags: [legibility, rea-economics, schema-authority, type-codegen, gospel-currency]
cites:
  - elohim/sdk/domains/CLAUDE.md
  - elohim/sdk/domains/shefa/CLAUDE.md
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - app/elohim-app/src/app/elohim/models/rea-bridge.model.ts
  - .claude/skills/rea-economics/SKILL.md
shift_objective: |
  Make REA type authoring self-locating. Add a three-row "which type pipeline do I use?"
  discriminator (JSON-schema → codegen · views.rs → ts-rs · domain wire-types crate), each row
  naming its trigger, source of truth, and codegen command; home it in elohim/sdk/CLAUDE.md and
  cite it from elohim/sdk/domains/CLAUDE.md and the rea-economics skill. Then decide whether
  REA_ACTIONS graduates to elohim/sdk/schemas/v1/enums/ or is recorded as a deliberate
  Rust-first exception to the schema-first rule. Done when a fresh-context agent primed for
  "add a new economic event type end-to-end" can name its pipeline without reading source.
---

## Concern

An agent primed to add a new economic-event type end-to-end cannot tell **which** type pipeline
it should use, because three coexist and no priming surface names all three together:

1. **JSON-schema → codegen** — `elohim/sdk/schemas/v1/` (+ `views/` for HTTP wire shapes), the
   pipeline root `CLAUDE.md` documents: write `{name}.schema.json`, add a schema-contract test,
   register in `INTERFACE_FILES`, run `schema:codegen:ts`.
2. **`views.rs` → ts-rs** — `elohim/elohim-storage/src/views.rs` → `cargo test export_bindings` →
   `elohim/sdk/storage-client-ts/src/generated/`.
3. **Domain wire-types crate** — `elohim/sdk/domains/{domain}/types/` (MessagePack, snake_case,
   `ts-rs` behind a `ts` feature), documented only in `elohim/sdk/domains/CLAUDE.md`. Shefa already
   has 30+ types wired to the DNA `content_store` coordinator zome, explicitly covering
   `Agreement, Commitment, EconomicEvent, PremiumGate`.

Surfaced by the /memory-ceremony 2026-07-30 Phase-4b coherence lens (a fresh-context agent primed
for exactly this sprint). It could not determine whether a new economic event type is a
coordinator-zome wire type, an HTTP view, or both — nor whether to extend the existing
`EconomicEvent` wire type rather than declare a parallel one in `views.rs`.

## Sub-concern: REA_ACTIONS has no governing schema

The gospel rule is stated in four places ("never hand-write a TypeScript interface that mirrors a
schema — it will drift"). But the REA action vocabulary is a hand-maintained Rust const —
`REA_ACTIONS` in `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (~24
verbs) — mirrored by a hand-written TS union in
`app/elohim-app/src/app/elohim/models/rea-bridge.model.ts`, with **no** schema in
`elohim/sdk/schemas/v1/enums/` between them and no codegen link. An agent following schema-first
doctrine goes looking for a governing schema that does not exist.

The 2026-07-30 ceremony flagged the exception inline in `.claude/skills/rea-economics/SKILL.md` so
it stops being a silent trap. The architectural question — whether REA_ACTIONS should graduate to a
protocol schema enum like the other DNA-notarized vocabularies — is the actual open work here.

## Proposed resolution

- One short "which pipeline do I use?" discriminator, placed where an authoring agent will hit it
  (candidate home: `elohim/sdk/CLAUDE.md`, cited from `elohim/sdk/domains/CLAUDE.md` and the
  `rea-economics` skill) — three rows, each naming its trigger, source of truth, and codegen command.
- Decide whether `REA_ACTIONS` graduates to `elohim/sdk/schemas/v1/enums/`. If it should not (e.g.
  it is DNA-integrity-bound and deliberately Rust-first), record *that* as the documented exception
  rather than leaving the doctrine looking universally applied.

## Provenance

`/memory-ceremony` 2026-07-30, Phase-4b Lens 2 (fresh-context downstream-reader coherence,
verdict YELLOW). The mechanical findings from the same lens — dead service class names, a
7-of-24 REA action subset, a stale `app/` prefix, and an ownership framing that pointed at
prototype TS mirrors as source of truth — were closed inline in the same cycle.
Related: [[feedback-backend-authoritative-frontend-senses]] (the substrate is authoritative;
TypeScript conforms), [[feedback_verify_the_measure_before_the_ranking]].
