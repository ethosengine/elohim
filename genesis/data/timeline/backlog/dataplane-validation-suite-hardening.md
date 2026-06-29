---
id: "backlog-dataplane-validation-suite-hardening"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane validation suite hardening — gate before flipping the Jenkins stage from advisory(UNSTABLE) to gating(FAILURE)"
slug: "dataplane-validation-suite-hardening"
written: "2026-06-29"
author: "p2p-dataplane validation-suite whole-branch review (MERGE-WITH-FIXES defers)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## Context
The per-concern dataplane validation suite (plan `2026-06-29-p2p-dataplane-validation-suite-plan.md`) landed as an **advisory** edge-Jenkins stage (`catchError → UNSTABLE`). These items are the deferred hardening from the whole-branch review — **all must be resolved before flipping the stage to gating (`FAILURE`)** and before the agentic-developer loop trusts the affected concerns as true-state candidates.

## BLOCKER on hardening (flip-to-FAILURE gate)
- **Deterministic-floor concerns show env-artifact reds, not true state.** Task-8 added `@dataplane` to 7 non-`@wip` resilience features, so the live-alpha dataplane run (default profile, no `-p alpha`, `E2E_STORAGE_URL` defaults to `localhost:8090` unreachable in CI) forces env-local scenarios (`observable-distribution` `@local`, chaos/kubectl-scaffolded ones) to false-red the `blob-durability` / `reconcile-inventory` / `keyspace-coverage` concerns. Fix options: (a) scope-out `@local`/fixture-dependent resilience scenarios from the `@dataplane` selection (keep `@concern:` for rollup but not `@dataplane`); (b) aggregate the resilience-suite results into byConcern SEPARATELY from the live-alpha stage so these concerns show their TRUE state; (c) run `-p alpha` / set `E2E_STORAGE_URL`. Also reconcile the README "deterministic floor … without re-authoring" wording — what runs is the live env-sensitive a2o resilience layer, NOT the Rust P-PROOFS integration tests.
- **P-PROOFS CLAIMED-vs-landed verification** (ci-investigator): confirm the Rust durability proofs (`2026-06-14-dataplane-proofs-plan`) status; the `blob-durability`/`keyspace-coverage` concerns should reflect those, not just the a2o resilience layer.

## Suite-integrity Minors (highest-value first)
- **content-sync cross-peer proof is "both heads non-empty", not "heads-equal"** (`content-sync.feature` 3rd scenario) — a divergence regression could false-pass green. Add a cross-peer heads-equality step (surfaces.ts + dataplane.steps.ts) before the loop relies on this baseline. HIGHEST value (false-pass on the suite's headline concern).
- **`peer-mesh` `divergentAnchor <= 100`** is loose vs observed 6/0 — tighten to `<= 25` so it catches runaway divergence.
- **content-sync feature missing `@requires:multi-node`** — note: scenarios 1–2 are single-peer, so per-scenario tagging is the correct form (not feature-level).
- **`genesis/a2o/.epr-meta` `new-feature-subdir-needs-meta` rule is circular** (`when:{write:".epr-meta"} require-sibling:".epr-meta"` is tautological) — fix the trigger to "any new file in a new subdir," or drop it.

## Cosmetics (final-polish)
- T1 `concernGlyph` vs inline per-scenario glyph DRY; `computeSummary` `{} as Record<…>` placeholder always-overwritten; Background `peer "alpha-A" at "alpha-A"` alias redundancy; CI runs `default` not `-p alpha` profile; README archive-glob doc drift (`cucumber-report-dataplane.json` not archived; only `sprint-report-dataplane.*`); README step-path drift (`steps/dataplane/<slug>.steps.ts` vs actual `steps/dataplane.steps.ts`).

Domain D5×D10. Acceptance: stage can flip to gating once the floor-concern false-reds are eliminated + P-PROOFS verified.
