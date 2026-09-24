---
id: "backlog-schema-test-contentview-red-blocks-chain"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ContentView schema test red since 2026-07-24 — and because schema:test chains ten scripts with &&, the other nine never run"
slug: "schema-test-contentview-red-blocks-chain"
written: "2026-09-24"
author: "memory-stasis verification lane (CLAIMED-ONLY Tier B, 2026-09-24)"
status: "open"
priority: "medium"
area: "elohim/sdk/schemas"
domain: "code"
tags: [schema, view-contract, gate, codegen, test-chain, sdk]
cites:
  - package.json
  - elohim/sdk/schemas/scripts/test-schema.mjs
  - elohim/sdk/schemas/v1/views/content-view.schema.json
  - genesis/docs/superpowers/plans/2026-07-24-closure-posture-axis-card-plan.md
  - genesis/docs/superpowers/plans/2026-07-24-third-party-gate-closure-plan.md
---

# ContentView schema test red — and the chain it silences

## What is red

`node elohim/sdk/schemas/scripts/test-schema.mjs` exits 1: 76 passed, 2 failed. The two are
`ContentView accepts valid record` and `ContentView accepts null for nullable fields` (the
assertions near lines 274 and 302 of the script). Both closure-posture plans of 2026-07-24 called
these failures "pre-existing" at landing; two months later they are still red, so the drift between
`content-view.schema.json` and the ContentView fixture the test feeds it was never reconciled.

## Why it matters more than two assertions

The root `package.json` `schema:test` script chains ten scripts with `&&`
(`test-schema.mjs && test-manifest-epr-floor.mjs && test-delegates-compute-schema.mjs && …`).
`test-schema.mjs` is first, so its exit 1 stops the other nine — manifest-epr-floor,
delegates-compute, republish-epr, acknowledges-reach-change, zd-feedback and the rest — from
running at all under `pnpm run schema:test`. A green change to any of those schemas is currently
unmeasured by the gate that names them.

It also keeps two plan docs in CLAIMED-ONLY: the verification lane on 2026-09-24 declined to write
a `verified_by:` receipt for `closure-posture-axis-card` and `third-party-gate-closure` because
their named gate exits non-zero, even though every closure and axis-card assertion in the same run
passes and `schema:validate` reads 3432 valid / 0 errors.

## Done when

1. `node elohim/sdk/schemas/scripts/test-schema.mjs` exits 0 — reconcile the schema and the
   fixture (the View Schema Contract rule: schema is the source of truth, Rust struct and codegen
   follow; decide whether the fixture or the schema drifted, and fix the drifting side).
2. `pnpm run schema:test` runs all ten scripts to completion (consider making the chain report
   every script's result instead of stopping at the first red, so one drift cannot hide nine).
3. The two 2026-07-24 plans get their `verified_by:` receipt from a full green run.
