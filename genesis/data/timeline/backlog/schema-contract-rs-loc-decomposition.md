---
id: "backlog-schema-contract-rs-loc-decomposition"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "schema_contract.rs is past the 3000-line soft ceiling (6341) and each additive view pushes it toward the 7000 hard ceiling — decompose before it crosses"
slug: "schema-contract-rs-loc-decomposition"
written: "2026-08-22"
author: "orchestrator (handed up by the dead_remaining stuck-vs-draining agent)"
status: "open"
priority: "low"
tags: [test-infra, schema-contract, loc-ceiling, bounded-code-fix]
---

# Decompose `elohim/elohim-storage/tests/schema_contract.rs` before the hard ceiling bites

At 6341 lines (measured 2026-08-22, after the `provideLoop` additions in 4ac8099b0), the
schema-contract harness is well past the 3000-line `rs-loc-ceiling` soft nudge and on a
one-way trajectory: every new view or additive field lands another test block here. The
sibling `p2p-mod-loc-ceiling-decomposition.md` row documents what happens when a file
actually crosses the 7000 hard ceiling — do this one while it is still a mechanical split.

## Proposed shape

Split by view family into `tests/schema_contract/` module files (content views, p2p/status
views, shefa/qahal views, auth/session views, …) with the shared harness helpers
(`assert_matches_schema`, fixture builders) in a `mod common`. Pure test-file move — no
schema, codegen, or production code changes; `cargo test --test schema_contract` keeps the
same target name via a thin `schema_contract.rs` that declares the modules.

## Done when

`schema_contract.rs` (the declaring file) is under 200 lines, no module file exceeds ~1500,
the full contract suite passes unchanged (228 tests at time of writing), and the
`rs-loc-ceiling` hook no longer fires on additive-view edits.
